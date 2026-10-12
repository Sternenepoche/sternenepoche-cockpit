//! Referenz aus dem Regelwerk: `sternenepoche doku` schreibt docs/REGELWERK.md und docs/REGELTEXT.md.
//!
//! Alle Zahlen kommen aus `regeln/regelwerk.ron`, nichts ist von Hand abgeschrieben. Der Test
//! `doku_passt_zum_regelwerk` schlägt an, sobald das Regelwerk sich ändert und die Dateien nicht neu
//! erzeugt wurden.

use kern::aktion::erlaubte_typen;
use kern::regeltext::regeltext;
use kern::{Regelwerk, Rolle};
use serde_json::Value;
use std::fmt::Write;

const KOPF: &str = "<!-- Erzeugt mit `sternenepoche doku` aus regeln/regelwerk.ron. Nicht von Hand ändern: \
der Test doku_passt_zum_regelwerk vergleicht diese Datei mit dem Regelwerk. -->";

/// Abschnitte in fester Reihenfolge, je mit einem Satz, was sie regeln.
const ABSCHNITTE: &[(&str, &str)] = &[
    ("welt", "Größe der Galaxie, Spielerzahl, Dauer der Epoche, Zeittakt."),
    ("zonen", "Planetenzonen mit Feldern und Ertragsfaktoren."),
    ("voelker", "Die vier Völker mit ihren Abweichungen vom Grundwert (1.0 = keine Abweichung)."),
    ("wirtschaft", "Produktion, Lager, Bevölkerung, Steuern, Bauschleifen."),
    ("stabilitaet", "Stabilität der Planeten: Ziele, Einflüsse, Unruhen."),
    ("gebaeude", "Gebäude: Kosten der ersten Stufe, Kostenfaktor je Stufe, Bedarf, Ertrag, Freischaltung."),
    ("forschung", "Forschungen: Kosten, Laborstufe, Forschungspunkte, Freischaltung."),
    ("einheiten", "Schiffe und Verteidigungsanlagen: Kosten, Kampfwerte, Ladung, Tempo, Schnellfeuer."),
    ("stufen", "Zivilisationsstufen II bis V: Bedingungen, Kosten, Freischaltungen."),
    ("flug", "Flugzeiten, Treibstoff, Haltedauern."),
    ("kampf", "Kampfrunden, Beute, Trümmer, Eroberung."),
    ("diplomatie", "Verträge, Kautionen, Nachrichten, Allianzen."),
    ("markt", "Marktgebühren und Lieferzeiten."),
    ("wertung", "Punkte: Werteinheiten, Gewichte, Stufenbonus."),
    ("agenten", "Rollen der Modelle: Takt, früheste Aufrufe, Grenzen je Aufruf."),
    ("zusatz", "Raketen und Großprojekte der Stufe V."),
];

fn zahl(v: &Value) -> String {
    match v {
        Value::Null => "–".into(),
        Value::Bool(b) => if *b { "ja" } else { "nein" }.into(),
        Value::String(s) => s.replace('|', "/"),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(zahl).collect::<Vec<_>>().join(", "),
        Value::Object(m) => m
            .iter()
            .filter(|(_, w)| !w.is_null() && *w != &Value::Bool(false))
            .map(|(k, w)| match w {
                Value::Object(_) | Value::Array(_) => format!("{k}: ({})", zahl(w)),
                _ => format!("{k} {}", zahl(w)),
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Eine Tabelle aus gleichartigen Einträgen: je Eintrag eine Zeile, je Feld eine Spalte.
fn tabelle(aus: &mut String, zeilen: &[(String, &serde_json::Map<String, Value>)]) {
    let mut spalten: Vec<&String> = Vec::new();
    for (_, m) in zeilen {
        for k in m.keys() {
            if !spalten.contains(&k) {
                spalten.push(k);
            }
        }
    }
    let _ = writeln!(aus, "| | {} |", spalten.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" | "));
    let _ = writeln!(aus, "|---|{}", "---|".repeat(spalten.len()));
    for (name, m) in zeilen {
        let zellen: Vec<String> = spalten.iter().map(|k| m.get(k.as_str()).map(zahl).unwrap_or_default()).collect();
        let _ = writeln!(aus, "| **{name}** | {} |", zellen.join(" | "));
    }
    aus.push('\n');
}

/// Einzelwerte eines Abschnitts als Liste; verschachtelte Werte stehen kompakt in einer Zeile.
fn werte(aus: &mut String, m: &serde_json::Map<String, Value>) {
    let _ = writeln!(aus, "| Wert | Einstellung |\n|---|---|");
    for (k, v) in m {
        let _ = writeln!(aus, "| `{k}` | {} |", zahl(v));
    }
    aus.push('\n');
}

pub fn regelwerk_md(r: &Regelwerk) -> String {
    let mut j = serde_json::to_value(r).expect("Regelwerk serialisierbar");
    let m = j.as_object_mut().unwrap();
    m.remove("cache");
    let mut aus = String::new();
    let _ = writeln!(aus, "{KOPF}\n");
    let _ = writeln!(aus, "# Regelwerk – Referenz\n");
    let _ = writeln!(
        aus,
        "Alle Zahlen des Spiels, Version {} (SHA-256 `{}`). Die Begründungen der Werte stehen als Kommentare in \
`regeln/regelwerk.ron`, die Mechanik in `docs/SPEZIFIKATION.md`. Mengen sind Einheiten des Guts, Zeiten Sekunden, \
wo nichts anderes steht; Faktoren 1.0 bedeuten keine Abweichung.\n",
        r.version, r.hash
    );
    let _ = writeln!(aus, "## Inhalt\n");
    for (name, _) in ABSCHNITTE {
        let _ = writeln!(aus, "- [{name}](#{name})");
    }
    let _ = writeln!(aus, "- [aktionen](#aktionen)\n");
    for (name, satz) in ABSCHNITTE {
        let Some(v) = m.get(*name) else { continue };
        let _ = writeln!(aus, "## {name}\n\n{satz}\n");
        match v {
            Value::Object(o) if o.values().all(Value::is_object) && !o.is_empty() => {
                let zeilen: Vec<(String, &serde_json::Map<String, Value>)> =
                    o.iter().map(|(k, w)| (k.clone(), w.as_object().unwrap())).collect();
                tabelle(&mut aus, &zeilen);
            }
            Value::Object(o) => werte(&mut aus, o),
            Value::Array(a) => {
                let zeilen: Vec<(String, &serde_json::Map<String, Value>)> = a
                    .iter()
                    .enumerate()
                    .filter_map(|(i, w)| w.as_object().map(|o| (format!("{}", i + 2), o)))
                    .collect();
                tabelle(&mut aus, &zeilen);
            }
            w => {
                let _ = writeln!(aus, "{}\n", zahl(w));
            }
        }
        if *name == "stufen" {
            let _ = writeln!(
                aus,
                "Eine 0 oder ein leeres Feld heißt: keine Bedingung dieser Art. Haltezeit: alle Bedingungen müssen {} Stunden \
ununterbrochen erfüllt sein, dann gilt der Aufstieg (Aktion `stufenaufstieg`, die Kosten werden dabei abgebucht).\n",
                r.stufen_haltezeit_stunden
            );
        }
    }
    let _ = writeln!(aus, "## aktionen\n\nWelche Rolle welche Aktion senden darf (Antwortschema der Engine, `kern::aktion`). \
Felder und Prüfungen jeder Aktion: `docs/AGENTEN-SCHNITTSTELLE.md`.\n");
    let _ = writeln!(aus, "| Rolle | Aktionen |\n|---|---|");
    for rolle in [Rolle::Stratege, Rolle::Verwalter, Rolle::Feldherr, Rolle::Diplomat, Rolle::Alle] {
        let mut typen = erlaubte_typen(rolle);
        typen.sort_unstable();
        typen.dedup();
        let _ = writeln!(aus, "| {} | {} |", rolle.name(), typen.iter().map(|t| format!("`{t}`")).collect::<Vec<_>>().join(", "));
    }
    aus
}

/// Der Regeltext, den die Modelle bekommen, je Rolle als Abschnitt.
pub fn regeltext_md(r: &Regelwerk) -> String {
    let mut aus = String::new();
    let _ = writeln!(aus, "{KOPF}\n");
    let _ = writeln!(aus, "# Regeltext der Rollen\n");
    let _ = writeln!(
        aus,
        "So steht das Spiel im Systemtext der Sprachmodelle, erzeugt aus dem Regelwerk ({}, `{}`). Jede Rolle \
bekommt die Abschnitte, die sie für ihre Entscheidungen braucht; „alle“ ist die vollständige Fassung für den \
menschlichen Spieler und für Prüfungen. Der Orchestrator stellt Antwortformat und Grenzen davor \
(`orchestrator/sternenepoche/prompt.py`, `crates/agenten/src/protocol.rs`).\n",
        r.version,
        &r.hash[..12]
    );
    for rolle in [Rolle::Alle, Rolle::Stratege, Rolle::Verwalter, Rolle::Feldherr, Rolle::Diplomat] {
        let _ = writeln!(aus, "## {}\n\n```text\n{}\n```\n", rolle.name(), regeltext(r, rolle).trim_end());
    }
    aus
}
