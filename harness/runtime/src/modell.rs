//! Modellanbieter je Rolle: Ollama (lokal), OpenRouter (Cloud) und ein eigener,
//! OpenAI-kompatibler Server (LM Studio, vLLM, llama.cpp – auch selbst trainierte Modelle).
//! Modellantworten sind Vorschläge; sie werden lokal geprüft, vom Wachposten gefiltert,
//! auf dem Server geprobt und erst dann gespielt.
use crate::konfig::{Geheimnisse, Konfig, RollenKonfig};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct ModellAntwort {
    pub text: String,
    pub tokens: u64,
    pub kosten_usd: f64,
    pub latenz_ms: u64,
    pub anbieter: String,
    pub modell: String,
}

pub struct Modelle {
    http: reqwest::blocking::Client,
    ollama_url: String,
    ollama_kontext: u32,
    ollama_nur_cpu: bool,
    ollama_timeout: Duration,
    openrouter_schluessel: String,
    openrouter_daten_verbieten: bool,
    openrouter_timeout: Duration,
    openai_url: String,
    openai_schluessel: String,
    openai_format: String,
    openai_timeout: Duration,
    pub budget_usd: f64,
    pub verbraucht_usd: std::sync::Mutex<f64>,
}

impl Modelle {
    pub fn neu(k: &Konfig, g: &Geheimnisse) -> Result<Self, String> {
        let http = reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none()).build().map_err(|e| e.to_string())?;
        Ok(Self { http, ollama_url: k.ollama.url.trim_end_matches('/').into(), ollama_kontext: k.ollama.kontext, ollama_nur_cpu: k.ollama.nur_cpu,
            ollama_timeout: Duration::from_secs(k.ollama.timeout_s), openrouter_schluessel: g.openrouter.clone(),
            openai_url: k.openai.url.trim_end_matches('/').into(), openai_schluessel: g.openai.clone(), openai_format: k.openai.antwortformat.clone(), openai_timeout: Duration::from_secs(k.openai.timeout_s),
            openrouter_daten_verbieten: k.openrouter.datenspeicherung_verbieten, openrouter_timeout: Duration::from_secs(k.openrouter.timeout_s),
            budget_usd: k.openrouter.budget_usd, verbraucht_usd: Default::default() })
    }

    pub fn fragen(&self, r: &RollenKonfig, system: &str, nutzer: &str, schema: &Value) -> Result<ModellAntwort, String> {
        let start = Instant::now();
        let mut a = match r.anbieter.as_str() {
            #[cfg(feature = "ollama")]
            "ollama" => self.ollama(r, system, nutzer, schema)?,
            #[cfg(feature = "openrouter")]
            "openrouter" => self.openrouter(r, system, nutzer, schema)?,
            #[cfg(feature = "openai")]
            "openai" => self.openai(r, system, nutzer, schema)?,
            andere => return Err(format!("Anbieter {andere} nicht eingebaut")),
        };
        a.latenz_ms = start.elapsed().as_millis() as u64;
        a.anbieter = r.anbieter.clone();
        a.modell = r.modell.clone();
        if a.text.len() > 200_000 { return Err("Modellantwort zu groß".into()); }
        Ok(a)
    }

    #[cfg(feature = "ollama")]
    fn ollama(&self, r: &RollenKonfig, system: &str, nutzer: &str, schema: &Value) -> Result<ModellAntwort, String> {
        let mut optionen = json!({"temperature":r.temperatur,"num_predict":r.max_tokens,"num_ctx":self.ollama_kontext});
        if self.ollama_nur_cpu { optionen["num_gpu"] = json!(0); }
        let body = json!({"model":r.modell,"stream":false,"think":false,"format":schema,"options":optionen,
            "messages":[{"role":"system","content":system},{"role":"user","content":nutzer}]});
        let v: Value = self.http.post(format!("{}/api/chat", self.ollama_url)).timeout(self.ollama_timeout).json(&body).send()
            .map_err(|e| format!("Ollama nicht erreichbar: {}", e.without_url()))?
            .error_for_status().map_err(|e| format!("Ollama meldet {}", e.status().map(|s| s.as_u16()).unwrap_or(0)))?
            .json().map_err(|e| format!("Ollama-Antwort unlesbar: {e}"))?;
        Ok(ModellAntwort { text: v["message"]["content"].as_str().unwrap_or("").to_string(),
            tokens: v["prompt_eval_count"].as_u64().unwrap_or(0) + v["eval_count"].as_u64().unwrap_or(0), ..Default::default() })
    }

    #[cfg(feature = "openrouter")]
    fn openrouter(&self, r: &RollenKonfig, system: &str, nutzer: &str, _schema: &Value) -> Result<ModellAntwort, String> {
        if self.openrouter_schluessel.is_empty() { return Err("OpenRouter-Schlüssel fehlt".into()); }
        let verbraucht = *self.verbraucht_usd.lock().unwrap();
        if verbraucht >= self.budget_usd { return Err(format!("OpenRouter-Budget von {:.2} USD erreicht", self.budget_usd)); }
        let mut body = json!({"model":r.modell,"max_tokens":r.max_tokens,"temperature":r.temperatur,"response_format":{"type":"json_object"},"usage":{"include":true},
            "messages":[{"role":"system","content":system},{"role":"user","content":nutzer}]});
        if self.openrouter_daten_verbieten { body["provider"] = json!({"data_collection":"deny"}); }
        let v: Value = self.http.post("https://openrouter.ai/api/v1/chat/completions").timeout(self.openrouter_timeout)
            .bearer_auth(&self.openrouter_schluessel).header("X-Title", "Sternenepoche Harness").json(&body).send()
            .map_err(|e| format!("OpenRouter nicht erreichbar: {}", e.without_url()))?
            .error_for_status().map_err(|e| format!("OpenRouter meldet {}", e.status().map(|s| s.as_u16()).unwrap_or(0)))?
            .json().map_err(|e| format!("OpenRouter-Antwort unlesbar: {e}"))?;
        let kosten = v["usage"]["cost"].as_f64().filter(|c| c.is_finite() && *c >= 0.0).unwrap_or(0.0);
        *self.verbraucht_usd.lock().unwrap() += kosten;
        Ok(ModellAntwort { text: v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string(),
            tokens: v["usage"]["total_tokens"].as_u64().unwrap_or(0), kosten_usd: kosten, ..Default::default() })
    }

    /// Eigener Server mit OpenAI-Chat-API. Versteht er das Antwortschema nicht (400/422),
    /// fragt der Harness einmal ohne Formatvorgabe; die Antwort wird ohnehin lokal geprüft.
    #[cfg(feature = "openai")]
    fn openai(&self, r: &RollenKonfig, system: &str, nutzer: &str, schema: &Value) -> Result<ModellAntwort, String> {
        let mut body = json!({"model":r.modell,"max_tokens":r.max_tokens,"temperature":r.temperatur,"stream":false,
            "messages":[{"role":"system","content":system},{"role":"user","content":nutzer}]});
        match self.openai_format.as_str() {
            "json_schema" => body["response_format"] = json!({"type":"json_schema","json_schema":{"name":"zug","schema":schema,"strict":false}}),
            "json_object" => body["response_format"] = json!({"type":"json_object"}),
            _ => {}
        }
        let senden = |b: &Value| {
            let mut q = self.http.post(format!("{}/chat/completions", self.openai_url)).timeout(self.openai_timeout).json(b);
            if !self.openai_schluessel.is_empty() { q = q.bearer_auth(&self.openai_schluessel); }
            q.send().map_err(|e| format!("Eigener Modellserver nicht erreichbar: {}", e.without_url()))
        };
        let mut antwort = senden(&body)?;
        if matches!(antwort.status().as_u16(), 400 | 422) && body.get("response_format").is_some() {
            body.as_object_mut().unwrap().remove("response_format");
            antwort = senden(&body)?;
        }
        let v: Value = antwort.error_for_status().map_err(|e| format!("Eigener Modellserver meldet {}", e.status().map(|s| s.as_u16()).unwrap_or(0)))?
            .json().map_err(|e| format!("Antwort des Modellservers unlesbar: {e}"))?;
        let nachricht = &v["choices"][0]["message"];
        // Denkende Modelle liefern die Antwort manchmal nur im reasoning-Feld.
        let text = nachricht["content"].as_str().filter(|t| !t.trim().is_empty()).or(nachricht["reasoning_content"].as_str()).unwrap_or("").to_string();
        Ok(ModellAntwort { text, tokens: v["usage"]["total_tokens"].as_u64().unwrap_or(0), ..Default::default() })
    }

    /// Modelle des eigenen Servers (GET /models).
    pub fn openai_modelle(&self) -> Result<Vec<String>, String> {
        let mut q = self.http.get(format!("{}/models", self.openai_url)).timeout(Duration::from_secs(8));
        if !self.openai_schluessel.is_empty() { q = q.bearer_auth(&self.openai_schluessel); }
        let v: Value = q.send().map_err(|_| format!("Eigener Modellserver unter {} nicht erreichbar", self.openai_url))?
            .error_for_status().map_err(|e| format!("Modellserver meldet {}", e.status().map(|s| s.as_u16()).unwrap_or(0)))?.json().map_err(|e| e.to_string())?;
        Ok(v["data"].as_array().into_iter().flatten().filter_map(|m| m["id"].as_str().map(str::to_string)).collect())
    }

    /// Liste der lokal vorhandenen Ollama-Modelle (für Selbsttest und Builder).
    pub fn ollama_modelle(&self) -> Result<Vec<String>, String> {
        let v: Value = self.http.get(format!("{}/api/tags", self.ollama_url)).timeout(Duration::from_secs(8)).send()
            .map_err(|_| format!("Ollama unter {} nicht erreichbar", self.ollama_url))?.json().map_err(|e| e.to_string())?;
        Ok(v["models"].as_array().into_iter().flatten().filter_map(|m| m["name"].as_str().map(str::to_string)).collect())
    }
    /// Prüft den OpenRouter-Schlüssel ohne Modellaufruf.
    pub fn openrouter_schluessel_pruefen(&self) -> Result<Value, String> {
        if self.openrouter_schluessel.is_empty() { return Err("OpenRouter-Schlüssel fehlt".into()); }
        let v: Value = self.http.get("https://openrouter.ai/api/v1/key").bearer_auth(&self.openrouter_schluessel).timeout(Duration::from_secs(10)).send()
            .map_err(|e| format!("OpenRouter nicht erreichbar: {}", e.without_url()))?
            .error_for_status().map_err(|e| format!("OpenRouter lehnt den Schlüssel ab ({})", e.status().map(|s| s.as_u16()).unwrap_or(0)))?
            .json().map_err(|e| e.to_string())?;
        Ok(v["data"].clone())
    }
}

/// Liest das JSON-Objekt aus einer Modellantwort (auch mit Codezaun oder Vorrede).
pub fn json_aus_text(text: &str) -> Result<Value, String> {
    let t = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) { if v.is_object() { return Ok(v); } }
    let start = t.find('{').ok_or("Modellantwort enthält kein JSON-Objekt")?;
    let ende = t.rfind('}').ok_or("Modellantwort enthält kein vollständiges JSON-Objekt")?;
    if ende <= start { return Err("Modellantwort enthält kein vollständiges JSON-Objekt".into()); }
    let v: Value = serde_json::from_str(&t[start..=ende]).map_err(|e| format!("Modell-JSON fehlerhaft: {e}"))?;
    if !v.is_object() { return Err("Modellantwort ist kein Objekt".into()); }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Eigener Modellserver: erst lehnt er das Antwortschema ab (400), dann antwortet er ohne Vorgabe.
    #[cfg(feature = "openai")]
    #[test]
    fn eigener_server_mit_rueckfall_ohne_schema() {
        use std::io::{BufRead, BufReader, Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let mut gesehen = Vec::new();
            for (status, body) in [("400 Bad Request", r#"{"error":"response_format json_schema not supported"}"#),
                                   ("200 OK", r#"{"choices":[{"message":{"content":"{\"aktionen\":[],\"begruendung\":\"warten\"}"}}],"usage":{"total_tokens":77}}"#)] {
                let (mut s, _) = l.accept().unwrap();
                let mut r = BufReader::new(s.try_clone().unwrap());
                let (mut kopf, mut laenge) = (String::new(), 0);
                loop { let mut z = String::new(); r.read_line(&mut z).unwrap(); if z.to_lowercase().starts_with("content-length:") { laenge = z[15..].trim().parse().unwrap(); } if z == "\r\n" { break; } kopf.push_str(&z); }
                let mut b = vec![0; laenge]; r.read_exact(&mut b).unwrap();
                gesehen.push(format!("{kopf}\n{}", String::from_utf8_lossy(&b)));
                write!(s, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            gesehen
        });
        let k: Konfig = serde_json::from_value(json!({"version":1,"name":"t","server":"https://spiel.example","rollen":{"verwalter":{"anbieter":"openai","modell":"mein-feintuning"}},
            "gedaechtnis":{"art":"sqlite","pfad":"g.sqlite3"},"openai":{"url":format!("http://127.0.0.1:{port}/v1")}})).unwrap();
        k.pruefen().unwrap();
        let m = Modelle::neu(&k, &Geheimnisse { openai: "sk-lokal-1".into(), ..Default::default() }).unwrap();
        let a = m.fragen(&k.rollen["verwalter"], "SYS", "LAGE", &json!({"type":"object"})).unwrap();
        assert_eq!(json_aus_text(&a.text).unwrap()["begruendung"], "warten");
        assert_eq!((a.tokens, a.anbieter.as_str(), a.modell.as_str()), (77, "openai", "mein-feintuning"));
        let anfragen = h.join().unwrap();
        assert!(anfragen[0].starts_with("POST /v1/chat/completions") && anfragen[0].contains("json_schema") && anfragen[0].to_lowercase().contains("authorization: bearer sk-lokal-1"), "{}", anfragen[0]);
        assert!(!anfragen[1].contains("response_format"), "zweiter Versuch ohne Formatvorgabe");
        // Unverschlüsseltes HTTP nur auf dem eigenen Rechner.
        let mut fremd = k.clone(); fremd.openai.url = "http://10.0.0.5:8000/v1".into();
        assert!(fremd.pruefen().is_err());
    }

    #[test]
    fn json_wird_robust_gelesen() {
        assert_eq!(json_aus_text("```json\n{\"aktionen\":[]}\n```").unwrap()["aktionen"], json!([]));
        assert_eq!(json_aus_text("Hier: {\"a\":1} fertig").unwrap()["a"], 1);
        assert!(json_aus_text("[1,2]").is_err());
        assert!(json_aus_text("nichts").is_err());
    }
}
