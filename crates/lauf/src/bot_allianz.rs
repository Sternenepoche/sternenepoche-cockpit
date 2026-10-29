//! Allianzen der Skriptbots (Karl 20.10.2026: „es soll zu beginn nicht feststehen wer sich wie findet … auf eine
//! Größe von 10 Mitgliedern wachsen … die Kommunikation untereinander so, dass sie sich koordinieren können –
//! aktuell sind das Ameisen ohne Ziel“).
//!
//! Wer wen einlädt, folgt aus dem Spielverlauf, nicht aus der Spielernummer: Nachbarschaft in bekannten Systemen,
//! Nichtangriffspakte, Briefkontakt und gemeinsamer Markthandel zählen, wer angreift oder Pakte ablehnt, wird nie
//! eingeladen. Einladungen laufen als Diplomatieanfrage (Menschen nehmen sie über die Antwortleiste an). Die
//! Absprachen stehen im Allianzchat: Vorstellung beim Beitritt, das Sammelsystem für die Dysonsphäre, das die
//! Mitglieder dort lesen und bei der Koloniewahl bevorzugen (`sammelsystem`, `kolonisieren`, `bot_dyson`).

use super::*;

/// Betreff, unter dem die Allianzleitung das Sammelsystem verkündet; Mitglieder lesen ihn aus dem Allianzchat.
pub(super) const SAMMEL_BETREFF: &str = "Sammelsystem";

/// Deterministische Streuung je Spieler, Gegenstand und Tag: wer heute gründet, wechselt oder zuerst eingeladen wird.
fn wuerfel(a: SpielerId, b: u64, tag: i64) -> u64 {
    let mut x = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ (tag as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    (x ^ (x >> 29)) % 1000
}

/// Beziehung aus Sicht von `a`: hoch für Paktpartner, Briefkontakte, Handelspartner und bekannte Nachbarn,
/// negativ nach Kämpfen oder abgelehnten Pakten (dann keine Einladung).
pub(super) fn beziehung(w: &Welt, a: SpielerId, b: SpielerId) -> i64 {
    if a == b { return i64::MIN / 2; }
    if w.kampfberichte.iter().any(|k| (k.angreifer == b && k.verteidiger.contains(&a)) || (k.angreifer == a && k.verteidiger.contains(&b))) { return -100; }
    if w.vertraege.iter().any(|v| v.art == Vertragsart::Nichtangriffspakt && v.status == Vertragsstatus::Abgelehnt
        && ((v.a == a && v.b == b) || (v.a == b && v.b == a))) { return -50; }
    let mut s = 0i64;
    if w.vertrag_zwischen(a, b, Vertragsart::Nichtangriffspakt) { s += 5; }
    if w.privat_kontakt(a, b) || w.privat_kontakt(b, a) { s += 2; }
    s += (w.handel.iter().rev().take(3000).filter(|h| (h.kaeufer == a && h.verkaeufer == b) || (h.kaeufer == b && h.verkaeufer == a)).count() as i64).min(5);
    let (ha, hb) = (w.planeten[w.spieler[a as usize].heimat as usize].koord, w.planeten[w.spieler[b as usize].heimat as usize].koord);
    if ha.sektor == hb.sektor && (!w.aufklaerungsregeln() || w.system_erfasst(a, hb)) {
        s += (12 - (ha.system as i64 - hb.system as i64).abs()).max(0);
    }
    s
}

/// Wie gut eine Allianz zu `sid` passt: Größe (bis zur Bot-Grenze) und die beste Beziehung zu einem Mitglied.
pub(super) fn allianz_passung(w: &Welt, sid: SpielerId, aid: u32) -> i64 {
    // Leere Allianzen (alle ausgetreten) passen nie: ihre Einladung wird abgelehnt statt jeden Zug vergeblich angenommen.
    let Some(a) = w.allianzen.iter().find(|a| a.id == aid && !a.mitglieder.is_empty()) else { return i64::MIN / 2 };
    let beste = a.mitglieder.iter().map(|m| beziehung(w, sid, *m)).max().unwrap_or(0);
    // Mit einem Mitglied im Streit (Kampf, abgelehnter Pakt) passt die Allianz nicht, wie gut die anderen auch sind.
    let schlechteste = a.mitglieder.iter().map(|m| beziehung(w, sid, *m)).min().unwrap_or(0);
    if schlechteste < 0 { return schlechteste; }
    a.mitglieder.len().min(w.bot_allianz_max()) as i64 * 3 + beste
}

/// Das zuletzt im Allianzchat verkündete Sammelsystem (Sektor, System).
pub(super) fn sammelsystem(w: &Welt, aid: u32) -> Option<(u8, u8)> {
    sammel_briefe(w, aid).next().and_then(|(_, sy)| sy)
}

/// Verkündungen des Sammelsystems im Allianzchat, die neueste zuerst: Zeit und das genannte System.
fn sammel_briefe(w: &Welt, aid: u32) -> impl Iterator<Item = (SimZeit, Option<(u8, u8)>)> + '_ {
    w.kommunikation.briefe.iter().rev().take(4000)
        .filter(move |(_, b)| b.kanal == "allianz" && b.betreff == SAMMEL_BETREFF && b.allianzen.contains(&aid))
        .map(|(id, _)| {
            let n = &w.nachrichten[*id as usize - 1];
            (n.zeit, n.text.split(|c: char| c.is_whitespace() || c == ',' || c == '.' || c == ';').find_map(|t| {
                let mut teile = t.split(':');
                let s = teile.next()?.parse().ok()?;
                let y = teile.next()?.parse().ok()?;
                teile.next().is_none().then_some((s, y))
            }))
        })
}

/// Seit wann das aktuelle Sammelsystem ohne Unterbrechung verkündet ist (die älteste Verkündung derselben Folge).
fn sammel_seit(w: &Welt, aid: u32) -> Option<SimZeit> {
    let mut briefe = sammel_briefe(w, aid);
    let (mut seit, aktuell) = briefe.next()?;
    for (zeit, sy) in briefe { if sy != aktuell { break; } seit = zeit; }
    Some(seit)
}

fn chat(w: &mut Welt, sid: SpielerId, betreff: &str, text: String) -> bool {
    tu(w, sid, json!({"typ": "brief_senden", "kanal": "allianz", "an": "", "betreff": betreff, "text": text, "antwort_auf": null}))
}

fn rolle(typ: Bottyp) -> &'static str {
    match typ {
        Bottyp::Haendler => "Handel und Logistik",
        Bottyp::Igel => "Verteidigung und Entsatz",
        Bottyp::Raeuber => "Flotten",
        _ => "Industrie und Forschung",
    }
}

/// Hat `sid` im Allianzchat von `aid` schon eine Nachricht mit diesem Betreff geschrieben?
fn schon_geschrieben(w: &Welt, sid: SpielerId, aid: u32, betreff: &str, seit: SimZeit) -> bool {
    w.kommunikation.briefe.iter().rev().take(4000).take_while(|(id, _)| w.nachrichten[**id as usize - 1].zeit >= seit)
        .any(|(id, b)| b.kanal == "allianz" && b.betreff == betreff && b.allianzen.contains(&aid) && w.nachrichten[*id as usize - 1].von == sid)
}

/// Einmal je Spieltag (im ersten Bot-Takt des Tages): gründen, wechseln, einladen, vorstellen, Sammelsystem.
pub(super) fn zug(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
    if typ == Bottyp::Raeuber || typ == Bottyp::Wehrlos || w.zeit % TAG >= TAKT || w.zeit < TAG { return; }
    let tag = w.zeit / TAG;
    let tag_beginn = w.zeit - w.zeit % TAG;
    let sp = &w.spieler[sid as usize];
    match sp.allianz {
        None => ohne_allianz(w, sid, typ, tag, tag_beginn),
        Some(aid) => {
            vorstellen(w, sid, typ, aid);
            if w.allianzen.iter().find(|a| a.id == aid).and_then(|a| a.mitglieder.first()) == Some(&sid) {
                einladen(w, sid, aid, tag, tag_beginn);
                sammeln(w, sid, aid, tag_beginn);
            }
            wechseln(w, sid, aid, tag);
        }
    }
}

fn ohne_allianz(w: &mut Welt, sid: SpielerId, typ: Bottyp, tag: i64, tag_beginn: SimZeit) {
    let sp = &w.spieler[sid as usize];
    if sp.stufe < 2 || tag < 2 { return; }
    // Offene Einladungen entscheidet `reagieren_mit` sofort; heute eingeladen worden: abwarten.
    if w.kommunikation.anfragen.iter().any(|a| a.an == sid && a.art == "allianz" && (a.status == "offen" || a.zeit >= tag_beginn - TAG)) { return; }
    // Gründen nur, wenn zu wenige Allianzen Platz haben, und gestreut: so steht nicht fest, wer gründet.
    let max = w.bot_allianz_max();
    let mit_platz = w.allianzen.iter().filter(|a| !a.mitglieder.is_empty() && a.mitglieder.len() + a.eingeladen.len() + 2 <= max).count();
    // Frei ist, wer ohne Allianz ist und in der letzten Woche keine Einladung abgelehnt hat (Räuber lehnen immer ab).
    let frei = (0..w.spieler.len() as u16).filter(|s| w.spieler_aktiv(*s) && w.spieler[*s as usize].allianz.is_none()
        && !w.kommunikation.anfragen.iter().rev().take_while(|a| w.zeit - a.zeit < 7 * TAG).any(|a| a.an == *s && a.art == "allianz" && a.status == "abgelehnt")).count();
    // Ab Tag 4 ohne Einladung: der Leitung der passendsten Allianz mit Platz einen Beitrittswunsch schreiben (höchstens
    // alle drei Tage). Der Brief ist der Kontakt, nach dem die Leitung am nächsten Tag einlädt (`einladen`).
    if tag >= 4 && mit_platz > 0 {
        let wunsch = w.allianzen.iter().filter(|a| !a.mitglieder.is_empty() && a.mitglieder.len() + a.eingeladen.len() < max)
            .map(|a| (allianz_passung(w, sid, a.id), a.mitglieder[0], a.name.clone())).filter(|(p, _, _)| *p >= 0).max_by_key(|(p, _, _)| *p);
        if let Some((_, leitung, name)) = wunsch {
            let schon = w.kommunikation.briefe.iter().rev().take(4000).take_while(|(id, _)| w.zeit - w.nachrichten[**id as usize - 1].zeit < 3 * TAG)
                .any(|(id, b)| b.kanal == "privat" && b.betreff == "Beitrittswunsch" && w.nachrichten[*id as usize - 1].von == sid);
            if schon { return; }
            {
                let heim = w.planeten[sp.heimat as usize].koord;
                let text = format!("Ich möchte der Allianz '{name}' beitreten: Schwerpunkt {}, Heimat im System {}:{}, Zivilisation {}. Bitte lade mich ein.",
                    rolle(typ), heim.sektor, heim.system, sp.stufe);
                let an = w.spieler[leitung as usize].name.clone();
                if tu(w, sid, json!({"typ": "brief_senden", "kanal": "privat", "an": an, "betreff": "Beitrittswunsch", "text": text, "antwort_auf": null})) {
                    zaehle("allianz: beitrittswunsch");
                }
                return;
            }
        }
    }
    // Wer gerade gewechselt ist oder eine passende Allianz mit Platz kennt, gründet nicht (sonst entstanden im Messlauf
    // Ketten aus Gründung und Wechsel mit vierzehn leeren Allianzen).
    let kennt_platz = w.allianzen.iter().any(|a| !a.mitglieder.is_empty() && a.mitglieder.len() + a.eingeladen.len() < max && allianz_passung(w, sid, a.id) >= 0);
    let gewechselt = w.kommunikation.briefe.iter().rev().take(4000).take_while(|(id, _)| w.zeit - w.nachrichten[**id as usize - 1].zeit < 7 * TAG)
        .any(|(id, b)| b.kanal == "allianz" && b.betreff == "Wechsel" && w.nachrichten[*id as usize - 1].von == sid);
    if (tag >= 4 && kennt_platz) || gewechselt { return; }
    // Wenige große Allianzen statt vieler kleiner: eine Neugründung je zehn freie Reiche.
    if mit_platz * 10 >= frei.max(1) || wuerfel(sid, 1, tag) >= 400 { return; }
    let name = match typ {
        Bottyp::Haendler => format!("Bund {}", sp.name),
        Bottyp::Igel => format!("Schildwall {}", sp.name),
        _ => format!("Werkbund {}", sp.name),
    };
    if tu(w, sid, json!({"typ": "allianz_gruenden", "name": name})) {
        zaehle("allianz: gegründet");
        let heim = w.planeten[w.spieler[sid as usize].heimat as usize].koord;
        chat(w, sid, "Gründung", format!("Gegründet von {}. Unser Ziel: gemeinsame Verteidigung, Handel im internen Markt und eine Dysonsphäre. Heimat im System {}:{}; das Sammelsystem verkünde ich, sobald wir vier sind.",
            w.spieler[sid as usize].name, heim.sektor, heim.system));
    }
}

fn vorstellen(w: &mut Welt, sid: SpielerId, typ: Bottyp, aid: u32) {
    if schon_geschrieben(w, sid, aid, "Vorstellung", 0) || w.allianzen.iter().find(|a| a.id == aid).and_then(|a| a.mitglieder.first()) == Some(&sid) { return; }
    let sp = &w.spieler[sid as usize];
    let heim = w.planeten[sp.heimat as usize].koord;
    let ziel = sammelsystem(w, aid).map(|(s, y)| format!(" Ich siedle im Sammelsystem {s}:{y}.")).unwrap_or_default();
    let text = format!("Ich bin {}: Schwerpunkt {}, Heimat im System {}:{}, Zivilisation {}, {} Kolonien.{ziel}", sp.name, rolle(typ), heim.sektor, heim.system, sp.stufe, w.kolonien(sid));
    if chat(w, sid, "Vorstellung", text) { zaehle("allianz: vorgestellt"); }
}

fn einladen(w: &mut Welt, sid: SpielerId, aid: u32, tag: i64, tag_beginn: SimZeit) {
    if w.kommunikation.anfragen.iter().rev().take_while(|a| a.zeit >= tag_beginn).any(|a| a.von == sid && a.art == "allianz") { return; }
    let Some(al) = w.allianzen.iter().find(|a| a.id == aid).cloned() else { return };
    let platz = w.bot_allianz_max().saturating_sub(al.mitglieder.len() + al.eingeladen.len());
    if platz == 0 { return; }
    let mut kandidaten: Vec<(i64, u64, SpielerId)> = (0..w.spieler.len() as u16)
        .filter(|b| *b != sid && w.spieler_aktiv(*b) && w.spieler[*b as usize].allianz.is_none() && !al.eingeladen.contains(b))
        .filter(|b| !w.kommunikation.anfragen.iter().any(|a| a.an == *b && a.art == "allianz" && a.status == "offen"))
        // Wer eine Einladung dieser Allianz in der letzten Woche abgelehnt hat (Räuber lehnen immer ab), bekommt keine
        // neue – sonst verbrauchten Absagen jeden Tag alle drei Einladungen, und Bots mit Beitrittswunsch kamen nie dran.
        .filter(|b| !w.kommunikation.anfragen.iter().rev().take_while(|a| w.zeit - a.zeit < 7 * TAG)
            .any(|a| a.an == *b && a.art == "allianz" && a.status == "abgelehnt" && a.allianz == Some(aid)))
        // Die beste Beziehung eines Mitglieds zählt, aber wer ein Mitglied angegriffen hat oder abgewiesen wurde (Beziehung
        // unter 0), wird nicht eingeladen – auch wenn ein anderes Mitglied ihn gut kennt (Test des Krieg-Agenten 20.10.2026).
        .filter(|b| al.mitglieder.iter().all(|m| beziehung(w, *m, *b) >= 0))
        .map(|b| (al.mitglieder.iter().map(|m| beziehung(w, *m, b)).max().unwrap_or(0), wuerfel(sid, b as u64, tag), b))
        // Nur mit echter Beziehung (Brief, Pakt, Handel oder bekannte Nachbarschaft) – für Menschen wie für Bots.
        .filter(|(rel, _, _)| *rel >= 2)
        .collect();
    kandidaten.sort_by_key(|(rel, los, _)| (std::cmp::Reverse(*rel), *los));
    let ziel = sammelsystem(w, aid).map(|(s, y)| format!(", Sammelsystem {s}:{y}")).unwrap_or_default();
    let mut gesendet = 0;
    for (_, _, b) in kandidaten {
        if gesendet >= platz.min(3) { break; }
        let text = format!("Einladung in die Allianz '{}' ({} Mitglieder{ziel}): gemeinsame Verteidigung, Handel im internen Markt und eine Dysonsphäre. Annehmen über die Antwortleiste.", al.name, al.mitglieder.len());
        let name = w.spieler[b as usize].name.clone();
        if tu(w, sid, json!({"typ": "diplomatie_anfrage", "partner": name, "art": "allianz", "text": text})) {
            zaehle("allianz: eingeladen");
            gesendet += 1;
        }
    }
}

/// Die Allianzleitung verkündet ab vier Mitgliedern das Sammelsystem für die Dysonsphäre (alle drei Tage erneut, damit
/// es im Chat sichtbar bleibt): ein System ohne Heimatwelten mit vielen freien Plätzen nahe den Heimatwelten der
/// Mitglieder, Nebelsysteme bevorzugt (natürlicher Xenokristall).
fn sammeln(w: &mut Welt, sid: SpielerId, aid: u32, tag_beginn: SimZeit) {
    let Some(al) = w.allianzen.iter().find(|a| a.id == aid).cloned() else { return };
    if al.mitglieder.len() < 4 || schon_geschrieben(w, sid, aid, SAMMEL_BETREFF, tag_beginn - 2 * TAG) { return; }
    let heimaten: Vec<Koord> = al.mitglieder.iter().map(|m| w.planeten[w.spieler[*m as usize].heimat as usize].koord).collect();
    let sektor = w.planeten[w.spieler[sid as usize].heimat as usize].koord.sektor;
    let plaetze = w.regeln.welt.plaetze_je_system;
    let bisher = sammelsystem(w, aid);
    let mut bestes: Option<(i64, u8)> = None;
    for n in 1..=w.regeln.welt.systeme_je_sektor {
        if w.planeten.iter().any(|p| p.heimat && p.koord.sektor == sektor && p.koord.system == n) { continue; }
        let eigene = (1..=plaetze).filter(|pos| w.belegung.get(&Koord::neu(sektor, n, *pos)).is_some_and(|pid| w.spieler[w.planeten[*pid as usize].besitzer as usize].allianz == Some(aid))).count() as i64;
        let frei = (1..=plaetze).filter(|pos| !w.belegung.contains_key(&Koord::neu(sektor, n, *pos))).count() as i64;
        // Die Dysonsphäre verlangt alle Plätze in Allianzhand (Kern `dyson_bedingungen`, „plaetze“): ein System mit fremden
        // Reichen taugt nicht (bot_lage 712, 60 Tage: vorher genügten 8 eigene oder freie Plätze, beide Projekte blieben
        // bei 5 bzw. 4 von 10 Mitgliedern und zwei fremden Planeten im System stehen). Ein schon verkündetes System, in dem
        // die Allianz siedelt, hält sie zehn Tage lang auch umkämpft – so lange gilt der Krieg um die Plätze (bot_krieg.rs);
        // bleibt es fremd besetzt, zieht sie in ein freies System um.
        let umkaempft_halten = bisher == Some((sektor, n)) && eigene > 0 && sammel_seit(w, aid).is_some_and(|t| w.zeit - t < 10 * TAG);
        if eigene + frei < plaetze as i64 && !umkaempft_halten { continue; }
        let weg: i64 = heimaten.iter().filter(|h| h.sektor == sektor).map(|h| (h.system as i64 - n as i64).abs()).sum();
        let nebel = w.xeno_planet(Koord::neu(sektor, n, 2));
        // Ein verkündetes System behält Vorrang, solange es taugt: Mitglieder haben dort schon gesiedelt.
        let wert = eigene * 40 + frei * 2 - weg + if nebel { 25 } else { 0 } + if bisher == Some((sektor, n)) { 60 } else { 0 };
        if bestes.map(|(b, _)| wert > b).unwrap_or(true) { bestes = Some((wert, n)); }
    }
    let Some((_, n)) = bestes else { return };
    let text = format!("Sammelsystem für unsere Dysonsphäre: System {sektor}:{n}. Wer eine Kolonie gründet, siedelt bitte dort; sobald wir dort vertreten sind, schlage ich das Projekt vor und wir zahlen gemeinsam ein.");
    if chat(w, sid, SAMMEL_BETREFF, text) { zaehle("allianz: sammelsystem verkündet"); }
}

/// Kleine Bünde lösen sich auf: wer nach Tag 6 noch mit höchstens zwei Partnern allein ist und eine größere Allianz
/// mit Platz kennt, in der er einen Partner hat, wechselt (dort lädt man ihn am nächsten Tag ein).
fn wechseln(w: &mut Welt, sid: SpielerId, aid: u32, tag: i64) {
    let Some(al) = w.allianzen.iter().find(|a| a.id == aid) else { return };
    if tag < 6 || al.mitglieder.len() > 3 || wuerfel(sid, 2, tag) >= 300 { return; }
    if w.ausbau.dyson.projekte.values().any(|p| p.allianz == aid && p.phase.offen()) { return; }
    let max = w.bot_allianz_max();
    let besser = w.allianzen.iter().filter(|a| a.id != aid && a.mitglieder.len() >= 5 && a.mitglieder.len() + a.eingeladen.len() < max)
        .map(|a| (allianz_passung(w, sid, a.id), a.name.clone())).filter(|(p, _)| *p > 12).max_by_key(|(p, _)| *p);
    let Some((_, ziel)) = besser else { return };
    chat(w, sid, "Wechsel", format!("Unser Bund bleibt zu klein für eine Dysonsphäre. Ich wechsle zur Allianz '{ziel}' und bleibe euch als Handelspartner verbunden."));
    if tu(w, sid, json!({"typ": "allianz_verlassen"})) { zaehle("allianz: gewechselt"); }
}
