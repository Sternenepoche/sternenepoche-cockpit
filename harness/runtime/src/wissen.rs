//! Wissensquellen, die der Besitzer frei definiert: das eigene Gedächtnis (nur lesendes
//! SQL auf SQLite oder DuckDB), ein eigener HTTP-Dienst – etwa ein selbst betriebenes
//! RAG – oder ein beliebiger MCP-Server über Streamable HTTP. Wir liefern kein RAG mit;
//! der Harness spricht jede Quelle über diese eine, abgesicherte Schnittstelle an:
//! HTTPS (HTTP nur auf 127.0.0.1), keine Weiterleitungen, Zeitlimit, Größenlimit, Schlüssel
//! nur aus den Geheimnissen. Antworten gelten für Modelle als fremde Daten (Quarantäne).
use crate::gedaechtnis::Speicher;
use crate::konfig::WissenKonfig;
use serde_json::{json, Value};
use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Setzt Frage, Rolle und Anzahl in eine JSON-Vorlage ein. Ersetzt wird in den Werten,
/// nicht im Text: eine Frage kann die Struktur des Körpers nicht verändern.
pub fn vorlage_fuellen(v: &Value, frage: &str, rolle: &str, anzahl: usize) -> Value {
    match v {
        Value::String(s) if s == "{{anzahl}}" => json!(anzahl),
        Value::String(s) => json!(s.replace("{{frage}}", frage).replace("{{rolle}}", rolle).replace("{{anzahl}}", &anzahl.to_string())),
        Value::Array(a) => Value::Array(a.iter().map(|x| vorlage_fuellen(x, frage, rolle, anzahl)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), vorlage_fuellen(x, frage, rolle, anzahl))).collect()),
        andere => andere.clone(),
    }
}

fn lokal(url: &str) -> bool { ["http://127.0.0.1", "http://localhost", "http://[::1]"].iter().any(|p| url.starts_with(p)) }

fn client(w: &WissenKonfig) -> Result<reqwest::blocking::Client, String> {
    crate::konfig::sichere_adresse(&w.url, &format!("Wissensquelle {}", w.name))?;
    reqwest::blocking::Client::builder().timeout(Duration::from_secs(w.timeout_s)).connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none()).https_only(!lokal(&w.url))
        .user_agent(concat!("sternenepoche-harness/", env!("CARGO_PKG_VERSION"))).build().map_err(|e| e.to_string())
}

/// Liest höchstens `max` Bytes; größere Antworten werden abgelehnt statt abgeschnitten.
fn lesen_begrenzt(r: reqwest::blocking::Response, max: usize) -> Result<(String, Vec<u8>), String> {
    let art = r.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    let mut b = Vec::new();
    r.take(max as u64 + 1).read_to_end(&mut b).map_err(|e| format!("Antwort unlesbar: {e}"))?;
    if b.len() > max { return Err(format!("Antwort größer als {} KiB", max / 1024)); }
    Ok((art, b))
}

fn mit_schluessel(req: reqwest::blocking::RequestBuilder, w: &WissenKonfig, schluessel: Option<&str>) -> Result<reqwest::blocking::RequestBuilder, String> {
    match (w.mit_schluessel, schluessel) {
        (true, Some(s)) if !s.is_empty() => Ok(req.header(w.schluessel_kopf.as_str(), s)),
        (true, _) => Err(format!("Schlüssel für {} fehlt (im Cockpit unter Wissen hinterlegen)", w.name)),
        _ => Ok(req),
    }
}

fn http(w: &WissenKonfig, frage: &str, rolle: &str, schluessel: Option<&str>) -> Result<Value, String> {
    let c = client(w)?;
    let req = if w.methode == "GET" {
        let mut url = reqwest::Url::parse(&w.url).map_err(|e| e.to_string())?;
        url.query_pairs_mut().append_pair("q", frage).append_pair("rolle", rolle);
        c.get(url)
    } else {
        let body = w.vorlage.as_ref().map(|v| vorlage_fuellen(v, frage, rolle, 5)).unwrap_or_else(|| json!({"frage":frage,"rolle":rolle,"anzahl":5}));
        c.post(&w.url).json(&body)
    };
    let r = mit_schluessel(req, w, schluessel)?.send().map_err(|e| format!("{} nicht erreichbar: {}", w.name, e.without_url()))?;
    let status = r.status().as_u16();
    let (_, b) = lesen_begrenzt(r, w.max_kb as usize * 1024)?;
    if !(200..300).contains(&status) { return Err(format!("{} antwortet {status}: {}", w.name, String::from_utf8_lossy(&b).chars().take(200).collect::<String>())); }
    let v: Value = serde_json::from_slice(&b).unwrap_or_else(|_| json!(String::from_utf8_lossy(&b)));
    if w.antwort_pfad.is_empty() { Ok(v) } else { v.pointer(&w.antwort_pfad).cloned().ok_or_else(|| format!("{} fehlt in der Antwort von {}", w.antwort_pfad, w.name)) }
}

/// Server-Sent Events: das JSON-RPC-Objekt mit passender id (Streamable HTTP).
pub fn sse_antwort(text: &str, id: i64) -> Option<Value> {
    let mut gefunden = None;
    for ereignis in text.replace("\r\n", "\n").split("\n\n") {
        let daten: String = ereignis.lines().filter_map(|z| z.strip_prefix("data:")).map(str::trim_start).collect::<Vec<_>>().join("\n");
        if let Ok(v) = serde_json::from_str::<Value>(&daten) { if v["id"] == id { gefunden = Some(v); } }
    }
    gefunden
}

struct McpSitzung<'a> { w: &'a WissenKonfig, schluessel: Option<&'a str>, c: reqwest::blocking::Client, sitzung: Option<String> }
impl<'a> McpSitzung<'a> {
    fn rufen(&mut self, methode: &str, params: Value, id: Option<i64>) -> Result<Value, String> {
        let mut body = json!({"jsonrpc":"2.0","method":methode,"params":params});
        if let Some(i) = id { body["id"] = json!(i); }
        let mut req = self.c.post(&self.w.url).header("Accept", "application/json, text/event-stream").header("MCP-Protocol-Version", "2025-06-18").json(&body);
        if let Some(s) = &self.sitzung { req = req.header("Mcp-Session-Id", s.as_str()); }
        let r = mit_schluessel(req, self.w, self.schluessel)?.send().map_err(|e| format!("MCP-Server {} nicht erreichbar: {}", self.w.name, e.without_url()))?;
        let status = r.status().as_u16();
        if let Some(s) = r.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()) { if s.len() <= 200 { self.sitzung = Some(s.to_string()); } }
        let (art, b) = lesen_begrenzt(r, self.w.max_kb as usize * 1024)?;
        if !(200..300).contains(&status) { return Err(format!("MCP-Server {} antwortet {status}", self.w.name)); }
        let Some(i) = id else { return Ok(Value::Null) };
        let text = String::from_utf8_lossy(&b);
        let v = if art.starts_with("text/event-stream") { sse_antwort(&text, i).ok_or("MCP: keine Antwort im Ereignisstrom")? } else { serde_json::from_str(&text).map_err(|_| "MCP: Antwort ist kein JSON".to_string())? };
        if let Some(e) = v.get("error") { return Err(format!("MCP-Fehler: {}", e["message"].as_str().unwrap_or("unbekannt"))); }
        Ok(v["result"].clone())
    }
    fn oeffnen(w: &'a WissenKonfig, schluessel: Option<&'a str>) -> Result<McpSitzung<'a>, String> {
        let mut s = McpSitzung { w, schluessel, c: client(w)?, sitzung: None };
        s.rufen("initialize", json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"sternenepoche-harness","version":env!("CARGO_PKG_VERSION")}}), Some(1))?;
        let _ = s.rufen("notifications/initialized", json!({}), None);
        Ok(s)
    }
}

fn mcp(w: &WissenKonfig, frage: &str, schluessel: Option<&str>) -> Result<Value, String> {
    let mut s = McpSitzung::oeffnen(w, schluessel)?;
    let r = s.rufen("tools/call", json!({"name":w.werkzeug,"arguments":{w.argument.clone():frage}}), Some(2))?;
    let text: Vec<&str> = r["content"].as_array().into_iter().flatten().filter_map(|c| c["text"].as_str()).collect();
    if r["isError"] == true { return Err(format!("Werkzeug {} meldet: {}", w.werkzeug, text.join(" ").chars().take(300).collect::<String>())); }
    Ok(json!({"text":text.join("\n"),"daten":r["structuredContent"]}))
}

/// Werkzeuge eines MCP-Servers (für die Einrichtung im Cockpit).
pub fn mcp_werkzeuge(w: &WissenKonfig, schluessel: Option<&str>) -> Result<Value, String> {
    let mut s = McpSitzung::oeffnen(w, schluessel)?;
    let r = s.rufen("tools/list", json!({}), Some(2))?;
    Ok(json!(r["tools"].as_array().into_iter().flatten().map(|t| json!({"name":t["name"],"beschreibung":t["description"].as_str().unwrap_or("").chars().take(300).collect::<String>(),"eingaben":t["inputSchema"]["properties"]})).collect::<Vec<_>>()))
}

/// Eine Abfrage an eine Wissensquelle. Liefert immer ein Objekt mit ok, quelle und
/// ergebnis oder fehler – für Cockpit, lokale API und Modellwerkzeug gleich.
pub fn abfragen(w: &WissenKonfig, frage: &str, rolle: &str, schluessel: Option<&str>, speicher: Option<&Mutex<Speicher>>) -> Value {
    let start = Instant::now();
    let frage: String = frage.trim().chars().filter(|c| !c.is_control() || *c == '\n').take(2000).collect();
    let r = if frage.is_empty() { Err("Frage fehlt".to_string()) } else {
        match w.art.as_str() {
            "gedaechtnis" => speicher.ok_or("Gedächtnis nicht geöffnet".to_string()).and_then(|s| s.lock().unwrap().nur_lesen(&frage, 200)),
            "http" => http(w, &frage, rolle, schluessel),
            "mcp" => mcp(w, &frage, schluessel),
            andere => Err(format!("Art {andere} unbekannt")),
        }
    };
    let ms = start.elapsed().as_millis() as u64;
    match r {
        Ok(v) => json!({"ok":true,"quelle":w.name,"art":w.art,"frage":frage,"ergebnis":v,"dauer_ms":ms}),
        Err(e) => json!({"ok":false,"quelle":w.name,"art":w.art,"frage":frage,"fehler":e,"dauer_ms":ms}),
    }
}

/// Beschreibung der Quellen einer Rolle für den Systemprompt.
pub fn fuer_rolle<'a>(quellen: &'a [WissenKonfig], rolle: &str) -> Vec<&'a WissenKonfig> {
    quellen.iter().filter(|w| w.rollen.iter().any(|r| r == rolle)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    fn quelle(art: &str, url: &str) -> WissenKonfig {
        serde_json::from_value(json!({"name":"test","art":art,"url":url,"vorlage":{"query":"{{frage}}","k":"{{anzahl}}","filter":{"rolle":"{{rolle}}"}},"antwort_pfad":"/treffer","werkzeug":"suche","mit_schluessel":true,"schluessel_kopf":"X-Api-Key"})).unwrap()
    }
    /// Ein Mini-HTTP-Server für einen einzigen Austausch; gibt die empfangene Anfrage zurück.
    fn einmal(antworten: Vec<(String, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}/rag", l.local_addr().unwrap().port());
        let h = std::thread::spawn(move || {
            let mut gesehen = Vec::new();
            for (art, body) in antworten {
                let (mut s, _) = l.accept().unwrap();
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut kopf = String::new();
                let mut laenge = 0;
                loop { let mut z = String::new(); r.read_line(&mut z).unwrap(); if z.to_lowercase().starts_with("content-length:") { laenge = z[15..].trim().parse().unwrap(); } if z == "\r\n" { break; } kopf.push_str(&z); }
                let mut b = vec![0; laenge]; r.read_exact(&mut b).unwrap();
                gesehen.push(format!("{kopf}\n{}", String::from_utf8_lossy(&b)));
                write!(s, "HTTP/1.1 200 OK\r\nContent-Type: {art}\r\nMcp-Session-Id: s1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            gesehen
        });
        (url, h)
    }
    #[test]
    fn eigenes_rag_ueber_http() {
        let (url, h) = einmal(vec![("application/json".into(), r#"{"treffer":[{"text":"Xator greift nachts an"}]}"#.into())]);
        let w = quelle("http", &url);
        let leer = Mutex::new(Speicher { db: crate::gedaechtnis::oeffnen("sqlite", &std::env::temp_dir().join(format!("w-{}.sqlite3", std::process::id())), std::path::Path::new("")).unwrap() });
        let a = abfragen(&w, "Wer ist \"Xator\"?", "feldherr", Some("geheim-123"), Some(&leer));
        assert_eq!(a["ok"], true, "{a}");
        assert_eq!(a["ergebnis"][0]["text"], "Xator greift nachts an");
        let anfrage = &h.join().unwrap()[0];
        assert!(anfrage.contains("x-api-key: geheim-123") || anfrage.contains("X-Api-Key: geheim-123"), "{anfrage}");
        assert!(anfrage.contains(r#""query":"Wer ist \"Xator\"?""#) && anfrage.contains(r#""k":5"#) && anfrage.contains(r#""rolle":"feldherr""#), "Vorlage sauber befüllt: {anfrage}");
        // Ohne Schlüssel wird gar nicht erst gefragt; fremde Hosts nur mit HTTPS.
        assert_eq!(abfragen(&w, "x", "feldherr", None, Some(&leer))["ok"], false);
        assert!(client(&quelle("http", "http://example.org/rag")).is_err());
        // SQL auf das eigene Gedächtnis: nur lesend.
        let g = quelle("gedaechtnis", "");
        assert_eq!(abfragen(&g, "SELECT count(*) AS n FROM notizen", "x", None, Some(&leer))["ergebnis"]["zeilen"][0][0], 0);
        for boese in ["DELETE FROM notizen", "SELECT 1; DROP TABLE notizen", "ATTACH 'x.db' AS y", "SELECT * FROM notizen -- x", "WITH a AS (DELETE FROM notizen) SELECT 1"] {
            assert_eq!(abfragen(&g, boese, "x", None, Some(&leer))["ok"], false, "{boese}");
        }
        let endlos = abfragen(&g, "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c) SELECT count(*) FROM c", "x", None, Some(&leer));
        assert!(endlos["ok"] == false && endlos["dauer_ms"].as_u64().unwrap() < 8000, "endlose Abfrage wird abgebrochen: {endlos}");
    }
    #[test]
    fn mcp_server_mit_ereignisstrom() {
        let init = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{}}}"#;
        let aufruf = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"Treffer A\"}],\"isError\":false}}\n\n";
        let (url, h) = einmal(vec![("application/json".into(), init.into()), ("application/json".into(), String::new()), ("text/event-stream".into(), aufruf.into())]);
        let mut w = quelle("mcp", &url);
        w.mit_schluessel = false;
        let leer = Mutex::new(Speicher { db: crate::gedaechtnis::oeffnen("sqlite", &std::env::temp_dir().join(format!("m-{}.sqlite3", std::process::id())), std::path::Path::new("")).unwrap() });
        let a = abfragen(&w, "Flottenstärke Xator", "feldherr", None, Some(&leer));
        assert_eq!(a["ergebnis"]["text"], "Treffer A", "{a}");
        let gesehen = h.join().unwrap();
        assert!(gesehen[2].to_lowercase().contains("mcp-session-id: s1") && gesehen[2].contains(r#""query":"Flottenstärke Xator""#), "{}", gesehen[2]);
    }
}
