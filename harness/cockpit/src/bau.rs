//! „Let's go“: alles Gewählte herunterladen, auf diesem Rechner bauen und lokal prüfen.
//! Jeder Schritt meldet Fortschritt und Protokoll; Downloads werden per SHA-256 geprüft.
use crate::abhaengigkeiten::{cargo_pfad, duckdb_paket, modell_da, ollama_modelle};
use crate::katalog::{Anbieter, Auswahl, Datenbank, Erweiterung};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

pub const SCHRITTE: [&str; 7] = ["Arbeitsordner vorbereiten", "Rust bereitstellen", "DuckDB laden und prüfen", "Modelle laden", "Harness bauen", "Konfiguration schreiben", "Lokale Prüfungen"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stand { Wartet, Laeuft, Fertig, Uebersprungen, Fehler }

pub enum Meldung { Schritt(usize, Stand, String), Fortschritt(usize, f32), Log(String), Pruefung(Value), Ende(Result<String, String>) }

pub fn exe_name() -> &'static str { if cfg!(windows) { "sternenepoche-harness.exe" } else { "sternenepoche-harness" } }

/// Vorgebauter Harness aus dem Download-Paket: sternenepoche-harness(.exe) und sternenepoche-harness.json
/// ({"quelle": QUELLE_HASH, "features": [...], "sha256": "..."}) neben dem Cockpit. Passt er zum eingebetteten
/// Quellcode und enthält alle gewählten Bausteine, wird er nur kopiert – der Spieler braucht weder Rust noch die
/// C++-Buildwerkzeuge. Selbst übersetzt wird nur auf Wunsch (Werkstatt → Bau, „selbst übersetzen“).
pub struct Vorgebaut { pub exe: PathBuf, pub sha256: String, pub features: Vec<String>, pub groesse: u64 }
pub fn vorgebaut() -> Option<Vorgebaut> {
    let ordner = std::env::var_os("STERNENEPOCHE_HARNESS_VORGEBAUT").map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)))?;
    let exe = ordner.join(exe_name());
    let m: Value = serde_json::from_slice(&std::fs::read(ordner.join("sternenepoche-harness.json")).ok()?).ok()?;
    if m["quelle"].as_str()? != crate::quelle::QUELLE_HASH { return None; }
    let groesse = std::fs::metadata(&exe).ok()?.len();
    Some(Vorgebaut { exe, sha256: m["sha256"].as_str()?.to_lowercase(), features: m["features"].as_array()?.iter().filter_map(|f| f.as_str().map(String::from)).collect(), groesse })
}
/// Der vorgebaute Harness für diese Auswahl – oder None (selbst übersetzen gewünscht, fehlt oder Bausteine fehlen).
pub fn vorgebaut_fuer(a: &Auswahl) -> Option<Vorgebaut> {
    if a.selbst_bauen { return None; }
    vorgebaut().filter(|v| a.features().iter().all(|f| v.features.iter().any(|g| g == f)))
}
fn datei_sha256(p: &Path) -> Result<String, String> {
    let mut f = std::fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let (mut hash, mut puffer) = (Sha256::new(), vec![0u8; 256 * 1024]);
    loop { let n = f.read(&mut puffer).map_err(|e| e.to_string())?; if n == 0 { break; } hash.update(&puffer[..n]); }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn kommando(programm: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut c = Command::new(programm);
    #[cfg(windows)]
    { use std::os::windows::process::CommandExt; c.creation_flags(0x0800_0000); }
    c
}

/// Ein Bau meldet an die Oberfläche und schreibt dasselbe Protokoll nach `bau.log` im
/// Arbeitsordner: Nach einem Fehlschlag (oder einem geschlossenen Fenster) bleibt nachlesbar,
/// was passiert ist.
struct Lauf { tx: Sender<Meldung>, datei: std::sync::Mutex<Option<std::fs::File>> }
impl Lauf {
    fn datei(&self, zeile: &str) {
        if let Some(f) = self.datei.lock().unwrap_or_else(|p| p.into_inner()).as_mut() { let _ = writeln!(f, "{zeile}"); }
    }
    fn schritt(&self, i: usize, s: Stand, t: impl Into<String>) {
        let t = t.into();
        self.datei(&format!("[{}] {:?} {t}", SCHRITTE.get(i).unwrap_or(&"?"), s));
        let _ = self.tx.send(Meldung::Schritt(i, s, t));
    }
    fn log(&self, t: impl Into<String>) { let t = t.into(); self.datei(&t); let _ = self.tx.send(Meldung::Log(t)); }
    fn fortschritt(&self, i: usize, f: f32) { let _ = self.tx.send(Meldung::Fortschritt(i, f)); }
}

pub fn starten(a: Auswahl, openrouter: String, modellprobe: bool) -> Receiver<Meldung> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(a.ordner());
        let datei = std::fs::File::create(a.ordner().join("bau.log")).ok();
        let l = Lauf { tx: tx.clone(), datei: std::sync::Mutex::new(datei) };
        l.datei(&format!("Bau {} · Cockpit {} · {} · Bausteine {}", crate::app::zeitstempel(crate::app::jetzt()), env!("CARGO_PKG_VERSION"), std::env::consts::OS, a.features().join(",")));
        // Ein Programmfehler im Bau endet als lesbare Fehlermeldung statt als hängender Schritt.
        let ende = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ablauf(&l, &a, &openrouter, modellprobe)))
            .unwrap_or_else(|p| Err(format!("Interner Fehler beim Bau: {}", crate::absturz::nutzlast(p.as_ref()))));
        match &ende { Err(e) => l.log(format!("Abbruch: {e}")), Ok(_) => l.datei("Fertig.") }
        let _ = tx.send(Meldung::Ende(ende));
    });
    rx
}

fn herunterladen(l: &Lauf, schritt: usize, url: &str, ziel: &Path) -> Result<String, String> {
    let mut r = reqwest::blocking::Client::builder().timeout(Duration::from_secs(900)).build().map_err(|e| e.to_string())?
        .get(url).send().map_err(|e| format!("Download {url}: {}", e.without_url()))?.error_for_status().map_err(|e| e.to_string())?;
    let gesamt = r.content_length().unwrap_or(0);
    let mut datei = std::fs::File::create(ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
    let mut hash = Sha256::new();
    let (mut puffer, mut gelesen) = (vec![0u8; 64 * 1024], 0u64);
    loop {
        let n = r.read(&mut puffer).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hash.update(&puffer[..n]);
        datei.write_all(&puffer[..n]).map_err(|e| e.to_string())?;
        gelesen += n as u64;
        if gesamt > 0 { l.fortschritt(schritt, gelesen as f32 / gesamt as f32); }
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn ablauf(l: &Lauf, a: &Auswahl, openrouter: &str, modellprobe: bool) -> Result<String, String> {
    let ordner = a.ordner();
    // 1 Arbeitsordner und Quellcode
    l.schritt(0, Stand::Laeuft, ordner.display().to_string());
    let quelle = ordner.join("quelle");
    for o in [&ordner, &quelle, &ordner.join("bin"), &ordner.join("plugins"), &ordner.join("tmp")] { std::fs::create_dir_all(o).map_err(|e| format!("{}: {e}", o.display()))?; }
    for alt in ["src", "skills"] { let _ = std::fs::remove_dir_all(quelle.join(alt)); }
    for (name, bytes) in crate::quelle::DATEIEN {
        let p = quelle.join(name);
        if let Some(o) = p.parent() { std::fs::create_dir_all(o).map_err(|e| e.to_string())?; }
        std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    if a.erweiterung == Erweiterung::WasmSandbox {
        if let Some(p) = crate::quelle::BEISPIEL_PLUGIN { std::fs::write(ordner.join("plugins/energiewaechter.wasm"), p).map_err(|e| e.to_string())?; }
    }
    l.schritt(0, Stand::Fertig, format!("{} Quelldateien (Stand {})", crate::quelle::DATEIEN.len(), crate::quelle::QUELLE_HASH));

    // 2 Rust – entfällt mit dem vorgebauten Harness
    let vor = crate::bau::vorgebaut_fuer(a);
    let cargo = if vor.is_some() { l.schritt(1, Stand::Uebersprungen, "nicht nötig – vorgebauter Harness liegt bei"); None } else {
    l.schritt(1, Stand::Laeuft, "");
    let cargo = match cargo_pfad() {
        Some(c) => c,
        None if cfg!(all(windows, target_arch = "x86_64")) => {
            l.log("Rust fehlt: lade rustup-init von static.rust-lang.org");
            let ziel = ordner.join("tmp/rustup-init.exe");
            herunterladen(l, 1, "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe", &ziel)?;
            let s = kommando(&ziel).args(["-y", "--profile", "minimal", "--default-toolchain", "stable"]).status().map_err(|e| e.to_string())?;
            if !s.success() { return Err("rustup-init ist fehlgeschlagen".into()); }
            cargo_pfad().ok_or("Rust installiert, aber cargo nicht gefunden – Cockpit neu starten")?
        }
        None => return Err("Rust fehlt: bitte über https://rustup.rs installieren".into()),
    };
    l.schritt(1, Stand::Fertig, cargo.display().to_string());
    Some(cargo) };

    // 3 DuckDB
    if a.datenbank == Datenbank::DuckDb {
        l.schritt(2, Stand::Laeuft, "");
        let p = duckdb_paket().ok_or("Keine offizielle DuckDB-Laufzeit für dieses System")?;
        let ziel_dll = ordner.join("bin").join(p.datei);
        if ziel_dll.exists() { l.schritt(2, Stand::Fertig, format!("{} vorhanden", p.datei)); } else {
            let zip_pfad = ordner.join("tmp/duckdb.zip");
            let hash = herunterladen(l, 2, p.url, &zip_pfad)?;
            if hash != p.sha256 { let _ = std::fs::remove_file(&zip_pfad); return Err(format!("DuckDB-Prüfsumme falsch: {hash} statt {}", p.sha256)); }
            let mut z = zip::ZipArchive::new(std::fs::File::open(&zip_pfad).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let mut eintrag = z.by_name(p.datei).map_err(|_| format!("{} fehlt im Archiv", p.datei))?;
            let mut aus = std::fs::File::create(&ziel_dll).map_err(|e| e.to_string())?;
            std::io::copy(&mut eintrag, &mut aus).map_err(|e| e.to_string())?;
            l.schritt(2, Stand::Fertig, format!("SHA-256 bestätigt ({}…), {} entpackt", &p.sha256[..16], p.datei));
        }
    } else { l.schritt(2, Stand::Uebersprungen, "SQLite gewählt"); }

    // 4 Modelle
    if a.nutzt(Anbieter::Ollama) {
        l.schritt(3, Stand::Laeuft, "");
        let vorhanden = ollama_modelle(&a.ollama_url)?;
        let mut fehlend: Vec<String> = a.aktive().iter().map(|r| a.rollen[r].clone()).filter(|w| w.anbieter == Anbieter::Ollama).map(|w| w.modell).filter(|m| modell_da(&vorhanden, m).is_none()).collect();
        fehlend.dedup();
        for m in &fehlend {
            l.log(format!("ollama pull {m}"));
            let r = reqwest::blocking::Client::builder().timeout(Duration::from_secs(7200)).build().map_err(|e| e.to_string())?
                .post(format!("{}/api/pull", a.ollama_url.trim_end_matches('/'))).json(&json!({"model":m,"stream":true})).send().map_err(|e| e.to_string())?;
            for zeile in BufReader::new(r).lines().map_while(Result::ok) {
                let v: Value = serde_json::from_str(&zeile).unwrap_or(Value::Null);
                if let Some(e) = v["error"].as_str() { return Err(format!("Ollama: {e}")); }
                if let (Some(t), Some(c)) = (v["total"].as_u64(), v["completed"].as_u64()) { if t > 0 { l.fortschritt(3, c as f32 / t as f32); } }
            }
        }
        l.schritt(3, Stand::Fertig, if fehlend.is_empty() { "alle Modelle vorhanden".into() } else { format!("geladen: {}", fehlend.join(", ")) });
    } else { l.schritt(3, Stand::Uebersprungen, "nur OpenRouter"); }
    if a.nutzt(Anbieter::OpenRouter) && !openrouter.is_empty() { crate::tresor::schreiben("openrouter", openrouter)?; }

    // 5 Bauen
    let kennung = a.bau_kennung();
    let exe = ordner.join("bin").join(exe_name());
    if a.gebaut.as_deref() == Some(kennung.as_str()) && exe.exists() { l.schritt(4, Stand::Uebersprungen, "bereits mit dieser Auswahl gebaut"); }
    else if let Some(v) = &vor {
        l.schritt(4, Stand::Laeuft, format!("vorgebauten Harness übernehmen ({:.1} MB)", v.groesse as f64 / 1e6));
        let ist = datei_sha256(&v.exe)?;
        if ist != v.sha256 { return Err(format!("Vorgebauter Harness beschädigt: SHA-256 {ist} statt {}", v.sha256)); }
        std::fs::copy(&v.exe, &exe).map_err(|e| format!("Installieren: {e}"))?;
        // macOS: Die Kopie erbt das Quarantäne-Merkmal des Downloads; Gatekeeper würde den
        // (geprüften) Harness sonst beim ersten Start blockieren, obwohl die App freigegeben ist.
        #[cfg(target_os = "macos")]
        { let _ = kommando("xattr").arg("-d").arg("com.apple.quarantine").arg(&exe).stderr(Stdio::null()).status(); }
        l.schritt(4, Stand::Fertig, format!("vorgebaut, SHA-256 geprüft ({}…) · Bausteine {}", &ist[..16], v.features.join(", ")));
    } else {
        let cargo = cargo.as_ref().ok_or("Rust fehlt")?;
        l.schritt(4, Stand::Laeuft, format!("cargo build --release --locked --features {}", a.features().join(",")));
        let pakete = crate::quelle::DATEIEN.iter().find(|(n, _)| *n == "Cargo.lock").map(|(_, b)| String::from_utf8_lossy(b).matches("[[package]]").count()).unwrap_or(150).max(1);
        let mut kind = kommando(cargo).args(["build", "--release", "--locked", "--no-default-features", "--features", &a.features().join(","), "--manifest-path"])
            .arg(quelle.join("Cargo.toml")).env("CARGO_TARGET_DIR", ordner.join("bau")).env("CARGO_TERM_COLOR", "never")
            .stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| format!("cargo: {e}"))?;
        let mut zaehler = 0usize;
        for zeile in BufReader::new(kind.stderr.take().unwrap()).lines().map_while(Result::ok) {
            if zeile.trim_start().starts_with("Compiling") { zaehler += 1; l.fortschritt(4, (zaehler as f32 / pakete as f32).min(0.98)); }
            l.log(zeile);
        }
        if !kind.wait().map_err(|e| e.to_string())?.success() { return Err("Bau fehlgeschlagen – Protokoll ansehen".into()); }
        std::fs::copy(ordner.join("bau/release").join(exe_name()), &exe).map_err(|e| format!("Installieren: {e}"))?;
        l.schritt(4, Stand::Fertig, format!("{} Pakete übersetzt, {}", zaehler, exe.display()));
    }

    // 6 Konfiguration (ohne Geheimnisse)
    l.schritt(5, Stand::Laeuft, "");
    let konfig = ordner.join("harness.json");
    std::fs::write(&konfig, serde_json::to_string_pretty(&a.harness_json()).unwrap()).map_err(|e| e.to_string())?;
    l.schritt(5, Stand::Fertig, "harness.json geschrieben – Schlüssel bleiben im Anmeldeinformationsspeicher");

    // 7 Lokale Prüfungen
    l.schritt(6, Stand::Laeuft, if modellprobe { "mit Modellprobe" } else { "" });
    let schluessel = crate::tresor::lesen(&crate::tresor::harness_name(&a.server)).unwrap_or_default();
    let mut test = kommando(&exe);
    test.arg("selbsttest").arg("--konfig").arg(&konfig).arg("--geheimnisse-stdin").current_dir(&ordner).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    if modellprobe { test.arg("--mit-modell"); }
    let mut kind = test.spawn().map_err(|e| e.to_string())?;
    let wissen: std::collections::BTreeMap<String, String> = a.wissen.iter().filter(|w| w.mit_schluessel)
        .filter_map(|w| crate::tresor::lesen(&crate::tresor::wissen_name(&w.name)).map(|s| (w.name.clone(), s))).collect();
    writeln!(kind.stdin.take().unwrap(), "{}", json!({"schluessel":schluessel,"openrouter":openrouter,"openai":crate::tresor::lesen("eigen").unwrap_or_default(),"wissen":wissen})).map_err(|e| e.to_string())?;
    let mut fehl: Vec<String> = Vec::new();
    for zeile in BufReader::new(kind.stdout.take().unwrap()).lines().map_while(Result::ok) {
        if let Ok(v) = serde_json::from_str::<Value>(&zeile) {
            if v["pruefung"].is_string() {
                if v["ok"] != true { fehl.push(format!("{}: {}", v["pruefung"].as_str().unwrap_or("?"), v["text"].as_str().unwrap_or(""))); l.log(format!("Prüfung fehlgeschlagen – {}", fehl.last().unwrap())); }
                let _ = l.tx.send(Meldung::Pruefung(v));
            }
        }
    }
    let status = kind.wait().map_err(|e| e.to_string())?;
    if fehl.is_empty() && !status.success() { fehl.push(format!("Selbsttest endete mit {status}")); }
    // Ein gespeicherter Harness-Schlüssel, den der Server ablehnt (widerrufen, abgelaufen, frühere Welt), blockiert den
    // Bau nicht: Er ist ohnehin wertlos, wird aus dem Tresor entfernt, und LOS holt einen neuen.
    let vorher = fehl.len();
    fehl.retain(|f| !(f.starts_with("schluessel:") && f.contains("401")));
    if fehl.len() < vorher {
        crate::tresor::loeschen(&crate::tresor::harness_name(&a.server));
        l.log("Gespeicherter Harness-Schlüssel gilt nicht mehr (widerrufen oder abgelaufen) – entfernt; LOS holt einen neuen.");
    }
    // Ein gerade nicht erreichbarer Spielserver ist kein Installationsfehler: Der Harness ist
    // installiert und geprüft; LOS prüft die Verbindung erneut. Der Singleplayer braucht ihn nie.
    let (server, rest): (Vec<String>, Vec<String>) = fehl.into_iter().partition(|f| f.starts_with("server:"));
    fehl = rest;
    for f in &server { l.log(format!("Hinweis: Spielserver gerade nicht erreichbar ({f}). Die Installation ist fertig; LOS prüft die Verbindung erneut.")); }
    if !fehl.is_empty() { l.schritt(6, Stand::Fehler, format!("{} Prüfung(en) fehlgeschlagen", fehl.len())); return Err(format!("Lokale Prüfungen nicht bestanden – {}", fehl.join(" · "))); }
    l.schritt(6, Stand::Fertig, "alle Prüfungen bestanden");
    Ok(kennung)
}

pub fn harness_exe(ordner: &Path) -> PathBuf { ordner.join("bin").join(exe_name()) }
