//! Ratsstimmen: mit einem Ratsmitglied sprechen – es antwortet in seiner Rolle und in seiner Stimme.
//!
//! Antwort: das Modell der Rolle (Ollama, OpenRouter oder eigener Server) mit der Persönlichkeit aus
//! assets/stimmen/rat.json und einem kurzen Lagebild. Stimme: Voicebox (lokal, 127.0.0.1:17493) mit dem
//! Profil der Figur; fehlt es dort, legt das Cockpit es aus dem eingebetteten Voicebox-Paket an
//! (assets/stimmen/<rolle>.<volk>/stimme/). Die GPU teilen sich mehrere Programme: Meldet der
//! Loki-Koordinator Arbeit oder rechnet ComfyUI gerade, bleibt Voicebox unberührt. Dann spricht, falls
//! vorhanden, Loki-Stimmen auf der CPU (langsamer), sonst bleibt es beim Text.
//! Der Text des Modells geht nie durch eine Shell (Loki-Stimmen bekommt ihn als Datei).
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const RAT: &str = include_str!("../assets/stimmen/rat.json");
const VOICEBOX: &str = "http://127.0.0.1:17493";
const KOORDINATOR: &str = "http://127.0.0.1:11435";
const COMFYUI: &str = "http://127.0.0.1:8189";

fn rat() -> &'static Value { static R: OnceLock<Value> = OnceLock::new(); R.get_or_init(|| serde_json::from_str(RAT).unwrap_or(Value::Null)) }
fn gespraeche() -> &'static Mutex<VecDeque<Value>> { static G: OnceLock<Mutex<VecDeque<Value>>> = OnceLock::new(); G.get_or_init(Default::default) }
fn live_speicher() -> &'static Mutex<(VecDeque<String>, HashMap<String, Vec<u8>>)> { static L: OnceLock<Mutex<(VecDeque<String>, HashMap<String, Vec<u8>>)>> = OnceLock::new(); L.get_or_init(Default::default) }
fn dienst() -> &'static Mutex<Value> { static D: OnceLock<Mutex<Value>> = OnceLock::new(); D.get_or_init(|| Mutex::new(json!({}))) }

fn voicebox_url() -> String { std::env::var("STERNENEPOCHE_VOICEBOX").unwrap_or_else(|_| VOICEBOX.into()).trim_end_matches('/').to_string() }
fn http(sekunden: u64) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder().timeout(Duration::from_secs(sekunden)).connect_timeout(Duration::from_secs(2)).build().unwrap_or_default()
}

/// Gesprochene Antworten für die Oberfläche (Pfad „live/<id>.wav“ im eigenen Protokoll).
pub fn live(pfad: &str) -> Option<Vec<u8>> {
    let p = pfad.trim_start_matches('/');
    if !p.starts_with("live/") { return None; }
    live_speicher().lock().ok()?.1.get(p).cloned()
}
fn live_ablegen(id: &str, wav: Vec<u8>) -> String {
    let pfad = format!("live/{id}.wav");
    let mut s = live_speicher().lock().unwrap();
    s.0.push_back(pfad.clone()); s.1.insert(pfad.clone(), wav);
    while s.0.len() > 16 { if let Some(alt) = s.0.pop_front() { s.1.remove(&alt); } }
    pfad
}

/// Was die Oberfläche zeigt: die letzten Gespräche und der Zustand der Stimm-Dienste.
pub fn stand() -> Value {
    json!({"gespraeche": gespraeche().lock().map(|g| g.iter().cloned().collect::<Vec<_>>()).unwrap_or_default(), "dienst": dienst().lock().map(|d| d.clone()).unwrap_or_default()})
}
fn aendern(id: &str, f: impl FnOnce(&mut Value)) {
    if let Ok(mut g) = gespraeche().lock() { if let Some(e) = g.iter_mut().find(|e| e["id"] == id) { f(e); } }
}

/// Modell einer Rolle, so wie der Harness es nutzt.
pub struct Modell { pub anbieter: String, pub url: String, pub modell: String, pub schluessel: String, pub temperatur: f32, pub kontext: u32 }

pub struct Anfrage {
    pub rolle: String, pub volk: String, pub frage: String,
    /// Bisheriger Verlauf mit dieser Figur: (vom Spieler?, Text).
    pub verlauf: Vec<(bool, String)>,
    pub modell: Modell,
    /// Lagebild und letzte Gedanken der Rolle (kurz, nur zur Orientierung).
    pub lage: Value,
    pub mit_stimme: bool,
}

/// Eine Frage stellen: Antwort und Stimme entstehen im Hintergrund; der Stand steht in stand().
pub fn fragen(a: Anfrage) -> Result<(), String> {
    let fid = format!("{}.{}", a.rolle, a.volk);
    if rat()["figuren"][&fid].is_null() { return Err(format!("Unbekanntes Ratsmitglied {fid}")); }
    let frage: String = a.frage.trim().chars().filter(|c| !c.is_control()).take(600).collect();
    if frage.is_empty() { return Err("Frage fehlt".into()); }
    let mut b = [0u8; 6];
    getrandom::getrandom(&mut b).map_err(|e| e.to_string())?;
    let id: String = b.iter().map(|x| format!("{x:02x}")).collect();
    {
        let mut g = gespraeche().lock().unwrap();
        if g.iter().any(|e| e["status"] == "denkt" || e["status"] == "vertont") { return Err("Der Rat antwortet noch auf die letzte Frage.".into()); }
        g.push_back(json!({"id":id,"rolle":a.rolle,"volk":a.volk,"frage":frage,"antwort":"","status":"denkt","audio":null,"hinweis":"","modell":a.modell.modell,"zeit":crate::app::jetzt()}));
        while g.len() > 40 { g.pop_front(); }
    }
    std::thread::spawn(move || {
        let t0 = std::time::Instant::now();
        match antworten(&a, &fid, &frage) {
            Err(e) => aendern(&id, |x| { x["status"] = json!("fehler"); x["hinweis"] = json!(e); }),
            Ok(text) => {
                let ms = t0.elapsed().as_millis() as u64;
                aendern(&id, |x| { x["antwort"] = json!(text); x["denk_ms"] = json!(ms); x["status"] = json!(if a.mit_stimme { "vertont" } else { "fertig" }); });
                if a.mit_stimme {
                    let t1 = std::time::Instant::now();
                    let ergebnis = vertonen(&fid, &text, |h| aendern(&id, |x| x["hinweis"] = json!(h)));
                    match ergebnis {
                        Ok((wav, weg)) => { let p = live_ablegen(&id, wav); aendern(&id, |x| { x["audio"] = json!(p); x["status"] = json!("fertig"); x["weg"] = json!(weg); x["hinweis"] = json!(""); x["ton_ms"] = json!(t1.elapsed().as_millis() as u64); }); }
                        Err(e) => aendern(&id, |x| { x["status"] = json!("fertig"); x["hinweis"] = json!(e); }),
                    }
                }
            }
        }
    });
    Ok(())
}

fn persona(fid: &str, volk: &str, lage: &Value) -> String {
    let f = &rat()["figuren"][fid];
    let rolle = fid.split('.').next().unwrap_or("");
    let titel = match rolle { "stratege" => "Stratege", "verwalter" => "Verwalter", "diplomat" => "Diplomat", _ => "Feldherr" };
    let s = |v: &Value| v.as_str().unwrap_or("").to_string();
    // Nur bekannte Werte: leere Felder würden das Modell zum Raten verleiten.
    let bekannt: serde_json::Map<String, Value> = lage.as_object().map(|o| o.iter()
        .filter(|(_, v)| !(v.is_null() || v.as_str() == Some("") || v.as_array().is_some_and(Vec::is_empty) || v.as_object().is_some_and(serde_json::Map::is_empty)))
        .map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
    let mut lage_text = if bekannt.keys().all(|k| k == "rat_tagt" || k == "warten_auf_erlaubnis") {
        "Keine aktuelle Lage: Der Rat ruht, du kennst gerade weder Vorräte noch Planeten. Sag das offen, wenn danach gefragt wird.".to_string()
    } else { Value::Object(bekannt).to_string() };
    if lage_text.chars().count() > 2400 { lage_text = lage_text.chars().take(2400).collect::<String>() + " …"; }
    format!("Du bist {name}, {titel} im Rat eines Reiches im Spiel Sternenepoche, vom Volk der {volkname}.\n\
        Dein Volk: {volkinfo}\n\
        So stellst du dich selbst vor: {vorstellung}\n\
        Deine Sprechweise: {sprechweise}\n\n\
        Der Mensch, der mit dir spricht, ist der Herrscher deines Reiches. Du berätst ihn als Mitglied seines Rates.\n\
        Aktuelle Lage des Reiches und deine letzten Gedanken im Rat (nur zur Orientierung, JSON): {lage_text}\n\n\
        Regeln für deine Antwort:\n\
        - Antworte auf Deutsch als gesprochene Rede, ganz in deiner Rolle und Persönlichkeit.\n\
        - Höchstens drei kurze Sätze, unter sechzig Wörtern.\n\
        - Keine Listen, kein Markdown, keine Emojis, keine Regieanweisungen in Klammern oder Sternchen.\n\
        - Kurze Zahlen als Wörter.\n\
        - Erfinde keine Spielfakten. Was du nicht aus der Lage weißt, sagst du in deiner Art offen.\n\
        - Du führst im Gespräch keine Befehle aus. Will der Herrscher etwas, sag, wie du es im Rat angehen würdest, \
        und dass er es dir als Hinweis für den nächsten Zug geben kann.",
        name = s(&f["name"]), volkname = volk_name(volk), volkinfo = s(&rat()["voelker"][volk]),
        vorstellung = s(&f["vorstellung"]), sprechweise = s(&f["sprechweise"]))
}
fn volk_name(v: &str) -> &'static str { match v { "veyari" => "Veyari", "aurelianer" => "Aurelianer", "krath" => "Krath", _ => "Syntheten" } }

fn antworten(a: &Anfrage, fid: &str, frage: &str) -> Result<String, String> {
    let mut nachrichten = vec![json!({"role":"system","content":persona(fid, &a.volk, &a.lage)})];
    for (spieler, text) in a.verlauf.iter().rev().take(8).rev() {
        nachrichten.push(json!({"role": if *spieler { "user" } else { "assistant" }, "content": text.chars().take(600).collect::<String>()}));
    }
    nachrichten.push(json!({"role":"user","content":frage}));
    let m = &a.modell;
    if m.modell.trim().is_empty() { return Err("Für diese Rolle ist kein Modell eingestellt (Rat → Modell).".into()); }
    let temp = m.temperatur.clamp(0.2, 1.1).max(0.7);
    let roh = match m.anbieter.as_str() {
        "openrouter" => {
            if m.schluessel.is_empty() { return Err("OpenRouter-Schlüssel fehlt (Werkstatt → Konto).".into()); }
            let v: Value = http(120).post("https://openrouter.ai/api/v1/chat/completions").bearer_auth(&m.schluessel)
                .json(&json!({"model":m.modell,"messages":nachrichten,"max_tokens":260,"temperature":temp})).send().map_err(|e| format!("OpenRouter: {e}"))?
                .json().map_err(|e| format!("OpenRouter: {e}"))?;
            v["choices"][0]["message"]["content"].as_str().map(str::to_string).ok_or_else(|| format!("OpenRouter: {}", v["error"]["message"].as_str().unwrap_or("leere Antwort")))?
        }
        "openai" => {
            let mut q = http(180).post(format!("{}/chat/completions", m.url.trim_end_matches('/'))).json(&json!({"model":m.modell,"messages":nachrichten,"max_tokens":260,"temperature":temp}));
            if !m.schluessel.is_empty() { q = q.bearer_auth(&m.schluessel); }
            let v: Value = q.send().map_err(|e| format!("Eigener Server: {e}"))?.json().map_err(|e| format!("Eigener Server: {e}"))?;
            v["choices"][0]["message"]["content"].as_str().map(str::to_string).ok_or("Eigener Server: leere Antwort")?
        }
        _ => {
            let v: Value = http(240).post(format!("{}/api/chat", m.url.trim_end_matches('/')))
                .json(&json!({"model":m.modell,"stream":false,"think":false,"messages":nachrichten,"keep_alive":"10m",
                    "options":{"temperature":temp,"num_ctx":m.kontext.clamp(4096, 16384),"num_predict":260}})).send().map_err(|e| format!("Ollama: {e}"))?
                .json().map_err(|e| format!("Ollama: {e}"))?;
            if let Some(e) = v["error"].as_str() { return Err(format!("Ollama: {e}")); }
            v["message"]["content"].as_str().map(str::to_string).ok_or("Ollama: leere Antwort")?
        }
    };
    let text = saeubern(&roh);
    if text.is_empty() { Err("Das Modell hat nichts Sprechbares geantwortet.".into()) } else { Ok(text) }
}

/// Denkspuren, Markdown, Regieanweisungen und Emojis entfernen; auf eine sprechbare Länge kürzen.
pub fn saeubern(roh: &str) -> String {
    let mut t = roh.to_string();
    while let (Some(a), Some(b)) = (t.find("<think>"), t.find("</think>")) { if b < a { break; } t.replace_range(a..b + 8, ""); }
    if let Some(b) = t.find("</think>") { t = t[b + 8..].to_string(); }
    let mut aus = String::new();
    let mut tiefe = 0;
    for c in t.chars() {
        match c {
            '*' | '_' | '#' | '`' | '>' => {}
            '[' | '(' if tiefe == 0 && aus.ends_with(' ') => tiefe = 1,
            ']' | ')' if tiefe > 0 => tiefe = 0,
            _ if tiefe > 0 => {}
            c if (c as u32) >= 0x1F000 || ('\u{2600}'..='\u{27BF}').contains(&c) => {}
            c if c.is_whitespace() => { if !aus.ends_with(' ') { aus.push(' '); } }
            c => aus.push(c),
        }
    }
    let mut s = aus.trim().trim_matches('"').trim_matches('„').trim_matches('“').trim().to_string();
    if s.chars().count() > 420 {
        let schnitt: String = s.chars().take(420).collect();
        let ende = schnitt.rfind(['.', '!', '?']).map(|i| i + 1).unwrap_or(schnitt.len());
        s = schnitt[..ende].to_string();
    }
    s
}

// ------------------------------------------------------------------------------------------ Stimme
/// Ist die GPU frei? Nein, wenn der Loki-Koordinator Arbeit meldet oder ComfyUI gerade rechnet.
/// Ohne Koordinator und ohne ComfyUI (Rechner anderer Spieler) gilt sie als frei.
fn gpu_belegt() -> Option<String> {
    let c = http(2);
    if let Ok(v) = c.get(format!("{KOORDINATOR}/status")).send().and_then(|r| r.json::<Value>()) {
        let phase = v["phase"].as_str().unwrap_or("idle");
        if phase != "idle" { return Some(format!("Loki-Koordinator: {phase}")); }
    }
    if let Ok(v) = c.get(format!("{COMFYUI}/queue")).send().and_then(|r| r.json::<Value>()) {
        if v["queue_running"].as_array().is_some_and(|a| !a.is_empty()) { return Some("ComfyUI rechnet".into()); }
    }
    None
}

fn voicebox_bereit() -> bool { http(2).get(format!("{}/health", voicebox_url())).send().is_ok_and(|r| r.status().is_success()) }

fn paket(fid: &str) -> Option<&'static [u8]> { crate::oberflaeche::datei(&format!("stimmen/{fid}/stimme/{fid}.voicebox.zip")) }
fn voicebox_name(fid: &str) -> Option<String> {
    let z = crate::oberflaeche::datei("stimmen/zeilen.json")?;
    serde_json::from_slice::<Value>(z).ok()?[fid]["voicebox_name"].as_str().map(str::to_string)
}

/// Profil-Id der Figur in Voicebox; legt das Profil aus dem eingebetteten Paket an, wenn es fehlt.
fn voicebox_profil(fid: &str) -> Result<String, String> {
    let name = voicebox_name(fid).ok_or("Für diese Figur ist noch keine Stimme eingebettet.")?;
    let vb = voicebox_url();
    let profile: Value = http(10).get(format!("{vb}/profiles")).send().and_then(|r| r.json()).map_err(|e| format!("Voicebox: {e}"))?;
    if let Some(p) = profile.as_array().into_iter().flatten().find(|p| p["name"] == name.as_str()) { return Ok(p["id"].as_str().unwrap_or("").to_string()); }
    let zip = paket(fid).ok_or("Voicebox-Paket fehlt im Cockpit.")?;
    let grenze = "----sternenepoche-stimme-7f3a";
    let mut body = format!("--{grenze}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{fid}.voicebox.zip\"\r\nContent-Type: application/zip\r\n\r\n").into_bytes();
    body.extend_from_slice(zip);
    body.extend_from_slice(format!("\r\n--{grenze}--\r\n").as_bytes());
    let r = http(60).post(format!("{vb}/profiles/import")).header("Content-Type", format!("multipart/form-data; boundary={grenze}")).body(body).send().map_err(|e| format!("Voicebox-Import: {e}"))?;
    let status = r.status();
    let v: Value = r.json().unwrap_or(Value::Null);
    if !status.is_success() { return Err(format!("Voicebox-Import: {}", v["detail"].as_str().unwrap_or(status.as_str()))); }
    v["id"].as_str().map(str::to_string).ok_or_else(|| "Voicebox-Import ohne Profil-Id".into())
}

/// Alle Ratsstimmen als Profile in Voicebox anlegen (für Hermes und andere lokale Agenten).
pub fn voicebox_einrichten() {
    std::thread::spawn(|| {
        let setzen = |v: Value| if let Ok(mut d) = dienst().lock() { d["einrichten"] = v; };
        if !voicebox_bereit() { setzen(json!({"ok":false,"text":format!("Voicebox läuft nicht ({}). Erst Voicebox starten.", voicebox_url())})); return; }
        setzen(json!({"laeuft":true,"text":"Profile werden angelegt …"}));
        let mut ok = 0; let mut fehler = vec![];
        for fid in rat()["figuren"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
            if voicebox_name(&fid).is_none() { continue; }
            match voicebox_profil(&fid) { Ok(_) => ok += 1, Err(e) => fehler.push(format!("{fid}: {e}")) }
        }
        setzen(json!({"ok":fehler.is_empty(),"text":format!("{ok} Ratsstimmen in Voicebox bereit{}", if fehler.is_empty() { String::new() } else { format!(" · {}", fehler.join("; ")) })}));
    });
}

/// Die GPU teilen sich viele: Nach drei Minuten ohne weitere Antwort gibt Voicebox das TTS-Modell
/// (etwa 5 GB VRAM) wieder frei. Die nächste Antwort lädt es neu (einige Sekunden).
fn spaeter_entladen() {
    static LETZTE: OnceLock<Mutex<std::time::Instant>> = OnceLock::new();
    let letzte = LETZTE.get_or_init(|| Mutex::new(std::time::Instant::now()));
    if let Ok(mut l) = letzte.lock() { *l = std::time::Instant::now(); }
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        if letzte.lock().is_ok_and(|l| l.elapsed() >= Duration::from_secs(175)) {
            let _ = http(20).post(format!("{}/models/unload", voicebox_url())).send();
        }
    });
}

/// Ort der Loki-Stimmen-Pipeline (für die CPU-Ersatzstimme): nur über die Umgebungsvariable `LOKI_STIMMEN`.
/// Ohne sie gibt es keine CPU-Ersatzstimme, und die Antwort bleibt Text (kein fester Pfad im verteilten Programm).
fn loki_stimmen() -> Option<(PathBuf, PathBuf)> {
    let wurzel = PathBuf::from(std::env::var("LOKI_STIMMEN").ok().filter(|p| !p.trim().is_empty())?);
    let skript = wurzel.join("stimmen.py");
    if !skript.is_file() { return None; }
    let env = std::fs::read_to_string(wurzel.join(".env")).unwrap_or_default();
    let python = env.lines().find_map(|z| z.trim().strip_prefix("STIMMEN_PYTHON=")).map(|p| PathBuf::from(p.trim()))?;
    python.is_file().then_some((python, skript))
}

/// Text in der Stimme der Figur sprechen. Rückgabe: WAV und der benutzte Weg.
fn vertonen(fid: &str, text: &str, hinweis: impl Fn(&str)) -> Result<(Vec<u8>, &'static str), String> {
    let belegt = gpu_belegt();
    let vb = voicebox_bereit();
    if vb && belegt.is_none() {
        hinweis("Voicebox spricht …");
        let id = voicebox_profil(fid)?;
        let r = http(240).post(format!("{}/generate/stream", voicebox_url()))
            .json(&json!({"profile_id":id,"text":text,"language":"de","engine":"qwen","model_size":"1.7B"})).send().map_err(|e| format!("Voicebox: {e}"))?;
        if !r.status().is_success() { return Err(format!("Voicebox: {}", r.text().unwrap_or_default().chars().take(200).collect::<String>())); }
        let wav = r.bytes().map_err(|e| format!("Voicebox: {e}"))?.to_vec();
        spaeter_entladen();
        return if wav.starts_with(b"RIFF") { Ok((wav, "voicebox")) } else { Err("Voicebox lieferte kein WAV.".into()) };
    }
    let grund = match (&belegt, vb) { (Some(b), _) => format!("GPU belegt ({b})"), (None, false) => format!("Voicebox läuft nicht ({})", voicebox_url()), _ => String::new() };
    let Some((python, skript)) = loki_stimmen() else { return Err(format!("{grund} – Antwort nur als Text.")); };
    hinweis(&format!("{grund} – Loki-Stimmen spricht auf der CPU (etwa zwei Minuten) …"));
    let ordner = std::env::temp_dir().join(format!("sternenepoche-stimme-{}", std::process::id()));
    std::fs::create_dir_all(&ordner).map_err(|e| e.to_string())?;
    let n = crate::app::jetzt();
    let (txt, wav) = (ordner.join(format!("{n}.txt")), ordner.join(format!("{n}.wav")));
    std::fs::write(&txt, text).map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(&python);
    cmd.arg("-X").arg("utf8").arg(&skript).arg("sprechen").arg(format!("sternenepoche/{}", fid.replace('.', "-")))
        .arg("--textdatei").arg(&txt).arg("-o").arg(&wav).arg("--geraet").arg("cpu")
        .env("PYTHONUTF8", "1").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x0800_0000); }
    let aus = cmd.output().map_err(|e| format!("Loki-Stimmen: {e}"))?;
    let ergebnis = std::fs::read(&wav);
    let _ = std::fs::remove_file(&txt); let _ = std::fs::remove_file(&wav);
    match ergebnis {
        Ok(b) if b.starts_with(b"RIFF") => Ok((b, "loki-cpu")),
        _ => Err(format!("Loki-Stimmen: {}", String::from_utf8_lossy(&aus.stderr).lines().last().unwrap_or("kein Ton"))),
    }
}

/// Alle eingebetteten Stimmpakete (je Figur referenz.wav + Referenztext im Voicebox-Format) und die
/// Anleitung in einen Ordner schreiben, damit jede lokale KI dieselben Stimmen nutzen kann.
pub fn ausschreiben(ziel: &std::path::Path) -> Result<usize, String> {
    std::fs::create_dir_all(ziel).map_err(|e| e.to_string())?;
    let mut n = 0;
    for name in crate::oberflaeche::vorhanden("stimmen/") {
        let datei = name.rsplit('/').next().unwrap_or(name);
        if !(datei.ends_with(".voicebox.zip") || datei == "rat.json" || datei == "zeilen.json") { continue; }
        if let Some(b) = crate::oberflaeche::datei(name) { std::fs::write(ziel.join(datei), b).map_err(|e| e.to_string())?; n += 1; }
    }
    let text = ["Ratsstimmen der Sternenepoche", "",
        "Jede <rolle>.<volk>.voicebox.zip ist eine Stimme: samples/referenz.wav ist die Referenzaufnahme, samples.json enthält den gesprochenen Referenztext.",
        "- Voicebox: Profil importieren (Profile -> Importieren); danach sprechen Hermes und andere Agenten in dieser Stimme.",
        "- Qwen3-TTS / ComfyUI (VoiceClone): referenz.wav als ref_audio, Referenztext als ref_text.",
        "rat.json enthält Namen, Stimmbeschreibungen, Vorstellungen und alle Zeilen.", ""].join("\r\n");
    std::fs::write(ziel.join("LIESMICH.txt"), text).map_err(|e| e.to_string())?;
    Ok(n)
}

/// Zustand der Dienste für die Anzeige (wird selten erneuert, ohne die Oberfläche zu bremsen).
pub fn dienste_pruefen() {
    std::thread::spawn(|| {
        let v = json!({"voicebox":voicebox_bereit(),"voicebox_url":voicebox_url(),"gpu_belegt":gpu_belegt(),"loki_stimmen":loki_stimmen().is_some(),"geprueft":crate::app::jetzt()});
        if let Ok(mut d) = dienst().lock() { let e = d["einrichten"].clone(); *d = v; if !e.is_null() { d["einrichten"] = e; } }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saeubern_entfernt_denkspuren_und_regie() {
        assert_eq!(saeubern("<think>hm</think>**Ja.** (lächelt) Das *geht*. 🚀"), "Ja. Das geht.");
        assert!(saeubern(&"Wort. ".repeat(200)).chars().count() <= 420);
    }
    #[test]
    fn persona_hat_alle_figuren() {
        for v in ["veyari", "aurelianer", "krath", "syntheten"] { for r in ["stratege", "verwalter", "diplomat", "feldherr"] {
            let p = persona(&format!("{r}.{v}"), v, &json!({}));
            assert!(p.contains("Sprechweise") && !p.contains("Du bist ,"), "{r}.{v}");
        } }
    }
}
