use fs2::FileExt;
use kern::typen::Rolle;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Read,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use sternenepoche_server::{
    digest, eingabe, harness, mcp, now, password_hash, password_matches, token, Game, Reply, API_VERSION,
};
use tiny_http::{Header, Method, Request, Response, Server};
mod kaefig;
mod public_catalog;
mod web_assets;

/// Woher die echte Client-Adresse kommt. Nie aus einem Header, den der Client selbst setzen kann.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Proxy {
    /// Direkter Betrieb: Socket-Adresse.
    Keiner,
    /// Caddy auf dem VPS überschreibt X-Real-IP mit {remote_host}.
    RealIp,
    /// Tailscale Serve/Funnel überschreibt X-Forwarded-For mit der Quelladresse;
    /// X-Real-IP reicht Tailscale ungeprüft durch und wird deshalb ignoriert.
    Tailscale,
}

struct Config {
    bind: String,
    admin_bind: String,
    root: PathBuf,
    origins: Vec<String>,
    proxy: Proxy,
    kaefig_mb: usize,
    timeout_s: u64,
}
fn args() -> Result<Config, String> {
    let mut c = Config {
        bind: "127.0.0.1:8890".into(),
        admin_bind: "127.0.0.1:8891".into(),
        root: "data/online".into(),
        origins: vec![
            "https://sternenepoche.github.io".into(),
            "http://127.0.0.1:8890".into(),
            "http://localhost:8890".into(),
        ],
        proxy: Proxy::Keiner,
        kaefig_mb: 4096,
        timeout_s: 20,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bind" => c.bind = args.next().ok_or("Adresse fehlt")?,
            "--admin-bind" => c.admin_bind = args.next().ok_or("Adminadresse fehlt")?,
            "--data" => c.root = args.next().ok_or("Datenverzeichnis fehlt")?.into(),
            "--origin" => c.origins.push(args.next().ok_or("Origin fehlt")?),
            "--trusted-proxy" => c.proxy = Proxy::RealIp,
            "--proxy-tailscale" => c.proxy = Proxy::Tailscale,
            "--speicher-mb" => c.kaefig_mb = args.next().and_then(|n| n.parse().ok()).filter(|n| *n >= 256).ok_or("--speicher-mb: Zahl ab 256")?,
            "--ohne-kaefig" => c.kaefig_mb = 0,
            "--help" => {
                println!("sternenepoche-server [--data PFAD] [--bind 127.0.0.1:8890] [--admin-bind 127.0.0.1:8891] [--origin https://spiel.example] [--trusted-proxy | --proxy-tailscale] [--speicher-mb 4096 | --ohne-kaefig]\nBeide Listener bleiben lokal. Öffentlicher Zugriff erfolgt über HTTPS-Reverse-Proxy. Adminzugriff beim VPS über SSH-Tunnel.\n--trusted-proxy: Caddy setzt X-Real-IP. --proxy-tailscale: Tailscale Funnel setzt X-Forwarded-For.");
                std::process::exit(0)
            }
            _ => return Err(format!("Unbekanntes Argument: {a}")),
        }
    }
    for s in [&c.bind, &c.admin_bind] {
        let a: SocketAddr = s.parse().map_err(|_| "Adresse mit numerischer IP und Port angeben")?;
        if !a.ip().is_loopback() {
            return Err("Listener nur auf Loopback; für Internetbetrieb HTTPS-Proxy davor schalten".into());
        }
    }
    if c.bind == c.admin_bind {
        return Err("Spiel und Verwaltung brauchen eigene Ports".into());
    }
    c.origins.push(format!("http://{}", c.bind));
    Ok(c)
}
fn header(req: &Request, name: &'static str) -> String {
    req.headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.to_string())
        .unwrap_or_default()
}
/// Antwortet niemals mit Software- oder Rechnernamen.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self' https://openrouter.ai http://127.0.0.1:* http://localhost:*; img-src 'self' data:; font-src 'self'; media-src 'self'; object-src 'none'; frame-src 'none'; worker-src 'none'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'";
fn answer(req: Request, status: u16, body: Vec<u8>, mime: &str, origin: &str, extra: &[(&'static str, String)]) {
    let mut r = Response::from_data(body).with_status_code(status);
    let cache = if status == 200 && mime == "image/webp" { "public, max-age=86400" } else { "no-store" };
    for (k, v) in [("Content-Type", mime), ("Cache-Control", cache), ("X-Content-Type-Options", "nosniff"), ("Referrer-Policy", "no-referrer"), ("Cross-Origin-Resource-Policy", "same-site"),
        ("Cross-Origin-Opener-Policy", "same-origin"), ("Permissions-Policy", "camera=(), microphone=(), geolocation=(), usb=(), payment=()"), ("Server", "sternenepoche"), ("Content-Security-Policy", CSP)] {
        r.add_header(Header::from_bytes(k, v).unwrap());
    }
    for (k, v) in extra {
        if let Ok(h) = Header::from_bytes(k.as_bytes(), v.as_bytes()) { r.add_header(h); }
    }
    if !origin.is_empty() {
        r.add_header(Header::from_bytes("Access-Control-Allow-Origin", origin).unwrap());
        r.add_header(Header::from_bytes("Vary", "Origin").unwrap());
        r.add_header(Header::from_bytes("Access-Control-Allow-Headers", "Authorization, Content-Type, X-Sternenepoche-Admin, X-Sternenepoche-Oberflaeche, Mcp-Protocol-Version").unwrap());
        r.add_header(Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap());
        r.add_header(Header::from_bytes("Access-Control-Expose-Headers", "Retry-After, X-Sternenepoche-Quote").unwrap());
    }
    let _ = req.respond(r);
}
fn json_status(req: Request, code: u16, v: Value, origin: &str, extra: &[(&'static str, String)]) {
    answer(req, code, v.to_string().into_bytes(), "application/json; charset=utf-8", origin, extra);
}
fn json_reply_mit(req: Request, result: Reply, origin: &str, extra: &[(&'static str, String)]) {
    let (code, v) = match result {
        Ok(v) => (200, v),
        Err((code, e)) => (code, json!({"error":e})),
    };
    json_status(req, code, v, origin, extra);
}
fn json_reply(req: Request, result: Reply, origin: &str) {
    json_reply_mit(req, result, origin, &[]);
}
type Limits = Arc<Mutex<HashMap<String, (i64, u32)>>>;
/// Zählt eine Anfrage im Minutenfenster. Liefert den Rest, solange das Limit nicht überschritten ist.
fn rest(limits: &Limits, key: String, max: u32) -> Option<u32> {
    let mut map = limits.lock().unwrap();
    let minute = now() / 60;
    map.retain(|_, v| v.0 >= minute - 1);
    // Bounding random-token/address churn also bounds the rate-limit table.
    if map.len() >= 10000 && !map.contains_key(&key) {
        return None;
    }
    let e = map.entry(key).or_insert((minute, 0));
    if e.0 != minute {
        *e = (minute, 0)
    }
    e.1 += 1;
    (e.1 <= max).then(|| max - e.1)
}
fn allowed(limits: &Limits, key: String, max: u32) -> bool {
    rest(limits, key, max).is_some()
}
fn request_allowed(limits: &Limits, ip: &str, authorization: &str) -> bool {
    // Random Authorization headers must not create an unlimited sequence of fresh budgets.
    // Keep enough headroom for several legitimate players behind the same NAT.
    allowed(limits, format!("ip:{ip}"), 6000)
        && allowed(limits, format!("requests:{ip}:{}", digest(authorization)), 300)
}
fn bis_naechste_minute() -> String {
    (60 - now() % 60).to_string()
}
/// Quote je Konto. Browser und Harness teilen sich dieselben Töpfe.
fn quote_pruefen(limits: &Limits, konto: i64, topf: &'static str, extra: &mut Vec<(&'static str, String)>) -> Result<(), (u16, String)> {
    let max = harness::quote(topf);
    match rest(limits, format!("q:{konto}:{topf}"), max) {
        Some(r) => {
            extra.push(("X-Sternenepoche-Quote", format!("{topf}={r}/{max}")));
            Ok(())
        }
        None => {
            extra.push(("Retry-After", bis_naechste_minute()));
            extra.push(("X-Sternenepoche-Quote", format!("{topf}=0/{max}")));
            Err((429, format!("Quote „{topf}“ erschöpft ({max} je Minute für Mensch und Harness gemeinsam); nach Retry-After erneut versuchen")))
        }
    }
}
/// Minimales Lesen eines Abfrageparameters mit Prozentdekodierung.
fn param(query: &str, key: &str) -> Option<String> {
    query.split('&').filter_map(|p| p.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| {
        let b = v.as_bytes();
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'+' => { out.push(b' '); i += 1 }
                b'%' if i + 2 < b.len() => match ((b[i + 1] as char).to_digit(16), (b[i + 2] as char).to_digit(16)) {
                    (Some(h), Some(l)) => { out.push((h * 16 + l) as u8); i += 3 }
                    _ => { out.push(b'%'); i += 1 }
                },
                c => { out.push(c); i += 1 }
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    })
}
fn client_ip(req: &Request, cfg: &Config, admin: bool) -> String {
    let socket = req.remote_addr().map(|a| a.ip());
    let geprueft = |v: &str| v.trim().parse::<IpAddr>().ok().map(|ip| ip.to_string());
    match (cfg.proxy, admin) {
        (Proxy::RealIp, false) => geprueft(&header(req, "X-Real-IP")).unwrap_or_else(|| "proxy".into()),
        (Proxy::Tailscale, false) if socket.is_some_and(|a| a.is_loopback()) => {
            let xff = header(req, "X-Forwarded-For");
            xff.rsplit(',').next().and_then(geprueft).unwrap_or_else(|| socket.map(|a| a.to_string()).unwrap_or_default())
        }
        _ => socket.map(|a| a.to_string()).unwrap_or_default(),
    }
}

#[allow(clippy::too_many_arguments)]
fn harness_route(g: &mut Game, method: &Method, path: &str, query: &str, raw: &str, body: &Value, limits: &Limits, extra: &mut Vec<(&'static str, String)>) -> Reply {
    if *method == Method::Get && path == "/api/harness/v1/info" {
        let mut v = g.harness_info();
        v["katalog_hinweis"] = json!("Öffentlicher Regelkatalog: GET /api/rules");
        return Ok(v);
    }
    // Schlüsselverwaltung nur mit Browsersitzung: ein Harness kann sich keine Rechte nachlegen.
    if path.starts_with("/api/harness/v1/schluessel") {
        let a = g.authenticate(raw)?;
        if g.fairplay_wartet_auf_bestaetigung(a.id) { return Err((403, "Fair-Play-Verwarnung zuerst bestätigen".into())); }
        quote_pruefen(limits, a.id, "meta", extra)?;
        if let Some(t) = g.fairplay_pruefen(body, query) { return Err(g.fairplay_verstoss(&a, &t, path)); }
        return match (method, path) {
            (Method::Get, "/api/harness/v1/schluessel") => g.harness_schluessel_liste(&a),
            (Method::Post, "/api/harness/v1/schluessel") => g.harness_schluessel_erstellen(&a, body),
            (Method::Post, "/api/harness/v1/schluessel/widerrufen") => g.harness_schluessel_widerrufen(&a, body),
            _ => Err((404, "Nicht gefunden".into())),
        };
    }
    let (a, s) = g.harness_authenticate(raw)?;
    if g.fairplay_wartet_auf_bestaetigung(a.id) { return Err((403, "Fair-Play-Verwarnung zuerst im Browser bestätigen".into())); }
    let topf = if path == "/api/harness/v1/mcp" { mcp::topf(body) } else { harness::quote_fuer_pfad(path) };
    quote_pruefen(limits, a.id, topf, extra)?;
    if let Some(t) = g.fairplay_pruefen(body, query) { return Err(g.fairplay_verstoss(&a, &t, path)); }
    match (method, path) {
        (Method::Get, "/api/harness/v1/ich") => Ok(g.harness_ich(&a, &s)),
        (Method::Get, "/api/harness/v1/schema") => g.harness_schema(&s, param(query, "rolle").as_deref()).map(|mut v| { v["katalog"] = public_catalog::catalog(&g.world.regeln); v }),
        (Method::Get, "/api/harness/v1/lage") => g.harness_lage(&a, &s, param(query, "rolle").as_deref()),
        (Method::Post, "/api/harness/v1/werkzeug") => g.harness_werkzeug(&a, body),
        (Method::Post, "/api/harness/v1/freigabe") => g.harness_freigabe(&a, &s, body),
        (Method::Post, "/api/harness/v1/handeln") => g.harness_handeln(&a, &s, body),
        (Method::Post, "/api/harness/v1/bericht") => g.harness_bericht(&a, &s, body),
        _ => Err((404, "Unbekannter Harness-Endpunkt; siehe GET /api/harness/v1/info".into())),
    }
}

fn serve(
    mut req: Request,
    admin: bool,
    game: &Arc<Mutex<Game>>,
    cfg: &Config,
    admin_key: &str,
    limits: &Limits,
) {
    let url = req.url().to_string();
    let (path, query) = match url.split_once('?') { Some((p, q)) => (p.to_string(), q.to_string()), None => (url.clone(), String::new()) };
    let origin = header(&req, "Origin");
    let host = header(&req, "Host");
    let local_admin = [
        cfg.admin_bind.clone(),
        cfg.admin_bind.replace("127.0.0.1", "localhost"),
    ];
    if admin
        && (!local_admin.contains(&host)
            || !req.remote_addr().is_some_and(|a| a.ip().is_loopback()))
    {
        json_reply(req, Err((403, "Verwaltung nur über lokale Adresse oder SSH-Tunnel".into())), "");
        return;
    }
    let origin_allowed = origin.is_empty()
        || if admin {
            local_admin.iter().any(|h| origin == format!("http://{h}"))
        } else {
            cfg.origins.contains(&origin)
        };
    if !origin_allowed {
        json_reply(req, Err((403, "Diese Website ist nicht für den Server freigegeben".into())), "");
        return;
    }
    if req.method() == &Method::Options {
        answer(req, 204, vec![], "application/json", &origin, &[]);
        return;
    }
    let ip = client_ip(&req, cfg, admin);
    if !request_allowed(limits, &ip, &header(&req, "Authorization")) {
        json_reply_mit(req, Err((429, "Zu viele Anfragen; in einer Minute erneut versuchen".into())), &origin, &[("Retry-After", bis_naechste_minute())]);
        return;
    }
    let method = req.method().clone();
    if method == Method::Get {
        if !admin && (path == "/" || path == "/index.html") {
            // Leser ohne JavaScript bekommen die verborgenen Fair-Play-Köder direkt im HTML.
            let html = include_str!("../../../web-client/index.html");
            let koeder = game.lock().unwrap().oberflaeche_html();
            answer(req, 200, html.replacen("<!--sternenepoche:oberflaeche-->", &koeder, 1).into_bytes(), "text/html; charset=utf-8", &origin, &[]);
            return;
        }
        let asset = match (admin, path.as_str()) {
            (false, "/app.js") => Some((include_bytes!("../../../web-client/app.js").as_slice(), "text/javascript; charset=utf-8")),
            (false,"/presentation.js")=>Some((include_bytes!("../../../web-client/presentation.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/ai-labels.js")=>Some((include_bytes!("../../../web-client/ai-labels.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/game-ui.js")=>Some((include_bytes!("../../../web-client/game-ui.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/fairplay-ui.js")=>Some((include_bytes!("../../../web-client/fairplay-ui.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/harness-regeln.html")=>Some((include_bytes!("../../../web-client/harness-regeln.html").as_slice(),"text/html; charset=utf-8")),
            (false,"/guide/schreiben.webp")=>Some((include_bytes!("../../../web-client/guide/schreiben.webp").as_slice(),"image/webp")),
            (false,"/guide/chat.webp")=>Some((include_bytes!("../../../web-client/guide/chat.webp").as_slice(),"image/webp")),
            (false,"/guide/anfragen.webp")=>Some((include_bytes!("../../../web-client/guide/anfragen.webp").as_slice(),"image/webp")),
            (false,"/guide/gruenden.webp")=>Some((include_bytes!("../../../web-client/guide/gruenden.webp").as_slice(),"image/webp")),
            (false,"/guide/briefkasten.webp")=>Some((include_bytes!("../../../web-client/guide/briefkasten.webp").as_slice(),"image/webp")),
            (false,"/guide/allianz.webp")=>Some((include_bytes!("../../../web-client/guide/allianz.webp").as_slice(),"image/webp")),
            (false,"/guide/wirtschaft.webp")=>Some((include_bytes!("../../../web-client/guide/wirtschaft.webp").as_slice(),"image/webp")),
            (false,"/guide/hilfe.webp")=>Some((include_bytes!("../../../web-client/guide/hilfe.webp").as_slice(),"image/webp")),
            (false,"/guide/allianzpost.webp")=>Some((include_bytes!("../../../web-client/guide/allianzpost.webp").as_slice(),"image/webp")),
            (false,"/guide/team-01-modelle.webp")=>Some((include_bytes!("../../../web-client/guide/team-01-modelle.webp").as_slice(),"image/webp")),
            (false,"/guide/team-02-ollama.webp")=>Some((include_bytes!("../../../web-client/guide/team-02-ollama.webp").as_slice(),"image/webp")),
            (false,"/guide/team-03-openrouter.webp")=>Some((include_bytes!("../../../web-client/guide/team-03-openrouter.webp").as_slice(),"image/webp")),
            (false,"/guide/team-04-rollen.webp")=>Some((include_bytes!("../../../web-client/guide/team-04-rollen.webp").as_slice(),"image/webp")),
            (false,"/guide/team-05-browserfreigabe.webp")=>Some((include_bytes!("../../../web-client/guide/team-05-browserfreigabe.webp").as_slice(),"image/webp")),
            (false,"/guide/team-06-pinwaende.webp")=>Some((include_bytes!("../../../web-client/guide/team-06-pinwaende.webp").as_slice(),"image/webp")),
            (false,"/agent-team.js")=>Some((include_bytes!("../../../web-client/agent-team.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/agent-team.css")=>Some((include_bytes!("../../../web-client/agent-team.css").as_slice(),"text/css; charset=utf-8")),
            (false,"/agenten-hilfe.html")=>Some((include_bytes!("../../../web-client/agenten-hilfe.html").as_slice(),"text/html; charset=utf-8")),
            (false,"/local-play.js")=>Some((include_bytes!("../../../web-client/local-play.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/local-galaxy.js")=>Some((include_bytes!("../../../web-client/local-galaxy.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/communication.js")=>Some((include_bytes!("../../../web-client/communication.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/game.css")=>Some((include_bytes!("../../../web-client/game.css").as_slice(),"text/css; charset=utf-8")),
            (false,"/galaxy.js")=>Some((include_bytes!("../../../web-client/galaxy.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/three.min.js")=>Some((include_bytes!("../../../web-client/three.min.js").as_slice(),"text/javascript; charset=utf-8")),
            (false,"/galaxy.css")=>Some((include_bytes!("../../../web-client/galaxy.css").as_slice(),"text/css; charset=utf-8")),
            (false,"/cockpit-look.css")=>Some((include_bytes!("../../../web-client/cockpit-look.css").as_slice(),"text/css; charset=utf-8")),
            (false,"/art.js")=>Some((include_bytes!("../../../web-client/art.js").as_slice(),"text/javascript; charset=utf-8")),
            (_, "/identity-ui.css")=>Some((include_bytes!("../../../web-client/identity-ui.css").as_slice(),"text/css; charset=utf-8")),
            (_, "/style.css") => Some((include_bytes!("../../../web-client/style.css").as_slice(), "text/css; charset=utf-8")),
            (true, "/") | (true, "/index.html") => Some((include_bytes!("../../../web-client/admin/index.html").as_slice(), "text/html; charset=utf-8")),
            (true,"/monitoring.js")=>Some((include_bytes!("../../../web-client/admin/monitoring.js").as_slice(),"text/javascript; charset=utf-8")),
            (true, "/admin.js") => Some((include_bytes!("../../../web-client/admin/admin.js").as_slice(), "text/javascript; charset=utf-8")),
            (false,p) => web_assets::get(p),
            _ => None,
        };
        if let Some((bytes, mime)) = asset {
            answer(req, 200, bytes.to_vec(), mime, &origin, &[]);
            return;
        }
        if path == "/config.js" && !admin {
            answer(req, 200, format!("window.STERNENEPOCHE={{api:location.origin,version:{API_VERSION}}};").into_bytes(), "text/javascript; charset=utf-8", &origin, &[]);
            return;
        }
        if path == "/api/admin/bootstrap" && admin {
            // Only the separate loopback UI can read this credential; public listener has no such route.
            json_reply(req, Ok(json!({"key":admin_key,"game_url":format!("http://{}",cfg.bind)})), &origin);
            return;
        }
    }
    if admin && !konstant_gleich(&header(&req, "X-Sternenepoche-Admin"), admin_key) {
        json_reply(req, Err((401, "Adminzugang erforderlich".into())), &origin);
        return;
    }
    if !admin && path.starts_with("/api/admin") {
        json_reply(req, Err((404, "Nicht gefunden".into())), &origin);
        return;
    }
    let mut body = Value::Null;
    if method == Method::Post {
        if !header(&req, "Content-Type").starts_with("application/json") {
            json_reply(req, Err((415, "JSON erwartet".into())), &origin);
            return;
        }
        if req.body_length().is_none() || req.body_length().is_some_and(|n| n > 65536) {
            json_reply(req, Err((413, "Befehle auf 64 KiB begrenzt; Content-Length erforderlich".into())), &origin);
            return;
        }
        let mut bytes = Vec::new();
        // Der Socket hat eine Lesefrist: ein absichtlich langsamer Sender blockiert keinen Arbeiter dauerhaft.
        if req.as_reader().take(65537).read_to_end(&mut bytes).is_err() || bytes.len() > 65536 {
            json_reply(req, Err((400, "Anfrage unvollständig oder zu langsam".into())), &origin);
            return;
        }
        body = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => {
                json_reply(req, Err((400, "JSON nicht lesbar".into())), &origin);
                return;
            }
        };
        if let Err(e) = eingabe::json_grenzen(&body) {
            json_reply(req, Err((400, e)), &origin);
            return;
        }
    }
    let raw = header(&req, "Authorization").strip_prefix("Bearer ").unwrap_or("").to_string();
    // Falle für maschinelles Auslesen: Links dorthin stehen nur verborgen in der Oberfläche.
    if !admin && path.starts_with("/api/intern/") {
        let mut g = game.lock().unwrap();
        let konto = g.authenticate(&raw).ok().or_else(|| g.harness_authenticate(&raw).ok().map(|(a, _)| a));
        let treffer = g.fairplay_pruefen(&body, &url);
        let result = match (konto, treffer) {
            (Some(a), Some(t)) => Err(g.fairplay_verstoss(&a, &t, &path)),
            _ => { g.fairplay_anonym(&ip, &url); Err((404, "Nicht gefunden".into())) }
        };
        drop(g);
        json_reply(req, result, &origin);
        return;
    }
    if !admin && path.starts_with("/api/harness/v1/") {
        let mut extra = Vec::new();
        if method == Method::Post && path == "/api/harness/v1/mcp" {
            // MCP (Streamable HTTP, JSON-Antworten): dieselben Methoden, Schlüssel, Quoten und Prüfungen.
            let mut g = game.lock().unwrap();
            let antwort = match g.harness_authenticate(&raw) {
                Err((code, e)) => Err((code, e)),
                Ok((a, s)) => {
                    if g.fairplay_wartet_auf_bestaetigung(a.id) { Err((403, "Fair-Play-Verwarnung zuerst im Browser bestätigen".to_string())) }
                    else if let Err(e) = quote_pruefen(limits, a.id, mcp::topf(&body), &mut extra) { Err(e) }
                    else if let Some(t) = g.fairplay_pruefen(&body, &query) { Err(g.fairplay_verstoss(&a, &t, &path)) }
                    else { Ok(mcp::bearbeiten(&mut g, &a, &s, &body, public_catalog::catalog)) }
                }
            };
            drop(g);
            match antwort {
                Ok(Some(v)) => json_status(req, 200, v, &origin, &extra),
                Ok(None) => answer(req, 202, vec![], "application/json", &origin, &extra),
                Err((code, e)) => json_status(req, code, mcp::fehler(&body, code, &e), &origin, &extra),
            }
            return;
        }
        let mut g = game.lock().unwrap();
        let result = harness_route(&mut g, &method, &path, &query, &raw, &body, limits, &mut extra);
        drop(g);
        json_reply_mit(req, result, &origin, &extra);
        return;
    }
    if !admin && method == Method::Post && path=="/api/registration/start" {
        if !allowed(limits,format!("email-start:{ip}"),6) {json_reply(req,Err((429,"Zu viele Codes angefordert; später erneut versuchen".into())),&origin);return;}
        let mail=match sternenepoche_server::mail::MailConfig::load(&cfg.root) {
            Ok(Some(config))=>config,
            _=>{json_reply(req,Err((503,"Neue Registrierung derzeit nicht verfügbar. Vorhandene Konten können sich anmelden.".into())),&origin);return;}
        };
        let challenge=game.lock().unwrap().begin_email(body["email"].as_str().unwrap_or(""));
        let result=match challenge {
            Err(e)=>Err(e),
            Ok(challenge)=>{
                // Network I/O never holds the authoritative world's mutex.
                let sent=mail.send_code(&challenge.email,&challenge.code);
                let saved=game.lock().unwrap().email_delivery(&challenge.id,sent.is_ok());
                match sent {Ok(())=>saved,Err(e)=>Err((503,e))}
            }
        };
        json_reply(req,result,&origin);return;
    }
    if !admin && method == Method::Post && path=="/api/registration/verify" {
        let result=if allowed(limits,format!("email-verify:{ip}"),20){game.lock().unwrap().verify_email(&body)}else{Err((429,"Zu viele Codeversuche; später erneut versuchen".into()))};
        json_reply(req,result,&origin);return;
    }
    if !admin && method == Method::Post && path == "/api/sitzung/einloesen" {
        // Einmalcode aus dem Cockpit („Go to Browser“): 60 Sekunden, genau einmal.
        let result = if allowed(limits, format!("auth:{ip}"), 30) { game.lock().unwrap().uebergabe_einloesen(&body) } else { Err((429, "Zu viele Anmeldungen; in einer Minute erneut versuchen".into())) };
        json_reply(req, result, &origin);
        return;
    }
    if !admin && method == Method::Post && ["/api/register", "/api/login"].contains(&path.as_str())
    {
        if !allowed(limits, format!("auth:{ip}"), 30) {
            json_reply(req, Err((429, "Zu viele Anmeldungen; in einer Minute erneut versuchen".into())), &origin);
            return;
        }
        let password = body["password"].as_str().unwrap_or("");
        // Expensive password work runs outside the world mutex.
        let result = if path == "/api/register" {
            if !sternenepoche_server::mail::MailConfig::load(&cfg.root).is_ok_and(|c|c.is_some()) {
                json_reply(req,Err((503,"Neue Registrierung derzeit nicht verfügbar. Vorhandene Konten können sich anmelden.".into())),&origin);return;
            }
            if body["registration_token"].as_str().is_none_or(|s|s.len()!=64) {
                json_reply(req,Err((403,"Zuerst E-Mail-Adresse bestätigen".into())),&origin);return;
            }
            match password_hash(password) {
                Ok(hash) => game.lock().unwrap().register_email(&body, hash),
                Err(e) => Err((400, e)),
            }
        } else {
            let account = game
                .lock()
                .unwrap()
                .account_by_name(body["name"].as_str().unwrap_or(""));
            let hash=account.as_ref().map(|a|a.password.clone()).unwrap_or_else(||"$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY3OA$Z9XRhYVbhvSMjrgd8qVTuXFtrV89zhU4BJG5ULtdhdA".into());
            let valid = password.len() <= 128 && password_matches(password, &hash);
            match account.filter(|_| valid) {
                Some(a) => {
                    let mut g=game.lock().unwrap();
                    // A concurrent private password reset must invalidate an in-flight login.
                    if !g.account_by_name(&a.name).is_some_and(|current|current.password==a.password && !current.banned){Err((401,"Anmeldung fehlgeschlagen; erneut anmelden".into()))}
                    else if !g.login_permitted(a.id){Err((429,"Zu viele fehlgeschlagene Anmeldungen; in einer Minute erneut versuchen".into()))}
                    else if g.check_second_factor(a.id,body["otp"].as_str().unwrap_or("")).unwrap_or(false){g.login_verified(a.id)}
                    else{let _=g.failed_login(&a.name);Err((401,"Name, Passwort oder Authenticator-Code falsch. Verwende einen neuen, noch nicht benutzten App-Code.".into()))}
                },
                None => {let _=game.lock().unwrap().failed_login(body["name"].as_str().unwrap_or(""));Err((401, "Name oder Passwort falsch".into()))},
            }
        };
        json_reply(req, result, &origin);
        return;
    }
    if !admin && method==Method::Post && matches!(path.as_str(),"/api/authenticator/setup"|"/api/authenticator/enable"|"/api/authenticator/disable") {
        if !allowed(limits,format!("authenticator:{ip}"),10){json_reply(req,Err((429,"Zu viele Sicherheitsänderungen; später erneut versuchen".into())),&origin);return;}
        let account=game.lock().unwrap().authenticate(&raw);
        let result=match account {
            Err(e)=>Err(e),
            Ok(a)=>{
                let password=body["password"].as_str().unwrap_or("");
                if password.len()>128 || !password_matches(password,&a.password){Err((401,"Aktuelles Passwort erforderlich".into()))}
                else{let mut g=game.lock().unwrap();match g.authenticate(&raw){
                    Err(e)=>Err(e),
                    Ok(a)=>match path.as_str(){
                        "/api/authenticator/setup"=>g.authenticator_start(&a),
                        "/api/authenticator/enable"=>g.authenticator_enable(&a,&body,&raw),
                        _=>g.authenticator_disable(&a,&body,&raw),
                    }
                }}
            }
        };
        json_reply(req,result,&origin);return;
    }
    let mut extra = Vec::new();
    let mut g = game.lock().unwrap();
    let result = if admin {
        match (method, path.as_str()) {
            (Method::Get, "/api/admin/status") => Ok(g.admin_status()),
            (Method::Post, "/api/admin/action") => g.admin(&body),
            _ => Err((404, "Nicht gefunden".into())),
        }
    } else {
        match (method, path.as_str()) {
            (Method::Get, "/api/lobby") | (Method::Get, "/api/health") => {
                let mut lobby = g.lobby();
                // Nur die Browseroberfläche fordert die unsichtbaren Köder an; Statusabfragen bleiben sauber.
                if header(&req, "X-Sternenepoche-Oberflaeche") == "browser" { lobby["oberflaeche"] = g.oberflaeche(None, "lobby"); }
                Ok(lobby)
            }
            (Method::Get, "/api/registration/status")=>Ok(json!({"enabled":sternenepoche_server::mail::MailConfig::load(&cfg.root).is_ok_and(|c|c.is_some()),"code_digits":6,"test_login":true})),
            (Method::Get, "/api/rules") => Ok(
                json!({"api_version":API_VERSION,"schema":kern::aktion::antwortschema(Rolle::Alle),"volk":VolkNames::values(),"catalog":public_catalog::catalog(&g.world.regeln),"text":format!("{}\n{}\n{}",kern::regeltext::regeltext(&g.world.regeln,Rolle::Alle),kern::kolonisation::REGELTEXT_V2,kern::ausscheiden::REGELTEXT),"fairplay":sternenepoche_server::fairplay::REGEL,"harness_regeln":sternenepoche_server::harness::REGELN_VERSION}),
            ),
            (method, path) => match g.authenticate(&raw) {
                Err(e) => Err(e),
                Ok(a) => {
                    let gesperrt_bis_bestaetigung = g.fairplay_wartet_auf_bestaetigung(a.id)
                        && !matches!(path, "/api/me" | "/api/logout" | "/api/fairplay/bestaetigen");
                    if gesperrt_bis_bestaetigung {
                        Err((403, "Fair-Play-Verwarnung zuerst bestätigen. Bis dahin sind Befehle gesperrt.".into()))
                    } else if let Err(e) = quote_pruefen(limits, a.id, harness::quote_fuer_pfad(path), &mut extra) {
                        Err(e)
                    } else if let Some(t) = g.fairplay_pruefen(&body, &query) {
                        Err(g.fairplay_verstoss(&a, &t, path))
                    } else {
                        match (method, path) {
                            (Method::Get, "/api/me") => Ok(g.me(&a)),
                            (Method::Get, "/api/workspace") => g.workspace(&a),
                            (Method::Post, "/api/workspace") => g.save_workspace(&a,&body),
                            (Method::Get, "/api/view") => g.view(&a, Rolle::Alle),
                            (Method::Post, "/api/context") => {
                                let role = body["rolle"].as_str().and_then(Rolle::aus_name).filter(|r| *r != Rolle::Alle);
                                match role {Some(r)=>g.view(&a,r).map(|view|json!({"view":view,"schema":kern::aktion::antwortschema(r),"text":kern::regeltext::regeltext(&g.world.regeln,r)})),None=>Err((400,"Regierungsrolle auswählen".into()))}
                            }
                            (Method::Post, "/api/claim") => g.claim(&a, &body),
                            (Method::Post, "/api/waitlist/leave") => g.leave_waitlist(&a, &body),
                            (Method::Post, "/api/tool") => g.tool(&a, &body),
                            (Method::Post, "/api/command") => g.command(&a, &body),
                            (Method::Post, "/api/lease") => g.lease(&a, &body),
                            (Method::Post, "/api/agent-report") => g.agent_report(&a, &body),
                            (Method::Post, "/api/fairplay/bestaetigen") => g.fairplay_bestaetigen(&a),
                            (Method::Post, "/api/sitzung/uebergabe") => g.uebergabe_erstellen(&a),
                            (Method::Post, "/api/logout") => g.logout(&a, &raw),
                            _ => Err((404, "Nicht gefunden".into())),
                        }
                    }
                }
            },
        }
    };
    drop(g);
    json_reply_mit(req, result, &origin, &extra);
}
/// Vergleich ohne frühen Abbruch, damit die Antwortzeit den Adminschlüssel nicht verrät.
fn konstant_gleich(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
struct VolkNames;
impl VolkNames {
    fn values() -> Vec<&'static str> {
        kern::typen::Volk::ALLE
            .into_iter()
            .map(|v| v.name())
            .collect()
    }
}
/// Lauscher mit Lese- und Schreibfrist. Angenommene Verbindungen erben die Fristen
/// (Windows und Linux); ein langsamer Client hält einen Arbeiter höchstens so lange.
fn lauscher(addr: &str, frist: Duration) -> Result<Server, String> {
    let a: SocketAddr = addr.parse().map_err(|_| "Adresse mit numerischer IP und Port angeben")?;
    let s = socket2::Socket::new(socket2::Domain::for_address(a), socket2::Type::STREAM, Some(socket2::Protocol::TCP)).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(frist)).map_err(|e| e.to_string())?;
    s.set_write_timeout(Some(frist)).map_err(|e| e.to_string())?;
    s.bind(&a.into()).map_err(|e| format!("{addr}: {e}"))?;
    s.listen(256).map_err(|e| e.to_string())?;
    Server::from_listener(std::net::TcpListener::from(s), None).map_err(|e| e.to_string())
}
fn run() -> Result<(), String> {
    let cfg = Arc::new(args()?);
    if cfg.kaefig_mb > 0 {
        match kaefig::einsperren(cfg.kaefig_mb) {
            Ok(text) => println!("{text}"),
            Err(e) => eprintln!("Warnung: {e}; Server läuft ohne Prozess-Käfig weiter"),
        }
    }
    std::fs::create_dir_all(&cfg.root).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cfg.root.join("server.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock_exclusive()
        .map_err(|_| "Diese Datenbank wird bereits von einem Server benutzt")?;
    let game = Arc::new(Mutex::new(Game::open(&cfg.root, rand::random())?));
    let keyfile = cfg.root.join("admin-token.txt");
    let key = if keyfile.exists() {
        std::fs::read_to_string(&keyfile)
            .map_err(|e| e.to_string())?
            .trim()
            .to_string()
    } else {
        let key = token();
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        use std::io::Write;
        let mut f = opts.open(&keyfile).map_err(|e| e.to_string())?;
        f.write_all(key.as_bytes()).map_err(|e| e.to_string())?;
        key
    };
    if key.len() != 64 {
        return Err("Admin-Token beschädigt".into());
    }
    let key = Arc::new(key);
    for (admin, addr, count) in [(false, &cfg.bind, 8), (true, &cfg.admin_bind, 2)] {
        // Public request pressure must not fill the private dashboard's limiter.
        let limits: Limits = Arc::new(Mutex::new(HashMap::new()));
        let server = Arc::new(lauscher(addr, Duration::from_secs(cfg.timeout_s))?);
        for _ in 0..count {
            let (s, g, c, k, l) = (
                server.clone(),
                game.clone(),
                cfg.clone(),
                key.clone(),
                limits.clone(),
            );
            thread::spawn(move || {
                for req in s.incoming_requests() {
                    serve(req, admin, &g, &c, &k, &l);
                }
            });
        }
    }
    println!(
        "Spiel: http://{} | Lokales Dashboard: http://{} | Daten: {} | Proxy: {:?}",
        cfg.bind,
        cfg.admin_bind,
        cfg.root.display(),
        cfg.proxy
    );
    let mut last = Instant::now();
    let mut remainder = 0.0;
    loop {
        thread::sleep(Duration::from_millis(200));
        let elapsed = last.elapsed().as_secs_f64();
        last = Instant::now();
        let mut g = game.lock().unwrap();
        if g.runtime.paused || g.world.beendet() {
            remainder = 0.0;
            continue;
        }
        // Resume never interprets machine sleep as an unattended raid catch-up.
        // Normal work/SQLite contention must not silently lose elapsed time.
        // A gap beyond one minute is treated as machine suspension, not catch-up.
        remainder += (if elapsed > 60.0 {0.2} else {elapsed}) * g.runtime.tempo as f64;
        let steps = remainder.floor().min(3600.0) as u32;
        if steps > 0 {
            remainder -= steps as f64;
            if let Err((_, e)) = g.advance(steps) {
                eprintln!("{e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn rotating_invalid_tokens_cannot_bypass_the_ip_budget() {
        let limits: Limits = Arc::new(Mutex::new(HashMap::new()));
        for n in 0..6000 {
            assert!(request_allowed(&limits, "test-ip", &format!("invalid-token-{n}")));
        }
        assert!(!request_allowed(&limits, "test-ip", "another-invalid-token"));
        assert!(request_allowed(&limits, "other-ip", "legitimate-session"));
    }
    #[test]
    fn konto_quote_gilt_fuer_browser_und_harness_gemeinsam() {
        let limits: Limits = Arc::new(Mutex::new(HashMap::new()));
        let mut extra = Vec::new();
        for _ in 0..harness::quote("handeln") { quote_pruefen(&limits, 7, harness::quote_fuer_pfad("/api/command"), &mut extra).unwrap(); }
        assert_eq!(quote_pruefen(&limits, 7, harness::quote_fuer_pfad("/api/harness/v1/handeln"), &mut extra).unwrap_err().0, 429);
        assert!(extra.iter().any(|(k, _)| *k == "Retry-After"));
        assert!(quote_pruefen(&limits, 8, "handeln", &mut extra).is_ok());
    }
    #[test]
    fn parameter_und_vergleich() {
        assert_eq!(param("rolle=verwalter&x=1", "rolle").as_deref(), Some("verwalter"));
        assert_eq!(param("q=a%20b+c", "q").as_deref(), Some("a b c"));
        assert_eq!(param("q=%zz", "q").as_deref(), Some("%zz"));
        assert!(param("rolle", "rolle").is_none());
        assert!(konstant_gleich("abc", "abc") && !konstant_gleich("abc", "abd") && !konstant_gleich("abc", "ab"));
    }
    #[test]
    fn langsame_clients_belegen_keinen_arbeiter_dauerhaft() {
        let server = lauscher("127.0.0.1:0", Duration::from_millis(800)).unwrap();
        let addr = server.server_addr().to_ip().unwrap();
        let server = Arc::new(server);
        let s = server.clone();
        thread::spawn(move || {
            for mut req in s.incoming_requests() {
                let mut b = Vec::new();
                let ok = req.as_reader().take(100_000).read_to_end(&mut b).is_ok();
                let _ = req.respond(Response::from_string(if ok { "ganz" } else { "frist" }));
            }
        });
        // Körper wird angekündigt, aber nie gesendet.
        let mut c = std::net::TcpStream::connect(addr).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        c.write_all(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 500\r\n\r\nabc").unwrap();
        let start = Instant::now();
        let mut antwort = String::new();
        let _ = c.read_to_string(&mut antwort);
        assert!(start.elapsed() < Duration::from_secs(6), "Körperfrist greift nicht");
        // Kleine Körper liest tiny_http selbst vorab; dann antwortet es nach der Frist mit 4xx.
        assert!(antwort.contains("frist") || antwort.is_empty() || antwort.starts_with("HTTP/1.1 4"), "{antwort}");
        eprintln!("Langsamer Körper: {}", antwort.lines().next().unwrap_or("Verbindung geschlossen"));
        // Größerer Körper erreicht den Arbeiter; dessen Lesen endet ebenfalls nach der Frist.
        let mut g = std::net::TcpStream::connect(addr).unwrap();
        g.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        g.write_all(b"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 50000\r\n\r\n").unwrap();
        g.write_all(&[b'a'; 3000]).unwrap();
        let start = Instant::now();
        let mut antwort = String::new();
        let _ = g.read_to_string(&mut antwort);
        assert!(start.elapsed() < Duration::from_secs(6), "Arbeiterfrist greift nicht");
        assert!(antwort.contains("frist") || antwort.starts_with("HTTP/1.1 4") || antwort.is_empty(), "{antwort}");
        // Kopfzeilen kommen nie vollständig an.
        let mut d = std::net::TcpStream::connect(addr).unwrap();
        d.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        d.write_all(b"GET / HTTP/1.1\r\nHo").unwrap();
        let start = Instant::now();
        let mut antwort = String::new();
        let _ = d.read_to_string(&mut antwort);
        assert!(start.elapsed() < Duration::from_secs(6), "Kopffrist greift nicht");
        // Danach bedient der Server weiter normal.
        let mut e = std::net::TcpStream::connect(addr).unwrap();
        e.write_all(b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").unwrap();
        let mut antwort = String::new();
        e.read_to_string(&mut antwort).unwrap();
        assert!(antwort.contains("ganz"));
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Serverstart fehlgeschlagen: {e}");
        std::process::exit(1)
    }
}
