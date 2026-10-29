//! Invasionsablauf v1 der Skriptbots: der Räuber erobert Kolonien mit Bombern, Kolonieschiff und
//! Bodentruppen (kampfkolonisieren); jeder Bot (außer Wehrlos) ruft bei einer Belagerung die
//! freigeschaltete Miliz aus, fliegt eigene Bodentruppen ein und verstärkt belagerte Verbündete am
//! Boden. Alles über dieselben Aktionen wie Spieler und Agenten. Welten ohne Invasion v1: nichts.
//!
//! 20.10.2026 (Karl: „Bots sollen bombardieren, invadieren, raiden selbst ausführen … keine Neulinge überrennen“):
//! In 30 Tagen Botlauf gab es 0 Bombardierungen und 0 Kampfkolonisationen, weil niemand den Bausatz vorbereitete
//! (Bomber erst ab Stufe V und Werft 7, Truppentransporter und Kaserne, mindestens 40 Landetruppen über der
//! Heimatgarnison, ein freies Kolonieschiff und ein freier Kolonieplatz) und die alte Zielwahl jeden bewaffneten
//! Frachter mitschickte und den Opferschutz nicht kannte. `vorbereiten` baut den Bausatz Schritt für Schritt,
//! `schlag` wählt das Ziel mit dem Kampfsimulator und respektiert den Opferschutz, `aufraeumen` ruft verwaiste
//! Blockadeflotten zurück (sie halten den Orbit sonst für immer und belegen einen Flottenplatz).

use super::{angriff_schaetzen, aufstiegs_ruecklage, baubar, bezahlbar, mitspieler, neuling, raub_erlaubt, sondenauftrag, zaehle, Bottyp, Opferschutz, SPAEH_SONDEN};
use kern::boden_katalog::ANZAHL;
use kern::typen::*;
use kern::welt::*;
use serde_json::{json, Value};

fn tu(w: &mut Welt, sid: SpielerId, a: Value) -> bool { w.handeln(sid, Rolle::Alle, &a).0 }

/// Garnison, die ein Bot zu Hause behält.
const RESERVE: i64 = 12;
/// Mindestgröße eines Angriffs am Boden (angreifende Bodeneinheiten).
const MIN_LANDUNG: i64 = 40;
/// Landetruppen, die `vorbereiten` über der Heimatgarnison ausbildet.
const LANDUNG_ZIEL: i64 = 80;
/// Mindestzahl Bomber für eine Invasion. Ein Bomber wirft 100 Bombenlast je Runde; zwei senken eine Kolonie mit
/// 40 Gebäudestufen (28.000 Strukturpunkte) in rund 20 Stunden auf 30 % – die Belagerung dauert danach 48 Stunden.
const MIN_BOMBER: i64 = 2;
/// Bomber, die `vorbereiten` anstrebt.
const BOMBER_ZIEL: i64 = 3;
/// Truppentransporter, die `vorbereiten` anstrebt (100 Plätze je Transporter).
const TRANSPORTER_ZIEL: i64 = 2;
/// Berichte, die für eine Invasion nicht älter sein dürfen.
const BERICHT_ALTER: i64 = 24 * STUNDE;

fn bodenstand(w: &Welt, pid: usize) -> [i64; ANZAHL] { w.ausbau.boden.bestand.get(&(pid as u32)).copied().unwrap_or_default() }
fn unterwegs_nach(w: &Welt, sid: SpielerId, ziel: Koord) -> bool {
    w.flotten.values().any(|f| f.besitzer == sid && f.ziel == ziel && f.zustand == Flottenzustand::Hinflug)
}
/// Truppen, die in `transporter` Truppentransporter passen (Reserve bleibt zu Hause).
fn verladen(b: &kern::kernwerte::BodenWerte, stand: &[i64; ANZAHL], transporter: i64, reserve: i64) -> [i64; ANZAHL] {
    let mut platz = transporter * b.transporter_plaetze;
    let mut rest_reserve = reserve;
    let mut out = [0; ANZAHL];
    for i in 0..ANZAHL {
        if b.angriff(i) == 0 { continue; }
        let behalten = rest_reserve.min(stand[i]);
        rest_reserve -= behalten;
        let n = (stand[i] - behalten).min(platz / b.plaetze(i));
        out[i] = n.max(0);
        platz -= out[i] * b.plaetze(i);
    }
    out
}
fn truppen_senden(w: &mut Welt, sid: SpielerId, start: usize, ziel: Koord, mission: &str, schiffe: Value, truppen: [i64; ANZAHL], tempo: f64) -> bool {
    let flug = json!({"typ":"flotte_senden","start":w.planeten[start].koord.to_string(),"ziel":ziel.to_string(),"mission":mission,"schiffe":schiffe,"geschwindigkeit":tempo});
    tu(w, sid, json!({"typ":"boden_senden","flug":flug,"truppen":truppen}))
}

/// Reaktion auf Belagerungen: eigene Planeten und Planeten von Verbündeten.
pub fn reaktion(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
    if !w.invasion_v1() || typ == Bottyp::Wehrlos || !w.spieler_aktiv(sid) { return; }
    let belagert: Vec<(usize, SpielerId)> = w.kolonisation.besetzungen.iter()
        .map(|(id, b)| (*id as usize, b.verteidiger))
        .filter(|(pid, v)| w.planeten[*pid].besitzer == *v && (*v == sid || w.verbuendet(sid, *v)))
        .collect();
    for (pid, verteidiger) in belagert {
        let ziel = w.planeten[pid].koord;
        if verteidiger == sid {
            let m = w.miliz_sicht(pid);
            let kosten = m["kosten"]["gesamt_legierung"].as_i64().unwrap_or(i64::MAX);
            if m["freigeschaltet"] == true && m["verfuegbar"].as_i64().unwrap_or(0) > 0
                && w.bestand_jetzt(pid)[Gut::Legierung.idx()] >= kosten * M {
                tu(w, sid, json!({"typ":"miliz_ausrufen","planet":ziel.to_string()}));
            }
        }
        if unterwegs_nach(w, sid, ziel) { continue; }
        // Eigene oder verbündete Bodentruppen von dem Planeten mit den meisten freien Truppen.
        let mission = if verteidiger == sid { "stationieren" } else { "verstaerken" };
        let quelle = w.spieler[sid as usize].planeten.iter().map(|p| *p as usize)
            .filter(|p| *p != pid && w.planeten[*p].blockade.is_none() && w.planeten[*p].einheiten[Einheit::Truppentransporter.idx()] > 0)
            .max_by_key(|p| (verladen(&w.regeln.kw().boden, &bodenstand(w, *p), w.planeten[*p].einheiten[Einheit::Truppentransporter.idx()], RESERVE).iter().sum::<i64>(), std::cmp::Reverse(*p)));
        let Some(q) = quelle else { continue };
        let transporter = w.planeten[q].einheiten[Einheit::Truppentransporter.idx()];
        let fracht = verladen(&w.regeln.kw().boden, &bodenstand(w, q), transporter, RESERVE);
        if fracht.iter().sum::<i64>() < 10 { continue; }
        let noetig = transporter_fuer(&w.regeln.kw().boden, &fracht);
        truppen_senden(w, sid, q, ziel, mission, json!({"truppentransporter":noetig}), fracht, 1.0);
    }
}

// ---------------------------------------------------------------------------------------------- Bestand

/// Kampfschiff im Sinn des Raubzugs und der Belagerung: Jäger, Kreuzer, Schlachtschiffe, Zerstörer, Schlachtkreuzer
/// und Bomber. Frachter, Sonden, Kolonie- und Arbeitsschiffe und Truppentransporter kämpfen nicht mit
/// (die alte Zielwahl schickte jeden Schiffstyp mit Angriffswert über 0 mit, also auch die Kleinen Transporter).
pub(super) fn kampfschiff(w: &Welt, e: Einheit) -> bool {
    e.ist_schiff() && w.regeln.einh(e).angriff > 0
        && w.regeln.flottendoktrin.get(&e).is_some_and(|d| matches!(d.klasse.as_str(), "jaeger" | "kreuzer" | "schlachter" | "bomber" | "zerstoerer" | "schlachtkreuzer"))
}
fn bomber(w: &Welt, pid: usize) -> i64 {
    (0..SCHIFFE).filter(|i| w.bombenlast(Einheit::ALLE[*i]) > 0).map(|i| w.planeten[pid].einheiten[i]).sum()
}
/// Kampfschiffe (mit oder ohne Bomber) auf dem Planeten als Gefechtsaufstellung.
pub(super) fn aufstellung(w: &Welt, pid: usize, mit_bomber: bool) -> [i64; EINHEITEN] {
    let mut ang = [0i64; EINHEITEN];
    for i in 0..SCHIFFE {
        let e = Einheit::ALLE[i];
        if kampfschiff(w, e) && (mit_bomber || w.bombenlast(e) == 0) { ang[i] = w.planeten[pid].einheiten[i]; }
    }
    ang
}
pub(super) fn verteidigungswert(w: &Welt, b: &Spionagebericht) -> Option<i64> {
    let r = &w.regeln;
    let volk = w.spieler[b.besitzer as usize].volk;
    let mut wert = 0;
    for (i, n) in b.schiffe.as_ref()?.iter().take(SCHIFFE).enumerate() { if r.einh(Einheit::ALLE[i]).angriff > 0 { wert += n * r.wert(&r.kosten_einheit(Einheit::ALLE[i], volk)); } }
    for (i, n) in b.verteidigung.as_ref()?.iter().take(EINHEITEN - SCHIFFE).enumerate() { wert += n * r.wert(&r.kosten_einheit(Einheit::ALLE[SCHIFFE + i], volk)); }
    Some(wert)
}
/// Gebäudestufen laut Bericht (Scanstärke 5), sonst eine mittlere Annahme.
fn gebaeudestufen(b: &Spionagebericht) -> i64 {
    b.gebaeude.as_ref().map(|g| g.iter().map(|n| *n as i64).sum::<i64>()).filter(|n| *n > 0).unwrap_or(25)
}
/// Sind für die Landung ein freier Kolonieplatz da (Kolonien plus laufende Gründungen unter dem Astrophysiklimit)?
fn kolonieplatz(w: &Welt, sid: SpielerId) -> bool {
    let laufend = w.flotten.values().filter(|f| f.besitzer == sid && matches!(f.mission, Mission::Kolonisieren | Mission::Kampfkolonisieren)
        && f.schiffe[Einheit::Kolonieschiff.idx()] + f.schiffe[Einheit::GrossesKolonieschiff.idx()] > 0
        && matches!(f.zustand, Flottenzustand::Hinflug | Flottenzustand::ImOrbit)).count();
    w.kolonien(sid) + laufend + 1 <= w.kolonien_erlaubt(sid)
}

/// Was auf der Heimatwelt für eine Landung bereitsteht.
struct Satz { kolonie: i64, transporter: i64, fracht: [i64; ANZAHL], truppen: i64, platz: bool, noetig: i64 }
/// Truppentransporter für eine Fracht (Truppenplätze aus den Kernwerten der Welt, Standard 100 je Transporter).
fn transporter_fuer(b: &kern::kernwerte::BodenWerte, fracht: &[i64; ANZAHL]) -> i64 {
    let je = b.transporter_plaetze.max(1);
    (fracht.iter().enumerate().map(|(i, n)| n * b.plaetze(i)).sum::<i64>() + je - 1) / je
}
impl Satz {
    fn lesen(w: &Welt, sid: SpielerId, pid: usize) -> Satz {
        let p = &w.planeten[pid];
        let transporter = p.einheiten[Einheit::Truppentransporter.idx()];
        let fracht = verladen(&w.regeln.kw().boden, &bodenstand(w, pid), transporter, RESERVE);
        Satz { kolonie: p.einheiten[Einheit::Kolonieschiff.idx()], transporter, fracht, truppen: fracht.iter().sum(), platz: kolonieplatz(w, sid),
            noetig: transporter_fuer(&w.regeln.kw().boden, &fracht) }
    }
    fn landung_moeglich(&self) -> bool { self.kolonie >= 1 && self.transporter >= 1 && self.truppen >= MIN_LANDUNG && self.platz }
    fn transporter_noetig(&self) -> i64 { self.noetig }
}

fn tempo_waehlen(w: &Welt, sid: SpielerId, pid: usize, ziel: Koord, flotte: &[i64; SCHIFFE]) -> Option<f64> {
    let deut = w.bestand_jetzt(pid)[Gut::Deuterium.idx()];
    [1.0, 0.8, 0.6, 0.4, 0.3].into_iter().find(|s| w.flugplan(sid, pid, ziel, flotte, milli(*s)).map(|p| 2 * p.treibstoff <= deut).unwrap_or(false))
}

// ---------------------------------------------------------------------------------------------- Vorbereitung

/// Einen Bauauftrag an die zweite Stelle der Bauschleife rücken (der laufende bleibt vorn, siehe `warteschlange_aendern`).
fn vorziehen(w: &mut Welt, sid: SpielerId, pid: usize, g: Gebaeude) {
    let Some(i) = w.planeten[pid].bauschleife.iter().rposition(|a| a.gebaeude == g) else { return };
    if i <= 1 { return; }
    let kennung = w.warteschlange_sicht(sid, "bau", pid)["kennung"].as_str().unwrap_or("").to_string();
    let koord = w.planeten[pid].koord.to_string();
    if tu(w, sid, json!({"typ":"warteschlange_aendern","planet":koord,"art":"bau","index":i,"ziel":1,"loeschen":false,"kennung":kennung})) {
        zaehle("invasion: ausbau vorgezogen");
    }
}

/// Baut den Bausatz für Bombardierung und Landung Schritt für Schritt aus: Kaserne 3 und Werft 7 auf der
/// Heimatwelt, Truppentransporter, Landetruppen, ab Stufe V Bomber und, wenn das Astrophysiklimit die Kolonien deckelt,
/// eine weitere Astrophysikstufe. Jede Bestellung nur aus dem Bestand über der Rücklage und nur, wenn die Flotte
/// (Unterhalt zehn Tage aus den Credits) tragbar bleibt. Gibt an, ob der Bausatz vollständig ist.
pub(super) fn vorbereiten(w: &mut Welt, sid: SpielerId, puffer: bool) -> bool {
    if !w.invasion_v1() || w.ausbau.industrie_version == 0 || !w.spieler_aktiv(sid) { return false; }
    let stufe = w.spieler[sid as usize].stufe;
    if stufe < 4 { return false; }
    let r = w.regeln.clone();
    let volk = w.spieler[sid as usize].volk;
    let pid = w.spieler[sid as usize].heimat as usize;
    let ks = w.planeten[pid].koord.to_string();
    // Der Bausatz gehört zur Rolle des Räubers und ist klein gegen seine Lager: nur die Rücklage für den nächsten
    // Aufstieg bleibt unberührt, nicht die für das nächste Ausbauziel (im Test blieb damit Werft 7 acht Tage aus).
    // Im Krieg (`puffer`) rüsten auch Ökonomen und Händler auf; sie lassen ein Viertel des Lagers unberührt.
    let frei = {
        let (b, rueck, grenze) = (w.bestand_jetzt(pid), aufstiegs_ruecklage(w, sid, pid), w.lagergrenze(pid));
        std::array::from_fn::<i64, GUETER, _>(|i| (b[i] - rueck[i] - if puffer { grenze[i] / 4 } else { 0 }).max(0))
    };
    let kann = |k: &[i64; GUETER]| bezahlbar(&frei, k);
    let schleife = |w: &Welt, g: Gebaeude| w.planeten[pid].bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;

    // 1. Gebäude: Kaserne 3 (Sturmpanzer, Truppentransporter) und Werft 7 (Bomber) schon ab Stufe IV: das Gebäude kostet wenig,
    // und die Bomber können gleich mit der Stufe V bestellt werden (die 30 Tage reichten sonst nicht für Aufbau und Schlag).
    let mut gebaut = false;
    for (g, soll, ab) in [(Gebaeude::Kaserne, 3u8, 4u8), (Gebaeude::Werft, 7u8, 4u8)] {
        if stufe < ab { continue; }
        let p = &w.planeten[pid];
        let stand = p.gebaeude[g.idx()] + schleife(w, g);
        // Die Bots füllen die Bauschleife (fünf Plätze) selbst mit zwei Aufträgen; der Ausbau darf sich dahinter einreihen
        // und rückt an die zweite Stelle, damit er nicht hinter jedem Wohnblock wartet.
        if stand >= soll || p.bauschleife.len() >= 4 || w.felder_belegt(pid) >= w.felder_gesamt(pid) { continue; }
        if r.geb(g).braucht.iter().any(|(b, st)| p.gebaeude[b.idx()] < *st) || !kann(&r.kosten_gebaeude(g, stand + 1)) { continue; }
        if tu(w, sid, json!({"typ":"bauen","planet":ks,"gebaeude":g.name()})) {
            zaehle("invasion: ausbau bestellt");
            vorziehen(w, sid, pid, g);
            gebaut = true;
            break;
        }
    }

    // 2. Truppentransporter und Bomber in der Werft.
    let in_werft = |w: &Welt, e: Einheit| -> i64 { w.planeten[pid].fertigung[0].iter().filter(|f| f.produkt == Produkt::Einheit(e)).map(|f| f.rest).sum() };
    let tragbar = |w: &Welt, e: Einheit, anzahl: i64| {
        let neu = anzahl * r.wert(&r.kosten_einheit(e, volk));
        let je_tag = mal(w.flottenwert(sid) + neu, r.wirtschaft.unterhalt_schiffe_je_tag);
        w.spieler[sid as usize].credits >= 10 * je_tag
    };
    let mut bestellt = false;
    let queue_voll = w.planeten[pid].fertigung[0].len() >= 4;
    for (e, ziel, ab) in [(Einheit::Truppentransporter, TRANSPORTER_ZIEL, 4u8), (Einheit::Bomber, BOMBER_ZIEL, 5u8)] {
        if stufe < ab || queue_voll || bestellt { continue; }
        let hat = w.planeten[pid].einheiten[e.idx()] + in_werft(w, e)
            + w.flotten.values().filter(|f| f.besitzer == sid).map(|f| f.schiffe[e.idx()]).sum::<i64>();
        if hat >= ziel || !baubar(w, sid, pid, e) { continue; }
        let kosten = r.kosten_einheit(e, volk);
        let n = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| frei[g] / kosten[g]).min().unwrap_or(0).min(ziel - hat);
        if n >= 1 && tragbar(w, e, n) && tu(w, sid, json!({"typ":"fertigen","planet":ks,"einheit":e.name(),"anzahl":n})) {
            zaehle(if e == Einheit::Bomber { "invasion: bomber bestellt" } else { "invasion: truppentransporter bestellt" });
            bestellt = true;
        }
    }

    // 3. Landetruppen: Sturmpanzer sind teuer, Sturminfanterie billig und schnell ausgebildet.
    let stand = bodenstand(w, pid);
    let in_ausbildung: i64 = w.ausbau.boden.auftraege.get(&(pid as u32)).map(|q| q.iter().map(|a| a.rest).sum()).unwrap_or(0);
    let bereit: i64 = (0..ANZAHL).filter(|i| w.regeln.kw().boden.angriff(*i) > 0).map(|i| stand[i]).sum::<i64>() + in_ausbildung;
    let queue_n = w.ausbau.boden.auftraege.get(&(pid as u32)).map(|q| q.len()).unwrap_or(0);
    if w.planeten[pid].gebaeude[Gebaeude::Kaserne.idx()] >= 1 && bereit < RESERVE + LANDUNG_ZIEL && queue_n < 6 && !w.werk_ruht(pid, Gebaeude::Kaserne) {
        let kosten = w.produktionskosten(pid, Gebaeude::Kaserne, w.regeln.kw().boden.kosten(0));
        let noetig = RESERVE + LANDUNG_ZIEL - bereit;
        let n = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| frei[g] / 2 / kosten[g]).min().unwrap_or(0).min(noetig).min(40);
        // Nicht stückweise bestellen (der erste Messlauf zählte 507 Kleinaufträge): erst ab acht Mann oder dem Rest.
        if n >= noetig.min(8) && n >= 1 && tu(w, sid, json!({"typ":"boden_bauen","planet":ks,"art":kern::boden::NAMEN[0],"anzahl":n})) { zaehle("invasion: truppen bestellt"); }
    }

    // 4. Das Astrophysiklimit deckelt die Kolonien: ohne freien Platz keine Landung. Eine Stufe einreihen, sobald der
    // übrige Bausatz steht (ein frühes Kolonieschiff würde die Kolonisierung freier Plätze vorher verbrauchen).
    let satz = Satz::lesen(w, sid, pid);
    let ohne_schiff = bomber(w, pid) >= MIN_BOMBER && satz.transporter >= 1 && satz.truppen >= MIN_LANDUNG;
    let sp = &w.spieler[sid as usize];
    // Kolonieforschung der Welt: laufende Welten Astrophysik (bis Stufe 14), neue Welten Terraforming (bis zur Höchstzahl).
    let kf = w.kolonie_forschung();
    if stufe >= 5 && ohne_schiff && !satz.platz && sp.forschung_schlange.is_empty() && sp.forschung_aktiv.as_ref().is_none_or(|a| a.forschung != kf)
        && w.kolonien(sid) >= w.kolonien_erlaubt(sid) && w.kolonien_erlaubt(sid) < w.kolonien_grenze().min(14)
        && w.planeten[pid].gebaeude[Gebaeude::Labor.idx()] >= r.forsch(kf).labor {
        if tu(w, sid, json!({"typ":"forschen","forschung":kf.name()})) { zaehle("invasion: astrophysik eingereiht"); }
    }
    let _ = gebaut;
    bomber(w, pid) >= MIN_BOMBER && satz.landung_moeglich()
}

// ---------------------------------------------------------------------------------------------- Angriff

/// Wozu der Schlag dient: Räuber erobern (Kampfkolonisieren), Kriegsparteien bombardieren auch ohne Landung.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Modus { Raub, Krieg }

/// Ruhe am Ort: nirgends in den letzten `pause_stunden` gekämpft (Opferschutz, wenn aktiv).
fn ort_ruhig(w: &Welt, schutz: &Opferschutz, ort: Koord) -> bool {
    let pause = schutz.pause_stunden as i64 * STUNDE;
    !(schutz.aktiv && w.kampfberichte.iter().rev().take_while(|k| w.zeit - k.zeit < pause).any(|k| k.ort == ort))
}

/// Räuber: Kampfkolonisation gegen die schwächste frisch ausgespähte fremde Kolonie (Aufruf früh im Bot-Zug, damit
/// Spähflüge und Raubzüge der Räuber die Flottenplätze nicht vorher belegen).
pub fn angriff(w: &mut Welt, sid: SpielerId, schutz: &Opferschutz) {
    if !w.invasion_v1() || w.spieler[sid as usize].stufe < 4 { return; }
    aufraeumen(w, sid, false);
    vorbereiten(w, sid, false);
    let _ = schlag(w, sid, schutz, Modus::Raub, &|w, ziel| !w.verbuendet(sid, ziel));
}

/// Eine Invasionsflotte starten, wenn der Bausatz bereitsteht und ein Ziel die Simulation besteht.
/// `feind` entscheidet, wer als Ziel in Frage kommt. Gibt das Ziel zurück, wenn eine Flotte gestartet wurde.
pub(super) fn schlag(w: &mut Welt, sid: SpielerId, schutz: &Opferschutz, modus: Modus, feind: &dyn Fn(&Welt, SpielerId) -> bool) -> Option<Koord> {
    if !w.invasion_v1() || !w.spieler_aktiv(sid) || w.spieler[sid as usize].stufe < 4 { return None; }
    let pid = w.spieler[sid as usize].heimat as usize;
    if w.planeten[pid].blockade.is_some() { return None; }
    if w.flotten.values().filter(|f| f.besitzer == sid).count() >= w.flottenplaetze(sid) { return None; }
    let satz = Satz::lesen(w, sid, pid);
    // Zweite Stufe: eine eigene Bombardierung hält den Orbit; Kolonieschiff und Truppen folgen und schließen sich an.
    if satz.landung_moeglich() { if let Some(k) = landung_nachschieben(w, sid, pid, &satz) { return Some(k); } }
    if w.flotten.values().any(|f| f.besitzer == sid && matches!(f.mission, Mission::Kampfkolonisieren | Mission::Bombardieren) && f.zustand != Flottenzustand::Rueckflug) {
        return None;
    }
    let bomber_n = bomber(w, pid);
    if bomber_n < MIN_BOMBER { if modus == Modus::Raub { zaehle("invasion: zu wenig bomber"); } return None; }
    let landung = satz.landung_moeglich();
    // Räuber erobern nur; im Krieg genügt die Bombardierung. Fehlt dem Räuber nur noch das Kolonieschiff und kommt es
    // bald (freier Kolonieplatz oder eine Astrophysikstufe unterwegs), beginnt die Bombardierung schon: das
    // Kolonieschiff folgt und schließt sich an (`landung_nachschieben`).
    if modus == Modus::Raub && !landung && !(satz.transporter >= 1 && satz.truppen >= MIN_LANDUNG && nachschub_naht(w, pid, &satz)) {
        zaehle("invasion: bausatz unvollständig");
        return None;
    }
    let ang = aufstellung(w, pid, true);
    let r = w.regeln.clone();
    let volk = w.spieler[sid as usize].volk;
    let eskorte: i64 = (0..SCHIFFE).map(|i| ang[i] * r.wert(&r.kosten_einheit(Einheit::ALLE[i], volk))).sum();
    let jetzt = w.zeit;
    let mut ziele: Vec<(i64, Koord, Spionagebericht)> = w.spieler[sid as usize].berichte.iter()
        .filter(|b| jetzt - b.zeit < BERICHT_ALTER && b.besitzer != sid && !w.verbuendet(sid, b.besitzer) && feind(w, b.besitzer))
        .filter(|b| !w.vertrag_zwischen(sid, b.besitzer, Vertragsart::Nichtangriffspakt))
        .filter(|b| !neuling(w, schutz, b.besitzer) && !unterwegs_nach(w, sid, b.ziel))
        // Mitspieler (keine Skriptbots) werden nie aus Raubgier erobert, nur im Krieg, den sie selbst ausgelöst haben.
        .filter(|b| !(modus == Modus::Raub && mitspieler(schutz, b.besitzer)))
        .filter(|b| if modus == Modus::Raub { raub_erlaubt(w, schutz, sid, b.besitzer, b.ziel) } else { ort_ruhig(w, schutz, b.ziel) })
        .filter_map(|b| {
            let zp = *w.belegung.get(&b.ziel)? as usize;
            let z = &w.planeten[zp];
            if z.heimat || z.besitzer != b.besitzer || z.blockade.is_some() { return None; }
            let v = verteidigungswert(w, b)?;
            (eskorte >= 2 * v).then_some((v, b.ziel, b.clone()))
        }).collect();
    if ziele.is_empty() {
        zaehle("invasion: kein ziel");
        if modus == Modus::Raub { kolonie_ausspaehen(w, sid, pid, schutz, feind); }
        return None;
    }
    ziele.sort_by_key(|(v, k, _)| (*v, *k));
    // Die drei schwächsten durchrechnen; das Ziel mit den meisten Gebäudestufen unter denen mit sicherem Sieg.
    let mut bestes: Option<(i64, i64, Koord)> = None;
    for (v, ziel, b) in ziele.into_iter().take(3) {
        let Some((chance, _, _)) = angriff_schaetzen(w, sid, &b, &ang) else { continue };
        if chance < 75 { zaehle("invasion: siegchance < 75"); continue; }
        let wert = gebaeudestufen(&b);
        if bestes.map(|(bw, bv, _)| wert > bw || (wert == bw && v < bv)).unwrap_or(true) { bestes = Some((wert, v, ziel)); }
    }
    let Some((_, _, ziel)) = bestes else { return None };

    let mut schiffe = serde_json::Map::new();
    let mut flotte = [0i64; SCHIFFE];
    for i in 0..SCHIFFE {
        if ang[i] > 0 { schiffe.insert(Einheit::ALLE[i].name().into(), json!(ang[i])); flotte[i] = ang[i]; }
    }
    if landung {
        let noetig = satz.transporter_noetig();
        schiffe.insert("kolonieschiff".into(), json!(1));
        schiffe.insert("truppentransporter".into(), json!(noetig));
        flotte[Einheit::Kolonieschiff.idx()] = 1;
        flotte[Einheit::Truppentransporter.idx()] = noetig;
    }
    let Some(tempo) = tempo_waehlen(w, sid, pid, ziel, &flotte) else { zaehle("invasion: treibstoff fehlt"); return None };
    let ok = if landung {
        truppen_senden(w, sid, pid, ziel, "kampfkolonisieren", Value::Object(schiffe), satz.fracht, tempo)
    } else {
        let ks = w.planeten[pid].koord.to_string();
        tu(w, sid, json!({"typ":"flotte_senden","start":ks,"ziel":ziel.to_string(),"mission":"bombardieren","schiffe":Value::Object(schiffe),"geschwindigkeit":tempo}))
    };
    zaehle(match (ok, landung) {
        (true, true) => "invasion: kampfkolonisieren gestartet",
        (true, false) => "invasion: bombardieren gestartet",
        (false, _) => "invasion: start abgelehnt",
    });
    ok.then_some(ziel)
}

/// Der Bausatz steht, aber kein Ziel hat einen frischen Bericht: die nächste bekannte fremde Kolonie ausspähen (die Späh-
/// flüge des Räubers gelten dem Nachbarn überhaupt, meist den Heimatwelten; im Messlauf blieben 583 Züge ohne Ziel).
/// Bekannt heißt: das eigene Wissen (`planetenwissen`) nennt den Besitzer.
fn kolonie_ausspaehen(w: &mut Welt, sid: SpielerId, pid: usize, schutz: &Opferschutz, feind: &dyn Fn(&Welt, SpielerId) -> bool) {
    let frei = w.flottenplaetze(sid) > w.flotten.values().filter(|f| f.besitzer == sid).count();
    if !frei || w.planeten[pid].einheiten[Einheit::Spionagesonde.idx()] < SPAEH_SONDEN { return; }
    let heim = w.planeten[pid].koord;
    let jetzt = w.zeit;
    let sp = &w.spieler[sid as usize];
    let mut kand: Vec<(i64, Koord)> = w.planeten.iter()
        .filter(|z| !z.heimat && z.besitzer != sid && z.koord.sektor == heim.sektor && (z.koord.system as i64 - heim.system as i64).abs() <= 15 && z.blockade.is_none())
        .filter(|z| !w.verbuendet(sid, z.besitzer) && feind(w, z.besitzer) && !neuling(w, schutz, z.besitzer) && !mitspieler(schutz, z.besitzer))
        .filter(|z| !sp.berichte.iter().any(|b| b.ziel == z.koord && jetzt - b.zeit < 12 * STUNDE))
        .filter(|z| !w.flotten.values().any(|f| f.besitzer == sid && f.ziel == z.koord && matches!(f.mission, Mission::Spionage | Mission::SystemErkunden)))
        .map(|z| (w.entfernung(heim, z.koord), z.koord)).collect();
    kand.sort();
    let Some(ziel) = kand.into_iter().take(6).map(|(_, k)| k).find(|k| w.planetenwissen(sid, *k)["spieler"].is_string() && w.system_erfasst(sid, *k)) else { return };
    if let Some(a) = sondenauftrag(w, sid, pid, ziel, "spionage", SPAEH_SONDEN) {
        if tu(w, sid, a) { zaehle("invasion: kolonie ausgespäht"); }
    }
}

/// Kommt das fehlende Kolonieschiff bald? Ein Kolonieplatz ist frei (dann baut `kolonisieren` das Schiff) oder wird
/// durch eine laufende oder eingereihte Astrophysikstufe frei.
fn nachschub_naht(w: &Welt, pid: usize, satz: &Satz) -> bool {
    let sp = &w.spieler[w.planeten[pid].besitzer as usize];
    satz.kolonie >= 1 || satz.platz
        || sp.forschung_aktiv.as_ref().is_some_and(|a| a.forschung == w.kolonie_forschung())
        || sp.forschung_schlange.iter().any(|(f, _, _)| *f == w.kolonie_forschung())
}

/// Kolonieschiff und Landetruppen zu einer eigenen Bombardierung nachschicken: sie hält den Orbit schon, oder ihre
/// Flotte ist noch unterwegs und das langsamere Landungsgeschwader trifft erst nach ihr ein (sonst nähme `kolonisieren` das
/// frisch gebaute Kolonieschiff mit und gründete damit eine freie Kolonie: im zweiten Messlauf blieb deshalb jede Landung aus).
fn landung_nachschieben(w: &mut Welt, sid: SpielerId, pid: usize, satz: &Satz) -> Option<Koord> {
    let noetig = satz.transporter_noetig();
    let mut flotte = [0i64; SCHIFFE];
    flotte[Einheit::Kolonieschiff.idx()] = 1;
    flotte[Einheit::Truppentransporter.idx()] = noetig;
    let kandidaten: Vec<(Koord, SimZeit)> = w.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Bombardieren)
        .filter_map(|f| {
            let zp = *w.belegung.get(&f.ziel)? as u32;
            if w.kolonisation.besetzungen.contains_key(&zp) { return None; }
            if w.flotten.values().any(|x| x.besitzer == sid && x.mission == Mission::Kampfkolonisieren && x.ziel == f.ziel && x.zustand != Flottenzustand::Rueckflug) { return None; }
            match f.zustand {
                Flottenzustand::ImOrbit => w.ausbau.boden.invasion.bombardements.get(&zp).is_some_and(|b| b.flotte == f.id).then_some((f.ziel, 0)),
                Flottenzustand::Hinflug => Some((f.ziel, f.ankunft)),
                Flottenzustand::Rueckflug => None,
            }
        }).collect();
    for (ziel, ankunft) in kandidaten {
        let Some(tempo) = tempo_waehlen(w, sid, pid, ziel, &flotte) else { continue };
        // Das Geschwader muss nach der Bombardierungsflotte ankommen, damit der Orbit schon gewonnen ist.
        let Ok(plan) = w.flugplan(sid, pid, ziel, &flotte, milli(tempo)) else { continue };
        if w.zeit + plan.dauer < ankunft + 30 * MINUTE { continue; }
        if truppen_senden(w, sid, pid, ziel, "kampfkolonisieren", json!({"kolonieschiff":1,"truppentransporter":noetig}), satz.fracht, tempo) {
            zaehle("invasion: landung nachgeschickt");
            return Some(ziel);
        }
    }
    None
}

/// Verwaiste Blockadeflotten zurückrufen: eine eigene Bombardier- oder Kampfkolonisierflotte im Orbit, deren
/// Bombardierung oder Belagerung nicht mehr läuft (Besatzung abgebrochen, Orbit verloren), und eine fertig bombardierende
/// Flotte, auf die bis 40 Stunden nach der Ankunft keine Landung folgte (die Bombardierung selbst dauert 12 bis 24 Stunden)
/// oder deren Krieg vorbei ist (`ende`).
pub(super) fn aufraeumen(w: &mut Welt, sid: SpielerId, ende: bool) {
    if !w.invasion_v1() { return; }
    let rufen: Vec<FlottenId> = w.flotten.values()
        .filter(|f| f.besitzer == sid && matches!(f.mission, Mission::Bombardieren | Mission::Kampfkolonisieren) && f.zustand == Flottenzustand::ImOrbit && w.zeit - f.ankunft > 2 * STUNDE)
        .filter(|f| {
            let Some(zp) = w.belegung.get(&f.ziel).map(|p| *p as u32) else { return true };
            let bomb = w.ausbau.boden.invasion.bombardements.get(&zp).filter(|b| b.flotte == f.id);
            let belagert = w.kolonisation.besetzungen.get(&zp).is_some_and(|b| b.flotte == f.id);
            if belagert { return false; }
            match bomb {
                None => true,
                Some(b) => f.mission == Mission::Bombardieren && ((b.abgeschlossen && w.zeit - f.ankunft > 40 * STUNDE) || ende),
            }
        }).map(|f| f.id).collect();
    for fid in rufen {
        if tu(w, sid, json!({"typ":"flotte_zurueckrufen","flotte":fid})) { zaehle("invasion: blockade zurückgerufen"); }
    }
}
