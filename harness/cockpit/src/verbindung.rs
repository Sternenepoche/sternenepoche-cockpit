//! Gespräche des Cockpits mit dem Spielserver: Anmeldung, Platz, Regeln, Harness-Schlüssel,
//! Einmalcode für den Browser und Fair-Play-Bestätigung. Das Passwort wird nur für die
//! Anmeldung gesendet und nie gespeichert; die Sitzung lebt nur im Arbeitsspeicher.
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct Sitzung { pub token: String, pub name: String, pub world_id: String }

pub struct Verbindung { pub server: String, http: reqwest::blocking::Client }

impl Verbindung {
    pub fn neu(server: &str) -> Result<Self, String> {
        let s = server.trim_end_matches('/');
        let parsed = reqwest::Url::parse(s).map_err(|_| "Serveradresse ungültig")?;
        if !parsed.username().is_empty() || parsed.password().is_some() || parsed.query().is_some() || parsed.fragment().is_some() { return Err("Serveradresse ohne Zugangsdaten angeben".into()); }
        let lokal = parsed.host_str().is_some_and(|h| h == "localhost" || h.parse::<std::net::IpAddr>().is_ok_and(sternenepoche_server::local::private_ip));
        if !s.starts_with("https://") && !lokal { return Err("Nur HTTPS-Server (oder der eigene Rechner)".into()); }
        let http = reqwest::blocking::Client::builder().timeout(Duration::from_secs(25)).redirect(reqwest::redirect::Policy::none()).https_only(!lokal)
            .user_agent(concat!("sternenepoche-cockpit/", env!("CARGO_PKG_VERSION"))).build().map_err(|e| e.to_string())?;
        Ok(Self { server: s.to_string(), http })
    }
    fn senden(&self, r: reqwest::blocking::RequestBuilder, token: Option<&str>) -> Result<Value, String> {
        let r = match token { Some(t) => r.bearer_auth(t), None => r };
        let antwort = r.send().map_err(|e| format!("Server nicht erreichbar: {}", e.without_url()))?;
        let status = antwort.status().as_u16();
        let v: Value = antwort.json().unwrap_or(Value::Null);
        if (200..300).contains(&status) { Ok(v) } else { Err(v["error"].as_str().map(str::to_string).unwrap_or_else(|| format!("Server antwortet {status}"))) }
    }
    fn get(&self, pfad: &str, token: Option<&str>) -> Result<Value, String> { self.senden(self.http.get(format!("{}{pfad}", self.server)), token) }
    fn post(&self, pfad: &str, body: Value, token: Option<&str>) -> Result<Value, String> { self.senden(self.http.post(format!("{}{pfad}", self.server)).json(&body), token) }

    pub fn info(&self) -> Result<Value, String> { self.get("/api/harness/v1/info", None) }
    /// Öffentlicher Regelkatalog (Schiffe, Gebäude, Forschung, Völker) für das Wiki im Monitor.
    pub fn regeln(&self) -> Result<Value, String> { self.get("/api/rules", None) }
    /// Name + Masterpasswort. Bei fremden Hosts bindet das Gerätegeheimnis dieses Rechners den
    /// Namen an dieses Cockpit; ein zweiter Spieler mit demselben Namen bekommt dann eine
    /// klare Ablehnung statt fremden Reichs.
    pub fn lokal_anmelden(&self, name: &str, password: &str, volk: &str) -> Result<Sitzung, String> {
        let geraet = crate::tresor::geraet()?;
        let v = self.post("/api/local/join", json!({"name":name.trim(),"password":password,"volk":volk,"geraet":geraet}), None)?;
        Ok(Sitzung { token: v["token"].as_str().ok_or("Keine lokale Sitzung")?.into(), name: v["name"].as_str().unwrap_or(name).into(), world_id: v["world_id"].as_str().unwrap_or("").into() })
    }
    pub fn lokal_info(&self) -> Result<Value, String> { self.get("/api/local/info", None) }
    /// Teilnehmer der eigenen Runde (nur Host): Name, Platz, Spielweise, Bindung.
    pub fn lokal_spieler(&self, s: &Sitzung) -> Result<Value, String> { self.get("/api/local/spieler", Some(&s.token)) }
    pub fn lokal_steuern(&self, s: &Sitzung, body: Value) -> Result<Value, String> { self.post("/api/local/control", body, Some(&s.token)) }
    pub fn anmelden(&self, name: &str, passwort: &str, otp: &str) -> Result<Sitzung, String> {
        let v = self.post("/api/login", json!({"name":name,"password":passwort,"otp":otp}), None)?;
        Ok(Sitzung { token: v["token"].as_str().ok_or("Anmeldung ohne Sitzung")?.to_string(), name: v["name"].as_str().unwrap_or(name).to_string(), world_id: v["world_id"].as_str().unwrap_or("").to_string() })
    }
    pub fn ich(&self, s: &Sitzung) -> Result<Value, String> { self.get("/api/me", Some(&s.token)) }
    pub fn allianz_verzeichnis(&self, s: &Sitzung) -> Result<Value, String> {
        self.get("/api/view",Some(&s.token)).map(|v|v["kommunikation"]["allianzen"].clone())
    }
    pub fn platz(&self, s: &Sitzung, modus: &str, volk: &str) -> Result<Value, String> {
        let welt = self.get("/api/lobby", None)?;
        self.post("/api/claim", json!({"mode":modus,"volk":volk,"world_id":welt["world_id"]}), Some(&s.token))
    }
    /// Lokale Runden: Das Cockpit hält je Welt genau einen eigenen Schlüssel. Ältere aktive
    /// Cockpit-Schlüssel dieses Kontos werden vorher widerrufen; sonst sperrt die Grenze von
    /// drei aktiven Schlüsseln nach wenigen Neustarts jeden weiteren Start.
    pub fn schluessel_ersetzen(&self, s: &Sitzung, rollen: &[String], manifest: Value, regeln: &str) -> Result<String, String> {
        let liste = self.get("/api/harness/v1/schluessel", Some(&s.token))?;
        for k in liste["schluessel"].as_array().into_iter().flatten() {
            if k["label"] == "Sternenepoche-Cockpit" && k["widerrufen"] == false && k["aktuelle_welt"] == true {
                self.post("/api/harness/v1/schluessel/widerrufen", json!({"id":k["id"]}), Some(&s.token))?;
            }
        }
        self.schluessel(s, rollen, manifest, regeln)
    }
    pub fn schluessel(&self, s: &Sitzung, rollen: &[String], manifest: Value, regeln: &str) -> Result<String, String> {
        let v = self.post("/api/harness/v1/schluessel", json!({"label":"Sternenepoche-Cockpit","rollen":rollen,"gueltig_tage":30,"manifest":manifest,"regeln_akzeptiert":regeln}), Some(&s.token))?;
        v["schluessel"].as_str().map(str::to_string).ok_or_else(|| "Server lieferte keinen Schlüssel".into())
    }
    pub fn uebergabe(&self, s: &Sitzung) -> Result<String, String> {
        self.post("/api/sitzung/uebergabe", json!({}), Some(&s.token))?["code"].as_str().map(str::to_string).ok_or_else(|| "Kein Übergabecode".into())
    }
    pub fn fairplay_bestaetigen(&self, s: &Sitzung) -> Result<Value, String> { self.post("/api/fairplay/bestaetigen", json!({}), Some(&s.token)) }
    pub fn harness_ich(&self, schluessel: &str) -> Result<Value, String> { self.get("/api/harness/v1/ich", Some(schluessel)) }
    /// Eintrittsadresse für den Browser: mit Einmalcode, falls eine Sitzung besteht.
    pub fn browser_adresse(&self, s: Option<&Sitzung>) -> String {
        match s.and_then(|s| self.uebergabe(s).ok()) { Some(code) => format!("{}/#uebergabe={code}", self.server), None => format!("{}/", self.server) }
    }
}
