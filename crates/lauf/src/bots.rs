//! Skriptbots fürs Balancing: Ökonom, Räuber, Igel und Händler.
//!
//! Die Bots handeln über dieselben Aktionen wie die Agenten. Über andere Spieler wissen
//! sie nur, was öffentlich ist oder was ihre eigenen Sonden berichtet haben.

use kern::regeln::{preis_array, Regelwerk, MAX_FORSCHUNG};
use kern::kampf::{kampf, Gruppe};
use kern::typen::*;
use kern::welt::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bottyp {
    Oekonom,
    Raeuber,
    Igel,
    Haendler,
    /// Kontrolltyp für die Balance: ein Ökonom, der sich auch nach Plünderungen nie schützt.
    Wehrlos,
}

impl Bottyp {
    /// Die vier Typen des Konzepts, die Standardmischung.
    pub const ALLE: [Bottyp; 4] = [Bottyp::Oekonom, Bottyp::Raeuber, Bottyp::Igel, Bottyp::Haendler];
    pub fn name(self) -> &'static str {
        match self {
            Bottyp::Oekonom => "oekonom",
            Bottyp::Raeuber => "raeuber",
            Bottyp::Igel => "igel",
            Bottyp::Haendler => "haendler",
            Bottyp::Wehrlos => "wehrlos",
        }
    }
    pub fn aus_name(s: &str) -> Option<Bottyp> {
        Bottyp::ALLE.into_iter().chain([Bottyp::Wehrlos]).find(|t| t.name() == s)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bot {
    pub typ: Bottyp,
    pub naechster: SimZeit,
    /// Wann welches Ziel zuletzt ausgespäht wurde.
    pub gespaeht: BTreeMap<Koord, SimZeit>,
}

impl Bot {
    pub fn neu(typ: Bottyp, sid: SpielerId) -> Bot {
        // Versetzter Takt, damit nicht alle Bots im selben Fenster handeln.
        Bot { typ, naechster: (sid as i64 % 8) * 900, gespaeht: BTreeMap::new() }
    }
}

const TAKT: i64 = 2 * STUNDE;
/// A probed colony target that is not confirmed free stays off the list this long.
const KOLONIE_SPAEHPAUSE: i64 = 7 * TAG;

fn tu(welt: &mut Welt, sid: SpielerId, aktion: Value) -> bool {
    welt.handeln(sid, Rolle::Alle, &aktion).0
}

fn bezahlbar(bestand: &[i64; GUETER], kosten: &[i64; GUETER]) -> bool {
    (0..GUETER).all(|g| bestand[g] >= kosten[g])
}

/// Mindeststufen, die ein Bot auf seiner Heimatwelt oder Kolonie anstrebt, in dieser Reihenfolge.
fn ziele(r: &Regelwerk, stufe: u8, typ: Bottyp, heimat: bool, nebel: bool) -> Vec<(Gebaeude, u8)> {
    use Gebaeude::*;
    if !heimat {
        // Konsumgüterwerk früh: Kolonien haben wenig Bevölkerung und liefern den Überschuss für Habitatmodule heim.
        let mut v = vec![(Erzmine, 4), (Kristallmine, 4), (Lager, 2), (Raumhafen, 1), (Konsumgueterwerk, 4), (Bauhof, 1), (Deuteriumsynthesizer, 3)];
        if nebel {
            // Der Extraktor braucht Fachkräfte: ohne Akademie nimmt sie ein Fusionskraftwerk vorher weg.
            v.extend([(Akademie, 2), (Xenoextraktor, 1), (Lager, 3), (Xenoextraktor, 2)]);
        }
        v.extend([(Erzmine, 8), (Kristallmine, 7), (Deuteriumsynthesizer, 5), (Lager, 3)]);
        if nebel {
            v.extend([(Lager, 4), (Xenoextraktor, 4)]);
        }
        if typ == Bottyp::Igel {
            v.insert(6, (Bunker, 2));
        }
        v.extend([(Konsumgueterwerk, 2), (Bauhof, 2), (Erzmine, 12), (Kristallmine, 11), (Deuteriumsynthesizer, 8), (Lager, 5)]);
        if stufe >= 5 {
            // Das Versorgungsnetz bringt jedem Planeten ein Viertel mehr Strom.
            v.push((Versorgungsnetz, 1));
        }
        return v;
    }
    let mut v = vec![(Erzmine, 5), (Kristallmine, 5), (Bauhof, 1), (Labor, 1), (Deuteriumsynthesizer, 2), (Erzmine, 8), (Kristallmine, 7), (Bauhof, 2), (Deuteriumsynthesizer, 4)];
    if stufe >= 2 {
        v.extend([(Akademie, 1), (Giesserei, 1), (Konsumgueterwerk, 2), (Elektronikwerk, 1), (Labor, 2), (Giesserei, 3), (Elektronikwerk, 2), (Akademie, 2), (Labor, 3), (Bauhof, 3), (Deuteriumsynthesizer, 6)]);
        if r.gebaeude.contains_key(&Geheimdienst) {v.extend([(Geheimdienst,1)]);}
        if typ == Bottyp::Igel {
            v.push((Bunker, 3));
        }
    }
    if stufe >= 3 {
        v.extend([(Raumhafen, 1), (Werft, 2), (Labor, 4), (Akademie, 3), (Werft, 4), (Giesserei, 5), (Elektronikwerk, 4), (Bauhof, 4), (Lager, 3)]);
        match typ {
            Bottyp::Haendler => v.push((Markt, 2)),
            Bottyp::Igel => v.extend([(Sensorphalanx, 2), (Bunker, 5)]),
            Bottyp::Raeuber => v.push((Raumhafen, 3)),
            Bottyp::Oekonom | Bottyp::Wehrlos => {}
        }
    }
    if stufe >= 4 {
        // Lager 7: die Kosten von Stufe V (200.000 Erz, 5.000 Xenokristall) müssen ins Lager passen.
        v.extend([(Orbitalwerft, 1), (Lager, 7), (Verwaltungszentrum, 2), (Labor, 6), (Akademie, 4), (Giesserei, 7), (Elektronikwerk, 6), (Verwaltungszentrum, 6), (Lager, 6)]);
    }
    if stufe >= 5 {
        // Großprojekte: der Orbitalring bringt 50 Felder, die anderen beiden Strom und Forschung.
        v.extend([(Orbitalring, 1), (Versorgungsnetz, 1), (Forschungsarchiv, 1)]);
    }
    // Was die nächste Stufe an Gebäuden verlangt, wie es im Regelwerk steht: so folgt der Bot auch geänderten Regeln.
    // Das Labor für ihre Forschung zuerst, bevor die Felder voll sind.
    if let Some(naechste) = r.stufen.get(stufe as usize - 1) {
        v.extend(naechste.gebaeude.iter().map(|(g, n)| (*g, *n)));
        let labor = naechste.forschung.keys().map(|f| r.forsch(*f).labor).max().unwrap_or(0);
        if labor > 0 {
            v.insert(0, (Labor, labor));
        }
    }
    v
}

/// Fehlen noch Habitatmodule für die Kolonien, die Stufe V verlangt? Dann haben Orbitalwerft und Konsumgüter
/// auf der Heimatwelt Vorrang vor Wachstum: unter voller Deckung verbraucht die Bevölkerung jede Einheit sofort,
/// und wer erst wächst, bekommt nie wieder 100 Konsumgüter für ein Modul zusammen.
fn habitat_fehlt(welt: &Welt, sid: SpielerId) -> bool {
    let r = &welt.regeln;
    let sp = &welt.spieler[sid as usize];
    if sp.stufe < 4 {
        return false;
    }
    let p = &welt.planeten[sp.heimat as usize];
    let soll = r.stufen.last().map(|s| s.kolonien as i64).unwrap_or(0);
    let unterwegs = welt.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren).count() as i64;
    let fehlend = (soll - welt.kolonien(sid) as i64 - unterwegs - p.einheiten[Einheit::Kolonieschiff.idx()]).max(0);
    let je_schiff = r.einh(Einheit::Kolonieschiff).kosten.get(&Gut::Habitatmodul).copied().unwrap_or(0);
    let bestellt: i64 = p.fertigung[1].iter().filter(|f| f.produkt == Produkt::Bauteil(Gut::Habitatmodul)).map(|f| f.rest).sum();
    ganz(welt.bestand_jetzt(sp.heimat as usize)[Gut::Habitatmodul.idx()]) + bestellt < je_schiff * fehlend
}

/// Wählt den nächsten Bau eines Planeten: erst Versorgung, dann Ziele der Stufe, dann die Mine mit der kürzesten Amortisation.
pub fn waehle_bau(welt: &Welt, sid: SpielerId, pid: usize, typ: Bottyp) -> Option<Gebaeude> {
    use Gebaeude::*;
    let r = &welt.regeln;
    let p = &welt.planeten[pid];
    let sp = &welt.spieler[sid as usize];
    let in_q = |g: Gebaeude| p.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;
    let lvl = |g: Gebaeude| p.gebaeude[g.idx()] + in_q(g);
    let frei = |g: Gebaeude| r.geb(g).ab_stufe <= sp.stufe;
    let bestand = welt.bestand_jetzt(pid);
    let kann = |g: Gebaeude| bezahlbar(&bestand, &r.kosten_gebaeude(g, lvl(g) + 1));
    let ohne_nahrung = r.volk(sp.volk).ohne_nahrung;
    // Ab Stufe IV bleibt auf der Heimatwelt ein Feld frei: der Orbitalring der Stufe V belegt selbst eines
    // und bringt 50 neue.
    let ein_feld = welt.felder_belegt(pid) + 1 >= welt.felder_gesamt(pid);
    if p.heimat && ein_feld && frei(Orbitalring) && lvl(Orbitalring) == 0 && welt.felder_belegt(pid) < welt.felder_gesamt(pid) {
        return kann(Orbitalring).then_some(Orbitalring);
    }
    let reserve = if p.heimat && sp.stufe >= 4 && lvl(Orbitalring) == 0 { 1 } else { 0 };
    if welt.felder_belegt(pid) + reserve >= welt.felder_gesamt(pid) {
        return None;
    }
    // Kostet ein Bau mehr, als das Lager fasst, wird er nie bezahlbar: dann zuerst das Lager.
    let grenze = welt.lagergrenze(pid);
    let zu_gross = |g: Gebaeude| {
        let k = r.kosten_gebaeude(g, lvl(g) + 1);
        (0..GUETER).any(|i| k[i] > mal(grenze[i], 0.85))
    };
    let oder_lager = |g: Gebaeude| -> Option<Gebaeude> {
        if kann(g) {
            Some(g)
        } else if zu_gross(g) && in_q(Lager) == 0 && kann(Lager) {
            Some(Lager)
        } else {
            None
        }
    };

    // Strom mit Puffer für den nächsten Verbraucher: das Kraftwerk, dessen nächste Stufe je zusätzlicher Einheit
    // am wenigsten kostet. Fusion nur, wenn die Deuteriumförderung ihren Mehrverbrauch trägt.
    if p.energie_erzeugung < p.energie_verbrauch + p.energie_verbrauch / 10 + 40 * M && in_q(Solarkraftwerk) + in_q(Fusionskraftwerk) == 0 {
        let mehr = |g: Gebaeude, grund: f64| r.stufenwert(grund, lvl(g) + 1) - r.stufenwert(grund, lvl(g));
        let solar = anteil(mehr(Solarkraftwerk, r.geb(Solarkraftwerk).ertrag), p.faktor[F_SOLAR], 1000);
        let fusion = mehr(Fusionskraftwerk, r.geb(Fusionskraftwerk).ertrag);
        let deuterium = mehr(Fusionskraftwerk, r.wirtschaft.fusion_deuterium);
        let je_einheit = |g: Gebaeude, e: i64| r.wert(&r.kosten_gebaeude(g, lvl(g) + 1)) / e.max(1);
        let fusion_ok = frei(Fusionskraftwerk) && p.rate[Gut::Deuterium.idx()] > deuterium + 20 * M;
        if fusion_ok && je_einheit(Fusionskraftwerk, fusion) < je_einheit(Solarkraftwerk, solar) {
            return oder_lager(Fusionskraftwerk);
        }
        return oder_lager(Solarkraftwerk);
    }
    if !ohne_nahrung {
        let bedarf = mal(p.bevoelkerung, r.wirtschaft.nahrung_je_1000) / 1000;
        if p.rate[Gut::Nahrung.idx()] < bedarf / 4 && in_q(Farm) == 0 {
            return kann(Farm).then_some(Farm);
        }
    }
    // Fehlen Habitatmodule (siehe habitat_fehlt): erst die Orbitalwerft, dann Konsumgüter, kein neuer Wohnraum.
    let module = p.heimat && habitat_fehlt(welt, sid);
    let ow_moeglich = frei(Orbitalwerft) && r.geb(Orbitalwerft).braucht.iter().all(|(b, st)| p.gebaeude[b.idx()] >= *st);
    if module && ow_moeglich && lvl(Orbitalwerft) == 0 {
        return oder_lager(Orbitalwerft);
    }
    // Konsumgüter decken: hebt die Stabilität, und nur ein Überschuss reicht für Habitatmodule. Heimatwelt bis
    // Stufe 20, Kolonien bis Stufe 8, nur wenn bezahlbar: das Werk wird je Stufe 1,6-mal teurer, Sparen darauf
    // hielte den übrigen Ausbau an.
    let kw_grenze = if p.heimat { 20 } else { 8 };
    let kw = frei(Konsumgueterwerk) && p.konsum_deckung < 1000 && in_q(Konsumgueterwerk) == 0 && lvl(Konsumgueterwerk) < kw_grenze && kann(Konsumgueterwerk);
    if kw && module {
        return Some(Konsumgueterwerk);
    }
    if p.bevoelkerung > mal(p.wohnraum, 0.55) && in_q(Wohnblock) == 0 && !module {
        return oder_lager(Wohnblock);
    }
    if kw {
        return Some(Konsumgueterwerk);
    }
    let eng = [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Legierung, Gut::Elektronik, Gut::Xenokristall]
        .iter()
        .any(|g| bestand[g.idx()] >= mal(grenze[g.idx()], 0.85));
    if eng && in_q(Lager) == 0 && kann(Lager) {
        return Some(Lager);
    }
    let arbeit_knapp = p.arbeit_bedarf > mal(p.arbeit_verfuegbar, 0.93);

    let nebel = welt.system(p.koord).map(|s| s.nebel).unwrap_or(false);
    let mut ziel_wert = 0i64;
    for (g, min) in ziele(r, sp.stufe, typ, p.heimat, nebel) {
        if !frei(g) || lvl(g) >= min {
            continue;
        }
        if r.geb(g).braucht.iter().any(|(b, st)| p.gebaeude[b.idx()] < *st) {
            continue;
        }
        if arbeit_knapp && r.geb(g).arbeiter >= 15.0 {
            break;
        }
        if let Some(b) = oder_lager(g) {
            return Some(b);
        }
        // Nicht bezahlbar: darauf sparen, nebenher nur Billiges bauen.
        ziel_wert = r.wert(&r.kosten_gebaeude(g, lvl(g) + 1));
        break;
    }
    if arbeit_knapp {
        return None;
    }
    // Mine mit der kürzesten Amortisation.
    let mut beste: Option<(i64, Gebaeude)> = None;
    for (g, gut, f) in [(Erzmine, Gut::Erz, F_ERZ), (Kristallmine, Gut::Kristall, F_KRISTALL), (Deuteriumsynthesizer, Gut::Deuterium, F_DEUTERIUM)] {
        let n = lvl(g);
        if n >= r.welt.max_gebaeudestufe || !kann(g) {
            continue;
        }
        let kosten = r.wert(&r.kosten_gebaeude(g, n + 1));
        if ziel_wert > 0 && kosten > ziel_wert / 4 {
            continue;
        }
        let mehr = anteil(r.stufenwert(r.geb(g).ertrag, n + 1) - r.stufenwert(r.geb(g).ertrag, n), p.faktor[f], 1000);
        let mut k = [0i64; GUETER];
        k[gut.idx()] = mehr.max(1);
        let amortisation = kosten / r.wert(&k).max(1);
        if beste.map(|(a, _)| amortisation < a).unwrap_or(true) {
            beste = Some((amortisation, g));
        }
    }
    beste.map(|(_, g)| g)
}

/// Was ein Planet für sein nächstes Ausbauziel und, auf der Heimatwelt, für den nächsten
/// Aufstieg zurückhält. Schiffe und Verteidigung werden nur aus dem Rest gekauft.
fn ruecklage(welt: &Welt, sid: SpielerId, pid: usize, typ: Bottyp) -> [i64; GUETER] {
    if let Some(g) = reparaturziel(welt, sid, pid) {
        if let Ok(kosten) = welt.reparaturkosten(pid, g) { return kosten; }
    }
    let r = &welt.regeln;
    let p = &welt.planeten[pid];
    let sp = &welt.spieler[sid as usize];
    let mut reserve = [0i64; GUETER];
    let nebel = welt.system(p.koord).map(|s| s.nebel).unwrap_or(false);
    for (g, min) in ziele(r, sp.stufe, typ, p.heimat, nebel) {
        let gr = r.geb(g);
        let stufe = p.gebaeude[g.idx()] + p.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;
        if gr.ab_stufe > sp.stufe || stufe >= min || gr.braucht.iter().any(|(b, st)| p.gebaeude[b.idx()] < *st) {
            continue;
        }
        reserve = r.kosten_gebaeude(g, stufe + 1);
        break;
    }
    if p.heimat && spart(welt, sid) {
        if let Some(regel) = r.stufen.get(sp.stufe as usize - 1) {
            let k = preis_array(&regel.kosten);
            for i in 0..GUETER {
                reserve[i] += k[i];
            }
        }
    }
    reserve
}

/// Bestand über der Rücklage.
fn verfuegbar(welt: &Welt, sid: SpielerId, pid: usize, typ: Bottyp) -> [i64; GUETER] {
    let b = welt.bestand_jetzt(pid);
    let reserve = ruecklage(welt, sid, pid, typ);
    let mut v = [0i64; GUETER];
    for i in 0..GUETER {
        v[i] = (b[i] - reserve[i]).max(0);
    }
    v
}

/// Repair existing productive capacity before buying more ships or new building levels.
/// Survival comes first; skip an unaffordable repair if another damaged building can be restored.
fn reparaturziel(welt: &Welt, sid: SpielerId, pid: usize) -> Option<Gebaeude> {
    if !welt.kolonisation.aktiv { return None; }
    let p = &welt.planeten[pid];
    let mut priorities = Vec::new();
    if !welt.regeln.volk(welt.spieler[sid as usize].volk).ohne_nahrung
        && (p.nahrung_deckung < 1000 || p.rate[Gut::Nahrung.idx()] < 0) {
        priorities.push(Gebaeude::Farm);
    }
    priorities.extend([Gebaeude::Solarkraftwerk, Gebaeude::Farm, Gebaeude::Erzmine,
        Gebaeude::Kristallmine, Gebaeude::Deuteriumsynthesizer, Gebaeude::Lager,
        Gebaeude::Konsumgueterwerk, Gebaeude::Wohnblock, Gebaeude::Raumhafen]);
    priorities.extend(Gebaeude::ALLE);
    let damaged = |g: &Gebaeude| p.gebaeude[g.idx()] > 0 && welt.integritaet(pid, *g) < 1000;
    let bestand = welt.bestand_jetzt(pid);
    priorities.iter().copied().filter(damaged)
        .find(|g| welt.reparaturkosten(pid, *g).is_ok_and(|k| bezahlbar(&bestand, &k)))
        .or_else(|| priorities.into_iter().find(damaged))
}

fn bauen(welt: &mut Welt, sid: SpielerId, pid: usize, typ: Bottyp) {
    if welt.kolonisation.reparaturen.keys().any(|(planet, _)| *planet as usize == pid) {
        return;
    }
    if let Some(g) = reparaturziel(welt, sid, pid) {
        // Construction and repairs share one site. Let existing orders finish first.
        if !welt.planeten[pid].bauschleife.is_empty() { return; }
        let kosten = welt.reparaturkosten(pid, g).unwrap();
        if bezahlbar(&welt.bestand_jetzt(pid), &kosten) {
            tu(welt, sid, json!({"typ":"reparieren","planet":welt.planeten[pid].koord.to_string(),"gebaeude":g.name()}));
            return;
        }
    }
    let p = &welt.planeten[pid];
    if p.gebaeude[Gebaeude::Xenoextraktor.idx()] > 0 && p.prioritaeten.first() != Some(&Gebaeude::Xenoextraktor) {
        let k = p.koord.to_string();
        tu(welt, sid, json!({"typ": "prioritaeten", "planet": k, "reihenfolge": ["xenoextraktor", "farm", "solarkraftwerk", "fusionskraftwerk"]}));
    }
    for _ in 0..2 {
        if welt.planeten[pid].bauschleife.len() >= 2 {
            return;
        }
        let Some(g) = waehle_bau(welt, sid, pid, typ) else {
            return;
        };
        let k = welt.planeten[pid].koord;
        if !tu(welt, sid, json!({"typ": "bauen", "planet": k.to_string(), "gebaeude": g.name()})) {
            return;
        }
    }
}

/// Für die Spur: was der Bot auf einem Planeten als Nächstes baut und was ihm für das nächste Ziel fehlt.
pub fn bau_diagnose(welt: &Welt, sid: SpielerId, pid: usize, typ: Bottyp) -> String {
    let r = &welt.regeln;
    let p = &welt.planeten[pid];
    let sp = &welt.spieler[sid as usize];
    let nebel = welt.system(p.koord).map(|s| s.nebel).unwrap_or(false);
    let bestand = welt.bestand_jetzt(pid);
    let mut ziel = "keins".to_string();
    for (g, min) in ziele(r, sp.stufe, typ, p.heimat, nebel) {
        let stufe = p.gebaeude[g.idx()] + p.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;
        if r.geb(g).ab_stufe > sp.stufe || stufe >= min {
            continue;
        }
        if let Some((b, st)) = r.geb(g).braucht.iter().find(|(b, st)| p.gebaeude[b.idx()] < **st) {
            ziel = format!("{} {} braucht {} {}", g.name(), stufe + 1, b.name(), st);
            break;
        }
        let k = r.kosten_gebaeude(g, stufe + 1);
        let fehlt: Vec<&str> = Gut::ALLE.iter().filter(|x| bestand[x.idx()] < k[x.idx()]).map(|x| x.name()).collect();
        ziel = format!("{} {} fehlt [{}]", g.name(), stufe + 1, fehlt.join(","));
        break;
    }
    let fehlt = |g: Gebaeude| -> String {
        let n = p.gebaeude[g.idx()] + p.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;
        let k = r.kosten_gebaeude(g, n + 1);
        let f: Vec<&str> = Gut::ALLE.iter().filter(|x| bestand[x.idx()] < k[x.idx()]).map(|x| x.name()).collect();
        format!("{}{}[{}]", g.name(), n + 1, f.join(","))
    };
    let strom_knapp = p.energie_erzeugung < p.energie_verbrauch + p.energie_verbrauch / 10 + 40 * M;
    let gt = Einheit::GrosserTransporter;
    let flotten: Vec<&Flotte> = welt.flotten.values().filter(|f| f.besitzer == sid).collect();
    let gt_flug: i64 = flotten.iter().map(|f| f.schiffe[gt.idx()]).sum();
    format!(
        "Bau {} Ziel {} (GT hier {} im Flug {} Flotten {}/{} ErzGrenze {} Erz {} Kri {} Deu {} Felder {}/{} Strom {} Arbeit {}/{} {} {} {} {} {})",
        waehle_bau(welt, sid, pid, typ).map(|g| g.name()).unwrap_or("-"),
        ziel,
        p.einheiten[gt.idx()],
        gt_flug,
        flotten.len(),
        welt.flottenplaetze(sid),
        ganz(welt.lagergrenze(pid)[Gut::Erz.idx()]),
        ganz(bestand[Gut::Erz.idx()]),
        ganz(bestand[Gut::Kristall.idx()]),
        ganz(bestand[Gut::Deuterium.idx()]),
        welt.felder_belegt(pid),
        welt.felder_gesamt(pid),
        if strom_knapp { "knapp" } else { "ok" },
        ganz(p.arbeit_bedarf),
        ganz(p.arbeit_verfuegbar),
        fehlt(Gebaeude::Solarkraftwerk),
        fehlt(Gebaeude::Erzmine),
        fehlt(Gebaeude::Kristallmine),
        fehlt(Gebaeude::Deuteriumsynthesizer),
        fehlt(Gebaeude::Wohnblock),
    )
}

/// Kann der Spieler die fehlenden Güter überhaupt beschaffen? Nein, wenn ein Gut fehlt,
/// das kein eigener Planet herstellt und das auch auf allen Planeten zusammen nicht reicht.
fn beschaffbar(welt: &Welt, sid: SpielerId, bestand: &[i64; GUETER], kosten: &[i64; GUETER]) -> bool {
    let sp = &welt.spieler[sid as usize];
    (0..GUETER).all(|g| {
        kosten[g] <= bestand[g]
            || sp.planeten.iter().any(|p| welt.planeten[*p as usize].rate[g] > 0)
            || sp.planeten.iter().map(|p| welt.bestand_jetzt(*p as usize)[g]).sum::<i64>() >= kosten[g]
    })
}

/// Sind alle Bedingungen der nächsten Stufe erfüllt, spart der Bot für ihre Kosten, höchstens 20 Tage
/// lang und nur, wenn sich jedes Gut der Kosten beschaffen lässt. Sonst stünde die Wirtschaft still.
fn spart(welt: &Welt, sid: SpielerId) -> bool {
    let sp = &welt.spieler[sid as usize];
    let b = welt.stufen_bedingungen(sid);
    if b.is_empty() || !b.iter().all(|(_, ok)| *ok) || sp.stufen_zaehler > 20 * TAG {
        return false;
    }
    let Some(regel) = welt.regeln.stufen.get(sp.stufe as usize - 1) else {
        return false;
    };
    beschaffbar(welt, sid, &welt.bestand_jetzt(sp.heimat as usize), &preis_array(&regel.kosten))
}

fn forschen(welt: &mut Welt, sid: SpielerId, typ: Bottyp) {
    use Forschung::*;
    let sp = &welt.spieler[sid as usize];
    if sp.forschung_aktiv.is_some() {
        return;
    }
    let r = welt.regeln.clone();
    // Zuerst, was die nächste Stufe verlangt, wie es im Regelwerk steht: ihre Forschung und für ihre Kolonien
    // Astrophysik (je zwei Stufen eine Kolonie mehr). So folgt der Bot auch geänderten Regeln.
    let mut plan: Vec<(Forschung, u8)> = Vec::new();
    if let Some(naechste) = r.stufen.get(sp.stufe as usize - 1) {
        plan.extend(naechste.forschung.iter().map(|(f, n)| (*f, *n)));
        if naechste.kolonien > 0 {
            plan.push((Astrophysik, 2 * naechste.kolonien - 1));
        }
    }
    plan.extend([(Energietechnik, 1), (Agrarwissenschaft, 1), (Energietechnik, 3), (Agrarwissenschaft, 2)]);
    if sp.stufe >= 2 {
        plan.extend([(Automatisierung, 1), (Soziologie, 1), (Werkstoffkunde, 1), (Computertechnik, 1), (Automatisierung, 2), (Soziologie, 2), (Verbrennungsantrieb, 2)]);
        match typ {
            Bottyp::Raeuber => plan.extend([(Spionagetechnik, 2), (Waffentechnik, 2), (Panzerung, 2), (Computertechnik, 2)]),
            Bottyp::Igel => plan.extend([(Schildtechnik, 2), (Panzerung, 2), (Waffentechnik, 2), (Spionagetechnik, 2)]),
            _ => plan.extend([(Werkstoffkunde, 2), (Automatisierung, 3)]),
        }
    }
    if sp.stufe >= 3 {
        plan.extend([(Impulsantrieb, 3), (Astrophysik, 1), (Logistik, 1), (Computertechnik, 2), (Energietechnik, 5), (Soziologie, 4), (Automatisierung, 4)]);
        if r.forschung.contains_key(&Ueberwachungstechnik) {plan.extend([(Spionagetechnik,2),(Ueberwachungstechnik,2),(Abschirmtechnik,2)]);}
        if sp.stufe >= 4 {
            // Drei Kolonien und der Hyperraumantrieb sind Bedingungen der Stufe V: vor den Vorlieben des Typs.
            plan.extend([(Astrophysik, 3), (Astrophysik, 5), (Hyperraumantrieb, 1)]);
        }
        match typ {
            Bottyp::Raeuber => plan.extend([(Waffentechnik, 4), (Computertechnik, 4), (Verbrennungsantrieb, 4), (Schildtechnik, 3), (Logistik, 3)]),
            Bottyp::Igel => plan.extend([(Schildtechnik, 4), (Panzerung, 4), (Waffentechnik, 4)]),
            _ => plan.extend([(Werkstoffkunde, 4), (Agrarwissenschaft, 4)]),
        }
    }
    if sp.stufe >= 4 {
        plan.extend([(Xenomaterialkunde, 1), (Soziologie, 6), (Energietechnik, 7), (Automatisierung, 6)]);
    }
    let heimat = sp.heimat as usize;
    let (stufe, stufen) = (sp.stufe, sp.forschung);
    let labor = welt.planeten[heimat].gebaeude[Gebaeude::Labor.idx()];
    let bestand = welt.bestand_jetzt(heimat);
    let moeglich = |f: Forschung| {
        let Some(fr)=r.forschung.get(&f) else {return false;};
        stufen[f.idx()] < MAX_FORSCHUNG && fr.ab_stufe <= stufe && labor >= fr.labor
            && (!matches!(f,Ueberwachungstechnik|Abschirmtechnik)||stufen[Spionagetechnik.idx()]>=1)
            && (f!=Ueberwachungstechnik || sp.planeten.iter().any(|pid|welt.planeten[*pid as usize].gebaeude[Gebaeude::Geheimdienst.idx()]>0))
    };
    for (f, min) in plan {
        if stufen[f.idx()] >= min || !moeglich(f) {
            continue;
        }
        let kosten = r.kosten_forschung(f, stufen[f.idx()] + 1);
        if bezahlbar(&bestand, &kosten) {
            tu(welt, sid, json!({"typ": "forschen", "forschung": f.name()}));
            return;
        }
        // Darauf sparen, außer es fehlt ein Gut, das der Spieler weder hat noch herstellt.
        if beschaffbar(welt, sid, &bestand, &kosten) {
            return;
        }
    }
    if spart(welt, sid) {
        return;
    }
    // Plan abgearbeitet: Terraforming, wenn die Heimatwelt voll ist, sonst die billigste verfügbare Stufe.
    // So liegt kein Lager brach, und Forschung kann niemand erbeuten.
    // Ein Feld bleibt ab Stufe IV für den Orbitalring frei (siehe waehle_bau): dann gilt die Heimatwelt schon als voll.
    let voll = welt.felder_belegt(heimat) + 1 >= welt.felder_gesamt(heimat);
    if voll && moeglich(Terraforming) && bezahlbar(&bestand, &r.kosten_forschung(Terraforming, stufen[Terraforming.idx()] + 1)) {
        tu(welt, sid, json!({"typ": "forschen", "forschung": Terraforming.name()}));
        return;
    }
    let mut beste: Option<(i64, Forschung)> = None;
    for f in Forschung::ALLE {
        if !moeglich(f) {
            continue;
        }
        let kosten = r.kosten_forschung(f, stufen[f.idx()] + 1);
        // Vor Stufe V bleibt der Xenokristall für den Aufstieg liegen.
        if !bezahlbar(&bestand, &kosten) || (stufe < 5 && kosten[Gut::Xenokristall.idx()] > 0) {
            continue;
        }
        let wert = r.wert(&kosten);
        if beste.map(|(w, _)| wert < w).unwrap_or(true) {
            beste = Some((wert, f));
        }
    }
    if let Some((_, f)) = beste {
        tu(welt, sid, json!({"typ": "forschen", "forschung": f.name()}));
    }
}

fn allgemein(welt: &mut Welt, sid: SpielerId) {
    let r = welt.regeln.clone();
    let sp = &welt.spieler[sid as usize];
    // Höchster Steuersatz ohne Stabilitätsverlust.
    if sp.steuersatz != r.stabilitaet.steuer_frei_bis {
        tu(welt, sid, json!({"typ": "steuersatz", "prozent": r.stabilitaet.steuer_frei_bis}));
    }
    let sp = &welt.spieler[sid as usize];
    if let Some(regel) = r.stufen.get(sp.stufe as usize - 1) {
        if sp.stufen_zaehler >= r.stufen_haltezeit_stunden * STUNDE
            && bezahlbar(&welt.bestand_jetzt(sp.heimat as usize), &preis_array(&regel.kosten))
        {
            tu(welt, sid, json!({"typ": "stufenaufstieg"}));
        }
    }
}

/// Wurde der Spieler in den letzten Tagen geplündert? Steht in seinen Vorfällen (Chronik der Engine).
fn gepluendert(welt: &Welt, sid: SpielerId) -> bool {
    welt.spieler[sid as usize].vorfaelle.iter().any(|v| v.art == "beute" && v.text.contains("wurde geplündert"))
}

/// Verteidigung auf jedem eigenen Planeten bis zu einem Anteil des dortigen Gebäudewerts, dazu ein Bunker.
/// Der Igel verteidigt immer bis zur Hälfte. Ökonom, Händler und Räuber schützen sich wie ein vernünftiger
/// Spieler erst, wenn sie geplündert wurden, dann bis 15 Prozent (Codex' Spanne für wirksamen Schutz: 15 bis 25).
/// Ohne das wird der Räuber mit seiner gehorteten Beute selbst das lohnendste Ziel anderer Räuber.
fn verteidigen(welt: &mut Welt, sid: SpielerId, typ: Bottyp) {
    let r = welt.regeln.clone();
    let sp = &welt.spieler[sid as usize];
    if sp.stufe < 2 {
        return;
    }
    let anteil_pm: i64 = match typ {
        Bottyp::Igel => 500,
        Bottyp::Oekonom | Bottyp::Haendler | Bottyp::Raeuber if gepluendert(welt, sid) => 150,
        _ => return,
    };
    let (volk, stufe) = (sp.volk, sp.stufe);
    let wahl: &[Einheit] = match stufe {
        2 => &[Einheit::Lasergeschuetz, Einheit::Raketenwerfer],
        3 => &[Einheit::Ionengeschuetz, Einheit::Lasergeschuetz, Einheit::Raketenwerfer],
        _ => &[Einheit::Gausskanone, Einheit::Ionengeschuetz, Einheit::Lasergeschuetz],
    };
    for pid in sp.planeten.clone() {
        let pid = pid as usize;
        let p = &welt.planeten[pid];
        let k = p.koord.to_string();
        // Bunker: Heimatwelt Stufe 4, Kolonien Stufe 2.
        let bunker_soll = if p.heimat { 4 } else { 2 };
        let bunker = p.gebaeude[Gebaeude::Bunker.idx()] + p.bauschleife.iter().filter(|a| a.gebaeude == Gebaeude::Bunker).count() as u8;
        if typ != Bottyp::Igel && bunker < bunker_soll && p.bauschleife.len() < 2 && welt.felder_belegt(pid) < welt.felder_gesamt(pid)
            && bezahlbar(&welt.bestand_jetzt(pid), &r.kosten_gebaeude(Gebaeude::Bunker, bunker + 1))
        {
            tu(welt, sid, json!({"typ": "bauen", "planet": k, "gebaeude": "bunker"}));
        }
        let p = &welt.planeten[pid];
        if !p.fertigung[0].is_empty() {
            continue;
        }
        let gebaeude: i64 = Gebaeude::ALLE.iter().map(|g| r.wert_gebaeude_bis(*g, p.gebaeude[g.idx()])).sum();
        let verteidigung: i64 = (SCHIFFE..EINHEITEN).map(|e| p.einheiten[e] * r.wert(&r.kosten_einheit(Einheit::ALLE[e], volk))).sum();
        if verteidigung * 1000 >= gebaeude * anteil_pm {
            continue;
        }
        let bestand = verfuegbar(welt, sid, pid, typ);
        for e in wahl {
            let kosten = r.kosten_einheit(*e, volk);
            // Höchstens ein Drittel des freien Bestands je Zug, damit der Ausbau weiterläuft.
            let n = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| bestand[g] / 3 / kosten[g]).min().unwrap_or(0).min(50);
            if n >= 1 {
                tu(welt, sid, json!({"typ": "fertigen", "planet": k, "einheit": e.name(), "anzahl": n}));
                break;
            }
        }
    }
}

fn geschuetzt(welt: &Welt, ziel: SpielerId) -> bool {
    welt.zeit < welt.spieler[ziel as usize].schutz_bis
}

/// Schätzt einen Angriff gegen den letzten Spionagebericht: Siegchance in Prozent, eigener Verlust und
/// Beute (ohne Ladegrenze), beide in Werteinheiten. Gegen ein wehrloses Ziel ohne Simulation, sonst mit
/// wenigen Läufen; das reicht für eine Botentscheidung und kostet einen Bruchteil des Werkzeugs.
fn angriff_schaetzen(welt: &Welt, sid: SpielerId, b: &Spionagebericht, ang: &[i64; EINHEITEN]) -> Option<(i64, i64, [i64; GUETER])> {
    let r = &welt.regeln;
    let sp = &welt.spieler[sid as usize];
    let (bs, bv) = (b.schiffe.as_ref()?, b.verteidigung.as_ref()?);
    let quote = r.volk(sp.volk).pluenderquote.unwrap_or(r.kampf.pluenderquote);
    let schutz = b.gebaeude.as_ref().map(|g| r.bunkerschutz(g[Gebaeude::Bunker.idx()]));
    let mut beute = [0i64; GUETER];
    for g in 0..8 {
        beute[g] = mal((b.bestand[g] - schutz.map(|s| s[g]).unwrap_or(0)).max(0), quote);
    }
    let mut vert = [0i64; EINHEITEN];
    vert[..SCHIFFE].copy_from_slice(bs);
    vert[SCHIFFE..].copy_from_slice(bv);
    if vert.iter().all(|n| *n == 0) {
        return Some((100, 0, beute));
    }
    let tech = |f: Forschung| 1.0 + r.kampf.tech_je_stufe * b.forschung.as_ref().map(|t| t[f.idx()]).unwrap_or(sp.forschung[f.idx()]) as f64;
    let a = Gruppe::neu(r, sp, *ang);
    let defender = r.volk(welt.spieler[b.besitzer as usize].volk);
    let v = Gruppe::mit_werten(r, b.besitzer, vert, tech(Forschung::Waffentechnik) * defender.waffen,
        tech(Forschung::Schildtechnik), tech(Forschung::Panzerung) * defender.panzerung);
    const LAEUFE: u64 = 8;
    let (mut siege, mut verlust) = (0i64, 0i64);
    for i in 0..LAEUFE {
        let nr = (welt.zeit as u64) << 24 | (sid as u64) << 8 | 0x80 | i;
        let mut rng = strom(welt.startwert, KANAL_SIMULATOR, nr);
        let erg = kampf(r, std::slice::from_ref(&a), std::slice::from_ref(&v), &mut rng);
        if erg.sieger == Sieger::Angreifer {
            siege += 1;
        }
        for e in 0..EINHEITEN {
            let weg = ang[e] - erg.angreifer[0][e];
            if weg > 0 {
                verlust += weg * r.wert(&r.kosten_einheit(Einheit::ALLE[e], sp.volk));
            }
        }
    }
    Some((siege * 100 / LAEUFE as i64, verlust / LAEUFE as i64, beute))
}

/// Der Räuber späht Nachbarn aus und greift an, wenn der Simulator Beute über den Verlusten verspricht.
fn raeuber(welt: &mut Welt, sid: SpielerId, bot: &mut Bot) {
    let r = welt.regeln.clone();
    let sp = &welt.spieler[sid as usize];
    if sp.stufe < 3 {
        return;
    }
    let pid = sp.heimat as usize;
    let p = &welt.planeten[pid];
    if p.gebaeude[Gebaeude::Werft.idx()] < 1 || p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 {
        return;
    }
    let k = p.koord;
    let ks = k.to_string();
    let volk = sp.volk;
    let n = |welt: &Welt, e: Einheit| welt.planeten[pid].einheiten[e.idx()];

    // Flotte aufbauen: Sonden, Jäger, Transporter, ab Stufe IV Kreuzer.
    if welt.planeten[pid].fertigung[0].is_empty() {
        let bestand = verfuegbar(welt, sid, pid, Bottyp::Raeuber);
        let stueck = |e: Einheit, teil: i64, max: i64| {
            let kosten = r.kosten_einheit(e, volk);
            (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| bestand[g] / teil / kosten[g]).min().unwrap_or(0).min(max)
        };
        let unterwegs: i64 = welt.flotten.values().filter(|f| f.besitzer == sid).map(|f| f.schiffe[Einheit::LeichterJaeger.idx()]).sum();
        let jaeger = n(welt, Einheit::LeichterJaeger) + unterwegs;
        let bestellung = if n(welt, Einheit::Spionagesonde) < 4 {
            Some((Einheit::Spionagesonde, stueck(Einheit::Spionagesonde, 1, 4)))
        } else if n(welt, Einheit::KleinerTransporter) * 4 < jaeger + 8 {
            Some((Einheit::KleinerTransporter, stueck(Einheit::KleinerTransporter, 2, 10)))
        } else if welt.spieler[sid as usize].stufe >= 4 && p.gebaeude[Gebaeude::Werft.idx()] >= 4 && n(welt, Einheit::Kreuzer) * 6 < jaeger {
            Some((Einheit::Kreuzer, stueck(Einheit::Kreuzer, 2, 15)))
        } else {
            Some((Einheit::LeichterJaeger, stueck(Einheit::LeichterJaeger, 3, 50)))
        };
        // Unterhalt der Flotte nach dem Auftrag muss für zehn Tage aus den Credits bezahlbar sein.
        let tragbar = |e: Einheit, anzahl: i64| {
            let neu = anzahl * r.wert(&r.kosten_einheit(e, volk));
            let je_tag = mal(welt.flottenwert(sid) + neu, r.wirtschaft.unterhalt_schiffe_je_tag);
            welt.spieler[sid as usize].credits >= 10 * je_tag
        };
        let bestellung = bestellung.filter(|(e, anzahl)| *e == Einheit::Spionagesonde || tragbar(*e, *anzahl));
        if let Some((e, anzahl)) = bestellung {
            if anzahl >= 1 {
                tu(welt, sid, json!({"typ": "fertigen", "planet": ks, "einheit": e.name(), "anzahl": anzahl}));
            }
        }
    }

    let frei = |welt: &Welt| welt.flottenplaetze(sid) as i64 - welt.flotten.values().filter(|f| f.besitzer == sid).count() as i64;
    let jetzt = welt.zeit;

    // Angriff auf das beste frisch ausgespähte Ziel.
    let jaeger = n(welt, Einheit::LeichterJaeger);
    let kreuzer = n(welt, Einheit::Kreuzer);
    let transporter = n(welt, Einheit::KleinerTransporter);
    if frei(welt) > 0 && jaeger + kreuzer >= 5 {
        // Nur die zwei Berichte mit dem meisten Lagerwert durchrechnen: der Simulator ist teuer.
        let mut berichte: Vec<(i64, Spionagebericht)> = welt.spieler[sid as usize]
            .berichte
            .iter()
            .filter(|b| jetzt - b.zeit < 8 * STUNDE && b.besitzer != sid && (welt.aufklaerungsregeln() || !geschuetzt(welt, b.besitzer)))
            .filter(|b| !welt.vertrag_zwischen(sid, b.besitzer, Vertragsart::Nichtangriffspakt))
            .map(|b| {
                let mut lager = [0i64; GUETER];
                lager.copy_from_slice(&b.bestand[..GUETER]);
                (-r.wert(&lager), b.clone())
            })
            .collect();
        berichte.sort_by_key(|(w, _)| *w);
        let mut bestes: Option<(i64, Koord, i64)> = None;
        for (lagerwert, b) in berichte.iter().take(2) {
            if -lagerwert < 20_000 * M {
                continue;
            }
            let mut ang = [0i64; EINHEITEN];
            ang[Einheit::LeichterJaeger.idx()] = jaeger;
            ang[Einheit::Kreuzer.idx()] = kreuzer;
            let Some((chance, verlust, beute)) = angriff_schaetzen(welt, sid, b, &ang) else {
                continue;
            };
            let verlust = verlust / M;
            let menge: i64 = beute.iter().sum::<i64>() / M;
            let wert = r.wert(&beute) / M;
            if chance >= 85 && wert > 2 * verlust + 3000 && bestes.map(|(w, _, _)| wert - verlust > w).unwrap_or(true) {
                bestes = Some((wert - verlust, b.ziel, menge));
            }
        }
        if let Some((_, ziel, menge)) = bestes {
            let ladung = r.einh(Einheit::KleinerTransporter).ladung;
            let mit = ((menge + ladung - 1) / ladung).clamp(1, transporter.max(1)).min(transporter);
            let mut schiffe = json!({"leichter_jaeger": jaeger});
            if kreuzer > 0 {
                schiffe["kreuzer"] = json!(kreuzer);
            }
            if mit > 0 {
                schiffe["kleiner_transporter"] = json!(mit);
            }
            // Langsamer fliegen, wenn der Treibstoff für volle Fahrt nicht reicht: der Verbrauch fällt quadratisch.
            let mut flotte = [0i64; SCHIFFE];
            flotte[Einheit::LeichterJaeger.idx()] = jaeger;
            flotte[Einheit::Kreuzer.idx()] = kreuzer;
            flotte[Einheit::KleinerTransporter.idx()] = mit;
            let deut = welt.bestand_jetzt(pid)[Gut::Deuterium.idx()];
            let tempo = [1.0, 0.8, 0.6, 0.4, 0.3].into_iter().find(|s| {
                welt.flugplan(sid, pid, ziel, &flotte, milli(*s)).map(|p| 2 * p.treibstoff <= deut).unwrap_or(false)
            });
            if let Some(tempo) = tempo {
                tu(welt, sid, json!({"typ": "flotte_senden", "start": ks, "ziel": ziel.to_string(), "mission": "angriff",
                    "schiffe": schiffe, "geschwindigkeit": tempo}));
            }
        }
    }

    // Neue Ziele ausspähen: belegte Plätze im eigenen Sektor, die nächsten zuerst.
    let mut sonden = n(welt, Einheit::Spionagesonde);
    let mut kandidaten: Vec<(i64, Koord)> = if welt.aufklaerungsregeln() {
        let unbesetzt: std::collections::BTreeSet<String> = welt.spieler.iter().filter(|s| !welt.spieler_aktiv(s.id)).map(|s| s.name.clone()).collect();
        let mut known = Vec::new();
        for system in 1..=r.welt.systeme_je_sektor {
            if (system as i64-k.system as i64).abs()>15 {continue;}
            for position in 1..=r.welt.plaetze_je_system {
                let coord=Koord::neu(k.sektor,system,position);
                let view=welt.planetenwissen(sid,coord);
                if view["status"]=="eigen" || view["status"]=="frei" || view["status"]=="freund" {continue;}
                // Unclaimed start places are public knowledge and frozen: no target, no probes.
                if view["spieler"].as_str().is_some_and(|n|unbesetzt.contains(n)) {continue;}
                if bot.gespaeht.get(&coord).is_some_and(|t|jetzt-*t<12*STUNDE) {continue;}
                known.push((welt.entfernung(k,coord),coord));
            }
        }
        known
    } else { welt
        .planeten
        .iter()
        .filter(|z| z.besitzer != sid && z.koord.sektor == k.sektor && (z.koord.system as i64 - k.system as i64).abs() <= 15)
        .filter(|z| !geschuetzt(welt, z.besitzer) && bot.gespaeht.get(&z.koord).map(|t| jetzt - *t >= 12 * STUNDE).unwrap_or(true))
        .map(|z| (welt.entfernung(k, z.koord), z.koord))
        .collect() };
    kandidaten.sort();
    for (_, ziel) in kandidaten {
        if frei(welt) <= 0 || sonden < 2 {
            break;
        }
        let mission=if welt.aufklaerungsregeln() && !welt.system_erfasst(sid,ziel) {"system_erkunden"} else {"spionage"};
        if welt.flotten.values().any(|f|f.besitzer==sid && f.ziel.sektor==ziel.sektor && f.ziel.system==ziel.system && f.mission==Mission::SystemErkunden) {continue;}
        if tu(welt, sid, json!({"typ": "flotte_senden", "start": ks, "ziel": ziel.to_string(), "mission": mission, "schiffe": {"spionagesonde": 2}})) {
            bot.gespaeht.insert(ziel, jetzt);
            sonden -= 2;
        }
    }
}

/// Der Händler verkauft, was sein Lager füllt, und kauft, was ihm fehlt.
fn haendler(welt: &mut Welt, sid: SpielerId) {
    let r = welt.regeln.clone();
    let sp = &welt.spieler[sid as usize];
    let pid = sp.heimat as usize;
    let p = &welt.planeten[pid];
    if p.gebaeude[Gebaeude::Markt.idx()] == 0 {
        return;
    }
    let k = p.koord.to_string();
    let bestand = welt.bestand_jetzt(pid);
    let grenze = welt.lagergrenze(pid);
    let credits = sp.credits;
    let eigene = welt.orders.iter().filter(|o| o.spieler == sid).count();
    if eigene >= 4 {
        return;
    }
    let handelbar = [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Legierung, Gut::Elektronik];
    for g in handelbar {
        let i = g.idx();
        let gewicht = r.wertung.gewichte[&g];
        let hat_order = welt.orders.iter().any(|o| o.spieler == sid && o.gut == g);
        if hat_order {
            continue;
        }
        if bestand[i] >= mal(grenze[i], 0.7) {
            let menge = ganz(bestand[i] / 5);
            if menge >= 100 {
                tu(welt, sid, json!({"typ": "markt_order", "planet": k, "gut": g.name(), "seite": "verkauf", "menge": menge, "preis": gewicht * 0.05}));
                return;
            }
        } else if bestand[i] < mal(grenze[i], 0.1) && credits > 1500 * M {
            let preis = gewicht * 0.06;
            let menge = (ganz(credits) as f64 / 4.0 / preis).floor().min(5000.0);
            if menge >= 100.0 {
                tu(welt, sid, json!({"typ": "markt_order", "planet": k, "gut": g.name(), "seite": "kauf", "menge": menge, "preis": preis}));
                return;
            }
        }
    }
}

/// Bietet Nachbarn Nichtangriffspakte an und nimmt Angebote an. Räuber schließen keine Pakte.
fn diplomatie(welt: &mut Welt, sid: SpielerId, typ: Bottyp) {
    kommunizieren(welt,sid,typ);

    let offen: Vec<u32> = welt
        .vertraege
        .iter()
        .filter(|v| v.b == sid && v.status == Vertragsstatus::Angeboten)
        .map(|v| v.id)
        .collect();
    for id in offen {
        let art = if typ == Bottyp::Raeuber { "vertrag_ablehnen" } else { "vertrag_annehmen" };
        tu(welt, sid, json!({"typ": art, "vertrag": id}));
    }
    if typ != Bottyp::Haendler || welt.zeit % (2 * TAG) >= TAKT {
        return;
    }
    let k = welt.planeten[welt.spieler[sid as usize].heimat as usize].koord;
    let mut nachbarn: Vec<(i64, SpielerId)> = if welt.aufklaerungsregeln() {
        // Names and active membership are public; hidden home coordinates are not.
        (0..welt.spieler.len() as u16).filter(|id|*id!=sid&&welt.spieler_aktiv(*id))
            .map(|id|((i64::from(id)-i64::from(sid)).rem_euclid(welt.spieler.len() as i64),id)).collect()
    } else {welt
        .planeten
        .iter()
        .filter(|p| p.heimat && p.besitzer != sid && p.koord.sektor == k.sektor)
        .map(|p| (welt.entfernung(k, p.koord), p.besitzer))
        .collect()};
    nachbarn.sort();
    for (_, b) in nachbarn.into_iter().take(3) {
        if welt.vertraege.iter().any(|v| {
            v.art == Vertragsart::Nichtangriffspakt
                && matches!(v.status, Vertragsstatus::Aktiv | Vertragsstatus::Angeboten)
                && ((v.a == sid && v.b == b) || (v.a == b && v.b == sid))
        }) {
            continue;
        }
        if !welt.spieler_aktiv(b) {continue;}
        let name = welt.spieler[b as usize].name.clone();
        tu(welt, sid, json!({"typ": "vertrag_anbieten", "partner": name, "art": "nichtangriffspakt", "kaution": 100}));
    }
}

/// Große Transporter des Spielers: auf Planeten, unterwegs und bestellt.
fn transporter_gesamt(welt: &Welt, sid: SpielerId) -> i64 {
    let gt = Einheit::GrosserTransporter;
    let sp = &welt.spieler[sid as usize];
    let auf_planeten: i64 = sp.planeten.iter().map(|p| welt.planeten[*p as usize].einheiten[gt.idx()]).sum();
    let unterwegs: i64 = welt.flotten.values().filter(|f| f.besitzer == sid).map(|f| f.schiffe[gt.idx()]).sum();
    let bestellt: i64 = welt.planeten[sp.heimat as usize].fertigung[0]
        .iter()
        .filter(|f| f.produkt == Produkt::Einheit(gt))
        .map(|f| f.rest)
        .sum();
    auf_planeten + unterwegs + bestellt
}

fn ladung_json(menge: &[i64; GUETER]) -> Value {
    let mut m = serde_json::Map::new();
    for g in Gut::ALLE {
        if menge[g.idx()] >= M {
            m.insert(g.name().to_string(), json!(ganz(menge[g.idx()])));
        }
    }
    Value::Object(m)
}

/// Passt eine Ladung großer Transporter in die Kapazität und lässt Treibstoff für Hin- und Rückflug übrig.
fn ladung_fuer(welt: &Welt, sid: SpielerId, start: usize, ziel: Koord, anzahl: i64, mut menge: [i64; GUETER]) -> Option<[i64; GUETER]> {
    let mut schiffe = [0i64; SCHIFFE];
    schiffe[Einheit::GrosserTransporter.idx()] = anzahl;
    let plan = welt.flugplan(sid, start, ziel, &schiffe, 1000).ok()?;
    let bestand = welt.bestand_jetzt(start);
    let d = Gut::Deuterium.idx();
    let treibstoff = plan.treibstoff.checked_mul(2)?;
    if bestand[d] < treibstoff {
        return None;
    }
    menge[d] = menge[d].min((bestand[d] - treibstoff - 200 * M).max(0));
    let summe: i64 = menge.iter().sum();
    if summe > plan.kapazitaet {
        for g in 0..GUETER {
            menge[g] = anteil(menge[g], plan.kapazitaet, summe);
        }
    }
    for g in 0..GUETER {
        menge[g] = menge[g] / M * M;
    }
    Some(menge)
}

/// Logistik zwischen eigenen Planeten: Baustoffe zu Kolonien mit freien Feldern, Xenokristall und
/// Überschuss der Kolonien zur Heimatwelt. Ohne sie wächst nach voller Heimatwelt nichts mehr.
fn logistik(welt: &mut Welt, sid: SpielerId, typ: Bottyp) {
    let r = welt.regeln.clone();
    let gt = Einheit::GrosserTransporter;
    let sp = &welt.spieler[sid as usize];
    let heimat = sp.heimat as usize;
    let kolonien: Vec<usize> = sp.planeten.iter().map(|p| *p as usize).filter(|p| *p != heimat).collect();
    let hp = &welt.planeten[heimat];
    if sp.stufe < 3 || kolonien.is_empty() || hp.gebaeude[Gebaeude::Werft.idx()] < r.einh(gt).werft || hp.gebaeude[Gebaeude::Raumhafen.idx()] < 1 {
        return;
    }
    let hk = hp.koord;
    let volk = sp.volk;
    let frei = |welt: &Welt| welt.flottenplaetze(sid) as i64 - welt.flotten.values().filter(|f| f.besitzer == sid).count() as i64;

    // Zwei Transporter für die Heimat, einer je Kolonie.
    let fehlt = 2 + kolonien.len() as i64 - transporter_gesamt(welt, sid);
    let bestand = welt.bestand_jetzt(heimat);
    let kosten = r.kosten_einheit(gt, volk);
    let bezahlbare = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| bestand[g] / kosten[g]).min().unwrap_or(0);
    let anzahl = fehlt.min(2).min(bezahlbare);
    if anzahl > 0 && welt.planeten[heimat].fertigung[0].is_empty() {
        tu(welt, sid, json!({"typ": "fertigen", "planet": hk.to_string(), "einheit": gt.name(), "anzahl": anzahl}));
    }

    // Kolonie zur Heimat: Xenokristall, was das Lager der Kolonie füllt, und Konsumgüter, wenn die Heimat
    // sie für ein Habitatmodul braucht (ihre eigene Bevölkerung verbraucht dort jede produzierte Einheit).
    let habitat = r.kosten_bauteil(Gut::Habitatmodul).map(|k| k[Gut::Konsumgut.idx()]).unwrap_or(0);
    let heim_braucht_konsum = welt.bestand_jetzt(heimat)[Gut::Konsumgut.idx()] < habitat + 50 * M;
    let heim_knapp = |welt: &Welt, g: Gut| welt.bestand_jetzt(heimat)[g.idx()] < mal(welt.lagergrenze(heimat)[g.idx()], 0.5);
    for &kp in &kolonien {
        let p = &welt.planeten[kp];
        if frei(welt) <= 1 || p.einheiten[gt.idx()] < 1 || p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 || p.blockade.is_some() {
            continue;
        }
        let k = p.koord;
        let b = welt.bestand_jetzt(kp);
        let grenze = welt.lagergrenze(kp);
        let mut menge = [0i64; GUETER];
        menge[Gut::Xenokristall.idx()] = b[Gut::Xenokristall.idx()];
        // Erz und Kristall nur, wenn die Heimat knapp ist und die Kolonie fast voll: sonst fehlen sie der
        // Kolonie für eigene Kraftwerke und Minen, und die Heimat schickt sie gleich wieder zurück.
        for g in [Gut::Erz, Gut::Kristall] {
            if heim_knapp(welt, g) {
                menge[g.idx()] = (b[g.idx()] - mal(grenze[g.idx()], 0.85)).max(0);
            }
        }
        let k_idx = Gut::Konsumgut.idx();
        let konsum = if heim_braucht_konsum { (b[k_idx] - 20 * M).max(0) } else { 0 };
        menge[k_idx] = konsum;
        if menge[Gut::Xenokristall.idx()] < 300 * M && r.wert(&menge) < 15_000 * M && konsum < habitat / 2 {
            continue;
        }
        let Some(menge) = ladung_fuer(welt, sid, kp, hk, 1, menge) else {
            continue;
        };
        tu(welt, sid, json!({"typ": "flotte_senden", "start": k.to_string(), "ziel": hk.to_string(), "mission": "transport",
            "schiffe": {gt.name(): 1}, "ladung": ladung_json(&menge)}));
    }

    // Beim Sparen für den Aufstieg nur noch die Nebelkolonien versorgen, solange Xenokristall für die
    // Aufstiegskosten fehlt: ohne ihren Extraktor gibt es keinen.
    let xeno_soll = r.stufen.get(welt.spieler[sid as usize].stufe as usize - 1).map(|s| preis_array(&s.kosten)[Gut::Xenokristall.idx()]).unwrap_or(0);
    let xeno_hat: i64 = kolonien.iter().chain(std::iter::once(&heimat)).map(|p| welt.bestand_jetzt(*p)[Gut::Xenokristall.idx()]).sum();
    let nur_nebel = spart(welt, sid);
    if nur_nebel && xeno_hat >= xeno_soll {
        return;
    }
    // Heimat zur Kolonie: Baustoffe für die Kolonie mit freien Feldern und dem kleinsten Bestand.
    // Hat sie einen Raumhafen, aber noch keinen Transporter, bleibt einer dort.
    // Beim Sparen ist die Rücklage der ganze Bestand; für den Extraktor geht ein kleiner Teil davon.
    let hb = if nur_nebel { welt.bestand_jetzt(heimat) } else { verfuegbar(welt, sid, heimat, typ) };
    let hrate = welt.planeten[heimat].rate;
    let mut ziele: Vec<(i64, usize)> = kolonien
        .iter()
        .copied()
        .filter(|kp| {
            let k = welt.planeten[*kp].koord;
            welt.felder_belegt(*kp) + 3 <= welt.felder_gesamt(*kp)
                && welt.planeten[*kp].blockade.is_none()
                && (!nur_nebel || welt.system(k).map(|s| s.nebel).unwrap_or(false))
                && !welt.flotten.values().any(|f| f.besitzer == sid && f.ziel == k && f.zustand == Flottenzustand::Hinflug)
        })
        .map(|kp| (r.wert(&welt.bestand_jetzt(kp)), kp))
        .collect();
    ziele.sort();
    let Some(&(_, kp)) = ziele.first() else {
        return;
    };
    let transporter = welt.planeten[heimat].einheiten[gt.idx()];
    if frei(welt) <= 1 || transporter < 1 {
        return;
    }
    let ziel = welt.planeten[kp].koord;
    let b = welt.bestand_jetzt(kp);
    let grenze = welt.lagergrenze(kp);
    let mut menge = [0i64; GUETER];
    for g in [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Legierung, Gut::Elektronik, Gut::Nahrung, Gut::Konsumgut] {
        let i = g.idx();
        if matches!(g, Gut::Nahrung | Gut::Konsumgut) && hrate[i] <= 0 {
            continue;
        }
        let anteil_heim = if nur_nebel { 0.15 } else { 0.5 };
        menge[i] = mal(hb[i], anteil_heim).min((mal(grenze[i], 0.9) - b[i]).max(0));
    }
    let stationieren = welt.planeten[kp].gebaeude[Gebaeude::Raumhafen.idx()] >= 1 && welt.planeten[kp].einheiten[gt.idx()] == 0
        && !welt.flotten.values().any(|f| f.besitzer == sid && f.ziel == ziel && f.mission == Mission::Stationieren);
    let anzahl = if stationieren { 1 } else { transporter.min(2) };
    let Some(menge) = ladung_fuer(welt, sid, heimat, ziel, anzahl, menge) else {
        return;
    };
    if !stationieren && r.wert(&menge) < 5_000 * M {
        return;
    }
    let mission = if stationieren { "stationieren" } else { "transport" };
    tu(welt, sid, json!({"typ": "flotte_senden", "start": hk.to_string(), "ziel": ziel.to_string(), "mission": mission,
        "schiffe": {gt.name(): anzahl}, "ladung": ladung_json(&menge)}));
}

/// Ab Stufe IV: Bauteile fertigen, Kolonieschiff bauen und losschicken. Die erste Kolonie geht in einen Nebel.
/// `gespaeht`: the bot's probe memory. Under the reconnaissance rules a probed target that is
/// not confirmed free is skipped for a week: the report alone does not last (a bot keeps
/// only its latest 30 reports), and without this memory bots probed the same occupied or
/// reserved home every few minutes, flooding its owner with "Spionage bemerkt".
fn kolonisieren(welt: &mut Welt, sid: SpielerId, gespaeht: &mut BTreeMap<Koord, SimZeit>) {
    let r = welt.regeln.clone();
    let sp = &welt.spieler[sid as usize];
    if sp.stufe < 4 {
        return;
    }
    let pid = sp.heimat as usize;
    let p = &welt.planeten[pid];
    let k = p.koord;
    let ks = k.to_string();
    let unterwegs = welt.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren).count();
    if welt.kolonien(sid) + unterwegs >= welt.kolonien_erlaubt(sid) || p.gebaeude[Gebaeude::Orbitalwerft.idx()] < 1 {
        return;
    }
    let bestand = welt.bestand_jetzt(pid);
    if p.einheiten[Einheit::Kolonieschiff.idx()] >= 1 {
        if welt.flotten.values().filter(|f| f.besitzer == sid).count() >= welt.flottenplaetze(sid) {
            return;
        }
        // Ziel: freier Platz der Lebenszone, zuerst im nächsten Nebelsystem, sonst so nah wie möglich.
        let in_nebel = sp.planeten.iter().any(|x| welt.system(welt.planeten[*x as usize].koord).map(|s| s.nebel).unwrap_or(false));
        let lz = &r.zonen[&Zone::Leben];
        let mut bestes: Option<(i64, Koord)> = None;
        for n in 1..=r.welt.systeme_je_sektor {
            let observed= !welt.aufklaerungsregeln() || welt.system_erfasst(sid,Koord::neu(k.sektor,n,1));
            let nebel = observed && welt.system(Koord::neu(k.sektor, n, 1)).map(|s| s.nebel).unwrap_or(false);
            if !in_nebel && !nebel && observed {
                continue;
            }
            let (von,bis)=if welt.aufklaerungsregeln(){(1,r.welt.plaetze_je_system)}else{(lz.von,lz.bis)};
            for pos in von..=bis {
                let z = Koord::neu(k.sektor, n, pos);
                if welt.aufklaerungsregeln() && sp.erkundet.get(&z).is_some_and(|e|e.zone!=Zone::Leben) {continue;}
                if welt.aufklaerungsregeln() && !sp.erkundet.contains_key(&z) && gespaeht.get(&z).is_some_and(|t| welt.zeit - *t < KOLONIE_SPAEHPAUSE) {continue;}
                if if welt.aufklaerungsregeln() {
                    let status=welt.planetenwissen(sid,z)["status"].as_str().unwrap_or("unbekannt").to_string();
                    !matches!(status.as_str(),"frei"|"unbekannt")
                } else {welt.belegung.contains_key(&z)} {
                    continue;
                }
                let d = welt.entfernung(k, z);
                if bestes.map(|(b, _)| d < b).unwrap_or(true) {
                    bestes = Some((d, z));
                }
            }
        }
        if let Some((_, ziel)) = bestes {
            if welt.aufklaerungsregeln() && !welt.system_erfasst(sid,ziel) {
                if !welt.flotten.values().any(|f| f.besitzer==sid && f.mission==Mission::SystemErkunden && f.ziel.sektor==ziel.sektor && f.ziel.system==ziel.system) {
                    if p.einheiten[Einheit::Spionagesonde.idx()]>0 {
                        tu(welt,sid,json!({"typ":"flotte_senden","start":ks,"ziel":ziel.to_string(),"mission":"system_erkunden","schiffe":{"spionagesonde":1}}));
                    } else if !p.fertigung[0].iter().any(|f|f.produkt==Produkt::Einheit(Einheit::Spionagesonde)) && bezahlbar(&bestand,&r.kosten_einheit(Einheit::Spionagesonde,sp.volk)) {
                        tu(welt,sid,json!({"typ":"fertigen","planet":ks,"einheit":"spionagesonde","anzahl":1}));
                    }
                }
                return;
            }
            if welt.kolonisation.aktiv {
                if !sp.erkundet.contains_key(&ziel) {
                    if !welt.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::Spionage && f.ziel == ziel) {
                        if p.einheiten[Einheit::Spionagesonde.idx()] > 0 {
                            if tu(welt, sid, json!({"typ":"flotte_senden","start":ks,"ziel":ziel.to_string(),"mission":"spionage","schiffe":{"spionagesonde":1}})) {
                                gespaeht.insert(ziel, welt.zeit);
                            }
                        } else if !p.fertigung[0].iter().any(|f| f.produkt == Produkt::Einheit(Einheit::Spionagesonde)) && bezahlbar(&bestand,&r.kosten_einheit(Einheit::Spionagesonde,sp.volk)) {
                            tu(welt, sid, json!({"typ":"fertigen","planet":ks,"einheit":"spionagesonde","anzahl":1}));
                        }
                    }
                    return;
                }
                let escort = Einheit::ALLE[..SCHIFFE].iter().copied().find(|e| *e != Einheit::Spionagesonde && *e != Einheit::Kolonieschiff && p.einheiten[e.idx()] > 0 && r.einh(*e).angriff > 0);
                let Some(escort) = escort else {
                    if !p.fertigung[0].iter().any(|f| f.produkt == Produkt::Einheit(Einheit::LeichterJaeger)) && bezahlbar(&bestand,&r.kosten_einheit(Einheit::LeichterJaeger,sp.volk)) {
                        tu(welt, sid, json!({"typ":"fertigen","planet":ks,"einheit":"leichter_jaeger","anzahl":1}));
                    }
                    return;
                };
                let required = welt.koloniefracht(sid);
                if required.iter().enumerate().any(|(i,n)| bestand[i] < *n) { return; }
                let cargo: serde_json::Map<String, serde_json::Value> = Gut::ALLE.iter().filter(|g| required[g.idx()] > 0)
                    .map(|g| (g.name().to_string(), json!((required[g.idx()] + M - 1) / M))).collect();
                let mut ships = serde_json::Map::new();
                ships.insert("kolonieschiff".into(), json!(1));
                ships.insert(escort.name().into(), json!(1));
                tu(welt, sid, json!({"typ":"flotte_senden","start":ks,"ziel":ziel.to_string(),"mission":"kolonisieren","schiffe":ships,"ladung":cargo}));
                return;
            }
            // Die Siedler brauchen Nahrung, bis die erste Farm steht.
            let nahrung = ganz(bestand[Gut::Nahrung.idx()]).min(2500);
            let erz = ganz(bestand[Gut::Erz.idx()]).min(3000);
            let kristall = ganz(bestand[Gut::Kristall.idx()]).min(2000);
            tu(welt, sid, json!({"typ": "flotte_senden", "start": ks, "ziel": ziel.to_string(), "mission": "kolonisieren",
                "schiffe": {"kolonieschiff": 1}, "geschwindigkeit": 1.0, "ladung": {"nahrung": nahrung, "erz": erz, "kristall": kristall}}));
        }
        return;
    }
    let bestellt = |produkt: Produkt, schleife: usize| -> i64 { p.fertigung[schleife].iter().filter(|f| f.produkt == produkt).map(|f| f.rest).sum() };
    if bestellt(Produkt::Einheit(Einheit::Kolonieschiff), 0) > 0 {
        return;
    }
    let schiff = &r.einh(Einheit::Kolonieschiff).kosten;
    let volk = sp.volk;
    let fuer_stufe_v = r.stufen.last().map(|s| s.kolonien as usize).unwrap_or(0);
    let fehlend = (welt.kolonien_erlaubt(sid).max(fuer_stufe_v) as i64 - welt.kolonien(sid) as i64 - unterwegs as i64).max(0);
    let (bevoelkerung, orbitalwerft_frei) = (p.bevoelkerung, p.fertigung[1].is_empty());
    let teile = [Gut::Habitatmodul, Gut::Antriebskern];
    let bestellt_teile: Vec<i64> = teile.iter().map(|g| bestellt(Produkt::Bauteil(*g), 1)).collect();
    let mut genug = true;
    let mut rest = bestand;
    for (i, g) in teile.into_iter().enumerate() {
        let je_schiff = schiff.get(&g).copied().unwrap_or(0);
        let hat = ganz(bestand[g.idx()]) + bestellt_teile[i];
        genug &= ganz(bestand[g.idx()]) >= je_schiff;
        // Bauteile für alle noch erlaubten Kolonien vorab: Habitatmodule kosten Konsumgüter, und die
        // verbraucht eine große Bevölkerung später vollständig selbst.
        // Alle fehlenden auf einmal, soweit der Bestand reicht: Konsumgüter liegen oft nur kurz nach einer Lieferung.
        if hat < je_schiff * fehlend && orbitalwerft_frei {
            if let Some(kosten) = r.kosten_bauteil(g) {
                let n = (0..GUETER).filter(|x| kosten[*x] > 0).map(|x| rest[x] / kosten[x]).min().unwrap_or(0).min(je_schiff * fehlend - hat);
                if n >= 1 && tu(welt, sid, json!({"typ": "fertigen", "planet": ks, "bauteil": g.name(), "anzahl": n})) {
                    for x in 0..GUETER {
                        rest[x] -= kosten[x] * n;
                    }
                }
            }
        }
    }
    if genug && bezahlbar(&welt.bestand_jetzt(pid), &r.kosten_einheit(Einheit::Kolonieschiff, volk)) && bevoelkerung > (r.wirtschaft.siedler + 6000) * M {
        tu(welt, sid, json!({"typ": "fertigen", "planet": ks, "einheit": "kolonieschiff", "anzahl": 1}));
    }
}

/// Ein Zug eines Bots im aktuellen Fenster.
pub fn zug(welt: &mut Welt, sid: SpielerId, bot: &mut Bot) {
    if welt.zeit < bot.naechster || welt.beendet() || !welt.spieler_aktiv(sid) {
        return;
    }
    bot.naechster = welt.zeit + TAKT;
    allgemein(welt, sid);
    for pid in welt.spieler[sid as usize].planeten.clone() {
        bauen(welt, sid, pid as usize, bot.typ);
    }
    forschen(welt, sid, bot.typ);
    match bot.typ {
        Bottyp::Raeuber => {
            raeuber(welt, sid, bot);
            verteidigen(welt, sid, bot.typ);
        }
        Bottyp::Igel => verteidigen(welt, sid, bot.typ),
        Bottyp::Haendler => {
            haendler(welt, sid);
            verteidigen(welt, sid, bot.typ);
        }
        Bottyp::Oekonom => verteidigen(welt, sid, bot.typ),
        Bottyp::Wehrlos => {}
    }
    logistik(welt, sid, bot.typ);
    // Probe memory older than the longest pause (a week) no longer decides anything.
    let jetzt = welt.zeit;
    bot.gespaeht.retain(|_, t| jetzt - *t < KOLONIE_SPAEHPAUSE);
    kolonisieren(welt, sid, &mut bot.gespaeht);
    diplomatie(welt, sid, bot.typ);
}

#[cfg(test)]
mod interactive_colony_tests {
    use super::*;
    #[test]
    fn public_diplomacy_supports_more_than_fifty_players() {
        let mut rules = Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        rules.welt.start_abstand=1;
        rules.welt.systeme_je_sektor=80;
        let mut w=Welt::neu(rules,712,128).unwrap();
        w.zeit=2*TAG;
        diplomatie(&mut w,100,Bottyp::Haendler);
        assert!(w.vertraege.iter().any(|v|v.a==100&&v.b==101));
        assert_eq!(w.spieler[100].statistik.abgelehnt,0);
    }
    #[test]
    fn raid_estimate_uses_the_defenders_faction() {
        let mut rules = kern::Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        rules.voelker.get_mut(&Volk::Krath).unwrap().waffen = 1000.0;
        rules.voelker.get_mut(&Volk::Krath).unwrap().panzerung = 1000.0;
        let mut w = Welt::neu(rules, 43, 2).unwrap();
        w.spieler[0].volk = Volk::Aurelianer;
        w.spieler[1].volk = Volk::Krath;
        let mut ships = vec![0; SCHIFFE];
        ships[Einheit::LeichterJaeger.idx()] = 5;
        let report = Spionagebericht { zeit:0, ziel:w.planeten[w.spieler[1].heimat as usize].koord, besitzer:1,
            bestand:vec![0; GUETER], schiffe:Some(ships), verteidigung:Some(vec![0; EINHEITEN-SCHIFFE]), gebaeude:None, forschung:None };
        let mut attackers = [0; EINHEITEN];
        attackers[Einheit::LeichterJaeger.idx()] = 1000;
        assert_eq!(angriff_schaetzen(&w, 0, &report, &attackers).unwrap().0, 0);
        w.spieler[1].volk = Volk::Aurelianer;
        assert_eq!(angriff_schaetzen(&w, 0, &report, &attackers).unwrap().0, 100);
    }

    #[test]
    fn bots_repair_bombarded_food_production_before_upgrading() {
        for typ in Bottyp::ALLE {
            let rules = kern::Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
            let mut w = Welt::neu(rules, 42, 2).unwrap();
            let pid = w.spieler[0].heimat as usize;
            w.spieler[0].volk = Volk::Aurelianer;
            w.planeten[pid].bestand.fill(100_000 * M);
            w.kolonisation.integritaet.insert((pid as PlanetId, Gebaeude::Farm), 0);
            w.raten_neu(pid);
            bauen(&mut w, 0, pid, typ);
            assert!(w.kolonisation.reparaturen.contains_key(&(pid as PlanetId, Gebaeude::Farm)), "{typ:?} did not repair destroyed farm");
            assert!(w.planeten[pid].bauschleife.is_empty());
            let done = w.kolonisation.reparaturen[&(pid as PlanetId, Gebaeude::Farm)].fertig;
            while w.zeit <= done { w.schritt(); }
            assert_eq!(w.integritaet(pid, Gebaeude::Farm), 1000);
        }
    }
    #[test]
    fn repairs_reserve_resources_and_do_not_spawn_duplicate_orders() {
        let rules = kern::Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let mut w = Welt::neu(rules, 42, 2).unwrap();
        let pid = w.spieler[0].heimat as usize;
        w.spieler[0].volk = Volk::Syntheten;
        w.planeten[pid].bestand.fill(100_000 * M);
        w.kolonisation.integritaet.insert((pid as PlanetId, Gebaeude::Solarkraftwerk), 0);
        w.raten_neu(pid);
        let costs = w.reparaturkosten(pid, Gebaeude::Solarkraftwerk).unwrap();
        assert!(costs.iter().any(|n| *n > 0));
        assert_eq!(ruecklage(&w, 0, pid, Bottyp::Raeuber), costs);
        bauen(&mut w, 0, pid, Bottyp::Raeuber);
        assert!(w.kolonisation.reparaturen.contains_key(&(pid as PlanetId, Gebaeude::Solarkraftwerk)));
        let stock = w.bestand_jetzt(pid);
        let actions = w.spieler[0].statistik.aktionen;
        bauen(&mut w, 0, pid, Bottyp::Raeuber);
        assert_eq!(w.bestand_jetzt(pid), stock);
        assert_eq!(w.spieler[0].statistik.aktionen, actions);
        assert_eq!(w.spieler[0].statistik.abgelehnt, 0);
        assert!(w.planeten[pid].bauschleife.is_empty());
    }
    #[test]
    fn logistics_orders_only_the_affordable_number_of_transporters() {
        let rules = Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let mut w = Welt::neu(rules, 42, 2).unwrap();
        let pid = w.spieler[0].heimat as usize;
        let colony = w.spieler[1].heimat;
        w.spieler[1].planeten.clear();
        w.spieler[0].planeten.push(colony);
        w.planeten[colony as usize].besitzer = 0;
        w.planeten[colony as usize].heimat = false;
        w.spieler[0].stufe = 3;
        w.spieler[0].forschung.fill(10);
        w.planeten[pid].gebaeude.fill(10);
        w.planeten[pid].bestand = w.regeln.kosten_einheit(Einheit::GrosserTransporter, w.spieler[0].volk);
        logistik(&mut w, 0, Bottyp::Haendler);
        assert_eq!(w.spieler[0].statistik.abgelehnt, 0);
        let orders: i64 = w.planeten[pid].fertigung[0].iter().filter(|f|f.produkt==Produkt::Einheit(Einheit::GrosserTransporter)).map(|f|f.rest).sum();
        assert_eq!(orders, 1, "a one-ship budget must produce one transporter, not reject a two-ship order");
    }
    #[test]
    fn colony_ship_budget_is_rechecked_after_component_orders() {
        let rules = Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let mut w = Welt::neu(rules, 42, 2).unwrap();
        let pid = w.spieler[0].heimat as usize;
        w.spieler[0].stufe = 4;
        w.spieler[0].forschung.fill(10);
        w.planeten[pid].gebaeude.fill(10);
        w.planeten[pid].bevoelkerung = 100_000 * M;
        w.planeten[pid].bestand = w.regeln.kosten_einheit(Einheit::Kolonieschiff, w.spieler[0].volk);
        kolonisieren(&mut w, 0, &mut BTreeMap::new());
        assert!(!w.planeten[pid].fertigung[1].is_empty(), "fixture must actually spend resources on orbital components");
        assert_eq!(w.spieler[0].statistik.abgelehnt, 0, "colony ship used the pre-component stock snapshot");
        assert!(w.bestand_jetzt(pid).iter().all(|n|*n>=0));
    }
    #[test]
    fn logistics_waits_for_the_complete_round_trip_fuel_budget() {
        let rules = Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let mut w = Welt::neu(rules, 83, 2).unwrap();
        let pid = w.spieler[0].heimat as usize;
        let target = w.planeten[w.spieler[1].heimat as usize].koord;
        w.spieler[0].erkundet.insert(target, Erkundung {zeit:0,felder:180,zone:Zone::Leben,reich_erz:1000,reich_kristall:1000,nebel:false});
        w.spieler[0].stufe = 3;
        w.spieler[0].forschung.fill(10);
        w.planeten[pid].gebaeude[Gebaeude::Raumhafen.idx()] = 1;
        w.planeten[pid].einheiten[Einheit::GrosserTransporter.idx()] = 1;
        w.planeten[pid].bestand.fill(100_000 * M);
        let mut ships = [0; SCHIFFE];
        ships[Einheit::GrosserTransporter.idx()] = 1;
        let fuel = 2 * w.flugplan(0, pid, target, &ships, 1000).unwrap().treibstoff;
        assert!(fuel > 0);
        let mut cargo = [0; GUETER];
        cargo[Gut::Erz.idx()] = 5000 * M;
        cargo[Gut::Deuterium.idx()] = 500 * M;
        w.planeten[pid].bestand[Gut::Deuterium.idx()] = fuel - 1;
        assert!(ladung_fuer(&w, 0, pid, target, 1, cargo).is_none(), "even a fractional fuel shortage must prevent a logistics order");
        w.planeten[pid].bestand[Gut::Deuterium.idx()] = fuel;
        let load = ladung_fuer(&w, 0, pid, target, 1, cargo).unwrap();
        assert_eq!(load[Gut::Deuterium.idx()], 0);
        let (ok, message) = w.handeln(0, Rolle::Alle, &json!({"typ":"flotte_senden","start":w.planeten[pid].koord.to_string(),"ziel":target.to_string(),"mission":"transport","schiffe":{"grosser_transporter":1},"ladung":ladung_json(&load)}));
        assert!(ok, "a fully funded transport must still start: {message}");
        assert_eq!(w.bestand_jetzt(pid)[Gut::Deuterium.idx()], 0);
        assert_eq!(w.spieler[0].statistik.abgelehnt, 0);
    }
    #[test]
    fn modern_bot_scouts_then_sends_escort_and_required_cargo() {
        let rules = kern::Regelwerk::laden(include_str!("../../../regeln/regelwerk.ron")).unwrap();
        let mut w = Welt::neu(rules, 42, 6).unwrap();
        w.kolonisationsregeln_v2_aktivieren();
        let p = w.spieler[0].heimat as usize;
        w.spieler[0].stufe = 4;
        w.spieler[0].forschung[Forschung::Astrophysik.idx()] = 4;
        w.planeten[p].gebaeude.fill(10);
        w.planeten[p].bevoelkerung = 100_000 * M;
        w.planeten[p].bestand.fill(100_000 * M);
        for e in [Einheit::Spionagesonde, Einheit::Kolonieschiff, Einheit::LeichterJaeger] { w.planeten[p].einheiten[e.idx()] = 1; }
        kolonisieren(&mut w, 0, &mut BTreeMap::new());
        let probe = w.flotten.values().find(|f| f.besitzer == 0 && f.mission == Mission::Spionage).unwrap_or_else(|| panic!("probe missing: {:?}",w.logpuffer));
        let target = probe.ziel;
        for _ in 0..192 {
            if w.spieler[0].erkundet.contains_key(&target) && !w.flotten.values().any(|f| f.besitzer == 0 && f.mission == Mission::Spionage) { break; }
            w.schritt();
        }
        assert!(w.spieler[0].erkundet.contains_key(&target));
        w.planeten[p].bestand.fill(100_000 * M);
        kolonisieren(&mut w, 0, &mut BTreeMap::new());
        let colony = w.flotten.values().find(|f| f.besitzer == 0 && f.mission == Mission::Kolonisieren).unwrap_or_else(|| panic!("colony missing: {:?}",w.logpuffer));
        assert_eq!(colony.ziel, target);
        assert!(colony.schiffe[Einheit::LeichterJaeger.idx()] > 0);
        let required = w.koloniefracht(0);
        for g in Gut::ALLE { assert!(colony.ladung[g.idx()] >= required[g.idx()]); }
    }
}

/// Baseline policies are explicitly script decisions, never presented as LLM reasoning.
fn kommunizieren(w:&mut Welt,sid:SpielerId,typ:Bottyp) {
    reagieren(w, sid, typ);
    let names=w.spieler.iter().map(|s|s.name.clone()).collect::<Vec<_>>();
    initiativen(w, sid, typ, &names);
}

/// Immediate, bounded diplomacy, independent of the economic turn. Kernel-only worlds mark
/// script players with the `ki` flag; hosts that run bots themselves use `reagieren_mit`.
pub fn reagieren(w:&mut Welt,sid:SpielerId,typ:Bottyp) {
    let scripts=w.spieler.iter().filter(|s|s.ki).map(|s|s.id).collect();
    reagieren_mit(w,sid,typ,&scripts);
}

/// Same policy with the host's real script registry (online and local runtimes switch the
/// kernel `ki` flag off, so it cannot identify their bots). Cheap when nothing is pending:
/// called for every bot after each player command in a local round.
pub fn reagieren_mit(w:&mut Welt,sid:SpielerId,typ:Bottyp,scripts:&std::collections::BTreeSet<SpielerId>) {
    for id in w.offene_antworten(sid).into_iter().take(4) {
        let n=w.nachrichten[id as usize-1].clone();
        let betreff=w.kommunikation.briefe.get(&id).map(|b|b.betreff.clone()).unwrap_or_default();
        let detail=antwort_text(&format!("{betreff} {}",n.text),typ);
        let (an,ich)=(w.spieler[n.von as usize].name.clone(),w.spieler[sid as usize].name.clone());
        tu(w,sid,json!({"typ":"brief_senden","kanal":"privat","an":an,"betreff":"Antwort","text":format!("Guten Tag, {an}. Ich bin {ich}. {detail}"),"antwort_auf":id}));
    }
    allianz_reagieren(w,sid,scripts);
    let requests=w.kommunikation.anfragen.iter().filter(|a|a.zielallianz.is_none()&&a.an==sid&&a.status=="offen"&&a.vertrag.is_none_or(|id|w.vertraege.iter().any(|v|v.id==id&&v.status==Vertragsstatus::Angeboten))).map(|a|(a.id,a.art.clone())).take(4).collect::<Vec<_>>();
    for (id,art) in requests {
        let accept=typ!=Bottyp::Raeuber && (art!="allianz"||w.spieler[sid as usize].allianz.is_none());
        tu(w,sid,json!({"typ":"diplomatie_entscheiden","anfrage":id,"annehmen":accept}));
    }
    let council=w.kommunikation.anfragen.iter().filter(|a|a.status=="offen"&&a.zielallianz.is_some_and(|id|w.allianz_leitung(sid,id))).take(4).map(|a|a.id).collect::<Vec<_>>();
    for id in council {tu(w,sid,json!({"typ":"diplomatie_entscheiden","anfrage":id,"annehmen":typ!=Bottyp::Raeuber}));}
    if typ != Bottyp::Raeuber && w.spieler[sid as usize].allianz.is_none() {
        if let Some(name) = w.allianzen.iter().find(|a| a.eingeladen.contains(&sid)).map(|a| a.name.clone()) {
            tu(w,sid,json!({"typ":"allianz_beitreten","allianz":name}));
        }
    }
    // Contract offers are decided here as well, so a player's offer gets its answer as soon
    // as the letters and requests do (the economic turn finds nothing left afterwards).
    let offen=w.vertraege.iter().filter(|v|v.b==sid&&v.status==Vertragsstatus::Angeboten).map(|v|v.id).collect::<Vec<_>>();
    for id in offen {
        tu(w,sid,json!({"typ":if typ==Bottyp::Raeuber {"vertrag_ablehnen"} else {"vertrag_annehmen"},"vertrag":id}));
    }
}

/// Script reply to a letter: the topics in the order the writer raised them (at most two),
/// each answered with what this bot type really does in `reagieren_mit` and its turn.
/// Nothing is promised that the policy would not carry out.
fn antwort_text(text: &str, typ: Bottyp) -> String {
    let t = format!(" {} ", text.to_lowercase());
    let raeuber = typ == Bottyp::Raeuber;
    let themen: [(&[&str], &str); 5] = [
        (&["allianz", "bündnis", "buendnis", "verbünd", "verbuend"],
            if raeuber { "Allianzen gehe ich nicht ein; ich bleibe unabhängig. Einzelne Geschäfte sind trotzdem möglich." }
            else { "Eine Allianz lebt von verlässlicher Versorgung und gemeinsamer Verteidigung. Lade mich in deine Allianz ein oder schicke eine diplomatische Anfrage; solange ich in keiner Allianz bin, nehme ich an." }),
        (&["frieden", "nichtangriff", " pakt", "waffenstillstand", "krieg"],
            if raeuber { "Pakte schließe ich nicht. Wer Ruhe will, sichert Lager und Verteidigung." }
            else { "Ein Nichtangriffspakt ist mir recht. Biete ihn unter Diplomatie als Vertrag an; ich nehme ihn an." }),
        (&["handel", " erz", "kristall", " ware ", "güter", "tausch", "kauf", "rohstoff"],
            match typ {
                Bottyp::Raeuber => "Nenne mir Ware, Menge und Gegenleistung. Ich handle nur über den Markt gegen sofortige Bezahlung; eine Nachricht allein liefert nichts.",
                Bottyp::Haendler => "Nenne mir Ware, Menge und Gegenleistung. Verbindlich wird es über ein Angebot im Markt oder im internen Markt unserer Allianz; eine Nachricht allein löst noch keine Lieferung aus.",
                _ => "Nenne mir Ware, Menge und Gegenleistung. Ein Angebot im Markt oder ein Handelsabkommen können wir verbindlich prüfen; eine Nachricht allein löst noch keine Lieferung aus.",
            }),
        (&["angriff", "hilfe", "schutz", "verteidig", "bedroh"],
            if raeuber { "Hilfe gibt es von mir nicht umsonst. Sichere deine Lager und unterschätze keinen fremden Anflug." }
            else { "Nenne das betroffene System und den Bedarf. Verbündete können einen Hilferuf prüfen. Eine Zusage ohne verfügbare Schiffe gebe ich dir nicht." }),
        (&[" wie ", "erkund", "anfäng", "anfang", "tipp"],
            "Beginne mit Energie und Versorgung, baue danach Wirtschaft und Forschung aus. In der Sternenkarte kannst du Systeme erkunden; der Codex erklärt Gebäude, Flotten und Verträge."),
    ];
    let mut gefunden: Vec<(usize, &str)> = themen.iter()
        .filter_map(|(worte, antwort)| worte.iter().filter_map(|w| t.find(w)).min().map(|pos| (pos, *antwort)))
        .collect();
    gefunden.sort_by_key(|(pos, _)| *pos);
    if gefunden.is_empty() {
        return match typ {
            Bottyp::Raeuber => "Meine Flottenplanung lässt mir gerade keine Zeit für weitere Gespräche.",
            Bottyp::Igel => "Ich sichere mein Reich. Über gegenseitige Hilfe und einen Nichtangriffspakt können wir sprechen.",
            Bottyp::Haendler => "Ich suche verlässliche Handelspartner. Welche Waren brauchst du oder kannst du anbieten?",
            _ => "Ich baue eine langfristige Wirtschaft auf. Welche Zusammenarbeit stellst du dir vor?",
        }.into();
    }
    gefunden.iter().take(2).map(|(_, a)| *a).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod antwort_tests {
    use super::*;
    #[test]
    fn antworten_folgen_den_themen_des_briefs_und_versprechen_nur_was_der_bot_tut() {
        let a = antwort_text("Hallo! Wollen wir eine Allianz gründen und gemeinsam handeln?", Bottyp::Oekonom);
        assert!(a.starts_with("Eine Allianz lebt"), "{a}");
        assert!(a.contains("Ware, Menge und Gegenleistung"), "{a}");
        let r = antwort_text("Bündnis und Frieden?", Bottyp::Raeuber);
        assert!(r.contains("Allianzen gehe ich nicht ein") && r.contains("Pakte schließe ich nicht"), "{r}");
        assert!(antwort_text("Ich möchte Erz handeln", Bottyp::Haendler).contains("internen Markt"));
        assert!(antwort_text("Guten Morgen", Bottyp::Igel).starts_with("Ich sichere mein Reich"));
        // Word starts, not fragments: "herzlich" is no ore trade, "waren" no goods.
        assert!(antwort_text("Herzliche Grüße, wir waren Nachbarn", Bottyp::Oekonom).starts_with("Ich baue"));
        // "wie" only as a word: "wieder" is no beginner question.
        assert!(!antwort_text("Bis wieder", Bottyp::Oekonom).contains("Beginne mit Energie"));
        assert!(antwort_text("Wie fange ich an?", Bottyp::Oekonom).contains("Beginne mit Energie"));
    }
}

/// Online runtimes disable the core AI flag to avoid a second economic controller.
/// Supply their real script registry instead of inferring controllers from that flag.
pub fn allianz_reagieren(w:&mut Welt,sid:SpielerId,scripts:&std::collections::BTreeSet<SpielerId>) {
    // One script representative answers the newest human/model alliance message.
    // Reply links prevent bot-to-bot chains and repeated answers on later ticks.
    if let Some(aid)=w.spieler[sid as usize].allianz {
        let representative=w.allianzen.iter().find(|a|a.id==aid).and_then(|a|a.mitglieder.iter().copied().filter(|id|scripts.contains(id)&&w.spieler_aktiv(*id)).min());
        if representative==Some(sid) {
            // Letter ids grow with time: scanning back one game day keeps this independent of
            // the epoch length. Older unanswered chat lines are history, not a pending question.
            let seit=w.zeit-TAG;
            let root=w.kommunikation.briefe.iter().rev().take_while(|(id,_)|w.nachrichten[**id as usize-1].zeit>=seit).find(|(id,b)|b.kanal=="allianz"&&b.antwort_auf.is_none()&&!b.automatisch&&w.nachrichten[**id as usize-1].von!=sid&&!scripts.contains(&w.nachrichten[**id as usize-1].von)&&w.brief_sichtbar(sid,**id)).map(|(id,_)|*id);
            if let Some(id)=root {
                if !w.kommunikation.briefe.range(id..).any(|(bid,b)|b.antwort_auf==Some(id)&&w.nachrichten[*bid as usize-1].von==sid) {
                    let n=&w.nachrichten[id as usize-1];let sender=w.spieler[n.von as usize].name.clone();let topic=n.text.to_lowercase();
                    let text=if topic.contains("hilfe")||topic.contains("angriff"){"Bitte trage den betroffenen Planeten und deinen Bedarf als Hilferuf ein. Ich prüfe meine verfügbaren Kräfte."}else if topic.contains("handel")||topic.contains("erz")||topic.contains("kristall"){"Stelle Ware, Menge und Preis im Allianzbereich Wirtschaft ein. Dort können wir verbindlich handeln und die Lieferung verfolgen."}else{"Willkommen in unserer Allianz. Lass uns Versorgung und Forschung abstimmen. Über den internen Markt und die Hilferufe bleiben unsere Absprachen nachvollziehbar."};
                    tu(w,sid,json!({"typ":"brief_senden","kanal":"allianz","an":"","betreff":"Antwort im Allianzchat","text":format!("Guten Tag, {sender}. {text}"),"antwort_auf":id}));
                }
            }
        }
    }
}

fn initiativen(w:&mut Welt,sid:SpielerId,typ:Bottyp,names:&[String]) {
    // One initiative per day and seat; no reply chains and no hidden enemy knowledge.
    if w.zeit % TAG >= TAKT || w.zeit<TAG {return;}
    let already=w.kommunikation.ereignisse.iter().rev().take_while(|e|e["time"].as_i64().unwrap_or(0)>=w.zeit-w.zeit%TAG).any(|e|e["player_id"]==sid && e["event_type"]=="communication.sent");
    if already {return;}
    if w.spieler[sid as usize].allianz.is_none() && typ!=Bottyp::Raeuber {
        let invitation=w.allianzen.iter().find(|a|a.eingeladen.contains(&sid)).map(|a|a.name.clone());
        if let Some(name)=invitation {tu(w,sid,json!({"typ":"allianz_beitreten","allianz":name}));}
        else if typ==Bottyp::Haendler {tu(w,sid,json!({"typ":"allianz_gruenden","name":format!("Bund {}",names[sid as usize])}));}
    }
    let partner=(0..w.spieler.len() as u16).cycle().skip(sid as usize+1).take(w.spieler.len()-1).find(|s|w.spieler_aktiv(*s));
    if let Some(p)=partner {
        if let Some(aid)=w.spieler[sid as usize].allianz {
            if w.allianz_leitung(sid,aid) && w.spieler[p as usize].allianz.is_none() {tu(w,sid,json!({"typ":"allianz_einladen","spieler":names[p as usize]}));}
            if w.spieler[p as usize].allianz.is_some_and(|a|a!=aid) && w.allianz_leitung(sid,aid) {
                let name=w.allianzen.iter().find(|a|Some(a.id)==w.spieler[p as usize].allianz).unwrap().name.clone();
                tu(w,sid,json!({"typ":"brief_senden","kanal":"diplomatie","an":name,"betreff":"Handelsbeziehungen","text":"Unsere Allianz möchte über sichere Handelswege und gegenseitige Hilfe sprechen.","antwort_auf":null}));
                let target=w.spieler[p as usize].allianz;
                if typ==Bottyp::Haendler&&!w.kommunikation.anfragen.iter().any(|a|a.art=="handelsabkommen"&&["offen","angenommen"].contains(&a.status.as_str())&&((a.allianz==Some(aid)&&a.zielallianz==target)||(a.allianz==target&&a.zielallianz==Some(aid)))) {
                    tu(w,sid,json!({"typ":"allianz_anfrage","allianz":name,"art":"handelsabkommen","text":"Wir bieten ein verbindliches Handelsabkommen für unsere Allianzen an."}));
                }
            }
            let k=w.planeten[w.spieler[sid as usize].heimat as usize].koord;
            // At most one weekly report per alliance and only with real content (open offers
            // and help calls). Daily boilerplate from every bot flooded the chat and grew each
            // save by hundreds of letters per game day.
            let woche=w.zeit-7*TAG;
            if (w.zeit/TAG)%7==0 && !w.kommunikation.briefe.iter().rev().take_while(|(id,_)|w.nachrichten[**id as usize-1].zeit>=woche).any(|(_,b)|b.kanal=="allianz"&&b.betreff=="Wochenbericht"&&b.allianzen.contains(&aid)) {
                let angebote=w.kommunikation.angebote.iter().filter(|o|o.allianz==aid&&o.status=="offen").count();
                let hilfe=w.kommunikation.hilfe.iter().filter(|h|h.allianz==aid&&h.status=="offen").map(|h|h.planet.to_string()).collect::<Vec<_>>();
                if angebote>0 || !hilfe.is_empty() {
                    let text=format!("Wochenbericht: {angebote} offene Angebote im internen Markt{}.",if hilfe.is_empty(){String::new()}else{format!(", offene Hilferufe für {}",hilfe.join(", "))});
                    tu(w,sid,json!({"typ":"brief_senden","kanal":"allianz","an":"","betreff":"Wochenbericht","text":text,"antwort_auf":null}));
                }
            }
            if !w.sichtbare_angriffe(sid).is_empty() && !w.kommunikation.hilfe.iter().any(|h|h.von==sid&&h.status=="offen") {tu(w,sid,json!({"typ":"allianz_hilfe","planet":k,"text":"Sensoren melden einen Angriff. Bitte Entsatz prüfen."}));}
            let help=w.kommunikation.hilfe.iter().find(|h|h.allianz==aid&&h.von!=sid&&h.status=="offen"&&!h.helfer.contains(&sid)).map(|h|h.id);
            if let Some(id)=help {tu(w,sid,json!({"typ":"allianz_hilfe_status","hilfe":id,"erledigt":false}));}
            let pid=w.spieler[sid as usize].heimat as usize;
            if typ==Bottyp::Haendler && w.planeten[pid].gebaeude[Gebaeude::Markt.idx()]>0 && w.bestand_jetzt(pid)[Gut::Erz.idx()]>5000*M && !w.kommunikation.angebote.iter().any(|o|o.von==sid&&o.status=="offen") {tu(w,sid,json!({"typ":"intern_anbieten","planet":k,"gut":"erz","menge":100,"preis":1}));}
            let offer=w.kommunikation.angebote.iter().find(|o|o.allianz==aid&&o.von!=sid&&o.status=="offen"&&o.preis<=M&&o.menge<=1000*M).map(|o|o.id);
            if typ!=Bottyp::Raeuber && w.spieler[sid as usize].credits>2000*M {if let Some(id)=offer {tu(w,sid,json!({"typ":"intern_kaufen","angebot":id,"planet":k}));}}
        }
        if !w.privat_kontakt(sid,p) {
            tu(w,sid,json!({"typ":"brief_senden","kanal":"privat","an":names[p as usize],"betreff":"Erster Kontakt","text":format!("Ich bin {}. Welche langfristigen Interessen hat dein Reich?",names[sid as usize]),"antwort_auf":null}));
        }
    }
}
