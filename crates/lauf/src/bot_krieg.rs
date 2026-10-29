//! Krieg und Verteidigung der Skriptbots über Allianzgrenzen hinweg: Kriegserklärung und Frieden zwischen Allianzen,
//! gemeinsame Gegenschläge, Entsatz und Bombardierung (Karl 20.10.2026: „von Handel bis Krieg, von Spieler zu Spieler
//! bis Allianzen gegen Allianzen“). Alles über dieselben Aktionen wie Spieler und Agenten.
//!
//! Der Kriegszustand steht im Kern (Anfrage „krieg“ im Status „erklaert“, Reich gegen Reich über `diplomatie_anfrage`,
//! Allianz gegen Allianz über `allianz_anfrage`); die Bots lesen ihn von dort und verkünden ihn im Allianzchat.
//! Ein Krieg ist für die Bots *aktiv*, solange innerhalb der Ruhezeit gekämpft wurde und er nicht länger als
//! `MAX_DAUER` läuft (Räuber lehnen Frieden ab, der Krieg bliebe sonst im Kern für immer „erklaert“). Danach bietet
//! die Leitung Frieden an (Bots nehmen ihn an), und eine Sperre verhindert die sofortige neue Erklärung.
//!
//! Wer führt was:
//! * Die Allianzleitung (erstes aktives Mitglied, bei Reichen ohne Allianz das Reich selbst) zählt die Angriffe auf
//!   ihre Mitglieder aus den Kampfberichten, erklärt nach wiederholten Angriffen desselben Reichs oder derselben
//!   Allianz den Krieg, verkündet ihn im Chat, nennt Ziele und bietet nach der Ruhezeit Frieden an. Dazu kommt
//!   die Konkurrenz um das Sammelsystem der Dysonsphäre: hält eine andere Allianz dort mehrere Welten, kann es Krieg
//!   zwischen den beiden großen Allianzen geben.
//! * Ökonomen und Händler im Krieg rüsten über dem Rücklagebestand, klären die Ziele des Gegners auf, schlagen
//!   zurück, wenn der Simulator den Sieg verspricht, und bombardieren ab Stufe V Kolonien des Gegners.
//! * Igel verteidigen und entsetzen (Hilferuf), Räuber bleiben außen vor (ihre Eroberungen laufen in
//!   `bot_invasion.rs`), Wehrlose tun nichts.
//!
//! Opferschutz: Neulinge werden nie Ziel; ein Ort, an dem in der Raubpause gekämpft wurde, ebenso wenig; je Gegenseite
//! höchstens drei Gegenschläge in 24 Stunden.

use super::invasion_bot::{self, verteidigungswert, Modus};
use super::*;

/// Angriffe auf Mitglieder zählen innerhalb dieses Fensters.
const FENSTER: i64 = 3 * TAG;
/// So lange ohne Kampf zwischen den Parteien ist ein Krieg für die Bots vorbei.
const RUHEZEIT: i64 = 36 * STUNDE;
/// Längste Dauer eines Kriegs für die Bots (danach Frieden, ob angenommen oder nicht).
const MAX_DAUER: i64 = 6 * TAG;
/// Zwischen zwei Kriegserklärungen gegen dieselbe Partei vergehen mindestens so viele Tage.
const SPERRE: i64 = 10 * TAG;
/// Frische eines Berichts, auf dem ein Gegenschlag aufbaut.
const BERICHT_ALTER: i64 = 8 * STUNDE;
/// Mehr Gegenschläge gegen dieselbe Partei in 24 Stunden gibt es nicht.
const SCHLAEGE_JE_TAG: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Gegner {
    Reich(SpielerId),
    Allianz(u32),
}

#[derive(Clone, Copy, Debug)]
struct Krieg {
    gegner: Gegner,
    /// Zeit der Erklärung.
    seit: SimZeit,
    /// Hat unsere Seite den Krieg erklärt (sonst die Gegenseite)?
    von_uns: bool,
}

/// Deterministische Streuung je Spieler, Gegenstand und Tag (0 bis 999).
fn wurf(a: SpielerId, b: u64, tag: i64) -> u64 {
    let mut x = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ (tag as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    (x ^ (x >> 29)) % 1000
}
fn schluessel(g: Gegner) -> u64 {
    match g { Gegner::Reich(s) => s as u64, Gegner::Allianz(a) => 1000 + a as u64 }
}

fn aktive_mitglieder(w: &Welt, aid: u32) -> Vec<SpielerId> {
    w.allianzen.iter().find(|a| a.id == aid).map(|a| a.mitglieder.iter().copied().filter(|m| w.spieler_aktiv(*m)).collect()).unwrap_or_default()
}
/// Die eigene Seite: alle aktiven Mitglieder der Allianz, sonst das Reich allein.
fn wir(w: &Welt, sid: SpielerId) -> Vec<SpielerId> {
    match w.spieler[sid as usize].allianz.map(|aid| aktive_mitglieder(w, aid)) {
        Some(m) if !m.is_empty() => m,
        _ => vec![sid],
    }
}
fn spieler_von(w: &Welt, g: Gegner) -> Vec<SpielerId> {
    match g {
        Gegner::Reich(s) => if w.spieler_aktiv(s) { vec![s] } else { Vec::new() },
        Gegner::Allianz(a) => aktive_mitglieder(w, a),
    }
}
fn name_von(w: &Welt, g: Gegner) -> String {
    match g {
        Gegner::Reich(s) => w.spieler[s as usize].name.clone(),
        Gegner::Allianz(a) => w.allianzen.iter().find(|x| x.id == a).map(|x| x.name.clone()).unwrap_or_default(),
    }
}
/// Der Sprecher: erstes aktives Mitglied der Allianz; ein Reich ohne Allianz spricht für sich.
fn sprecher(w: &Welt, sid: SpielerId) -> bool {
    match w.spieler[sid as usize].allianz {
        None => true,
        Some(aid) => aktive_mitglieder(w, aid).first() == Some(&sid),
    }
}

/// Die Kriege, an denen `sid` beteiligt ist (neueste Erklärung je Gegner, höchstens `MAX_DAUER` alt). Ein Krieg Reich gegen
/// Reich betrifft alle Mitglieder der Allianz der Beteiligten: sie folgen ihrem Mitglied.
fn kriege(w: &Welt, sid: SpielerId) -> Vec<Krieg> {
    let aid = w.spieler[sid as usize].allianz;
    let meine = |s: SpielerId| s == sid || (aid.is_some() && w.spieler[s as usize].allianz == aid);
    let mut aus: BTreeMap<Gegner, Krieg> = BTreeMap::new();
    for a in w.kommunikation.anfragen.iter().rev().take_while(|a| w.zeit - a.zeit < MAX_DAUER) {
        if a.art != "krieg" || a.status != "erklaert" { continue; }
        let (gegner, von_uns) = match a.zielallianz {
            Some(z) => match a.allianz {
                Some(x) if aid == Some(x) => (Gegner::Allianz(z), true),
                Some(x) if aid == Some(z) => (Gegner::Allianz(x), false),
                _ => continue,
            },
            None if meine(a.von) && !meine(a.an) => (Gegner::Reich(a.an), true),
            None if meine(a.an) && !meine(a.von) => (Gegner::Reich(a.von), false),
            None => continue,
        };
        aus.entry(gegner).or_insert(Krieg { gegner, seit: a.zeit, von_uns });
    }
    aus.into_values().collect()
}

/// Zeit des letzten Kampfs zwischen den Seiten seit `seit` (sonst `seit`).
fn letzter_kampf(w: &Welt, a: &[SpielerId], b: &[SpielerId], seit: SimZeit) -> SimZeit {
    w.kampfberichte.iter().rev().take_while(|k| k.zeit >= seit)
        .find(|k| (a.contains(&k.angreifer) && k.verteidiger.iter().any(|v| b.contains(v))) || (b.contains(&k.angreifer) && k.verteidiger.iter().any(|v| a.contains(v))))
        .map(|k| k.zeit).unwrap_or(seit)
}

/// Aktive Kriege samt den Spielern der Gegenseite.
fn aktive_kriege(w: &Welt, sid: SpielerId) -> Vec<(Krieg, Vec<SpielerId>)> {
    let ich = wir(w, sid);
    kriege(w, sid).into_iter().filter_map(|k| {
        let gegner = spieler_von(w, k.gegner);
        if gegner.is_empty() { return None; }
        let letzte = letzter_kampf(w, &ich, &gegner, k.seit);
        (w.zeit - letzte < RUHEZEIT && w.zeit - k.seit < MAX_DAUER).then_some((k, gegner))
    }).collect()
}

// ---------------------------------------------------------------------------------------------- Chat

fn chat(w: &mut Welt, sid: SpielerId, betreff: &str, text: String) -> bool {
    w.spieler[sid as usize].allianz.is_some()
        && tu(w, sid, json!({"typ": "brief_senden", "kanal": "allianz", "an": "", "betreff": betreff, "text": text, "antwort_auf": null}))
}
/// Allianzchat-Briefe mit diesem Betreff seit `seit`, neueste zuerst: (Absender, Text).
fn chat_lesen(w: &Welt, aid: u32, betreff: &str, seit: SimZeit) -> Vec<(SpielerId, String)> {
    w.kommunikation.briefe.iter().rev()
        .take_while(|(id, _)| w.nachrichten[**id as usize - 1].zeit >= seit)
        .filter(|(_, b)| b.kanal == "allianz" && b.betreff == betreff && b.allianzen.contains(&aid))
        .map(|(id, _)| { let n = &w.nachrichten[*id as usize - 1]; (n.von, n.text.clone()) })
        .collect()
}
fn koordinaten(text: &str) -> Vec<Koord> {
    text.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '(' | ')' | '.' | '!')).filter(|t| t.matches(':').count() == 2).filter_map(|t| t.parse().ok()).collect()
}
/// Die im Allianzchat zuletzt genannten Gegenschlagziele (höchstens einen Tag alt).
fn chat_ziele(w: &Welt, sid: SpielerId) -> Vec<Koord> {
    let Some(aid) = w.spieler[sid as usize].allianz else { return Vec::new() };
    chat_lesen(w, aid, "Gegenschlag", w.zeit - TAG).into_iter().flat_map(|(_, t)| koordinaten(&t)).collect()
}

// ---------------------------------------------------------------------------------------------- Erklärung

struct Bilanz { anzahl: usize, letzte: SimZeit, opfer: SpielerId, ort: Koord }

/// Angriffe auf die eigene Seite im Fenster, gezählt je Angreiferpartei (Allianz, sonst Reich).
fn angriffsbilanz(w: &Welt, ich: &[SpielerId]) -> BTreeMap<Gegner, Bilanz> {
    let mut aus: BTreeMap<Gegner, Bilanz> = BTreeMap::new();
    for k in w.kampfberichte.iter().rev().take_while(|k| w.zeit - k.zeit < FENSTER) {
        if ich.contains(&k.angreifer) || !k.mission.feindlich() { continue; }
        let Some(opfer) = k.verteidiger.iter().copied().find(|v| ich.contains(v)) else { continue };
        let partei = match w.spieler[k.angreifer as usize].allianz { Some(a) => Gegner::Allianz(a), None => Gegner::Reich(k.angreifer) };
        let e = aus.entry(partei).or_insert(Bilanz { anzahl: 0, letzte: k.zeit, opfer, ort: k.ort });
        e.anzahl += 1;
    }
    aus
}

/// Gab es zwischen den Seiten in den letzten `SPERRE` Tagen schon eine Erklärung (laufend oder beendet)?
fn gesperrt(w: &Welt, ich: &[SpielerId], g: Gegner, aid: Option<u32>) -> bool {
    let paar = |a: &kern::kommunikation::Anfrage| a.art == "krieg" && match (a.zielallianz, g) {
        (Some(z), Gegner::Allianz(b)) => aid.is_some() && ((a.allianz == aid && z == b) || (a.allianz == Some(b) && Some(z) == aid)),
        (None, Gegner::Reich(x)) => (ich.contains(&a.von) && a.an == x) || (ich.contains(&a.an) && a.von == x),
        _ => false,
    };
    // Ein Krieg, der im Kern noch besteht (Räuber nehmen keinen Frieden an), sperrt auch nach der Sperrfrist: sonst lehnte
    // der Kern die erneute Erklärung ab („Vereinbarung oder Angebot besteht bereits“, klassisch 365-mal, 20.10.2026).
    w.kommunikation.anfragen.iter().rev().take_while(|a| w.zeit - a.zeit < SPERRE)
        .any(|a| matches!(a.status.as_str(), "erklaert" | "beendet") && paar(a))
        || w.kommunikation.anfragen.iter().any(|a| matches!(a.status.as_str(), "erklaert" | "offen") && paar(a))
}

/// Die Leitung zählt die Angriffe und erklärt nach wiederholten Angriffen desselben Reichs oder derselben Allianz den Krieg.
/// Igel greifen früh zur Erklärung (zwei Angriffe), Ökonomen etwas überlegter (zwei, mit Streuung), Händler suchen erst
/// den Ausgleich (drei).
fn erklaeren(w: &mut Welt, sid: SpielerId, typ: Bottyp, schutz: &Opferschutz) {
    if !sprecher(w, sid) || w.kampfberichte.last().is_none_or(|k| w.zeit - k.zeit >= FENSTER) { return; }
    let ich = wir(w, sid);
    let aid = w.spieler[sid as usize].allianz;
    let schwelle = if typ == Bottyp::Haendler { 3 } else { 2 };
    let tag = w.zeit / TAG;
    let mut wahl: Option<(Gegner, Bilanz)> = None;
    for (g, b) in angriffsbilanz(w, &ich) {
        if b.anzahl < schwelle || w.zeit - b.letzte > RUHEZEIT || gesperrt(w, &ich, g, aid) { continue; }
        if let Gegner::Reich(x) = g {
            // Neulinge, Ausgeschiedene und Verbündete werden nicht zum Kriegsgegner.
            if neuling(w, schutz, x) || !w.spieler_aktiv(x) || w.verbuendet(sid, x) { continue; }
        }
        if typ != Bottyp::Igel && wurf(sid, schluessel(g), tag) >= 700 { zaehle("krieg: erklärung verschoben"); continue; }
        if wahl.as_ref().is_none_or(|(_, alt)| b.anzahl > alt.anzahl) { wahl = Some((g, b)); }
    }
    let Some((g, b)) = wahl else { return };
    let (n, name) = (b.anzahl, name_von(w, g));
    let opfer = w.spieler[b.opfer as usize].name.clone();
    let text = format!("Kriegserklärung: {n} Angriffe von {name} auf unsere Reiche in drei Tagen, zuletzt auf {opfer} bei {}. Wir antworten mit Gegenschlägen auf bekannte Ziele, wenn der Sieg sicher ist; Frieden nach {} Stunden Ruhe oder auf Angebot.",
        b.ort, RUHEZEIT / STUNDE);
    let ok = match g {
        Gegner::Reich(_) => tu(w, sid, json!({"typ": "diplomatie_anfrage", "partner": name, "art": "krieg", "text": text})),
        Gegner::Allianz(_) => tu(w, sid, json!({"typ": "allianz_anfrage", "allianz": name, "art": "krieg", "text": text})),
    };
    if !ok { zaehle("krieg: erklärung abgelehnt"); return; }
    zaehle(match g { Gegner::Reich(_) => "krieg: erklärt (reich)", Gegner::Allianz(_) => "krieg: erklärt (allianz)" });
    chat(w, sid, "Krieg", format!("Wir erklären {name} den Krieg: {n} Angriffe in drei Tagen, zuletzt auf {opfer}. Wer eine Kampfflotte hat, rüstet und schlägt auf bekannte Ziele zurück; die Leitung nennt Ziele im Chat (Betreff Gegenschlag). Igel sichern und entsetzen. Neulinge bleiben tabu."));
}

/// Konkurrenz um das Sammelsystem: hält eine andere Allianz dort mehrere Welten, erklärt die Leitung ihr (mit Streuung)
/// den Krieg – so kommt es auch zwischen den großen Allianzen zum Streit, ohne dass vorher ein Angriff fiel.
fn konkurrenz(w: &mut Welt, sid: SpielerId) {
    let Some(aid) = w.spieler[sid as usize].allianz else { return };
    if !sprecher(w, sid) || w.zeit % TAG >= TAKT || w.zeit < 5 * TAG { return; }
    let Some((s, y)) = allianz_bot::sammelsystem(w, aid) else { return };
    // Sichtbar nur, wenn ein Mitglied dort lebt (Allianzsystem).
    if !w.allianzsystem(sid, Koord::neu(s, y, 1)) { return; }
    let mut je: BTreeMap<u32, usize> = BTreeMap::new();
    let mut eigene = 0usize;
    for pos in 1..=w.regeln.welt.plaetze_je_system {
        let Some(pid) = w.belegung.get(&Koord::neu(s, y, pos)) else { continue };
        match w.spieler[w.planeten[*pid as usize].besitzer as usize].allianz {
            Some(a) if a == aid => eigene += 1,
            Some(a) => *je.entry(a).or_default() += 1,
            None => {}
        }
    }
    let ich = wir(w, sid);
    let tag = w.zeit / TAG;
    // Wer dort mindestens drei Welten hält und nicht weniger als wir, verdrängt uns; mit Streuung von etwa 25 % je Tag.
    let mut wahl: Option<(usize, u32)> = None;
    for (b, n) in je {
        let partner = w.allianzen.iter().find(|x| x.id == b).and_then(|x| x.mitglieder.first().copied());
        if n < 3 || n + 1 < eigene || gesperrt(w, &ich, Gegner::Allianz(b), Some(aid)) { continue; }
        if partner.is_some_and(|p| w.allianzvertrag_zwischen(sid, p, Vertragsart::Nichtangriffspakt) || w.allianzvertrag_zwischen(sid, p, Vertragsart::Verteidigungsbuendnis)) { continue; }
        if wurf(sid, 31 + b as u64, tag) >= 250 { continue; }
        if wahl.is_none_or(|(m, _)| n > m) { wahl = Some((n, b)); }
    }
    let Some((n, b)) = wahl else { return };
    let name = name_von(w, Gegner::Allianz(b));
    let text = format!("Kriegserklärung: Ihr haltet {n} Welten in unserem Sammelsystem {s}:{y}, wir {eigene}. Das System gehört dem, der es zuerst besiedelte und hält; wir bestehen darauf. Frieden nach {} Stunden Ruhe oder auf Angebot.", RUHEZEIT / STUNDE);
    if tu(w, sid, json!({"typ": "allianz_anfrage", "allianz": name, "art": "krieg", "text": text})) {
        zaehle("krieg: erklärt (sammelsystem)");
        chat(w, sid, "Krieg", format!("Wir erklären der Allianz {name} den Krieg um das Sammelsystem {s}:{y} ({n} ihrer Welten gegen {eigene} unserer). Ziele nennt die Leitung im Chat (Betreff Gegenschlag)."));
    }
}

/// Grenzreibung: In den Systemen, in denen Mitglieder wohnen, hält eine andere Allianz mehr Welten als wir – der Druck ist die
/// Summe der Mehrheit in diesen Systemen. Ab zwei Welten Druck erklärt die Leitung (mit Streuung, je Tag bei Ökonomen 40 %,
/// Igeln 25 %, Händlern 15 %) der drängenden Allianz den Krieg. Messlauf Startwert 712: zwei Allianzen teilten vier Systeme
/// (1:12 mit drei Welten der einen und einer der anderen, 2:13 umgekehrt), ohne dass eine ein fremdes Sammelsystem erreichte.
fn reibung(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
    let Some(aid) = w.spieler[sid as usize].allianz else { return };
    if !sprecher(w, sid) || w.zeit % TAG >= TAKT || w.zeit < 12 * TAG { return; }
    let systeme: std::collections::BTreeSet<(u8, u8)> = aktive_mitglieder(w, aid).iter().flat_map(|m| w.spieler[*m as usize].planeten.iter())
        .map(|p| { let k = w.planeten[*p as usize].koord; (k.sektor, k.system) }).collect();
    let mut druck: BTreeMap<u32, usize> = BTreeMap::new();
    for (s, y) in systeme {
        let mut je: BTreeMap<u32, usize> = BTreeMap::new();
        for pos in 1..=w.regeln.welt.plaetze_je_system {
            let Some(pid) = w.belegung.get(&Koord::neu(s, y, pos)) else { continue };
            if let Some(a) = w.spieler[w.planeten[*pid as usize].besitzer as usize].allianz { *je.entry(a).or_default() += 1; }
        }
        let eigene = je.get(&aid).copied().unwrap_or(0);
        for (b, n) in je.into_iter().filter(|(b, n)| *b != aid && *n > eigene) { *druck.entry(b).or_default() += n - eigene; }
    }
    let ich = wir(w, sid);
    let tag = w.zeit / TAG;
    let chance = match typ { Bottyp::Igel => 250, Bottyp::Haendler => 150, _ => 400 };
    let mut wahl: Option<(usize, u32)> = None;
    for (b, d) in druck {
        let partner = w.allianzen.iter().find(|x| x.id == b).and_then(|x| x.mitglieder.first().copied());
        if d < 2 || gesperrt(w, &ich, Gegner::Allianz(b), Some(aid)) { continue; }
        if partner.is_some_and(|p| w.allianzvertrag_zwischen(sid, p, Vertragsart::Nichtangriffspakt) || w.allianzvertrag_zwischen(sid, p, Vertragsart::Verteidigungsbuendnis)) { continue; }
        if wurf(sid, 77 + b as u64, tag) >= chance { continue; }
        if wahl.is_none_or(|(m, _)| d > m) { wahl = Some((d, b)); }
    }
    let Some((d, b)) = wahl else { return };
    let name = name_von(w, Gegner::Allianz(b));
    let text = format!("Kriegserklärung: In den Systemen, in denen wir wohnen, drängt ihr uns zurück (Druck {d} Welten). Wir halten unsere Systeme und holen sie uns notfalls zurück. Frieden nach {} Stunden Ruhe oder auf Angebot.", RUHEZEIT / STUNDE);
    if tu(w, sid, json!({"typ": "allianz_anfrage", "allianz": name, "art": "krieg", "text": text})) {
        zaehle("krieg: erklärt (grenzreibung)");
        chat(w, sid, "Krieg", format!("Wir erklären der Allianz {name} den Krieg: in unseren Systemen halten sie {d} Welten mehr als wir. Ziele nennt die Leitung im Chat (Betreff Gegenschlag)."));
    }
}

/// Die Leitung bietet Frieden an, wenn die Ruhezeit um ist oder der Krieg zu lange läuft. Räuber lehnen ab (Frieden gilt
/// für die Bots trotzdem als geschlossen, siehe `aktive_kriege`). Je Gegenseite gibt es höchstens ein Angebot in fünf Tagen,
/// ob offen, abgelehnt oder angenommen (der erste Messlauf zählte 141 Angebote an Räuber in 18 Tagen, sie verbrauchten
/// das Nachrichtenkontingent der Leitung, und Kriegserklärungen scheiterten daran).
fn frieden(w: &mut Welt, sid: SpielerId) {
    if !sprecher(w, sid) { return; }
    let aid = w.spieler[sid as usize].allianz;
    let ich = wir(w, sid);
    let mut offene: BTreeMap<Gegner, SimZeit> = BTreeMap::new();
    for a in w.kommunikation.anfragen.iter().rev().take_while(|a| w.zeit - a.zeit < 30 * TAG).filter(|a| a.art == "krieg" && a.status == "erklaert") {
        let g = match a.zielallianz {
            Some(z) if a.allianz.is_some() && aid == a.allianz => Gegner::Allianz(z),
            Some(z) if aid == Some(z) && a.allianz.is_some() => Gegner::Allianz(a.allianz.unwrap_or(z)),
            Some(_) => continue,
            None if ich.contains(&a.von) && !ich.contains(&a.an) => Gegner::Reich(a.an),
            None if ich.contains(&a.an) && !ich.contains(&a.von) => Gegner::Reich(a.von),
            None => continue,
        };
        offene.entry(g).or_insert(a.zeit);
    }
    for (g, seit) in offene {
        let gegner = spieler_von(w, g);
        if gegner.is_empty() { continue; }
        let ruhe = w.zeit - letzter_kampf(w, &ich, &gegner, seit);
        if ruhe < RUHEZEIT && w.zeit - seit < MAX_DAUER { continue; }
        let name = name_von(w, g);
        let paar = |a: &kern::kommunikation::Anfrage| a.art == "frieden" && match (a.zielallianz, g) {
            (Some(z), Gegner::Allianz(b)) => (a.allianz == aid && z == b) || (a.allianz == Some(b) && Some(z) == aid),
            (None, Gegner::Reich(x)) => (ich.contains(&a.von) && a.an == x) || (ich.contains(&a.an) && a.von == x),
            _ => false,
        };
        // Auch ein älteres, noch offenes Angebot zählt: Räuber antworten nie, und jeder Zug versuchte es neu – der Kern
        // lehnte „Gleiche Anfrage ist bereits offen“ in der klassischen Epoche 23.106-mal ab (20.10.2026).
        // Ein angenommener Frieden zählt ebenso (der Kern sperrt dann jedes neue Angebot; bot_lage 20.10.2026: je Lauf über
        // 1.000-mal „Vereinbarung oder Angebot besteht bereits“ zwischen zwei Allianzen).
        let kuerzlich = w.kommunikation.anfragen.iter().rev().take_while(|a| w.zeit - a.zeit < 5 * TAG).any(|a| paar(a))
            || w.kommunikation.anfragen.iter().any(|a| matches!(a.status.as_str(), "offen" | "angenommen") && paar(a));
        if kuerzlich { continue; }
        let text = format!("Friedensangebot: Seit {} Stunden ist zwischen uns kein Schuss mehr gefallen. Wir beenden den Krieg.", ruhe / STUNDE);
        let ok = match g {
            Gegner::Reich(_) => tu(w, sid, json!({"typ": "diplomatie_anfrage", "partner": name, "art": "frieden", "text": text})),
            Gegner::Allianz(_) => tu(w, sid, json!({"typ": "allianz_anfrage", "allianz": name, "art": "frieden", "text": text})),
        };
        if ok {
            zaehle("krieg: frieden angeboten");
            chat(w, sid, "Frieden", format!("Wir bieten {name} Frieden an: {} Stunden ohne Gefecht. Bis zur Antwort bleiben Gegenschläge aus.", ruhe / STUNDE));
        }
    }
}

/// Hat die Gegenseite uns den Krieg erklärt, sagt die Leitung es der Allianz: eine Antwort im Chat, kein Schweigen.
fn kriegserklaerung_melden(w: &mut Welt, sid: SpielerId) {
    let Some(aid) = w.spieler[sid as usize].allianz else { return };
    if !sprecher(w, sid) { return; }
    for k in kriege(w, sid).into_iter().filter(|k| !k.von_uns) {
        if chat_lesen(w, aid, "Krieg", k.seit).iter().any(|_| true) { continue; }
        let name = name_von(w, k.gegner);
        let text = format!("{name} hat uns den Krieg erklärt. Wir halten die Linie: Kampfflotten rüsten, Igel sichern und entsetzen, Ziele nennt die Leitung im Chat (Betreff Gegenschlag). Frieden nach {} Stunden Ruhe oder auf Angebot.", RUHEZEIT / STUNDE);
        if chat(w, sid, "Krieg", text) { zaehle("krieg: erklärung beantwortet"); }
    }
}

// ---------------------------------------------------------------------------------------------- Rüstung und Aufklärung

/// Kampfschiffe des Reichs (ohne Bomber): auf der Heimatwelt und unterwegs.
fn kampfschiffe(w: &Welt, sid: SpielerId, pid: usize) -> i64 {
    (0..SCHIFFE).filter(|i| invasion_bot::kampfschiff(w, Einheit::ALLE[*i]) && w.bombenlast(Einheit::ALLE[*i]) == 0)
        .map(|i| w.planeten[pid].einheiten[i] + w.flotten.values().filter(|f| f.besitzer == sid).map(|f| f.schiffe[i]).sum::<i64>()).sum()
}

/// Im Krieg die Doktrin des Schweren Jägers erforschen (Waffentechnik 2, Schildtechnik 3, Impulsantrieb 2), wie sie die Räuber
/// von Anfang an verfolgen. Der Schwere Jäger braucht 25 Elektronik, der Kreuzer 100 – und Elektronik fehlt allen Reichen dauernd
/// (Lagerfüllung 0 bis 4 Prozent im Messlauf): ohne diese Forschung blieb jede Rüstung unbezahlbar.
fn doktrin_erforschen(w: &mut Welt, sid: SpielerId) {
    let Some(d) = w.regeln.flottendoktrin.get(&Einheit::SchwererJaeger) else { return };
    let sp = &w.spieler[sid as usize];
    if sp.forschung_schlange.len() >= 2 { return; }
    // Die Forschung der nächsten Stufe geht vor (Hyperraumantrieb für Stufe V): die Doktrin reiht sich erst danach ein.
    if w.regeln.stufen.get(sp.stufe as usize - 1).is_some_and(|st| st.forschung.iter().any(|(f, n)| sp.forschung[f.idx()] < *n)) { return; }
    let fehlt = d.voraussetzungen.iter().find(|(f, n)| {
        let aktiv = sp.forschung_aktiv.as_ref().is_some_and(|a| a.forschung == **f) as usize;
        let geplant = sp.forschung[f.idx()] as usize + aktiv + sp.forschung_schlange.iter().filter(|(x, _, _)| x == *f).count();
        geplant < **n as usize
    }).map(|(f, _)| *f);
    if let Some(f) = fehlt {
        if tu(w, sid, json!({"typ": "forschen", "forschung": f.name()})) { zaehle("krieg: doktrin erforscht"); }
    }
}

/// Im Krieg eine Kampfflotte aufbauen: das stärkste bezahlbare Schiff, höchstens ein Drittel des freien Bestands je
/// Zug (die Hälfte), Unterhalt zehn Tage aus den Credits gedeckt. Gerüstet wird aus dem Überschuss, nie aus dem Geld für den nächsten
/// Aufstieg. Ziel: sechs Kampfschiffe ab Stufe IV, zehn ab Stufe V.
fn ruesten(w: &mut Welt, sid: SpielerId) {
    let sp = &w.spieler[sid as usize];
    if sp.stufe < 4 { return; }
    let pid = sp.heimat as usize;
    let p = &w.planeten[pid];
    if p.gebaeude[Gebaeude::Werft.idx()] < 1 || p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 || p.fertigung[0].len() >= 3 { return; }
    let soll = if sp.stufe >= 5 { 10 } else { 6 };
    let in_werft: i64 = p.fertigung[0].iter().filter(|f| matches!(f.produkt, Produkt::Einheit(e) if invasion_bot::kampfschiff(w, e) && w.bombenlast(e) == 0)).map(|f| f.rest).sum();
    let hat = kampfschiffe(w, sid, pid) + in_werft;
    if hat >= soll { return; }
    let r = w.regeln.clone();
    let volk = sp.volk;
    // Der Bestand über der Aufstiegsrücklage, ohne zehn Prozent jedes Lagers (höchstens ein Viertel des Bestands: Elektronik
    // liegt dauernd bei null bis fünf Prozent, ein fester Puffer hätte sie ganz gesperrt). Die Rücklage für das nächste Ausbauziel zählt
    // hier nicht (im ersten Messlauf blieb mit ihr und einem Viertel Lager alles unbezahlbar: kein Bot rüstete je).
    let frei = {
        let (b, rueck, grenze) = (w.bestand_jetzt(pid), aufstiegs_ruecklage(w, sid, pid), w.lagergrenze(pid));
        std::array::from_fn::<i64, GUETER, _>(|i| (b[i] - rueck[i] - (grenze[i] / 10).min(b[i] / 4)).max(0))
    };
    for e in [Einheit::Schlachtschiff, Einheit::Kreuzer, Einheit::Sturmjaeger, Einheit::SchwererJaeger, Einheit::Abfangjaeger] {
        if r.einh(e).angriff < 100 || !baubar(w, sid, pid, e) { continue; }
        let kosten = r.kosten_einheit(e, volk);
        let n = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| frei[g] / 2 / kosten[g]).min().unwrap_or(0).min(soll - hat).min(6);
        if n < 1 { continue; }
        let neu = n * r.wert(&kosten);
        let je_tag = mal(w.flottenwert(sid) + neu, r.wirtschaft.unterhalt_schiffe_je_tag);
        if w.spieler[sid as usize].credits < 10 * je_tag { zaehle("krieg: rüstung nicht tragbar"); return; }
        let ks = w.planeten[pid].koord.to_string();
        if tu(w, sid, json!({"typ": "fertigen", "planet": ks, "einheit": e.name(), "anzahl": n})) { zaehle("krieg: rüstung bestellt"); }
        return;
    }
    zaehle("krieg: rüstung nicht bezahlbar");
}

/// Bekannte Planeten der Gegenseite, ohne etwas zu verraten: Koordinaten, die das eigene Wissen (`planetenwissen`)
/// dem Gegner zuordnet.
fn entdeckt(w: &Welt, sid: SpielerId, feinde: &[SpielerId]) -> Vec<Koord> {
    let mut aus = Vec::new();
    for f in feinde {
        let name = &w.spieler[*f as usize].name;
        for pid in &w.spieler[*f as usize].planeten {
            let k = w.planeten[*pid as usize].koord;
            if w.planetenwissen(sid, k)["spieler"].as_str() == Some(name.as_str()) { aus.push(k); }
        }
    }
    aus
}

/// Ziele des Gegners auskundschaften: veraltete Berichte erneuern, im Chat genannte Orte ansehen und, wenn nichts
/// bekannt ist, Planeten aus dem eigenen Wissen ansteuern. Fünf Sonden je Spähflug (siehe `SPAEH_SONDEN`). Nur wer eine
/// Flotte für einen Gegenschlag hat, späht; ein Ziel, das die eigene Flotte ohnehin nicht besiegt, wird höchstens
/// einmal am Tag erneut angesehen (sonst kreisten die Sonden im Takt um denselben Planeten).
fn aufklaeren(w: &mut Welt, sid: SpielerId, feinde: &[SpielerId], chat_ziele: &[Koord], schutz: &Opferschutz) -> bool {
    let pid = w.spieler[sid as usize].heimat as usize;
    if w.planeten[pid].gebaeude[Gebaeude::Raumhafen.idx()] < 1 { return false; }
    let ang = invasion_bot::aufstellung(w, pid, false);
    if ang.iter().sum::<i64>() < 5 { return false; }
    let jetzt = w.zeit;
    let frei = w.flottenplaetze(sid) as i64 - w.flotten.values().filter(|f| f.besitzer == sid).count() as i64;
    let r = w.regeln.clone();
    let volk = w.spieler[sid as usize].volk;
    let eskorte: i64 = (0..SCHIFFE).map(|i| ang[i] * r.wert(&r.kosten_einheit(Einheit::ALLE[i], volk))).sum();
    let sp = &w.spieler[sid as usize];
    // Frisch genug ist ein Bericht, solange das Ziel für die eigene Flotte erreichbar scheint (vier Stunden), sonst einen Tag.
    let frisch = |b: &Spionagebericht| jetzt - b.zeit < if verteidigungswert(w, b).is_some_and(|v| eskorte < 2 * v) { TAG } else { BERICHT_ALTER / 2 };
    let mut kand: Vec<Koord> = sp.berichte.iter().filter(|b| feinde.contains(&b.besitzer) && !frisch(b)).map(|b| b.ziel).collect();
    kand.extend(chat_ziele.iter().copied().filter(|k| !sp.berichte.iter().any(|b| b.ziel == *k && frisch(b))));
    if kand.is_empty() && !sp.berichte.iter().any(|b| feinde.contains(&b.besitzer) && jetzt - b.zeit < BERICHT_ALTER) && w.zeit % (6 * STUNDE) < TAKT {
        kand.extend(entdeckt(w, sid, feinde));
    }
    let heim = w.planeten[pid].koord;
    if kand.is_empty() && w.zeit % (6 * STUNDE) < TAKT && !sp.berichte.iter().any(|b| feinde.contains(&b.besitzer)) {
        return umfeld_erkunden(w, sid, pid, feinde);
    }
    let mut kand: Vec<(i64, Koord)> = kand.into_iter()
        .filter(|k| w.belegung.get(k).is_none_or(|p| { let b = w.planeten[*p as usize].besitzer; !neuling(w, schutz, b) && !w.verbuendet(sid, b) }))
        .filter(|k| !w.flotten.values().any(|f| f.besitzer == sid && f.ziel == *k && matches!(f.mission, Mission::Spionage | Mission::SystemErkunden)))
        .map(|k| (w.entfernung(heim, k), k)).collect();
    kand.sort();
    kand.dedup_by_key(|k| k.1);
    let Some((_, ziel)) = kand.first().copied() else { return false };
    let mission = if w.aufklaerungsregeln() && !w.system_erfasst(sid, ziel) { "system_erkunden" } else { "spionage" };
    let anzahl = if mission == "spionage" { SPAEH_SONDEN } else { 1 };
    if frei > 0 && w.planeten[pid].einheiten[Einheit::Spionagesonde.idx()] >= anzahl {
        if let Some(a) = sondenauftrag(w, sid, pid, ziel, mission, anzahl) {
            if tu(w, sid, a) { zaehle("krieg: aufklärung gestartet"); return true; }
        }
        return false;
    }
    // Sonden fehlen: bis zu zwei Spähflüge auf Vorrat bestellen (300 Kristall je Sonde).
    let fehlt = 2 * SPAEH_SONDEN - sonden_geplant(w, sid, pid);
    if fehlt > 0 && w.planeten[pid].fertigung[0].len() < 3 {
        let kosten = w.regeln.kosten_einheit(Einheit::Spionagesonde, w.spieler[sid as usize].volk);
        let bestand = w.bestand_jetzt(pid);
        let n = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| bestand[g] / kosten[g]).min().unwrap_or(0).min(fehlt);
        if n >= 1 {
            let ks = w.planeten[pid].koord.to_string();
            if tu(w, sid, json!({"typ": "fertigen", "planet": ks, "einheit": "spionagesonde", "anzahl": n})) { zaehle("krieg: sonden bestellt"); }
        }
    }
    false
}

/// Der Gegner ist unbekannt: seine Angriffe kamen aus der Nachbarschaft des angegriffenen Orts (Räuber greifen höchstens
/// 15 Systeme von ihrer Heimat entfernt an). Das nächste unerfasste System dort erkunden; danach nennen die Bewohnerlisten
/// des Systems den Gegner, und `aufklaeren` späht ihn aus.
fn umfeld_erkunden(w: &mut Welt, sid: SpielerId, pid: usize, feinde: &[SpielerId]) -> bool {
    if !w.aufklaerungsregeln() || w.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::SystemErkunden) { return false; }
    let frei = w.flottenplaetze(sid) > w.flotten.values().filter(|f| f.besitzer == sid).count();
    if !frei || w.planeten[pid].einheiten[Einheit::Spionagesonde.idx()] < 1 { return false; }
    let heim = w.planeten[pid].koord;
    let orte: Vec<Koord> = w.kampfberichte.iter().rev().take_while(|k| w.zeit - k.zeit < 6 * TAG)
        .filter(|k| feinde.contains(&k.angreifer)).map(|k| k.ort).collect();
    let systeme = w.regeln.welt.systeme_je_sektor;
    // Räuber greifen zuerst die nächsten Nachbarn an: erst die Systeme nahe am angegriffenen Ort, dann die näher an der Heimat.
    let ziel = orte.iter().flat_map(|o| (1..=systeme).filter(move |n| (*n as i64 - o.system as i64).abs() <= 15).map(move |n| ((n as i64 - o.system as i64).abs(), Koord::neu(o.sektor, n, 1))))
        .filter(|(_, k)| !w.system_erfasst(sid, *k)).min_by_key(|(d, k)| (*d, w.entfernung(heim, *k), *k)).map(|(_, k)| k);
    let Some(ziel) = ziel else { return false };
    let Some(a) = sondenauftrag(w, sid, pid, ziel, "system_erkunden", 1) else { return false };
    let ok = tu(w, sid, a);
    if ok { zaehle("krieg: umfeld erkundet"); }
    ok
}

/// Gegenschlag mit der Kampfflotte der Heimatwelt auf das beste frisch ausgespähte Ziel der Gegenseite, wenn der Simulator
/// den Sieg verspricht (Siegchance ab 75 Prozent, Verluste nicht über Beute plus 3.000).
fn gegenschlag(w: &mut Welt, sid: SpielerId, feinde: &[SpielerId], ich: &[SpielerId], chat_ziele: &[Koord], schutz: &Opferschutz) -> bool {
    let pid = w.spieler[sid as usize].heimat as usize;
    if w.planeten[pid].blockade.is_some() || w.planeten[pid].gebaeude[Gebaeude::Raumhafen.idx()] < 1 { return false; }
    let frei = w.flottenplaetze(sid) as i64 - w.flotten.values().filter(|f| f.besitzer == sid).count() as i64;
    if frei <= 0 { zaehle("krieg: keine flottenplätze"); return false; }
    let ang = invasion_bot::aufstellung(w, pid, false);
    if ang.iter().sum::<i64>() < 5 { zaehle("krieg: zu wenig kampfschiffe"); return false; }
    // Höchstens drei Schläge je Tag gegen dieselbe Gegenseite.
    let heute = w.kampfberichte.iter().rev().take_while(|k| w.zeit - k.zeit < TAG).filter(|k| ich.contains(&k.angreifer) && k.verteidiger.iter().any(|v| feinde.contains(v))).count();
    let unterwegs = w.flotten.values().filter(|f| ich.contains(&f.besitzer) && f.mission == Mission::Angriff && f.zustand == Flottenzustand::Hinflug
        && w.belegung.get(&f.ziel).is_some_and(|p| feinde.contains(&w.planeten[*p as usize].besitzer))).count();
    if heute + unterwegs >= SCHLAEGE_JE_TAG { zaehle("krieg: tageslimit erreicht"); return false; }
    let r = w.regeln.clone();
    let jetzt = w.zeit;
    let mut berichte: Vec<(i64, Spionagebericht)> = w.spieler[sid as usize].berichte.iter()
        .filter(|b| jetzt - b.zeit < BERICHT_ALTER && feinde.contains(&b.besitzer) && (w.aufklaerungsregeln() || !geschuetzt(w, b.besitzer)))
        .filter(|b| !neuling(w, schutz, b.besitzer) && !w.verbuendet(sid, b.besitzer) && !w.vertrag_zwischen(sid, b.besitzer, Vertragsart::Nichtangriffspakt))
        .filter(|b| w.belegung.get(&b.ziel).is_some_and(|p| w.planeten[*p as usize].besitzer == b.besitzer))
        .filter(|b| !w.flotten.values().any(|f| f.besitzer == sid && f.ziel == b.ziel && f.mission == Mission::Angriff))
        .filter(|b| !(schutz.aktiv && w.kampfberichte.iter().rev().take_while(|k| w.zeit - k.zeit < schutz.pause_stunden as i64 * STUNDE).any(|k| k.ort == b.ziel)))
        .map(|b| {
            let mut lager = [0i64; GUETER];
            lager.copy_from_slice(&b.bestand[..GUETER]);
            // Im Chat genannte Ziele zuerst.
            (-r.wert(&lager) - if chat_ziele.contains(&b.ziel) { 1_000_000 * M } else { 0 }, b.clone())
        }).collect();
    berichte.sort_by_key(|(wert, _)| *wert);
    let mut bestes: Option<(i64, Koord, i64)> = None;
    for (_, b) in berichte.iter().take(3) {
        let Some((chance, verlust, beute)) = angriff_schaetzen(w, sid, b, &ang) else { continue };
        let (verlust, menge, wert) = (verlust / M, beute.iter().sum::<i64>() / M, r.wert(&beute) / M);
        if chance < 75 { zaehle("krieg: siegchance < 75"); continue; }
        if verlust > wert + 3000 { zaehle("krieg: verlust zu hoch"); continue; }
        if bestes.map(|(s, _, _)| wert - verlust > s).unwrap_or(true) { bestes = Some((wert - verlust, b.ziel, menge)); }
    }
    let Some((_, ziel, menge)) = bestes else { return false };
    let transporter = w.planeten[pid].einheiten[Einheit::KleinerTransporter.idx()];
    let ladung = r.einh(Einheit::KleinerTransporter).ladung;
    let mit = ((menge + ladung - 1) / ladung).clamp(1, transporter.max(1)).min(transporter);
    let mut schiffe = json!({});
    let mut flotte = [0i64; SCHIFFE];
    for i in 0..SCHIFFE {
        if ang[i] > 0 { schiffe[Einheit::ALLE[i].name()] = json!(ang[i]); flotte[i] = ang[i]; }
    }
    if mit > 0 { schiffe["kleiner_transporter"] = json!(mit); flotte[Einheit::KleinerTransporter.idx()] = mit; }
    let deut = w.bestand_jetzt(pid)[Gut::Deuterium.idx()];
    let Some(tempo) = [1.0, 0.8, 0.6, 0.4, 0.3].into_iter().find(|s| w.flugplan(sid, pid, ziel, &flotte, milli(*s)).map(|p| 2 * p.treibstoff <= deut).unwrap_or(false)) else {
        zaehle("krieg: treibstoff fehlt");
        return false;
    };
    let ks = w.planeten[pid].koord.to_string();
    let neu = w.naechste_flotte;
    let ok = tu(w, sid, json!({"typ": "flotte_senden", "start": ks, "ziel": ziel.to_string(), "mission": "angriff", "schiffe": schiffe, "geschwindigkeit": tempo}));
    zaehle(if ok { "krieg: gegenschlag gestartet" } else { "krieg: gegenschlag abgelehnt" });
    if ok && ich.len() > 1 {
        // Koordiniert: ein Mitglied mit demselben Ziel hat einen Verband geöffnet – anschließen (gemeinsame Ankunft, höchstens
        // sechs Stunden Verzögerung); sonst selbst einen Verband öffnen, damit andere Mitglieder anschließen können.
        let fuehrung = w.flotten.values().find(|f| f.id != neu && ich.contains(&f.besitzer) && f.verband == Some(f.id) && f.mission == Mission::Angriff
            && f.zustand == Flottenzustand::Hinflug && f.ziel == ziel && f.ankunft > w.zeit).map(|f| f.id);
        match fuehrung {
            Some(fu) => if tu(w, sid, json!({"typ": "verband_beitreten", "flotte": neu, "fuehrung": fu})) { zaehle("krieg: verband beigetreten"); },
            None => if tu(w, sid, json!({"typ": "verband_oeffnen", "flotte": neu})) { zaehle("krieg: verband geöffnet"); },
        }
    }
    if ok {
        let besitzer = w.belegung.get(&ziel).map(|p| w.spieler[w.planeten[*p as usize].besitzer as usize].name.clone()).unwrap_or_default();
        if let Some(aid) = w.spieler[sid as usize].allianz {
            if chat_lesen(w, aid, "Gegenschlag", w.zeit - 12 * STUNDE).iter().all(|(_, t)| !koordinaten(t).contains(&ziel)) {
                chat(w, sid, "Gegenschlag", format!("Gegenschlag gestartet auf {ziel} ({besitzer}). Wer eine Flotte frei hat, schließt sich an."));
            }
        }
    }
    ok
}

/// Die Leitung nennt im Chat ein Ziel, das sie selbst frisch ausgespäht hat und das noch nicht genannt wurde.
fn ziel_nennen(w: &mut Welt, sid: SpielerId, feinde: &[SpielerId], schutz: &Opferschutz) {
    let Some(aid) = w.spieler[sid as usize].allianz else { return };
    if !sprecher(w, sid) { return; }
    let genannt: Vec<Koord> = chat_lesen(w, aid, "Gegenschlag", w.zeit - 12 * STUNDE).into_iter().flat_map(|(_, t)| koordinaten(&t)).collect();
    let jetzt = w.zeit;
    let ziel = w.spieler[sid as usize].berichte.iter()
        .filter(|b| jetzt - b.zeit < BERICHT_ALTER && feinde.contains(&b.besitzer) && !genannt.contains(&b.ziel) && !neuling(w, schutz, b.besitzer))
        .max_by_key(|b| b.zeit).map(|b| (b.ziel, b.besitzer));
    if let Some((k, besitzer)) = ziel {
        let name = w.spieler[besitzer as usize].name.clone();
        if chat(w, sid, "Gegenschlag", format!("Ziel für den Gegenschlag: {k} ({name}), frisch ausgespäht. Wer eine Flotte frei hat, sieht es sich an und schlägt bei sicherem Sieg zu.")) {
            zaehle("krieg: ziel genannt");
        }
    }
}

/// Ein Zug von Krieg und Verteidigung (nach Räuber und Verteidigung); `schutz` ist der Opferschutz des Servers.
pub(super) fn zug(w: &mut Welt, sid: SpielerId, typ: Bottyp, schutz: &Opferschutz) {
    if typ == Bottyp::Wehrlos || typ == Bottyp::Raeuber || !w.spieler_aktiv(sid) || w.spieler[sid as usize].stufe < 3 { return; }
    // Diplomatie der Leitung.
    frieden(w, sid);
    kriegserklaerung_melden(w, sid);
    erklaeren(w, sid, typ, schutz);
    konkurrenz(w, sid);
    reibung(w, sid, typ);
    // Igel sichern und entsetzen; kämpfen tun Ökonomen und Händler.
    if typ == Bottyp::Igel { return; }
    let kriege = aktive_kriege(w, sid);
    if kriege.is_empty() {
        invasion_bot::aufraeumen(w, sid, true);
        return;
    }
    let feinde: Vec<SpielerId> = kriege.iter().flat_map(|(_, g)| g.iter().copied()).collect();
    let ich = wir(w, sid);
    let ziele = chat_ziele(w, sid);
    doktrin_erforschen(w, sid);
    ruesten(w, sid);
    ziel_nennen(w, sid, &feinde, schutz);
    // Ab Stufe V Bombardierung: nach einem halben Tag Krieg den Bausatz vorbereiten, dann mit Bombern zuschlagen.
    let dauer = kriege.iter().map(|(k, _)| w.zeit - k.seit).max().unwrap_or(0);
    if w.spieler[sid as usize].stufe >= 5 && dauer >= TAG / 2 { invasion_bot::vorbereiten(w, sid, true); }
    invasion_bot::aufraeumen(w, sid, false);
    if gegenschlag(w, sid, &feinde, &ich, &ziele, schutz) { return; }
    if w.spieler[sid as usize].stufe >= 5 {
        if let Some(k) = invasion_bot::schlag(w, sid, schutz, Modus::Krieg, &|_, s| feinde.contains(&s)) {
            let besitzer = w.belegung.get(&k).map(|p| w.spieler[w.planeten[*p as usize].besitzer as usize].name.clone()).unwrap_or_default();
            chat(w, sid, "Gegenschlag", format!("Bombardierung von {k} ({besitzer}) gestartet. Wer Truppen und ein Kolonieschiff frei hat, folgt zur Landung."));
            return;
        }
    }
    aufklaeren(w, sid, &feinde, &ziele, schutz);
}
