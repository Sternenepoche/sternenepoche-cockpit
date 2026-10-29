//! Rats-Pinwand des Servers (Harness-API v1, Abschnitt 4): Abbildung der Planungsfelder
//! einer Modellantwort (aufgaben, an_team) auf Pinwand-Operationen und kompakte Karten
//! für den Prompt. Reine Funktionen; das Senden übernimmt die Schleife. Fehlt die Pinwand
//! auf dem Server, bleibt die lokale Planung (Gedächtnis) der Weg.
use crate::konfig::ROLLEN;
use serde_json::{json, Value};

/// Kartenarten laut Vertrag.
pub const ARTEN: [&str; 6] = ["auftrag", "anfrage", "zusage", "beschluss", "befund", "warnung"];
/// Status laut Vertrag; die letzten drei sind abgeschlossen.
pub const STATUS: [&str; 7] = ["offen", "angenommen", "in_arbeit", "blockiert", "erledigt", "abgelehnt", "hinfaellig"];
/// Status, die das Modell setzen darf (dazu „aktiv“ aus der lokalen Planung).
pub const STATUS_MODELL: [&str; 8] = ["offen", "angenommen", "in_arbeit", "aktiv", "blockiert", "erledigt", "abgelehnt", "hinfaellig"];

fn kurz(s: &str, n: usize) -> String { s.chars().filter(|c| !c.is_control() || *c == '\n').take(n).collect::<String>().trim().to_string() }

/// Lokaler Status → Pinwand-Status („aktiv“ der lokalen Planung heißt dort „in_arbeit“).
pub fn status_server(s: &str) -> Option<&'static str> {
    match s { "aktiv" => Some("in_arbeit"), x => STATUS.iter().copied().find(|t| *t == x) }
}
/// Pinwand-Status → Status des lokalen Spiegels (offen, aktiv, erledigt, blockiert).
pub fn status_lokal(s: &str) -> &'static str {
    match s { "angenommen" | "in_arbeit" | "aktiv" => "aktiv", "blockiert" => "blockiert", "erledigt" | "abgelehnt" | "hinfaellig" => "erledigt", _ => "offen" }
}

/// Empfänger: eine Rolle, „alle“ oder „besitzer“ (der Mensch hinter dem Reich).
fn empfaenger(an: &Value, eigene: &str) -> Result<Vec<String>, String> {
    let liste: Vec<String> = match an {
        Value::String(s) => vec![s.clone()],
        Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
        _ => vec![],
    };
    let mut gueltig: Vec<String> = liste.into_iter().filter(|r| r == "alle" || r == "besitzer" || (ROLLEN.contains(&r.as_str()) && r != eigene)).take(5).collect();
    gueltig.sort();
    gueltig.dedup();
    if gueltig.is_empty() { Err("Empfänger fehlt (eine andere Rolle, alle oder besitzer)".into()) } else { Ok(gueltig) }
}

/// `an_team`-Nachricht → `{op:"neu", art:"anfrage"|"befund"|"warnung", …}`. Ohne Art gilt:
/// eine Frage (mit „?“) ist eine Anfrage, sonst ein Befund.
pub fn an_team_karte(m: &Value, rolle: &str) -> Result<Value, String> {
    let text = kurz(m["text"].as_str().unwrap_or(""), 1500);
    if text.is_empty() { return Err("Nachricht ohne Text".into()); }
    let an = empfaenger(&m["an"], rolle)?;
    let art = match m["art"].as_str() {
        Some(a @ ("anfrage" | "befund" | "warnung")) => a,
        Some(_) => return Err("an_team: art anfrage, befund oder warnung".into()),
        None => if text.contains('?') { "anfrage" } else { "befund" },
    };
    let titel = m["titel"].as_str().map(|t| kurz(t, 100)).filter(|t| !t.is_empty()).unwrap_or_else(|| kurz(text.lines().next().unwrap_or(""), 100));
    Ok(json!({"op":"neu","art":art,"an":an,"titel":titel,"text":text}))
}

/// Eine Aufgaben-Operation des Modells → Pinwand-Operation.
/// - `neu`: Auftrag (oder `art` beschluss/zusage/warnung) an `rolle`.
/// - `status`: `karte` (oder `id`) und Status; `erledigt` nur mit Beleg (request_id eines
///   bestätigten Zugs, `beleg` wird von der Schleife geprüft und übergeben).
/// - `notiz`: Antwort auf eine Karte.
pub fn aufgabe_karte(op: &Value, rolle: &str, beleg: Option<&str>) -> Result<Value, String> {
    let karte = || op["karte"].as_i64().or_else(|| op["id"].as_i64()).ok_or_else(|| "karte fehlt (Nummer der Pinwand-Karte)".to_string());
    match op["op"].as_str() {
        Some("neu") => {
            let titel = kurz(op["titel"].as_str().unwrap_or(""), 100);
            if titel.is_empty() { return Err("Aufgabe ohne Titel".into()); }
            let art = op["art"].as_str().unwrap_or("auftrag");
            if !["auftrag", "beschluss", "zusage", "warnung", "anfrage", "befund"].contains(&art) { return Err(format!("Kartenart {art} unbekannt")); }
            if art == "beschluss" && rolle != "stratege" { return Err("Beschlüsse fasst nur der Stratege".into()); }
            let an = match op.get("rolle").filter(|v| !v.is_null()) {
                Some(v) => empfaenger(v, "").unwrap_or_else(|_| vec![rolle.to_string()]),
                None => if art == "beschluss" { vec!["alle".into()] } else { vec![rolle.to_string()] },
            };
            let mut k = json!({"op":"neu","art":art,"an":an,"titel":titel,"text":kurz(op["text"].as_str().unwrap_or(""), 1500)});
            if let Some(p) = op["prioritaet"].as_i64() { k["prioritaet"] = json!(p.clamp(1, 3)); }
            if let Some(f) = op["frist_stunden"].as_f64().filter(|f| f.is_finite() && *f > 0.0) { k["frist_stunden"] = json!(f.min(720.0)); }
            Ok(k)
        }
        Some("status") => {
            let st = op["status"].as_str().and_then(status_server).ok_or("status: offen, angenommen, in_arbeit, blockiert, erledigt, abgelehnt oder hinfaellig")?;
            let mut k = json!({"op":"status","karte":karte()?,"status":st});
            if let Some(t) = op["text"].as_str().map(|t| kurz(t, 800)).filter(|t| !t.is_empty()) { k["text"] = json!(t); }
            if st == "erledigt" {
                let b = beleg.ok_or("erledigt nur mit Beleg: erst wenn der Server den Zug bestätigt hat (beleg_request_id eines eigenen, angenommenen Zugs); sonst in_arbeit setzen")?;
                k["beleg_request_id"] = json!(b);
            }
            Ok(k)
        }
        Some("notiz") => {
            let text = kurz(op["text"].as_str().unwrap_or(""), 800);
            if text.is_empty() { return Err("Notiz ohne Text".into()); }
            let mut k = json!({"op":"antworten","karte":karte()?,"text":text});
            if let Some(st) = op["status"].as_str().and_then(status_server).filter(|s| *s != "erledigt") { k["status"] = json!(st); }
            Ok(k)
        }
        _ => Err("unbekannte Aufgaben-Operation (neu, status, notiz)".into()),
    }
}

/// Allianz-Pinwand (`bereich: "allianz"`): Operation des Modells aus `an_allianz` → Pinwand-Operation. `an` sind
/// Spielernamen der Mitglieder oder „alle“ (der Server prüft sie); Beschlüsse nur für die Allianzleitung (prüft der
/// Server); erledigt wie im Rat nur mit Beleg eines eigenen, angenommenen Zugs.
pub fn allianz_karte(op: &Value, beleg: Option<&str>) -> Result<Value, String> {
    let karte = || op["karte"].as_i64().ok_or_else(|| "karte fehlt (Nummer der Allianz-Karte)".to_string());
    let mut k = match op["op"].as_str() {
        Some("neu") => {
            let titel = kurz(op["titel"].as_str().unwrap_or(""), 100);
            if titel.is_empty() { return Err("Allianz-Karte ohne Titel".into()); }
            let art = op["art"].as_str().unwrap_or("anfrage");
            if !ARTEN.contains(&art) { return Err(format!("Kartenart {art} unbekannt")); }
            let an: Vec<String> = match &op["an"] {
                Value::String(s) => vec![kurz(s, 60)],
                Value::Array(a) => a.iter().filter_map(|x| x.as_str()).map(|s| kurz(s, 60)).filter(|s| !s.is_empty()).take(12).collect(),
                _ => vec![],
            };
            let an = if an.is_empty() { vec!["alle".to_string()] } else { an };
            let mut k = json!({"op":"neu","art":art,"an":an,"titel":titel,"text":kurz(op["text"].as_str().unwrap_or(""), 1500)});
            if let Some(p) = op["prioritaet"].as_i64() { k["prioritaet"] = json!(p.clamp(1, 3)); }
            if let Some(f) = op["frist_stunden"].as_f64().filter(|f| f.is_finite() && *f > 0.0) { k["frist_stunden"] = json!(f.min(720.0)); }
            k
        }
        Some("antworten") => {
            let text = kurz(op["text"].as_str().unwrap_or(""), 800);
            if text.is_empty() { return Err("Antwort ohne Text".into()); }
            let mut k = json!({"op":"antworten","karte":karte()?,"text":text});
            if let Some(st) = op["status"].as_str().and_then(status_server).filter(|s| *s != "erledigt") { k["status"] = json!(st); }
            k
        }
        Some("status") => {
            let st = op["status"].as_str().and_then(status_server).ok_or("status: offen, angenommen, in_arbeit, blockiert, erledigt, abgelehnt oder hinfaellig")?;
            let mut k = json!({"op":"status","karte":karte()?,"status":st});
            if let Some(t) = op["text"].as_str().map(|t| kurz(t, 800)).filter(|t| !t.is_empty()) { k["text"] = json!(t); }
            if st == "erledigt" {
                k["beleg_request_id"] = json!(beleg.ok_or("erledigt nur mit Beleg: request_id eines eigenen, vom Server angenommenen Zugs; sonst in_arbeit setzen")?);
            }
            k
        }
        _ => return Err("an_allianz: op neu, antworten oder status".into()),
    };
    k["bereich"] = json!("allianz");
    Ok(k)
}

/// Karten aus einer Lage oder Pinwand-Antwort (`karten`, `pinwand`, Liste).
pub fn karten_aus(v: &Value) -> Option<Vec<Value>> {
    let liste = if v.is_array() { v } else if v["karten"].is_array() { &v["karten"] } else if v["pinwand"].is_array() { &v["pinwand"] } else if v["pinwand"]["karten"].is_array() { &v["pinwand"]["karten"] } else { return None };
    liste.as_array().cloned()
}

/// Ist die Karte für diese Rolle offen (an sie oder von ihr, nicht abgeschlossen)?
pub fn betrifft(k: &Value, rolle: &str) -> bool {
    let an = k["an"].as_array().map(|a| a.iter().any(|x| x == rolle || x == "alle")).unwrap_or(k["an"] == rolle || k["an"] == "alle");
    let offen = !["erledigt", "abgelehnt", "hinfaellig"].contains(&k["status"].as_str().unwrap_or("offen"));
    offen && (an || k["von"] == rolle)
}

/// Karte kompakt für den Prompt; Texte werden mit `entschaerfen` begrenzt.
/// Nummer einer Karte: `id` (volle Karte aus `GET pinwand`) oder `karte` (Kurzform in der Lage).
pub fn nummer(k: &Value) -> Value { if k["id"].is_null() { k["karte"].clone() } else { k["id"].clone() } }

pub fn kompakt(k: &Value, entschaerfen: &dyn Fn(&str, usize) -> String) -> Value {
    let alle: &[Value] = k["antworten"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut antworten: Vec<Value> = alle[alle.len().saturating_sub(2)..].iter()
        .map(|a| json!({"von":a["von"],"text":entschaerfen(a["text"].as_str().unwrap_or(""), 240),"status":a["status"]})).collect();
    // Kurzform der Lage: nur Zahl der Antworten und die letzte.
    if antworten.is_empty() && k["letzte_antwort"].is_object() {
        antworten.push(json!({"von":k["letzte_antwort"]["von"],"text":entschaerfen(k["letzte_antwort"]["text"].as_str().unwrap_or(""), 240)}));
    }
    let mut v = json!({"karte":nummer(k),"version":k["version"],"art":k["art"],"von":k["von"],"an":k["an"],"titel":entschaerfen(k["titel"].as_str().unwrap_or(""), 100),
        "status":k["status"],"prioritaet":k["prioritaet"]});
    if let Some(t) = k["text"].as_str().filter(|t| !t.is_empty()) { v["text"] = json!(entschaerfen(t, 400)); }
    for f in ["frist_text", "ueberfaellig", "bezug", "beleg"] { if !k[f].is_null() && k[f] != false { v[f] = k[f].clone(); } }
    if !antworten.is_empty() { v["antworten"] = json!(antworten); }
    let n = k["antworten"].as_array().map(Vec::len).or(k["antworten"].as_u64().map(|n| n as usize)).unwrap_or(0);
    if n > antworten.len() { v["antworten_gesamt"] = json!(n); }
    v
}

/// Jüngste Änderung einer offenen Karte, die eine andere Rolle (oder der Besitzer) an diese Rolle gerichtet
/// oder auf einer eigenen Karte beantwortet hat.
pub fn neueste_fremde(karten: &[Value], rolle: &str) -> i64 {
    let fremd = |k: &&Value| k["von"] != rolle || k["antworten"].as_array().and_then(|a| a.last()).is_some_and(|a| a["von"] != rolle);
    karten.iter().filter(|k| betrifft(k, rolle)).filter(fremd).filter_map(|k| k["geaendert"].as_i64().or(k["erstellt"].as_i64())).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_team_wird_anfrage_oder_befund() {
        let k = an_team_karte(&json!({"an":"verwalter","text":"Brauchst du Kristall für die Werft?"}), "diplomat").unwrap();
        assert_eq!(k, json!({"op":"neu","art":"anfrage","an":["verwalter"],"titel":"Brauchst du Kristall für die Werft?","text":"Brauchst du Kristall für die Werft?"}));
        let b = an_team_karte(&json!({"an":"alle","text":"Xator spioniert 1:27:9 aus.\nZweite Zeile"}), "feldherr").unwrap();
        assert_eq!((b["art"].as_str(), b["titel"].as_str(), b["an"].clone()), (Some("befund"), Some("Xator spioniert 1:27:9 aus."), json!(["alle"])));
        assert_eq!(an_team_karte(&json!({"an":"stratege","text":"x","art":"warnung"}), "verwalter").unwrap()["art"], "warnung");
        assert!(an_team_karte(&json!({"an":"diplomat","text":"an mich selbst"}), "diplomat").is_err(), "nicht an sich selbst");
        assert!(an_team_karte(&json!({"an":"verwalter","text":"x","art":"beschluss"}), "diplomat").is_err());
    }
    #[test]
    fn allianz_karten_mit_namen_und_beleg() {
        let k = allianz_karte(&json!({"op":"neu","art":"auftrag","an":["Sonnenrat10"],"titel":"Anteil einzahlen","text":"Rest fehlt","prioritaet":1}), None).unwrap();
        assert_eq!(k, json!({"op":"neu","art":"auftrag","an":["Sonnenrat10"],"titel":"Anteil einzahlen","text":"Rest fehlt","prioritaet":1,"bereich":"allianz"}));
        assert_eq!(allianz_karte(&json!({"op":"neu","titel":"Lage im System"}), None).unwrap()["an"], json!(["alle"]));
        assert!(allianz_karte(&json!({"op":"status","karte":3,"status":"erledigt"}), None).unwrap_err().contains("Beleg"));
        assert_eq!(allianz_karte(&json!({"op":"status","karte":3,"status":"erledigt"}), Some("z-1")).unwrap()["beleg_request_id"], "z-1");
        assert_eq!(allianz_karte(&json!({"op":"antworten","karte":3,"text":"Zusage","status":"erledigt"}), None).unwrap().get("status"), None);
        assert!(allianz_karte(&json!({"op":"aendern","karte":3}), None).is_err());
    }
    #[test]
    fn aufgaben_werden_karten_mit_beleg() {
        let neu = aufgabe_karte(&json!({"op":"neu","titel":"Kolonie 1:27:9 sichern","rolle":"feldherr","prioritaet":1,"text":"40 Kreuzer","frist_stunden":12}), "stratege", None).unwrap();
        assert_eq!(neu, json!({"op":"neu","art":"auftrag","an":["feldherr"],"titel":"Kolonie 1:27:9 sichern","text":"40 Kreuzer","prioritaet":1,"frist_stunden":12.0}));
        let beschluss = aufgabe_karte(&json!({"op":"neu","titel":"Keine Angriffe auf Allianz Nord","art":"beschluss"}), "stratege", None).unwrap();
        assert_eq!(beschluss["an"], json!(["alle"]));
        assert!(aufgabe_karte(&json!({"op":"neu","titel":"x","art":"beschluss"}), "verwalter", None).is_err(), "Beschluss nur Stratege");
        let st = aufgabe_karte(&json!({"op":"status","karte":12,"status":"aktiv"}), "feldherr", None).unwrap();
        assert_eq!(st, json!({"op":"status","karte":12,"status":"in_arbeit"}));
        assert!(aufgabe_karte(&json!({"op":"status","karte":12,"status":"erledigt"}), "feldherr", None).unwrap_err().contains("Beleg"));
        let fertig = aufgabe_karte(&json!({"op":"status","id":12,"status":"erledigt","text":"40 Kreuzer stationiert"}), "feldherr", Some("abc123")).unwrap();
        assert_eq!(fertig, json!({"op":"status","karte":12,"status":"erledigt","text":"40 Kreuzer stationiert","beleg_request_id":"abc123"}));
        let notiz = aufgabe_karte(&json!({"op":"notiz","karte":12,"text":"Übernommen","status":"in_arbeit"}), "feldherr", None).unwrap();
        assert_eq!(notiz, json!({"op":"antworten","karte":12,"text":"Übernommen","status":"in_arbeit"}));
        assert!(aufgabe_karte(&json!({"op":"status","status":"offen"}), "feldherr", None).is_err(), "Karte fehlt");
        assert_eq!((status_lokal("in_arbeit"), status_lokal("hinfaellig"), status_lokal("offen")), ("aktiv", "erledigt", "offen"));
    }
    #[test]
    fn karten_filter_und_kompakt() {
        let karten = vec![json!({"id":1,"version":2,"art":"auftrag","von":"stratege","an":["feldherr"],"titel":"A","text":"t","status":"offen","geaendert":50,"antworten":[{"von":"feldherr","text":"1"},{"von":"feldherr","text":"2"},{"von":"stratege","text":"3"}]}),
            json!({"id":2,"art":"befund","von":"verwalter","an":["alle"],"titel":"B","status":"erledigt","geaendert":90}),
            json!({"id":3,"art":"anfrage","von":"feldherr","an":["stratege"],"titel":"C","status":"offen","geaendert":70})];
        let fuer: Vec<i64> = karten.iter().filter(|k| betrifft(k, "feldherr")).filter_map(|k| k["id"].as_i64()).collect();
        assert_eq!(fuer, vec![1, 3], "abgeschlossene fallen weg, eigene bleiben");
        assert_eq!(neueste_fremde(&karten, "feldherr"), 50);
        let k = kompakt(&karten[0], &|s, n| s.chars().take(n).collect());
        assert_eq!(k["antworten"].as_array().unwrap().len(), 2);
        assert_eq!(k["antworten_gesamt"], 3);
        // Kurzform aus der Lage (karte statt id, Zahl der Antworten, letzte Antwort).
        let kurz = kompakt(&json!({"karte":5,"version":2,"art":"anfrage","von":"diplomat","an":["verwalter"],"titel":"Kristall?","status":"offen","antworten":3,"letzte_antwort":{"von":"verwalter","text":"morgen"}}), &|s, n| s.chars().take(n).collect());
        assert_eq!((kurz["karte"].clone(), kurz["antworten"][0]["text"].clone(), kurz["antworten_gesamt"].clone()), (json!(5), json!("morgen"), json!(3)));
        assert_eq!(karten_aus(&json!({"karten":[1,2]})).unwrap().len(), 2);
        assert_eq!(karten_aus(&json!({"lage":{},"pinwand":[{"id":1}]})).unwrap().len(), 1);
        assert!(karten_aus(&json!({"lage":{}})).is_none());
    }
}
