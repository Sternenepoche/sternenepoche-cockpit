//! Entwicklungsansicht (--entwicklung PORT): dieselbe Oberfläche mit denselben Daten in
//! einem normalen Browser, nur auf 127.0.0.1, nur mit Zufallsschlüssel in der Adresse und
//! nur für die eigene Herkunft (fremde Seiten werden abgewiesen). Für Gestaltung und
//! Prüfung; im normalen Betrieb ist sie aus.
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::io::Read;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Response, Server};

#[derive(Default)]
struct Speicher { seq: u64, letzte: BTreeMap<String, (u64, Value)>, strom: VecDeque<(u64, String, Value)> }

pub struct Entwicklung { speicher: Arc<Mutex<Speicher>>, pub befehle: Receiver<String>, pub adresse: String }

impl Entwicklung {
    /// `ui_ordner`: Oberflächendateien von der Platte statt eingebettet (Gestalten ohne Neubau).
    pub fn starten(port: u16, ui_ordner: Option<std::path::PathBuf>) -> Result<Self, String> {
        let server = Server::http(("127.0.0.1", port)).map_err(|e| format!("Entwicklungsansicht auf Port {port}: {e}"))?;
        let mut b = [0u8; 16];
        getrandom::getrandom(&mut b).map_err(|e| e.to_string())?;
        let schluessel: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let adresse = format!("http://127.0.0.1:{port}/index.html?t={schluessel}");
        let speicher: Arc<Mutex<Speicher>> = Default::default();
        let (tx, rx) = channel();
        let sp = speicher.clone();
        std::thread::spawn(move || for mut req in server.incoming_requests() {
            let url = req.url().to_string();
            let (pfad, abfrage) = url.split_once('?').unwrap_or((&url, ""));
            let param = |k: &str| abfrage.split('&').filter_map(|p| p.split_once('=')).find(|(a, _)| *a == k).map(|(_, v)| v.to_string());
            let herkunft = req.headers().iter().find(|h| h.field.equiv("Origin")).map(|h| h.value.to_string());
            let fremd = herkunft.is_some_and(|o| o != format!("http://127.0.0.1:{port}"));
            let antwort = |code: u16, art: &str, b: Vec<u8>| {
                let mut r = Response::from_data(b).with_status_code(code);
                r.add_header(Header::from_bytes("Content-Type", art).unwrap());
                r.add_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
                r.add_header(Header::from_bytes("Content-Security-Policy", crate::oberflaeche::CSP).unwrap());
                r
            };
            if pfad.starts_with("/api/") {
                if fremd || param("t").as_deref() != Some(schluessel.as_str()) { let _ = req.respond(antwort(403, "application/json", b"{\"error\":\"verboten\"}".to_vec())); continue; }
                match (req.method().clone(), pfad) {
                    (Method::Get, "/api/zustand") => {
                        let seit: u64 = param("seit").and_then(|s| s.parse().ok()).unwrap_or(0);
                        let s = sp.lock().unwrap();
                        let mut n: Vec<Value> = if seit == 0 { s.letzte.iter().map(|(a, (_, d))| json!([a, d])).collect() } else { vec![] };
                        n.extend(s.strom.iter().filter(|(q, _, _)| *q > seit).map(|(_, a, d)| json!([a, d])));
                        let _ = req.respond(antwort(200, "application/json", json!({"seq":s.seq,"nachrichten":n}).to_string().into_bytes()));
                    }
                    (Method::Post, "/api/befehl") => {
                        let mut b = String::new();
                        let _ = req.as_reader().take(1 << 20).read_to_string(&mut b);
                        let _ = tx.send(b);
                        let _ = req.respond(antwort(200, "application/json", b"{\"ok\":true}".to_vec()));
                    }
                    _ => { let _ = req.respond(antwort(404, "application/json", b"{}".to_vec())); }
                }
                continue;
            }
            let sicher = pfad.len() > 1 && !pfad.contains("..") && pfad.bytes().all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b));
            if let Some(d) = ui_ordner.as_ref().filter(|_| sicher).and_then(|o| std::fs::read(o.join(&pfad[1..])).ok()) {
                let _ = req.respond(antwort(200, crate::oberflaeche::art(pfad), d));
                continue;
            }
            if let Some(b) = crate::stimmen::live(pfad) { let _ = req.respond(antwort(200, "audio/wav", b)); continue; }
            match crate::oberflaeche::datei(pfad) {
                Some(d) => { let _ = req.respond(antwort(200, crate::oberflaeche::art(if pfad == "/" { "index.html" } else { pfad }), d.to_vec())); }
                None => { let _ = req.respond(antwort(404, "text/plain", b"nicht gefunden".to_vec())); }
            }
        });
        Ok(Self { speicher, befehle: rx, adresse })
    }
    /// Wie Oberflaeche::senden: Zustandsarten (zustand, auswahl, daten, katalog) als letzter Stand,
    /// Ereignisse als Strom.
    pub fn senden(&self, art: &str, daten: &Value) {
        let mut s = self.speicher.lock().unwrap();
        s.seq += 1;
        let q = s.seq;
        if art == "ereignisse" || art == "detail" { s.strom.push_back((q, art.into(), daten.clone())); while s.strom.len() > 400 { s.strom.pop_front(); } }
        else { s.letzte.insert(art.into(), (q, daten.clone())); s.strom.push_back((q, art.into(), daten.clone())); while s.strom.len() > 400 { s.strom.pop_front(); } }
    }
}
