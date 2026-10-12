//! Routinen: wiederkehrende Aufträge ohne Modell, wie Cron-Jobs eines Agenten-Harness.
//! - Inventur: alle Planeten als Liste (und als Zeitreihe im Gedächtnis).
//! - Analyse: Engpässe, Reichweiten, leere Schleifen, Trends – als Befunde mit Vorschlag.
//! - Auto-Ausbau: füllt leere Bauschleifen nach festen, nachvollziehbaren Regeln.
//!   Die Aufträge laufen wie jeder Zug durch Wachposten, Probe und Server.
//! Alles hier rechnet nur mit dem Lagebild der Harness-API; nichts davon fragt ein Modell.
use serde_json::{json, Value};

const ROHSTOFFE: [&str; 4] = ["erz", "kristall", "deuterium", "nahrung"];
/// Grundversorgung, wenn der Besitzer keine eigene Reihenfolge vorgibt.
pub const STANDARD_REIHENFOLGE: [&str; 7] = ["erzmine", "kristallmine", "solarkraftwerk", "farm", "deuteriumsynthesizer", "lager", "wohnblock"];

fn f(v: &Value) -> f64 { v.as_f64().unwrap_or(0.0) }
fn kurz_zahl(x: f64) -> String { if x.abs() >= 10_000.0 { format!("{:.1}k", x / 1000.0) } else { format!("{x:.0}") } }

/// Ein Planet als Zeile der Inventur: das Wichtigste auf einen Blick.
pub fn planet_zeile(p: &Value) -> Value {
    let saldo = f(&p["energie"]["erzeugung"]) - f(&p["energie"]["verbrauch"]);
    let mut fuellung = serde_json::Map::new();
    for g in ROHSTOFFE {
        let grenze = f(&p["lager"][g]);
        if grenze > 0.0 { fuellung.insert(g.into(), json!(((f(&p["bestand"][g]) / grenze) * 100.0).round())); }
    }
    let schleife = p["bauschleife"].as_array().cloned().unwrap_or_default();
    let summe = |v: &Value| v.as_object().map(|o| o.values().map(f).sum::<f64>()).unwrap_or(0.0);
    let baubar_jetzt: Vec<String> = p["baubar"].as_array().into_iter().flatten()
        .filter(|b| b["fehlt"].as_array().is_some_and(|x| x.is_empty()) && b["braucht"].as_array().is_some_and(|x| x.is_empty()))
        .filter_map(|b| b["gebaeude"].as_str().map(|g| format!("{g} {}", b["stufe"]))).collect();
    json!({
        "koord":p["koord"],"heimat":p["heimat"],"zone":p["zone"],
        "felder":p["felder"],"bevoelkerung":p["bevoelkerung"],"wohnraum":p["wohnraum"],"stabilitaet":p["stabilitaet"],
        "energie":{"erzeugung":p["energie"]["erzeugung"],"verbrauch":p["energie"]["verbrauch"],"saldo":saldo},
        "bestand":p["bestand"],"rate":p["rate"],"lager":p["lager"],"fuellung_prozent":fuellung,
        "voll_in_stunden":p["voll_in_stunden"],"vorrat_reicht_sekunden":p["vorrat_reicht_sekunden"],
        "nahrung_deckung":p["nahrung_deckung"],"konsum_deckung":p["konsum_deckung"],"arbeit":p["arbeit"],"fachkraefte":p["fachkraefte"],
        "gebaeude":p["gebaeude"],
        "bauschleife":{"belegt":schleife.len(),"plaetze":p["bauschleife_plaetze"],"aktuell":schleife.first().map(|a| json!({"gebaeude":a["gebaeude"],"stufe":a["stufe"],"rest_min":a["rest_min"]}))},
        "fertigung":p["fertigung"].as_array().map(Vec::len).unwrap_or(0),
        "schiffe":summe(&p["schiffe"]),"verteidigung":summe(&p["verteidigung"]),"blockade":p["blockade"],
        "baubar_jetzt":baubar_jetzt,
    })
}

/// Inventur: alle Planeten des Reichs als Liste, dazu Summen über das Reich.
pub fn inventur(l: &Value) -> Value {
    let v = &l["lage"];
    let planeten: Vec<Value> = v["planeten"].as_array().into_iter().flatten().map(planet_zeile).collect();
    let mut bestand = serde_json::Map::new();
    let mut rate = serde_json::Map::new();
    for g in ["erz", "kristall", "deuterium", "nahrung", "legierung", "elektronik", "konsumgut", "xenokristall"] {
        bestand.insert(g.into(), json!(planeten.iter().map(|p| f(&p["bestand"][g])).sum::<f64>()));
        rate.insert(g.into(), json!(planeten.iter().map(|p| f(&p["rate"][g])).sum::<f64>()));
    }
    json!({"reich":v["name"],"stufe":v["stufe"],"rang":v["rang"],"spielerzahl":v["spielerzahl"],"punkte":v["punkte"]["gesamt"],"credits":v["credits"],"planeten":planeten,
        "summe":{"planeten":planeten.len(),"bestand":bestand,"rate_je_stunde":rate,
            "energie_saldo":planeten.iter().map(|p| f(&p["energie"]["saldo"])).sum::<f64>(),
            "bevoelkerung":planeten.iter().map(|p| f(&p["bevoelkerung"])).sum::<f64>(),
            "leere_bauschleifen":planeten.iter().filter(|p| p["bauschleife"]["belegt"] == 0).count(),
            "schiffe":planeten.iter().map(|p| f(&p["schiffe"])).sum::<f64>(),"flotten_unterwegs":v["flotten"].as_array().map(Vec::len)}})
}

fn gut_name(g: &str) -> String {
    match g { "konsumgut" => "Konsumgüter".into(), "xenokristall" => "Xenokristall".into(), _ => { let mut c = g.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() } }
}

fn ist_strom(b: &Value) -> bool { b["ertrag"]["art"] == "Strom" }

fn befund(stufe: u8, planet: &str, thema: &str, text: String, rolle: &str, vorschlag: &str) -> Value {
    json!({"stufe":stufe,"planet":planet,"thema":thema,"text":text,"rolle":rolle,"vorschlag":vorschlag})
}

/// Analyse: Befunde mit Dringlichkeit (2 = sofort, 1 = bald, 0 = Hinweis), wer handeln sollte
/// und ein konkreter Vorschlag. `vorher` ist die letzte Inventur (für Trends).
pub fn analyse(l: &Value, vorher: &[Value]) -> Value {
    let v = &l["lage"];
    let mut b = Vec::new();
    for a in v["angriffe"].as_array().into_iter().flatten() {
        b.push(befund(2, a["ziel"].as_str().unwrap_or(""), "Angriff", format!("Feindlicher Anflug auf {}, Ankunft {}", a["ziel"].as_str().unwrap_or("?"), a["ankunft"].as_str().unwrap_or("?")), "feldherr", "Verteidigung prüfen, Flotte saven oder Verstärkung schicken"));
    }
    for p in v["planeten"].as_array().into_iter().flatten() {
        let k = p["koord"].as_str().unwrap_or("?");
        let (erz, verb) = (f(&p["energie"]["erzeugung"]), f(&p["energie"]["verbrauch"]));
        if erz < verb { b.push(befund(2, k, "Energie", format!("Energie fehlt: Erzeugung {erz:.0}, Verbrauch {verb:.0} – Gebäude arbeiten gedrosselt"), "verwalter", "Kraftwerk ausbauen")); }
        let nd = f(&p["nahrung_deckung"]);
        if p["nahrung_deckung"].is_number() && nd < 100.0 { b.push(befund(2, k, "Nahrung", format!("Nahrung deckt nur {nd:.0} % des Bedarfs"), "verwalter", "Farm ausbauen oder Nahrung liefern")); }
        let kd = f(&p["konsum_deckung"]);
        if p["konsum_deckung"].is_number() && kd < 100.0 && f(&p["rate"]["konsumgut"]) < 0.0 { b.push(befund(1, k, "Konsumgüter", format!("Konsumgüter decken nur {kd:.0} % – das drückt die Stabilität"), "verwalter", "Konsumgüterwerk bauen oder Konsumgüter kaufen")); }
        for (g, s) in p["vorrat_reicht_sekunden"].as_object().into_iter().flatten() {
            let h = f(s) / 3600.0;
            if h < 6.0 { b.push(befund(if h < 1.0 { 2 } else { 1 }, k, "Vorrat", if h < 0.05 { format!("{} ist aufgebraucht", gut_name(g)) } else { format!("{} reicht nur noch {h:.1} h", gut_name(g)) }, "verwalter", "Erzeugung steigern, Verbrauch senken oder liefern")); }
        }
        for (g, s) in p["voll_in_stunden"].as_object().into_iter().flatten() {
            let h = f(s);
            if h < 2.0 { b.push(befund(1, k, "Lager", format!("Lager für {} {}", gut_name(g), if h <= 0.0 { "ist voll – Erzeugung geht verloren".to_string() } else { format!("in {h:.1} h voll") }), "verwalter", "Lager ausbauen oder Rohstoffe ausgeben/verschicken")); }
        }
        let belegt = p["bauschleife"].as_array().map(Vec::len).unwrap_or(0);
        if belegt == 0 { b.push(befund(1, k, "Bauschleife", "Bauschleife ist leer".into(), "verwalter", "Nächsten Ausbau planen (Auto-Ausbau kann das übernehmen)")); }
        if f(&p["arbeit"]["verfuegbar"]) < f(&p["arbeit"]["bedarf"]) { b.push(befund(1, k, "Arbeitskräfte", format!("Arbeitskräfte fehlen: {} verfügbar, {} gebraucht", p["arbeit"]["verfuegbar"], p["arbeit"]["bedarf"]), "verwalter", "Wohnblock bauen oder Prioritäten setzen")); }
        if f(&p["fachkraefte"]["verfuegbar"]) < f(&p["fachkraefte"]["bedarf"]) { b.push(befund(1, k, "Fachkräfte", format!("Fachkräfte fehlen: {} verfügbar, {} gebraucht", p["fachkraefte"]["verfuegbar"], p["fachkraefte"]["bedarf"]), "verwalter", "Akademie ausbauen")); }
        let st = f(&p["stabilitaet"]);
        if p["stabilitaet"].is_number() && st < 40.0 { b.push(befund(2, k, "Stabilität", format!("Stabilität {st:.0} – Unruhen drohen, Bauten können pausieren"), "verwalter", "Versorgung sichern, Steuer senken")); }
        let (fb, fg) = (f(&p["felder"]["belegt"]), f(&p["felder"]["gesamt"]));
        if fg > 0.0 && fb / fg >= 0.9 { b.push(befund(1, k, "Felder", format!("Bauplätze fast belegt: {fb:.0} von {fg:.0}"), "stratege", "Terraforming oder neue Kolonie planen")); }
        if !p["blockade"].is_null() { b.push(befund(2, k, "Blockade", format!("Planet blockiert durch {}", p["blockade"]["durch"].as_str().unwrap_or("?")), "feldherr", "Blockade brechen oder Diplomatie")); }
        // Trend gegenüber der letzten Inventur: sinkt ein Rohstoff, obwohl die Rate positiv ist (Ausgaben)?
        if let Some(alt) = vorher.iter().find(|x| x["koord"] == p["koord"]) {
            let stunden = ((crate::gedaechtnis::jetzt() - alt["inventur_zeit"].as_i64().unwrap_or(0)) as f64 / 3600.0).max(0.0);
            if stunden >= 0.25 {
                for g in ROHSTOFFE {
                    let d = (f(&p["bestand"][g]) - f(&alt["bestand"][g])) / stunden;
                    if d < -1.0 && f(&p["bestand"][g]) < f(&p["lager"][g]) * 0.1 { b.push(befund(1, k, "Trend", format!("{g} sinkt um {} je Stunde und ist fast aufgebraucht", kurz_zahl(-d)), "verwalter", "Ausgaben drosseln oder Erzeugung erhöhen")); }
                }
            }
        }
    }
    if v["forschung"]["aktiv"].is_null() && v["forschung"]["moeglich"].as_array().is_some_and(|m| m.iter().any(|x| x["fehlt"].as_array().is_some_and(|f| f.is_empty()))) {
        b.push(befund(1, "", "Forschung", "Keine Forschung aktiv, obwohl eine bezahlbar wäre".into(), "verwalter", "Forschung starten (z. B. Energietechnik)"));
    }
    if let Some(n) = v["naechste_stufe"].as_object() {
        let offen: Vec<String> = n.get("bedingungen").and_then(Value::as_array).into_iter().flatten().filter(|x| x["erfuellt"] != true).filter_map(|x| x["text"].as_str().map(str::to_string)).collect();
        if !offen.is_empty() { b.push(befund(0, "", "Stufenaufstieg", format!("Für {} fehlt: {}", n.get("name").and_then(Value::as_str).unwrap_or("die nächste Stufe"), offen.join(", ")), "stratege", "Aufgaben für Verwalter anlegen")); }
    }
    b.sort_by(|x, y| y["stufe"].as_u64().cmp(&x["stufe"].as_u64()));
    let zaehl = |s: u64| b.iter().filter(|x| x["stufe"].as_u64() == Some(s)).count();
    let kurz = format!("{} dringend · {} bald · {} Hinweise", zaehl(2), zaehl(1), zaehl(0));
    json!({"befunde":b,"kurz":kurz,"warnungen":v["warnungen"],"inventur":inventur(l)["summe"]})
}

/// Geplanter Bauauftrag mit Begründung (für Cockpit, Gedächtnis und Teamnachricht).
pub struct Auftrag { pub aktion: Value, pub grund: String }

/// Auto-Ausbau: je Planet mit leerer Bauschleife höchstens ein Auftrag, insgesamt höchstens
/// `max`. Regeln in dieser Reihenfolge: Energie vor allem, dann Nahrung, volle Lager,
/// fehlende Arbeitskräfte, danach die Reihenfolge des Besitzers (niedrigste Stufe zuerst,
/// damit alles gleichmäßig wächst). Nur bezahlbare Stufen ohne fehlende Voraussetzung;
/// Verbraucher nur, wenn der Strom danach noch reicht.
pub fn ausbau_plan(l: &Value, reihenfolge: &[String], max: usize) -> (Vec<Auftrag>, Vec<String>) {
    let ordnung: Vec<String> = if reihenfolge.is_empty() { STANDARD_REIHENFOLGE.iter().map(|s| s.to_string()).collect() } else { reihenfolge.to_vec() };
    let mut auftraege = Vec::new();
    let mut uebersprungen = Vec::new();
    let mut planeten: Vec<&Value> = l["lage"]["planeten"].as_array().into_iter().flatten().collect();
    planeten.sort_by_key(|p| p["heimat"] != true);
    for p in planeten {
        if auftraege.len() >= max { break; }
        let k = p["koord"].as_str().unwrap_or("?");
        if p["bauschleife"].as_array().is_some_and(|s| !s.is_empty()) { continue; }
        let saldo = f(&p["energie"]["erzeugung"]) - f(&p["energie"]["verbrauch"]);
        let arbeit_frei = f(&p["arbeit"]["verfuegbar"]) - f(&p["arbeit"]["bedarf"]);
        let alle: Vec<&Value> = p["baubar"].as_array().into_iter().flatten().filter(|b| b["braucht"].as_array().is_none_or(|x| x.is_empty())).collect();
        let bezahlbar = |b: &&Value| b["fehlt"].as_array().is_none_or(|x| x.is_empty());
        let name = |b: &Value| b["gebaeude"].as_str().unwrap_or("").to_string();
        // Kraftwerk mit dem meisten Strom je Rohstoffeinheit.
        let kraftwerk = || alle.iter().filter(|b| ist_strom(b)).max_by(|a, b| {
            let wert = |x: &Value| f(&x["ertrag"]["plus"]) / x["kosten"].as_object().map(|o| o.values().map(f).sum::<f64>()).unwrap_or(1.0).max(1.0);
            wert(a).partial_cmp(&wert(b)).unwrap_or(std::cmp::Ordering::Equal)
        }).copied();
        let gebaeude = |g: &str| alle.iter().find(|b| b["gebaeude"] == g).copied();
        let strom_reicht = |b: &Value| ist_strom(b) || saldo - f(&b["strom_plus"]) >= 0.0;
        let knapp_voll = ["erz", "kristall", "deuterium"].iter().any(|g| p["voll_in_stunden"][*g].as_f64().is_some_and(|h| h < 3.0));
        let (wahl, grund): (Option<&Value>, String) = if saldo < 0.0 {
            (kraftwerk(), format!("Energie fehlt ({saldo:.0})"))
        } else if (p["nahrung_deckung"].is_number() && f(&p["nahrung_deckung"]) < 100.0) || f(&p["rate"]["nahrung"]) < 0.0 {
            (gebaeude("farm").filter(|b| strom_reicht(b)).or_else(kraftwerk), "Nahrung knapp".into())
        } else if knapp_voll {
            (gebaeude("lager"), "Lager bald voll".into())
        } else if arbeit_frei < 10.0 {
            (gebaeude("wohnblock").filter(|b| strom_reicht(b)).or_else(kraftwerk), "Arbeitskräfte knapp".into())
        } else {
            let mut beste: Option<(&Value, usize)> = None;
            for (i, g) in ordnung.iter().enumerate() {
                let Some(b) = gebaeude(g) else { continue };
                if !strom_reicht(b) || f(&b["arbeiter_plus"]) > arbeit_frei { continue; }
                let besser = match beste { None => true, Some((x, j)) => (f(&b["stufe"]), i) < (f(&x["stufe"]), j) };
                if besser { beste = Some((b, i)); }
            }
            match beste {
                Some((b, _)) => (Some(b), "gleichmäßiger Ausbau nach deiner Reihenfolge".into()),
                None => (kraftwerk(), "Strom reicht für den nächsten Ausbau nicht".into()),
            }
        };
        match wahl {
            Some(b) if bezahlbar(&b) => auftraege.push(Auftrag { aktion: json!({"typ":"bauen","planet":k,"gebaeude":name(b)}), grund: format!("{k}: {} auf Stufe {} – {grund}", name(b), b["stufe"]) }),
            Some(b) => uebersprungen.push(format!("{k}: wartet auf Rohstoffe für {} {} (fehlt {})", name(b), b["stufe"], b["fehlt"].as_array().map(|x| x.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_default())),
            None => uebersprungen.push(format!("{k}: nichts Passendes baubar ({grund})")),
        }
    }
    (auftraege, uebersprungen)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn planet(erzeugung: i64, verbrauch: i64, schleife: Value) -> Value {
        json!({"koord":"1:2:3","heimat":true,"energie":{"erzeugung":erzeugung,"verbrauch":verbrauch},"nahrung_deckung":100,"konsum_deckung":100,
            "bestand":{"erz":2000,"kristall":1200,"deuterium":400,"nahrung":800},"lager":{"erz":12800,"kristall":12800,"deuterium":12800,"nahrung":12800},
            "rate":{"erz":24,"nahrung":5},"voll_in_stunden":{"erz":446},"arbeit":{"verfuegbar":600,"bedarf":81},"fachkraefte":{"verfuegbar":30,"bedarf":0},
            "stabilitaet":50,"felder":{"belegt":6,"gesamt":180},"bauschleife":schleife,"bauschleife_plaetze":5,"blockade":null,
            "baubar":[
                {"gebaeude":"erzmine","stufe":2,"kosten":{"erz":90,"kristall":22},"fehlt":[],"braucht":[],"ertrag":{"plus":39,"art":"erz je Stunde"},"strom_plus":13,"arbeiter_plus":33},
                {"gebaeude":"kristallmine","stufe":3,"kosten":{"erz":72,"kristall":36},"fehlt":[],"braucht":[],"ertrag":{"plus":26,"art":"kristall je Stunde"},"strom_plus":13,"arbeiter_plus":33},
                {"gebaeude":"solarkraftwerk","stufe":2,"kosten":{"erz":112,"kristall":45},"fehlt":[],"braucht":[],"ertrag":{"plus":29,"art":"Strom"},"strom_plus":0,"arbeiter_plus":5},
                {"gebaeude":"werft","stufe":1,"kosten":{"erz":400},"fehlt":[],"braucht":["raumhafen 1"],"ertrag":null,"strom_plus":16,"arbeiter_plus":22},
                {"gebaeude":"lager","stufe":2,"kosten":{"erz":99999},"fehlt":["erz"],"braucht":[],"ertrag":{"plus":7680,"art":"Lagerplatz je Rohstoff"},"strom_plus":0,"arbeiter_plus":2}]})
    }
    #[test]
    fn auto_ausbau_folgt_den_regeln() {
        // Energiedefizit: zuerst ein Kraftwerk.
        let l = json!({"lage":{"planeten":[planet(24, 29, json!([]))]}});
        let (a, _) = ausbau_plan(&l, &[], 3);
        assert_eq!(a[0].aktion, json!({"typ":"bauen","planet":"1:2:3","gebaeude":"solarkraftwerk"}));
        // Genug Strom: niedrigste Stufe aus der Reihenfolge, Werft (fehlende Voraussetzung) nie.
        let l = json!({"lage":{"planeten":[planet(100, 29, json!([]))]}});
        let (a, _) = ausbau_plan(&l, &["kristallmine".into(), "erzmine".into(), "werft".into()], 3);
        assert_eq!(a[0].aktion["gebaeude"], "erzmine", "Stufe 2 vor Stufe 3");
        // Belegte Schleife: nichts tun. Lager unbezahlbar: übersprungen mit Grund.
        let l = json!({"lage":{"planeten":[planet(100, 29, json!([{"gebaeude":"farm"}]))]}});
        assert!(ausbau_plan(&l, &[], 3).0.is_empty());
        let mut p = planet(100, 29, json!([]));
        p["voll_in_stunden"]["erz"] = json!(1);
        let (a, ueber) = ausbau_plan(&json!({"lage":{"planeten":[p]}}), &[], 3);
        assert!(a.is_empty() && ueber[0].contains("wartet auf Rohstoffe für lager"));
        // Verbraucher nur, wenn der Strom danach reicht: Saldo 10, Mine braucht 13 → Kraftwerk.
        let l = json!({"lage":{"planeten":[planet(39, 29, json!([]))]}});
        assert_eq!(ausbau_plan(&l, &[], 3).0[0].aktion["gebaeude"], "solarkraftwerk");
    }
    #[test]
    fn inventur_und_analyse() {
        let mut p = planet(24, 29, json!([]));
        p["vorrat_reicht_sekunden"] = json!({"konsumgut": 1800});
        let l = json!({"lage":{"name":"Testreich","planeten":[p],"angriffe":[{"ziel":"1:2:3","ankunft":"20:00"}],"forschung":{"aktiv":null,"moeglich":[{"fehlt":[]}]},"warnungen":["x"]}});
        let i = inventur(&l);
        assert_eq!(i["summe"]["planeten"], 1);
        assert_eq!(i["planeten"][0]["energie"]["saldo"], -5.0);
        assert_eq!(i["planeten"][0]["baubar_jetzt"].as_array().unwrap().len(), 3, "nur bezahlbar und ohne fehlende Voraussetzung");
        let a = analyse(&l, &[]);
        let themen: Vec<&str> = a["befunde"].as_array().unwrap().iter().filter_map(|b| b["thema"].as_str()).collect();
        for t in ["Angriff", "Energie", "Vorrat", "Bauschleife", "Forschung"] { assert!(themen.contains(&t), "{t} fehlt in {themen:?}"); }
        assert_eq!(a["befunde"][0]["stufe"], 2, "dringende Befunde zuerst");
    }
}
