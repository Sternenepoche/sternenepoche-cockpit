//! Bettet den Quellcode des Harness (harness/runtime) und das Beispiel-Plugin ein.
//! Der Builder schreibt ihn auf dem Rechner des Spielers aus und baut ihn dort mit
//! genau den gewählten Bausteinen.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn sammeln(basis: &Path, rel: &Path, out: &mut Vec<(String, PathBuf)>) {
    let ordner = basis.join(rel);
    let mut eintraege: Vec<_> = std::fs::read_dir(&ordner).unwrap_or_else(|e| panic!("{}: {e}", ordner.display())).filter_map(Result::ok).collect();
    eintraege.sort_by_key(|e| e.file_name());
    for e in eintraege {
        let name = e.file_name().to_string_lossy().into_owned();
        let pfad = rel.join(&name);
        if e.path().is_dir() { sammeln(basis, &pfad, out); }
        else if name.ends_with(".rs") || name.ends_with(".md") { out.push((pfad.to_string_lossy().replace('\\', "/"), e.path())); }
    }
}

fn main() {
    let hier = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let runtime = hier.join("../runtime").canonicalize().expect("harness/runtime fehlt");
    let mut dateien = vec![("Cargo.toml".to_string(), runtime.join("Cargo.toml")), ("Cargo.lock".to_string(), runtime.join("Cargo.lock"))];
    sammeln(&runtime, Path::new("src"), &mut dateien);
    sammeln(&runtime, Path::new("skills"), &mut dateien);
    let mut hash = Sha256::new();
    let mut code = String::from("pub const DATEIEN: &[(&str, &[u8])] = &[\n");
    for (name, pfad) in &dateien {
        println!("cargo:rerun-if-changed={}", pfad.display());
        hash.update(name.as_bytes());
        hash.update(std::fs::read(pfad).unwrap());
        code.push_str(&format!("    ({name:?}, include_bytes!({:?})),\n", pfad.display().to_string()));
    }
    code.push_str("];\n");
    let h: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    code.push_str(&format!("pub const QUELLE_HASH: &str = {:?};\n", &h[..16]));
    let plugin = hier.join("../beispiele/energiewaechter/target/wasm32-unknown-unknown/release/energiewaechter.wasm");
    println!("cargo:rerun-if-changed={}", plugin.display());
    if plugin.exists() {
        code.push_str(&format!("pub const BEISPIEL_PLUGIN: Option<&[u8]> = Some(include_bytes!({:?}));\n", plugin.display().to_string()));
    } else {
        code.push_str("pub const BEISPIEL_PLUGIN: Option<&[u8]> = None;\n");
    }
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("quelle.rs"), code).unwrap();
    oberflaeche(&hier);
}

/// Die Oberfläche des Cockpits (ui/), die Bilder des Spiels (Porträts, Schiffe, Gebäude,
/// Planeten, Ressourcen …) und die 3D-Figuren des Rats werden eingebettet: das Cockpit
/// zeigt sie ohne Netz und ohne lose Dateien.
fn oberflaeche(hier: &Path) {
    let mut liste: Vec<(String, PathBuf)> = Vec::new();
    let ui = hier.join("ui");
    println!("cargo:rerun-if-changed={}", ui.display());
    sammeln_alle(&ui, "", &mut liste);
    let spiel = hier.join("../../web-client");
    if spiel.join("assets").is_dir() {
        println!("cargo:rerun-if-changed={}", spiel.join("assets").display());
        for e in std::fs::read_dir(spiel.join("assets")).unwrap().filter_map(Result::ok) {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.ends_with(".webp") && !n.contains(".archive.") { liste.push((format!("bilder/{n}"), e.path())); }
        }
        if spiel.join("three.min.js").exists() { liste.push(("js/three.min.js".into(), spiel.join("three.min.js"))); }
    }
    let marke = hier.join("../../docs/branding/sternenepoche-mark.png");
    let bot_audio = hier.join("../../web-client/audio/bots");
    if bot_audio.is_dir() {
        println!("cargo:rerun-if-changed={}",bot_audio.display());
        for e in std::fs::read_dir(bot_audio).unwrap().filter_map(Result::ok) {
            let n=e.file_name().to_string_lossy().into_owned();
            if n.ends_with(".ogg") || n.ends_with(".json") {liste.push((format!("bot-stimmen/{n}"),e.path()));}
        }
    }
    if marke.exists() { liste.push(("bilder/marke.png".into(), marke)); }
    let figuren = hier.join("assets/figuren");
    println!("cargo:rerun-if-changed={}", figuren.display());
    if figuren.is_dir() {
        for e in std::fs::read_dir(&figuren).unwrap().filter_map(Result::ok) {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.ends_with(".glb") || n.ends_with(".webp") { liste.push((format!("figuren/{n}"), e.path())); }
        }
    }
    // Ratsstimmen: Texte, gesprochene Zeilen (OGG) und je Figur das Voicebox-Paket der Stimme.
    let stimmen = hier.join("assets/stimmen");
    println!("cargo:rerun-if-changed={}", stimmen.display());
    let mut st = Vec::new();
    sammeln_alle(&stimmen, "stimmen", &mut st);
    liste.extend(st.into_iter().filter(|(n, _)| n.ends_with(".json") || n.ends_with(".ogg") || n.ends_with(".voicebox.zip")));
    liste.sort();
    let mut code = String::from("pub const DATEIEN_UI: &[(&str, &[u8])] = &[\n");
    for (name, pfad) in &liste {
        println!("cargo:rerun-if-changed={}", pfad.display());
        code.push_str(&format!("    ({name:?}, include_bytes!({:?})),\n", pfad.display().to_string()));
    }
    code.push_str("];\n");
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("oberflaeche.rs"), code).unwrap();
}

fn sammeln_alle(ordner: &Path, rel: &str, out: &mut Vec<(String, PathBuf)>) {
    let Ok(eintraege) = std::fs::read_dir(ordner) else { return };
    for e in eintraege.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().into_owned();
        let pfad = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        if e.path().is_dir() { sammeln_alle(&e.path(), &pfad, out); } else { out.push((pfad, e.path())); }
    }
}
