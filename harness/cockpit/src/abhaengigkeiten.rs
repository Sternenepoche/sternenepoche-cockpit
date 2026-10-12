//! Was der gewählte Harness braucht, und ob es schon da ist. Grundlage für die Liste
//! nach „Fertig“ und für die Schritte nach „Let's go“.
use crate::katalog::{Anbieter, Auswahl, Datenbank, Erweiterung, Schnittstelle};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stand { Vorhanden, WirdGeladen, Manuell, Fehlt }

#[derive(Clone, Debug)]
pub struct Abh { pub name: String, pub stand: Stand, pub detail: String, pub groesse: String, pub blockiert: bool, pub hilfe: Option<String> }
fn abh(name: &str, stand: Stand, detail: impl Into<String>, groesse: &str) -> Abh {
    Abh { name: name.into(), stand, detail: detail.into(), groesse: groesse.into(), blockiert: matches!(stand, Stand::Manuell | Stand::Fehlt), hilfe: None }
}

/// Offizielle DuckDB-1.5.6-Laufzeit; Prüfsummen laut GitHub-Release (28.09.2026).
pub struct DuckDbPaket { pub url: &'static str, pub sha256: &'static str, pub groesse_mb: f32, pub datei: &'static str }
pub fn duckdb_paket() -> Option<DuckDbPaket> {
    let (name, sha256, groesse_mb, datei) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => ("libduckdb-windows-amd64.zip", "44cf59583f9951d2cb09b1bf115a63ecb2d8901e363903029d86c7d8683fe96a", 13.4, "duckdb.dll"),
        ("windows", "aarch64") => ("libduckdb-windows-arm64.zip", "c63285647e94c46d309b60d5dd1ff3cb2cdf9ca03e415a818ff7d8dbf838e0e8", 14.4, "duckdb.dll"),
        ("linux", "x86_64") => ("libduckdb-linux-amd64.zip", "b845005f5132a7d8180057c35e14a7626632258782f871a90861b19c1c03841b", 41.4, "libduckdb.so"),
        ("linux", "aarch64") => ("libduckdb-linux-arm64.zip", "b72ed9f05003f5e9d2015f7ceada6416b377d9dd33169cdde3c9e33897856eee", 37.8, "libduckdb.so"),
        ("macos", _) => ("libduckdb-osx-universal.zip", "e0bc007d9b0094c0970ac1847a8601d10aad07cbd2910ce586ec810f77b638d6", 36.2, "libduckdb.dylib"),
        _ => return None,
    };
    let url = Box::leak(format!("https://github.com/duckdb/duckdb/releases/download/v1.5.6/{name}").into_boxed_str());
    Some(DuckDbPaket { url, sha256, groesse_mb, datei })
}

pub fn cargo_pfad() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    if Command::new(exe).arg("--version").output().is_ok_and(|o| o.status.success()) { return Some(PathBuf::from(exe)); }
    let heim = std::env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| PathBuf::from(h).join(".cargo")))?;
    let p = heim.join("bin").join(exe);
    p.exists().then_some(p)
}
fn ausgabe(cmd: &mut Command) -> Option<String> { cmd.output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()) }

#[cfg(windows)]
fn msvc() -> Option<String> {
    let vswhere = PathBuf::from(std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into())).join(r"Microsoft Visual Studio\Installer\vswhere.exe");
    ausgabe(Command::new(vswhere).args(["-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "displayName"])).filter(|s| !s.is_empty())
}

#[cfg(windows)]
pub fn freier_platz(ordner: &Path) -> Option<u64> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut p = ordner.to_path_buf();
    while !p.exists() { p = p.parent()?.to_path_buf(); }
    let breit: Vec<u16> = p.as_os_str().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    let mut frei = 0u64;
    (unsafe { GetDiskFreeSpaceExW(breit.as_ptr(), &mut frei, std::ptr::null_mut(), std::ptr::null_mut()) } != 0).then_some(frei)
}
#[cfg(not(windows))]
pub fn freier_platz(_: &Path) -> Option<u64> { None }

pub fn ollama_modelle(url: &str) -> Result<Vec<(String, u64)>, String> {
    let v: Value = reqwest::blocking::Client::new().get(format!("{}/api/tags", url.trim_end_matches('/'))).timeout(Duration::from_secs(5)).send()
        .map_err(|_| "Ollama läuft nicht".to_string())?.json().map_err(|e| e.to_string())?;
    Ok(v["models"].as_array().into_iter().flatten().filter_map(|m| Some((m["name"].as_str()?.to_string(), m["size"].as_u64().unwrap_or(0)))).collect())
}
/// Grafikkarte und Videospeicher (NVIDIA), damit das Cockpit sagen kann, welches Modell passt.
pub fn gpu() -> Value {
    let aus = crate::bau::kommando("nvidia-smi").args(["--query-gpu=name,memory.total,memory.used", "--format=csv,noheader,nounits"]).output();
    match aus {
        Ok(o) if o.status.success() => {
            let t = String::from_utf8_lossy(&o.stdout);
            let z: Vec<&str> = t.lines().next().unwrap_or("").split(',').map(str::trim).collect();
            serde_json::json!({"name":z.first(),"vram_mb":z.get(1).and_then(|x| x.parse::<u64>().ok()),"belegt_mb":z.get(2).and_then(|x| x.parse::<u64>().ok())})
        }
        _ => serde_json::json!({"name":null}),
    }
}
pub fn modell_da(vorhanden: &[(String, u64)], modell: &str) -> Option<u64> {
    vorhanden.iter().find(|(n, _)| n == modell || n.trim_end_matches(":latest") == modell.trim_end_matches(":latest")).map(|(_, g)| *g)
}
pub fn gb(b: u64) -> String { format!("{:.1} GB", b as f64 / 1e9) }

/// Vorschlag aus den installierten Modellen: das größte Chat-Modell, das in den Videospeicher passt
/// (ohne Einbettungs-, Bild- und Spezialmodelle). Ohne GPU-Angabe: das größte bis 9 GB.
pub fn empfehlung(vorhanden: &[(String, u64)], vram_mb: Option<u64>) -> Option<String> {
    let grenze = vram_mb.map(|v| v * 1_000_000 * 85 / 100).unwrap_or(9_000_000_000);
    let chat = |n: &str| { let n = n.to_lowercase(); !["embed", "-vl", "vision", "ocr", "coder", "summary", "rerank", "whisper", "tts", "cpu"].iter().any(|x| n.contains(x)) };
    vorhanden.iter().filter(|(n, g)| *g > 500_000_000 && *g <= grenze && chat(n)).max_by_key(|(_, g)| *g).map(|(n, _)| n.clone())
}

/// Downloadgröße eines Ollama-Modells aus dem Manifest der Ollama-Bibliothek (nur Metadaten).
pub fn modell_groesse(modell: &str) -> Option<u64> {
    let (name, tag) = modell.split_once(':').unwrap_or((modell, "latest"));
    let pfad = if name.contains('/') { name.to_string() } else { format!("library/{name}") };
    let v: Value = reqwest::blocking::Client::new().get(format!("https://registry.ollama.ai/v2/{pfad}/manifests/{tag}"))
        .header("Accept", "application/vnd.docker.distribution.manifest.v2+json").timeout(Duration::from_secs(6)).send().ok()?.json().ok()?;
    let s: u64 = v["layers"].as_array()?.iter().filter_map(|l| l["size"].as_u64()).sum();
    (s > 0).then_some(s)
}

/// Modellnamen aus dem öffentlichen Katalog von OpenRouter (None, wenn nicht erreichbar).
fn openrouter_katalog() -> Option<Vec<String>> {
    let v: serde_json::Value = reqwest::blocking::Client::builder().timeout(Duration::from_secs(8)).build().ok()?
        .get("https://openrouter.ai/api/v1/models").send().ok()?.json().ok()?;
    Some(v["data"].as_array()?.iter().filter_map(|m| m["id"].as_str().map(String::from)).collect())
}

/// Wohin Ollama Modelle lädt (Umgebungsvariable OLLAMA_MODELS, sonst der Standardordner im Profil).
pub fn ollama_ordner() -> std::path::PathBuf {
    std::env::var_os("OLLAMA_MODELS").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| std::path::PathBuf::from(h).join(".ollama").join("models")))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub fn ermitteln(a: &Auswahl, openrouter: &str) -> Vec<Abh> {
    let ordner = a.ordner();
    let mut liste = Vec::new();
    let vor = crate::bau::vorgebaut_fuer(a);
    if let Some(v) = &vor {
        let gebaut = a.gebaut.as_deref() == Some(a.bau_kennung().as_str()) && ordner.join("bin").join(crate::bau::exe_name()).exists();
        liste.push(abh("Harness (vorgebaut)", if gebaut { Stand::Vorhanden } else { Stand::WirdGeladen },
            format!("liegt dem Cockpit bei, SHA-256 wird geprüft · Bausteine {} · Rust und C++-Buildwerkzeuge nicht nötig", v.features.join(", ")),
            if gebaut { "".to_string() } else { format!("{:.1} MB, wird kopiert", v.groesse as f64 / 1e6) }.as_str()));
    } else {
    match cargo_pfad().and_then(|c| ausgabe(Command::new(c).arg("--version"))) {
        Some(v) => liste.push(abh("Rust-Werkzeugkette", Stand::Vorhanden, v, "")),
        None if cfg!(windows) => liste.push(abh("Rust-Werkzeugkette", Stand::WirdGeladen, "rustup-init von static.rust-lang.org, Profil minimal", "≈ 300 MB")),
        None => { let mut x = abh("Rust-Werkzeugkette", Stand::Manuell, "Installieren mit: curl https://sh.rustup.rs -sSf | sh", "≈ 300 MB"); x.hilfe = Some("https://rustup.rs".into()); liste.push(x); }
    }
    #[cfg(windows)]
    match msvc() {
        Some(v) => liste.push(abh("C++-Buildwerkzeuge (Linker)", Stand::Vorhanden, v, "")),
        None => { let mut x = abh("C++-Buildwerkzeuge (Linker)", Stand::Manuell, "winget install Microsoft.VisualStudio.2022.BuildTools --override \"--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended\"", "≈ 4 GB"); x.hilfe = Some("https://visualstudio.microsoft.com/de/visual-cpp-build-tools/".into()); liste.push(x); }
    }
    let pakete = crate::quelle::DATEIEN.iter().find(|(n, _)| *n == "Cargo.lock").map(|(_, b)| String::from_utf8_lossy(b).matches("[[package]]").count()).unwrap_or(0);
    let gebaut = a.gebaut.as_deref() == Some(a.bau_kennung().as_str()) && ordner.join("bin").join(if cfg!(windows) { "sternenepoche-harness.exe" } else { "sternenepoche-harness" }).exists();
    liste.push(abh("Harness bauen", if gebaut { Stand::Vorhanden } else { Stand::WirdGeladen },
        format!("Bausteine: {} · bis zu {pakete} geprüfte Bibliotheken von crates.io (Cargo.lock)", a.features().join(", ")), if gebaut { "" } else { "≈ 150 MB Download, 1 GB Bauordner" }));
    }
    if a.datenbank == Datenbank::DuckDb {
        match duckdb_paket() {
            Some(p) if ordner.join("bin").join(p.datei).exists() => liste.push(abh("DuckDB 1.5.6 Laufzeit", Stand::Vorhanden, "bin/".to_string() + p.datei, "")),
            Some(p) => liste.push(abh("DuckDB 1.5.6 Laufzeit", Stand::WirdGeladen, format!("{} · SHA-256 {}…", p.url, &p.sha256[..16]), &format!("{:.1} MB", p.groesse_mb))),
            None => liste.push(abh("DuckDB 1.5.6 Laufzeit", Stand::Fehlt, "Für dieses System gibt es keine offizielle Laufzeit; SQLite wählen", "")),
        }
    } else {
        liste.push(abh("SQLite", Stand::Vorhanden, "wird in den Harness eingebaut", ""));
    }
    if a.nutzt(Anbieter::Ollama) {
        match ollama_modelle(&a.ollama_url) {
            Err(_) => { let mut x = abh("Ollama", Stand::Manuell, format!("nicht erreichbar unter {} – installieren und starten", a.ollama_url), ""); x.hilfe = Some("https://ollama.com/download".into()); liste.push(x); }
            Ok(vorhanden) => {
                liste.push(abh("Ollama", Stand::Vorhanden, format!("{} {} unter {}", vorhanden.len(), if vorhanden.len() == 1 { "lokales Modell" } else { "lokale Modelle" }, a.ollama_url), ""));
                let mut gesehen = Vec::new();
                for r in a.aktive() {
                    let w = &a.rollen[&r];
                    if w.anbieter != Anbieter::Ollama || gesehen.contains(&w.modell) { continue; }
                    gesehen.push(w.modell.clone());
                    liste.push(match modell_da(&vorhanden, &w.modell) {
                        Some(g) => abh(&format!("Modell {}", w.modell), Stand::Vorhanden, "lokal vorhanden", &gb(g)),
                        None => {
                            let ziel = ollama_ordner();
                            let groesse = modell_groesse(&w.modell);
                            let frei = freier_platz(&ziel);
                            let groesse_text = groesse.map(gb).unwrap_or_else(|| "Größe unbekannt".into());
                            match (groesse, frei) {
                                (_, None) => abh(&format!("Modell {}", w.modell), Stand::WirdGeladen, format!("nicht installiert – „Let's go“ lädt es mit ollama pull nach {}", ziel.display()), &groesse_text),
                                (Some(g), Some(f)) if f < g + 10_000_000_000 => abh(&format!("Modell {}", w.modell), Stand::Fehlt,
                                    format!("nicht installiert – zu wenig Platz in {} ({} frei, nötig {} plus 10 GB Reserve). Anderes Modell wählen.", ziel.display(), gb(f), gb(g)), &groesse_text),
                                (_, Some(f)) => abh(&format!("Modell {}", w.modell), Stand::WirdGeladen, format!("nicht installiert – „Let's go“ lädt es mit ollama pull nach {} ({} frei)", ziel.display(), gb(f)), &groesse_text),
                            }
                        }
                    });
                }
            }
        }
    }
    if a.nutzt(Anbieter::OpenRouter) {
        liste.push(if openrouter.is_empty() { abh("OpenRouter-Schlüssel", Stand::Fehlt, "unten eingeben; wird im Anmeldeinformationsspeicher abgelegt", "") }
            else { abh("OpenRouter-Schlüssel", Stand::Vorhanden, format!("Budget {:.2} USD, Daten dürfen nicht gespeichert werden", a.openrouter_budget), "") });
        // Gibt es die gewählten Modelle bei OpenRouter? (öffentlicher Katalog, ohne Schlüssel)
        let mut gewaehlt: Vec<String> = a.aktive().iter().map(|r| &a.rollen[r.as_str()]).filter(|w| w.anbieter == Anbieter::OpenRouter).map(|w| w.modell.trim().to_string()).collect();
        gewaehlt.sort(); gewaehlt.dedup();
        if let Some(katalog) = openrouter_katalog() {
            for m in gewaehlt {
                liste.push(if katalog.iter().any(|k| *k == m) { abh(&format!("OpenRouter-Modell {m}"), Stand::Vorhanden, "im Katalog von openrouter.ai", "") }
                    else { let mut x = abh(&format!("OpenRouter-Modell {m}"), Stand::Fehlt, "gibt es bei OpenRouter nicht – Namen auf openrouter.ai/models nachsehen und im Rat oder im Assistenten eintragen", ""); x.hilfe = Some("https://openrouter.ai/models".into()); x });
            }
        }
    }
    if a.erweiterung == Erweiterung::WasmSandbox {
        liste.push(abh("WASM-Sandbox", Stand::Vorhanden, "Wasmi-Interpreter mit Treibstoff- und Speichergrenze, Beispiel-Plugin Energiewächter", ""));
    }
    if a.schnittstelle == Schnittstelle::LokaleApiMcp {
        liste.push(abh("Lokale API und MCP", Stand::Vorhanden, "nur 127.0.0.1:8895, Zufallsschlüssel in lokale-api.json, keine Browseranfragen", ""));
    }
    // Selbst übersetzen braucht Platz für den Bauordner; der vorgebaute Harness nur für sich und das Gedächtnis.
    let noetig: u64 = if vor.is_some() { 300_000_000 } else { 3_000_000_000 };
    match freier_platz(&ordner) {
        Some(frei) if frei < noetig => liste.push(abh("Speicherplatz", Stand::Fehlt, format!("{} frei in {} – mindestens {} nötig", gb(frei), ordner.display(), gb(noetig)), "")),
        Some(frei) => liste.push(abh("Speicherplatz", Stand::Vorhanden, format!("{} frei in {}", gb(frei), ordner.display()), "")),
        None => liste.push(abh("Speicherplatz", Stand::Vorhanden, format!("nicht geprüft · {}", ordner.display()), "")),
    }
    liste
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empfehlung_nimmt_installiertes_chatmodell_das_passt() {
        let v: Vec<(String, u64)> = vec![("thinkingcap-27b:latest".into(), 15_309_040_147), ("werkstatt-vl-27b:latest".into(), 16_407_098_445), ("qwen3.5:4b".into(), 3_397_841_620),
            ("nemotron-3-embed:1b-q4_k_m".into(), 749_352_367), ("loki:summary-4b".into(), 3_397_841_673), ("gemma3:1b".into(), 815_319_791)];
        assert_eq!(empfehlung(&v, Some(16376)).as_deref(), Some("qwen3.5:4b"));
        assert_eq!(empfehlung(&v, Some(24000)).as_deref(), Some("thinkingcap-27b:latest"));
        assert_eq!(empfehlung(&[], Some(16376)), None);
    }
}
