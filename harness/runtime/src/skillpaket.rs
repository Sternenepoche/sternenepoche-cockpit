//! Skill-Paket des Servers (`GET /api/harness/v1/skills`): Liste und Texte, zwischengespeichert
//! im Harness-Ordner (`skills-server/`), damit der Harness auch bei kurzem Serverausfall dieselben
//! Skills nutzt. Je Rolle wird eine feste Auswahl innerhalb eines Zeichenbudgets in den Prompt
//! gelegt; kein Skill fällt dabei ganz weg, zu lange werden an Abschnittsgrenzen gekürzt.
//! Fehlt das Paket, gelten die eingebetteten Skills (skills/*.md) wie bisher.
use serde_json::{json, Value};
use std::path::Path;

pub const ORDNER: &str = "skills-server";
pub const PRAEFIX: &str = "sternenepoche-";

#[derive(Clone, Debug, PartialEq)]
pub struct Skill { pub name: String, pub beschreibung: String, pub text: String, pub hash: String }

/// Skills je Rolle (Kurzname ohne Präfix), in Reihenfolge der Wichtigkeit.
pub fn fuer_rolle(rolle: &str) -> &'static [&'static str] {
    match rolle {
        "stratege" => &["grundlagen", "rat", "wirtschaft", "diplomatie", "fehler"],
        "verwalter" => &["grundlagen", "rat", "wirtschaft", "handel", "monde-dyson", "fehler"],
        "feldherr" => &["grundlagen", "rat", "flotte", "kampf", "monde-dyson", "fehler"],
        "diplomat" => &["grundlagen", "rat", "diplomatie", "handel", "fehler"],
        _ => &["grundlagen", "rat", "fehler"],
    }
}

fn kennname(s: &str) -> bool { !s.is_empty() && s.len() <= 60 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_') }

/// Namen aus der Liste des Servers (`{"skills":[{name,beschreibung,hash?}]}` oder Liste).
pub fn liste_lesen(v: &Value) -> Vec<(String, String, String)> {
    let a = if v.is_array() { v } else { &v["skills"] };
    a.as_array().into_iter().flatten().filter_map(|s| {
        let name = s["name"].as_str().or(s.as_str())?.trim().to_string();
        kennname(&name).then(|| (name, s["beschreibung"].as_str().unwrap_or("").chars().take(400).collect(), s["hash"].as_str().or(s["version"].as_str()).unwrap_or("").chars().take(80).collect()))
    }).collect()
}

/// Text eines Skills aus `{"name","text"}` (oder `inhalt`/`markdown`), höchstens 40 000 Zeichen.
pub fn text_lesen(v: &Value) -> Option<String> {
    let t = v["text"].as_str().or(v["inhalt"].as_str()).or(v["markdown"].as_str()).or(v.as_str())?;
    let t: String = t.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').take(40_000).collect();
    (!t.trim().is_empty()).then_some(t)
}

pub fn cache_laden(basis: &Path) -> Vec<Skill> {
    let o = basis.join(ORDNER);
    let Ok(index) = std::fs::read_to_string(o.join("index.json")) else { return vec![] };
    let Ok(v) = serde_json::from_str::<Value>(&index) else { return vec![] };
    v["skills"].as_array().into_iter().flatten().filter_map(|s| {
        let name = s["name"].as_str().filter(|n| kennname(n))?.to_string();
        let text = std::fs::read_to_string(o.join(format!("{name}.md"))).ok()?;
        Some(Skill { beschreibung: s["beschreibung"].as_str().unwrap_or("").into(), hash: s["hash"].as_str().unwrap_or("").into(), text, name })
    }).collect()
}

pub fn cache_speichern(basis: &Path, skills: &[Skill]) -> Result<(), String> {
    let o = basis.join(ORDNER);
    std::fs::create_dir_all(&o).map_err(|e| format!("{}: {e}", o.display()))?;
    for s in skills {
        let p = o.join(format!("{}.md", s.name));
        if std::fs::read_to_string(&p).ok().as_deref() != Some(s.text.as_str()) { std::fs::write(&p, &s.text).map_err(|e| format!("{}: {e}", p.display()))?; }
    }
    let index = json!({"skills":skills.iter().map(|s| json!({"name":s.name,"beschreibung":s.beschreibung,"hash":s.hash})).collect::<Vec<_>>()});
    std::fs::write(o.join("index.json"), serde_json::to_string_pretty(&index).unwrap()).map_err(|e| e.to_string())
}

/// Kürzt einen Markdown-Text auf höchstens `n` Zeichen an Abschnitts- oder Absatzgrenzen.
pub fn kuerzen_an_grenzen(t: &str, n: usize) -> (String, bool) {
    if t.chars().count() <= n { return (t.to_string(), false); }
    let mut out = String::new();
    let mut zaehler = 0usize;
    // Blöcke: Abschnitte (## …) und darin Absätze; nur ganze Blöcke übernehmen.
    for block in t.split("\n\n") {
        let laenge = block.chars().count() + 2;
        if zaehler + laenge > n.saturating_sub(40) { break; }
        out.push_str(block);
        out.push_str("\n\n");
        zaehler += laenge;
    }
    if out.trim().is_empty() {
        // Ein einziger, sehr langer Absatz: an der letzten Zeilengrenze im Budget schneiden.
        let roh: String = t.chars().take(n.saturating_sub(40)).collect();
        out = roh.rsplit_once('\n').map(|(a, _)| a.to_string()).unwrap_or(roh);
    }
    (format!("{}\n[… gekürzt: vollständig mit {{typ:\"regel\"}} oder im Skill-Paket nachlesen]", out.trim_end()), true)
}

/// Auswahl für eine Rolle innerhalb von `budget` Zeichen: jeder gewählte Skill bekommt mindestens
/// seinen fairen Anteil; was kürzere nicht brauchen, geht an längere (Wasserstand-Verteilung).
/// Ergebnis: (Name, Text) und die Namen der gekürzten Skills.
pub fn auswahl(paket: &[Skill], rolle: &str, budget: usize) -> (Vec<(String, String)>, Vec<String>) {
    let gewaehlt: Vec<&Skill> = fuer_rolle(rolle).iter().filter_map(|k| {
        paket.iter().find(|s| s.name == format!("{PRAEFIX}{k}")).or_else(|| paket.iter().find(|s| s.name == *k))
    }).collect();
    let laengen: Vec<usize> = gewaehlt.iter().map(|s| s.text.chars().count()).collect();
    let mut anteil = vec![0usize; gewaehlt.len()];
    let mut rest = budget;
    let mut offen: Vec<usize> = (0..gewaehlt.len()).collect();
    offen.sort_by_key(|i| laengen[*i]);
    while let Some(&i) = offen.first() {
        let fair = rest / offen.len();
        let a = laengen[i].min(fair);
        anteil[i] = a;
        rest -= a;
        offen.remove(0);
    }
    let mut gekuerzt = Vec::new();
    let texte = gewaehlt.iter().zip(anteil).map(|(s, a)| {
        let (t, k) = kuerzen_an_grenzen(&s.text, a.max(400));
        if k { gekuerzt.push(s.name.clone()); }
        (s.name.clone(), t)
    }).collect();
    (texte, gekuerzt)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn s(name: &str, n: usize) -> Skill {
        let abschnitt = format!("## Abschnitt\n{}\n\n", "Satz. ".repeat(20));
        let mut t = format!("# {name}\n\n");
        while t.chars().count() < n { t.push_str(&abschnitt); }
        Skill { name: format!("{PRAEFIX}{name}"), beschreibung: String::new(), text: t, hash: String::new() }
    }
    #[test]
    fn auswahl_je_rolle_im_budget() {
        let paket: Vec<Skill> = ["grundlagen", "rat", "wirtschaft", "flotte", "kampf", "diplomatie", "handel", "monde-dyson", "fehler"].iter().map(|n| s(n, if *n == "kampf" { 9000 } else { 1000 })).collect();
        let (texte, gekuerzt) = auswahl(&paket, "feldherr", 9000);
        let namen: Vec<&str> = texte.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(namen, vec!["sternenepoche-grundlagen", "sternenepoche-rat", "sternenepoche-flotte", "sternenepoche-kampf", "sternenepoche-monde-dyson", "sternenepoche-fehler"]);
        let summe: usize = texte.iter().map(|(_, t)| t.chars().count()).sum();
        assert!(summe <= 9000 + 200, "Budget eingehalten: {summe}");
        assert_eq!(gekuerzt, vec!["sternenepoche-kampf".to_string()], "nur der lange Skill wird gekürzt");
        assert!(texte[3].1.contains("gekürzt") && texte[3].1.starts_with("# kampf"));
        // Diplomat bekommt seine eigene Auswahl, fehler ist überall dabei.
        let (d, _) = auswahl(&paket, "diplomat", 50_000);
        assert_eq!(d.iter().map(|(n, _)| n.trim_start_matches(PRAEFIX)).collect::<Vec<_>>(), vec!["grundlagen", "rat", "diplomatie", "handel", "fehler"]);
        assert_eq!(liste_lesen(&json!({"skills":[{"name":"sternenepoche-rat","beschreibung":"Rat"},{"name":"../böse"}]})).len(), 1);
    }
    #[test]
    fn cache_schreibt_und_liest() {
        let o = std::env::temp_dir().join(format!("skillcache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let paket = vec![s("grundlagen", 500), s("rat", 300)];
        cache_speichern(&o, &paket).unwrap();
        assert_eq!(cache_laden(&o), paket);
        let _ = std::fs::remove_dir_all(&o);
    }
}
