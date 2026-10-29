//! Mondverhalten der Skriptbots (Mondgeneration STERNE13): Monde ausbauen und versorgen, Solarsonden für die Energie,
//! Sprungtore öffnen und eigene Flotten durch das Hyperraumloch verlegen, Räuber scannen mit der Tiefenphalanx, kriegerische
//! Bots bauen das Werftdock bis zum Weltenkrümmer und setzen ihn gegen Monde ein. Alles über dieselben Aktionen wie Agenten.
//!
//! 20.10.2026 (Karl: „Skriptbots sollen Monde ausbauen und Weltenkrümmer selbst bauen – keine Lücken“).
//! Befund vor der Änderung (`bot_lage`, 30 Bots, 30 Tage): Startwert 712 elf Monde und 3 Mondgebäudestufen, Startwert 2026 neun
//! Monde und 0 Stufen, nirgends ein Weltenkrümmer. Nach Messung und Testwelt waren es sieben Lücken, die sich gegenseitig verdeckten:
//!  1. Der Plan blieb am ersten blockierten Schritt hängen (Sprungtor an siebter Stelle: 500.000 Erz, Hyperraumantrieb 3, Stufe V).
//!  2. Mondbasis 1 (10.000 Erz, 20.000 Kristall, 10.000 Deuterium) liefert nur ein Transport: das Mondlager fasst ohne Mondbasis
//!     4.000, die Fähre ruht bis Mondbasis 1. Der Nachschub verlangte aber zwei freie Flottenplätze, Frachter auf dem Mutterplaneten
//!     (`logistik` baut sie nur für Kolonien und schickt alle Großen jeden Zug auf Fahrt) und alles Fehlende auf einmal aus höchstens
//!     zwei Dritteln des Bestands – ein Bot, der sein Erz sofort ausgibt, hat nie so viel übrig.
//!  3. Die Lieferung rundete die Ladung ab und hielt dann die „kleine“ letzte Lieferung für zu klein: Monde mit 9.834 von 10.000
//!     Erz blieben für immer stehen.
//!  4. Billige Stufen (Regolith, Kryolager) füllten das letzte freie Feld, während die Mondbasis auf Rohstoffe wartete: 7 von
//!     7 Feldern, Mondbasis 3 nie mehr baubar.
//!  5. Die Fähre wurde einmal mit dem Deckel von Mondbasis 1 gesetzt und blieb dabei.
//!  6. Auf Monden über Kolonien (ohne Raumhafen und Werft) gab es nie einen Frachter.
//!  7. Die Logistik der Bots belegt alle Flottenplätze, auch den letzten (junge Kolonien): 22 von 232 Lieferungen kamen los.
//! Jetzt: Plan nach Feldbudget des Mondes, der erste ausführbare oder lieferbare Schritt gewinnt (Unbezahlbares und Nichtlieferbares
//! wird übersprungen), Teillieferungen aus 30 % des Planetenbestands über der Aufstiegsrücklage, Kleine Transporter als eigene
//! Mondfrachter (Heimatplanet für Kolonien), die Fähre füllt das Mondlager nach Bedarf, Computertechnik gegen volle Flottenplätze.
//! Ergebnis (`bot_lage` 30 Tage, Startwert 712 / 2026): Mondgebäudestufen 3 → 39 / 0 → 71, Stufe V am Tag 30 unverändert (21 / 24 von 30).
//! Der Weltenkrümmer (Räuber, Igel als Vergeltung) braucht nach den Regeln Stufe IV für die Mondgebäude (frühestens Tag 14,5), 7,0
//! Tage Bauzeit bis Werftdock 10 (seit 21.10.2026 reicht die Regelstufe 8 aus `weltentruemmer_werftdock` in monde.ron: rund
//! 2,3 Tage weniger Dock, dafür 18,1 statt 14,8 Tage Fertigung bei den Krath), Stufe V (frühestens Tag 23) mit der Doktrin aus flottenausbau.json (bis 20.10.2026
//! Hyperraumantrieb 6 und Waffentechnik 10, rund 780.000 Forschungspunkte; jetzt 5 und 8), dann 14,8 Tage (Krath, sonst 17,4) Fertigung: vor Tag 38 nicht. Die Tests `monde_kruemmer.rs` bereiten Stufe, Forschung
//! und Vorrat vor und beweisen den Rest: Bestellung, Fertigung, Spähen, Einsatz, Ausgang.

use super::{baubar, mond_ruecklage, zaehle, Bottyp};
use kern::kampf::{kampf_ohne_protokoll, Gruppe};
use kern::monde::{MondGebaeude, MondId, MONDGEBAEUDE};
use kern::typen::*;
use kern::welt::*;
use serde_json::{json, Value};

fn tu(w: &mut Welt, sid: SpielerId, a: Value) -> bool { w.handeln(sid, Rolle::Alle, &a).0 }

/// Wunschliste je Bottyp (Gebäude, Zielstufe) nach Vorrang. Die Mondbasis ergänzt `plan` selbst, so weit die
/// Felder es verlangen. Auf dem Krümmermond (`wk` = Werftdock-Stufe des Weltenkrümmers aus der Regel, siehe `kruemmer_mond`)
/// steht das Werftdock bis zum Weltenkrümmer gleich nach dem Grundstock; das Feldbudget in `plan` streicht dann, was nicht mehr
/// danebenpasst.
fn wunschliste(typ: Bottyp, wk: Option<u8>) -> Vec<(MondGebaeude, u8)> {
    use MondGebaeude::*;
    let mut v = vec![(Mondbasis, 1), (Regolithfoerderung, 1), (Mondbasis, 2), (Werftdock, 1)];
    // Das Krümmer-Werftdock gehört auf dem Krümmermond vor das Kryolager, sonst braucht der Mond ein Feld mehr, als er hat.
    if let Some(dock) = wk { v.extend([(Mondbasis, 3), (Werftdock, dock), (Kryolager, 1)]); } else { v.extend([(Kryolager, 1), (Mondbasis, 3)]); }
    match typ {
        Bottyp::Raeuber => v.extend([(Tiefenphalanx, 2), (Werftdock, 3), (Tiefenphalanx, 3)]),
        Bottyp::Igel => v.extend([(Ortungsschleier, 2), (Werftdock, 4)]),
        Bottyp::Haendler => v.extend([(Regolithfoerderung, 4), (Kryolager, 3), (Mondbasis, 4)]),
        _ => v.extend([(Regolithfoerderung, 4), (Mondbasis, 4), (Werftdock, 2)]),
    }
    v.extend([(Sprungtor, 1), (Regolithfoerderung, 6), (Mondbasis, 6), (Sprungtor, 3)]);
    v
}

/// Ausbaupfad für einen Mond mit `felder_max` Feldern. Jede Gebäudestufe belegt ein Feld, die Mondbasis schaltet
/// `1 + 3 · Stufe` frei: die Summe aller Stufen muss darunter bleiben, sonst ist der letzte Bau nie möglich
/// (Werftdock 10 braucht Mondbasis 5 und, mit Regolith 1, 16 Felder). Schritte, die nicht mehr passen, entfallen.
fn plan(typ: Bottyp, felder_max: u16, wk: Option<u8>) -> Vec<(MondGebaeude, u8)> {
    let mut ziel = [0u8; MONDGEBAEUDE];
    let mut aus = Vec::new();
    for (g, z) in wunschliste(typ, wk) {
        let mut t = ziel;
        t[g.idx()] = t[g.idx()].max(z);
        let andere: u16 = MondGebaeude::ALLE.iter().filter(|x| **x != MondGebaeude::Mondbasis).map(|x| t[x.idx()] as u16).sum();
        let noetig = ((andere / 2).max(1)) as u8;
        let basis = t[MondGebaeude::Mondbasis.idx()].max(noetig);
        if andere + basis as u16 > felder_max { continue; }
        if basis > ziel[MondGebaeude::Mondbasis.idx()] { aus.push((MondGebaeude::Mondbasis, basis)); }
        t[MondGebaeude::Mondbasis.idx()] = basis;
        if g != MondGebaeude::Mondbasis { aus.push((g, z)); }
        ziel = t;
    }
    aus
}

/// Was einen Schritt im Moment sperrt, außer fehlenden Rohstoffen.
enum Huerde { Keine, Stufe, Braucht, Max, Feld, Forschung(Vec<(Forschung, u8)>) }

fn huerde(w: &Welt, sid: SpielerId, mid: MondId, g: MondGebaeude) -> Huerde {
    let Some(r) = w.monde_regeln() else { return Huerde::Stufe };
    let (m, sp, regel) = (&w.monde.monde[&mid], &w.spieler[sid as usize], r.geb(g));
    if sp.stufe < regel.ab_stufe { return Huerde::Stufe; }
    if regel.braucht.iter().any(|(b, st)| m.gebaeude[b.idx()] < *st) { return Huerde::Braucht; }
    let forschung: Vec<(Forschung, u8)> = regel.forschung.iter().filter(|(f, n)| sp.forschung[f.idx()] < **n).map(|(f, n)| (*f, *n)).collect();
    if !forschung.is_empty() { return Huerde::Forschung(forschung); }
    if regel.max_stufe > 0 && m.gebaeude[g.idx()] + 1 > regel.max_stufe { return Huerde::Max; }
    let (belegt, frei, _) = w.mond_felder(mid);
    if belegt >= frei { return Huerde::Feld; }
    Huerde::Keine
}

/// Fehlende Rohstoffe für die nächste Stufe von `g` (Tausendstel), gemessen am Mondbestand.
fn kosten_fehlen(w: &Welt, mid: MondId, g: MondGebaeude) -> ([i64; GUETER], [i64; GUETER]) {
    let r = w.monde_regeln().expect("Mondregeln");
    let m = &w.monde.monde[&mid];
    let kosten = r.kosten(g, m.gebaeude[g.idx()] + 1);
    let mut fehlt = [0i64; GUETER];
    for i in 0..GUETER { fehlt[i] = (kosten[i] - m.bestand[i]).max(0); }
    (kosten, fehlt)
}

pub fn zug(w: &mut Welt, sid: SpielerId, typ: Bottyp, schutz: &super::Opferschutz) {
    if !w.monde.aktiv || typ == Bottyp::Wehrlos { return; }
    let eigene: Vec<MondId> = w.monde.monde.values().filter(|m| m.besitzer == sid).map(|m| m.id).collect();
    for mid in &eigene {
        ausbauen(w, sid, *mid, typ);
        kruemmer_fertigen(w, sid, *mid, typ);
        energie(w, sid, *mid, typ);
        faehre(w, sid, *mid, typ);
    }
    if typ == Bottyp::Raeuber { phalanx(w, sid, &eigene); }
    freigeben(w, sid, &eigene);
    sprungtor(w, sid, &eigene, typ);
    kruemmer_einsatz(w, sid, typ, &eigene, schutz);
}

/// Der erste Schritt des Plans, den der Mond bezahlen kann oder der sich per Transport beliefern lässt, wird gebaut
/// bzw. beliefert. Schritte, die Stufe, Forschung, Vorgebäude oder ein freies Feld vermissen, und unbezahlbare
/// (Sprungtor: 500.000 Erz) übersprungen – der Bot wartet nicht auf einen einzigen Schritt.
fn ausbauen(w: &mut Welt, sid: SpielerId, mid: MondId, typ: Bottyp) {
    let (felder_max, koord) = { let m = &w.monde.monde[&mid]; (m.felder_max, m.koord) };
    if !w.monde.monde[&mid].bauschleife.is_empty() { zaehle("monde: bauschleife läuft"); return; }
    if w.spieler[sid as usize].stufe < 4 { zaehle("monde: stufe unter IV"); return; }
    let plan = plan(typ, felder_max, kruemmer_dock(w, sid, mid, typ));
    let mondbasis_ziel = plan.iter().filter(|(g, _)| *g == MondGebaeude::Mondbasis).map(|(_, z)| *z).max().unwrap_or(0);
    for (g, ziel) in plan {
        if w.monde.monde[&mid].gebaeude[g.idx()] >= ziel { continue; }
        match huerde(w, sid, mid, g) {
            Huerde::Keine => {}
            Huerde::Forschung(f) => { zaehle("monde: forschung fehlt"); for (x, n) in f { forschung_anstossen(w, sid, x, n); } continue; }
            Huerde::Feld => { zaehle("monde: kein freies feld"); continue; }
            Huerde::Stufe => { zaehle("monde: hürde stufe"); continue; }
            Huerde::Braucht => { zaehle("monde: hürde vorgebäude"); continue; }
            Huerde::Max => { zaehle("monde: hürde höchststufe"); continue; }
        }
        // Das letzte freie Feld gehört der Mondbasis, solange sie ihr Ziel nicht erreicht hat: Wer es mit einer billigen Stufe füllt,
        // während die Mondbasis auf Rohstoffe wartet, sperrt den Mond für immer (bot_lage 712, Tag 20: Mondbasis 2, Regolith 4,
        // Kryolager 1 auf 7 von 7 Feldern, Mondbasis 3 wartete 1.686-mal auf ein freies Feld).
        if g != MondGebaeude::Mondbasis && w.monde.monde[&mid].gebaeude[MondGebaeude::Mondbasis.idx()] < mondbasis_ziel {
            let (belegt, frei, _) = w.mond_felder(mid);
            if frei.saturating_sub(belegt) <= 1 { zaehle("monde: letztes feld gehört der mondbasis"); continue; }
        }
        let (kosten, fehlt) = kosten_fehlen(w, mid, g);
        // Beim Sparen auf den Weltenkrümmer bekommt der Mond nur noch Billiges (höchstens ein Zwanzigstel des Krümmerpreises):
        // der Mondbestand gehört dann dem Krümmer, auch wenn er ein Sprungtor bezahlen könnte.
        if kruemmer_bereit(w, sid, mid, typ) && w.regeln.wert(&kosten) * 20 > w.regeln.wert(&w.regeln.kosten_einheit(Einheit::Weltentruemmer, w.spieler[sid as usize].volk)) {
            zaehle("monde: spart für krümmer");
            continue;
        }
        if fehlt.iter().all(|x| *x == 0) {
            if tu(w, sid, json!({"typ":"mond_bauen","mond":koord.to_string(),"gebaeude":g.name()})) { zaehle("monde: gebaut"); } else { zaehle("monde: bau abgelehnt"); }
            return;
        }
        // Legierung, Elektronik und Xenokristall kommen nicht binnen Stunden nach: Fehlen sie auf Mond und Planeten, liefert der Bot
        // auch das Übrige nicht an (bot_lage 2026, Tag 35: ein Räuber parkte 500.000 Erz und 270.000 Kristall auf dem Mond für ein
        // Sprungtor, das 20.000 Legierung und 3.000 Xenokristall braucht – der Planet hatte 82 Legierung).
        let quellen = [w.monde.monde[&mid].mutter as usize, w.spieler[sid as usize].heimat as usize].map(|p| w.bestand_jetzt(p));
        if [Gut::Legierung, Gut::Elektronik, Gut::Xenokristall].iter().any(|g| kosten[g.idx()] > w.monde.monde[&mid].bestand[g.idx()] + quellen[0][g.idx()].max(quellen[1][g.idx()])) {
            zaehle("monde: schritt nicht lieferbar");
            continue;
        }
        // Was ein Vielfaches dessen kostet, was der Mutterplanet überhaupt hat, ist noch kein Ziel (Sprungtor: 1 Mio. Kristall).
        let reich = [w.monde.monde[&mid].mutter as usize, w.spieler[sid as usize].heimat as usize].into_iter().map(|p| w.regeln.wert(&w.bestand_jetzt(p))).max().unwrap_or(0);
        if w.regeln.wert(&kosten) > 6 * reich + w.regeln.wert(&w.monde.monde[&mid].bestand) {
            zaehle("monde: schritt zu teuer");
            continue;
        }
        // Knappe Güter zuerst, wenn der Planet sie nicht in dieser Fahrt mitgeben kann (30 % seines Bestands): Erz, Kristall und Deuterium
        // folgen, sobald Legierung, Elektronik und Xenokristall auf dem Mond liegen (bot_lage 712, Tag 60: drei Monde parkten je 500.000
        // Erz und 1 Mio. Kristall für ein Sprungtor, dem 700 Elektronik fehlten). Reicht der Bestand, fährt alles zusammen.
        let mut knapp = [0i64; GUETER];
        for g in [Gut::Legierung, Gut::Elektronik, Gut::Xenokristall] { knapp[g.idx()] = fehlt[g.idx()]; }
        let auf_einmal = knapp.iter().enumerate().all(|(i, f)| *f * 10 <= quellen[0][i].max(quellen[1][i]) * 3);
        let sende = if knapp.iter().any(|x| *x > 0) && !auf_einmal { knapp } else { fehlt };
        match nachschub(w, sid, mid, sende, ABGABE_PROMILLE) {
            Nachschub::Gesendet => return,
            Nachschub::Unterwegs => { zaehle("monde: nachschub unterwegs"); return; }
            Nachschub::Wartet => continue,
        }
    }
}

enum Nachschub { Gesendet, Unterwegs, Wartet }

/// Frachter, die der Bot für Mondlieferungen einsetzt, nach Frachtraum absteigend.
const FRACHTER: [Einheit; 4] = [Einheit::Schwerlastfrachter, Einheit::Handelsfrachter, Einheit::GrosserTransporter, Einheit::KleinerTransporter];
/// Anteil des Planetenbestands je Gut, den eine Lieferung höchstens mitnimmt (Promille): Der Planet behält siebzig
/// Prozent. Die Bots geben ihr Erz sofort für Planetengebäude aus; wer alles auf einmal verlangte, bekam nie etwas
/// (Testwelt 20.10.2026: „planet gibt nichts ab“ in 322 von 500 Zügen), wer beliebig viel nimmt, bremst den Aufstieg.
const ABGABE_PROMILLE: i64 = 300;
/// Beim Sparen auf den Weltenkrümmer gibt der Planet die Hälfte seines Bestands je Zug ab: Auf dem Mond ist das Geld
/// vor den eigenen Käufen des Bots sicher (ein Räuber gab 9 Mio. Erz in zwölf Tagen für Schiffe aus).
const SPAR_PROMILLE: i64 = 500;
/// Kleinere Lieferungen lohnen die Fahrt nicht (Flottenplatz, Treibstoff), außer sie decken den ganzen Rest. 300 statt 1.000:
/// Der Planet hat nach seinen eigenen Käufen oft nur 2.000 Erz übrig, davon gehen 30 % (bot_lage 712, Tag 20: 1.984-mal „planet gibt
/// nichts ab“, weil 600 Erz unter der Grenze lagen und Kristall und Deuterium auf dem Mond schon reichten).
const MIN_LIEFERUNG: i64 = 300 * M;

/// Schickt fehlende Güter vom Mutterplaneten zum eigenen Mond, so viel der Planet abgeben kann (`abgabe` Promille seines
/// Bestands je Gut), wenn Frachter und ein Flottenplatz da sind; fehlt der Frachter, bestellt der Bot ihn. Die Kosten des
/// Aufstiegs, auf die der Heimatplanet gerade spart, zählen nicht zum Bestand. Gewählt wird der Frachtertyp, dessen gesamter Frachtraum die Menge
/// gerade trägt, sonst der mit dem größten.
fn nachschub(w: &mut Welt, sid: SpielerId, mid: MondId, fehlt: [i64; GUETER], abgabe: i64) -> Nachschub {
    let (mutter, k) = { let m = &w.monde.monde[&mid]; (m.mutter as usize, m.koord) };
    // Quelle ist der Mutterplanet, wenn er fliegen kann (Raumhafen und Frachter oder eine Werft für sie); eine junge Kolonie
    // hat beides nicht, dann schickt der Heimatplanet.
    let kann_fliegen = |p: usize| w.planeten[p].besitzer == sid && w.planeten[p].gebaeude[Gebaeude::Raumhafen.idx()] >= 1
        && (FRACHTER.iter().any(|e| w.planeten[p].einheiten[e.idx()] > 0) || baubar(w, sid, p, Einheit::KleinerTransporter));
    // Können beide fliegen, liefert, wer mehr vom Fehlenden abgeben kann (bei Gleichstand der Mutterplanet, die Fahrt ist kürzer):
    // Vorher lieferte immer der Mutterplanet, auch leer (Fortsetzung der r48-Welt, Tag 182: Krümmermond mit Werftdock 7, der
    // Mondbasis 9 fehlten 196.000 Deuterium, der Mutterplanet hatte 0).
    let heimat = w.spieler[sid as usize].heimat as usize;
    let gibt = |p: usize| {
        let (b, r) = (w.bestand_jetzt(p), mond_ruecklage(w, sid, p));
        Gut::ALLE.iter().map(|g| fehlt[g.idx()].max(0).min((b[g.idx()] - r[g.idx()]).max(0) * abgabe / 1000)).sum::<i64>()
    };
    let pid = match (kann_fliegen(mutter), heimat != mutter && kann_fliegen(heimat)) {
        (true, true) => if gibt(heimat) > gibt(mutter) { heimat } else { mutter },
        (true, false) => mutter,
        (false, _) => heimat,
    };
    if w.planeten[pid].besitzer != sid { return Nachschub::Wartet; }
    let start = w.planeten[pid].koord;
    if w.monde.flottenziel.iter().any(|(f, z)| *z == mid && w.flotten.get(f).is_some_and(|fl| fl.besitzer == sid && fl.zustand == Flottenzustand::Hinflug && fl.mission == Mission::Transport)) { return Nachschub::Unterwegs; }
    // Ein Flottenplatz genügt (vorher zwei): die Fahrt zum eigenen Mond dauert ein bis zwei Stunden.
    if w.flotten.values().filter(|f| f.besitzer == sid).count() >= w.flottenplaetze(sid) {
        zaehle("monde: keine flottenplätze");
        plaetze_nachruesten(w, sid);
        return Nachschub::Wartet;
    }
    if w.planeten[pid].gebaeude[Gebaeude::Raumhafen.idx()] < 1 { zaehle("monde: planet ohne raumhafen"); return Nachschub::Wartet; }
    let bestand = w.bestand_jetzt(pid);
    // Spart der Heimatplanet für den Aufstieg, gehören dessen Kosten nicht zur Abgabe (bot_lage 2026, Tag 25: Bots mit 360.000 Erz
    // und 760.000 Kristall auf Stufe IV lieferten dem Mond gar nichts, 2.116-mal „spart für den Aufstieg“, ein Mond mit 9.449 von
    // 10.000 Erz für die Mondbasis 1 stand still).
    let reserve = mond_ruecklage(w, sid, pid);
    let mut frei = bestand;
    for i in 0..GUETER { frei[i] = (bestand[i] - reserve[i]).max(0); }
    let mut gesamt_fehlt = 0;
    let mut wunsch = [0i64; GUETER];
    let mut deckt_alles = true;
    for g in Gut::ALLE {
        let i = g.idx();
        if fehlt[i] <= 0 { continue; }
        // Eine Einheit Zugabe gegen das Abrunden der Ladung. Antriebskerne sind Stückgut für den Weltenkrümmer (20 Stück): genau
        // die fehlende Zahl und der ganze Vorrat – mit Zugabe und halbem Vorrat je Fahrt deckte keine Lieferung je den Rest, und
        // unter 300 Einheiten fuhr sie nie.
        let stueckgut = g == Gut::Antriebskern;
        let braucht = if stueckgut { fehlt[i] } else { fehlt[i] + fehlt[i] / 50 + M };
        gesamt_fehlt += braucht;
        let gibt = if stueckgut { frei[i] } else { frei[i] * abgabe / 1000 };
        deckt_alles &= braucht <= gibt;
        wunsch[i] = braucht.min(gibt) / M * M;
    }
    let summe: i64 = wunsch.iter().sum();
    if summe == 0 || (summe < MIN_LIEFERUNG && !deckt_alles) {
        zaehle(if reserve.iter().any(|x| *x > 0) { "monde: planet spart für den aufstieg" } else { "monde: planet gibt nichts ab" });
        return Nachschub::Wartet;
    }
    // Frachtraum je Typ (alle Schiffe des Typs auf dem Planeten, Hinflug zum Mond).
    let mut wahl: Vec<(Einheit, i64, i64)> = Vec::new();
    for e in FRACHTER {
        let n = w.planeten[pid].einheiten[e.idx()];
        if n == 0 { continue; }
        let mut schiffe = [0i64; SCHIFFE];
        schiffe[e.idx()] = n;
        if let Ok(plan) = w.flugplan(sid, pid, k, &schiffe, 1000) { wahl.push((e, n, plan.kapazitaet)); }
    }
    if wahl.is_empty() { frachter_bestellen(w, sid, pid, gesamt_fehlt); return Nachschub::Wartet; }
    wahl.sort_by_key(|(_, _, kap)| *kap);
    let reihe: Vec<(Einheit, i64, i64)> = wahl.iter().copied().filter(|(_, _, kap)| *kap >= summe).chain(wahl.iter().rev().copied().filter(|(_, _, kap)| *kap < summe)).collect();
    for (e, n, kap) in reihe {
        let einzeln = kap / n.max(1);
        if einzeln <= 0 { continue; }
        let anzahl = ((summe + einzeln - 1) / einzeln).clamp(1, n);
        let mut schiffe = [0i64; SCHIFFE];
        schiffe[e.idx()] = anzahl;
        let Ok(plan) = w.flugplan(sid, pid, k, &schiffe, 1000) else { continue };
        let faktor = plan.kapazitaet.min(summe);
        let mut ladung = serde_json::Map::new();
        let mut deut = 0;
        for g in Gut::ALLE {
            let x = anteil(wunsch[g.idx()], faktor, summe.max(1)) / M;
            if x > 0 { ladung.insert(g.name().into(), json!(x)); }
            if g == Gut::Deuterium { deut = x * M; }
        }
        if bestand[Gut::Deuterium.idx()] < 2 * plan.treibstoff + deut { zaehle("monde: treibstoff fehlt"); continue; }
        if tu(w, sid, json!({"typ":"flotte_senden","start":start.to_string(),"ziel":k.to_string(),"zielart":"mond","mission":"transport","schiffe":{e.name():anzahl},"ladung":ladung})) {
            zaehle("monde: nachschub gesendet");
            return Nachschub::Gesendet;
        }
        zaehle("monde: nachschub abgelehnt");
    }
    Nachschub::Wartet
}

/// Bestellt Kleine Transporter als Mondfrachter des Quellplaneten: so viele, dass zwei Fahrten den Bedarf tragen (drei bis zwölf),
/// bis zu sechs je Zug, nur aus freiem Bestand und solange in der Werftschleife noch zwei Plätze frei sind (die Räuber füllen sie
/// mit Jägern und lassen sie nie leer: Testwelt 20.10.2026, 1.176 Absagen in 14 Tagen). Es sind Kleine, nicht Große Transporter:
/// `logistik` schickt alle Großen jeden Zug auf Kolonie-Fahrt, bevor der Mond an der Reihe ist (bot_lage 712, Tag 30: Händler mit
/// 543.000 Kristall im Lager und einem Mond, der fünf Tage ohne Lieferung blieb, weil nie ein Großer Transporter daheim stand);
/// Kleine fliegen nur die Räuber, und die brauchen für ihre Beute nur ein paar.
fn frachter_bestellen(w: &mut Welt, sid: SpielerId, pid: usize, summe: i64) {
    let kt = Einheit::KleinerTransporter;
    if !baubar(w, sid, pid, kt) { zaehle("monde: frachter nicht baubar"); return; }
    let p = &w.planeten[pid];
    if p.fertigung[0].len() + 2 > w.regeln.wirtschaft.warteschlange { zaehle("monde: werft belegt (frachter)"); return; }
    let mut eins = [0i64; SCHIFFE];
    eins[kt.idx()] = 1;
    let Ok(plan) = w.flugplan(sid, pid, p.koord, &eins, 1000) else { return };
    let soll = ((summe + 2 * plan.kapazitaet - 1) / (2 * plan.kapazitaet.max(1))).clamp(3, 12);
    let bestellt: i64 = p.fertigung[0].iter().filter(|f| f.produkt == Produkt::Einheit(kt)).map(|f| f.rest).sum();
    let hat = p.einheiten[kt.idx()] + bestellt;
    let kosten = w.regeln.kosten_einheit(kt, w.spieler[sid as usize].volk);
    let frei = w.bestand_jetzt(pid);
    // Die ersten Frachter dürfen den ganzen Bestand kosten (ohne sie bekommt der Mond nie seine Mondbasis 1: 18.000 Erz im Bestand
    // hatte kein Bot, 6.000 schon), weitere nur aus einem Drittel.
    let teiler = if hat < 3 { 1 } else { 3 };
    let bezahlbar = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| frei[g] / teiler / kosten[g]).min().unwrap_or(0);
    if hat >= soll { zaehle("monde: frachter genug bestellt"); return; }
    let n = (soll - hat).clamp(0, 6).min(bezahlbar);
    if n <= 0 { zaehle("monde: frachter unbezahlbar"); return; }
    let ks = p.koord.to_string();
    if tu(w, sid, json!({"typ":"fertigen","planet":ks,"einheit":kt.name(),"anzahl":n})) { zaehle("monde: frachter bestellt"); }
}

/// Reiht die Forschung für einen Mondschritt oder den Weltenkrümmer ein: nur auf Stufe V (davor gehört der Xenokristall dem
/// Aufstieg), nur bei leerer Warteschlange und nur, wenn der Heimatplanet höchstens 60 % seines Bestands dafür braucht – ein
/// unbezahlbarer Kopf der Warteschlange hielte die eigene Forschung des Bots an. Die Bots forschen nach ihrem Plan und danach das
/// Billigste, und dabei ist die Forschung fast immer belegt: die Krümmer-Doktrin (seit 20.10.2026 Hyperraumantrieb 5 und
/// Waffentechnik 8, Werte aus der Regel) käme von selbst nie; der Bot reiht sie hinter der laufenden Forschung ein.
fn forschung_anstossen(w: &mut Welt, sid: SpielerId, f: Forschung, ziel: u8) -> bool {
    let sp = &w.spieler[sid as usize];
    if sp.stufe < 5 || sp.forschung[f.idx()] >= ziel || !sp.forschung_schlange.is_empty() { return false; }
    if sp.forschung_aktiv.as_ref().is_some_and(|a| a.forschung == f) { return false; }
    let kosten = w.regeln.kosten_forschung(f, sp.forschung[f.idx()] + 1);
    let bestand = w.bestand_jetzt(sp.heimat as usize);
    if (0..GUETER).any(|g| kosten[g] * 10 > bestand[g] * 6) { zaehle("monde: forschung unbezahlbar"); return false; }
    let ok = tu(w, sid, json!({"typ":"forschen","forschung":f.name()}));
    if ok { zaehle("monde: forschung angestoßen"); }
    ok
}

/// Die Logistik der Bots belegt die Flottenplätze (Computertechnik 2 gibt drei, auch den letzten für junge Kolonien): Der Mond
/// bekam in 5 Spieltagen 22 von 232 Lieferungen los. Ein Platz mehr kostet Computertechnik Stufe n+1: 400 · 2ⁿ Kristall,
/// 600 · 2ⁿ Deuterium, 400 · 2ⁿ Forschungspunkte – wenig neben dem Mond. Der Bot reiht sie ein (bis Stufe 5), wenn sein
/// Heimatplanet das Fünffache trägt und die Warteschlange leer ist.
fn plaetze_nachruesten(w: &mut Welt, sid: SpielerId) {
    let sp = &w.spieler[sid as usize];
    let stufe = sp.forschung[Forschung::Computertechnik.idx()];
    if stufe >= 5 || sp.stufe < 3 || !sp.forschung_schlange.is_empty() { return; }
    if sp.forschung_aktiv.as_ref().is_some_and(|a| a.forschung == Forschung::Computertechnik) { return; }
    let kosten = w.regeln.kosten_forschung(Forschung::Computertechnik, stufe + 1);
    let bestand = w.bestand_jetzt(sp.heimat as usize);
    if (0..GUETER).any(|g| bestand[g] < 5 * kosten[g]) { return; }
    if tu(w, sid, json!({"typ":"forschen","forschung":Forschung::Computertechnik.name()})) { zaehle("monde: computertechnik eingereiht"); }
}

/// Solarsonden im Werftdock, solange die Energie nicht reicht – nur wenn Tiefenphalanx oder Sprungtor volle Energie
/// brauchen oder die Regolithförderung unter 80 % läuft (20.10.2026: sonst zahlt der Mond Kristall für nichts).
fn energie(w: &mut Welt, sid: SpielerId, mid: MondId, typ: Bottyp) {
    // Der Mondbestand gehört beim Sparen dem Weltenkrümmer.
    if kruemmer_bereit(w, sid, mid, typ) { return; }
    let sicht = w.mond_sicht(mid);
    if sicht["werftdock"]["stufe"].as_u64().unwrap_or(0) == 0 || !sicht["werftdock"]["schleife"].as_array().is_some_and(|a| a.is_empty()) { return; }
    let erz = sicht["energie"]["erzeugung"].as_f64().unwrap_or(0.0);
    let verb = sicht["energie"]["verbrauch"].as_f64().unwrap_or(0.0);
    let je = sicht["energie"]["je_solarsonde"].as_f64().unwrap_or(0.0);
    if verb <= erz || je <= 0.0 { return; }
    let geb = |n: &str| sicht["gebaeude"][n].as_u64().unwrap_or(0);
    let braucht = geb("tiefenphalanx") > 0 || geb("sprungtor") > 0 || (geb("regolithfoerderung") > 0 && erz < 0.8 * verb);
    if !braucht { return; }
    let n = ((verb - erz) / je).ceil() as i64 + 1;
    let kosten = w.regeln.kosten_einheit(Einheit::Solarsonde, w.spieler[sid as usize].volk);
    let bestand = w.monde.monde[&mid].bestand;
    let leistbar = Gut::ALLE.iter().filter(|g| kosten[g.idx()] > 0).map(|g| bestand[g.idx()] / kosten[g.idx()]).min().unwrap_or(0);
    let n = n.min(leistbar).min(200);
    let koord = sicht["koord"].as_str().unwrap_or("").to_string();
    if n > 0 { tu(w, sid, json!({"typ":"mond_fertigen","mond":koord,"einheit":"solarsonde","anzahl":n})); }
    else {
        let mut fehlt = [0i64; GUETER];
        for g in Gut::ALLE { fehlt[g.idx()] = (kosten[g.idx()] * 20 - bestand[g.idx()]).max(0); }
        nachschub(w, sid, mid, fehlt, ABGABE_PROMILLE);
    }
}

/// Versorgungsfähre nach Bedarf: Je Gut der Anteil der Fährleistung (die Hälfte; Erz und Kristall je zwei Teile,
/// Deuterium ein Teil), solange der Mondbestand unter dem Bedarf des nächsten Schritts liegt (höchstens das Mondlager); sonst
/// ruht sie. Die Fähre folgt der Mondbasis (vorher einmalig mit einem Viertel des
/// Deckels von Stufe 1 gesetzt) und parkt keine Vorräte, die der Planet selbst braucht. Sie holt stündlich, auch wenn der Bot
/// sein Erz in seinem Zug schon ausgegeben hat – so kommt der Mond an Erz, das die Lieferung im Zug nicht mehr findet (20.10.2026).
fn faehre(w: &mut Welt, sid: SpielerId, mid: MondId, typ: Bottyp) {
    let Some(r) = w.monde_regeln() else { return };
    let m = &w.monde.monde[&mid];
    let basis = m.gebaeude[MondGebaeude::Mondbasis.idx()] as i64;
    if basis == 0 { return; }
    let deckel = r.faehre_je_mondbasis_stunde * basis / 2;
    let lager = w.mond_lager(mid);
    // Bedarf: Kosten des ersten Planschritts ohne andere Hürde, höchstens so viel, wie das Mondlager fasst (Mondbasis 1 lagert
    // 6.400 Erz, Mondbasis 2 kostet 14.500: die Fähre füllt das Lager, den Rest bringt ein Transport), dazu Betriebsdeuterium.
    let mut bedarf = [0i64; GUETER];
    for (g, ziel) in plan(typ, m.felder_max, kruemmer_dock(w, sid, mid, typ)) {
        if m.gebaeude[g.idx()] >= ziel || !matches!(huerde(w, sid, mid, g), Huerde::Keine) { continue; }
        let k = r.kosten(g, m.gebaeude[g.idx()] + 1);
        for i in 0..GUETER { bedarf[i] = k[i].min(lager[i]); }
        break;
    }
    if m.gebaeude[MondGebaeude::Tiefenphalanx.idx()] > 0 { bedarf[Gut::Deuterium.idx()] = bedarf[Gut::Deuterium.idx()].max(r.tiefenphalanx.scan_deuterium * M * 2); }
    let koord = m.koord.to_string();
    let bestand = m.bestand;
    // Spart der Heimatplanet für den Aufstieg, fährt die Fähre nur, was er über die Aufstiegskosten hinaus doppelt hat.
    let planet = w.bestand_jetzt(m.mutter as usize);
    let reserve = mond_ruecklage(w, sid, m.mutter as usize);
    for (gut, teile) in [(Gut::Erz, 2), (Gut::Kristall, 2), (Gut::Deuterium, 1)] {
        let i = gut.idx();
        let ueberschuss = if reserve[i] > 0 { (planet[i] - reserve[i]).max(0) >= 2 * bedarf[i] } else { true };
        let soll = if bestand[i] < bedarf[i] && ueberschuss { deckel * teile / 5 } else { 0 };
        let ist = w.monde.faehren.iter().find(|f| f.mond == mid && f.gut == gut).map(|f| f.menge_je_stunde).unwrap_or(0);
        if soll != ist && deckel >= 50 { tu(w, sid, json!({"typ":"mond_faehre","mond":koord,"gut":gut.name(),"menge_je_stunde":soll})); }
    }
}

/// Deuterium, das der Mond für seinen nächsten Bauschritt (Plan wie `ausbauen`, mit der Regel „das letzte Feld gehört der
/// Mondbasis“) behält, auf dem Krümmermond mit fertigem Werftdock das Deuterium des Weltenkrümmers.
fn mond_deuterium_bedarf(w: &Welt, sid: SpielerId, mid: MondId, typ: Bottyp) -> i64 {
    let Some(r) = w.monde_regeln() else { return 0 };
    let m = &w.monde.monde[&mid];
    let d = Gut::Deuterium.idx();
    let dock = kruemmer_dock(w, sid, mid, typ);
    if dock.is_some_and(|z| m.gebaeude[MondGebaeude::Werftdock.idx()] >= z) && kruemmer_gesamt(w, sid) == 0 {
        return w.regeln.kosten_einheit(Einheit::Weltentruemmer, w.spieler[sid as usize].volk)[d];
    }
    let plan = plan(typ, m.felder_max, dock);
    let mondbasis_ziel = plan.iter().filter(|(g, _)| *g == MondGebaeude::Mondbasis).map(|(_, z)| *z).max().unwrap_or(0);
    let (belegt, frei, _) = w.mond_felder(mid);
    for (g, ziel) in plan {
        if m.gebaeude[g.idx()] >= ziel || !matches!(huerde(w, sid, mid, g), Huerde::Keine) { continue; }
        if g != MondGebaeude::Mondbasis && m.gebaeude[MondGebaeude::Mondbasis.idx()] < mondbasis_ziel && frei.saturating_sub(belegt) <= 1 { continue; }
        return r.kosten(g, m.gebaeude[g.idx()] + 1)[d];
    }
    0
}

/// Tiefenphalanx der Räuber: ein Scan kostet 5.000 Deuterium bei 5 Minuten Abklingzeit, also bis 60.000 je Stunde – mehr, als
/// Fähre und Mondförderung bringen. Gescannt wird nur aus dem Deuterium über dem Bedarf des nächsten Bauschritts (Fortsetzung der
/// r49-Welt, Tag 150: der Krümmermond eines Räubers mit fertiger Doktrin hielt 1.500 bis 4.700 Deuterium bei 1.600 je Stunde
/// Fähre, der Mondbasis 9 fehlten 196.000 – das Werftdock 8 kam nie).
fn phalanx(w: &mut Welt, sid: SpielerId, eigene: &[MondId]) {
    for mid in eigene {
        let sicht = w.mond_sicht(*mid);
        let s = &sicht["tiefenphalanx"];
        if s["stufe"].as_u64().unwrap_or(0) == 0 || s["bereit_ab"].as_i64().unwrap_or(0) > w.zeit { continue; }
        let scan = s["scan_deuterium"].as_i64().unwrap_or(5000) * M;
        if w.monde.monde[mid].bestand[Gut::Deuterium.idx()] < mond_deuterium_bedarf(w, sid, *mid, Bottyp::Raeuber) + scan {
            zaehle("monde: phalanx spart deuterium");
            continue;
        }
        let reichweite = s["reichweite_systeme"].as_i64().unwrap_or(0);
        let k = w.monde.monde[mid].koord;
        let ziel = w.planeten.iter().filter(|p| p.besitzer != sid && w.planet_aktiv(p.id as usize) && p.koord.sektor == k.sektor
            && (p.koord.system as i64 - k.system as i64).abs() <= reichweite).map(|p| p.koord).min_by_key(|z| (z.system as i64 - k.system as i64).abs());
        if let Some(z) = ziel { tu(w, sid, json!({"typ":"phalanx_scannen","mond":k.to_string(),"ziel":z.to_string()})); }
    }
}

fn freigeben(w: &mut Welt, sid: SpielerId, eigene: &[MondId]) {
    if w.spieler[sid as usize].allianz.is_none() { return; }
    for mid in eigene {
        let m = &w.monde.monde[mid];
        if m.gebaeude[MondGebaeude::Sprungtor.idx()] > 0 && !w.monde.freigaben.contains(mid) {
            let k = m.koord.to_string();
            tu(w, sid, json!({"typ":"sprungtor_freigabe","mond":k,"freigegeben":true}));
        }
    }
}

/// Verlegt Kampfschiffe zwischen zwei eigenen (oder freigegebenen verbündeten) Toren, wenn ein Ende
/// deutlich mehr Flotte hat; offene Löcher anderer, die der Bot sieht, nutzt er zum eigenen Ende hin.
fn sprungtor(w: &mut Welt, sid: SpielerId, eigene: &[MondId], typ: Bottyp) {
    let kampf = |e: &Einheit| e.ist_schiff() && !matches!(e, Einheit::Spionagesonde | Einheit::Weltentruemmer | Einheit::Kolonieschiff | Einheit::GrossesKolonieschiff) && w.regeln.einh(*e).angriff > 0;
    let schiffe_am = |w: &Welt, mid: MondId| -> i64 {
        let m = &w.monde.monde[&mid];
        let p = &w.planeten[m.mutter as usize];
        Einheit::ALLE.iter().filter(|e| kampf(e)).map(|e| m.einheiten[e.idx()] + if p.besitzer == sid { p.einheiten[e.idx()] } else { 0 }).sum()
    };
    // Eigene offene Löcher, die schon stehen, nutzt der Bot nicht doppelt.
    if w.monde.loecher.values().any(|l| l.oeffner == sid) { return; }
    let tore: Vec<MondId> = eigene.iter().copied().filter(|m| w.mond_sicht(*m)["sprungtor"]["bereit"] == true).collect();
    let mut ziele: Vec<MondId> = tore.clone();
    for m in w.monde.monde.values() {
        if m.besitzer != sid && w.monde.freigaben.contains(&m.id) && w.verbuendet(sid, m.besitzer) && m.gebaeude[MondGebaeude::Sprungtor.idx()] > 0 && w.zeit >= m.sprungtor_bereit { ziele.push(m.id); }
    }
    let _ = typ;
    for von in &tore {
        for nach in &ziele {
            if von == nach { continue; }
            let (a, b) = (schiffe_am(w, *von), schiffe_am(w, *nach));
            if a < 10 || a < 2 * b + 10 { continue; }
            let m = &w.monde.monde[von];
            let (k, gegen) = (m.koord, w.monde.monde[nach].koord);
            let eigen_ziel = w.monde.monde[nach].besitzer == sid;
            // Hälfte der Kampfschiffe des Mutterplaneten (bzw. des Mondes), Kapazität beachtet.
            let p = &w.planeten[m.mutter as usize];
            let (art, quelle) = if Einheit::ALLE.iter().filter(|e| kampf(e)).any(|e| m.einheiten[e.idx()] > 0) { ("mond", m.einheiten) } else { ("planet", p.einheiten) };
            let stufe = m.gebaeude[MondGebaeude::Sprungtor.idx()].min(w.monde.monde[nach].gebaeude[MondGebaeude::Sprungtor.idx()]) as i64;
            let mut kap = w.monde_regeln().map(|r| r.hyperraumloch.kapazitaet_je_stufe).unwrap_or(0) * stufe;
            let mut auswahl = serde_json::Map::new();
            for e in Einheit::ALLE.iter().filter(|e| kampf(e)) {
                let st = w.regeln.einh(*e).struktur.max(1);
                let n = (quelle[e.idx()] / 2).min(kap / st);
                if n > 0 { auswahl.insert(e.name().into(), json!(n)); kap -= n * st; }
            }
            if auswahl.is_empty() { continue; }
            let danach = if eigen_ziel { "landen" } else { "orbit" };
            tu(w, sid, json!({"typ":"hyperraumloch_oeffnen","mond":k.to_string(),"gegenstelle":gegen.to_string(),
                "erstdurchreise":{"von":{"art":art,"koord":k.to_string()},"schiffe":auswahl,"danach":danach}}));
            return;
        }
    }
}

// ---------------------------------------------------------------- Weltenkrümmer

/// Bauen und einsetzen wollen nur die kriegerischen Typen: der Räuber gegen jeden zulässigen Rivalen, der Igel nur als
/// Vergeltung gegen Reiche, die ihn angegriffen haben. Ökonom und Händler bleiben bei der Wirtschaft – ein Weltenkrümmer
/// kostet 5 Mio. Erz, 4 Mio. Kristall und hat keinen friedlichen Nutzen (20.10.2026).
fn kruemmer_typ(typ: Bottyp) -> bool { matches!(typ, Bottyp::Raeuber | Bottyp::Igel) }

/// Der Mond, der das Werftdock bis zur Krümmerstufe (Regel, seit 21.10.2026 Stufe 8) bekommt: bei Räubern und Igeln der Mond mit
/// den meisten Feldern (bei Gleichstand der älteste). Ein Reich baut höchstens einen Weltenkrümmer, ein zweites Krümmerdock wäre verschenkt.
fn kruemmer_mond(w: &Welt, sid: SpielerId, mid: MondId, typ: Bottyp) -> bool {
    kruemmer_typ(typ) && w.monde.monde.values().filter(|m| m.besitzer == sid).max_by_key(|m| (m.felder_max, std::cmp::Reverse(m.id))).is_some_and(|m| m.id == mid)
}

/// Legierung und Elektronik, auf die ein Krümmerreich hinarbeitet (Markt in bots.rs kauft sie und verkauft sie nicht): ab
/// drei Stufen unter der Krümmerstufe auf dem Krümmermond die fehlenden Werftdock-Stufen bis dahin und der Weltenkrümmer selbst (100.000 Legierung,
/// 50.000 Elektronik); mit einem Krümmer im Reich nichts. Was schon auf dem Mond liegt, zählt mit.
pub(super) fn kruemmer_bedarf(w: &Welt, sid: SpielerId, typ: Bottyp) -> [i64; GUETER] {
    let mut bedarf = [0i64; GUETER];
    let Some(r) = w.monde_regeln() else { return bedarf };
    if !kruemmer_typ(typ) || kruemmer_gesamt(w, sid) > 0 { return bedarf; }
    let Some(m) = w.monde.monde.values().filter(|m| m.besitzer == sid).max_by_key(|m| (m.felder_max, std::cmp::Reverse(m.id))) else { return bedarf };
    let dock = m.gebaeude[MondGebaeude::Werftdock.idx()];
    if dock + 3 < r.weltentruemmer_werftdock { return bedarf; }
    for stufe in dock + 1..=r.weltentruemmer_werftdock {
        let k = r.kosten(MondGebaeude::Werftdock, stufe);
        for i in 0..GUETER { bedarf[i] += k[i]; }
    }
    // Die Doktrin (Fortsetzung bot_lage 712, Tag 80: Werftdock 10 stand, aber Hyperraumantrieb 5 kostet 16.000 Xenokristall –
    // 11.225-mal „forschung unbezahlbar“): jede fehlende Stufe der Voraussetzungen zählt mit.
    let sp = &w.spieler[sid as usize];
    if let Some(d) = w.regeln.flottendoktrin.get(&Einheit::Weltentruemmer) {
        for (f, ziel) in &d.voraussetzungen {
            for stufe in sp.forschung[f.idx()] + 1..=*ziel {
                let k = w.regeln.kosten_forschung(*f, stufe);
                for i in 0..GUETER { bedarf[i] += k[i]; }
            }
        }
    }
    let wk = w.regeln.kosten_einheit(Einheit::Weltentruemmer, sp.volk);
    for g in [Gut::Legierung, Gut::Elektronik, Gut::Xenokristall] { bedarf[g.idx()] = (bedarf[g.idx()] + wk[g.idx()] - m.bestand[g.idx()]).max(0); }
    for g in Gut::ALLE { if !matches!(g, Gut::Legierung | Gut::Elektronik | Gut::Xenokristall) { bedarf[g.idx()] = 0; } }
    bedarf
}

/// Die nächste fehlende Stufe der Krümmer-Doktrin (Hyperraumantrieb, Waffentechnik), sobald der Krümmermond Werftdock 7 hat
/// und das Reich noch keinen Krümmer besitzt – für Rücklage und Logistik in bots.rs. Fortsetzung bot_lage 712, Tag 90: ein
/// Räuber mit Werftdock 10 und 167.000 Kristall daheim kam nie an Hyperraumantrieb 4 (160.000 Kristall, verlangt 60 % Deckung),
/// weil er jeden Überschuss in Schiffe steckte.
pub(super) fn kruemmer_forschung(w: &Welt, sid: SpielerId) -> Option<(Forschung, u8)> {
    // Werftdock 7 hat nur der Krümmermond eines Räubers oder Igels (die anderen planen höchstens Stufe 4).
    if kruemmer_gesamt(w, sid) > 0 { return None; }
    let m = w.monde.monde.values().filter(|m| m.besitzer == sid).max_by_key(|m| (m.felder_max, std::cmp::Reverse(m.id)))?;
    if m.gebaeude[MondGebaeude::Werftdock.idx()] + 3 < w.monde_regeln()?.weltentruemmer_werftdock { return None; }
    let sp = &w.spieler[sid as usize];
    let d = w.regeln.flottendoktrin.get(&Einheit::Weltentruemmer)?;
    d.voraussetzungen.iter().find(|(f, n)| sp.forschung[f.idx()] < **n && sp.forschung_aktiv.as_ref().is_none_or(|a| a.forschung != **f))
        .map(|(f, _)| (*f, sp.forschung[f.idx()] + 1))
}

/// Was dem Krümmermond nach der Doktrin für seine nächste Stufe fehlt (abzüglich Mondbestand): die nächste Werftdock-Stufe bis
/// zur Krümmerstufe, die Mondbasis, wenn ihr das letzte Feld gehört, danach der Weltenkrümmer selbst ohne Antriebskerne (die baut
/// die Orbitalwerft). Die Heimat legt das vor Gebäuden, Schiffen und Forschung zurück, nicht vor der Mondlieferung selbst – die
/// bringt es auf den Mond (bots.rs `kruemmer_spar`). Anlass: Fortsetzung der r48-Welt, Tag 186 – ein Räuber mit fertiger
/// Doktrin und Werftdock 7, Heimat 1.242 Erz und 0 Deuterium; der Mondbasis 9 für das letzte Feld fehlten 196.000 Deuterium,
/// alles andere hatte die Heimat verbaut und verflogen. Bis Tag 170 hatten die beiden Räuber mit Doktrin und Krümmerdock je
/// rund 100.000 Erz daheim – 5 Mio. Erz, 4 Mio. Kristall und 1 Mio. Deuterium spart so keiner. Ein Igel (Ortungsschleier statt
/// Tiefenphalanx auf dem Mond) spart den Krümmer erst, wenn es einen Anlass gibt (`kruemmer_bereit`).
pub(super) fn kruemmer_schritt_fehlt(w: &Welt, sid: SpielerId) -> [i64; GUETER] {
    let mut v = [0i64; GUETER];
    if kruemmer_gesamt(w, sid) > 0 { return v; }
    let Some(r) = w.monde_regeln() else { return v };
    let Some(m) = w.monde.monde.values().filter(|m| m.besitzer == sid).max_by_key(|m| (m.felder_max, std::cmp::Reverse(m.id))) else { return v };
    let dock = m.gebaeude[MondGebaeude::Werftdock.idx()];
    if dock + 3 < r.weltentruemmer_werftdock || !m.bauschleife.is_empty() { return v; }
    if !w.einheit_forschung_fehlend(sid, Einheit::Weltentruemmer).is_empty() { return v; }
    if dock >= r.weltentruemmer_werftdock {
        let igel = m.gebaeude[MondGebaeude::Ortungsschleier.idx()] > 0 && m.gebaeude[MondGebaeude::Tiefenphalanx.idx()] == 0;
        if igel && kruemmer_ziele(w, sid, Bottyp::Igel, 1, &super::Opferschutz::default()).is_empty() { return v; }
        let mut kosten = w.regeln.kosten_einheit(Einheit::Weltentruemmer, w.spieler[sid as usize].volk);
        kosten[Gut::Antriebskern.idx()] = 0;
        for i in 0..GUETER { v[i] = (kosten[i] - m.bestand[i]).max(0); }
        return v;
    }
    let (belegt, frei, max) = w.mond_felder(m.id);
    let g = if frei.saturating_sub(belegt) > 1 { MondGebaeude::Werftdock } else if frei < max { MondGebaeude::Mondbasis } else if belegt < frei { MondGebaeude::Werftdock } else { return v };
    let kosten = r.kosten(g, m.gebaeude[g.idx()] + 1);
    for i in 0..GUETER { v[i] = (kosten[i] - m.bestand[i]).max(0); }
    v
}

/// Werftdock-Stufe des Weltenkrümmers (Regel `weltentruemmer_werftdock`, seit 21.10.2026 8), wenn `mid` der Krümmermond ist.
fn kruemmer_dock(w: &Welt, sid: SpielerId, mid: MondId, typ: Bottyp) -> Option<u8> {
    kruemmer_mond(w, sid, mid, typ).then(|| w.monde_regeln().map(|r| r.weltentruemmer_werftdock).unwrap_or(10))
}

/// Mutterplanet des Krümmermonds, sobald dessen Werftdock nahe am Krümmer ist (Regelstufe − 3) und das Reich noch keinen hat.
pub(super) fn kruemmer_mutter(w: &Welt, sid: SpielerId, typ: Bottyp) -> Option<usize> {
    if !kruemmer_typ(typ) || kruemmer_gesamt(w, sid) > 0 { return None; }
    let r = w.monde_regeln()?;
    let m = w.monde.monde.values().filter(|m| m.besitzer == sid).max_by_key(|m| (m.felder_max, std::cmp::Reverse(m.id)))?;
    (m.gebaeude[MondGebaeude::Werftdock.idx()] + 3 >= r.weltentruemmer_werftdock).then_some(m.mutter as usize)
}

fn kruemmer_gesamt(w: &Welt, sid: SpielerId) -> i64 {
    let wk = Einheit::Weltentruemmer;
    w.monde.monde.values().filter(|m| m.besitzer == sid).map(|m| m.einheiten[wk.idx()] + m.fertigung.iter().filter(|f| f.einheit == wk).map(|f| f.rest).sum::<i64>()).sum::<i64>()
        + w.flotten.values().filter(|f| f.besitzer == sid).map(|f| f.schiffe[wk.idx()]).sum::<i64>()
}

/// Bereit zum Bau: Werftdock auf der Krümmerstufe (Regel, seit 21.10.2026 Stufe 8) frei, Stufe V, Doktrin (Voraussetzungen aus der Regel, seit 20.10.2026 Hyperraumantrieb 5, Waffentechnik 8) da, die 5.000 Mann
/// Besatzung auf dem Mutterplaneten, noch kein Krümmer im Reich, bei Igeln ein Anlass (ein Angreifer mit Mond).
fn kruemmer_bereit(w: &Welt, sid: SpielerId, mid: MondId, typ: Bottyp) -> bool {
    if !kruemmer_typ(typ) { return false; }
    let (Some(r), Some(er)) = (w.monde_regeln(), w.regeln.einheiten.get(&Einheit::Weltentruemmer)) else { return false };
    let m = &w.monde.monde[&mid];
    m.gebaeude[MondGebaeude::Werftdock.idx()] >= r.weltentruemmer_werftdock && m.fertigung.is_empty() && kruemmer_gesamt(w, sid) < 1
        && w.spieler[sid as usize].stufe >= er.ab_stufe && w.einheit_forschung_fehlend(sid, Einheit::Weltentruemmer).is_empty()
        && w.planeten[m.mutter as usize].besitzer == sid && w.planeten[m.mutter as usize].bevoelkerung >= (er.besatzung + 500) * M
        && (typ != Bottyp::Igel || !kruemmer_ziele(w, sid, typ, 1, &super::Opferschutz::default()).is_empty())
}

/// Ein Weltenkrümmer je Reich, sobald das Werftdock die Krümmerstufe hat, Stufe V und die Doktrin stehen und Mond plus Mutterplanet
/// die Kosten tragen. Die Bestellung zahlt alles auf einmal. Die Antriebskerne (20 Stück, Orbitalwerft des Planeten) werden
/// gleich bestellt, sie sind billig und brauchen Zeit; das Übrige schafft der Bot auf den Mond, wo seine eigenen Käufe es nicht
/// wegnehmen. Die Forschung reiht er ein, sobald das Werftdock drei Stufen unter der Krümmerstufe steht (seit 21.10.2026 Stufe 5).
fn kruemmer_fertigen(w: &mut Welt, sid: SpielerId, mid: MondId, typ: Bottyp) {
    if !kruemmer_typ(typ) { return; }
    let Some(r) = w.monde_regeln() else { return };
    let wk = Einheit::Weltentruemmer;
    let Some(er) = w.regeln.einheiten.get(&wk).cloned() else { return };
    let (dock, koord, mutter) = { let m = &w.monde.monde[&mid]; (m.gebaeude[MondGebaeude::Werftdock.idx()], m.koord, m.mutter as usize) };
    if kruemmer_gesamt(w, sid) >= 1 || !w.monde.monde[&mid].fertigung.is_empty() { return; }
    if dock + 3 >= r.weltentruemmer_werftdock && w.spieler[sid as usize].stufe >= er.ab_stufe {
        let doktrin = w.regeln.flottendoktrin.get(&wk).map(|p| p.voraussetzungen.clone()).unwrap_or_default();
        for (f, n) in doktrin { if w.spieler[sid as usize].forschung[f.idx()] < n && forschung_anstossen(w, sid, f, n) { break; } }
    }
    if dock < r.weltentruemmer_werftdock { return; }
    if w.spieler[sid as usize].stufe < er.ab_stufe { zaehle("monde: krümmer: stufe fehlt"); return; }
    if !w.einheit_forschung_fehlend(sid, wk).is_empty() { zaehle("monde: krümmer: forschung fehlt"); return; }
    if !kruemmer_bereit(w, sid, mid, typ) { zaehle("monde: krümmer: kein anlass"); return; }
    let kosten = w.regeln.kosten_einheit(wk, w.spieler[sid as usize].volk);
    let planet = w.bestand_jetzt(mutter);
    let bestand = w.monde.monde[&mid].bestand;
    let ak = Gut::Antriebskern.idx();
    let fehlend: Vec<Gut> = Gut::ALLE.iter().copied().filter(|g| bestand[g.idx()] + planet[g.idx()] < kosten[g.idx()]).collect();
    if fehlend.is_empty() {
        let ok = tu(w, sid, json!({"typ":"mond_fertigen","mond":koord.to_string(),"einheit":wk.name(),"anzahl":1}));
        zaehle(if ok { "monde: krümmer bestellt" } else { "monde: krümmer abgelehnt" });
        return;
    }
    // Antriebskerne fertigt die Orbitalwerft. Hat der Mutterplanet keine (oft eine Kolonie), baut die Heimat sie, und die
    // Mondlieferung nimmt sie mit (frische Welt 712, r54, Tag 120: die ganzen Krümmerkosten lagen auf dem Mond, 96-mal „wartet auf
    // antriebskerne“ – die Bestellung auf der Kolonie ohne Orbitalwerft scheiterte stumm).
    let heimat = w.spieler[sid as usize].heimat as usize;
    let werk = if w.planeten[mutter].gebaeude[Gebaeude::Orbitalwerft.idx()] >= 1 { mutter } else { heimat };
    // Ohne Orbitalwerft auf dem Werkplaneten keine Antriebskerne: die Werft gehört zur Krümmerkette und kommt vor der Rücklage
    // (die hält sonst auch die 8.000 Erz zurück, die sie kostet); bots.rs `ziele_planet` hat sie auch in den Bauzielen.
    if fehlend.contains(&Gut::Antriebskern) && w.planeten[werk].gebaeude[Gebaeude::Orbitalwerft.idx()] == 0
        && !w.planeten[werk].bauschleife.iter().any(|a| a.gebaeude == Gebaeude::Orbitalwerft) {
        let ok = tu(w, sid, json!({"typ":"bauen","planet":w.planeten[werk].koord.to_string(),"gebaeude":Gebaeude::Orbitalwerft.name()}));
        zaehle(if ok { "monde: krümmer: orbitalwerft gebaut" } else { "monde: krümmer: orbitalwerft abgelehnt" });
    }
    if fehlend.contains(&Gut::Antriebskern) {
        let p = &w.planeten[werk];
        let ks = p.koord.to_string();
        let vorrat = w.bestand_jetzt(werk);
        let bestellt: i64 = p.fertigung[1].iter().filter(|f| f.produkt == Produkt::Bauteil(Gut::Antriebskern)).map(|f| f.rest).sum();
        let daheim = if werk != mutter { vorrat[ak] } else { 0 };
        let n = (kosten[ak] - bestand[ak] - planet[ak] - daheim) / M - bestellt;
        if n > 0 {
            let stueck = w.regeln.kosten_bauteil(Gut::Antriebskern).unwrap_or([0; GUETER]);
            let bezahlbar = (0..GUETER).filter(|g| stueck[*g] > 0).map(|g| vorrat[g] / stueck[g]).min().unwrap_or(0);
            if bezahlbar > 0 {
                let ok = tu(w, sid, json!({"typ":"fertigen","planet":ks,"bauteil":"antriebskern","anzahl":n.min(bezahlbar)}));
                zaehle(if ok { "monde: antriebskerne bestellt" } else { "monde: antriebskerne abgelehnt" });
            } else { zaehle("monde: antriebskerne unbezahlbar"); }
        }
    }
    kruemmer_tauschen(w, sid);
    // Sparen: alles andere auf den Mond holen – die Antriebskerne auch, wenn die Heimat sie baut.
    let mut fehlt_mond = [0i64; GUETER];
    for g in Gut::ALLE { if g != Gut::Antriebskern || werk != mutter { fehlt_mond[g.idx()] = (kosten[g.idx()] - bestand[g.idx()] - if g == Gut::Antriebskern { planet[ak] } else { 0 }).max(0); } }
    if fehlt_mond.iter().all(|x| *x == 0) { zaehle("monde: krümmer: wartet auf antriebskerne"); return; }
    zaehle("monde: krümmer: spart");
    nachschub(w, sid, mid, fehlt_mond, SPAR_PROMILLE);
}

/// Deuterium ist beim Weltenkrümmer der Engpass (Fortsetzung der r49-Welt mit r53, Tag 200: Mond 5,0 Mio. Erz und 4,0 Mio.
/// Kristall, aber 547.000 von 1 Mio. Deuterium, rund 40.000 je Tag dazu): Was die Heimat an Erz und Kristall über dem noch
/// Fehlenden hat, tauscht sie im Soforthandel gegen das fehlende Deuterium (Kurse der Kernwerte, `tauschfaktor`, ein Zehntel
/// des Lagers bleibt); die Lieferung bringt es auf den Mond.
fn kruemmer_tauschen(w: &mut Welt, sid: SpielerId) {
    let fehlt = kruemmer_schritt_fehlt(w, sid);
    let (e, k, d) = (Gut::Erz.idx(), Gut::Kristall.idx(), Gut::Deuterium.idx());
    let heimat = w.spieler[sid as usize].heimat as usize;
    let (hb, grenze) = (w.bestand_jetzt(heimat), w.lagergrenze(heimat));
    let mut d_rest = (fehlt[d] - hb[d]) as f64;
    if d_rest <= 0.0 { return; }
    let raum = (mal(grenze[d], 0.9) - hb[d]).max(0) as f64;
    d_rest = d_rest.min(raum);
    let mut angebot = serde_json::Map::new();
    for (i, g) in [(e, Gut::Erz), (k, Gut::Kristall)] {
        if d_rest < M as f64 { break; }
        let f = super::dyson_bot::tauschfaktor(w, g, Gut::Deuterium);
        if f <= 0.0 { continue; }
        let frei = (hb[i] - fehlt[i] - grenze[i] / 10).max(0) as f64;
        let menge = ((d_rest / f).min(frei) as i64) / M;
        if menge < 1000 { continue; }
        angebot.insert(g.name().into(), json!(menge));
        d_rest -= (menge * M) as f64 * f;
    }
    if angebot.is_empty() { return; }
    let koord = w.planeten[heimat].koord.to_string();
    if tu(w, sid, json!({"typ":"soforthandel","planet":koord,"angebot":angebot,"ziel":"deuterium"})) { zaehle("monde: krümmer: erz/kristall gegen deuterium"); }
}

/// Zulässige Ziele für `n` Weltenkrümmer: bekannte Monde anderer Reiche (Kartensicht), deren Besitzer aktiv, nicht verbündet,
/// ohne Pakt, kein Neuling (Stufe III und höher), nicht unter Schutz und in 24 Stunden in keinen fremden Kampf verwickelt ist
/// (wie der Opferschutz der Räuber; eigene Überfälle des Räubers zählen nicht). Der Igel greift nur Reiche an, die ihn angegriffen
/// haben. Die Zerstörungschance muss mindestens 25 % je Versuch betragen. Beste zuerst: höhere Chance, dann näher.
fn kruemmer_ziele(w: &Welt, sid: SpielerId, typ: Bottyp, n: i64, schutz: &super::Opferschutz) -> Vec<(i64, i64, MondId)> {
    let Some(r) = w.monde_regeln() else { return Vec::new() };
    let von = w.planeten[w.spieler[sid as usize].heimat as usize].koord;
    let mut aus = Vec::new();
    for m in w.monde.monde.values().filter(|m| m.besitzer != sid) {
        let o = &w.spieler[m.besitzer as usize];
        if !w.spieler_aktiv(o.id) || o.stufe < 3 || w.zeit < o.schutz_bis || w.verbuendet(sid, o.id) { continue; }
        if w.vertrag_zwischen(sid, o.id, Vertragsart::Nichtangriffspakt) { continue; }
        if typ == Bottyp::Igel && !o.angegriffen.contains(&sid) { continue; }
        // Monde von Mitspielern nur als Antwort: wenn der Mitspieler diesen Bot selbst angegriffen hat.
        if super::mitspieler(schutz, o.id) && !o.angegriffen.contains(&sid) { continue; }
        if w.mond_kartensicht(sid, m.koord).is_none() { continue; }
        if w.kampfberichte.iter().rev().take_while(|k| w.zeit - k.zeit < 24 * STUNDE).any(|k| (k.verteidiger.contains(&o.id) && k.angreifer != sid) || k.angreifer == o.id) { continue; }
        if w.monde.flottenziel.iter().any(|(f, z)| *z == m.id && w.flotten.get(f).is_some_and(|fl| fl.besitzer == sid)) { continue; }
        let (zerst, _) = r.zerstoerung_promille(m.durchmesser, m.typ, n);
        if zerst < 250 { continue; }
        aus.push((zerst, -w.entfernung(von, m.koord), m.id));
    }
    aus.sort_by(|a, b| b.cmp(a));
    aus
}

/// Der Weltenkrümmer verlässt den Mond nur gegen ein ausgespähtes Ziel: Ohne frischen Mondspionagebericht (24 h)
/// schickt der Bot fünf Sonden, mit Bericht rechnet er den Kampf gegen die gemeldeten Schiffe und Geschütze durch und
/// fliegt bei sicherem Sieg mit `mond_zerstoeren`. Der Mond zahlt Treibstoff für Hin- und Rückflug.
fn kruemmer_einsatz(w: &mut Welt, sid: SpielerId, typ: Bottyp, eigene: &[MondId], schutz: &super::Opferschutz) {
    if !kruemmer_typ(typ) { return; }
    let wk = Einheit::Weltentruemmer;
    if w.flotten.values().any(|f| f.besitzer == sid && f.schiffe[wk.idx()] > 0) { return; }
    let Some(&mid) = eigene.iter().find(|m| w.monde.monde[*m].einheiten[wk.idx()] > 0) else { return };
    // Jeder Versuch kann den Krümmer kosten (Verlustrisiko ab 30 % bei 4.000 km): drei Tage Pause nach einem Ausgang.
    if w.spieler[sid as usize].vorfaelle.iter().rev().take_while(|v| w.zeit - v.zeit < 3 * TAG).any(|v| v.art.starts_with("mond_zerstoer")) {
        zaehle("monde: krümmer: pause nach versuch");
        return;
    }
    if w.flotten.values().filter(|f| f.besitzer == sid).count() >= w.flottenplaetze(sid) { zaehle("monde: krümmer: keine flottenplätze"); return; }
    let (n, mkoord, mutter) = { let m = &w.monde.monde[&mid]; (m.einheiten[wk.idx()], m.koord, m.mutter as usize) };
    let ziele = kruemmer_ziele(w, sid, typ, n, schutz);
    if ziele.is_empty() { zaehle("monde: krümmer: kein ziel"); return; }
    let heimat = w.spieler[sid as usize].heimat as usize;
    for (_, _, tid) in ziele.into_iter().take(3) {
        let t = w.monde.monde[&tid].clone();
        let bericht = w.monde.spionageberichte.iter().rev().find(|b| b.spieler == sid && b.mond == tid && w.zeit - b.zeit < 24 * STUNDE).cloned();
        let Some(b) = bericht else {
            // Ausspähen: fünf Sonden vom Heimatplaneten, falls vorhanden, sonst bestellen.
            let p = &w.planeten[heimat];
            if w.flotten.values().any(|f| f.besitzer == sid && f.ziel == t.koord && f.mission == Mission::Spionage) { return; }
            if p.einheiten[Einheit::Spionagesonde.idx()] >= 5 {
                let ok = tu(w, sid, json!({"typ":"flotte_senden","start":p.koord.to_string(),"ziel":t.koord.to_string(),"zielart":"mond","mission":"spionage","schiffe":{"spionagesonde":5}}));
                zaehle(if ok { "monde: krümmer: mondspionage" } else { "monde: krümmer: spionage abgelehnt" });
            } else if p.fertigung[0].is_empty() {
                let ks = p.koord.to_string();
                tu(w, sid, json!({"typ":"fertigen","planet":ks,"einheit":"spionagesonde","anzahl":5}));
                zaehle("monde: krümmer: sonden bestellt");
            }
            return;
        };
        let (Some(schiffe), Some(verteidigung)) = (b.schiffe.as_ref(), b.verteidigung.as_ref()) else { zaehle("monde: krümmer: bericht ohne bestand"); continue };
        // Ein Mond ohne nennenswerte Gebäude lohnt den Einsatz nicht, außer der Igel rächt sich.
        let wert: u32 = b.gebaeude.as_ref().map(|g| g.values().map(|x| *x as u32).sum()).unwrap_or(0);
        if typ != Bottyp::Igel && wert < 3 { zaehle("monde: krümmer: mond ohne wert"); continue; }
        let mut vert = [0i64; EINHEITEN];
        for (name, anzahl) in schiffe.iter().chain(verteidigung.iter()) { if let Some(e) = Einheit::aus_name(name) { vert[e.idx()] += *anzahl; } }
        let r = w.regeln.clone();
        let sp = &w.spieler[sid as usize];
        let tech = |f: Forschung| 1.0 + r.kampf.tech_je_stufe * b.forschung.as_ref().and_then(|t| t.get(f.name())).map(|x| *x as f64).unwrap_or(sp.forschung[f.idx()] as f64);
        let mut ang = [0i64; EINHEITEN];
        ang[wk.idx()] = n;
        let a = Gruppe::neu(&r, sp, ang);
        let vol = r.volk(w.spieler[t.besitzer as usize].volk);
        let v = Gruppe::mit_werten(&r, t.besitzer, vert, tech(Forschung::Waffentechnik) * vol.waffen, tech(Forschung::Schildtechnik), tech(Forschung::Panzerung) * vol.panzerung);
        let mut siege = 0;
        for i in 0..6u64 {
            let nr = (w.zeit as u64) << 24 | (sid as u64) << 8 | 0xC0 | i;
            let mut rng = strom(w.startwert, KANAL_SIMULATOR, nr);
            if kampf_ohne_protokoll(&r, std::slice::from_ref(&a), std::slice::from_ref(&v), &mut rng).sieger == Sieger::Angreifer { siege += 1; }
        }
        if siege < 6 { zaehle("monde: krümmer: sieg unsicher"); continue; }
        let mut schiffe_arr = [0i64; SCHIFFE];
        schiffe_arr[wk.idx()] = n;
        let Ok(plan) = w.flugplan(sid, mutter, t.koord, &schiffe_arr, 1000) else { continue };
        // Der Krümmer fliegt langsam (Tempo 100): ein Ziel, das länger als zwei Tage entfernt ist, lohnt Treibstoff und Risiko nicht.
        if plan.dauer > 2 * TAG { zaehle("monde: krümmer: ziel zu weit"); continue; }
        let sprit = 2 * plan.treibstoff;
        if w.monde.monde[&mid].bestand[Gut::Deuterium.idx()] < sprit {
            let mut fehlt = [0i64; GUETER];
            fehlt[Gut::Deuterium.idx()] = sprit - w.monde.monde[&mid].bestand[Gut::Deuterium.idx()];
            nachschub(w, sid, mid, fehlt, ABGABE_PROMILLE);
            zaehle("monde: krümmer: treibstoff nachgeliefert");
            return;
        }
        let ok = tu(w, sid, json!({"typ":"flotte_senden","start":mkoord.to_string(),"startart":"mond","ziel":t.koord.to_string(),"zielart":"mond",
            "mission":"mond_zerstoeren","schiffe":{wk.name():n}}));
        zaehle(if ok { "monde: krümmer: einsatz gestartet" } else { "monde: krümmer: einsatz abgelehnt" });
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPEN: [Bottyp; 4] = [Bottyp::Raeuber, Bottyp::Igel, Bottyp::Haendler, Bottyp::Oekonom];

    /// Zielstufen des Plans je Gebäude.
    fn ziele(plan: &[(MondGebaeude, u8)]) -> [u8; MONDGEBAEUDE] {
        let mut z = [0u8; MONDGEBAEUDE];
        for (g, s) in plan { z[g.idx()] = z[g.idx()].max(*s); }
        z
    }

    /// Für jede Felderzahl passt der Plan in den Mond und in die Felder, die die Mondbasis freischaltet – sonst bliebe der
    /// letzte Bau für immer unmöglich (20.10.2026: Werftdock 10 braucht Mondbasis 5, sonst „kein freies Feld“).
    #[test]
    fn plan_passt_in_die_felder_und_zur_mondbasis() {
        for typ in TYPEN {
            for felder in 4..=60u16 {
                let z = ziele(&plan(typ, felder, kruemmer_typ(typ).then_some(10)));
                let summe: u16 = z.iter().map(|s| *s as u16).sum();
                assert!(summe <= felder, "{typ:?} {felder} Felder: Summe {summe}");
                assert!(summe <= 3 * z[MondGebaeude::Mondbasis.idx()] as u16 + 1, "{typ:?} {felder} Felder: Mondbasis {} trägt {summe} Stufen nicht", z[MondGebaeude::Mondbasis.idx()]);
            }
        }
    }

    /// Räuber und Igel bekommen Werftdock 10 ab 16 Felder (Mondbasis 5, Regolith 1), die anderen Typen nie.
    #[test]
    fn werftdock_zehn_nur_fuer_kriegerische_und_ab_sechzehn_felder() {
        for typ in [Bottyp::Raeuber, Bottyp::Igel] {
            assert!(!plan(typ, 15, Some(10)).contains(&(MondGebaeude::Werftdock, 10)), "{typ:?} mit 15 Feldern");
            let p = plan(typ, 16, Some(10));
            assert!(p.contains(&(MondGebaeude::Werftdock, 10)) && p.contains(&(MondGebaeude::Mondbasis, 5)), "{typ:?} mit 16 Feldern: {p:?}");
        }
        for typ in [Bottyp::Haendler, Bottyp::Oekonom] {
            assert!(!plan(typ, 60, None).iter().any(|(g, s)| *g == MondGebaeude::Werftdock && *s >= 10), "{typ:?} baut kein Werftdock 10");
        }
    }

    /// Der Grundausbau steht in jedem Plan vorn, das teure Sprungtor erst nach dem Werftdock.
    #[test]
    fn grundausbau_vor_dem_sprungtor() {
        for typ in TYPEN {
            let p = plan(typ, 30, kruemmer_typ(typ).then_some(10));
            let pos = |g: MondGebaeude| p.iter().position(|(x, _)| *x == g).unwrap();
            assert_eq!(p[0], (MondGebaeude::Mondbasis, 1), "{typ:?}");
            assert!(pos(MondGebaeude::Regolithfoerderung) < pos(MondGebaeude::Werftdock) && pos(MondGebaeude::Werftdock) < pos(MondGebaeude::Sprungtor), "{typ:?}: {p:?}");
        }
    }
}
