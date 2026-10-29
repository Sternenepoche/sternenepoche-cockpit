//! Regeln und Aktionen im Systemprompt, ohne mitten im Text abzuschneiden.
//! - Der Aktionskatalog des Servers (`GET aktionen?rolle=R`) wird zu einem kompakten,
//!   vollständigen Block: jede Aktion der Rolle mit Zweck, Beispiel und den wichtigsten Fallen.
//! - Der Regeltext (`schema.regeltext`) wird nach Abschnitten (`## …`) budgetiert: ganze
//!   Abschnitte oder gar nicht; ausgelassene stehen namentlich im Prompt (nachschlagbar mit
//!   `{typ:"regel",stichwort}`). Ohne Katalog bleibt „## Aktionen deiner Rolle“ immer vollständig.
use serde_json::Value;

pub const AKTIONEN_TITEL: &str = "Aktionen deiner Rolle";

#[derive(Debug, Clone, PartialEq)]
pub struct Abschnitt { pub titel: String, pub text: String }

/// Zerlegt den Regeltext in Abschnitte. Der Abschnitt „Aktionen deiner Rolle“ umfasst seine
/// Überschrift und die folgende Aufzählung (`- …`); was danach ohne eigene Überschrift folgt
/// (Kolonisation, Ausscheiden, Invasion), wird zu „Weitere Regeln“.
pub fn abschnitte(t: &str) -> Vec<Abschnitt> {
    let mut out: Vec<Abschnitt> = Vec::new();
    let mut titel = String::new();
    let mut text = String::new();
    let mut in_aktionen = false;
    let schliessen = |out: &mut Vec<Abschnitt>, titel: &str, text: &mut String| {
        if !text.trim().is_empty() { out.push(Abschnitt { titel: titel.to_string(), text: text.trim_end().to_string() }); }
        text.clear();
    };
    for zeile in t.lines() {
        if let Some(t2) = zeile.strip_prefix("## ") {
            schliessen(&mut out, &titel, &mut text);
            titel = t2.trim().to_string();
            in_aktionen = titel == AKTIONEN_TITEL;
            text.push_str(zeile);
            text.push('\n');
            continue;
        }
        if in_aktionen && !zeile.trim().is_empty() && !zeile.starts_with("- ") && !zeile.starts_with("  ") {
            schliessen(&mut out, &titel, &mut text);
            titel = "Weitere Regeln".into();
            in_aktionen = false;
        }
        text.push_str(zeile);
        text.push('\n');
    }
    schliessen(&mut out, &titel, &mut text);
    out
}

/// Regeltext im Budget. `mit_katalog`: die Aktionsliste kommt aus dem Katalog und fällt hier weg.
/// Sonst steht „## Aktionen deiner Rolle“ immer vollständig drin, auch über dem Budget.
/// Ergebnis: Text und Titel der ausgelassenen Abschnitte.
pub fn regeltext_budget(t: &str, budget: usize, mit_katalog: bool) -> (String, Vec<String>) {
    let teile = abschnitte(t);
    let n = |s: &str| s.chars().count() + 2;
    let pflicht: usize = if mit_katalog { 0 } else { teile.iter().filter(|a| a.titel == AKTIONEN_TITEL).map(|a| n(&a.text)).sum() };
    let mut rest = budget.saturating_sub(pflicht);
    let mut drin = vec![false; teile.len()];
    let mut ausgelassen = Vec::new();
    for (i, a) in teile.iter().enumerate() {
        if a.titel == AKTIONEN_TITEL { drin[i] = !mit_katalog; continue; }
        if n(&a.text) <= rest { rest -= n(&a.text); drin[i] = true; } else { ausgelassen.push(if a.titel.is_empty() { "Einleitung".to_string() } else { a.titel.clone() }); }
    }
    let mut s: String = teile.iter().zip(&drin).filter(|(_, d)| **d).map(|(a, _)| a.text.as_str()).collect::<Vec<_>>().join("\n\n");
    if !ausgelassen.is_empty() {
        s.push_str(&format!("\n\n(Aus Platzgründen ausgelassene Regelabschnitte – bei Bedarf mit {{typ:\"regel\",stichwort}} nachschlagen: {})", ausgelassen.join(", ")));
    }
    (s, ausgelassen)
}

fn kurz(s: &str, n: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= n { return s; }
    let t: String = s.chars().take(n).collect();
    format!("{}…", t.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&t))
}
fn texte(v: &Value) -> Vec<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => vec![s.clone()],
        Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(str::to_string).or_else(|| x["text"].as_str().map(str::to_string))).filter(|s| !s.trim().is_empty()).collect(),
        _ => vec![],
    }
}

/// Einträge des Katalogs: Liste von Objekten mit `typ` oder Objekt `typ → Eintrag`.
pub fn katalog_eintraege(k: &Value) -> Vec<(String, Value)> {
    let a = if k["aktionen"].is_null() { k } else { &k["aktionen"] };
    match a {
        Value::Array(l) => l.iter().filter_map(|e| e["typ"].as_str().map(|t| (t.to_string(), e.clone()))).collect(),
        Value::Object(m) => m.iter().map(|(t, e)| (t.clone(), e.clone())).collect(),
        _ => vec![],
    }
}

/// Aktionen, die fast jeder Rat braucht: im Aktionsblock immer ausführlich. Allianz- und Dysonaktionen nur, wenn Agenda
/// oder Ziele sie anbieten (KI-Weg-Lauf 28.10.2026: ausführlich gelistet, versuchte qwen3:8b sie Zug um Zug ohne Allianz).
pub const KERN_AKTIONEN: [&str; 11] = ["bauen", "forschen", "fertigen", "flotte_senden", "markt_order", "stufenaufstieg", "doktrin", "topf_umbuchen",
    "brief_senden", "vertrag_anbieten", "boden_senden"];
/// Obergrenze des Aktionsblocks in Zeichen (KI-Weg-Lauf 27.10.2026: mit vollen Beispielen über 90 000 Zeichen für den
/// Verwalter; Ollama behielt vom 40 000-Token-Prompt nur das Ende, Regeln und Seele fielen weg).
pub const AKTIONEN_MAX: usize = 16_000;

/// Beispiel ohne leere Werte (0, false, null, "", {}, []): Pflichtfelder erzwingt das Antwortschema ohnehin.
pub fn beispiel_knapp(v: &Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(o.iter().filter_map(|(k, x)| {
            let x = beispiel_knapp(x);
            let leer = match &x { Value::Null => true, Value::Bool(b) => !b, Value::Number(n) => n.as_f64() == Some(0.0),
                Value::String(s) => s.is_empty(), Value::Array(a) => a.is_empty(), Value::Object(m) => m.is_empty() };
            (!leer || k == "typ").then(|| (k.clone(), x))
        }).collect()),
        Value::Array(a) => Value::Array(a.iter().map(beispiel_knapp).collect()),
        x => x.clone(),
    }
}

/// Kompakter, vollständiger Aktionsblock der Rolle. `typen` sind die laut Antwortschema
/// erlaubten Typen: jede davon erscheint, auch wenn der Katalog sie (noch) nicht beschreibt.
/// `wichtig` (Typen aus Agenda- und Zielvorlagen) und `KERN_AKTIONEN` bekommen Zweck, Kennungen, Beispiel und Fallen,
/// alle übrigen eine Kurzzeile; der Block bleibt unter `AKTIONEN_MAX`.
pub fn aktionen_block(katalog: &Value, rolle: &str, typen: &[String], wichtig: &[String]) -> String {
    let mut eintraege: Vec<(String, Value)> = katalog_eintraege(katalog).into_iter()
        .filter(|(_, e)| e["rollen"].as_array().is_none_or(|r| r.iter().any(|x| x == rolle || x == "alle")))
        .filter(|(t, _)| typen.is_empty() || typen.contains(t)).collect();
    for t in typen { if !eintraege.iter().any(|(x, _)| x == t) { eintraege.push((t.clone(), Value::Null)); } }
    let ausfuehrlich = |t: &str| wichtig.iter().any(|w| w == t) || KERN_AKTIONEN.contains(&t);
    let zeile = |t: &str, e: &Value, voll: bool| {
        let mut z = format!("- {t}");
        if e["unumkehrbar"] == true { z.push_str(" [unumkehrbar]"); }
        let zweck = e["zweck"].as_str().unwrap_or("");
        if !zweck.is_empty() { z.push_str(&format!(" – {}", kurz(zweck, if voll { 140 } else { 70 }))); }
        if !voll { if e.is_null() { z.push_str(" (siehe Antwortschema)"); } return z; }
        let ids = texte(&e["ids_aus"]);
        if !ids.is_empty() { z.push_str(&format!(" IDs aus: {}.", kurz(&ids.join(", "), 90))); }
        match &e["beispiel"] {
            Value::Null => if e.is_null() { z.push_str(" (siehe Antwortschema)"); },
            Value::String(b) => z.push_str(&format!(" Beispiel: {}", kurz(b, 260))),
            b => z.push_str(&format!(" Beispiel: {}", kurz(&beispiel_knapp(b).to_string(), 260))),
        }
        let fallen = texte(&e["fallen"]);
        if !fallen.is_empty() { z.push_str(&format!(" Achtung: {}", fallen.iter().take(2).map(|f| kurz(f, 90)).collect::<Vec<_>>().join(" | "))); }
        z
    };
    let mut s = String::from("Jede Zeile: typ – Zweck (wichtige Aktionen mit Kennungen, Beispiel ohne leere Felder und Fallen). Felder und Pflichtangaben stehen im Antwortschema; IDs nur aus der Lage oder einer Abfrage, nie erfinden.\n");
    let mut zeilen: Vec<String> = eintraege.iter().map(|(t, e)| zeile(t, e, ausfuehrlich(t))).collect();
    // Über der Grenze: erst die Kernliste (ohne Agenda-Typen) auf Kurzzeilen, dann alles.
    if zeilen.iter().map(|z| z.chars().count() + 1).sum::<usize>() > AKTIONEN_MAX {
        zeilen = eintraege.iter().map(|(t, e)| zeile(t, e, wichtig.iter().any(|w| w == t))).collect();
    }
    if zeilen.iter().map(|z| z.chars().count() + 1).sum::<usize>() > AKTIONEN_MAX {
        zeilen = eintraege.iter().map(|(t, e)| zeile(t, e, false)).collect();
    }
    for z in zeilen { s.push_str(&z); s.push('\n'); }
    // Missionen für flotte_senden (aus missionen.json des Servers).
    let missionen: Vec<String> = match &katalog["missionen"] {
        Value::Array(l) => l.iter().filter_map(|m| {
            let name = m["mission"].as_str().or(m["name"].as_str()).or(m["typ"].as_str()).or(m.as_str())?;
            if m["rollen"].as_array().is_some_and(|r| !r.iter().any(|x| x == rolle || x == "alle")) { return None; }
            let zweck = m["zweck"].as_str().map(|z| format!(": {}", kurz(z, 90))).unwrap_or_default();
            let feind = if m["feindlich"] == true { " (feindlich)" } else { "" };
            Some(format!("{name}{feind}{zweck}"))
        }).collect(),
        Value::Object(o) => o.iter().filter(|(_, m)| m["rollen"].as_array().is_none_or(|r| r.iter().any(|x| x == rolle || x == "alle")))
            .map(|(n, m)| format!("{n}{}{}", if m["feindlich"] == true { " (feindlich)" } else { "" }, m["zweck"].as_str().map(|z| format!(": {}", kurz(z, 90))).unwrap_or_default())).collect(),
        _ => vec![],
    };
    if !missionen.is_empty() && eintraege.iter().any(|(t, _)| t == "flotte_senden") { s.push_str(&format!("Missionen für flotte_senden: {}\n", missionen.join("; "))); }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn regeltext() -> String {
        let mut t = String::new();
        for i in 0..30 { t.push_str(&format!("## Abschnitt {i}\n{}\n\n", "Regelsatz mit Inhalt. ".repeat(40))); }
        t.push_str("## Aktionen deiner Rolle\n");
        for i in 0..40 { t.push_str(&format!("- {{\"typ\":\"aktion_{i}\",\"planet\":\"1:2:3\"}}\n")); }
        t.push_str("\nKolonisation: Freie Ziele müssen zuerst erkundet sein.\nAusscheiden: Ein Reich wird besiegt, wenn …");
        t
    }
    #[test]
    fn aktionsliste_wird_nie_abgeschnitten() {
        let t = regeltext();
        assert!(t.chars().count() > 26_000, "so lang wie ein echter Rollen-Regeltext");
        let (s, aus) = regeltext_budget(&t, 12_000, false);
        for i in 0..40 { assert!(s.contains(&format!("\"typ\":\"aktion_{i}\"")), "aktion_{i} fehlt"); }
        assert!(s.contains("## Aktionen deiner Rolle"));
        assert!(!aus.is_empty() && s.contains("ausgelassene Regelabschnitte"));
        // Keine halben Abschnitte: jeder enthaltene Abschnitt ist vollständig.
        for a in abschnitte(&t) { if s.contains(&format!("## {}\n", a.titel)) { assert!(s.contains(&a.text), "{} nur halb", a.titel); } }
        assert!(s.chars().count() <= 12_000 + 600, "Budget plus Hinweis: {}", s.chars().count());
        // Selbst mit winzigem Budget bleibt die Aktionsliste vollständig.
        let (klein, _) = regeltext_budget(&t, 100, false);
        assert!(klein.contains("aktion_39") && klein.contains("aktion_0"));
        // Mit Katalog fällt die Liste weg (der Katalogblock ersetzt sie), der Rest bleibt erhalten.
        let (mit, _) = regeltext_budget(&t, 50_000, true);
        assert!(!mit.contains("aktion_5") && mit.contains("Kolonisation: Freie Ziele") && mit.contains("## Abschnitt 29"));
        let teile = abschnitte(&t);
        assert_eq!(teile.last().unwrap().titel, "Weitere Regeln");
    }
    #[test]
    fn katalogblock_ist_vollstaendig() {
        let k = json!({"aktionen":[
            {"typ":"bauen","rollen":["verwalter"],"zweck":"Gebäude ausbauen","beispiel":{"typ":"bauen","planet":"1:2:3","gebaeude":"erzmine"},"fallen":["Strom vorher prüfen","Bauschleife voll","dritte Falle"],"ids_aus":["lage.planeten"]},
            {"typ":"abreissen","rollen":["verwalter"],"zweck":"Stufe abbauen","unumkehrbar":true,"beispiel":"{\"typ\":\"abreissen\"}"},
            {"typ":"angriff_fremd","rollen":["feldherr"],"zweck":"gehört nicht hierher"},
            {"typ":"flotte_senden","rollen":["verwalter","feldherr"],"zweck":"Flotte schicken","beispiel":{"typ":"flotte_senden","mission":"transport"}}],
            "missionen":[{"mission":"transport","zweck":"Güter bringen","rollen":["verwalter","feldherr"]},{"mission":"angriff","feindlich":true,"rollen":["feldherr"]}]});
        let typen: Vec<String> = ["bauen", "abreissen", "flotte_senden", "forschen"].iter().map(|s| s.to_string()).collect();
        let b = aktionen_block(&k, "verwalter", &typen, &["abreissen".to_string()]);
        assert!(b.contains("- bauen – Gebäude ausbauen IDs aus: lage.planeten. Beispiel: {") && b.contains("\"gebaeude\":\"erzmine\""), "{b}");
        assert!(b.contains("Strom vorher prüfen | Bauschleife voll") && !b.contains("dritte Falle"), "höchstens zwei Fallen");
        assert!(b.contains("- abreissen [unumkehrbar] – Stufe abbauen Beispiel:"), "Agenda-Typ ausführlich: {b}");
        assert!(!b.contains("angriff_fremd"), "fremde Rollen fallen weg");
        assert!(b.contains("- forschen (siehe Antwortschema)"), "Typen aus dem Schema fehlen nie");
        assert!(b.contains("Missionen für flotte_senden: transport: Güter bringen") && !b.contains("angriff (feindlich)"));
        // Katalog als Objekt typ → Eintrag; seltene Aktion nur als Kurzzeile.
        let k2 = json!({"aktionen":{"bauen":{"zweck":"x"},"selten":{"zweck":"y","beispiel":{"typ":"selten","a":1},"fallen":["f"]}}});
        let b2 = aktionen_block(&k2, "verwalter", &[], &[]);
        assert!(b2.contains("- bauen – x") && b2.contains("- selten – y\n"), "{b2}");
    }

    /// Echter Umfang: 120 Aktionen mit vollen Beispielen (alle Schiffe auf 0) bleiben unter der Grenze, und die
    /// Beispiele verlieren ihre leeren Felder.
    #[test]
    fn aktionsblock_bleibt_im_budget() {
        let schiffe: serde_json::Map<String, Value> = (0..40).map(|i| (format!("schiff_{i}"), json!(if i == 3 { 2 } else { 0 }))).collect();
        let k = json!({"aktionen":(0..120).map(|i| json!({"typ":format!("aktion_{i}"),"rollen":["verwalter"],"zweck":"Zweck ".repeat(30),
            "ids_aus":["lage.planeten[].koord"],"beispiel":{"typ":format!("aktion_{i}"),"flug":{"schiffe":schiffe,"ladung":{"erz":0},"mission":"transport"},"probe":false},
            "fallen":["Falle eins ".repeat(10),"Falle zwei ".repeat(10)]})).collect::<Vec<_>>()});
        let wichtig: Vec<String> = (0..6).map(|i| format!("aktion_{i}")).collect();
        let b = aktionen_block(&k, "verwalter", &[], &wichtig);
        assert!(b.chars().count() <= AKTIONEN_MAX + 400, "{} Zeichen", b.chars().count());
        assert!(b.contains("- aktion_119 – ") && b.contains("\"schiff_3\":2") && !b.contains("\"schiff_0\""), "alle Aktionen da, Beispiel knapp");
        assert_eq!(beispiel_knapp(&json!({"typ":"x","a":0,"b":{"c":false,"d":"z"},"e":[]})), json!({"typ":"x","b":{"d":"z"}}));
    }
}
