//! Verbindung zum Spielserver: ausschließlich die Harness-API v1 mit Harness-Schlüssel.
//! Der Harness liest nie Browserseiten oder Browser-Datenfeeds (Fair Play).
use serde_json::{json, Value};
use std::time::Duration;

pub struct Server {
    basis: String,
    schluessel: String,
    http: reqwest::blocking::Client,
    pub quote: std::sync::Mutex<String>,
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
        Ok(Self { basis: basis.trim_end_matches('/').to_string(), schluessel: schluessel.to_string(), http, quote: Default::default() })
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
    pub fn lage(&self, rolle: &str) -> Antwort { self.get(&format!("/api/harness/v1/lage?rolle={rolle}")) }
    pub fn werkzeug(&self, abfrage: &Value) -> Antwort { self.post("/api/harness/v1/werkzeug", &json!({"abfrage":abfrage})) }
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
