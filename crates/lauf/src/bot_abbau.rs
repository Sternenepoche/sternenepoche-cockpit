//! Abbau der Skriptbots: Trümmerfelder mit Recyclern bergen, Asteroidengürtel und natürliche Rohstoffringe mit
//! Bergbauschiffen abbauen (Karl 20.10.2026: „Trümmerfelder abbauen und von Asteroidengürteln unterscheiden“).
//! Alles über dieselben Aktionen wie Spieler und Agenten.
//!
//! Was der Kern hergibt (geprüft am 20.10.2026):
//! - Ein Trümmerfeld liegt auf einem Planetenplatz (Position 1 bis 12) und steht in `Welt::truemmer`. Nur die Mission
//!   `recyceln` holt es ab, nur mit Recyclern, und nie auf Position 0.
//! - Der Asteroidengürtel ist Position 0 eines Systems, aber nicht jedes System hat einen (in Online-Welten jedes
//!   fünfte, zufällig); dazu kommt in jedem fünften System ein natürlicher Rohstoffring auf Position 5 bis 8.
//!   Beides baut nur die Mission `abbau` mit Bergbauschiffen ab; daraus entsteht nie ein Trümmerfeld.
//! - Sichtbar sind Trümmerfelder, Gürtel und Ringe nur in erfassten Systemen (eigene Planeten oder eine Systemsonde).
//!
//! Wer es tut: jeder Bot ab Stufe III von der Heimatwelt aus. Der Räuber birgt zuerst, was seine Kämpfe hinterlassen
//! haben, der Igel und alle anderen Verteidiger ihre Trümmer vor der eigenen Tür, dazu jeder die natürlichen Bergungsfelder
//! seiner erfassten Systeme. Gürtel und Ringe bauen die Bots ab, die den Platz dafür haben (vier Flottenplätze). Wer nichts
//! Lohnendes kennt, schickt gelegentlich eine Systemsonde in ein neues System seines Sektors. Zufall gibt es nur aus dem
//! Startwert der Welt (deterministisch, aber nicht vorhersagbar).
//!
//! Was ein Flug kostet und wann er lohnt: Treibstoff für Hin- und Rückflug aus dem Tank der Heimat, mindestens das Achtfache
//! des Treibstoffwerts als Ertrag, Ladung höchstens so viel wie die Kapazität der Schiffe und das Lager der Heimat hergeben.
//! Ein Bot hält höchstens einen Bergungs- und einen Abbauflug zugleich und lässt einen Flottenplatz frei (nur eigene
//! Kampftrümmer dürfen ihn nehmen). Schiffe kauft er erst ab Stufe IV, nur aus dem Bestand über den Rücklagen von
//! `bots.rs` (nächstes Bauziel, Aufstieg beim Sparen) plus Forschung und Kolonieschiff, nur wenn ein Ziel lohnt und
//! nie mehr, als die Stufe vorsieht. Recycler, die `kolonisieren` für das frühe Habitatmodul braucht, bleiben daheim.

use super::*;
use std::collections::BTreeSet;

/// Ein Flottenplatz bleibt für Kolonisieren, Logistik und Spionage frei; nur eigene Kampftrümmer dürfen ihn nehmen
/// (20.10.2026: Räuber füllen ihre Plätze mit Spähflügen und Angriffen, ohne diese Ausnahme birgt keiner seine Beute).
const PLATZ_RESERVE: i64 = 1;
/// Kleinstes Trümmerfeld (Erz plus Kristall, ganze Einheiten), für das sich ein Recycler lohnt.
const FELD_MIN: i64 = 3_000;
/// Längste einfache Anreise zu einem Trümmerfeld und zu einem Gürtel/Ring (Sekunden).
const ANREISE_BERGUNG: i64 = 8 * STUNDE;
const ANREISE_BERGBAU: i64 = 12 * STUNDE;
/// Eigene Kämpfe dieser Zeit gelten als „eigene Kampftrümmer“.
const KAMPF_FRIST: i64 = 4 * TAG;
/// Der Flug muss mindestens so viel mehr wert sein als sein Treibstoff.
const WERT_ZU_TREIBSTOFF: i64 = 8;
/// Mindestanteil der Ladung, der ins Lager passt; sonst bleibt das Schiff daheim.
const PLATZ_MIN: f64 = 0.5;
/// Zusätzlicher Treibstoff, der nach dem Flug im Tank der Heimat bleibt.
const SPRIT_RESERVE: i64 = 200 * M;
/// So viele Aufträge dürfen schon in der Werftschleife stehen, wenn der Bot ein Abbauschiff bestellt.
const SCHLEIFE_MAX: usize = 2;
/// Schiffe für den Abbau kauft ein Bot erst ab dieser Stufe. Auf Stufe III braucht die Wirtschaft jedes Erz und jeden
/// Kristall für Wohnraum, Forschung und die Bedingungen von Stufe IV; vorhandene Recycler und Bergbauschiffe (etwa die
/// Recycler fürs Habitatmodul) fliegen schon ab Stufe III. 20.10.2026 (ablauf_naiv, vier Völker, 55 Tage): mit Käufen
/// auf Stufe III erreichte der langsamste Volk Stufe IV einen Tag später (21,7 statt 20,7 Tage, über der 15-Prozent-Grenze
/// des Tests); ab Stufe IV kaufen ändert die Stufe-IV-Zeiten gar nicht, Stufe V kam im Schnitt früher.
const KAUF_AB_STUFE: u8 = 4;
/// Flottenplätze, die ein Bot mindestens braucht, bevor er Recycler kauft (Bergungsflüge dauern nur Stunden).
const PLAETZE_FUER_BERGUNG: usize = 3;
/// Flottenplätze für Bergbauschiffe: ein Abbauflug hält seinen Platz 10 bis 50 Stunden. Die Logistik fliegt mit zwei Plätzen
/// und verlangt für jeden Start zwei freie; ein dritter Platz für den Abbau ließe ihr nur einen Flug zugleich, einer
/// muss für Kolonisieren und Spionage frei bleiben. 20.10.2026 (bot_lage 712/2026, Tag 30): mit Abbau schon auf drei
/// Plätzen (244 und 248 Abbauflüge) flogen 34 und 37 Prozent weniger Transporte, und Stufe V kam im Median 2,1 und
/// 0,7 Tage später; auf vier Plätzen (Computertechnik 3, das haben vor allem Räuber) bleibt die Logistik fast unberührt.
const PLAETZE_FUER_ABBAU: usize = 4;

/// Ein geplanter Flug.
struct Auftrag {
    ziel: Koord,
    /// Eingesetzte Schiffe (Bedarf des Ziels, begrenzt durch die Stufe und bei Gürtel und Ring durch das Lager).
    anzahl: i64,
    tempo: f64,
    halte_stunden: i64,
    punkte: f64,
    eigen: bool,
}

/// Lager und Tank der Heimatwelt, einmal je Planung gelesen.
struct Lager {
    grenze: [i64; GUETER],
    bestand: [i64; GUETER],
}

impl Lager {
    fn lesen(w: &Welt, pid: usize) -> Lager {
        Lager { grenze: w.lagergrenze(pid), bestand: w.bestand_jetzt(pid) }
    }

    /// Anteil des Ertrags, der noch ins Lager der Heimatwelt passt (ein Viertel darf darüber hinaus ankommen: der Bot
    /// verkauft oder verbraucht es bis zur Rückkehr).
    fn platzfaktor(&self, ertrag: &[i64; GUETER]) -> f64 {
        let (mut nutz, mut summe) = (0i64, 0i64);
        for g in [Gut::Erz, Gut::Kristall] {
            let i = g.idx();
            if ertrag[i] <= 0 { continue; }
            nutz += ertrag[i].min((self.grenze[i] - self.bestand[i]).max(0) + ertrag[i] / 4);
            summe += ertrag[i];
        }
        if summe == 0 { 0.0 } else { nutz as f64 / summe as f64 }
    }

    /// Freier Raum für Erz und Kristall zusammen.
    fn raum(&self) -> i64 {
        [Gut::Erz, Gut::Kristall].iter().map(|g| (self.grenze[g.idx()] - self.bestand[g.idx()]).max(0)).sum()
    }
}

/// Gleichverteilte Zahl in [0, 1) aus Startwert, Spieler und zwei Schlüsseln (SplitMix64): bei gleicher Lage
/// gleiche Entscheidung, aber zwischen Bots, Orten und Zeitfenstern verschieden.
fn wuerfel(w: &Welt, sid: SpielerId, a: u64, b: u64) -> f64 {
    let mut x = w.startwert ^ (u64::from(sid) << 40) ^ a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

fn ort_schluessel(k: Koord) -> u64 {
    (u64::from(k.sektor) << 16) | (u64::from(k.system) << 8) | u64::from(k.position)
}

fn frei(w: &Welt, sid: SpielerId) -> i64 {
    w.flottenplaetze(sid) as i64 - w.flotten.values().filter(|f| f.besitzer == sid).count() as i64
}

/// Schiffe einer Art: (daheim auf der Heimatwelt, daheim plus unterwegs plus bestellt).
fn bilanz(w: &Welt, sid: SpielerId, pid: usize, e: Einheit) -> (i64, i64) {
    let p = &w.planeten[pid];
    let unterwegs: i64 = w.flotten.values().filter(|f| f.besitzer == sid).map(|f| f.schiffe[e.idx()]).sum();
    let bestellt: i64 = p.fertigung[0].iter().filter(|f| f.produkt == Produkt::Einheit(e)).map(|f| f.rest).sum();
    (p.einheiten[e.idx()], p.einheiten[e.idx()] + unterwegs + bestellt)
}

/// Sieht der Spieler diesen Platz? Online-Regeln: nur erfasste Systeme; ältere Welten: der eigene Sektor.
fn sichtbar(w: &Welt, sid: SpielerId, k: Koord) -> bool {
    if w.aufklaerungsregeln() {
        w.system_erfasst(sid, k)
    } else {
        w.spieler[sid as usize].planeten.iter().any(|p| w.planeten[*p as usize].koord.sektor == k.sektor)
    }
}

/// Wartet eigene Kampfbeute auf Bergung (Recycler daheim, keiner unterwegs)? Dann behält die Aufklärung den letzten
/// Flottenplatz frei – sonst belegte Koloniespähen den einzigen Platz pausenlos (Test eigene_kampftruemmer, 22.10.2026).
pub(super) fn bergung_wartet(w: &Welt, sid: SpielerId) -> bool {
    let pid = w.spieler[sid as usize].heimat as usize;
    if w.planeten[pid].einheiten[Einheit::Recycler.idx()] < 1
        || w.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::Recyceln) { return false; }
    eigene_kampforte(w, sid).iter().any(|k| sichtbar(w, sid, *k) && w.truemmer.get(k).is_some_and(|t| t[0] + t[1] >= FELD_MIN * M))
}

/// Orte, an denen der Spieler in den letzten Tagen selbst gekämpft hat (aus den eigenen Kampfberichten).
fn eigene_kampforte(w: &Welt, sid: SpielerId) -> BTreeSet<Koord> {
    w.kampfberichte.iter().rev().take_while(|b| w.zeit - b.zeit < KAMPF_FRIST)
        .filter(|b| b.angreifer == sid || b.verteidiger.contains(&sid) || b.angreifer_gruppen.iter().any(|g| g.spieler == sid))
        .map(|b| b.ort).collect()
}

/// Ladekapazität eines Schiffs dieser Art für den Spieler (Volk, Logistik), in Tausendsteln.
fn kapazitaet_je(w: &Welt, sid: SpielerId, pid: usize, e: Einheit) -> i64 {
    let mut s = [0i64; SCHIFFE];
    s[e.idx()] = 1;
    w.flugplan(sid, pid, w.planeten[pid].koord, &s, 1000).map(|p| p.kapazitaet).unwrap_or(0)
}

/// Schnellste Fahrt, deren Hin- und Rückflug der Tank der Heimat trägt: (Dauer, Treibstoff einfach, Tempo).
fn fahrt(w: &Welt, sid: SpielerId, pid: usize, deut: i64, ziel: Koord, schiffe: &[i64; SCHIFFE]) -> Option<(i64, i64, f64)> {
    [1.0, 0.8, 0.6, 0.4].into_iter().find_map(|s| {
        let plan = w.flugplan(sid, pid, ziel, schiffe, milli(s)).ok()?;
        (2 * plan.treibstoff + SPRIT_RESERVE <= deut).then_some((plan.dauer, plan.treibstoff, s))
    })
}

/// Wert des Treibstoffs für Hin- und Rückflug.
fn spritwert(w: &Welt, treibstoff: i64) -> i64 {
    let mut s = [0i64; GUETER];
    s[Gut::Deuterium.idx()] = 2 * treibstoff;
    w.regeln.wert(&s)
}

/// Recycler, die `kolonisieren` für frühe Habitatmodule umbauen will; sie bleiben daheim.
fn recycler_fuer_kolonie(w: &Welt, sid: SpielerId, pid: usize) -> i64 {
    let r = &w.regeln;
    let p = &w.planeten[pid];
    if w.ausbau.industrie_version == 0 || p.gebaeude[Gebaeude::Orbitalwerft.idx()] > 0 { return 0; }
    let unterwegs = w.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren).count();
    let fuer_stufe_v = r.stufen.last().map(|s| s.kolonien as usize).unwrap_or(0);
    let fehlend = (w.kolonien_erlaubt(sid).max(fuer_stufe_v) as i64 - w.kolonien(sid) as i64 - unterwegs as i64).max(0);
    let je_schiff = r.einh(Einheit::Kolonieschiff).kosten.get(&Gut::Habitatmodul).copied().unwrap_or(0);
    let hat = ganz(w.bestand_jetzt(pid)[Gut::Habitatmodul.idx()])
        + p.fertigung.iter().flatten().filter(|f| f.produkt == Produkt::Bauteil(Gut::Habitatmodul)).map(|f| f.rest).sum::<i64>();
    (je_schiff * fehlend - hat).clamp(0, 2)
}

/// Systemreichtum (Erz, Kristall in Promille) so, wie der Spieler ihn kennt: in eigenen Systemen verrät es die
/// eigene Förderung, sonst Scan oder Erkundung eines Platzes im System, sonst der Durchschnitt.
fn reichtum(w: &Welt, sid: SpielerId, k: Koord) -> (i64, i64) {
    let Some(sys) = w.system(k) else { return (1050, 1050) };
    let sp = &w.spieler[sid as usize];
    if sp.planeten.iter().any(|p| { let q = w.planeten[*p as usize].koord; q.sektor == k.sektor && q.system == k.system }) {
        return (sys.reich_erz, sys.reich_kristall);
    }
    for pos in 1..=w.regeln.welt.plaetze_je_system {
        let q = Koord::neu(k.sektor, k.system, pos);
        if let Some(s) = w.scans.get(&(sid, q)) {
            return ((s.erz[0] + s.erz[1]) / 2, (s.kristall[0] + s.kristall[1]) / 2);
        }
        if let Some(e) = sp.erkundet.get(&q) { return (e.reich_erz, e.reich_kristall); }
    }
    (1050, 1050)
}

/// Bestes Trümmerfeld für höchstens `max_schiffe` Recycler (mindestens ein fiktives Schiff, damit auch ohne Schiff
/// klar wird, ob sich eines lohnt).
fn bergung_planen(w: &Welt, sid: SpielerId, pid: usize, max_schiffe: i64) -> Option<Auftrag> {
    let cap = kapazitaet_je(w, sid, pid, Einheit::Recycler);
    if cap <= 0 { return None; }
    let lager = Lager::lesen(w, pid);
    let eigene = eigene_kampforte(w, sid);
    let fenster = (w.zeit / (6 * STUNDE)) as u64;
    let mut bestes: Option<Auftrag> = None;
    for (k, t) in &w.truemmer {
        if k.position == 0 || !sichtbar(w, sid, *k) { continue; }
        let summe = t[0] + t[1];
        if summe < FELD_MIN * M { continue; }
        if w.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::Recyceln && f.ziel == *k) { continue; }
        let bedarf = (summe + cap - 1) / cap;
        let n = bedarf.clamp(1, max_schiffe.max(1));
        let mut schiffe = [0i64; SCHIFFE];
        schiffe[Einheit::Recycler.idx()] = n;
        let Some((dauer, sprit, tempo)) = fahrt(w, sid, pid, lager.bestand[Gut::Deuterium.idx()], *k, &schiffe) else { continue };
        if dauer > ANREISE_BERGUNG { continue; }
        let nimm = summe.min(n * cap);
        let mut ertrag = [0i64; GUETER];
        ertrag[Gut::Erz.idx()] = anteil(t[0], nimm, summe);
        ertrag[Gut::Kristall.idx()] = nimm - ertrag[Gut::Erz.idx()];
        let wert = w.regeln.wert(&ertrag);
        if wert < WERT_ZU_TREIBSTOFF * spritwert(w, sprit) + 1_500 * M { continue; }
        let platz = lager.platzfaktor(&ertrag);
        if platz < PLATZ_MIN { continue; }
        let eigen = eigene.contains(k);
        // Wert je Stunde samt drei Stunden Entladen, Warten und Planen; eigene Beute zählt anderthalbfach, und ein
        // Würfel je Ort und Sechsstundenfenster macht die Wahl zwischen ähnlichen Feldern unvorhersehbar.
        let stunden = (2 * dauer) as f64 / STUNDE as f64 + 3.0;
        let punkte = wert as f64 / M as f64 / stunden * platz * if eigen { 1.5 } else { 1.0 }
            * (0.8 + 0.4 * wuerfel(w, sid, ort_schluessel(*k), fenster));
        if bestes.as_ref().is_none_or(|b| punkte > b.punkte) {
            bestes = Some(Auftrag { ziel: *k, anzahl: n, tempo, halte_stunden: 0, punkte, eigen });
        }
    }
    bestes
}

/// Bestes Abbauziel (Gürtel oder Ring in einem erfassten System) für höchstens `max_schiffe` Bergbauschiffe.
fn bergbau_planen(w: &Welt, sid: SpielerId, pid: usize, max_schiffe: i64) -> Option<Auftrag> {
    let cap = kapazitaet_je(w, sid, pid, Einheit::Bergbauschiff);
    if cap <= 0 { return None; }
    let r = &w.regeln;
    let lager = Lager::lesen(w, pid);
    let fenster = (w.zeit / (6 * STUNDE)) as u64;
    let mut bestes: Option<Auftrag> = None;
    for z in w.bergbauziele(sid) {
        let Some(k) = z["koord"].as_str().and_then(|s| s.parse::<Koord>().ok()) else { continue };
        // Gürtel (Position 0) und Ringe (Position > 0) sind die einzigen Abbauziele; ein Trümmerfeld ist keines.
        if (k.position == 0) != (z["art"] == "Asteroidengürtel") { continue; }
        if w.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::Abbau && f.ziel == k) { continue; }
        // Der Ring einer fremden Welt ist ohne diplomatischen Zugang gesperrt.
        if k.position > 0 && w.belegung.get(&k).is_some_and(|id| {
            let b = w.planeten[*id as usize].besitzer;
            b != sid && !w.verbuendet(sid, b)
        }) { continue; }
        let (re, rk) = reichtum(w, sid, k);
        let erz_h = r.flug.abbau_erz_je_stunde * re;
        let kristall_h = r.flug.abbau_kristall_je_stunde * rk;
        let je_schiff_stunde = (erz_h + kristall_h) as f64 / 1000.0 * M as f64;
        if je_schiff_stunde <= 0.0 { continue; }
        // Schiffe, die ins Lager passen, und Stunden, bis sie voll sind; wer von Hand fliegt, hält selten auf die
        // Minute genau, darum 80 bis 100 Prozent der Füllzeit.
        let n = max_schiffe.max(1).min(((lager.raum() + lager.raum() / 4) / cap).max(1));
        let voll = cap as f64 / je_schiff_stunde;
        let u = wuerfel(w, sid, ort_schluessel(k), fenster ^ 0x5bd1);
        let stunden = ((voll * (0.8 + 0.2 * u)).ceil() as i64).clamp(2, r.flug.abbau_max_stunden);
        let mut schiffe = [0i64; SCHIFFE];
        schiffe[Einheit::Bergbauschiff.idx()] = n;
        let Some((dauer, sprit, tempo)) = fahrt(w, sid, pid, lager.bestand[Gut::Deuterium.idx()], k, &schiffe) else { continue };
        if dauer > ANREISE_BERGBAU { continue; }
        let nimm = ((n as f64 * je_schiff_stunde * stunden as f64) as i64).min(n * cap);
        if nimm < n * cap / 2 { continue; }
        let mut ertrag = [0i64; GUETER];
        ertrag[Gut::Erz.idx()] = (nimm as f64 * erz_h as f64 / (erz_h + kristall_h).max(1) as f64) as i64;
        ertrag[Gut::Kristall.idx()] = nimm - ertrag[Gut::Erz.idx()];
        let wert = r.wert(&ertrag);
        if wert < WERT_ZU_TREIBSTOFF * spritwert(w, sprit) + 1_500 * M { continue; }
        let platz = lager.platzfaktor(&ertrag);
        if platz < PLATZ_MIN { continue; }
        let gesamt_h = (2 * dauer) as f64 / STUNDE as f64 + stunden as f64 + 3.0;
        let punkte = wert as f64 / M as f64 / gesamt_h * platz * (0.8 + 0.4 * wuerfel(w, sid, ort_schluessel(k), fenster));
        if bestes.as_ref().is_none_or(|b| punkte > b.punkte) {
            bestes = Some(Auftrag { ziel: k, anzahl: n, tempo, halte_stunden: stunden, punkte, eigen: false });
        }
    }
    bestes
}

/// Baukosten des Kolonieschiffs, solange noch eine Kolonie offen ist: `kolonisieren` hat Vorrang vor Schiffskäufen.
fn kolonie_ruecklage(w: &Welt, sid: SpielerId) -> [i64; GUETER] {
    let sp = &w.spieler[sid as usize];
    let r = &w.regeln;
    if sp.stufe < r.einh(Einheit::Kolonieschiff).ab_stufe { return [0; GUETER]; }
    let unterwegs = w.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren).count();
    let fuer_stufe_v = r.stufen.last().map(|s| s.kolonien as usize).unwrap_or(0);
    if w.kolonien(sid) + unterwegs >= w.kolonien_erlaubt(sid).max(fuer_stufe_v) { return [0; GUETER]; }
    r.kosten_einheit(Einheit::Kolonieschiff, sp.volk)
}

/// Was neben dem nächsten Bauziel (`ruecklage` in bots.rs, beim Sparen samt den Kosten der nächsten Stufe) im Lager
/// bleibt, bevor ein Schiff gekauft wird: die nächste Stufe jeder Forschung, die die nächste Stufe verlangt (dazu
/// Astrophysik, solange Kolonien fehlen), und das Kolonieschiff. Je Gut gilt der größte dieser Posten.
/// 20.10.2026 (bot_lage Startwert 712, Tag 10): ohne diese Rücklage gaben 30 Bots 78 Kaufaufträge für Schiffe aus dem
/// Bestand über dem nächsten Bauziel; Astrophysik 1 fehlte danach 14 statt 10 Bots, nur noch 9 statt 13 hatten eine Kolonie.
/// Die vollen Kosten der nächsten Stufe zusätzlich zurückzulegen verhinderte dagegen fast jeden Kauf (19.478-mal „nicht
/// bezahlbar“ bis Tag 20, elf Schiffe bei zwei Händlern) und brachte beim Stufentempo nichts: Stufe V hatten am Tag 25
/// ohne Abbau 5, mit dieser Rücklage 2 und ohne sie wieder 5 Bots.
fn schutz_ruecklage(w: &Welt, sid: SpielerId) -> [i64; GUETER] {
    let sp = &w.spieler[sid as usize];
    let r = &w.regeln;
    let mut res = kolonie_ruecklage(w, sid);
    let mut halte = |k: [i64; GUETER]| { for g in 0..GUETER { res[g] = res[g].max(k[g]); } };
    if let Some(regel) = r.stufen.get(sp.stufe as usize - 1) {
        for (f, n) in &regel.forschung {
            if sp.forschung[f.idx()] < *n && sp.forschung[f.idx()] < MAX_FORSCHUNG { halte(r.kosten_forschung(*f, sp.forschung[f.idx()] + 1)); }
        }
    }
    let kolonien_ziel = r.stufen.last().map(|s| s.kolonien as usize).unwrap_or(0);
    let kf = w.kolonie_forschung();
    if w.kolonien_erlaubt(sid) < kolonien_ziel && sp.forschung[kf.idx()] < MAX_FORSCHUNG {
        halte(r.kosten_forschung(kf, sp.forschung[kf.idx()] + 1));
    }
    res
}

/// Bestellt bis zu zwei Schiffe, wenn der Bestand über allen Rücklagen (Ausbauziel, Aufstieg, Forschung, Kolonieschiff) liegt, die
/// Werftschleife nicht voll ist und der Unterhalt der Flotte aus den Credits tragbar bleibt.
fn bestellen(w: &mut Welt, sid: SpielerId, typ: Bottyp, pid: usize, e: Einheit, fehlt: i64) -> bool {
    if fehlt < 1 { return false; }
    // Die Werftschleife hat fünf Plätze und arbeitet parallel; ein einzelner Auftrag (Sonden, Jäger, Geschütze) darf
    // davorstehen. Nur eine volle Schleife hält den Kauf auf (Räuber füllen sie sonst in jedem Zug zuerst).
    if w.planeten[pid].fertigung[0].len() >= SCHLEIFE_MAX {
        zaehle("abbau: kauf wartet, werftschleife belegt");
        return false;
    }
    if w.spieler[sid as usize].stufe < KAUF_AB_STUFE {
        zaehle("abbau: kauf erst ab der Aufbaustufe");
        return false;
    }
    // Ohne genug Flottenplätze bliebe das Schiff im Hangar (und die Logistik verlöre ihren Platz).
    if w.flottenplaetze(sid) < if e == Einheit::Bergbauschiff { PLAETZE_FUER_ABBAU } else { PLAETZE_FUER_BERGUNG } {
        zaehle("abbau: kauf ohne genug flottenplätze");
        return false;
    }
    if !baubar(w, sid, pid, e) {
        zaehle("abbau: kauf nicht möglich (stufe, werft, forschung)");
        return false;
    }
    // Solange noch eine Kolonie fehlt, gehört die Werft dem Kolonieschiff: Aufträge für Recycler und Bergbauschiffe vor
    // ihm hielten es auf (bot_lage 712, Tag 30: „Kolonieschiff in Fertigung“ 288-mal statt 53-mal, Stufe V im Median
    // einen Tag später).
    if kolonie_ruecklage(w, sid).iter().any(|k| *k > 0) {
        zaehle("abbau: kauf wartet, die Werft gehört dem Kolonieschiff");
        return false;
    }
    let r = w.regeln.clone();
    let volk = w.spieler[sid as usize].volk;
    let kosten = r.kosten_einheit(e, volk);
    let mut spielraum = verfuegbar(w, sid, pid, typ);
    let schutz = schutz_ruecklage(w, sid);
    for g in 0..GUETER { spielraum[g] = (spielraum[g] - schutz[g]).max(0); }
    let bezahlbare = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| spielraum[g] / kosten[g]).min().unwrap_or(0);
    let besatzung = r.einh(e).besatzung * M;
    let n = fehlt.min(2).min(bezahlbare);
    if n < 1 {
        zaehle("abbau: kauf über der rücklage nicht bezahlbar");
        return false;
    }
    if w.planeten[pid].bevoelkerung < 1500 * M + n * besatzung {
        zaehle("abbau: kauf ohne besatzung");
        return false;
    }
    let je_tag = mal(w.flottenwert(sid) + n * r.wert(&kosten), r.wirtschaft.unterhalt_schiffe_je_tag);
    if w.spieler[sid as usize].credits < 10 * je_tag {
        zaehle("abbau: kauf, unterhalt nicht tragbar");
        return false;
    }
    let ks = w.planeten[pid].koord.to_string();
    tu(w, sid, json!({"typ": "fertigen", "planet": ks, "einheit": e.name(), "anzahl": n}))
}

/// Obergrenze der Schiffe, die der Bot auf dieser Stufe für diese Aufgabe hält.
fn schiffe_grenze(w: &Welt, sid: SpielerId, typ: Bottyp, bergbau: bool) -> i64 {
    let basis = (w.spieler[sid as usize].stufe as i64 - 2).clamp(1, 3);
    match (typ, bergbau) {
        (Bottyp::Raeuber, true) => basis.min(2),
        (Bottyp::Raeuber, false) => basis + 1,
        (Bottyp::Haendler, true) => basis + 1,
        _ => basis,
    }
}

/// Zögern: ab und zu lässt ein Bot eine gute Gelegenheit verstreichen (außer eigener Beute).
fn zoegert(w: &Welt, sid: SpielerId, salz: u64) -> bool {
    wuerfel(w, sid, (w.zeit / 900) as u64, salz) < 0.12
}

fn bergung(w: &mut Welt, sid: SpielerId, typ: Bottyp, pid: usize) {
    let (daheim, gesamt) = bilanz(w, sid, pid, Einheit::Recycler);
    let reserve = recycler_fuer_kolonie(w, sid, pid);
    let verwendbar = (daheim - reserve).max(0);
    let Some(a) = bergung_planen(w, sid, pid, schiffe_grenze(w, sid, typ, false)) else {
        zaehle("abbau: kein freies trümmerfeld in reichweite");
        return;
    };
    let laeuft = w.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::Recyceln);
    if verwendbar > 0 && !laeuft {
        if frei(w, sid) - if a.eigen { 0 } else { PLATZ_RESERVE } < 1 {
            zaehle("abbau: bergung ohne flottenplatz");
        } else if !a.eigen && zoegert(w, sid, 1) {
            zaehle("abbau: bergung verschoben");
        } else {
            let ok = tu(w, sid, json!({"typ": "flotte_senden", "start": w.planeten[pid].koord.to_string(), "ziel": a.ziel.to_string(),
                "mission": "recyceln", "schiffe": {"recycler": a.anzahl.min(verwendbar)}, "geschwindigkeit": a.tempo}));
            zaehle(if ok && a.eigen { "abbau: recycler zu eigenen kampftrümmern" } else if ok { "abbau: recycler zu fremdem feld" } else { "abbau: recycler abgelehnt" });
            if ok { return; }
        }
    }
    // Mehr Schiffe nur, wenn das beste Feld mehr trägt, als daheim steht.
    if gesamt < reserve + a.anzahl && bestellen(w, sid, typ, pid, Einheit::Recycler, reserve + a.anzahl - gesamt) {
        zaehle("abbau: recycler bestellt");
    }
}

fn bergbau(w: &mut Welt, sid: SpielerId, typ: Bottyp, pid: usize) {
    if w.flottenplaetze(sid) < PLAETZE_FUER_ABBAU {
        zaehle("abbau: bergbau braucht vier flottenplätze");
        return;
    }
    let (daheim, gesamt) = bilanz(w, sid, pid, Einheit::Bergbauschiff);
    let Some(a) = bergbau_planen(w, sid, pid, schiffe_grenze(w, sid, typ, true)) else {
        zaehle("abbau: kein freier gürtel oder ring in reichweite");
        return;
    };
    let laeuft = w.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::Abbau);
    if daheim > 0 && !laeuft {
        if frei(w, sid) - PLATZ_RESERVE < 1 {
            zaehle("abbau: bergbau ohne flottenplatz");
        } else if zoegert(w, sid, 2) {
            zaehle("abbau: bergbau verschoben");
        } else {
            let ok = tu(w, sid, json!({"typ": "flotte_senden", "start": w.planeten[pid].koord.to_string(), "ziel": a.ziel.to_string(),
                "mission": "abbau", "schiffe": {"bergbauschiff": a.anzahl.min(daheim)}, "geschwindigkeit": a.tempo, "haltedauer_stunden": a.halte_stunden}));
            zaehle(if ok && a.ziel.position == 0 { "abbau: bergbauschiffe zum gürtel" } else if ok { "abbau: bergbauschiffe zum ring" } else { "abbau: bergbau abgelehnt" });
            if ok { return; }
        }
    }
    if gesamt < a.anzahl && bestellen(w, sid, typ, pid, Einheit::Bergbauschiff, a.anzahl - gesamt) {
        zaehle("abbau: bergbauschiff bestellt");
    }
}

/// Ein Sondenflug in ein noch unbekanntes System des eigenen Sektors, solange wenig Ziele bekannt sind (je weniger,
/// desto öfter). Gürtel, Ringe und fremde Trümmerfelder sieht niemand ohne erfasstes System.
fn erkunden(w: &mut Welt, sid: SpielerId, pid: usize) {
    if !w.aufklaerungsregeln() { return; }
    let k = w.planeten[pid].koord;
    let systeme = w.regeln.welt.systeme_je_sektor;
    if w.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::SystemErkunden) { return; }
    let bekannte_ziele = w.bergbauziele(sid).len() + w.truemmer.keys().filter(|t| t.position > 0 && sichtbar(w, sid, **t)).count();
    let pause = if bekannte_ziele == 0 { 3 * STUNDE } else if bekannte_ziele < 3 { 12 * STUNDE } else { 40 * STUNDE };
    let zuletzt = w.aufklaerung.systemberichte.range((sid, 0, 0)..=(sid, u8::MAX, u8::MAX)).map(|(_, b)| b.zeit).max().unwrap_or(-pause);
    let jitter = (wuerfel(w, sid, (w.zeit / STUNDE) as u64, 3) * 0.5 * pause as f64) as i64;
    if w.zeit - zuletzt < pause + jitter { return; }
    // Die drei nächsten unerfassten Systeme bis 20 Systeme Abstand; ein Würfel wählt eines davon.
    let mut kandidaten: Vec<u8> = (1..=systeme)
        .filter(|n| (*n as i64 - k.system as i64).abs() <= 20 && !w.system_erfasst(sid, Koord::neu(k.sektor, *n, 1)))
        .collect();
    kandidaten.sort_by_key(|n| ((*n as i64 - k.system as i64).abs(), *n));
    kandidaten.truncate(3);
    if kandidaten.is_empty() { return; }
    let n = kandidaten[(wuerfel(w, sid, (w.zeit / STUNDE) as u64, 4) * kandidaten.len() as f64) as usize % kandidaten.len()];
    let ziel = Koord::neu(k.sektor, n, 1);
    if frei(w, sid) < 1 + PLATZ_RESERVE { return; }
    if let Some(a) = sondenauftrag(w, sid, pid, ziel, "system_erkunden", 1) {
        if tu(w, sid, a) { zaehle("abbau: system erkundet für abbauziele"); }
    } else if w.planeten[pid].einheiten[Einheit::Spionagesonde.idx()] == 0 && sonden_geplant(w, sid, pid) == 0
        && w.planeten[pid].fertigung[0].len() < SCHLEIFE_MAX
        && bezahlbar(&w.bestand_jetzt(pid), &w.regeln.kosten_einheit(Einheit::Spionagesonde, w.spieler[sid as usize].volk)) {
        let ks = k.to_string();
        tu(w, sid, json!({"typ": "fertigen", "planet": ks, "einheit": "spionagesonde", "anzahl": 1}));
    }
}

/// Ein Zug des Abbaus (nach Logistik und Markt).
pub(super) fn zug(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
    // Der Kontrolltyp „Wehrlos“ bleibt ein reiner Ökonom ohne Flotte.
    if typ == Bottyp::Wehrlos { return; }
    let sp = &w.spieler[sid as usize];
    if sp.stufe < 3 || w.regeln.flottendoktrin.is_empty() || !w.spieler_aktiv(sid) { return; }
    let pid = sp.heimat as usize;
    let p = &w.planeten[pid];
    if p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 || p.gebaeude[Gebaeude::Werft.idx()] < 1 || p.blockade.is_some() { return; }
    bergung(w, sid, typ, pid);
    bergbau(w, sid, typ, pid);
    erkunden(w, sid, pid);
}
