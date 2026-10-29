// Bettet den Beschreibungskatalog (content/beschreibungen.json, docs/DESIGNREGELN.md §6) ein.
// Fehlt die Datei noch, wird ein leerer Katalog eingebettet; die Oberfläche blendet die Felder dann aus.
// Agentenzugang (26.10.2026): Aktionshilfen, Missionshilfe und Skill-Paket aus content/agenten/ (agenten.rs).
// Fehlen sie (Quellexporte ohne content/), bleibt der Katalog leer statt dass der Bau scheitert.
use std::{env, fs, path::{Path, PathBuf}};

fn rust_text(p: &Path) -> String {
    format!("include_str!({:?})", p.display().to_string())
}

fn agenten(wurzel: &Path, aus: &Path) {
    let agenten = wurzel.join("content/agenten");
    println!("cargo:rerun-if-changed={}", agenten.display());
    let mut hilfe: Vec<PathBuf> = fs::read_dir(agenten.join("aktionen")).map(|d| d.filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json")).collect()).unwrap_or_default();
    hilfe.sort();
    let mut skills: Vec<(String, PathBuf)> = fs::read_dir(agenten.join("skills")).map(|d| d.filter_map(|e| e.ok())
        .filter_map(|e| { let p = e.path().join("SKILL.md"); p.is_file().then(|| (e.file_name().to_string_lossy().into_owned(), p)) }).collect()).unwrap_or_default();
    skills.sort();
    let missionen = agenten.join("missionen.json");
    println!("cargo:rerun-if-changed={}", agenten.join("aktionen").display());
    println!("cargo:rerun-if-changed={}", agenten.join("skills").display());
    for p in hilfe.iter().chain(skills.iter().map(|(_, p)| p)).chain(std::iter::once(&missionen)) {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let mut code = String::from("pub const AKTIONSHILFE_JSON: &[&str] = &[");
    for p in &hilfe { code.push_str(&rust_text(p)); code.push(','); }
    code.push_str("];\npub const MISSIONSHILFE_JSON: &str = ");
    code.push_str(&if missionen.is_file() { rust_text(&missionen) } else { "\"{}\"".into() });
    code.push_str(";\npub const SKILLS: &[(&str, &str)] = &[");
    for (n, p) in &skills { code.push_str(&format!("({n:?}, {}),", rust_text(p))); }
    code.push_str("];\n");
    fs::write(aus.join("agenten_inhalt.rs"), code).unwrap();
}

fn main() {
    let wurzel = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let quelle = wurzel.join("content/beschreibungen.json");
    println!("cargo:rerun-if-changed={}", quelle.display());
    println!("cargo:rerun-if-changed=build.rs");
    let mut inhalt = fs::read(&quelle).unwrap_or_default();
    if inhalt.starts_with(&[0xEF, 0xBB, 0xBF]) {
        inhalt.drain(..3);
    }
    let gueltig = inhalt.first().is_some_and(|b| *b == b'{') && inhalt.len() < 8 * 1024 * 1024;
    if !gueltig {
        inhalt = br#"{"eintraege":{}}"#.to_vec();
    }
    let aus = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(aus.join("beschreibungen.json"), inhalt).unwrap();
    agenten(&wurzel, &aus);
}
