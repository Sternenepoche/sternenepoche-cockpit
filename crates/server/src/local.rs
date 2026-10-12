//! Embedded offline/LAN runtime. No separate server executable, account service or model required.
//!
//! Threads: a few HTTP workers do all socket work with read/write deadlines, check origin and
//! host, read and limit bodies, verify the master password (Argon2) and serve static files.
//! Only parsed requests reach the single game thread, which owns the world, advances time and
//! answers. A stalled or sleeping LAN client can therefore never stop the shared world.
use super::*;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tiny_http::{Header, Method, Request, Response, Server};

const GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 77, 77);
const DISCOVERY_PORT: u16 = 38991;
const PROBE: &[u8] = b"STERNENEPOCHE_LOCAL_V1";
/// Concurrent requests (one thread each). Above this, new requests get "busy" at once.
const HTTP_CONNECTIONS: usize = 64;
/// A client that neither sends nor reads for this long loses its connection, not the world.
const IO_DEADLINE: Duration = Duration::from_secs(10);
const BODY_LIMIT: usize = 131_072;
/// Requests queued for the game thread before new ones get "busy" instead of piling up.
const QUEUE: usize = 64;
/// Wall-clock interval of local time steps. Each step advances whole decision windows.
const TICK: Duration = Duration::from_secs(5);
/// Join attempts with a wrong master password per address and minute.
const JOIN_FAILURES: u8 = 8;
/// `accounts.password` markers of local rounds (never valid Argon2 hashes, so the online
/// login can never accept them). Host-bound names join only from the host machine;
/// device-bound names only with the device secret of their first join; released names
/// bind to the next device that joins with them.
const HOST_BOUND: &str = "lokal-host";
const DEVICE_PREFIX: &str = "lokal-geraet:";
const RELEASED: &str = "lokal-frei";

/// Upgrade helper: read the existing human host identity without opening or advancing a world.
pub fn saved_host_player(root: &Path) -> Result<Option<String>,String> {
    let p=root.join("spiel.sqlite3");
    if !p.exists() {return Ok(None);}
    let db=Connection::open_with_flags(p,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e|e.to_string())?;
    db.query_row("SELECT name FROM accounts WHERE password=?1 AND mode!='agent' ORDER BY id LIMIT 1",[HOST_BOUND],|r|r.get(0)).optional().map_err(|e|e.to_string())
}

/// Full local galaxy: eight sectors, 120 systems each; participants share safe start systems.
pub fn dimensions(bots: u16) -> Result<(u8, u8), String> {
    match bots {
        0..=50 => Ok((2, 80)),
        51..=100 => Ok((4, 80)),
        101..=200 => Ok((4, 120)),
        201..=400 => Ok((8, 120)),
        _ => Err("Bitte 0 bis 400 Bots wählen".into()),
    }
}
pub fn private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v) => v.is_private() || v.is_loopback() || v.is_link_local(),
        std::net::IpAddr::V6(v) => {
            v.is_loopback() || v.is_unique_local() || v.is_unicast_link_local()
        }
    }
}
pub fn seat_capacity(bots: u16) -> Result<u16, String> {
    let (s, n) = dimensions(bots)?;
    Ok(u16::from(s) * u16::from(n) * 4 / 5 - bots)
}
/// Addresses under which LAN guests reach this host (for display and QR code). Uses the
/// routing table only (a connected UDP socket sends nothing).
pub fn lan_addresses(port: u16) -> Vec<String> {
    let mut out = Vec::new();
    for probe in ["192.168.0.1:9", "10.0.0.1:9", "172.16.0.1:9", "192.0.2.1:9"] {
        let ip = UdpSocket::bind("0.0.0.0:0")
            .and_then(|s| s.connect(probe).map(|_| s))
            .and_then(|s| s.local_addr())
            .map(|a| a.ip());
        if let Ok(ip) = ip {
            let url = format!("http://{ip}:{port}");
            if private_ip(ip) && !ip.is_loopback() && !ip.is_unspecified() && !out.contains(&url) {
                out.push(url);
            }
        }
    }
    out
}

/// QR code of a LAN address as SVG. It carries only the address; guests still type the
/// master password, so a photo of the screen does not open the round.
pub fn qr_svg(text: &str) -> Option<String> {
    let qr = qrcodegen::QrCode::encode_text(text, qrcodegen::QrCodeEcc::Medium).ok()?;
    let (n, rand) = (qr.size(), 4);
    let mut path = String::new();
    for y in 0..n {
        for x in 0..n {
            if qr.get_module(x, y) {
                path.push_str(&format!("M{},{}h1v1h-1z", x + rand, y + rand));
            }
        }
    }
    let s = n + 2 * rand;
    Some(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {s} {s}\" shape-rendering=\"crispEdges\"><rect width=\"{s}\" height=\"{s}\" fill=\"#fff\"/><path d=\"{path}\" fill=\"#000\"/></svg>"))
}

/// A request after every transport check: exactly what the game thread needs.
struct Job {
    method: Method,
    path: String,
    role: Option<String>,
    raw: String,
    body: Value,
    peer: IpAddr,
    /// The worker verified the master password; the plaintext never reaches the game thread.
    join: bool,
    reply: mpsc::SyncSender<Reply>,
}

pub struct LocalSession {
    pub url: String,
    /// LAN addresses for guests (host mode only).
    pub lan_urls: Vec<String>,
    pub discovery_warning: Option<String>,
    pub world_id: String,
    stop: Arc<AtomicBool>,
    threads: Vec<thread::JoinHandle<()>>,
    game: Option<thread::JoinHandle<()>>,
    _lock: std::fs::File,
}
#[derive(serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalView { tab:String, planet:String, galaxy:LocalGalaxyView, scroll:u32 }
#[derive(serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalGalaxyView { stage:String, sektor:u8, system:u8 }
impl LocalSession {
    /// Opens (or creates) the local world under `root`. A new world gets a fresh random
    /// galaxy; an existing one continues exactly where it was saved.
    pub fn start(
        root: &Path,
        bots: u16,
        host: bool,
        port: u16,
        password: &str,
    ) -> Result<Self, String> {
        dimensions(bots)?;
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("sitzung.lock"))
            .map_err(|e| e.to_string())?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| {
            "Dieser lokale Spielstand läuft bereits in einem anderen Cockpit".to_string()
        })?;
        // A hash is held by the workers; the plaintext is never written to the world or advertised.
        let hash = Arc::new(password_hash(password)?);
        let mut game = Game::open_world(root, OsRng.next_u64(), Some(bots))?;
        game.local_host = host;
        game.db.execute_batch("CREATE TABLE IF NOT EXISTS local_viewprefs(account INTEGER PRIMARY KEY REFERENCES accounts(id),state TEXT NOT NULL)").map_err(|e|e.to_string())?;
        if game.runtime.bots.len() != bots as usize {
            return Err(format!(
                "Dieser Spielstand gehört zu {} Bots, nicht zu {bots}",
                game.runtime.bots.len()
            ));
        }
        let world_id = game.runtime.world_id.clone();
        let bind: SocketAddr = if host {
            (Ipv4Addr::UNSPECIFIED, port).into()
        } else {
            (Ipv4Addr::LOCALHOST, 0).into()
        };
        let server = Arc::new(listener(bind).map_err(|e| {
            if host {
                format!("Lokale Sitzung konnte Port {port} nicht öffnen ({e}). Anderen Host-Port wählen oder das andere Programm beenden.")
            } else {
                format!("Lokale Sitzung konnte nicht starten: {e}")
            }
        })?);
        let port = server
            .server_addr()
            .to_ip()
            .ok_or("Keine lokale Adresse")?
            .port();
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();
        let mut warning = None;
        if host {
            match discovery_socket() {
                Ok(socket) => {
                    let stop = stop.clone();
                    let count = bots;
                    threads.push(thread::spawn(move || {
                        let mut bytes = [0u8; 256];
                        while !stop.load(Ordering::Relaxed) {
                            if let Ok((n, peer)) = socket.recv_from(&mut bytes) {
                                if &bytes[..n] == PROBE && private_ip(peer.ip()) {
                                    let data = json!({"protocol":"sternenepoche-local-v1","port":port,"name":"Sternenepoche · Lokale Sitzung","bots":count,"api_version":API_VERSION}).to_string();
                                    let _ = socket.send_to(data.as_bytes(), peer);
                                }
                            }
                        }
                    }));
                }
                Err(e) => {
                    warning = Some(format!("Automatische Host-Suche nicht verfügbar ({e}). Beitritt über IP-Adresse möglich."));
                }
            }
        }
        let (tx, rx) = mpsc::sync_channel::<Job>(QUEUE);
        let failures = Arc::new(Mutex::new(BTreeMap::<IpAddr, (Instant, u8)>::new()));
        {
            // One short-lived thread per request (bounded): a client that stalls while sending
            // or receiving holds only its own thread until IO_DEADLINE, never the others.
            let (server, stop) = (server.clone(), stop.clone());
            let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            threads.push(thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let req = match server.recv_timeout(Duration::from_millis(200)) {
                        Ok(Some(req)) => req,
                        Ok(None) => continue,
                        Err(_) => break,
                    };
                    if active.fetch_add(1, Ordering::SeqCst) >= HTTP_CONNECTIONS {
                        active.fetch_sub(1, Ordering::SeqCst);
                        reply(req, Err(err(503, "Zu viele gleichzeitige Anfragen; gleich erneut versuchen")));
                        continue;
                    }
                    let (tx, hash, failures, slot) =
                        (tx.clone(), hash.clone(), failures.clone(), active.clone());
                    let spawned = thread::Builder::new()
                        .name("lokal-http".into())
                        .stack_size(1024 * 1024)
                        .spawn(move || {
                            handle(req, &tx, &hash, &failures, host);
                            slot.fetch_sub(1, Ordering::SeqCst);
                        });
                    if spawned.is_err() {
                        active.fetch_sub(1, Ordering::SeqCst);
                    }
                }
            }));
        }
        let flag = stop.clone();
        let game = thread::spawn(move || run(game, rx, flag));
        Ok(Self {
            url: format!("http://127.0.0.1:{port}"),
            lan_urls: if host { lan_addresses(port) } else { Vec::new() },
            discovery_warning: warning,
            world_id,
            stop,
            threads,
            game: Some(game),
            _lock: lock,
        })
    }
}
impl Drop for LocalSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        // The game thread writes the final checkpoint before it ends.
        if let Some(g) = self.game.take() {
            let _ = g.join();
        }
    }
}
/// Listener whose accepted sockets inherit read and write deadlines (Windows and Linux).
fn listener(addr: SocketAddr) -> Result<Server, String> {
    let s = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))
        .map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(IO_DEADLINE)).map_err(|e| e.to_string())?;
    s.set_write_timeout(Some(IO_DEADLINE)).map_err(|e| e.to_string())?;
    s.bind(&addr.into()).map_err(|e| e.to_string())?;
    s.listen(128).map_err(|e| e.to_string())?;
    Server::from_listener(std::net::TcpListener::from(s), None).map_err(|e| e.to_string())
}
fn discovery_socket() -> Result<UdpSocket, String> {
    let s =
        Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|e| e.to_string())?;
    s.set_reuse_address(true).map_err(|e| e.to_string())?;
    s.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT)).into())
        .map_err(|e| e.to_string())?;
    let s: UdpSocket = s.into();
    s.join_multicast_v4(&GROUP, &Ipv4Addr::UNSPECIFIED)
        .map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|e| e.to_string())?;
    Ok(s)
}
pub fn discover() -> Result<Vec<Value>, String> {
    let s = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_millis(150)))
        .map_err(|e| e.to_string())?;
    s.set_multicast_ttl_v4(1).map_err(|e| e.to_string())?;
    for _ in 0..2 {
        s.send_to(PROBE, (GROUP, DISCOVERY_PORT))
            .map_err(|e| e.to_string())?;
    }
    let start = Instant::now();
    let mut found = BTreeMap::new();
    let mut buf = [0u8; 2048];
    while start.elapsed() < Duration::from_millis(1200) {
        if let Ok((n, peer)) = s.recv_from(&mut buf) {
            if !private_ip(peer.ip()) {
                continue;
            }
            if let Ok(mut v) = serde_json::from_slice::<Value>(&buf[..n]) {
                if v["protocol"] != "sternenepoche-local-v1" {
                    continue;
                }
                if let Some(port) = v["port"].as_u64().filter(|p| *p > 0 && *p <= 65535) {
                    let url = format!("http://{}:{port}", peer.ip());
                    v["url"] = json!(url);
                    found.insert(url, v);
                }
            }
        }
    }
    Ok(found.into_values().collect())
}
fn header(req: &Request, name: &'static str) -> String {
    req.headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.to_string())
        .unwrap_or_default()
}
fn answer(req: Request, code: u16, bytes: Vec<u8>, mime: &str) {
    let mut r = Response::from_data(bytes).with_status_code(code);
    for (k, v) in [
        ("Content-Type", mime),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
    ] {
        r.add_header(Header::from_bytes(k, v).unwrap());
    }
    let _ = req.respond(r);
}
fn reply(req: Request, result: Reply) {
    let (code, value) = match result {
        Ok(v) => (200, v),
        Err((c, s)) => (c, json!({"error":s})),
    };
    answer(
        req,
        code,
        value.to_string().into_bytes(),
        "application/json; charset=utf-8",
    );
}
/// Socket side of one request. Never touches the world; blocks only this worker.
fn handle(
    mut req: Request,
    tx: &mpsc::SyncSender<Job>,
    hash: &str,
    failures: &Mutex<BTreeMap<IpAddr, (Instant, u8)>>,
    host: bool,
) {
    let Some(peer) = req.remote_addr().copied() else {
        return reply(req, Err(err(400, "Unbekannte Gegenstelle")));
    };
    if !private_ip(peer.ip()) {
        return reply(req, Err(err(403, "Nur im lokalen Netzwerk erreichbar")));
    }
    let origin = header(&req, "Origin");
    let host_header = header(&req, "Host");
    if !origin.is_empty() && origin != format!("http://{host_header}") {
        return reply(req, Err(err(403, "Fremder Webseiten-Ursprung")));
    }
    // Block DNS rebinding as well as cross-origin form submissions.
    if !host_header
        .parse::<SocketAddr>()
        .is_ok_and(|a| private_ip(a.ip()))
    {
        return reply(req, Err(err(403, "Numerische lokale IP-Adresse verwenden")));
    }
    let url = req.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let path = path.to_string();
    let role = query
        .split('&')
        .find_map(|p| p.strip_prefix("rolle="))
        .map(str::to_string);
    if req.method() == &Method::Get && !path.starts_with("/api/") {
        if path == "/config.js" {
            return answer(
                req,
                200,
                format!("window.STERNENEPOCHE={{api:location.origin,version:{API_VERSION},local:true}};").into_bytes(),
                "text/javascript; charset=utf-8",
            );
        }
        return match client_asset(&path) {
            Some((b, mime)) => answer(req, 200, b.to_vec(), mime),
            None => reply(req, Err(err(404, "Datei nicht gefunden"))),
        };
    }
    if req.body_length().is_some_and(|n| n > BODY_LIMIT) {
        return reply(req, Err(err(413, "Anfrage zu groß")));
    }
    let mut bytes = Vec::new();
    if req
        .as_reader()
        .take(BODY_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > BODY_LIMIT
    {
        return reply(req, Err(err(400, "Anfrage unvollständig, zu langsam oder zu groß")));
    }
    let mut body = if bytes.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(v) => v,
            Err(_) => return reply(req, Err(err(400, "Ungültiges JSON"))),
        }
    };
    if let Err(e) = eingabe::json_grenzen(&body) {
        return reply(req, Err(err(400, &e)));
    }
    let raw = header(&req, "Authorization")
        .strip_prefix("Bearer ")
        .unwrap_or("")
        .to_string();
    let method = req.method().clone();
    let join = method == Method::Post && path == "/api/local/join";
    if join {
        // Argon2 costs tens of milliseconds: verified here, outside the world.
        let ip = peer.ip();
        {
            let mut f = failures.lock().unwrap_or_else(|p| p.into_inner());
            f.retain(|_, (t, _)| t.elapsed() < Duration::from_secs(60));
            if f.get(&ip).is_some_and(|(_, n)| *n >= JOIN_FAILURES) {
                return reply(
                    req,
                    Err(err(429, "Bitte eine Minute bis zur nächsten Anmeldung warten")),
                );
            }
        }
        let password = body["password"].as_str().unwrap_or("");
        let offline_name_only = !host && ip.is_loopback() && password.is_empty();
        if password.len() > 128 || (!offline_name_only && !password_matches(password, hash)) {
            let mut f = failures.lock().unwrap_or_else(|p| p.into_inner());
            let e = f.entry(ip).or_insert((Instant::now(), 0));
            e.1 = e.1.saturating_add(1);
            return reply(req, Err(err(401, "Masterpasswort stimmt nicht")));
        }
        failures.lock().unwrap_or_else(|p| p.into_inner()).remove(&ip);
        if let Some(o) = body.as_object_mut() {
            o.remove("password");
        }
    }
    let (back, answer_rx) = mpsc::sync_channel(1);
    let job = Job {
        method,
        path,
        role,
        raw,
        body,
        peer: peer.ip(),
        join,
        reply: back,
    };
    match tx.try_send(job) {
        Ok(()) => {}
        Err(mpsc::TrySendError::Full(_)) => {
            return reply(req, Err(err(503, "Die Welt ist gerade ausgelastet; gleich erneut versuchen")))
        }
        Err(mpsc::TrySendError::Disconnected(_)) => {
            return reply(req, Err(err(503, "Lokale Sitzung wird beendet")))
        }
    }
    let result = answer_rx
        .recv_timeout(Duration::from_secs(60))
        .unwrap_or_else(|_| Err(err(503, "Die Welt antwortet gerade nicht")));
    reply(req, result);
}
/// The game thread: answers requests and advances time. Time follows the chosen tempo
/// exactly (whole decision windows, the remainder carries over); a suspended machine does
/// not catch up, matching the online server.
fn run(mut game: Game, rx: mpsc::Receiver<Job>, stop: Arc<AtomicBool>) {
    let mut last = Instant::now();
    let mut next = last + TICK;
    let mut rest = 0.0f64;
    while !stop.load(Ordering::Relaxed) {
        let wait = next
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(200));
        match rx.recv_timeout(wait) {
            Ok(job) => {
                let result = game.local_job(&job);
                let _ = job.reply.send(result);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let now = Instant::now();
        if now < next {
            continue;
        }
        let elapsed = now.duration_since(last).as_secs_f64();
        last = now;
        next = now + TICK;
        if game.runtime.paused || game.world.beendet() {
            rest = 0.0;
            continue;
        }
        let seconds;
        (seconds, rest) = time_step(rest, elapsed, game.runtime.tempo, game.world.regeln.fenster());
        if seconds == 0 {
            continue;
        }
        let measured = Instant::now();
        match game.advance(seconds) {
            Ok(_) => game.local_error = None,
            Err((_, e)) => game.local_error = Some(e),
        }
        game.local_tick_times
            .push(measured.elapsed().as_secs_f64() * 1000.0);
        if game.local_tick_times.len() > 60 {
            game.local_tick_times.remove(0);
        }
    }
    if let Err((_, e)) = game.checkpoint_now() {
        eprintln!("Lokaler Spielstand: letzter Checkpoint fehlgeschlagen: {e}");
    }
}
/// Game seconds due after `elapsed` wall seconds at `tempo`. The kernel advances in whole
/// decision windows; the remainder carries over, so 1× really is real time (previously each
/// 5-second tick jumped a full 30-second window: 6× instead of 1×). Longer gaps (sleep,
/// debugger) count as two ticks at most: no unattended catch-up.
fn time_step(rest: f64, elapsed: f64, tempo: u32, window: i64) -> (u32, f64) {
    let window = window.max(1) as f64;
    let rest = rest + elapsed.clamp(0.0, TICK.as_secs_f64() * 2.0) * tempo as f64;
    let seconds = ((rest / window).floor() * window).min(3600.0);
    (seconds as u32, rest - seconds)
}
impl Game {
    fn local_job(&mut self, job: &Job) -> Reply {
        let owner = job.peer.is_loopback();
        if job.join {
            return self.local_join(&job.body, owner);
        }
        self.local_route(
            &job.method,
            &job.path,
            &job.raw,
            &job.body,
            job.role.as_deref(),
            owner,
        )
    }
    /// Name + master password (verified by the worker). The name decides the empire, so it
    /// is bound on first use: to the host machine, or to the joining device's secret.
    /// A second player who picks a taken name gets a clear refusal instead of the
    /// other player's empire.
    fn local_join(&mut self, body: &Value, from_host: bool) -> Reply {
        let name = str_field(body, "name")?.trim().to_string();
        let device = body["geraet"].as_str().unwrap_or("");
        let binding = if from_host {
            HOST_BOUND.to_string()
        } else {
            if !(32..=128).contains(&device.len())
                || !device
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err(err(
                    400,
                    "Gerätekennung fehlt. Seite neu laden oder das Cockpit aktualisieren.",
                ));
            }
            format!("{DEVICE_PREFIX}{}", digest(device))
        };
        let auth = match self.account_by_name(&name) {
            Some(a) => {
                let mine = a.password == binding
                    || a.password == RELEASED
                    || (from_host && !a.password.starts_with(DEVICE_PREFIX));
                if !mine {
                    return Err(err(409, if a.password == HOST_BOUND {
                        "Dieser Name gehört dem Host dieser Runde. Bitte einen anderen Namen wählen."
                    } else {
                        "Dieser Name spielt schon von einem anderen Gerät. Anderen Namen wählen oder den Host bitten, den Namen freizugeben."
                    }));
                }
                if a.password != binding {
                    self.db
                        .execute(
                            "UPDATE accounts SET password=?1 WHERE id=?2",
                            params![binding, a.id],
                        )
                        .map_err(|_| err(503, "Anmeldung nicht gespeichert"))?;
                }
                self.login_verified(a.id)?
            }
            None => self.register(&json!({"name":name}), binding)?,
        };
        let a = self.authenticate(auth["token"].as_str().unwrap_or(""))?;
        // The cockpit and its agents take a seat at once. The browser asks for
        // `platz_waehlen` and lets the player choose mode and people in the claim dialog.
        if a.sid.is_none() && body["platz_waehlen"] != true {
            self.claim(&a, &json!({"mode":body["mode"].as_str().unwrap_or("gemischt"),"volk":body["volk"].as_str().unwrap_or("veyari")}))?;
        }
        let a = self.account_by_name(&name).ok_or_else(|| err(503, "Konto nicht lesbar"))?;
        let mut auth = auth;
        auth["spieler"] = json!(a.sid);
        auth["mode"] = json!(a.mode);
        Ok(auth)
    }
    fn local_info(&self, host: bool) -> Value {
        let mut samples = self.local_tick_times.clone();
        samples.sort_by(f64::total_cmp);
        let free = self.world.aufklaerung.inaktive_spieler.len();
        json!({"local":true,"host":host,"world_id":self.runtime.world_id,"sekunden":self.world.zeit,
            "bots":self.runtime.bots.len(),"sektoren":self.world.regeln.welt.sektoren,"systeme":self.world.regeln.welt.systeme_je_sektor,
            "tempo":self.runtime.tempo,"paused":self.runtime.paused,"beendet":self.world.beendet(),
            "plaetze":self.runtime.seat_capacity,"freigabe":self.runtime.admission_limit,
            "belegt":(self.runtime.seat_capacity as usize).saturating_sub(free),"wartende":self.runtime.waitlist.len(),
            "tick_ms":self.local_tick_times.last(),"tick_p95_ms":samples.get(samples.len().saturating_sub(1)*95/100),"tick_samples":samples.len(),
            "fehler":self.storage_error.as_ref().or(self.local_error.as_ref())})
    }
    fn local_route(
        &mut self,
        method: &Method,
        path: &str,
        raw: &str,
        body: &Value,
        role: Option<&str>,
        owner: bool,
    ) -> Reply {
        let host = self.local_host;
        if method == &Method::Get {
            match path {
                "/api/local/info" => return Ok(self.local_info(host)),
                "/api/lobby" | "/api/health" => return Ok(self.lobby()),
                "/api/rules" => {
                    return Ok(
                        json!({"api_version":API_VERSION,"schema":kern::aktion::antwortschema(Rolle::Alle),"volk":Volk::ALLE.into_iter().map(|v|v.name()).collect::<Vec<_>>(),"catalog":public_catalog::catalog(&self.world.regeln),"text":format!("{}\n{}\n{}",kern::regeltext::regeltext(&self.world.regeln,Rolle::Alle),kern::kolonisation::REGELTEXT_V2,kern::ausscheiden::REGELTEXT),"harness_regeln":crate::harness::REGELN_VERSION}),
                    )
                }
                "/api/registration/status" => return Ok(json!({"enabled":false,"local":true})),
                "/api/harness/v1/info" => return Ok(self.harness_info()),
                _ => {}
            }
        }
        if method == &Method::Post && path == "/api/sitzung/einloesen" {
            return self.uebergabe_einloesen(body);
        }
        if path.starts_with("/api/harness/v1/") && !path.starts_with("/api/harness/v1/schluessel") {
            let (a, s) = self.harness_authenticate(raw)?;
            return match (method, path) {
                (&Method::Get, "/api/harness/v1/ich") => Ok(self.harness_ich(&a, &s)),
                (&Method::Get, "/api/harness/v1/schema") => {
                    self.harness_schema(&s, role).map(|mut v| {
                        v["katalog"] = public_catalog::catalog(&self.world.regeln);
                        v
                    })
                }
                (&Method::Get, "/api/harness/v1/lage") => self.harness_lage(&a, &s, role),
                (&Method::Post, "/api/harness/v1/werkzeug") => self.harness_werkzeug(&a, body),
                (&Method::Post, "/api/harness/v1/freigabe") => self.harness_freigabe(&a, &s, body),
                (&Method::Post, "/api/harness/v1/handeln") => self.harness_handeln(&a, &s, body),
                (&Method::Post, "/api/harness/v1/bericht") => self.harness_bericht(&a, &s, body),
                (&Method::Post, "/api/harness/v1/mcp") => {
                    mcp::bearbeiten(self, &a, &s, body, public_catalog::catalog)
                        .ok_or_else(|| err(400, "Keine MCP-Antwort"))
                }
                _ => Err(err(404, "Harness-Endpunkt unbekannt")),
            };
        }
        let a = self.authenticate(raw)?;
        match (method, path) {
            (&Method::Get, "/api/local/viewprefs") => self.local_viewprefs(&a, None),
            (&Method::Post, "/api/local/viewprefs") => self.local_viewprefs(&a, Some(body)),
            (&Method::Post, "/api/local/agent") if owner => {
                let name = str_field(body, "name")?;
                if self.account_by_name(name).is_none_or(|a| a.sid.is_none())
                    && self.available_admissions() <= self.runtime.waitlist.len()
                {
                    return Err(err(409,"Keine freigegebenen Plätze für weitere Agenten. Der Host kann die Spielerfreigabe in der lokalen Sitzung erhöhen."));
                }
                if self
                    .account_by_name(name)
                    .is_some_and(|a| a.mode != "agent")
                {
                    return Err(err(
                        409,
                        "Dieser Name gehört bereits einem menschlichen oder gemischten Spieler",
                    ));
                }
                self.local_join(
                    &json!({"name":name,"volk":body["volk"].as_str().unwrap_or("veyari"),"mode":"agent"}),
                    true,
                )
            }
            (&Method::Get, "/api/me") => Ok(self.me(&a)),
            (&Method::Get, "/api/view") => self.view(&a, Rolle::Alle),
            (&Method::Get, "/api/workspace") => self.workspace(&a),
            (&Method::Post, "/api/workspace") => self.save_workspace(&a, body),
            (&Method::Post, "/api/claim") => self.claim(&a, body),
            (&Method::Post, "/api/tool") => self.tool(&a, body),
            (&Method::Post, "/api/context") => {
                let role = body["rolle"]
                    .as_str()
                    .and_then(Rolle::aus_name)
                    .filter(|r| *r != Rolle::Alle)
                    .ok_or_else(|| err(400, "Regierungsrolle auswählen"))?;
                if body["schema"] == true {
                    Ok(json!({"schema":kern::aktion::antwortschema(role)}))
                } else {
                    self.view(&a,role).map(|view|json!({"view":view,"schema":kern::aktion::antwortschema(role),"text":kern::regeltext::regeltext(&self.world.regeln,role)}))
                }
            }
            (&Method::Post, "/api/waitlist/leave") => self.leave_waitlist(&a, body),
            (&Method::Post, "/api/fairplay/bestaetigen") => self.fairplay_bestaetigen(&a),
            // Bots react inside the command's own transaction (see `Game::command`).
            (&Method::Post, "/api/command") => self.command(&a, body),
            (&Method::Post, "/api/lease") => self.lease(&a, body),
            (&Method::Post, "/api/agent-report") => self.agent_report(&a, body),
            (&Method::Post, "/api/sitzung/uebergabe") => self.uebergabe_erstellen(&a),
            (&Method::Post, "/api/logout") => self.logout(&a, raw),
            (&Method::Post, "/api/local/control") if owner => {
                let release = body["name_freigeben"].as_str().map(str::to_string);
                if let Some(name) = &release {
                    let target = self
                        .account_by_name(name)
                        .ok_or_else(|| err(404, "Name unbekannt"))?;
                    if target.password == HOST_BOUND {
                        return Err(err(409, "Namen des Hosts und seiner Agenten bleiben an diesen Rechner gebunden"));
                    }
                }
                self.transaction(|g| {
                    if let Some(p) = body["paused"].as_bool() {
                        g.runtime.paused = p;
                    }
                    if let Some(t) = body["tempo"]
                        .as_u64()
                        .filter(|t| [1, 10, 30, 60].contains(t))
                    {
                        g.runtime.tempo = t as u32;
                    }
                    if let Some(n) = body["plaetze"]
                        .as_u64()
                        .filter(|n| *n >= 1 && *n <= g.runtime.seat_capacity as u64)
                    {
                        g.runtime.admission_limit = n as u16;
                        g.admit_waiting()?;
                    }
                    if let Some(name) = &release {
                        // The next device that joins with this name takes it over; open
                        // sessions of the old device end now.
                        g.db.execute("UPDATE accounts SET password=?1 WHERE name=?2", params![RELEASED, name])
                            .and_then(|_| g.db.execute("DELETE FROM sessions WHERE account=(SELECT id FROM accounts WHERE name=?1)", [name]))
                            .map_err(|_| err(503, "Freigabe nicht gespeichert"))?;
                    }
                    Ok(json!({"ok":true}))
                })
            }
            (&Method::Get, "/api/local/spieler") if owner => self.local_players(),
            (&Method::Get, "/api/harness/v1/schluessel") => self.harness_schluessel_liste(&a),
            (&Method::Post, "/api/harness/v1/schluessel") => {
                self.harness_schluessel_erstellen(&a, body)
            }
            (&Method::Post, "/api/harness/v1/schluessel/widerrufen") => {
                self.harness_schluessel_widerrufen(&a, body)
            }
            _ => Err(err(404, "Unbekannter lokaler Endpunkt")),
        }
    }
    /// Host view of the participants: who plays where, from which kind of device.
    fn local_viewprefs(&mut self, a: &Account, body: Option<&Value>) -> Reply {
        if let Some(v)=body {
            self.world_matches(v)?;
            let state: LocalView = serde_json::from_value(v["state"].clone()).map_err(|_|err(400,"Ungültige Spielansicht"))?;
            let (sectors,systems)=dimensions(self.runtime.bots.len() as u16).map_err(|e|err(400,&e))?;
            if !["reich","gebaeude","kolonie","forschen","werft","verteidigung","flotten","karte","kolonien","kampf","aktionen","briefkasten","allianzbereich","allianzpost","berichte","imperium","freischaltungen","agent","pinwaende","profil","regeln"].contains(&state.tab.as_str())
                || state.planet.len()>24 || !state.planet.bytes().all(|b|b.is_ascii_digit()||b==b':')
                || !["galaxy","sector","system"].contains(&state.galaxy.stage.as_str())
                || !(1..=sectors).contains(&state.galaxy.sektor) || !(1..=systems).contains(&state.galaxy.system)
                || state.scroll>100000 {
                return Err(err(400,"Spielansicht außerhalb der lokalen Welt"));
            }
            // These account-private preferences do not mutate the simulation. One small
            // SQLite write avoids serializing the whole galaxy on every navigation click.
            self.db.execute("INSERT INTO local_viewprefs VALUES(?1,?2) ON CONFLICT(account) DO UPDATE SET state=excluded.state",params![a.id,serde_json::to_string(&state).map_err(|_|err(400,"Ungültige Spielansicht"))?]).map_err(|_|err(503,"Spielansicht nicht gespeichert"))?;
        }
        let raw:Option<String>=self.db.query_row("SELECT state FROM local_viewprefs WHERE account=?1",[a.id],|r|r.get(0)).optional().map_err(|_|err(503,"Spielansicht nicht lesbar"))?;
        Ok(json!({"world_id":self.runtime.world_id,"state":raw.map(|r|serde_json::from_str::<Value>(&r)).transpose().map_err(|_|err(503,"Spielansicht beschädigt"))?}))
    }
    fn local_players(&self) -> Reply {
        let mut s = self
            .db
            .prepare("SELECT name,password,sid,mode FROM accounts ORDER BY id")
            .map_err(|_| err(503, "Spielerliste nicht lesbar"))?;
        let rows = s
            .query_map([], |r| {
                let binding: String = r.get(1)?;
                Ok(json!({"name":r.get::<_,String>(0)?,"spieler":r.get::<_,Option<i64>>(2)?,"mode":r.get::<_,String>(3)?,
                    "bindung":if binding==HOST_BOUND {"host"} else if binding.starts_with(DEVICE_PREFIX) {"geraet"} else if binding==RELEASED {"frei"} else {"alt"}}))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(|_| err(503, "Spielerliste nicht lesbar"))?;
        Ok(json!({"spieler":rows}))
    }
}

pub fn client_asset(path: &str) -> Option<(&'static [u8], &'static str)> {
    match path {
        "/" | "/index.html" => Some((
            include_bytes!("../../../web-client/index.html"),
            "text/html; charset=utf-8",
        )),
        "/audio/bots/oekonom.ogg" => Some((
            include_bytes!("../../../web-client/audio/bots/oekonom.ogg"),
            "audio/ogg",
        )),
        "/audio/bots/haendler.ogg" => Some((
            include_bytes!("../../../web-client/audio/bots/haendler.ogg"),
            "audio/ogg",
        )),
        "/audio/bots/igel.ogg" => Some((
            include_bytes!("../../../web-client/audio/bots/igel.ogg"),
            "audio/ogg",
        )),
        "/audio/bots/raeuber.ogg" => Some((
            include_bytes!("../../../web-client/audio/bots/raeuber.ogg"),
            "audio/ogg",
        )),
        "/local-play.js" => Some((
            include_bytes!("../../../web-client/local-play.js"),
            "text/javascript; charset=utf-8",
        )),
        "/local-galaxy.js" => Some((
            include_bytes!("../../../web-client/local-galaxy.js"),
            "text/javascript; charset=utf-8",
        )),
        // The remainder of this allowlist is shared with the online client.
        _ => local_client_asset(path).or_else(|| web_assets::get(path)),
    }
}
include!("local_assets.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    fn root() -> PathBuf {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../laeufe/local-tests");
        // Each test world holds a full database; they are evidence for a day, not forever.
        // Only test-created folders (64 hex digits) older than 24 h are removed.
        for d in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let name = d.file_name().to_string_lossy().into_owned();
            let old = d
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t.elapsed().is_ok_and(|e| e > Duration::from_secs(86_400)));
            if old && name.len() == 64 && name.bytes().all(|b| b.is_ascii_hexdigit()) {
                let _ = std::fs::remove_dir_all(d.path());
            }
        }
        base.join(token())
    }
    fn http(url: &str, method: &str, path: &str, body: Value, auth: &str) -> (u16, Value) {
        let addr = url.trim_start_matches("http://");
        let mut s = std::net::TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let bytes = if method == "GET" {
            String::new()
        } else {
            body.to_string()
        };
        write!(s,"{method} {path} HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {auth}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{bytes}",bytes.len()).unwrap();
        let mut response = String::new();
        s.read_to_string(&mut response).unwrap();
        let (headers, data) = response.split_once("\r\n\r\n").unwrap();
        (
            headers.split_whitespace().nth(1).unwrap().parse().unwrap(),
            serde_json::from_str(data).unwrap(),
        )
    }
    #[test]
    fn world_thresholds_and_maximum_are_real_playable_worlds() {
        assert!(dimensions(401).is_err());
        for (bots, expected) in [
            (0, (2, 80)),
            (30, (2, 80)),
            (50, (2, 80)),
            (51, (4, 80)),
            (100, (4, 80)),
            (101, (4, 120)),
            (200, (4, 120)),
            (201, (8, 120)),
            (400, (8, 120)),
        ] {
            let g = Game::open_world(&root(), 712, Some(bots)).unwrap();
            assert_eq!(g.runtime.bots.len(), bots as usize);
            assert_eq!(
                g.world.spieler.len(),
                bots as usize + seat_capacity(bots).unwrap() as usize
            );
            assert_eq!(
                (
                    g.world.regeln.welt.sektoren,
                    g.world.regeln.welt.systeme_je_sektor
                ),
                expected
            );
            assert_eq!(
                g.world.systeme.len(),
                expected.0 as usize * expected.1 as usize
            );
            assert_eq!(
                g.available_admissions(),
                32.min(seat_capacity(bots).unwrap()) as usize
            );
        }
    }
    #[test]
    fn offline_http_join_diplomacy_idempotency_pause_handoff_and_resume() {
        let root = root();
        let password = "lokales-masterpasswort";
        let g = Game::open_world(&root, 712, Some(30)).unwrap();
        let target = g.world.spieler[0].name.clone();
        drop(g);
        let local = LocalSession::start(&root, 30, false, 0, password).unwrap();
        let url = &local.url;
        assert_eq!(http(url, "GET", "/api/view", json!({}), "").0, 401);
        assert_eq!(
            http(
                url,
                "POST",
                "/api/local/join",
                json!({"name":"Entdecker","password":"wrong"}),
                ""
            )
            .0,
            401
        );
        let (code, s) = http(
            url,
            "POST",
            "/api/local/join",
            json!({"name":"Entdecker","password":password}),
            "",
        );
        assert_eq!(code, 200, "{s}");
        let t = s["token"].as_str().unwrap();
        let (_, me) = http(url, "GET", "/api/me", json!({}), t);
        assert_eq!(me["spieler"], 30);
        let (_, other) = http(
            url,
            "POST",
            "/api/local/join",
            json!({"name":"Mitspieler","password":password}),
            "",
        );
        let (_, otherme) = http(
            url,
            "GET",
            "/api/me",
            json!({}),
            other["token"].as_str().unwrap(),
        );
        assert_eq!(otherme["spieler"], 31);
        let (_, view) = http(url, "GET", "/api/view", json!({}), t);
        assert!(view["planeten"].is_array());
        let cmd = json!({"world_id":me["world_id"],"request_id":"brief-1","aktionen":[{"typ":"brief_senden","kanal":"privat","an":target,"betreff":"Handel","text":"Ich möchte Erz handeln. Wie geht das?","antwort_auf":null}]});
        let (code, r) = http(url, "POST", "/api/command", cmd.clone(), t);
        assert_eq!(code, 200, "{r}");
        assert_eq!(r["ergebnisse"][0]["ok"], true, "{r}");
        let (_, v1) = http(url, "GET", "/api/view", json!({}), t);
        assert!(
            v1.to_string().contains("Ware, Menge und Gegenleistung"),
            "reply missing"
        );
        assert_eq!(http(url, "POST", "/api/command", cmd, t).0, 200);
        let (_, v2) = http(url, "GET", "/api/view", json!({}), t);
        assert_eq!(v1["kommunikation"], v2["kommunikation"]);
        assert_eq!(
            http(
                url,
                "POST",
                "/api/local/control",
                json!({"paused":true,"tempo":60}),
                t
            )
            .0,
            200
        );
        let (_, info) = http(url, "GET", "/api/local/info", json!({}), "");
        assert_eq!(info["tempo"], 60);
        assert_eq!(info["paused"], true);
        let (_, code) = http(url, "POST", "/api/sitzung/uebergabe", json!({}), t);
        assert_eq!(
            http(
                url,
                "POST",
                "/api/sitzung/einloesen",
                json!({"code":code["code"]}),
                ""
            )
            .0,
            200
        );
        assert_eq!(
            http(
                url,
                "POST",
                "/api/sitzung/einloesen",
                json!({"code":code["code"]}),
                ""
            )
            .0,
            401
        );
        let world = me["world_id"].clone();
        drop(local);
        let local = LocalSession::start(&root, 30, false, 0, password).unwrap();
        let (code, s) = http(
            &local.url,
            "POST",
            "/api/local/join",
            json!({"name":"Entdecker","password":password}),
            "",
        );
        assert_eq!(code, 200, "{s}");
        assert_eq!(s["world_id"], world);
        drop(local);
        let g = Game::open_world(&root, 712, Some(30)).unwrap();
        assert_eq!(g.runtime.world_id, world.as_str().unwrap());
        assert!(!String::from_utf8_lossy(&g.world.zu_bytes()).contains(password));
    }

    #[test]
    fn saved_view_pause_and_checkpoint_survive_restart_without_cross_account_leaks() {
        let root=root();
        let pass="lokale-test-partie";
        let local=LocalSession::start(&root,0,false,0,pass).unwrap();
        let (code,s)=http(&local.url,"POST","/api/local/join",json!({"name":"Entdecker"}),"");
        assert_eq!(code,200,"{s}");
        let t=s["token"].as_str().unwrap();
        let (_,info)=http(&local.url,"GET","/api/local/info",json!({}),"");
        let state=json!({"tab":"karte","planet":"1:1:1","galaxy":{"stage":"system","sektor":2,"system":80},"scroll":123});
        let payload=json!({"world_id":info["world_id"],"state":state});
        assert_eq!(http(&local.url,"POST","/api/local/viewprefs",payload.clone(),"").0,401);
        assert_eq!(http(&local.url,"POST","/api/local/viewprefs",payload.clone(),t).0,200);
        let mut wrong=payload.clone();wrong["world_id"]=json!("old-world");
        assert_eq!(http(&local.url,"POST","/api/local/viewprefs",wrong,t).0,409);
        let mut invalid=payload.clone();invalid["state"]["galaxy"]["sektor"]=json!(8);
        assert_eq!(http(&local.url,"POST","/api/local/viewprefs",invalid,t).0,400);
        let mut secret=payload.clone();secret["state"]["password"]=json!("no-credentials");
        assert_eq!(http(&local.url,"POST","/api/local/viewprefs",secret,t).0,400);
        let (_,other)=http(&local.url,"POST","/api/local/join",json!({"name":"Mitspieler","password":pass}),"");
        assert!(http(&local.url,"GET","/api/local/viewprefs",json!({}),other["token"].as_str().unwrap()).1["state"].is_null());
        assert_eq!(http(&local.url,"POST","/api/local/control",json!({"paused":true,"speichern":true}),t).0,200);
        let (_,paused)=http(&local.url,"GET","/api/local/info",json!({}),"");
        assert_eq!(paused["paused"],true);
        drop(local);
        assert_eq!(saved_host_player(&root).unwrap().as_deref(),Some("Entdecker"));
        let resumed=LocalSession::start(&root,0,false,0,pass).unwrap();
        let (_,s)=http(&resumed.url,"POST","/api/local/join",json!({"name":"Entdecker","password":pass}),"");
        let (_,p)=http(&resumed.url,"GET","/api/local/viewprefs",json!({}),s["token"].as_str().unwrap());
        assert_eq!(p["state"],state);
        let (_,info)=http(&resumed.url,"GET","/api/local/info",json!({}),"");
        assert_eq!(info["paused"],true);
        assert_eq!(info["sekunden"],paused["sekunden"]);
        assert_eq!(info["world_id"],paused["world_id"]);
        drop(resumed);
        let lan=LocalSession::start(&root,0,true,0,pass).unwrap();
        assert_eq!(http(&lan.url,"POST","/api/local/join",json!({"name":"Gast"}),"").0,401);
        assert_eq!(http(&lan.url,"POST","/api/local/join",json!({"name":"Gast","password":pass}),"").0,200);
        drop(lan);
    }
    #[test]
    fn local_addresses_and_discovery_do_not_advertise_credentials() {
        assert!(private_ip("192.168.1.10".parse().unwrap()));
        assert!(private_ip("127.0.0.1".parse().unwrap()));
        assert!(!private_ip("8.8.8.8".parse().unwrap()));
        let local = LocalSession::start(&root(), 30, true, 0, "lan-test-masterpasswort").unwrap();
        assert!(
            local.discovery_warning.is_none(),
            "{:?}",
            local.discovery_warning
        );
        let port = reqwest_port(&local.url);
        let results = discover().unwrap();
        assert!(results.iter().any(|r| r["port"] == port));
        assert!(!json!(results).to_string().contains("passwort"));
    }
    #[test]
    fn bots_join_alliances_and_trade_goods_for_real() {
        let mut g = Game::open_world(&root(), 712, Some(30)).unwrap();
        let auth = g
            .register(
                &json!({"name":"Allianzspieler"}),
                password_hash("test-masterpasswort").unwrap(),
            )
            .unwrap();
        let a = g.authenticate(auth["token"].as_str().unwrap()).unwrap();
        g.claim(&a, &json!({"mode":"gemischt","volk":"veyari"}))
            .unwrap();
        let a = g.account_by_name("Allianzspieler").unwrap();
        let sid = a.sid.unwrap();
        let target = g.world.spieler[0].name.clone();
        let cmd = json!({"world_id":g.runtime.world_id,"request_id":"allianz-1","aktionen":[{"typ":"allianz_gruenden","name":"Entdeckerbund"},{"typ":"allianz_einladen","spieler":target}]});
        let v = g
            .local_route(
                &Method::Post,
                "/api/command",
                auth["token"].as_str().unwrap(),
                &cmd,
                None,
                true,
            )
            .unwrap();
        assert!(
            v["ergebnisse"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["ok"] == true),
            "{v}"
        );
        assert_eq!(
            g.world.spieler[sid as usize].allianz,
            g.world.spieler[0].allianz
        );
        let chat = json!({"world_id":g.runtime.world_id,"request_id":"allianz-chat","aktionen":[{"typ":"brief_senden","kanal":"allianz","an":"","betreff":"Handel","text":"Wie handeln wir Erz?","antwort_auf":null}]});
        let v = g
            .local_route(
                &Method::Post,
                "/api/command",
                auth["token"].as_str().unwrap(),
                &chat,
                None,
                true,
            )
            .unwrap();
        assert_eq!(v["ergebnisse"][0]["ok"], true, "{v}");
        let replies = g
            .world
            .nachrichten
            .iter()
            .filter(|n| n.von == 0 && n.text.contains("Ware, Menge und Preis"))
            .count();
        assert_eq!(
            replies, 1,
            "messages={:?}; log={:?}; ki={}",
            g.world.nachrichten, g.world.logpuffer, g.world.spieler[0].ki
        );
        for _ in 0..3 {
            lauf::bots::reagieren(&mut g.world, 0, Bottyp::Oekonom);
            lauf::bots::allianz_reagieren(
                &mut g.world,
                0,
                &g.runtime.bots.keys().copied().collect(),
            );
        }
        assert_eq!(
            g.world
                .nachrichten
                .iter()
                .filter(|n| n.von == 0 && n.text.contains("Ware, Menge und Preis"))
                .count(),
            replies
        );
        let seller = g.world.spieler[0].heimat as usize;
        let buyer = g.world.spieler[sid as usize].heimat as usize;
        g.world.planeten[seller].gebaeude[Gebaeude::Markt.idx()] = 1;
        g.world.planeten[seller].bestand[Gut::Erz.idx()] = 1000 * M;
        g.world.spieler[sid as usize].credits = 10000 * M;
        let seller_coord = g.world.planeten[seller].koord;
        let buyer_coord = g.world.planeten[buyer].koord;
        g.world
            .intern_anbieten(0, seller_coord, Gut::Erz, 100 * M, M)
            .unwrap();
        g.world.intern_kaufen(sid, 1, buyer_coord).unwrap();
        assert!(g.world.kommunikation.angebote[0].ankunft.is_some());
        assert_eq!(g.world.kommunikation.angebote[0].kaeufer, Some(sid));
    }
    fn reqwest_port(url: &str) -> u16 {
        url.rsplit(':').next().unwrap().parse().unwrap()
    }
    fn count(g: &Game, sql: &str) -> u64 {
        g.db.query_row(sql, [&g.runtime.world_id], |r| r.get::<_, u64>(0))
            .unwrap()
    }

    /// A LAN client that announces a body and never sends it (laptop lid closed) or never
    /// finishes its headers may only block its own connection, never the shared world.
    #[test]
    fn stalled_lan_clients_never_stop_the_world() {
        let local =
            LocalSession::start(&root(), 30, false, 0, "stall-test-masterpasswort").unwrap();
        let addr = local.url.trim_start_matches("http://").to_string();
        let mut stalled = Vec::new();
        for _ in 0..8 {
            let mut s = std::net::TcpStream::connect(&addr).unwrap();
            write!(s,"POST /api/command HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: 50000\r\n\r\n").unwrap();
            s.write_all(&[b' '; 3000]).unwrap();
            stalled.push(s);
            let mut h = std::net::TcpStream::connect(&addr).unwrap();
            h.write_all(b"GET /api/local/info HTTP/1.1\r\nHo").unwrap();
            stalled.push(h);
        }
        std::thread::sleep(Duration::from_millis(300));
        let start = Instant::now();
        for _ in 0..5 {
            assert_eq!(http(&local.url, "GET", "/api/local/info", json!({}), "").0, 200);
        }
        assert!(start.elapsed() < Duration::from_secs(3), "{:?}", start.elapsed());
        drop(stalled);
    }

    #[test]
    fn names_belong_to_the_host_or_to_one_device() {
        let mut g = Game::open_world(&root(), 712, Some(30)).unwrap();
        let (dev_a, dev_b) = ("a".repeat(40), "b".repeat(40));
        let host = g
            .local_join(&json!({"name":"Kapitaen","volk":"krath"}), true)
            .unwrap();
        assert!(host["spieler"].is_u64(), "{host}");
        let owner = host["token"].as_str().unwrap().to_string();
        // A guest who picks a taken name gets a refusal, never someone else's empire.
        let taken = g
            .local_join(&json!({"name":"Kapitaen","geraet":dev_a}), false)
            .unwrap_err();
        assert_eq!(taken.0, 409, "{taken:?}");
        let guest = g
            .local_join(&json!({"name":"Gast","geraet":dev_a}), false)
            .unwrap();
        for other in [
            json!({"name":"Gast","geraet":dev_b}),
            json!({"name":"gAsT","geraet":dev_b}),
        ] {
            assert_eq!(g.local_join(&other, false).unwrap_err().0, 409);
        }
        assert_eq!(
            g.local_join(&json!({"name":"Gast"}), false).unwrap_err().0,
            400
        );
        // Not even the host machine takes over a guest's empire by typing its name.
        assert_eq!(
            g.local_join(&json!({"name":"Gast"}), true).unwrap_err().0,
            409
        );
        let again = g
            .local_join(&json!({"name":"Gast","geraet":dev_a}), false)
            .unwrap();
        assert_eq!(again["spieler"], guest["spieler"]);
        // The host releases the name: old sessions end, the next device takes it over.
        g.local_route(
            &Method::Post,
            "/api/local/control",
            &owner,
            &json!({"name_freigeben":"Gast"}),
            None,
            true,
        )
        .unwrap();
        assert_eq!(
            g.authenticate(again["token"].as_str().unwrap()).unwrap_err().0,
            401
        );
        let moved = g
            .local_join(&json!({"name":"Gast","geraet":dev_b}), false)
            .unwrap();
        assert_eq!(moved["spieler"], guest["spieler"]);
        assert_eq!(
            g.local_join(&json!({"name":"Gast","geraet":dev_a}), false)
                .unwrap_err()
                .0,
            409
        );
        // Names of the host machine stay bound; guests cannot release or claim them.
        assert_eq!(
            g.local_route(
                &Method::Post,
                "/api/local/control",
                &owner,
                &json!({"name_freigeben":"Kapitaen"}),
                None,
                true
            )
            .unwrap_err()
            .0,
            409
        );
        assert_eq!(
            g.local_route(
                &Method::Post,
                "/api/local/control",
                moved["token"].as_str().unwrap(),
                &json!({"name_freigeben":"Kapitaen"}),
                None,
                false
            )
            .unwrap_err()
            .0,
            404
        );
        // Browser guests choose mode and people themselves.
        let browser = g
            .local_join(
                &json!({"name":"Browsergast","geraet":"c".repeat(40),"platz_waehlen":true}),
                false,
            )
            .unwrap();
        assert!(browser["spieler"].is_null(), "{browser}");
        let players = g.local_players().unwrap();
        let binding = |n: &str| {
            players["spieler"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["name"] == n)
                .unwrap()["bindung"]
                .clone()
        };
        assert_eq!(binding("Kapitaen"), "host");
        assert_eq!(binding("Gast"), "geraet");
        assert!(!players.to_string().contains(&digest(&dev_b)));
    }

    #[test]
    fn tempo_is_real_time_and_never_catches_up() {
        // 1×: nothing after 5 s, exactly one 30-second window after 30 s of wall time.
        let (mut rest, mut total) = (0.0, 0);
        for _ in 0..6 {
            let (s, r) = time_step(rest, 5.0, 1, 30);
            total += s;
            rest = r;
        }
        assert_eq!((total, rest), (30, 0.0));
        assert_eq!(time_step(0.0, 5.0, 1, 30).0, 0);
        assert_eq!(time_step(0.0, 5.0, 10, 30), (30, 20.0));
        assert_eq!(time_step(0.0, 5.0, 30, 30).0, 150);
        assert_eq!(time_step(0.0, 5.0, 60, 30).0, 300);
        // A suspended laptop counts as two ticks at most.
        assert_eq!(time_step(0.0, 3600.0, 60, 30).0, 600);
        assert_eq!(time_step(0.0, -1.0, 60, 30).0, 0);
    }

    #[test]
    fn bots_answer_letters_from_harness_agents_at_once() {
        let mut g = Game::open_world(&root(), 712, Some(30)).unwrap();
        g.local_join(&json!({"name":"Ratsagent","volk":"veyari","mode":"agent"}), true)
            .unwrap();
        let a = g.account_by_name("Ratsagent").unwrap();
        let key = g.harness_schluessel_erstellen(&a,&json!({"label":"Test","rollen":["diplomat"],"regeln_akzeptiert":crate::harness::REGELN_VERSION,"manifest":{"client":"test","version":"1","modelle":[{"rolle":"diplomat","anbieter":"ollama","modell":"test"}],"datenbank":"sqlite","werkzeuge":["lage"],"plugins":[]}})).unwrap()["schluessel"].as_str().unwrap().to_owned();
        let (a, s) = g.harness_authenticate(&key).unwrap();
        let lease = g
            .harness_freigabe(&a, &s, &json!({"aktion":"start"}))
            .unwrap();
        let target = g.world.spieler[0].name.clone();
        let v = g.harness_handeln(&a,&s,&json!({"request_id":"brief-harness","rolle":"diplomat","freigabe":lease["freigabe"],"aktionen":[{"typ":"brief_senden","kanal":"privat","an":target,"betreff":"Handel","text":"Wie handeln wir Erz?","antwort_auf":null}]})).unwrap();
        assert_eq!(v["ergebnisse"][0]["ok"], true, "{v}");
        let sid = a.sid.unwrap();
        let letter = g
            .world
            .nachrichten
            .iter()
            .rposition(|n| n.von == sid && n.an == vec![0])
            .unwrap() as u64
            + 1;
        let reply = g
            .world
            .kommunikation
            .briefe
            .values()
            .find(|b| b.antwort_auf == Some(letter))
            .expect("bot reply in the same transaction");
        assert!(!reply.automatisch, "a real answer, not the 1-hour receipt");
        assert!(g.world.nachrichten[reply.id as usize - 1]
            .text
            .contains("Ware, Menge und Gegenleistung"));
    }

    /// Pure time steps commit archives but checkpoint at a bounded rate. After a crash the
    /// world resumes from the checkpoint, the archive tail is removed, and the replay is
    /// identical to a run that checkpointed every step.
    #[test]
    fn throttled_checkpoints_recover_identically_after_a_crash() {
        let (crash, reference) = (root(), root());
        let mut a = Game::open_world(&crash, 712, Some(30)).unwrap();
        let mut b = Game::open_world(&reference, 712, Some(30)).unwrap();
        a.tick_checkpoint_secs = 86_400;
        b.tick_checkpoint_secs = 0;
        let start = a.world.zeit;
        for _ in 0..30 {
            a.advance(3600).unwrap();
            b.advance(3600).unwrap();
        }
        assert_eq!(a.world.hash(), b.world.hash());
        let tail = count(
            &a,
            "SELECT count(*) FROM game_actions WHERE world_id=?1 AND simulation_seconds>0",
        );
        assert!(tail > 0, "bots acted");
        drop(a); // crash: no final checkpoint
        let mut a = Game::open_world(&crash, 712, Some(30)).unwrap();
        assert_eq!(a.world.zeit, start);
        assert_eq!(
            count(&a, "SELECT count(*) FROM game_actions WHERE world_id=?1 AND simulation_seconds>0"),
            0
        );
        assert_eq!(
            count(&a, "SELECT count(*) FROM communication_events WHERE world_id=?1"),
            a.world.kommunikation.ereignisse_gesamt()
        );
        for _ in 0..30 {
            a.advance(3600).unwrap();
        }
        assert_eq!(a.world.hash(), b.world.hash());
        for sql in [
            "SELECT count(*) FROM game_actions WHERE world_id=?1",
            "SELECT count(*) FROM communication_events WHERE world_id=?1",
            "SELECT count(*) FROM communication_messages WHERE world_id=?1",
        ] {
            assert_eq!(count(&a, sql), count(&b, sql), "{sql}");
        }
        let (t, bytes): (i64, i64) = b
            .db
            .query_row(
                "SELECT simulation_seconds,world_bytes FROM checkpoint_info",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(t, b.world.zeit);
        assert!(bytes > 0);
    }

    /// Long local rounds keep only recent history in memory; the archive keeps all of it.
    #[test]
    fn long_local_rounds_keep_only_recent_history_in_memory() {
        let mut g = Game::open_world(&root(), 712, Some(30)).unwrap();
        for _ in 0..4 * 24 {
            g.advance(3600).unwrap();
        }
        let k = &g.world.kommunikation;
        let today = g.world.zeit - g.world.zeit.rem_euclid(TAG);
        assert!(k.ereignisse_ausgelagert > 0, "events were moved out");
        assert!(k
            .ereignisse
            .iter()
            .all(|e| e["time"].as_i64().unwrap() >= today - TAG));
        assert_eq!(
            count(&g, "SELECT count(*) FROM communication_events WHERE world_id=?1"),
            k.ereignisse_gesamt()
        );
        // A new letter continues the numbering after the archived events.
        let host = g
            .local_join(&json!({"name":"Chronist","volk":"veyari"}), true)
            .unwrap();
        let a = g.authenticate(host["token"].as_str().unwrap()).unwrap();
        let target = g.world.spieler[1].name.clone();
        let before = g.world.kommunikation.ereignisse_gesamt();
        g.command(&a,&json!({"world_id":g.runtime.world_id,"request_id":"chronik-1","aktionen":[{"typ":"brief_senden","kanal":"privat","an":target,"betreff":"Gruß","text":"Guten Tag.","antwort_auf":null}]})).unwrap();
        let first = &g.world.kommunikation.ereignisse[0];
        assert_eq!(
            first["id"].as_u64().unwrap(),
            g.world.kommunikation.ereignisse_ausgelagert + 1
        );
        assert!(g.world.kommunikation.ereignisse_gesamt() > before);
        assert_eq!(
            count(&g, "SELECT count(*) FROM communication_events WHERE world_id=?1"),
            g.world.kommunikation.ereignisse_gesamt()
        );
        g.checkpoint_now().unwrap();
        let (hash, root) = (g.world.hash(), g.data_root.clone());
        drop(g);
        let g = Game::open_world(&root, 712, Some(30)).unwrap();
        assert_eq!(hash, g.world.hash());
    }

    /// Live-path soak: the real transaction path of a local round (`advance` with throttled
    /// checkpoints, archive and retention), one game hour per transaction. Reports per game
    /// day: checkpoint bytes, events and letters in memory, transaction times. Explicit:
    /// `STERNENEPOCHE_SOAK_BOTS=400 STERNENEPOCHE_SOAK_DAYS=120 cargo test --release -p
    /// sternenepoche-server live_path_soak -- --ignored --nocapture`
    #[test]
    #[ignore = "long soak; run explicitly in release mode"]
    fn live_path_soak() {
        let env = |k: &str, d: u64| {
            std::env::var(k)
                .ok()
                .and_then(|n| n.parse().ok())
                .unwrap_or(d)
        };
        let (bots, days) = (env("STERNENEPOCHE_SOAK_BOTS", 30) as u16, env("STERNENEPOCHE_SOAK_DAYS", 365));
        let root = std::env::var("STERNENEPOCHE_SOAK_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root());
        let started = Instant::now();
        let mut g = Game::open_world(&root, 712, Some(bots)).unwrap();
        // One human-like host and a guest act every game day through the normal command path.
        for (name, dev) in [("Soakhost", None), ("Soakgast", Some("d".repeat(40)))] {
            let body = match &dev {
                Some(d) => json!({"name":name,"geraet":d,"volk":"krath"}),
                None => json!({"name":name,"volk":"veyari"}),
            };
            g.local_join(&body, dev.is_none()).unwrap();
        }
        let mut times: Vec<f64> = Vec::new();
        let mut report = Vec::new();
        let out = root.join("soak.jsonl");
        for day in (g.world.zeit / TAG) as u64..days {
            for hour in 0..24 {
                if hour == 12 {
                    for name in ["Soakhost", "Soakgast"] {
                        let a = g.account_by_name(name).unwrap();
                        let sid = a.sid.unwrap();
                        let target = g.world.spieler[(day % bots.max(1) as u64) as usize].name.clone();
                        let p = g.world.spieler[sid as usize].heimat as usize;
                        let mut actions = vec![json!({"typ":"brief_senden","kanal":"privat","an":target,"betreff":"Handel","text":"Ich suche Erz. Was bietest du?","antwort_auf":null})];
                        if let Some(b) = lauf::bots::waehle_bau(&g.world, sid, p, Bottyp::Oekonom) {
                            actions.push(json!({"typ":"bauen","planet":g.world.planeten[p].koord,"gebaeude":b.name()}));
                        }
                        let t = Instant::now();
                        g.command(&a, &json!({"world_id":g.runtime.world_id,"request_id":format!("soak-{name}-{day}"),"aktionen":actions})).unwrap();
                        times.push(t.elapsed().as_secs_f64() * 1000.0);
                    }
                }
                let t = Instant::now();
                g.advance(3600).unwrap();
                times.push(t.elapsed().as_secs_f64() * 1000.0);
            }
            let (checkpoint, world_bytes): (i64, i64) = g
                .db
                .query_row(
                    "SELECT simulation_seconds,world_bytes FROM checkpoint_info",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            let mut sorted = times.clone();
            sorted.sort_by(f64::total_cmp);
            let pct = |q: f64| sorted.get(((sorted.len() as f64 - 1.0) * q) as usize).copied();
            let db_bytes: u64 = ["spiel.sqlite3", "spiel.sqlite3-wal"]
                .iter()
                .map(|f| std::fs::metadata(root.join(f)).map(|m| m.len()).unwrap_or(0))
                .sum();
            let line = json!({"bots":bots,"tag":day+1,"wall_s":started.elapsed().as_secs_f64(),
                "checkpoint_tag":checkpoint/TAG,"checkpoint_bytes":world_bytes,"welt_bytes_jetzt":g.world.zu_bytes().len(),
                "ereignisse_speicher":g.world.kommunikation.ereignisse.len(),"ereignisse_gesamt":g.world.kommunikation.ereignisse_gesamt(),
                "briefe":g.world.nachrichten.len(),"kampfberichte":g.world.kampfberichte.len(),"tageswerte":g.world.tageswerte.len(),
                "transaktion_ms_p50":pct(0.5),"transaktion_ms_p95":pct(0.95),"transaktion_ms_max":pct(1.0),"db_bytes":db_bytes,
                "stufen":(1..=5).map(|s|g.world.spieler.iter().filter(|p|g.world.spieler_aktiv(p.id)&&p.stufe==s).count()).collect::<Vec<_>>(),
                "spionage_bemerkt_7_tage_max_je_ziel":g.world.spieler.iter().map(|p|p.vorfaelle.iter().filter(|v|v.art=="spionage"&&v.text.starts_with("Spionage durch")).count()).max(),
                "spionage_bemerkt_7_tage_summe":g.world.spieler.iter().map(|p|p.vorfaelle.iter().filter(|v|v.art=="spionage"&&v.text.starts_with("Spionage durch")).count()).sum::<usize>(),
                "botaktionen":g.runtime.bot_actions});
            println!("SOAK {line}");
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&out).unwrap();
            writeln!(f, "{line}").unwrap();
            report.push(line);
            times.clear();
            assert!(g.admin_status()["invariant_errors"].as_array().unwrap().is_empty());
        }
        g.checkpoint_now().unwrap();
        let hash = g.world.hash();
        drop(g);
        assert_eq!(hash, Game::open_world(&root, 712, Some(bots)).unwrap().world.hash());
    }

    /// Reserved start places receive probes too. Their chronicle obeys the same 7-day limit
    /// as every other seat (before, it grew for the whole epoch: megabytes per world).
    #[test]
    fn reserved_seats_keep_only_the_chronicle_window() {
        let mut g = Game::open_world(&root(), 712, Some(30)).unwrap();
        let frei = *g.world.aufklaerung.inaktive_spieler.iter().next().unwrap();
        for tag in 0..3 {
            g.world.spieler[frei as usize].vorfaelle.push(kern::welt::Vorfall {
                zeit: tag * TAG,
                art: "spionage".into(),
                text: "Spionage durch Test auf 1:1:1 bemerkt".into(),
            });
        }
        for _ in 0..(10 * 24) {
            g.advance(3600).unwrap();
        }
        let grenze = g.world.zeit - g.world.regeln.agenten.chronik_tage * TAG - TAG;
        assert!(!g.world.spieler_aktiv(frei));
        assert!(g.world.spieler[frei as usize]
            .vorfaelle
            .iter()
            .all(|v| v.zeit >= grenze));
    }

    #[test]
    fn qr_code_carries_only_the_address() {
        let url = "http://192.168.1.20:8892";
        let svg = qr_svg(url).unwrap();
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        // Version 2 (25 modules) plus the quiet zone of four modules on each side.
        let qr = qrcodegen::QrCode::encode_text(url, qrcodegen::QrCodeEcc::Medium).unwrap();
        assert!(svg.contains(&format!("viewBox=\"0 0 {s} {s}\"", s = qr.size() + 8)));
        assert!(svg.matches('M').count() > 100);
        assert!(!svg.contains("8892") && !svg.to_lowercase().contains("pass"));
        if std::env::var("QR_ZEIGEN").is_ok() {
            println!("QR {svg}");
        }
    }

    /// The local client must not depend on files only the online server ships.
    #[test]
    fn every_file_of_the_client_page_is_served_locally() {
        let html = include_str!("../../../web-client/index.html");
        let mut missing = Vec::new();
        for attr in ["src=\"", "href=\""] {
            for part in html.split(attr).skip(1) {
                let target = part.split('"').next().unwrap();
                if target.starts_with("http") || target.starts_with('#') || target == "./" || target == "config.js" {
                    continue;
                }
                let path = format!("/{}", target.split(['?', '#']).next().unwrap());
                if client_asset(&path).is_none() {
                    missing.push(path);
                }
            }
        }
        assert!(missing.is_empty(), "{missing:?}");
    }

    /// Real engine/event queue; accelerated decision windows, no fabricated end state.
    #[test]
    fn all_five_levels_execute_bot_policies_and_restore() {
        for level in 1..=5 {
            let root = root();
            let mut g = Game::open_world(&root, 712, Some(30)).unwrap();
            std::sync::Arc::make_mut(&mut g.world.regeln)
                .welt
                .fenster_sekunden = 300;
            // Explicit advanced-tier fixtures complement natural progression in the year run.
            for sid in 0..30 {
                let p = g.world.spieler[sid].heimat as usize;
                g.world.spieler[sid].stufe = level;
                g.world.spieler[sid].credits = 1_000_000 * M;
                g.world.planeten[p].bestand.fill(100_000 * M);
                for b in Gebaeude::ALLE {
                    if g.world.regeln.geb(b).ab_stufe <= level {
                        g.world.planeten[p].gebaeude[b.idx()] = 3;
                    }
                }
                for f in Forschung::ALLE {
                    if g.world.regeln.forsch(f).ab_stufe <= level {
                        g.world.spieler[sid].forschung[f.idx()] = 3;
                    }
                }
                g.world.raten_neu(p);
            }
            g.db.execute_batch("BEGIN IMMEDIATE").unwrap();
            for _ in 0..240 {
                g.advance_steps(3600).unwrap();
            }
            assert!(
                g.world.spieler[..30]
                    .iter()
                    .all(|s| s.statistik.aktionen > 10),
                "Tier {level}"
            );
            assert!(
                g.admin_status()["invariant_errors"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                "Tier {level}"
            );
            g.save().unwrap();
            g.db.execute_batch("COMMIT").unwrap();
            let hash = g.world.hash();
            drop(g);
            assert_eq!(
                hash,
                Game::open_world(&root, 712, Some(30)).unwrap().world.hash()
            );
        }
    }

    /// Real engine/event queue; accelerated decision windows, no fabricated end state.
    #[test]
    #[ignore = "365-day stress run; run explicitly in release mode"]
    fn annual_host_two_guests_and_pure_agent() {
        let bots = std::env::var("STERNENEPOCHE_SOAK_BOTS")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(30);
        let root = std::env::var("STERNENEPOCHE_SOAK_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root());
        let started = Instant::now();
        let mut g = Game::open_world(&root, 712, Some(bots)).unwrap();
        // An interrupted replay may have committed archive hours after its last
        // checkpoint. Remove only that test-world tail before replaying those hours.
        g.db.execute(
            "DELETE FROM game_actions WHERE world_id=?1 AND simulation_seconds>?2",
            params![g.runtime.world_id, g.world.zeit],
        )
        .unwrap();
        g.db.execute(
            "DELETE FROM game_domain_events WHERE world_id=?1 AND simulation_seconds>?2",
            params![g.runtime.world_id, g.world.zeit],
        )
        .unwrap();
        g.db.execute(
            "DELETE FROM communication_events WHERE world_id=?1 AND event_id>?2",
            params![g.runtime.world_id, g.world.kommunikation.ereignisse_gesamt()],
        )
        .unwrap();
        g.db.execute(
            "DELETE FROM communication_messages WHERE world_id=?1 AND message_id>?2",
            params![g.runtime.world_id, g.world.nachrichten.len()],
        )
        .unwrap();
        std::sync::Arc::make_mut(&mut g.world.regeln)
            .welt
            .fenster_sekunden = 300;
        let mut users = Vec::new();
        for (name, mode) in [
            ("Testhost", "gemischt"),
            ("MitspielerEins", "mensch"),
            ("MitspielerZwei", "mensch"),
            ("Agentenspieler", "agent"),
        ] {
            let v = g
                .local_join(&json!({"name":name,"volk":"veyari"}), true)
                .unwrap();
            let a = g.authenticate(v["token"].as_str().unwrap()).unwrap();
            g.claim(&a, &json!({"mode":mode,"volk":"veyari"})).unwrap();
            users.push(g.account_by_name(name).unwrap());
        }
        // Scripted controllers replace the three humans only in this test.
        for (i, a) in users[..3].iter().enumerate() {
            g.runtime
                .bots
                .entry(a.sid.unwrap())
                .or_insert_with(|| lauf::bots::Bot::neu(Bottyp::ALLE[i], a.sid.unwrap()));
        }
        let a = users[3].clone();
        let sid = a.sid.unwrap();
        // A resumed fixture cannot recover a previously issued raw key. Revoke
        // only this synthetic controller's old keys through the normal API.
        let old_keys = g.harness_schluessel_liste(&a).unwrap();
        for key in old_keys["schluessel"].as_array().unwrap() {
            if key["label"] == "Jahrestest" && key["widerrufen"] == false {
                g.harness_schluessel_widerrufen(&a, &json!({"id":key["id"]}))
                    .unwrap();
            }
        }
        let key=g.harness_schluessel_erstellen(&a,&json!({"label":"Jahrestest","rollen":["verwalter"],"regeln_akzeptiert":crate::harness::REGELN_VERSION,"manifest":{"client":"soak-model-fixture","version":"1","modelle":[{"rolle":"verwalter","anbieter":"regel","modell":"deterministische-testantwort"}],"datenbank":"sqlite","werkzeuge":["lage"],"plugins":[]}})).unwrap()["schluessel"].as_str().unwrap().to_owned();
        let (_, s) = g.harness_authenticate(&key).unwrap();
        let mut calls =
            g.db.query_row(
                "SELECT count(*) FROM commands WHERE account=?1 AND request_id LIKE 'annual-%'",
                [a.id],
                |r| r.get::<_, u64>(0),
            )
            .unwrap();
        let mut accepted=g.db.query_row("SELECT coalesce(sum(json_extract(j.value,'$.ok')=1),0) FROM commands c,json_each(c.result,'$.ergebnisse') j WHERE c.account=?1 AND c.request_id LIKE 'annual-%'",[a.id],|r|r.get::<_,u64>(0)).unwrap();
        let mut levels = std::collections::BTreeSet::from([1]);
        for hour in (g.world.zeit / 3600)..365 * 24 {
            let already_recorded =
                g.db.query_row(
                    "SELECT count(*) FROM commands WHERE account=?1 AND request_id=?2",
                    params![a.id, format!("annual-{hour}")],
                    |r| r.get::<_, u64>(0),
                )
                .unwrap()
                    > 0;
            if hour % 4 == 0
                && !already_recorded
                && !g.world.beendet()
                && g.world.spieler_aktiv(sid)
            {
                let _ = g.harness_freigabe(&a, &s, &json!({"aktion":"stop"}));
                let lease = g
                    .harness_freigabe(&a, &s, &json!({"aktion":"start"}))
                    .unwrap();
                let p = g.world.spieler[sid as usize].heimat as usize;
                let action=lauf::bots::waehle_bau(&g.world,sid,p,Bottyp::Oekonom).map(|b|json!({"typ":"bauen","planet":g.world.planeten[p].koord,"gebaeude":b.name()}));
                let actions = action.into_iter().collect::<Vec<_>>();
                let v=g.harness_handeln(&a,&s,&json!({"request_id":format!("annual-{hour}"),"rolle":"verwalter","freigabe":lease["freigabe"],"aktionen":actions,"begruendung":"Deterministische Modellantwort für wiederholbaren Langzeittest"})).unwrap();
                calls += 1;
                accepted += v["ergebnisse"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| v["ok"] == true)
                    .count() as u64;
            }
            // Batch the real action archive for one replay hour. Without a transaction
            // each bot event forces a separate disk flush; live ticks already batch them.
            g.db.execute_batch("BEGIN IMMEDIATE").unwrap();
            g.advance_steps(3600).unwrap();
            if hour % 24 == 23 {
                g.save().unwrap();
            }
            g.db.execute_batch("COMMIT").unwrap();
            for sp in &g.world.spieler {
                if g.world.spieler_aktiv(sp.id) {
                    levels.insert(sp.stufe);
                }
            }
            if hour % 720 == 719 || hour == 8759 {
                assert!(g.admin_status()["invariant_errors"]
                    .as_array()
                    .unwrap()
                    .is_empty());
                println!(
                    "ANNUAL {}",
                    json!({"bots":bots,"tag":g.world.zeit/TAG,"wall_seconds":started.elapsed().as_secs_f64(),"aktionen":g.runtime.bot_actions,"agent_calls":calls,"levels":levels,"root":root})
                );
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }
        assert_eq!(g.world.zeit, 365 * TAG);
        assert!(calls > 100);
        assert!(accepted > 0);
        assert!(g.world.spieler[..bots as usize]
            .iter()
            .all(|s| s.statistik.aktionen > 30));
        assert!(g.runtime.bot_trace.values().all(|v| v.len() <= 20));
        assert!(g.runtime.public_history.len() <= 168);
        assert_eq!(
            g.db.query_row(
                "SELECT count(*) FROM communication_events WHERE world_id=?1",
                [&g.runtime.world_id],
                |r| r.get::<_, usize>(0)
            )
            .unwrap(),
            g.world.kommunikation.ereignisse_gesamt() as usize
        );
        assert_eq!(
            g.db.query_row(
                "SELECT count(*) FROM communication_messages WHERE world_id=?1",
                [&g.runtime.world_id],
                |r| r.get::<_, usize>(0)
            )
            .unwrap(),
            g.world.nachrichten.len()
        );
        let hash = g.world.hash();
        let actions = g.runtime.bot_actions;
        g.save().unwrap();
        drop(g);
        let restored = Game::open_world(&root, 712, Some(bots)).unwrap();
        assert_eq!(hash, restored.world.hash());
        assert_eq!(actions, restored.runtime.bot_actions);
        let report = json!({"passed":true,"days":365,"bots":bots,"simulated_humans":3,"pure_agents":1,"model":"deterministic fixture through real harness auth/lease/action API","decision_window_seconds":300,"levels_reached":levels,"bot_actions":actions,"agent_calls":calls,"agent_accepted":accepted,"wall_seconds":started.elapsed().as_secs_f64(),"world_hash":hash});
        std::fs::write(
            root.join("annual-report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        println!("ANNUAL_FINAL {report}");
    }

    /// Run explicitly on marked COPIES of annual fixtures, never on production data.
    #[test]
    #[ignore = "Requires a marked copy of a completed 365-day fixture"]
    fn mature_year_uses_production_transactions_and_restores() {
        let root = PathBuf::from(std::env::var("STERNENEPOCHE_YEAR_COPY").unwrap());
        assert!(root.join("annual-copy.marker").is_file());
        let bots: u16 = std::env::var("STERNENEPOCHE_SOAK_BOTS").unwrap().parse().unwrap();
        let mut g = Game::open_world(&root, 712, Some(bots)).unwrap();
        assert_eq!(g.world.zeit, 365 * TAG);
        assert!(g.admin_status()["invariant_errors"].as_array().unwrap().is_empty());
        let archived = count(&g,"SELECT count(*) FROM communication_events WHERE world_id=?1");
        assert_eq!(archived,g.world.kommunikation.ereignisse_gesamt());
        let start_hash = g.world.hash();
        // The real year ends at day 365. Extend ONLY this marked benchmark copy;
        // otherwise advance() would correctly be a no-op in a finished epoch.
        std::sync::Arc::make_mut(&mut g.world.regeln).welt.epoche_tage = 730;
        std::sync::Arc::make_mut(&mut g.world.regeln).welt.fenster_sekunden = 30;
        g.runtime.paused = false;
        assert!(!g.world.beendet());
        let mut ticks = Vec::new();
        // Production transaction path: rollback copy, archives and bounded checkpoints.
        for _ in 0..240 {
            let now = Instant::now();
            g.advance(30).unwrap();
            ticks.push(now.elapsed().as_secs_f64() * 1000.0);
        }
        assert!(g.admin_status()["invariant_errors"].as_array().unwrap().is_empty());
        g.checkpoint_now().unwrap();
        let hash = g.world.hash();
        let seconds = g.world.zeit;
        assert_eq!(seconds,365 * TAG + 7200,"All 240 ticks must advance the mature world");
        let retained = g.world.kommunikation.ereignisse.len();
        drop(g);
        let restored = Game::open_world(&root, 712, Some(bots)).unwrap();
        assert_eq!(hash,restored.world.hash());
        assert_eq!(seconds,restored.world.zeit);
        assert_eq!(count(&restored,"SELECT count(*) FROM communication_events WHERE world_id=?1"),restored.world.kommunikation.ereignisse_gesamt());
        ticks.sort_by(f64::total_cmp);
        let result = json!({"passed":true,"bots":bots,"year_loaded":true,"epoch_extended_in_benchmark_copy":true,"production_transactions":240,"seconds":seconds,"tick_p50_ms":ticks[120],"tick_p95_ms":ticks[227],"tick_max_ms":ticks[239],"start_hash":start_hash,"restored_hash":hash,"retained_communication_events":retained,"archived_events_at_start":archived});
        std::fs::write(root.join("current-runtime-report.json"),serde_json::to_vec_pretty(&result).unwrap()).unwrap();
        println!("CURRENT_YEAR {result}");
    }
}
