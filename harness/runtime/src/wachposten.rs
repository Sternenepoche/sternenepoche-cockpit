//! Wachposten: lokales Sicherheits-Gate vor jedem Spielzug, aufgebaut wie die
//! Rechteverwaltung eines modernen Harness. Je Rolle und Aktionstyp gilt
//! „erlauben“, „fragen“ (wartet auf den Besitzer) oder „verbieten“. Dazu kommen feste
//! Grenzen (Aktionen je Zug, Geschenke je Tag). Der Server prüft danach ohnehin alles.
use crate::konfig::WachpostenKonfig;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct Pruefung {
    pub frei: Vec<Value>,
    pub wartend: Vec<(Value, String)>,
    pub abgelehnt: Vec<(Value, String)>,
}

/// Schlüssel einer Aktion für die Rechte, z. B. „schenken“ oder „flotte_senden:angriff“.
pub fn kennung(a: &Value) -> String {
    let typ = a["typ"].as_str().unwrap_or("");
    match a["mission"].as_str() { Some(m) if typ == "flotte_senden" => format!("{typ}:{m}"), _ => typ.to_string() }
}

/// Wert eines Marktauftrags: Menge × Preis, ohne Preis die Menge.
fn marktwert(a: &Value) -> f64 {
    let menge = a["menge"].as_f64().unwrap_or(0.0).abs();
    match a["preis"].as_f64() { Some(p) if p.is_finite() && p > 0.0 => menge * p, _ => menge }
}

/// Kennung mit Wachposten-Grenzen: große Marktaufträge heißen „markt_order:gross“.
pub fn kennung_mit(k: &WachpostenKonfig, a: &Value) -> String {
    if a["typ"] == "markt_order" && marktwert(a) >= k.markt_frage_ab { "markt_order:gross".into() } else { kennung(a) }
}

/// Gilt für eine Aktion: zuerst die genaue Kennung, dann der Typ, sonst die Grundregel.
pub fn recht(k: &WachpostenKonfig, rechte: &BTreeMap<String, String>, a: &Value) -> String {
    let typ = a["typ"].as_str().unwrap_or("");
    let genau = kennung_mit(k, a);
    if let Some(r) = rechte.get(&genau).or_else(|| rechte.get(&kennung(a))).or_else(|| rechte.get(typ)) { return r.clone(); }
    let riskant = k.riskant.iter().any(|r| *r == genau || *r == kennung(a) || *r == typ);
    if riskant && k.modus == "bestaetigen" { "fragen".into() } else { "erlauben".into() }
}

pub fn pruefen(k: &WachpostenKonfig, rechte: &BTreeMap<String, String>, erlaubte_typen: &[String], aktionen: &[Value], geschenkt_heute: f64) -> Pruefung {
    let mut p = Pruefung::default();
    let mut geschenkt = geschenkt_heute;
    for a in aktionen {
        let Some(typ) = a.get("typ").and_then(Value::as_str) else { p.abgelehnt.push((a.clone(), "Aktion ohne typ".into())); continue };
        if !erlaubte_typen.is_empty() && !erlaubte_typen.iter().any(|t| t == typ) {
            p.abgelehnt.push((a.clone(), format!("{typ} gehört nicht zu dieser Rolle"))); continue;
        }
        if p.frei.len() + p.wartend.len() >= k.max_aktionen_je_zug {
            p.abgelehnt.push((a.clone(), format!("Mehr als {} Aktionen in einem Zug", k.max_aktionen_je_zug))); continue;
        }
        let mut regel = recht(k, rechte, a);
        if regel == "verbieten" { p.abgelehnt.push((a.clone(), format!("{} ist für diese Rolle verboten", kennung_mit(k, a)))); continue; }
        if typ == "schenken" {
            let betrag = a["credits"].as_f64().unwrap_or(0.0).max(0.0);
            if geschenkt + betrag > k.max_geschenk_credits_je_tag {
                if regel == "erlauben" && k.modus == "automatisch" {
                    p.abgelehnt.push((a.clone(), format!("Geschenk über dem Tageslimit von {} Credits", k.max_geschenk_credits_je_tag))); continue;
                }
                regel = "fragen".into();
            } else { geschenkt += betrag; }
        }
        if regel == "fragen" { p.wartend.push((a.clone(), format!("{} braucht deine Bestätigung", kennung_mit(k, a)))); }
        else { p.frei.push(a.clone()); }
    }
    p
}

/// Erlaubte Aktionstypen einer Rolle aus dem Antwortschema des Servers.
pub fn typen_aus_schema(schema: &Value) -> Vec<String> {
    schema["properties"]["aktionen"]["items"]["anyOf"].as_array().into_iter().flatten()
        .filter_map(|v| v["properties"]["typ"]["enum"][0].as_str().map(str::to_string)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn k(modus: &str) -> WachpostenKonfig { WachpostenKonfig { modus: modus.into(), ..Default::default() } }
    #[test]
    fn rechte_matrix_und_grenzen() {
        let typen = vec!["bauen".to_string(), "schenken".into(), "flotte_senden".into(), "abreissen".into()];
        let aktionen = vec![json!({"typ":"bauen","planet":"1:2:3","gebaeude":"farm"}), json!({"typ":"schenken","an":"Xator","credits":50000}),
            json!({"typ":"flotte_senden","mission":"angriff"}), json!({"typ":"flotte_senden","mission":"transport"}), json!({"typ":"brief_senden"}), json!({"keintyp":1})];
        let leer = BTreeMap::new();
        let p = pruefen(&k("bestaetigen"), &leer, &typen, &aktionen, 0.0);
        assert_eq!((p.frei.len(), p.wartend.len(), p.abgelehnt.len()), (2, 2, 2));
        let auto = pruefen(&k("automatisch"), &leer, &typen, &aktionen[..3], 0.0);
        assert_eq!(auto.frei.len(), 2);
        assert!(auto.abgelehnt[0].1.contains("Tageslimit"), "Geschenk auch automatisch begrenzt");
        // Besitzer-Rechte überschreiben die Grundregel.
        let mut rechte = BTreeMap::new();
        rechte.insert("flotte_senden:angriff".to_string(), "verbieten".to_string());
        rechte.insert("abreissen".to_string(), "fragen".to_string());
        rechte.insert("bauen".to_string(), "fragen".to_string());
        let r = pruefen(&k("automatisch"), &rechte, &typen, &[json!({"typ":"flotte_senden","mission":"angriff"}), json!({"typ":"abreissen"}), json!({"typ":"bauen"})], 0.0);
        assert!(r.abgelehnt[0].1.contains("verboten"));
        assert_eq!(r.wartend.len(), 2);
        let viele: Vec<Value> = (0..7).map(|_| json!({"typ":"bauen"})).collect();
        assert_eq!(pruefen(&k("automatisch"), &leer, &typen, &viele, 0.0).abgelehnt.len(), 3);
    }
    #[test]
    fn neue_missionen_und_grosse_marktauftraege_fragen() {
        let typen: Vec<String> = ["flotte_senden", "markt_order", "planet_aufgeben", "dyson_auszahlen", "bauen"].iter().map(|s| s.to_string()).collect();
        let leer = BTreeMap::new();
        let w = k("bestaetigen");
        for m in ["invasion", "bombardieren", "kampfkolonisieren", "mond_zerstoeren", "blockade", "angriff"] {
            assert_eq!(recht(&w, &leer, &json!({"typ":"flotte_senden","mission":m})), "fragen", "{m}");
        }
        for m in ["transport", "stationieren", "system_erkunden", "saven", "terraformen", "spionage"] {
            assert_eq!(recht(&w, &leer, &json!({"typ":"flotte_senden","mission":m})), "erlauben", "{m}");
        }
        let klein = json!({"typ":"markt_order","gut":"erz","seite":"kauf","menge":1000,"preis":0.8});
        let gross = json!({"typ":"markt_order","gut":"erz","seite":"kauf","menge":200000,"preis":1.5});
        assert_eq!(recht(&w, &leer, &klein), "erlauben");
        assert_eq!(recht(&w, &leer, &gross), "fragen");
        let p = pruefen(&w, &leer, &typen, &[gross.clone(), json!({"typ":"planet_aufgeben"}), json!({"typ":"dyson_auszahlen"}), json!({"typ":"bauen"})], 0.0);
        assert_eq!((p.frei.len(), p.wartend.len()), (1, 3));
        assert!(p.wartend[0].1.contains("markt_order:gross"));
        // Eigene Rechte des Besitzers: erst die genaue Kennung, dann der Typ.
        let mut rechte = BTreeMap::new();
        rechte.insert("markt_order".to_string(), "erlauben".to_string());
        assert_eq!(recht(&w, &rechte, &gross), "erlauben");
        rechte.insert("markt_order:gross".to_string(), "verbieten".to_string());
        assert_eq!(recht(&w, &rechte, &gross), "verbieten");
        assert_eq!(recht(&w, &rechte, &klein), "erlauben");
    }
}
