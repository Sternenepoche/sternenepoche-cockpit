//! Harness-Konfiguration (harness.json). Enthält nie Geheimnisse: Harness-Schlüssel und
//! OpenRouter-Schlüssel kommen über die Standardeingabe oder Umgebungsvariablen.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const ROLLEN: [&str; 4] = ["stratege", "verwalter", "feldherr", "diplomat"];
/// Lesende Werkzeuge des Servers und eigene, lokale Werkzeuge des Harness.
pub const SERVER_WERKZEUGE: [&str; 7] = ["kosten", "flugzeit", "kampfsimulator", "regel", "galaxie", "kolonieplan", "briefarchiv"];
/// Werkzeuge, die der Harness selbst beantwortet (ohne Serveranfrage).
pub const LOKALE_WERKZEUGE: [&str; 4] = ["gedaechtnis_suche", "analyse", "inventur", "wissen"];
/// Ereignisse, die eine Rolle außerhalb ihres Takts wecken.
pub const AUSLOESER: [&str; 4] = ["angriff", "brief", "bauschleife_leer", "aufgabe"];
pub const RECHTE: [&str; 3] = ["erlauben", "fragen", "verbieten"];
/// Wiederkehrende Aufträge ohne Modell (wie Cron-Jobs).
pub const ROUTINEN: [&str; 3] = ["inventur", "analyse", "auto_ausbau"];
/// Arten von Wissensquellen: eigenes Gedächtnis (nur lesendes SQL), eigener HTTP-/RAG-Dienst, MCP-Server.
pub const WISSENSARTEN: [&str; 3] = ["gedaechtnis", "http", "mcp"];

/// Namen für Routinen, Wissensquellen und Dateien: a-z, 0-9, _ und -.
pub fn kennname(s: &str) -> bool { !s.is_empty() && s.len() <= 40 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-') }

fn wahr() -> bool { true }
fn temperatur() -> f32 { 0.2 }
fn max_tokens() -> u32 { 1500 }
fn takt() -> u64 { 600 }
fn ollama_url() -> String { "http://127.0.0.1:11434".into() }
fn kontext_tokens() -> u32 { 16384 }
fn timeout_modell() -> u64 { 180 }
fn budget() -> f64 { 1.0 }
fn modus() -> String { "bestaetigen".into() }
fn riskant() -> Vec<String> {
    ["schenken", "vertrag_anbieten", "vertrag_kuendigen", "abreissen", "schleife_leeren", "allianz_verlassen", "allianz_ausschliessen", "raketen_starten",
     "flotte_senden:angriff", "flotte_senden:pluenderung", "flotte_senden:belagerung", "flotte_senden:blockade", "flotte_senden:kolonisieren", "intern_anbieten", "markt_order"]
        .iter().map(|s| s.to_string()).collect()
}
fn vier() -> usize { 4 }
fn eins() -> u8 { 1 }
fn max_zeichen() -> usize { 24_000 }
fn acht() -> usize { 8 }
fn fuenf() -> usize { 5 }
fn treibstoff() -> u64 { 50_000_000 }
fn speicher_mb() -> u32 { 16 }
fn port() -> u16 { 8895 }
fn pause() -> u64 { 20 }
fn fuenfzig() -> usize { 50 }
fn vierzig() -> usize { 40 }
fn sechzig() -> u64 { 60 }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Konfig {
    pub version: u32,
    pub name: String,
    pub server: String,
    /// Grundsätze des Besitzers für alle Rollen (wie eine CLAUDE.md für das Reich).
    #[serde(default)]
    pub doktrin: String,
    #[serde(default)]
    pub planung: PlanungKonfig,
    #[serde(default)]
    pub wache: WacheKonfig,
    pub rollen: BTreeMap<String, RollenKonfig>,
    #[serde(default)]
    pub ollama: OllamaKonfig,
    #[serde(default)]
    pub openrouter: OpenRouterKonfig,
    /// Eigener, OpenAI-kompatibler Modellserver (LM Studio, vLLM, llama.cpp, eigenes Fine-Tuning).
    #[serde(default)]
    pub openai: OpenAiKonfig,
    pub gedaechtnis: GedaechtnisKonfig,
    #[serde(default)]
    pub wachposten: WachpostenKonfig,
    #[serde(default)]
    pub kontext: KontextKonfig,
    #[serde(default)]
    pub plugins: Vec<PluginKonfig>,
    #[serde(default)]
    pub lokale_api: LokaleApiKonfig,
    #[serde(default)]
    pub schleife: SchleifenKonfig,
    /// Wiederkehrende Aufträge: Inventur aller Planeten, Analyse, Auto-Ausbau.
    #[serde(default)]
    pub routinen: Vec<RoutineKonfig>,
    /// Frei definierbare Wissensquellen (Datenbank, eigenes RAG, MCP-Server).
    #[serde(default)]
    pub wissen: Vec<WissenKonfig>,
}

fn dreissig() -> u32 { 30 }
fn drei() -> usize { 3 }
fn zwanzig() -> u64 { 20 }
fn max_kb() -> u32 { 256 }
fn post() -> String { "POST".into() }
fn query() -> String { "query".into() }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutineKonfig {
    pub name: String,
    /// inventur, analyse oder auto_ausbau
    pub art: String,
    #[serde(default = "wahr")]
    pub aktiv: bool,
    #[serde(default = "dreissig")]
    pub alle_minuten: u32,
    /// Auto-Ausbau: Gebäude in dieser Reihenfolge ausgleichen (leer = Grundversorgung).
    #[serde(default)]
    pub reihenfolge: Vec<String>,
    /// Auto-Ausbau: höchstens so viele Bauaufträge je Lauf.
    #[serde(default = "drei")]
    pub max_auftraege: usize,
    /// Ergebnis als Teamnachricht an diese Rolle (leer = nur Cockpit und Gedächtnis).
    #[serde(default)]
    pub melden_an: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WissenKonfig {
    pub name: String,
    /// gedaechtnis (nur lesendes SQL auf das eigene Gedächtnis), http (eigener Dienst/RAG) oder mcp
    pub art: String,
    /// Erklärung für Modelle und Cockpit: was die Quelle weiß und wie man fragt.
    #[serde(default)]
    pub beschreibung: String,
    #[serde(default)]
    pub url: String,
    /// http: POST (JSON-Vorlage) oder GET (Frage als Parameter q)
    #[serde(default = "post")]
    pub methode: String,
    /// http: JSON-Körper; Texte "{{frage}}", "{{rolle}}" und "{{anzahl}}" werden ersetzt.
    #[serde(default)]
    pub vorlage: Option<serde_json::Value>,
    /// http: JSON-Zeiger auf die Treffer in der Antwort, z. B. /results
    #[serde(default)]
    pub antwort_pfad: String,
    /// mcp: Name des Werkzeugs und des Arguments, das die Frage bekommt
    #[serde(default)]
    pub werkzeug: String,
    #[serde(default = "query")]
    pub argument: String,
    /// Rollen, die die Quelle als Werkzeug nutzen dürfen (leer = keine; Cockpit und lokale API immer).
    #[serde(default)]
    pub rollen: Vec<String>,
    #[serde(default = "zwanzig")]
    pub timeout_s: u64,
    #[serde(default = "max_kb")]
    pub max_kb: u32,
    /// Der Schlüssel kommt aus den Geheimnissen (nie aus harness.json) und geht in diesen Kopf.
    #[serde(default)]
    pub mit_schluessel: bool,
    #[serde(default = "authorization")]
    pub schluessel_kopf: String,
}
fn authorization() -> String { "Authorization".into() }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollenKonfig {
    #[serde(default = "wahr")]
    pub aktiv: bool,
    pub anbieter: String,
    pub modell: String,
    #[serde(default = "temperatur")]
    pub temperatur: f32,
    #[serde(default = "max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "takt")]
    pub takt_sekunden: u64,
    #[serde(default)]
    pub seele: String,
    #[serde(default)]
    pub auftrag: String,
    #[serde(default)]
    pub skills: Vec<String>,
    /// Rechte je Aktionstyp („bauen“, „flotte_senden:angriff“): erlauben, fragen, verbieten.
    #[serde(default)]
    pub rechte: BTreeMap<String, String>,
    /// Erlaubte lesende Werkzeuge; fehlt die Liste, sind alle erlaubt.
    #[serde(default)]
    pub werkzeuge: Option<Vec<String>>,
    #[serde(default)]
    pub ausloeser: Vec<String>,
    /// Ersatzmodell, falls das Hauptmodell nicht antwortet.
    #[serde(default)]
    pub ersatz: Option<Ersatz>,
    /// Persönlichkeit als Markdown-Datei (soul.md), relativ zum Harness-Ordner, z. B. seelen/verwalter.md.
    /// Wird vor jedem Zug neu gelesen und ersetzt seele und auftrag.
    #[serde(default)]
    pub seele_datei: String,
    /// Höchstens so viele Modellzüge je Stunde (0 = ohne Grenze).
    #[serde(default)]
    pub max_aufrufe_stunde: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ersatz { pub anbieter: String, pub modell: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanungKonfig {
    #[serde(default = "wahr")]
    pub aktiv: bool,
    #[serde(default = "vierzig")]
    pub max_offen: usize,
}
impl Default for PlanungKonfig { fn default() -> Self { Self { aktiv: true, max_offen: 40 } } }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WacheKonfig {
    /// Wie oft die Lage auf Auslöser geprüft wird.
    #[serde(default = "sechzig")]
    pub sekunden: u64,
}
impl Default for WacheKonfig { fn default() -> Self { Self { sekunden: 60 } } }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OllamaKonfig {
    #[serde(default = "ollama_url")]
    pub url: String,
    #[serde(default = "kontext_tokens")]
    pub kontext: u32,
    #[serde(default)]
    pub nur_cpu: bool,
    #[serde(default = "timeout_modell")]
    pub timeout_s: u64,
}
impl Default for OllamaKonfig {
    fn default() -> Self { Self { url: ollama_url(), kontext: kontext_tokens(), nur_cpu: false, timeout_s: timeout_modell() } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRouterKonfig {
    #[serde(default = "budget")]
    pub budget_usd: f64,
    #[serde(default = "wahr")]
    pub datenspeicherung_verbieten: bool,
    #[serde(default = "timeout_modell")]
    pub timeout_s: u64,
}
impl Default for OpenRouterKonfig {
    fn default() -> Self { Self { budget_usd: budget(), datenspeicherung_verbieten: true, timeout_s: timeout_modell() } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenAiKonfig {
    /// Basisadresse bis /v1, z. B. http://127.0.0.1:1234/v1 (LM Studio) oder http://127.0.0.1:8000/v1 (vLLM).
    #[serde(default = "openai_url")]
    pub url: String,
    /// json_schema (Standard), json_object oder text – je nachdem, was der Server versteht.
    #[serde(default = "json_schema")]
    pub antwortformat: String,
    #[serde(default = "timeout_modell")]
    pub timeout_s: u64,
}
fn openai_url() -> String { "http://127.0.0.1:1234/v1".into() }
fn json_schema() -> String { "json_schema".into() }
impl Default for OpenAiKonfig {
    fn default() -> Self { Self { url: openai_url(), antwortformat: json_schema(), timeout_s: timeout_modell() } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GedaechtnisKonfig {
    pub art: String,
    pub pfad: String,
    #[serde(default)]
    pub duckdb_bibliothek: String,
    /// Notizen je Rolle, die erhalten bleiben (ältere werden kompaktiert).
    #[serde(default = "fuenfzig")]
    pub notizen_behalten: usize,
    #[serde(default = "acht")]
    pub erinnerungen_im_kontext: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WachpostenKonfig {
    /// "bestaetigen": riskante Züge warten auf den Spieler. "automatisch": nur Grenzen.
    #[serde(default = "modus")]
    pub modus: String,
    #[serde(default = "riskant")]
    pub riskant: Vec<String>,
    #[serde(default = "vier")]
    pub max_aktionen_je_zug: usize,
    #[serde(default)]
    pub max_geschenk_credits_je_tag: f64,
    #[serde(default = "wahr")]
    pub probe_vor_zug: bool,
    #[serde(default = "eins")]
    pub korrekturrunden: u8,
}
impl Default for WachpostenKonfig {
    fn default() -> Self { Self { modus: modus(), riskant: riskant(), max_aktionen_je_zug: 4, max_geschenk_credits_je_tag: 0.0, probe_vor_zug: true, korrekturrunden: 1 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KontextKonfig {
    #[serde(default = "max_zeichen")]
    pub max_zeichen: usize,
    #[serde(default = "acht")]
    pub notizen: usize,
    #[serde(default = "fuenf")]
    pub letzte_zuege: usize,
}
impl Default for KontextKonfig {
    fn default() -> Self { Self { max_zeichen: max_zeichen(), notizen: 8, letzte_zuege: 5 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginKonfig {
    pub name: String,
    pub pfad: String,
    #[serde(default = "treibstoff")]
    pub treibstoff: u64,
    #[serde(default = "speicher_mb")]
    pub speicher_mb: u32,
    #[serde(default)]
    pub rollen: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LokaleApiKonfig {
    #[serde(default)]
    pub aktiv: bool,
    #[serde(default = "port")]
    pub port: u16,
}
impl Default for LokaleApiKonfig {
    fn default() -> Self { Self { aktiv: false, port: port() } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchleifenKonfig {
    #[serde(default = "pause")]
    pub pause_sekunden: u64,
}
impl Default for SchleifenKonfig {
    fn default() -> Self { Self { pause_sekunden: pause() } }
}

/// Geheimnisse leben nur im Arbeitsspeicher des Harness-Prozesses.
#[derive(Clone, Default, Deserialize)]
pub struct Geheimnisse {
    #[serde(default)]
    pub schluessel: String,
    #[serde(default)]
    pub openrouter: String,
    /// Schlüssel des eigenen Modellservers (leer, wenn der Server keinen verlangt).
    #[serde(default)]
    pub openai: String,
    /// Authorization-Werte je Wissensquelle (z. B. „Bearer …“ für ein eigenes RAG).
    #[serde(default)]
    pub wissen: BTreeMap<String, String>,
}
impl std::fmt::Debug for Geheimnisse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = |x: &str| if x.is_empty() { "fehlt" } else { "***" };
        write!(f, "Geheimnisse {{ schluessel: {}, openrouter: {}, openai: {}, wissen: {:?} }}", s(&self.schluessel), s(&self.openrouter), s(&self.openai),
            self.wissen.keys().map(|k| format!("{k}=***")).collect::<Vec<_>>())
    }
}

/// Relativer Pfad in einem festen Unterordner des Harness, z. B. seelen/verwalter.md.
pub fn sichere_datei(pfad: &str, ordner: &str, endung: &str) -> Result<(), String> {
    let name = pfad.strip_prefix(&format!("{ordner}/")).and_then(|n| n.strip_suffix(endung)).ok_or_else(|| format!("Datei muss {ordner}/NAME{endung} heißen"))?;
    if kennname(name) { Ok(()) } else { Err(format!("Dateiname {pfad}: nur a-z, 0-9, _ und -")) }
}

/// Erlaubt HTTPS überall und HTTP nur auf dem eigenen Rechner.
pub fn sichere_adresse(url: &str, wofuer: &str) -> Result<(), String> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")).ok_or_else(|| format!("{wofuer}: Adresse muss mit https:// beginnen"))?;
    if rest.contains('@') { return Err(format!("{wofuer}: Adresse darf keine Zugangsdaten enthalten")); }
    if url.starts_with("http://") {
        let host = rest.split(['/', ':']).next().unwrap_or("");
        if !["127.0.0.1", "localhost", "[::1]"].contains(&host) && !rest.starts_with("[::1]") {
            return Err(format!("{wofuer}: unverschlüsseltes HTTP nur für den eigenen Rechner (127.0.0.1)"));
        }
    }
    Ok(())
}

impl Konfig {
    pub fn laden(pfad: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
        let k: Konfig = serde_json::from_str(&text).map_err(|e| format!("harness.json: {e}"))?;
        k.pruefen()?;
        Ok(k)
    }
    pub fn speichern(&self, pfad: &Path) -> Result<(), String> {
        self.pruefen()?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(pfad, text).map_err(|e| format!("{}: {e}", pfad.display()))
    }
    pub fn pruefen(&self) -> Result<(), String> {
        if self.version != 1 { return Err("harness.json: version 1 erwartet".into()); }
        sichere_adresse(self.server.trim_end_matches('/'), "Server")?;
        if self.server.trim_end_matches('/').matches('/').count() > 2 { return Err("Server: nur Ursprung angeben, z. B. https://spiel.example".into()); }
        sichere_adresse(&self.ollama.url, "Ollama")?;
        sichere_adresse(self.openai.url.trim_end_matches('/'), "Eigener Modellserver")?;
        if !["json_schema", "json_object", "text"].contains(&self.openai.antwortformat.as_str()) { return Err("openai.antwortformat: json_schema, json_object oder text".into()); }
        if self.rollen.is_empty() || !self.rollen.values().any(|r| r.aktiv) { return Err("Mindestens eine Rolle aktivieren".into()); }
        for (name, r) in &self.rollen {
            if !ROLLEN.contains(&name.as_str()) { return Err(format!("Unbekannte Rolle {name}")); }
            if !r.aktiv { continue; }
            match r.anbieter.as_str() {
                "ollama" if cfg!(feature = "ollama") => {}
                "openrouter" if cfg!(feature = "openrouter") => {}
                "openai" if cfg!(feature = "openai") => {}
                andere => return Err(format!("{name}: Anbieter {andere} ist in diesem Harness nicht eingebaut")),
            }
            if r.modell.trim().is_empty() || r.modell.len() > 120 { return Err(format!("{name}: Modell fehlt")); }
            if !(0.0..=2.0).contains(&r.temperatur) || !(64..=16384).contains(&r.max_tokens) || !(60..=86_400).contains(&r.takt_sekunden) {
                return Err(format!("{name}: Temperatur 0–2, max_tokens 64–16384, Takt 60–86400 Sekunden"));
            }
            if r.seele.chars().count() > 4000 || r.auftrag.chars().count() > 6000 { return Err(format!("{name}: Rollentexte zu lang")); }
            if let Some((k, v)) = r.rechte.iter().find(|(k, v)| !RECHTE.contains(&v.as_str()) || k.len() > 60) { return Err(format!("{name}: Recht {k}={v} ungültig (erlauben, fragen, verbieten)")); }
            if let Some(w) = r.werkzeuge.iter().flatten().find(|w| !SERVER_WERKZEUGE.contains(&w.as_str()) && !LOKALE_WERKZEUGE.contains(&w.as_str())) { return Err(format!("{name}: unbekanntes Werkzeug {w}")); }
            if let Some(a) = r.ausloeser.iter().find(|a| !AUSLOESER.contains(&a.as_str())) { return Err(format!("{name}: unbekannter Auslöser {a}")); }
            if let Some(s) = r.skills.iter().find(|s| s.is_empty() || s.len() > 40 || !s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')) { return Err(format!("{name}: Skillname „{s}“ ungültig (a-z, 0-9, _ und -)")); }
            if let Some(e) = &r.ersatz { if !["ollama", "openrouter", "openai"].contains(&e.anbieter.as_str()) || e.modell.trim().is_empty() { return Err(format!("{name}: Ersatzmodell unvollständig")); } }
            if !r.seele_datei.is_empty() { sichere_datei(&r.seele_datei, "seelen", ".md").map_err(|e| format!("{name}: {e}"))?; }
            if r.max_aufrufe_stunde > 600 { return Err(format!("{name}: höchstens 600 Modellzüge je Stunde")); }
        }
        let mut namen = Vec::new();
        for r in &self.routinen {
            if !kennname(&r.name) || namen.contains(&r.name) { return Err(format!("Routine „{}“: eindeutiger Name aus a-z, 0-9, _ und - nötig", r.name)); }
            namen.push(r.name.clone());
            if !ROUTINEN.contains(&r.art.as_str()) { return Err(format!("Routine {}: Art {} unbekannt (inventur, analyse, auto_ausbau)", r.name, r.art)); }
            if !(1..=1440).contains(&r.alle_minuten) { return Err(format!("Routine {}: alle 1 bis 1440 Minuten", r.name)); }
            if !(1..=10).contains(&r.max_auftraege) || r.reihenfolge.len() > 30 || r.reihenfolge.iter().any(|g| !kennname(g)) { return Err(format!("Routine {}: 1–10 Aufträge, höchstens 30 Gebäudenamen", r.name)); }
            if !r.melden_an.is_empty() && !ROLLEN.contains(&r.melden_an.as_str()) { return Err(format!("Routine {}: melden_an muss eine Rolle sein", r.name)); }
        }
        let mut namen = Vec::new();
        for w in &self.wissen {
            if !kennname(&w.name) || namen.contains(&w.name) { return Err(format!("Wissensquelle „{}“: eindeutiger Name aus a-z, 0-9, _ und - nötig", w.name)); }
            namen.push(w.name.clone());
            if !WISSENSARTEN.contains(&w.art.as_str()) { return Err(format!("Wissensquelle {}: Art {} unbekannt (gedaechtnis, http, mcp)", w.name, w.art)); }
            if w.art != "gedaechtnis" {
                sichere_adresse(&w.url, &format!("Wissensquelle {}", w.name))?;
                if w.url.chars().any(|c| c.is_whitespace() || c.is_control()) { return Err(format!("Wissensquelle {}: Adresse ungültig", w.name)); }
            }
            if !["GET", "POST"].contains(&w.methode.as_str()) { return Err(format!("Wissensquelle {}: Methode GET oder POST", w.name)); }
            if w.art == "mcp" && (w.werkzeug.trim().is_empty() || w.werkzeug.len() > 100 || w.argument.trim().is_empty()) { return Err(format!("Wissensquelle {}: MCP braucht werkzeug und argument", w.name)); }
            if !w.antwort_pfad.is_empty() && !w.antwort_pfad.starts_with('/') { return Err(format!("Wissensquelle {}: antwort_pfad ist ein JSON-Zeiger wie /results", w.name)); }
            if !(1..=120).contains(&w.timeout_s) || !(1..=4096).contains(&w.max_kb) { return Err(format!("Wissensquelle {}: Zeitlimit 1–120 s, Antwort 1–4096 KiB", w.name)); }
            if w.beschreibung.chars().count() > 1000 { return Err(format!("Wissensquelle {}: Beschreibung höchstens 1000 Zeichen", w.name)); }
            if let Some(r) = w.rollen.iter().find(|r| !ROLLEN.contains(&r.as_str())) { return Err(format!("Wissensquelle {}: unbekannte Rolle {r}", w.name)); }
            if w.schluessel_kopf.is_empty() || w.schluessel_kopf.len() > 60 || !w.schluessel_kopf.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') || ["host", "content-length", "cookie", "transfer-encoding"].contains(&w.schluessel_kopf.to_lowercase().as_str()) {
                return Err(format!("Wissensquelle {}: Kopfname für den Schlüssel ungültig", w.name));
            }
        }
        match self.gedaechtnis.art.as_str() {
            "sqlite" if cfg!(feature = "sqlite") => {}
            "duckdb" if cfg!(feature = "duckdb") => {}
            andere => return Err(format!("Gedächtnis {andere} ist in diesem Harness nicht eingebaut")),
        }
        if self.doktrin.chars().count() > 6000 { return Err("Doktrin höchstens 6000 Zeichen".into()); }
        if !(20..=3600).contains(&self.wache.sekunden) { return Err("Wache: 20 bis 3600 Sekunden".into()); }
        if !(5..=1000).contains(&self.gedaechtnis.notizen_behalten) || self.gedaechtnis.erinnerungen_im_kontext > 40 { return Err("Gedächtnis: 5–1000 Notizen, höchstens 40 Erinnerungen im Kontext".into()); }
        if !["bestaetigen", "automatisch"].contains(&self.wachposten.modus.as_str()) { return Err("Wachposten: modus bestaetigen oder automatisch".into()); }
        if !(1..=8).contains(&self.wachposten.max_aktionen_je_zug) { return Err("Wachposten: 1 bis 8 Aktionen je Zug".into()); }
        if !self.plugins.is_empty() && !cfg!(feature = "wasm") { return Err("Plugins brauchen den Baustein WASM-Sandbox".into()); }
        for p in &self.plugins {
            if p.treibstoff == 0 || p.treibstoff > 2_000_000_000 || !(1..=256).contains(&p.speicher_mb) { return Err(format!("Plugin {}: Treibstoff 1–2 Mrd., Speicher 1–256 MB", p.name)); }
        }
        if self.lokale_api.aktiv && !cfg!(feature = "lokale-api") { return Err("Lokale API ist in diesem Harness nicht eingebaut".into()); }
        if self.lokale_api.port < 1024 { return Err("Lokale API: Port ab 1024".into()); }
        Ok(())
    }
    pub fn aktive_rollen(&self) -> Vec<String> {
        ROLLEN.iter().filter(|r| self.rollen.get(**r).is_some_and(|k| k.aktiv)).map(|r| r.to_string()).collect()
    }
    pub fn pfad(&self, basis: &Path, relativ: &str) -> PathBuf {
        let p = Path::new(relativ);
        if p.is_absolute() { p.to_path_buf() } else { basis.join(p) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn beispiel() -> Konfig {
        serde_json::from_value(serde_json::json!({"version":1,"name":"Test","server":"http://127.0.0.1:8892",
            "rollen":{"verwalter":{"anbieter":"ollama","modell":"qwen3:8b"}},"gedaechtnis":{"art":"sqlite","pfad":"g.sqlite3"}})).unwrap()
    }
    #[test]
    fn sichere_adressen_und_grenzen() {
        let k = beispiel();
        k.pruefen().unwrap();
        assert!(sichere_adresse("http://spiel.example", "Server").is_err());
        assert!(sichere_adresse("https://user:pw@spiel.example", "Server").is_err());
        assert!(sichere_adresse("https://spiel.example", "Server").is_ok());
        let mut k2 = k.clone();
        k2.server = "https://spiel.example/api/x".into();
        assert!(k2.pruefen().is_err());
        let mut k3 = k.clone();
        k3.rollen.get_mut("verwalter").unwrap().anbieter = "shell".into();
        assert!(k3.pruefen().is_err());
        assert!(serde_json::from_str::<Konfig>(r#"{"version":1,"name":"x","server":"https://a","rollen":{},"gedaechtnis":{"art":"sqlite","pfad":"x"},"geheim":1}"#).is_err());
        assert!(format!("{:?}", Geheimnisse { schluessel: "sep_x".into(), openrouter: String::new(), openai: "sk-lokal".into(), wissen: [("rag".to_string(), "Bearer geheim".to_string())].into() }).contains("***") && !format!("{:?}", Geheimnisse { schluessel: String::new(), openrouter: String::new(), openai: "sk-lokal".into(), wissen: [("rag".to_string(), "Bearer geheim".to_string())].into() }).contains("geheim")
            && !format!("{:?}", Geheimnisse { openai: "sk-lokal".into(), ..Default::default() }).contains("sk-lokal"));
    }
}
