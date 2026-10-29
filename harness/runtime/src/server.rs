//! Verbindung zum Spielserver: ausschließlich die Harness-API v1 mit Harness-Schlüssel.
//! Der Harness liest nie Browserseiten oder Browser-Datenfeeds (Fair Play).
use serde_json::{json, Value};
use std::time::Duration;

pub struct Server {
    basis: String,
    schluessel: String,
    http: reqwest::blocking::Client,
    pub quote: std::sync::Mutex<String>,
    /// Endpunkte, die dieser Server (noch) nicht kennt: der Harness fällt dort auf das bisherige Verhalten zurück.
    fehlt: std::sync::Mutex<std::collections::BTreeSet<&'static str>>,
    /// Älterer Server: `werkzeug` nimmt noch kein Feld `rolle`.
    werkzeug_ohne_rolle: std::sync::atomic::AtomicBool,
}

/// Endpunkte der Harness-API v1, die nicht jeder Server hat (Vertrag docs/AGENTEN-ZUGANG.md).
pub const ZUSATZ: [&str; 5] = ["agenda", "aktionen", "skills", "pinwand", "akte"];

/// Prozent-Kodierung für Abfrageparameter (Koordinaten, Abschnittsnamen).
pub fn kodieren(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.,".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

impl ApiFehler {
    /// Der Server kennt den Endpunkt nicht (älterer Stand): 404 „Unbekannter Harness-Endpunkt“, 405 oder 501.
    pub fn endpunkt_fehlt(&self) -> bool {
        self.status == 405 || self.status == 501 || (self.status == 404 && (self.text.contains("Unbekannter Harness-Endpunkt") || self.text.contains("Nicht gefunden") || self.text.is_empty()))
    }
}

#[derive(Debug)]
pub struct ApiFehler {
    pub status: u16,
    pub text: String,
    pub warten_s: Option<u64>,
}
impl std::fmt::Display for ApiFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server {}: {}", self.status, self.text)
    }
}
pub type Antwort = Result<Value, ApiFehler>;

impl Server {
    pub fn neu(basis: &str, schluessel: &str) -> Result<Self, String> {
        crate::konfig::sichere_adresse(basis, "Server")?;
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("sternenepoche-harness/", env!("CARGO_PKG_VERSION")))
            .https_only(!basis.starts_with("http://127.0.0.1") && !basis.starts_with("http://localhost"))
            .build().map_err(|e| e.to_string())?;
        Ok(Self { basis: basis.trim_end_matches('/').to_string(), schluessel: schluessel.to_string(), http, quote: Default::default(), fehlt: Default::default(), werkzeug_ohne_rolle: Default::default() })
    }

    /// Kennt der Server diesen Zusatzendpunkt? (Bis zur ersten 404-Antwort wird es angenommen.)
    pub fn hat(&self, endpunkt: &str) -> bool { !self.fehlt.lock().unwrap().contains(endpunkt) }

    /// Antwort eines Zusatzendpunkts: `Ok(None)`, wenn der Server ihn nicht kennt (dann merken).
    fn zusatz(&self, endpunkt: &'static str, r: Antwort) -> Result<Option<Value>, ApiFehler> {
        match r {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.endpunkt_fehlt() => { self.fehlt.lock().unwrap().insert(endpunkt); Ok(None) }
            Err(e) => Err(e),
        }
    }
    fn zusatz_get(&self, endpunkt: &'static str, pfad: &str) -> Result<Option<Value>, ApiFehler> {
        if !self.hat(endpunkt) { return Ok(None); }
        let r = self.get(pfad);
        self.zusatz(endpunkt, r)
    }

    fn senden(&self, anfrage: reqwest::blocking::RequestBuilder) -> Antwort {
        let anfrage = if self.schluessel.is_empty() { anfrage } else { anfrage.bearer_auth(&self.schluessel) };
        let r = anfrage.send().map_err(|e| ApiFehler { status: 0, text: format!("Server nicht erreichbar: {}", e.without_url()), warten_s: None })?;
        let status = r.status().as_u16();
        if let Some(q) = r.headers().get("x-sternenepoche-quote").and_then(|v| v.to_str().ok()) { *self.quote.lock().unwrap() = q.to_string(); }
        let warten_s = r.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
        let text = r.text().map_err(|e| ApiFehler { status, text: e.to_string(), warten_s })?;
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if (200..300).contains(&status) { Ok(v) } else {
            Err(ApiFehler { status, text: v["error"].as_str().map(str::to_string).unwrap_or_else(|| text.chars().take(200).collect()), warten_s })
        }
    }
    fn get(&self, pfad: &str) -> Antwort { self.senden(self.http.get(format!("{}{pfad}", self.basis))) }
    fn post(&self, pfad: &str, body: &Value) -> Antwort { self.senden(self.http.post(format!("{}{pfad}", self.basis)).json(body)) }

    pub fn info(&self) -> Antwort { self.get("/api/harness/v1/info") }
    pub fn ich(&self) -> Antwort { self.get("/api/harness/v1/ich") }
    pub fn schema(&self, rolle: &str) -> Antwort { self.get(&format!("/api/harness/v1/schema?rolle={rolle}")) }
    /// Volle Lage (Routinen, Cockpit-Übersicht, Analyse).
    pub fn lage(&self, rolle: &str) -> Antwort { self.get(&format!("/api/harness/v1/lage?rolle={rolle}")) }
    /// Kompakte Lage für den Modellzug. Ein älterer Server ignoriert `kompakt` und liefert die volle Lage.
    pub fn lage_kompakt(&self, rolle: &str) -> Antwort { self.get(&format!("/api/harness/v1/lage?rolle={rolle}&kompakt=1")) }
    /// Ein Planet vollständig (mit Mond); nur der Abschnitt planeten, damit nicht die ganze übrige Lage mitkommt.
    pub fn lage_planet(&self, rolle: &str, planet: &str) -> Antwort { self.get(&format!("/api/harness/v1/lage?rolle={rolle}&planet={}&teile=planeten", kodieren(planet))) }
    /// Nur diese Abschnitte der Lage.
    pub fn lage_teile(&self, rolle: &str, teile: &[String]) -> Antwort {
        self.get(&format!("/api/harness/v1/lage?rolle={rolle}&kompakt=1&teile={}", kodieren(&teile.join(","))))
    }
    /// Lesende Abfrage, geprüft in der Rolle des Zugs. Ältere Server kennen das Feld `rolle` nicht (400): dann ohne.
    pub fn werkzeug(&self, abfrage: &Value, rolle: Option<&str>) -> Antwort {
        use std::sync::atomic::Ordering;
        if let Some(r) = rolle.filter(|_| !self.werkzeug_ohne_rolle.load(Ordering::SeqCst)) {
            match self.post("/api/harness/v1/werkzeug", &json!({"abfrage":abfrage,"rolle":r})) {
                Err(e) if e.status == 400 && e.text.contains("Unbekanntes Feld") && e.text.contains("rolle") => { self.werkzeug_ohne_rolle.store(true, Ordering::SeqCst); }
                andere => return andere,
            }
        }
        self.post("/api/harness/v1/werkzeug", &json!({"abfrage":abfrage}))
    }
    /// Agenda der Rolle (Punkte mit gültigen Aktionsvorlagen); `None` bei älterem Server.
    pub fn agenda(&self, rolle: &str) -> Result<Option<Value>, ApiFehler> { self.zusatz_get("agenda", &format!("/api/harness/v1/agenda?rolle={rolle}")) }
    /// Aktionskatalog der Rolle; `None` bei älterem Server.
    pub fn aktionen(&self, rolle: &str) -> Result<Option<Value>, ApiFehler> { self.zusatz_get("aktionen", &format!("/api/harness/v1/aktionen?rolle={rolle}")) }
    /// Skill-Paket: Liste; `None` bei älterem Server.
    pub fn skills(&self) -> Result<Option<Value>, ApiFehler> { self.zusatz_get("skills", "/api/harness/v1/skills") }
    /// Ein Skill des Pakets.
    pub fn skill(&self, name: &str) -> Result<Option<Value>, ApiFehler> {
        if !self.hat("skills") { return Ok(None); }
        match self.get(&format!("/api/harness/v1/skills?name={}", kodieren(name))) { Ok(v) => Ok(Some(v)), Err(e) if e.status == 404 => Ok(None), Err(e) => Err(e) }
    }
    /// Offene Karten der Rats-Pinwand; `None` bei älterem Server.
    pub fn pinwand(&self, rolle: &str) -> Result<Option<Value>, ApiFehler> { self.zusatz_get("pinwand", &format!("/api/harness/v1/pinwand?rolle={rolle}&umfang=offen")) }
    /// Offene Karten der Allianz-Pinwand an dieses Reich (oder an alle); `None` ohne Allianz oder bei älterem Server
    /// (der `bereich` nicht kennt und dann die Rats-Pinwand liefern würde – erkennbar am fehlenden `bereich`).
    pub fn pinwand_allianz(&self, rolle: &str) -> Result<Option<Value>, ApiFehler> {
        if !self.hat("pinwand") { return Ok(None); }
        match self.get(&format!("/api/harness/v1/pinwand?rolle={rolle}&umfang=offen&bereich=allianz")) {
            Ok(v) if v["bereich"] == "allianz" => Ok(Some(v)),
            Ok(_) => Ok(None),
            Err(e) if e.status == 409 || e.status == 400 || e.endpunkt_fehlt() => Ok(None),
            Err(e) => Err(e),
        }
    }
    /// Pinwand schreiben (`op` neu, antworten, status, aendern); `rolle` und `freigabe` setzt der Aufrufer.
    pub fn pinwand_schreiben(&self, karte: &Value) -> Result<Option<Value>, ApiFehler> {
        if !self.hat("pinwand") { return Ok(None); }
        let r = self.post("/api/harness/v1/pinwand", karte);
        // Nur ein unbekannter Pfad heißt „keine Pinwand“; 404 für eine unbekannte Karte ist ein echter Fehler.
        match r { Err(e) if e.status == 405 || e.status == 501 || (e.status == 404 && e.text.contains("Unbekannter Harness-Endpunkt")) => { self.fehlt.lock().unwrap().insert("pinwand"); Ok(None) }, r => r.map(Some) }
    }
    /// Eigene Notiz zu einem fremden Reich (leerer Text löscht sie).
    pub fn akte(&self, freigabe: &str, spieler: &Value, text: &str) -> Result<Option<Value>, ApiFehler> {
        if !self.hat("akte") { return Ok(None); }
        let r = self.post("/api/harness/v1/akte", &json!({"freigabe":freigabe,"spieler":spieler,"text":text}));
        match r { Err(e) if e.status == 405 || e.status == 501 || (e.status == 404 && e.text.contains("Unbekannter Harness-Endpunkt")) => { self.fehlt.lock().unwrap().insert("akte"); Ok(None) }, r => r.map(Some) }
    }
    pub fn freigabe_start(&self, rollen: &[String]) -> Antwort { self.post("/api/harness/v1/freigabe", &json!({"aktion":"start","rollen":rollen})) }
    pub fn freigabe_puls(&self, freigabe: &str) -> Antwort { self.post("/api/harness/v1/freigabe", &json!({"aktion":"puls","freigabe":freigabe})) }
    pub fn freigabe_stop(&self, freigabe: &str) -> Antwort { self.post("/api/harness/v1/freigabe", &json!({"aktion":"stop","freigabe":freigabe})) }
    pub fn handeln(&self, zug: &Value) -> Antwort { self.post("/api/harness/v1/handeln", zug) }
    pub fn bericht(&self, bericht: &Value) -> Antwort { self.post("/api/harness/v1/bericht", bericht) }
}

pub fn anfrage_id() -> String {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("Zufallsquelle");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Prüfung gegen einen kleinen Ersatzserver auf 127.0.0.1: neue Endpunkte, Rückfall bei älterem Server.
#[cfg(all(test, feature = "lokale-api"))]
mod tests {
    use super::*;
    /// Ersatzserver: `neu = true` kennt die Harness-API mit Zusätzen, sonst nur den alten Stand.
    fn ersatz(neu: bool) -> (String, std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>) {
        let s = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}", s.server_addr().to_ip().unwrap().port());
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let l2 = log.clone();
        std::thread::spawn(move || for mut req in s.incoming_requests() {
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            let url = req.url().to_string();
            l2.lock().unwrap().push((url.clone(), body.clone()));
            let pfad = url.split('?').next().unwrap_or("").to_string();
            let (code, antwort) = match pfad.as_str() {
                "/api/harness/v1/werkzeug" if !neu && body.contains("\"rolle\"") => (400, json!({"error":"Unbekanntes Feld „rolle“; erlaubt: abfrage"})),
                "/api/harness/v1/werkzeug" => (200, json!({"ergebnis":{"ok":true},"fremdtexte":[]})),
                "/api/harness/v1/lage" => (200, if neu && url.contains("kompakt=1") { json!({"lage":{"planeten":["1:2:3 kurz"]},"teile_verfuegbar":{"markt":1200},"pinwand":[]}) } else { json!({"lage":{"planeten":[{"koord":"1:2:3"}]}}) }),
                "/api/harness/v1/agenda" if neu => (200, json!({"punkte":[{"titel":"Bauschleife leer","vorlagen":[{"typ":"bauen"}]}]})),
                "/api/harness/v1/pinwand" if neu && body.contains("\"karte\":999") => (404, json!({"error":"Karte nicht gefunden"})),
                "/api/harness/v1/pinwand" if neu => (200, json!({"ok":true,"karte":{"id":7}})),
                _ => (404, json!({"error":"Unbekannter Harness-Endpunkt; siehe GET /api/harness/v1/info"})),
            };
            let _ = req.respond(tiny_http::Response::from_string(antwort.to_string()).with_status_code(code));
        });
        (url, log)
    }
    #[test]
    fn neue_endpunkte_und_rueckfall() {
        let (url, log) = ersatz(true);
        let s = Server::neu(&url, "sep_test").unwrap();
        assert!(s.agenda("verwalter").unwrap().is_some());
        assert!(s.lage_kompakt("verwalter").unwrap()["teile_verfuegbar"].is_object());
        s.werkzeug(&json!({"typ":"kosten"}), Some("verwalter")).unwrap();
        assert!(log.lock().unwrap().last().unwrap().1.contains("\"rolle\":\"verwalter\""), "Werkzeug mit Rolle");
        assert_eq!(s.pinwand_schreiben(&json!({"op":"neu"})).unwrap().unwrap()["karte"]["id"], 7);
        assert_eq!(s.pinwand_schreiben(&json!({"op":"status","karte":999})).unwrap_err().status, 404, "unbekannte Karte ist ein Fehler");
        assert!(s.hat("pinwand"), "Pinwand bleibt verfügbar");
        assert!(s.aktionen("verwalter").unwrap().is_none() && !s.hat("aktionen"), "fehlender Endpunkt wird gemerkt");
        let n = log.lock().unwrap().len();
        assert!(s.aktionen("verwalter").unwrap().is_none());
        assert_eq!(log.lock().unwrap().len(), n, "kein zweiter Versuch");
        assert!(s.lage_planet("verwalter", "1:2:3").is_ok());
        assert!(log.lock().unwrap().last().unwrap().0.contains("planet=1%3A2%3A3"));

        // Älterer Server: Werkzeug ohne Rolle, Agenda und Pinwand fehlen, Lage ist voll.
        let (url, log) = ersatz(false);
        let alt = Server::neu(&url, "sep_test").unwrap();
        assert_eq!(alt.werkzeug(&json!({"typ":"kosten"}), Some("feldherr")).unwrap()["ergebnis"]["ok"], true);
        assert!(!log.lock().unwrap().last().unwrap().1.contains("rolle"));
        alt.werkzeug(&json!({"typ":"kosten"}), Some("feldherr")).unwrap();
        assert_eq!(log.lock().unwrap().iter().filter(|(_, b)| b.contains("rolle")).count(), 1, "Rolle nur einmal versucht");
        assert!(alt.agenda("feldherr").unwrap().is_none() && alt.pinwand("feldherr").unwrap().is_none());
        assert!(alt.pinwand_schreiben(&json!({"op":"neu"})).unwrap().is_none());
        assert!(alt.lage_kompakt("feldherr").unwrap()["teile_verfuegbar"].is_null());
    }
}
