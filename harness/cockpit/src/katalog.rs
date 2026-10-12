//! Die vollständige Harness-Konfiguration, wie sie das Cockpit bearbeitet.
//! Bausteine (Datenbank, WASM, lokale API) bestimmen den Bau; alles andere wirkt
//! sofort im laufenden Harness (harness.json + Befehl konfig_neu).
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Öffentliche Spieladresse. Eine einzige Stelle, damit ein neuer, neutraler Hostname
/// nur hier und in der Server-Konfiguration geändert werden muss.
pub const STANDARD_SERVER: &str = "https://sternenepoche.taila4f584.ts.net";
/// Umgezogener Spielserver im selben Tailnet (anderer Gerätename): gespeicherte Adresse auf STANDARD_SERVER setzen.
/// Ohne Liste alter Namen – die gehören nicht in die ausgelieferte exe.
pub fn veralteter_server(gespeichert: &str) -> bool {
    let host = |u: &str| u.trim().trim_end_matches('/').trim_start_matches("https://").to_ascii_lowercase();
    let (alt, neu) = (host(gespeichert), host(STANDARD_SERVER));
    let netz = |h: &str| h.split_once('.').map(|(_, rest)| rest.to_string()).unwrap_or_default();
    gespeichert.trim().starts_with("https://") && alt != neu && neu.ends_with(".ts.net") && netz(&alt) == netz(&neu)
}

pub const ROLLEN: [(&str, &str, &str); 4] = [
    ("stratege", "KI 1 · Stratege", "plant, setzt Ziele, verteilt Aufgaben"),
    ("verwalter", "KI 2 · Verwalter", "Wirtschaft, Bau, Forschung, Markt"),
    ("diplomat", "KI 3 · Diplomat", "Briefe, Verträge, Allianzen, Handel"),
    ("feldherr", "KI 4 · Feldherr", "Flotten, Aufklärung, Verteidigung"),
];
pub const SKILLS: [(&str, &str); 10] = [
    ("navigation", "Navigation"), ("kampf", "Kampf"), ("verteidigung", "Verteidigung"), ("anpassung", "Anpassung"),
    ("ausbau", "Ausbau"), ("aufklaerung", "Aufklärung"), ("diplomatie", "Diplomatie"), ("handel", "Handel"),
    ("bauketten", "Bauketten"), ("auswertung", "Auswertung"),
];
/// Lesende Werkzeuge mit Erklärung (Server und lokal).
pub const WERKZEUGE: [(&str, &str); 11] = [
    ("kosten", "Kosten und Bauzeit von Gebäuden, Forschung, Einheiten"), ("flugzeit", "Flugdauer, Treibstoff, Laderaum"),
    ("kampfsimulator", "Kampf gegen eigenen Spionagebericht simulieren"), ("regel", "Regeltext nachschlagen"),
    ("galaxie", "Bekannte Systeme und Planeten eines Sektors"), ("kolonieplan", "Kolonie planen und prüfen"),
    ("briefarchiv", "Ältere Briefe lesen"), ("gedaechtnis_suche", "Im eigenen Langzeitgedächtnis suchen (lokal)"),
    ("analyse", "Befunde zu allen Planeten: Engpässe, Reichweiten, Trends (lokal)"), ("inventur", "Alle Planeten als kompakte Liste (lokal)"),
    ("wissen", "Eigene Wissensquellen: Datenbank, RAG, MCP (lokal, nur freigegebene)"),
];
pub const ROUTINEN: [(&str, &str, &str); 3] = [
    ("inventur", "Planeten-Inventur", "holt alle Planeten in eine Liste und speichert sie als Zeitreihe im Gedächtnis"),
    ("analyse", "Analyse", "findet Engpässe, volle Lager, leere Schleifen und Trends – mit Vorschlag je Befund"),
    ("auto_ausbau", "Auto-Ausbau", "füllt leere Bauschleifen nach festen Regeln: Strom, Nahrung, Lager, dann deine Reihenfolge"),
];
/// Gebäude für die Reihenfolge des Auto-Ausbaus (Grundversorgung und Wirtschaft).
pub const AUSBAU_GEBAEUDE: [&str; 13] = ["erzmine", "kristallmine", "deuteriumsynthesizer", "farm", "solarkraftwerk", "fusionskraftwerk", "lager", "wohnblock",
    "labor", "akademie", "giesserei", "elektronikwerk", "konsumgueterwerk"];
pub const AUSLOESER: [(&str, &str); 4] = [
    ("angriff", "Neuer feindlicher Anflug erkannt"), ("brief", "Neuer Brief eingegangen"),
    ("bauschleife_leer", "Bauschleife eines Planeten ist leer"), ("aufgabe", "Neue Aufgabe für diese Rolle"),
];
/// Spielaktionen je Rolle (wie im Spielkern) mit Gruppe für die Rechte-Matrix.
pub const AKTIONEN: [(&str, &str, &[&str]); 30] = [
    ("bauen", "Wirtschaft", &["verwalter"]), ("reparieren", "Wirtschaft", &["verwalter"]), ("abreissen", "Wirtschaft", &["verwalter"]),
    ("schleife_leeren", "Wirtschaft", &["verwalter"]), ("forschen", "Wirtschaft", &["verwalter"]), ("steuersatz", "Wirtschaft", &["verwalter"]),
    ("prioritaeten", "Wirtschaft", &["verwalter"]), ("fertigen", "Wirtschaft", &["verwalter", "feldherr"]), ("stufenaufstieg", "Wirtschaft", &["stratege", "verwalter"]),
    ("doktrin", "Führung", &["stratege"]), ("meldung", "Führung", &["stratege", "verwalter", "feldherr", "diplomat"]),
    ("markt_order", "Handel", &["verwalter"]), ("markt_storno", "Handel", &["verwalter"]), ("intern_anbieten", "Handel", &["diplomat", "verwalter"]),
    ("intern_kaufen", "Handel", &["diplomat", "verwalter"]), ("schenken", "Handel", &["diplomat"]),
    ("flotte_senden", "Flotte", &["verwalter", "feldherr"]), ("flotte_zurueckrufen", "Flotte", &["verwalter", "feldherr"]), ("flotte_ausspaehen", "Flotte", &["feldherr"]),
    ("verband_oeffnen", "Flotte", &["feldherr"]), ("raketen_bauen", "Flotte", &["verwalter", "feldherr"]), ("raketen_starten", "Flotte", &["feldherr"]),
    ("brief_senden", "Diplomatie", &["diplomat"]), ("vertrag_anbieten", "Diplomatie", &["diplomat"]), ("vertrag_annehmen", "Diplomatie", &["diplomat"]),
    ("vertrag_kuendigen", "Diplomatie", &["diplomat"]), ("allianz_gruenden", "Diplomatie", &["diplomat"]), ("allianz_beitreten", "Diplomatie", &["diplomat"]),
    ("allianz_verlassen", "Diplomatie", &["diplomat"]), ("diplomatie_anfrage", "Diplomatie", &["diplomat"]),
];
pub const MISSIONEN: [&str; 12] = ["angriff", "pluenderung", "belagerung", "blockade", "kolonisieren", "transport", "stationieren", "saven", "spionage", "system_erkunden", "abbau", "recyceln"];
/// Grundeinstellung: diese Aktionen fragen, bis der Besitzer anders entscheidet.
pub const RISKANT: [&str; 15] = ["schenken", "vertrag_anbieten", "vertrag_kuendigen", "abreissen", "schleife_leeren", "allianz_verlassen", "raketen_starten",
    "flotte_senden:angriff", "flotte_senden:pluenderung", "flotte_senden:belagerung", "flotte_senden:blockade", "flotte_senden:kolonisieren", "intern_anbieten", "markt_order", "allianz_gruenden"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anbieter { Ollama, OpenRouter, Eigen }
impl Anbieter {
    pub fn name(self) -> &'static str { match self { Anbieter::Ollama => "ollama", Anbieter::OpenRouter => "openrouter", Anbieter::Eigen => "openai" } }
    pub fn aus_name(n: &str) -> Option<Self> { match n { "ollama" => Some(Anbieter::Ollama), "openrouter" => Some(Anbieter::OpenRouter), "openai" | "eigen" => Some(Anbieter::Eigen), _ => None } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Datenbank { Sqlite, DuckDb }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wache { Bestaetigen, Automatisch }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Erweiterung { WasmSandbox, Ohne }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Schnittstelle { LokaleApiMcp, Ohne }

fn w_true() -> bool { true }
fn w_temp() -> f32 { 0.2 }
fn w_tokens() -> u32 { 1500 }
fn w_alle_werkzeuge() -> Vec<String> { WERKZEUGE.iter().map(|(w, _)| w.to_string()).collect() }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RollenWahl {
    pub aktiv: bool,
    pub anbieter: Anbieter,
    pub modell: String,
    pub takt_minuten: u32,
    pub skills: Vec<String>,
    #[serde(default)]
    pub seele: String,
    #[serde(default)]
    pub auftrag: String,
    #[serde(default = "w_temp")]
    pub temperatur: f32,
    #[serde(default = "w_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub rechte: BTreeMap<String, String>,
    #[serde(default = "w_alle_werkzeuge")]
    pub werkzeuge: Vec<String>,
    #[serde(default)]
    pub ausloeser: Vec<String>,
    #[serde(default)]
    pub ersatz_modell: String,
    #[serde(default = "ollama_anbieter")]
    pub ersatz_anbieter: Anbieter,
    /// Persönlichkeit als Markdown (wird als seelen/ROLLE.md geschrieben und vom Harness vor jedem Zug gelesen).
    #[serde(default)]
    pub seele_md: String,
    /// Höchstens so viele Modellzüge je Stunde (0 = ohne Grenze).
    #[serde(default)]
    pub max_aufrufe_stunde: u32,
    #[serde(default)]
    pub vorlage: String,
    /// Volk der Ratsfigur (veyari, aurelianer, krath, syntheten); leer = Volk des eigenen Reichs.
    #[serde(default)]
    pub gestalt: String,
}

/// Voreinstellung, solange Ollama nicht gefragt wurde; danach wählt das Cockpit ein installiertes Modell.
pub const STANDARD_MODELL: &str = "qwen3:8b";

fn w_drei() -> usize { 3 }
fn w_eigen_url() -> String { "http://127.0.0.1:1234/v1".into() }
fn w_num_ctx() -> u32 { 16384 }
fn w_post() -> String { "POST".into() }
fn w_query() -> String { "query".into() }
fn w_zwanzig() -> u64 { 20 }
fn w_kb() -> u32 { 256 }
fn w_auth() -> String { "Authorization".into() }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutineWahl {
    pub name: String,
    pub art: String,
    pub aktiv: bool,
    pub alle_minuten: u32,
    #[serde(default)]
    pub reihenfolge: Vec<String>,
    #[serde(default = "w_drei")]
    pub max_auftraege: usize,
    #[serde(default)]
    pub melden_an: String,
}

/// Eine frei definierte Wissensquelle. Schlüssel liegen nie hier, sondern im Tresor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WissenWahl {
    pub name: String,
    /// gedaechtnis, http oder mcp
    pub art: String,
    #[serde(default)]
    pub beschreibung: String,
    #[serde(default)]
    pub url: String,
    #[serde(default = "w_post")]
    pub methode: String,
    /// JSON-Körper als Text, mit {{frage}}, {{rolle}}, {{anzahl}}
    #[serde(default)]
    pub vorlage: String,
    #[serde(default)]
    pub antwort_pfad: String,
    #[serde(default)]
    pub werkzeug: String,
    #[serde(default = "w_query")]
    pub argument: String,
    #[serde(default)]
    pub rollen: Vec<String>,
    #[serde(default = "w_zwanzig")]
    pub timeout_s: u64,
    #[serde(default = "w_kb")]
    pub max_kb: u32,
    #[serde(default)]
    pub mit_schluessel: bool,
    #[serde(default = "w_auth")]
    pub schluessel_kopf: String,
}
impl WissenWahl {
    pub fn neu(art: &str, name: &str) -> Self {
        let (url, vorlage, pfad, beschreibung) = match art {
            "http" => ("https://", "{\"frage\": \"{{frage}}\", \"rolle\": \"{{rolle}}\", \"anzahl\": \"{{anzahl}}\"}", "/treffer", "Eigenes RAG: was es weiß und wie man fragt"),
            "mcp" => ("https://", "", "", "MCP-Server: was er kann"),
            _ => ("", "", "", "Eigenes Gedächtnis (SQL, nur lesend): Inventur-Zeitreihe, Züge, Aufgaben, Erinnerungen"),
        };
        Self { name: name.into(), art: art.into(), beschreibung: beschreibung.into(), url: url.into(), methode: "POST".into(), vorlage: vorlage.into(), antwort_pfad: pfad.into(),
            werkzeug: String::new(), argument: "query".into(), rollen: vec![], timeout_s: 20, max_kb: 256, mit_schluessel: false, schluessel_kopf: "Authorization".into() }
    }
}
fn standard_routinen() -> Vec<RoutineWahl> {
    vec![RoutineWahl { name: "inventur".into(), art: "inventur".into(), aktiv: true, alle_minuten: 30, reihenfolge: vec![], max_auftraege: 3, melden_an: String::new() },
         RoutineWahl { name: "analyse".into(), art: "analyse".into(), aktiv: true, alle_minuten: 30, reihenfolge: vec![], max_auftraege: 3, melden_an: String::new() },
         RoutineWahl { name: "auto_ausbau".into(), art: "auto_ausbau".into(), aktiv: false, alle_minuten: 15, reihenfolge: vec![], max_auftraege: 3, melden_an: "verwalter".into() }]
}
fn standard_wissen() -> Vec<WissenWahl> { vec![WissenWahl::neu("gedaechtnis", "gedaechtnis")] }
fn ollama_anbieter() -> Anbieter { Anbieter::Ollama }

fn w_offen() -> usize { 40 }
fn w_notizen() -> usize { 50 }
fn w_erinnerungen() -> usize { 8 }
fn w_aktionen() -> usize { 4 }
fn w_korrektur() -> u8 { 1 }
fn w_wache() -> u64 { 60 }
fn w_kontext() -> usize { 24_000 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Auswahl {
    #[serde(default = "crate::lokale_agenten::plaetze")]
    pub lokale_agenten: Vec<crate::lokale_agenten::Platz>,
    #[serde(default = "lokale_bots")]
    pub lokale_bots: u16,
    #[serde(default = "lokaler_port")]
    pub lokaler_port: u16,
    #[serde(default)]
    pub lokaler_host_aktiv: bool,
    #[serde(default = "w_true")]
    pub lokaler_sparmodus: bool,
    pub server: String,
    pub rollen: BTreeMap<String, RollenWahl>,
    pub datenbank: Datenbank,
    pub wache: Wache,
    pub erweiterung: Erweiterung,
    pub schnittstelle: Schnittstelle,
    pub ollama_url: String,
    pub openrouter_budget: f64,
    /// Eigener, OpenAI-kompatibler Modellserver (LM Studio, vLLM, llama.cpp, eigenes Fine-Tuning).
    #[serde(default = "w_eigen_url")]
    pub eigen_url: String,
    /// Kontextfenster für Ollama (num_ctx); der Prompt eines Zugs muss hineinpassen.
    #[serde(default = "w_num_ctx")]
    pub ollama_kontext: u32,
    /// Zuletzt gewählter Spielstil (nur zur Anzeige).
    #[serde(default)]
    pub spielstil: String,
    pub arbeitsordner: String,
    #[serde(default)]
    pub gebaut: Option<String>,
    /// Harness selbst aus dem Quelltext übersetzen (Rust + C++-Buildwerkzeuge), statt den vorgebauten zu nehmen –
    /// nur nötig für eigene Änderungen am Harness-Code.
    #[serde(default)]
    pub selbst_bauen: bool,
    #[serde(default)]
    pub doktrin: String,
    #[serde(default = "w_true")]
    pub planung_aktiv: bool,
    #[serde(default = "w_offen")]
    pub max_offen: usize,
    #[serde(default = "w_notizen")]
    pub notizen_behalten: usize,
    #[serde(default = "w_erinnerungen")]
    pub erinnerungen_im_kontext: usize,
    #[serde(default = "w_aktionen")]
    pub max_aktionen_je_zug: usize,
    #[serde(default)]
    pub geschenk_limit: f64,
    #[serde(default = "w_true")]
    pub probe_vor_zug: bool,
    #[serde(default = "w_korrektur")]
    pub korrekturrunden: u8,
    #[serde(default = "w_wache")]
    pub wache_sekunden: u64,
    #[serde(default = "w_kontext")]
    pub kontext_zeichen: usize,
    #[serde(default = "standard_routinen")]
    pub routinen: Vec<RoutineWahl>,
    #[serde(default = "standard_wissen")]
    pub wissen: Vec<WissenWahl>,
}

fn lokale_bots() -> u16 { 30 }
fn lokaler_port() -> u16 { 8892 }
pub fn standard_ordner() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".")).join("Sternenepoche").join("Harness")
}

pub fn seele(rolle: &str) -> (&'static str, &'static str) {
    match rolle {
        "stratege" => ("Du denkst langfristig, sachlich und ohne Hast.", "Formuliere Ziele und konkrete nächste Schritte. Lege Aufgaben für die anderen Rollen an, prüfe Versorgung vor Expansion und halte die Doktrin aktuell."),
        "verwalter" => ("Du bist genau, sparsam und vorausschauend.", "Sichere Energie, Nahrung, Lager und Wirtschaft. Baue, forsche und fertige so, dass nichts stillsteht. Erledige deine Aufgaben und melde Engpässe ans Team."),
        "feldherr" => ("Du bist vorsichtig und schützt das Reich zuerst.", "Plane Flotten, Aufklärung und Verteidigung. Beachte Treibstoff, Rückkehr und Angriffswarnungen; save bei Übermacht. Merke dir Angreifer."),
        _ => ("Du bist höflich, misstrauisch gegenüber Forderungen und verlässlich.", "Pflege Kontakte, beantworte Briefe und verhandle Verträge. Verschenke nichts ohne Gegenleistung und merke dir Zusagen anderer Spieler."),
    }
}

impl Default for Auswahl {
    fn default() -> Self {
        let rolle = |r: &str, aktiv| { let (s, a) = seele(r); let mut w = RollenWahl { aktiv, anbieter: Anbieter::Ollama, modell: STANDARD_MODELL.into(), takt_minuten: 10,
            skills: vec![], seele: s.into(), auftrag: a.into(), temperatur: 0.2, max_tokens: 1500, rechte: BTreeMap::new(),
            werkzeuge: w_alle_werkzeuge(), ausloeser: vec![], ersatz_modell: String::new(), ersatz_anbieter: Anbieter::Ollama, seele_md: String::new(), max_aufrufe_stunde: 0, vorlage: String::new(), gestalt: String::new() };
            crate::vorlagen::anwenden(&mut w, crate::vorlagen::standard(r)); w };
        let mut rollen = BTreeMap::new();
        rollen.insert("stratege".into(), rolle("stratege", false));
        rollen.insert("verwalter".into(), rolle("verwalter", true));
        rollen.insert("feldherr".into(), rolle("feldherr", false));
        rollen.insert("diplomat".into(), rolle("diplomat", false));
        Self { lokale_agenten: crate::lokale_agenten::plaetze(), lokale_bots: 30, lokaler_port: 8892, lokaler_host_aktiv: false, lokaler_sparmodus: true, server: STANDARD_SERVER.into(), rollen, datenbank: Datenbank::Sqlite, wache: Wache::Bestaetigen, erweiterung: Erweiterung::Ohne,
            schnittstelle: Schnittstelle::LokaleApiMcp, ollama_url: "http://127.0.0.1:11434".into(), openrouter_budget: 1.0, eigen_url: w_eigen_url(), ollama_kontext: w_num_ctx(), spielstil: String::new(),
            arbeitsordner: standard_ordner().display().to_string(), gebaut: None, selbst_bauen: false, doktrin: String::new(), planung_aktiv: true, max_offen: 40, notizen_behalten: 50,
            erinnerungen_im_kontext: 8, max_aktionen_je_zug: 4, geschenk_limit: 0.0, probe_vor_zug: true, korrekturrunden: 1, wache_sekunden: 60, kontext_zeichen: 24_000,
            routinen: standard_routinen(), wissen: standard_wissen() }
    }
}

impl Auswahl {
    pub fn ordner(&self) -> PathBuf { PathBuf::from(&self.arbeitsordner) }
    pub fn aktive(&self) -> Vec<String> {
        ROLLEN.iter().map(|(r, _, _)| r.to_string()).filter(|r| self.rollen.get(r).is_some_and(|w| w.aktiv)).collect()
    }
    pub fn nutzt(&self, a: Anbieter) -> bool { self.rollen.values().any(|r| r.aktiv && (r.anbieter == a || (!r.ersatz_modell.is_empty() && r.ersatz_anbieter == a))) }
    pub fn features(&self) -> Vec<&'static str> {
        let mut f = Vec::new();
        if self.nutzt(Anbieter::Ollama) { f.push("ollama"); }
        if self.nutzt(Anbieter::OpenRouter) { f.push("openrouter"); }
        if self.nutzt(Anbieter::Eigen) { f.push("openai"); }
        f.push(match self.datenbank { Datenbank::Sqlite => "sqlite", Datenbank::DuckDb => "duckdb" });
        if self.erweiterung == Erweiterung::WasmSandbox { f.push("wasm"); }
        if self.schnittstelle == Schnittstelle::LokaleApiMcp { f.push("lokale-api"); }
        f
    }
    /// Gleiche Bausteine und gleicher Quellcode ergeben denselben Bau.
    pub fn bau_kennung(&self) -> String { format!("{}|{}", crate::quelle::QUELLE_HASH, self.features().join(",")) }
    pub fn gebaut_aktuell(&self) -> bool { self.gebaut.as_deref() == Some(self.bau_kennung().as_str()) && crate::bau::harness_exe(&self.ordner()).exists() }
    /// Wirksames Recht einer Aktion für eine Rolle (Besitzer-Einstellung oder Grundregel).
    pub fn recht(&self, rolle: &str, kennung: &str) -> String {
        if let Some(r) = self.rollen.get(rolle).and_then(|w| w.rechte.get(kennung)) { return r.clone(); }
        if RISKANT.contains(&kennung) && self.wache == Wache::Bestaetigen { "fragen".into() } else { "erlauben".into() }
    }
    pub fn pruefen(&self) -> Result<(), String> {
        if self.aktive().is_empty() { return Err("Mindestens eine KI-Rolle aktivieren".into()); }
        crate::verbindung::Verbindung::neu(&self.server)?;
        for (r, w) in &self.rollen { if w.aktiv && w.modell.trim().is_empty() { return Err(format!("Modell für {r} fehlt")); } }
        if self.arbeitsordner.trim().is_empty() { return Err("Arbeitsordner fehlt".into()); }
        let kennname = |s: &str| !s.is_empty() && s.len() <= 40 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        for (i, r) in self.routinen.iter().enumerate() {
            if !kennname(&r.name) || self.routinen[..i].iter().any(|x| x.name == r.name) { return Err(format!("Routine „{}“: eindeutiger Name aus a–z, 0–9, - und _", r.name)); }
        }
        for (i, w) in self.wissen.iter().enumerate() {
            if !kennname(&w.name) || self.wissen[..i].iter().any(|x| x.name == w.name) { return Err(format!("Wissensquelle „{}“: eindeutiger Name aus a–z, 0–9, - und _", w.name)); }
            if w.art != "gedaechtnis" && !(w.url.starts_with("https://") && w.url.len() > 8 || w.url.starts_with("http://127.0.0.1") || w.url.starts_with("http://localhost")) { return Err(format!("Wissensquelle {}: Adresse mit https:// (oder http://127.0.0.1 auf diesem Rechner)", w.name)); }
            if w.art == "http" && w.methode == "POST" && !w.vorlage.trim().is_empty() && serde_json::from_str::<Value>(&w.vorlage).is_err() { return Err(format!("Wissensquelle {}: Vorlage ist kein gültiges JSON", w.name)); }
            if w.art == "mcp" && w.werkzeug.trim().is_empty() { return Err(format!("Wissensquelle {}: MCP-Werkzeug auswählen", w.name)); }
        }
        Ok(())
    }
    /// soul.md je Rolle in den Arbeitsordner schreiben (seelen/ROLLE.md).
    pub fn seelen_schreiben(&self) -> Result<(), String> {
        let o = self.ordner().join("seelen");
        std::fs::create_dir_all(&o).map_err(|e| format!("{}: {e}", o.display()))?;
        for (r, w) in &self.rollen {
            if w.seele_md.trim().is_empty() { continue; }
            let p = o.join(format!("{r}.md"));
            if std::fs::read_to_string(&p).ok().as_deref() != Some(w.seele_md.as_str()) { std::fs::write(&p, &w.seele_md).map_err(|e| format!("{}: {e}", p.display()))?; }
        }
        Ok(())
    }
    /// Außerhalb bearbeitete soul.md-Dateien übernehmen (die Datei gewinnt).
    pub fn seelen_laden(&mut self) -> Vec<String> {
        let o = self.ordner().join("seelen");
        let mut geaendert = Vec::new();
        for (r, w) in self.rollen.iter_mut() {
            if let Ok(t) = std::fs::read_to_string(o.join(format!("{r}.md"))) {
                if !t.trim().is_empty() && t != w.seele_md { w.seele_md = t; geaendert.push(r.clone()); }
            }
        }
        geaendert
    }
    pub fn harness_json(&self) -> Value {
        let rollen: serde_json::Map<String, Value> = self.rollen.iter().map(|(r, w)| {
            let mut v = json!({"aktiv":w.aktiv,"anbieter":w.anbieter.name(),"modell":w.modell.trim(),"takt_sekunden":(w.takt_minuten.clamp(1, 1440) * 60),"max_aufrufe_stunde":w.max_aufrufe_stunde,
                "temperatur":w.temperatur,"max_tokens":w.max_tokens,"seele":if w.seele.trim().is_empty() { seele(r).0 } else { w.seele.as_str() },"auftrag":if w.auftrag.trim().is_empty() { seele(r).1 } else { w.auftrag.as_str() },"skills":w.skills,"rechte":w.rechte,"werkzeuge":w.werkzeuge,"ausloeser":w.ausloeser});
            if !w.ersatz_modell.trim().is_empty() { v["ersatz"] = json!({"anbieter":w.ersatz_anbieter.name(),"modell":w.ersatz_modell.trim()}); }
            if !w.seele_md.trim().is_empty() { v["seele_datei"] = json!(format!("seelen/{r}.md")); }
            (r.clone(), v)
        }).collect();
        let duck = if cfg!(windows) { "bin/duckdb.dll" } else if cfg!(target_os = "macos") { "bin/libduckdb.dylib" } else { "bin/libduckdb.so" };
        json!({"version":1,"name":"Sternenepoche-Cockpit","server":self.server.trim_end_matches('/'),"doktrin":self.doktrin,"rollen":rollen,
            "planung":{"aktiv":self.planung_aktiv,"max_offen":self.max_offen},"wache":{"sekunden":self.wache_sekunden},
            "kontext":{"max_zeichen":self.kontext_zeichen},
            "ollama":{"url":self.ollama_url,"kontext":self.ollama_kontext.clamp(2048, 262_144)},"openrouter":{"budget_usd":self.openrouter_budget},
            "openai":{"url":self.eigen_url.trim_end_matches('/')},
            "gedaechtnis":match self.datenbank { Datenbank::Sqlite => json!({"art":"sqlite","pfad":"gedaechtnis.sqlite3","notizen_behalten":self.notizen_behalten,"erinnerungen_im_kontext":self.erinnerungen_im_kontext}),
                Datenbank::DuckDb => json!({"art":"duckdb","pfad":"gedaechtnis.duckdb","duckdb_bibliothek":duck,"notizen_behalten":self.notizen_behalten,"erinnerungen_im_kontext":self.erinnerungen_im_kontext}) },
            "wachposten":{"modus":match self.wache { Wache::Bestaetigen => "bestaetigen", Wache::Automatisch => "automatisch" },"riskant":RISKANT,"max_aktionen_je_zug":self.max_aktionen_je_zug,
                "max_geschenk_credits_je_tag":self.geschenk_limit,"probe_vor_zug":self.probe_vor_zug,"korrekturrunden":self.korrekturrunden},
            "plugins":if self.erweiterung == Erweiterung::WasmSandbox && crate::quelle::BEISPIEL_PLUGIN.is_some() { json!([{"name":"energiewaechter","pfad":"plugins/energiewaechter.wasm","rollen":["verwalter"]}]) } else { json!([]) },
            "lokale_api":{"aktiv":self.schnittstelle == Schnittstelle::LokaleApiMcp,"port":8895},
            "routinen":self.routinen.iter().map(|r| json!({"name":r.name,"art":r.art,"aktiv":r.aktiv,"alle_minuten":r.alle_minuten.clamp(1, 1440),"reihenfolge":r.reihenfolge,"max_auftraege":r.max_auftraege.clamp(1, 10),"melden_an":r.melden_an})).collect::<Vec<_>>(),
            "wissen":self.wissen.iter().map(|w| {
                let mut v = json!({"name":w.name,"art":w.art,"beschreibung":w.beschreibung,"methode":w.methode,"antwort_pfad":w.antwort_pfad.trim(),"rollen":w.rollen,"timeout_s":w.timeout_s,"max_kb":w.max_kb,"mit_schluessel":w.mit_schluessel,"schluessel_kopf":w.schluessel_kopf});
                if w.art != "gedaechtnis" { v["url"] = json!(w.url.trim()); }
                if w.art == "mcp" { v["werkzeug"] = json!(w.werkzeug.trim()); v["argument"] = json!(w.argument.trim()); }
                if let Ok(vorlage) = serde_json::from_str::<Value>(&w.vorlage) { if w.art == "http" { v["vorlage"] = vorlage; } }
                v
            }).collect::<Vec<_>>()})
    }
    pub fn manifest(&self) -> Value {
        json!({"client":"sternenepoche-cockpit","version":env!("CARGO_PKG_VERSION"),
            "modelle":self.aktive().iter().map(|r| { let w = &self.rollen[r]; json!({"rolle":r,"anbieter":w.anbieter.name(),"modell":w.modell.chars().take(100).collect::<String>()}) }).collect::<Vec<_>>(),
            "datenbank":match self.datenbank { Datenbank::Sqlite => "sqlite", Datenbank::DuckDb => "duckdb" },
            "werkzeuge":self.manifest_werkzeuge(),"plugins":if self.erweiterung == Erweiterung::WasmSandbox { vec!["energiewaechter"] } else { vec![] }})
    }
    /// Werkzeugliste fürs Manifest. Der Server erlaubt dort nur kurze Texte (höchstens 24 × 40
    /// Zeichen) und keine weiteren Felder; Routinen und Wissensquellen stehen deshalb als
    /// „routine:…“ und „wissen:…“ darin – nur die Art, nie Adressen.
    fn manifest_werkzeuge(&self) -> Vec<String> {
        let mut v: Vec<String> = ["lage", "abfragen", "wachposten", "probe", "planung", "gedaechtnis"].iter().map(|s| s.to_string()).collect();
        for r in self.routinen.iter().filter(|r| r.aktiv) { v.push(format!("routine:{}", r.art)); }
        for w in &self.wissen { v.push(format!("wissen:{}", w.art)); }
        let mut eindeutig = Vec::new();
        for x in v { if !eindeutig.contains(&x) { eindeutig.push(x); } }
        eindeutig.truncate(24);
        eindeutig
    }
}

/// Bereiche der Autonomie: statt 40 Einzelrechten ein Regler je Bereich.
pub const GRUPPEN: [(&str, &str); 5] = [("Wirtschaft", "Bauen, Forschen, Fertigen, Steuern"), ("Handel", "Markt und Handel im Bündnis"),
    ("Flotte", "Flotten, Raketen, Aufklärung"), ("Diplomatie", "Briefe, Verträge, Allianzen"), ("Führung", "Doktrin, Meldungen, Stufenaufstieg")];
pub const AUTONOMIE: [(&str, &str); 4] = [("autopilot", "Autopilot – handelt selbst"), ("vorsicht", "Vorsicht – Riskantes fragt"), ("erlaubnis", "Nur mit Erlaubnis"), ("gesperrt", "Gesperrt")];

/// Rechte-Kennungen eines Bereichs mit den Rollen, die sie laut Spielregeln nutzen dürfen.
pub fn kennungen(gruppe: &str) -> Vec<(String, &'static [&'static str])> {
    let mut v = Vec::new();
    for (typ, g, rollen) in AKTIONEN {
        if g != gruppe { continue; }
        if typ == "flotte_senden" { for m in MISSIONEN { v.push((format!("flotte_senden:{m}"), rollen)); } } else { v.push((typ.to_string(), rollen)); }
    }
    v
}

impl Auswahl {
    /// Autonomie eines Bereichs für eine Rolle (oder alle Rollen) setzen.
    pub fn autonomie_setzen(&mut self, rolle: Option<&str>, gruppe: &str, stufe: &str) {
        for (k, erlaubt) in kennungen(gruppe) {
            for r in erlaubt.iter().filter(|r| rolle.is_none_or(|x| x == **r)) {
                let Some(w) = self.rollen.get_mut(*r) else { continue };
                match stufe {
                    "autopilot" => { w.rechte.insert(k.clone(), "erlauben".into()); }
                    "erlaubnis" => { w.rechte.insert(k.clone(), "fragen".into()); }
                    "gesperrt" => { w.rechte.insert(k.clone(), "verbieten".into()); }
                    _ => { w.rechte.remove(&k); }
                }
            }
        }
    }
    /// Abgeleitete Stufe: einheitlich gesetzt → diese Stufe, nichts gesetzt → Vorsicht, sonst „gemischt“.
    pub fn autonomie(&self, rolle: &str, gruppe: &str) -> &'static str {
        let Some(w) = self.rollen.get(rolle) else { return "–" };
        let eigene: Vec<Option<&String>> = kennungen(gruppe).into_iter().filter(|(_, r)| r.contains(&rolle)).map(|(k, _)| w.rechte.get(&k)).collect();
        if eigene.is_empty() { return "–"; }
        if eigene.iter().all(Option::is_none) { return "vorsicht"; }
        for (stufe, recht) in [("autopilot", "erlauben"), ("erlaubnis", "fragen"), ("gesperrt", "verbieten")] {
            if eigene.iter().all(|x| x.is_some_and(|v| v == recht)) { return stufe; }
        }
        "gemischt"
    }
}

pub fn einstellungen_pfad() -> PathBuf {
    // Für Tests und mehrere Profile nebeneinander: eigener Ort per Umgebungsvariable.
    if let Some(p) = std::env::var_os("STERNENEPOCHE_COCKPIT_EINSTELLUNGEN") { return PathBuf::from(p); }
    std::env::var_os("APPDATA").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))).unwrap_or_else(|| PathBuf::from(".")).join("Sternenepoche").join("cockpit.json")
}
pub fn laden() -> Auswahl {
    let mut a: Auswahl = std::fs::read_to_string(einstellungen_pfad()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    // Umgezogener Spielserver: alte Adresse auf die neue setzen (der Harness-Schlüssel wird beim nächsten LOS neu geholt).
    if veralteter_server(&a.server) { a.server = STANDARD_SERVER.into(); }
    // Ältere Einstellungen ohne soul.md: aus Seele und Auftrag eine soul.md bilden.
    for (r, w) in a.rollen.iter_mut() {
        if w.seele_md.trim().is_empty() {
            let titel = ROLLEN.iter().find(|(k, _, _)| k == r).map(|(_, t, _)| *t).unwrap_or(r);
            w.seele_md = format!("# {titel}\n\n## Wer du bist\n{}\n\n## Auftrag\n{}\n", w.seele.trim(), w.auftrag.trim());
        }
    }
    a.seelen_laden();
    a
}

// ---- Profile: ganze Harness-Einstellungen speichern, laden und weitergeben (ohne Geheimnisse)
pub fn profil_ordner() -> PathBuf { einstellungen_pfad().parent().map(|p| p.join("profile")).unwrap_or_else(|| PathBuf::from("profile")) }
pub fn profile() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(profil_ordner()).into_iter().flatten().filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".json")).map(str::to_string)).collect();
    v.sort();
    v
}
fn profil_name_ok(n: &str) -> bool { !n.is_empty() && n.chars().count() <= 40 && n.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ') }
pub fn profil_speichern(name: &str, a: &Auswahl) -> Result<PathBuf, String> {
    let name = name.trim();
    if !profil_name_ok(name) { return Err("Profilname: Buchstaben, Ziffern, Leerzeichen, - und _ (höchstens 40)".into()); }
    let o = profil_ordner();
    std::fs::create_dir_all(&o).map_err(|e| e.to_string())?;
    let mut kopie = a.clone();
    kopie.gebaut = None;
    let p = o.join(format!("{name}.json"));
    std::fs::write(&p, serde_json::to_string_pretty(&kopie).unwrap()).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(p)
}
/// Lädt ein Profil; Server, Arbeitsordner und Baustand des aktuellen Cockpits bleiben.
pub fn profil_laden(name: &str, aktuell: &Auswahl) -> Result<Auswahl, String> {
    if !profil_name_ok(name) { return Err("Profilname ungültig".into()); }
    let p = profil_ordner().join(format!("{name}.json"));
    let mut a: Auswahl = serde_json::from_str(&std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?).map_err(|e| format!("Profil {name}: {e}"))?;
    a.server = aktuell.server.clone();
    a.arbeitsordner = aktuell.arbeitsordner.clone();
    a.gebaut = aktuell.gebaut.clone();
    Ok(a)
}
pub fn profil_loeschen(name: &str) { if profil_name_ok(name) { let _ = std::fs::remove_file(profil_ordner().join(format!("{name}.json"))); } }
pub fn speichern(a: &Auswahl) -> Result<(), String> {
    let p = einstellungen_pfad();
    if let Some(o) = p.parent() { std::fs::create_dir_all(o).map_err(|e| e.to_string())?; }
    std::fs::write(&p, serde_json::to_string_pretty(a).unwrap()).map_err(|e| format!("{}: {e}", p.display()))
}

#[cfg(test)]
mod tests_server_umzug {
    use super::*;
    #[test]
    fn umgezogener_server_im_selben_tailnet() {
        let netz = STANDARD_SERVER.trim_start_matches("https://").split_once('.').unwrap().1.to_string();
        assert!(veralteter_server(&format!("https://alter-name.{netz}")));
        assert!(veralteter_server(&format!("https://alter-name.{netz}/")));
        assert!(!veralteter_server(STANDARD_SERVER));
        assert!(!veralteter_server("https://anderer.example.org"));
        assert!(!veralteter_server("http://127.0.0.1:8892"));
        assert!(!veralteter_server("https://fremd.tailanders.ts.net"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn harness_json_enthaelt_alle_harness_bausteine() {
        let mut a = Auswahl::default();
        a.doktrin = "Nie angreifen ohne Spionagebericht.".into();
        a.rollen.get_mut("verwalter").unwrap().rechte.insert("abreissen".into(), "verbieten".into());
        a.rollen.get_mut("verwalter").unwrap().ersatz_modell = "qwen3:4b".into();
        let v = a.harness_json();
        assert_eq!(v["doktrin"], "Nie angreifen ohne Spionagebericht.");
        assert_eq!(v["rollen"]["verwalter"]["rechte"]["abreissen"], "verbieten");
        assert_eq!(v["rollen"]["verwalter"]["ersatz"]["modell"], "qwen3:4b");
        assert_eq!(v["rollen"]["verwalter"]["ausloeser"][0], "bauschleife_leer");
        assert!(v["planung"]["aktiv"] == true && v["gedaechtnis"]["notizen_behalten"] == 50);
        assert_eq!(a.recht("diplomat", "schenken"), "fragen");
        assert_eq!(a.recht("verwalter", "abreissen"), "verbieten");
        // Alte cockpit.json ohne neue Felder bleibt lesbar.
        let alt = r#"{"server":"https://x","rollen":{"verwalter":{"aktiv":true,"anbieter":"Ollama","modell":"m","takt_minuten":5,"skills":[]}},"datenbank":"Sqlite","wache":"Bestaetigen","erweiterung":"Ohne","schnittstelle":"Ohne","ollama_url":"http://127.0.0.1:11434","openrouter_budget":1.0,"arbeitsordner":"x"}"#;
        let a2: Auswahl = serde_json::from_str(alt).unwrap();
        assert!(a2.planung_aktiv && a2.rollen["verwalter"].werkzeuge.len() == WERKZEUGE.len());
        assert!(a2.routinen.len() == 3 && a2.wissen[0].art == "gedaechtnis");
        // soul.md, Budget, Routinen und Wissensquellen landen in harness.json, Schlüssel nie.
        let mut a3 = Auswahl::default();
        a3.rollen.get_mut("verwalter").unwrap().max_aufrufe_stunde = 6;
        let mut rag = WissenWahl::neu("http", "rag");
        rag.url = "https://rag.example/suche".into(); rag.rollen = vec!["diplomat".into()]; rag.mit_schluessel = true;
        a3.wissen.push(rag);
        a3.pruefen().unwrap();
        let v = a3.harness_json();
        assert_eq!(v["rollen"]["verwalter"]["seele_datei"], "seelen/verwalter.md");
        assert_eq!(v["rollen"]["verwalter"]["max_aufrufe_stunde"], 6);
        assert_eq!(v["routinen"][2]["art"], "auto_ausbau");
        assert_eq!(v["wissen"][1]["vorlage"]["frage"], "{{frage}}");
        assert!(v["wissen"][0].get("url").is_none() && !v.to_string().contains("Bearer"));
        // Manifest hält den Vertrag des Servers ein (nur bekannte Felder, Werkzeuge ≤ 24 × 40 Zeichen, keine Adressen).
        let m = a3.manifest();
        let erlaubt = ["client", "version", "modelle", "datenbank", "werkzeuge", "plugins"];
        assert!(m.as_object().unwrap().keys().all(|k| erlaubt.contains(&k.as_str())), "{m}");
        let w = m["werkzeuge"].as_array().unwrap();
        assert!(w.len() <= 24 && w.iter().all(|x| x.as_str().unwrap().chars().count() <= 40));
        assert!(w.contains(&json!("routine:inventur")) && w.contains(&json!("wissen:http")) && !m.to_string().contains("rag.example"));
        a3.wissen[1].vorlage = "{kaputt".into();
        assert!(a3.pruefen().is_err());
    }
}
