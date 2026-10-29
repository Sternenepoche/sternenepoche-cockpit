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
    /// Tatsächlich genutztes Kontextfenster: wächst, wenn ein Prompt nicht hineinpasst, und bleibt dann (sonst lädt
    /// Ollama das Modell bei jedem Wechsel neu).
    ollama_kontext_aktiv: std::sync::atomic::AtomicU32,
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
        Ok(Self { http, ollama_url: k.ollama.url.trim_end_matches('/').into(), ollama_kontext: k.ollama.kontext, ollama_kontext_aktiv: k.ollama.kontext.into(), ollama_nur_cpu: k.ollama.nur_cpu,
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
        let zeichen = system.chars().count() + nutzer.chars().count();
        let mut num_ctx = self.kontext_fuer(system, nutzer, r.max_tokens);
        let mut vorher = 0u64;
        loop {
            let mut optionen = json!({"temperature":r.temperatur,"num_predict":r.max_tokens,"num_ctx":num_ctx});
            if self.ollama_nur_cpu { optionen["num_gpu"] = json!(0); }
            let body = json!({"model":r.modell,"stream":false,"think":false,"format":schema_verpacken(schema),"options":optionen,
                "messages":[{"role":"system","content":system},{"role":"user","content":nutzer}]});
            let v: Value = self.http.post(format!("{}/api/chat", self.ollama_url)).timeout(self.ollama_timeout).json(&body).send()
                .map_err(|e| format!("Ollama nicht erreichbar: {}", e.without_url()))?
                .error_for_status().map_err(|e| format!("Ollama meldet {}", e.status().map(|s| s.as_u16()).unwrap_or(0)))?
                .json().map_err(|e| format!("Ollama-Antwort unlesbar: {e}"))?;
            let prompt = v["prompt_eval_count"].as_u64().unwrap_or(0);
            if prompt_gekuerzt(prompt, zeichen, num_ctx) {
                // Prompt länger als das Fenster: Ollama behält still nur die Hälfte (das Ende). Größer neu fragen – bringt
                // das nichts, ist das Fenster des Modells selbst erschöpft (qwen3:8b: 40 960).
                let neu = num_ctx.saturating_mul(2).min(131_072);
                if neu == num_ctx || prompt == vorher {
                    return Err(format!("Prompt ({zeichen} Zeichen) passt nicht ins Kontextfenster des Modells {}: Ollama verarbeitete nur {prompt} Tokens (num_ctx {num_ctx}) – Kontext kürzen (kontext.max_zeichen, regel_zeichen, skills_zeichen) oder ein Modell mit größerem Fenster wählen", r.modell));
                }
                vorher = prompt;
                self.ollama_kontext_aktiv.fetch_max(neu, std::sync::atomic::Ordering::Relaxed);
                num_ctx = neu;
                continue;
            }
            return Ok(ModellAntwort { text: v["message"]["content"].as_str().unwrap_or("").to_string(),
                tokens: prompt + v["eval_count"].as_u64().unwrap_or(0), ..Default::default() });
        }
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

    /// Kontextfenster für Ollama: Ein zu langer Prompt wird von Ollama gekürzt (es behält die Hälfte des Fensters, das
    /// Ende) – Seele, Regeln und Schema im Systemprompt gingen verloren (KI-Weg-Lauf 27.10.2026: Prompts mit 35 000
    /// Tokens bei Werkseinstellung 16 384). Schätzung 2,8 Zeichen je Token (gemessen 2,98 bei deutschem Text mit JSON)
    /// plus Antwort und Reserve; aufgerundet auf 8 192, höchstens 131 072; das Fenster schrumpft danach nicht wieder.
    pub fn kontext_fuer(&self, system: &str, nutzer: &str, antwort: u32) -> u32 {
        use std::sync::atomic::Ordering;
        let noetig = ((system.chars().count() + nutzer.chars().count()) as f64 / 2.8) as u32 + antwort + 512;
        let aktiv = self.ollama_kontext_aktiv.load(Ordering::Relaxed).max(self.ollama_kontext);
        if noetig <= aktiv { return aktiv; }
        let neu = noetig.div_ceil(8192).saturating_mul(8192).min(131_072);
        self.ollama_kontext_aktiv.fetch_max(neu, Ordering::Relaxed);
        neu
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

/// Hat Ollama den Prompt gekürzt? Gemessen 27.10.2026 (Ollama 0.40.11): Ist der Prompt länger als num_ctx N (oder als
/// das Fenster des Modells), verarbeitet es genau N/2 + 2 Tokens und wirft den Anfang weg. Erkennbar daran oder an
/// einer Tokenzahl unter ~5 Zeichen je Token.
pub fn prompt_gekuerzt(prompt_tokens: u64, zeichen: usize, num_ctx: u32) -> bool {
    if prompt_tokens == 0 || zeichen < 2_000 { return false; }
    let am_rand = [num_ctx as u64 / 2, num_ctx as u64].iter().any(|g| prompt_tokens.abs_diff(*g) <= 8);
    am_rand || (prompt_tokens as f64) < zeichen as f64 / 5.0
}

/// Antwortschema für Modellserver mit Grammatik (Ollama): Aktionen und Abfragen als `{aktion, felder}`.
///
/// KI-Weg-Lauf 28.10.2026: Ollama setzt das Schema mit alphabetisch sortierten Feldern als Grammatik um (die
/// Reihenfolge aus dem Schema geht verloren). Modelle beginnen eine Aktion mit `"typ"`; erlaubt war das nur bei Aktionen,
/// deren übrige Pflichtfelder hinter „typ“ sortieren – `allianz_gruenden` (Feld „name“) war so unerreichbar, der Diplomat
/// schrieb Zug um Zug `allianz_verlassen`, obwohl er laut Begründung gründen wollte. „aktion“ steht vor „felder“: das
/// Modell legt zuerst die Aktion fest. `json_aus_text` macht daraus wieder `{typ, …}`.
pub fn schema_verpacken(s: &Value) -> Value {
    let mut s = s.clone();
    for feld in ["aktionen", "abfragen"] {
        let Some(varianten) = s.pointer_mut(&format!("/properties/{feld}/items/anyOf")).and_then(Value::as_array_mut) else { continue };
        for v in varianten.iter_mut() {
            let Some(props) = v.get("properties").and_then(Value::as_object).filter(|p| p.contains_key("typ")) else { continue };
            let mut felder = props.clone();
            let typ = felder.remove("typ").unwrap_or(Value::Null);
            let pflicht: Vec<Value> = v["required"].as_array().into_iter().flatten().filter(|r| r.as_str() != Some("typ")).cloned().collect();
            let mut neu = json!({"type":"object","additionalProperties":false,"required":["aktion","felder"],
                "properties":{"aktion":typ,"felder":{"type":"object","additionalProperties":v.get("additionalProperties").cloned().unwrap_or(json!(false)),"properties":felder,"required":pflicht}}});
            if let Some(d) = v.get("description") { neu["description"] = d.clone(); }
            *v = neu;
        }
    }
    s
}

/// `{aktion, felder}` (siehe `schema_verpacken`) zurück in `{typ, …}`; alles andere bleibt unverändert.
fn auspacken(v: &mut Value) {
    for feld in ["aktionen", "abfragen"] {
        for x in v.get_mut(feld).and_then(Value::as_array_mut).into_iter().flatten() {
            let Some(o) = x.as_object().filter(|o| o.len() == 2) else { continue };
            if let (Some(t), Some(f)) = (o.get("aktion").and_then(Value::as_str), o.get("felder").and_then(Value::as_object)) {
                let mut neu = serde_json::Map::new();
                neu.insert("typ".into(), json!(t));
                neu.extend(f.clone());
                *x = Value::Object(neu);
            }
        }
    }
}

/// Liest das JSON-Objekt aus einer Modellantwort (auch mit Codezaun oder Vorrede).
pub fn json_aus_text(text: &str) -> Result<Value, String> {
    let t = text.trim();
    let mut v = match serde_json::from_str::<Value>(t) {
        Ok(v) if v.is_object() => v,
        _ => {
            let start = t.find('{').ok_or("Modellantwort enthält kein JSON-Objekt")?;
            let ende = t.rfind('}').ok_or("Modellantwort enthält kein vollständiges JSON-Objekt")?;
            if ende <= start { return Err("Modellantwort enthält kein vollständiges JSON-Objekt".into()); }
            let v: Value = serde_json::from_str(&t[start..=ende]).map_err(|e| format!("Modell-JSON fehlerhaft: {e}"))?;
            if !v.is_object() { return Err("Modellantwort ist kein Objekt".into()); }
            v
        }
    };
    auspacken(&mut v);
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

    /// Ein Prompt über dem eingestellten Fenster vergrößert es (auf 8 192 gerundet) – und es bleibt groß.
    #[test]
    fn kontextfenster_waechst_mit_dem_prompt() {
        let k: Konfig = serde_json::from_value(json!({"version":1,"name":"t","server":"https://spiel.example","rollen":{"verwalter":{"anbieter":"ollama","modell":"x"}},
            "gedaechtnis":{"art":"sqlite","pfad":"g.sqlite3"}})).unwrap();
        let m = Modelle::neu(&k, &Geheimnisse::default()).unwrap();
        assert_eq!(m.kontext_fuer("kurz", "kurz", 1500), 16_384);
        let lang = "ä".repeat(63_000);
        let n = m.kontext_fuer(&lang, "", 2500);
        assert!(n >= 63_000 * 10 / 28 + 2500 && n % 8192 == 0, "{n}");
        assert_eq!(m.kontext_fuer("kurz", "kurz", 1500), n, "Fenster schrumpft nicht (kein Neuladen)");
        assert_eq!(m.kontext_fuer(&"x".repeat(2_000_000), "", 0), 131_072);
        // Gekürzt erkannt: genau das halbe Fenster (16 386 bei 32 768) oder viel zu wenige Tokens für den Text.
        assert!(prompt_gekuerzt(16_386, 150_000, 32_768) && prompt_gekuerzt(9_000, 80_000, 65_536));
        assert!(!prompt_gekuerzt(19_500, 76_000, 65_536) && !prompt_gekuerzt(0, 76_000, 65_536) && !prompt_gekuerzt(300, 1_000, 16_384));
    }

    /// Verpackt für die Grammatik, beim Lesen wieder `{typ, …}`; Antworten in alter Form bleiben gültig.
    #[test]
    fn aktionen_verpackt_und_ausgepackt() {
        let s = json!({"properties":{"aktionen":{"type":"array","items":{"anyOf":[
            {"type":"object","additionalProperties":false,"required":["name","typ"],"properties":{"name":{"type":"string"},"typ":{"enum":["allianz_gruenden"]}}},
            {"type":"object","required":["typ"],"properties":{"typ":{"enum":["allianz_verlassen"]}},"description":"geht"}]}}}});
        let v = schema_verpacken(&s);
        let a = &v["properties"]["aktionen"]["items"]["anyOf"][0];
        assert_eq!(a["required"], json!(["aktion","felder"]));
        assert_eq!(a["properties"]["aktion"], json!({"enum":["allianz_gruenden"]}));
        assert_eq!(a["properties"]["felder"]["required"], json!(["name"]));
        assert!(a["properties"]["felder"]["properties"].get("typ").is_none());
        assert_eq!(v["properties"]["aktionen"]["items"]["anyOf"][1]["description"], "geht");
        let r = json_aus_text(r#"{"aktionen":[{"aktion":"allianz_gruenden","felder":{"name":"Bund"}},{"typ":"bauen","planet":"1:2:3"}],"abfragen":[{"aktion":"kosten","felder":{}}],"begruendung":"x"}"#).unwrap();
        assert_eq!(r["aktionen"][0], json!({"typ":"allianz_gruenden","name":"Bund"}));
        assert_eq!(r["aktionen"][1], json!({"typ":"bauen","planet":"1:2:3"}), "alte Form bleibt");
        assert_eq!(r["abfragen"][0], json!({"typ":"kosten"}));
    }

    #[test]
    fn json_wird_robust_gelesen() {
        assert_eq!(json_aus_text("```json\n{\"aktionen\":[]}\n```").unwrap()["aktionen"], json!([]));
        assert_eq!(json_aus_text("Hier: {\"a\":1} fertig").unwrap()["a"], 1);
        assert!(json_aus_text("[1,2]").is_err());
        assert!(json_aus_text("nichts").is_err());
    }
}
