//! Skriptbots fürs Balancing: Ökonom, Räuber, Igel und Händler.
//!
//! Die Bots handeln über dieselben Aktionen wie die Agenten. Über andere Spieler wissen
//! sie nur, was öffentlich ist oder was ihre eigenen Sonden berichtet haben.

#[path = "bot_dialog.rs"]
mod dialog;
#[path = "bot_monde.rs"]
mod monde_bot;
#[path = "bot_invasion.rs"]
mod invasion_bot;
#[path = "bot_dyson.rs"]
mod dyson_bot;
#[path = "bot_allianz.rs"]
mod allianz_bot;
#[path = "bot_abbau.rs"]
mod abbau_bot;
#[path = "bot_krieg.rs"]
mod krieg_bot;

use kern::regeln::{preis_array, Regelwerk, MAX_FORSCHUNG};
use kern::kampf::{kampf_ohne_protokoll, Gruppe};
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
    /// Schutz der Opfer vor Räuberbots; der Server setzt ihn aus seiner Startkonfiguration (`--opferschutz`).
    #[serde(default)]
    pub opferschutz: Opferschutz,
}

impl Bot {
    pub fn neu(typ: Bottyp, sid: SpielerId) -> Bot {
        // Versetzter Takt, damit nicht alle Bots im selben Fenster handeln.
        Bot { typ, naechster: (sid as i64 % 8) * 900, gespaeht: BTreeMap::new(), opferschutz: Opferschutz::default() }
    }
}

/// Opferschutz gegen Räuberbots (Karl 20.10.2026: „muss zum Serverstart eingestellt und auch ausgeschaltet werden
/// können“). Aktiv: Neulinge (unter Stufe II oder Stufe II und Heimatwelt jünger als `neulinge_tage`) werden weder
/// ausgespäht noch angegriffen, und jedes Reich wird von allen Räubern zusammen höchstens einmal je `pause_stunden`
/// angegriffen. Aus: Räuber greifen jedes lohnende Ziel an, sobald der Simulator den Sieg verspricht.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opferschutz {
    pub aktiv: bool,
    pub pause_stunden: u32,
    pub neulinge_tage: u32,
}

impl Default for Opferschutz {
    /// Bot-Grundwert (Analyseläufe, ältere Spielstände). Neue Server- und App-Welten starten dagegen mit
    /// `Opferschutz::neue_welt()` = aus (Karl 23.10.2026); der Server gibt jedem Bot seinen Stand vor.
    fn default() -> Self {
        Opferschutz { aktiv: true, pause_stunden: 24, neulinge_tage: 5 }
    }
}

impl Opferschutz {
    /// Neue Welten: aus (Karl 23.10.2026: „beides aus! man kann es beim Start des Servers einstellen“) –
    /// einschalten mit `--opferschutz an` (Server) bzw. `settings.opferschutz` / `/api/local/control`.
    pub fn neue_welt() -> Self {
        Opferschutz { aktiv: false, ..Opferschutz::default() }
    }
}

const TAKT: i64 = 2 * STUNDE;
/// A probed colony target that is not confirmed free stays off the list this long.
const KOLONIE_SPAEHPAUSE: i64 = 7 * TAG;
/// Belegung des Wohnraums, ab der der nächste Wohnblock Vorrang hat (siehe `waehle_bau`).
const WOHNEN_AB: f64 = 0.4;
/// Ohne anderen sinnvollen Bau: Wohnraum auf Vorrat ab dieser Belegung.
const WOHNEN_VORRAT_AB: f64 = 0.25;
/// So viele freie Felder bleiben für Wohnblöcke, Minen und Lager bauen nicht hinein.
const FELDER_FUER_WOHNRAUM: u16 = 4;

thread_local! {
    static ZAEHLER: std::cell::RefCell<BTreeMap<&'static str, u64>> = const { std::cell::RefCell::new(BTreeMap::new()) };
}
/// Diagnose für `bot_lage`: wie oft eine Botentscheidung an welchem Grund hing (je Thread, nie gespeichert,
/// ohne Einfluss auf das Spiel).
fn zaehle(grund: &'static str) {
    ZAEHLER.with(|z| *z.borrow_mut().entry(grund).or_default() += 1);
}
#[allow(dead_code)] // nur für Diagnosebeispiele und Tests, nicht im Programm `sternenepoche`
pub fn diagnose_zaehler() -> BTreeMap<&'static str, u64> {
    ZAEHLER.with(|z| z.borrow().clone())
}

/// Free power and workers from stockpiled raw extraction before buying more buildings.
/// Shared core predictions decide every step; food, power and processing stay supplied.
fn last_ausgleichen(w:&mut Welt,sid:SpielerId) {
    if w.ausbau.industrie_version==0{return;}
    for id in w.spieler[sid as usize].planeten.clone() {
        let pid=id as usize;
        // Expensive synthetic xeno must not drain electronics needed for research and industry.
        let stock=w.bestand_jetzt(pid);let levels=w.produktionsstufen(pid);
        if !w.xeno_planet(w.planeten[pid].koord) && w.planeten[pid].gebaeude[Gebaeude::Xenoextraktor.idx()]>0 {
            let starved=stock[Gut::Elektronik.idx()]<5000*M || stock[Gut::Legierung.idx()]<10000*M || stock[Gut::Xenokristall.idx()]>=6000*M;
            let surplus=stock[Gut::Elektronik.idx()]>=10000*M && stock[Gut::Legierung.idx()]>=20000*M && stock[Gut::Xenokristall.idx()]<5000*M;
            let target=if starved{0}else if surplus{100}else{levels[Gebaeude::Xenoextraktor.idx()]};
            if target!=levels[Gebaeude::Xenoextraktor.idx()] {tu(w,sid,json!({"typ":"produktion_setzen","planet":w.planeten[pid].koord,"gebaeude":"xenoextraktor","prozent":target}));}
        }
        let mut mines=[(Gebaeude::Erzmine,Gut::Erz),(Gebaeude::Kristallmine,Gut::Kristall),(Gebaeude::Deuteriumsynthesizer,Gut::Deuterium)];
        let stock=w.bestand_jetzt(pid);let limits=w.lagergrenze(pid);
        mines.sort_by_key(|(_,g)|std::cmp::Reverse(stock[g.idx()]*1000/limits[g.idx()].max(1)));
        let mut reduced=false;
        for _ in 0..30 {
            let e=w.industrie_berechnen(&w.planeten[pid]);
            if e.energie_deckung_promille==1000&&e.arbeit_bedarf<=e.arbeit_verfuegbar {break;}
            let levels=w.produktionsstufen(pid);
            let Some((g,_))=mines.iter().find(|(g,good)|w.planeten[pid].gebaeude[g.idx()]>0&&levels[g.idx()]>=10&&stock[good.idx()]>1000*M) else{break;};
            if !tu(w,sid,json!({"typ":"produktion_setzen","planet":w.planeten[pid].koord,"gebaeude":g.name(),"prozent":levels[g.idx()]-10})){break;}
            reduced=true;
        }
        if reduced {continue;}
        // Gedrosselte Mine eines knappen Guts (unter 10 % des Lagers) bekommt 10 Prozentpunkte von einer Mine,
        // deren Gut zur Hälfte gefüllt ist: sonst bleibt z. B. Erz bei 10 % hängen, weil der Strom für die
        // Erholung fehlt, und ohne Erz entsteht kein Kraftwerk mehr (Veyari-Verklemmung, Messung 17.10.).
        {
            let levels=w.produktionsstufen(pid);let p=&w.planeten[pid];
            let knapp=mines.iter().rev().find(|(g,good)|p.gebaeude[g.idx()]>0&&levels[g.idx()]<100&&stock[good.idx()]<limits[good.idx()]/10).copied();
            let voll=mines.iter().find(|(g,good)|p.gebaeude[g.idx()]>0&&levels[g.idx()]>=20&&stock[good.idx()]>=limits[good.idx()]/2).copied();
            if let (Some((k,_)),Some((v,_)))=(knapp,voll) {
                let k_neu=levels[k.idx()]+10;let v_neu=levels[v.idx()]-10;let koord=p.koord;
                if tu(w,sid,json!({"typ":"produktion_setzen","planet":koord,"gebaeude":v.name(),"prozent":v_neu})) {
                    tu(w,sid,json!({"typ":"produktion_setzen","planet":koord,"gebaeude":k.name(),"prozent":k_neu}));
                }
                continue;
            }
        }
        // Recover one step per turn once generation/population supports it, without oscillation.
        for (g,_) in mines.into_iter().rev() {
            let p=&w.planeten[pid];let mut levels=w.produktionsstufen(pid);
            if p.gebaeude[g.idx()]==0||levels[g.idx()]>=100 {continue;}
            levels[g.idx()]+=10;let sp=&w.spieler[sid as usize];
            let e=kern::planer::wirtschaft::berechnen_geregelt_stufe(&w.regeln,p,&sp.forschung,sp.volk,
                &std::array::from_fn(|i|w.integritaet(pid,Gebaeude::ALLE[i])),w.ausbau.personen.get(&id),&levels,w.xeno_planet(p.koord),w.dyson_wirkung(p),sp.stufe);
            if e.energie_erzeugung>=e.energie_verbrauch+e.energie_verbrauch/20&&e.arbeit_bedarf<=e.arbeit_verfuegbar*95/100 {
                tu(w,sid,json!({"typ":"produktion_setzen","planet":p.koord,"gebaeude":g.name(),"prozent":levels[g.idx()]}));break;
            }
        }
    }
}

fn tu(welt: &mut Welt, sid: SpielerId, aktion: Value) -> bool {
    welt.handeln(sid, Rolle::Alle, &aktion).0
}

fn bezahlbar(bestand: &[i64; GUETER], kosten: &[i64; GUETER]) -> bool {
    (0..GUETER).all(|g| bestand[g] >= kosten[g])
}

/// Local stock, paid orders and probes expected back at this launch planet.
fn sonden_geplant(w: &Welt, sid: SpielerId, pid: usize) -> i64 {
    let p=&w.planeten[pid];
    p.einheiten[Einheit::Spionagesonde.idx()]
        + p.fertigung[0].iter().filter(|a|a.produkt==Produkt::Einheit(Einheit::Spionagesonde)).map(|a|a.rest).sum::<i64>()
        + w.flotten.values().filter(|f|f.besitzer==sid && matches!(f.mission,Mission::Spionage|Mission::SystemErkunden|Mission::FlottenSpionage)
            && w.kolonisation.rueckwege.get(&f.id).map(|r|r.ziel as usize).unwrap_or(f.start as usize)==pid)
            .map(|f|f.schiffe[Einheit::Spionagesonde.idx()]).sum::<i64>()
}

/// Select the fastest affordable round trip; an unfunded order is never submitted.
fn sondenauftrag(w: &Welt, sid: SpielerId, pid: usize, ziel: Koord, mission: &str, anzahl: i64) -> Option<Value> {
    if anzahl<1 || w.planeten[pid].einheiten[Einheit::Spionagesonde.idx()]<anzahl
        || w.flotten.values().filter(|f|f.besitzer==sid).count()>=w.flottenplaetze(sid) {return None;}
    // Letzter freier Platz gehört der Bergung eigener Kampftrümmer, wenn sie wartet.
    if w.flotten.values().filter(|f|f.besitzer==sid).count()+1>=w.flottenplaetze(sid) && abbau_bot::bergung_wartet(w,sid) {return None;}
    let mut ships=[0;SCHIFFE];ships[Einheit::Spionagesonde.idx()]=anzahl;
    let deut=w.bestand_jetzt(pid)[Gut::Deuterium.idx()];
    let tempo=(1..=10).rev().find(|s|w.flugplan(sid,pid,ziel,&ships,s*100).is_ok_and(|p|2*p.treibstoff<=deut))?;
    Some(json!({"typ":"flotte_senden","start":w.planeten[pid].koord,"ziel":ziel,"mission":mission,
        "schiffe":{"spionagesonde":anzahl},"geschwindigkeit":tempo as f64/10.0}))
}

/// Darf hier ein Xenoextraktor stehen? Generation 12 synthetisiert überall, klassisch nur im Nebelsystem.
fn xeno_erlaubt(welt: &Welt, k: Koord) -> bool {
    welt.ausbau.industrie_version > 0 || welt.system(k).is_some_and(|s| s.nebel)
}

/// `ziele` für einen bestimmten Planeten: Der Mutterplanet des Krümmermonds bekommt zuerst Lager 10 – der Kern bezahlt den
/// Weltenkrümmer aus Mond und Mutterplanet, und 5 Mio. Erz / 4 Mio. Kristall passten mit Lager 7–8 (1,4–2,3 Mio.) plus Mondlager
/// (500.000) nie hinein (Probe Tag 121–145: zwei Räuber mit voller Doktrin sparten vergeblich).
fn ziele_planet(welt: &Welt, sid: SpielerId, pid: usize, typ: Bottyp) -> Vec<(Gebaeude, u8)> {
    let (sp, p) = (&welt.spieler[sid as usize], &welt.planeten[pid]);
    let nebel = if welt.ausbau.industrie_version > 0 { welt.xeno_planet(p.koord) } else { welt.system(p.koord).map(|s| s.nebel).unwrap_or(false) };
    let mut v = ziele(&welt.regeln, sp.stufe, typ, p.heimat, nebel, xeno_erlaubt(welt, p.koord));
    if let Some(mutter) = monde_bot::kruemmer_mutter(welt, sid, typ) {
        if mutter == pid { v.insert(0, (Gebaeude::Lager, 10)); }
        // Die 20 Antriebskerne des Weltenkrümmers fertigt nur eine Orbitalwerft (Werft 4). Hat der Mutterplanet keine, baut die
        // Heimat sie (bot_monde.rs `kruemmer_fertigen` bestellt dort): frische Welt 712, r54/r56, Tag 120–145 – Krümmerkosten
        // vollständig auf dem Mond, 2.430-mal „Antriebskerne brauchen eine Orbitalwerft“, weder Kolonie noch Heimat hatten eine.
        let werk = if welt.planeten[mutter].gebaeude[Gebaeude::Orbitalwerft.idx()] >= 1 { mutter } else { sp.heimat as usize };
        if werk == pid { v.insert(0, (Gebaeude::Orbitalwerft, 1)); v.insert(0, (Gebaeude::Werft, 4)); }
    }
    v
}

/// Mindeststufen, die ein Bot auf seiner Heimatwelt oder Kolonie anstrebt, in dieser Reihenfolge.
fn ziele(r: &Regelwerk, stufe: u8, typ: Bottyp, heimat: bool, nebel: bool, xeno_erlaubt: bool) -> Vec<(Gebaeude, u8)> {
    use Gebaeude::*;
    if !heimat {
        // Every colony gets a complete local supply/industry chain before specialization.
        // Energy, food, housing and workforce shortages are handled before these targets.
        let mut v = vec![(Bauhof,1),(Erzmine,4),(Kristallmine,4),(Raumhafen,1),(Werft,1),
            (Deuteriumsynthesizer,3),(Lager,2),(Akademie,2),(Giesserei,2),
            (Elektronikwerk,2),(Konsumgueterwerk,2)];
        if nebel {v.extend([(Xenoextraktor,1),(Lager,3),(Xenoextraktor,2)]);}
        match typ {
            Bottyp::Oekonom|Bottyp::Wehrlos => v.extend([(Giesserei,4),(Elektronikwerk,4),(Konsumgueterwerk,4),(Lager,4)]),
            Bottyp::Haendler => v.extend([(Markt,2),(Raumhafen,3),(Lager,4),(Werft,4),(Giesserei,3),(Elektronikwerk,3)]),
            Bottyp::Igel => v.extend([(Bunker,3),(Kaserne,3),(Werft,3),(Sensorphalanx,2),(Lager,3)]),
            Bottyp::Raeuber => v.extend([(Werft,4),(Raumhafen,3),(Kaserne,3),(Deuteriumsynthesizer,8),(Giesserei,4)]),
        }
        v.extend([(Bauhof,2),(Erzmine,10),(Kristallmine,9),(Deuteriumsynthesizer,6),(Lager,5)]);
        if nebel {v.extend([(Akademie,3),(Xenoextraktor,4)]);}
        // Forschung zählt reichsweit: Labore auf Kolonien (Forscher bildet `industrie_personal` aus).
        if stufe>=3 {v.push((Labor, if stufe>=4 {5} else {3}));}
        if stufe>=3 && typ!=Bottyp::Wehrlos {v.push((Kaserne,if stufe>=4{3}else{1}));}
        if stufe>=5 {v.push((Versorgungsnetz,1));}
        return v;
    }
    let mut v = vec![(Erzmine, 5), (Kristallmine, 5), (Bauhof, 1), (Labor, 1), (Deuteriumsynthesizer, 2), (Erzmine, 8), (Kristallmine, 7), (Bauhof, 2), (Deuteriumsynthesizer, 4)];
    if stufe >= 2 {
        v.extend([(Akademie, 1), (Giesserei, 1), (Konsumgueterwerk, 2), (Raumhafen, 1), (Werft, 1), (Elektronikwerk, 1), (Labor, 2), (Giesserei, 3), (Elektronikwerk, 2), (Akademie, 2), (Labor, 3), (Bauhof, 3), (Deuteriumsynthesizer, 6)]);
        if r.gebaeude.contains_key(&Geheimdienst) {v.extend([(Geheimdienst,1)]);}
        if typ == Bottyp::Igel {
            v.push((Bunker, 3));
        }
    }
    if stufe >= 3 {
        v.extend([(Raumhafen, 1), (Kaserne,1), (Werft, 2), (Labor, 7), (Akademie, 3), (Werft, 4), (Markt, 1), (Giesserei, 5), (Elektronikwerk, 4), (Bauhof, 4), (Lager, 3)]);
        match typ {
            Bottyp::Haendler => v.push((Markt, 2)),
            Bottyp::Igel => v.extend([(Sensorphalanx, 2), (Bunker, 5)]),
            Bottyp::Raeuber => v.push((Raumhafen, 3)),
            Bottyp::Oekonom | Bottyp::Wehrlos => {}
        }
    }
    if stufe >= 4 {
        // Lager 7: die Kosten von Stufe V (200.000 Erz, 5.000 Xenokristall) müssen ins Lager passen.
        // Xenoextraktor nur, wo der Kern ihn erlaubt: klassisch scheiterte er 1.173-mal je Epoche an Heimatwelten ohne
        // Nebel, und die Heimat baute danach nichts mehr (Spur Igel 20.10.2026: „Ziel xenoextraktor“ ab Tag 75).
        v.push((Orbitalwerft, 1));
        if xeno_erlaubt { v.push((Xenoextraktor, 1)); }
        v.extend([(Kaserne,3), (Lager, 7), (Verwaltungszentrum, 2), (Labor, 10), (Akademie, 4), (Giesserei, 7), (Elektronikwerk, 6), (Verwaltungszentrum, 6), (Lager, 6)]);
    }
    if stufe >= 5 {
        // Großprojekte: der Orbitalring bringt 50 Felder, die anderen beiden Strom und Forschung.
        v.extend([(Orbitalring, 1), (Kaserne,6), (Versorgungsnetz, 1), (Forschungsarchiv, 1)]);
        // Weltenkrümmer (Räuber, Igel): 100.000 Legierung und 50.000 Elektronik plus das Werftdock (damals Stufe 10) – mit Gießerei 7 und
        // Elektronikwerk 6 kam keiner hin (bot_lage 712, Tag 61: bester Krümmermond Werftdock 9, Heimat 6.924 Legierung, 896 Elektronik).
        if matches!(typ, Bottyp::Raeuber | Bottyp::Igel) { v.extend([(Giesserei, 10), (Elektronikwerk, 8)]); }
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

/// Gebäudestufen, die der nächsten Zivilisationsstufe auf diesem Planeten noch fehlen (Voraussetzungen zuerst).
/// Leer, wenn ein anderer eigener Planet die Bedingung schon erfüllt.
pub fn stufengebaeude_fehlend(welt: &Welt, sid: SpielerId, pid: usize) -> Vec<(Gebaeude, u8)> {
    let r = &welt.regeln;
    let sp = &welt.spieler[sid as usize];
    let p = &welt.planeten[pid];
    let Some(naechste) = r.stufen.get(sp.stufe as usize - 1) else { return Vec::new() };
    let lvl = |g: Gebaeude| p.gebaeude[g.idx()] + p.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;
    let mut v: Vec<(Gebaeude, u8)> = Vec::new();
    for (g, n) in &naechste.gebaeude {
        if sp.planeten.iter().any(|q| welt.planeten[*q as usize].gebaeude[g.idx()] >= *n) || lvl(*g) >= *n {
            continue;
        }
        for (b, st) in &r.geb(*g).braucht {
            if lvl(*b) < *st && !v.iter().any(|(x, _)| x == b) {
                v.push((*b, st - lvl(*b)));
            }
        }
        v.push((*g, n - lvl(*g)));
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

/// Fehlt der Heimatwelt die Konsumgüterdeckung, die die nächste Stufe verlangt?
fn konsum_fuer_stufe_fehlt(welt: &Welt, sid: SpielerId, pid: usize) -> bool {
    let p = &welt.planeten[pid];
    p.heimat && welt.regeln.stufen.get(welt.spieler[sid as usize].stufe as usize - 1)
        .is_some_and(|s| s.konsum_deckung > 0.0 && p.konsum_deckung < milli(s.konsum_deckung))
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
    let frei_bestand = {
        let reserve = aufstiegs_ruecklage(welt, sid, pid);
        std::array::from_fn::<i64, GUETER, _>(|i| (bestand[i] - reserve[i]).max(0))
    };
    let kann = |g: Gebaeude| bezahlbar(&frei_bestand, &r.kosten_gebaeude(g, lvl(g) + 1));
    // Reserve research access before extraction/housing consumes the last building sites.
    let terra_labor=r.forsch(Forschung::Terraforming).labor;
    let freie_felder=welt.felder_gesamt(pid).saturating_sub(welt.felder_belegt(pid));
    // Nur wo die Forschung Terraforming Felder bringt (laufende Welten); neue Welten (0.4.7) formen Felder mit Terraformern.
    if r.wirtschaft.terraforming_felder>0 && p.heimat && sp.stufe>=3 && freie_felder<=12 && lvl(Labor)<terra_labor {
        return (freie_felder>0 && kann(Labor)).then_some(Labor);
    }
    let ohne_nahrung = r.volk(sp.volk).ohne_nahrung;
    // Ab Stufe IV bleibt auf der Heimatwelt ein Feld frei: der Orbitalring der Stufe V belegt selbst eines
    // und bringt 50 neue.
    let ein_feld = welt.felder_belegt(pid) + 1 >= welt.felder_gesamt(pid);
    if p.heimat && ein_feld && frei(Orbitalring) && lvl(Orbitalring) == 0 && welt.felder_belegt(pid) < welt.felder_gesamt(pid) {
        return kann(Orbitalring).then_some(Orbitalring);
    }
    // Nebelwelten sind klein: vor Stufe IV bleiben vier Felder für den Xenoextraktor frei (20.10.2026: alle Nebelkolonien
    // waren mit Wohnblöcken und Minen voll, als der Extraktor freigeschaltet wurde).
    let xeno_welt = welt.ausbau.industrie_version > 0 && welt.xeno_planet(p.koord);
    let reserve = if p.heimat && sp.stufe >= 4 && lvl(Orbitalring) == 0 { 1 }
        else if xeno_welt && lvl(Xenoextraktor) == 0 && !frei(Xenoextraktor) { 4 } else { 0 };
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

    // Felder für die Gebäude der nächsten Stufe (samt Voraussetzungen wie Raumhafen für die Werft) freihalten:
    // sonst steht die Heimatwelt voll mit Minen da und kann die Werft der Stufe IV nie bauen (Messung 17.10.).
    if p.heimat {
        let fehlend = stufengebaeude_fehlend(welt, sid, pid);
        let noetig: u16 = fehlend.iter().map(|(_, n)| *n as u16).sum();
        let frei_felder = welt.felder_gesamt(pid).saturating_sub(welt.felder_belegt(pid) + reserve);
        if noetig > 0 && frei_felder <= noetig {
            for (g, _) in &fehlend {
                if frei(*g) && r.geb(*g).braucht.iter().all(|(b, st)| p.gebaeude[b.idx()] >= *st) {
                    return oder_lager(*g);
                }
            }
            return None;
        }
    }
    // Strom mit Puffer für den nächsten Verbraucher: das Kraftwerk, dessen nächste Stufe je zusätzlicher Einheit
    // am wenigsten kostet. Fusion nur, wenn die Deuteriumförderung ihren Mehrverbrauch trägt.
    if p.energie_erzeugung < p.energie_verbrauch + p.energie_verbrauch / 10 + 40 * M && in_q(Solarkraftwerk) + in_q(Fusionskraftwerk) == 0 {
        let mehr = |g: Gebaeude, grund: f64| r.stufenwert(grund, lvl(g) + 1) - r.stufenwert(grund, lvl(g));
        let solar = anteil(mehr(Solarkraftwerk, r.geb(Solarkraftwerk).ertrag), p.faktor[F_SOLAR], 1000);
        let fusion = mehr(Fusionskraftwerk, r.geb(Fusionskraftwerk).ertrag);
        let deuterium = mehr(Fusionskraftwerk, r.wirtschaft.fusion_deuterium);
        let je_einheit = |g: Gebaeude, e: i64| r.wert(&r.kosten_gebaeude(g, lvl(g) + 1)) / e.max(1);
        let fusion_ok = frei(Fusionskraftwerk) && p.rate[Gut::Deuterium.idx()] > deuterium + 20 * M;
        // Ist das günstigere Fusionskraftwerk nicht bezahlbar, das Solarkraftwerk: sonst baute die Heimat gar nichts mehr
        // (Spur Igel 6, klassisch 20.10.2026: von Tag 60 bis 365 „Strom knapp“, „Bau -“, nie ein Markt für Xenokristall).
        if fusion_ok && je_einheit(Fusionskraftwerk, fusion) < je_einheit(Solarkraftwerk, solar) {
            if let Some(b) = oder_lager(Fusionskraftwerk) {
                return Some(b);
            }
        }
        return oder_lager(Solarkraftwerk);
    }
    if !ohne_nahrung {
        let bedarf = mal(p.bevoelkerung, r.wirtschaft.nahrung_je_1000) / 1000;
        if p.rate[Gut::Nahrung.idx()] < bedarf / 4 && in_q(Farm) == 0 {
            // Wie beim Strom: passt die Farm nicht ins Lager, zuerst das Lager (Spur 23.10.2026: Krath mit 181.000 Einwohnern
            // 50 Tage „Bau -“ – Farm 20 kostete mehr als 20.480 Erz).
            return oder_lager(Farm);
        }
    }
    // Natürlicher Xenokristall vor Wohnraum und Konsum: fördert ohne Einsatzstoffe und trägt Hyperraumantrieb und Stufe V.
    if xeno_welt && frei(Xenoextraktor) && lvl(Xenoextraktor) < 4 && in_q(Xenoextraktor) == 0 {
        if let Some(b) = oder_lager(Xenoextraktor) {
            return Some(b);
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
    // Verlangt die nächste Stufe Konsumgüterdeckung und fehlt sie, dürfen zwei Werkstufen nacheinander in der Schleife
    // stehen: eine Stufe nach der anderen hielt mit der wachsenden Bevölkerung nicht Schritt (Startwert 2026: ein Händler
    // blieb mit 72 % auf Stufe III).
    let kw_schleife = if konsum_fuer_stufe_fehlt(welt, sid, pid) { 2 } else { 1 };
    // Wie bei Nahrung: schon bauen, wenn der Überschuss unter ein Viertel des Bedarfs fällt. Die Deckung zeigt höchstens
    // 100 %, und wer erst bei Mangel baut, fällt mit wachsender Bevölkerung genau beim Aufstieg unter die 80 % der
    // Stufe IV (Spur Startwert 2026: 100 % bis Stunde 690, dann 78 % – das Werk wartete hinter einem Wohnblock).
    let konsum_bedarf = mal(p.bevoelkerung, r.wirtschaft.konsum_je_1000) / 1000;
    let konsum_knapp = p.konsum_deckung < 1000 || p.rate[Gut::Konsumgut.idx()] < konsum_bedarf / 4;
    let kw = frei(Konsumgueterwerk) && konsum_knapp && in_q(Konsumgueterwerk) < kw_schleife && lvl(Konsumgueterwerk) < kw_grenze && kann(Konsumgueterwerk);
    if kw {
        return Some(Konsumgueterwerk);
    }
    // Bauhof vor langen Bauten: die Bauzeit teilt sich durch 1 + Bauhofstufe, eine Stufe kostet einen Bruchteil eines
    // großen Wohnblocks (mit Bauhof 4 baute Wohnblock 26 volle 56 Spielstunden und hielt jeden anderen Bau auf).
    let wb_kosten = r.kosten_gebaeude(Wohnblock, lvl(Wohnblock) + 1);
    if frei(Bauhof) && in_q(Bauhof) == 0 && lvl(Bauhof) < 20 && welt.bauzeit(pid, &wb_kosten) > 12 * STUNDE
        && r.wert(&r.kosten_gebaeude(Bauhof, lvl(Bauhof) + 1)) * 20 < r.wert(&wb_kosten) && kann(Bauhof) {
        return Some(Bauhof);
    }
    // Wachstum ist logistisch (Rate × freier Wohnraumanteil): ab 40 % Belegung der nächste Wohnblock. Mit der alten
    // Schwelle 55 % wuchsen die Bots mit höchstens 45 % der Rate, und ab Tag 25 stand jede Heimatwelt mit 3–10 %
    // freiem Wohnraum still (bot_lage 20.10.2026: Stufe IV blieb bei 60–107 von 150 Tausend Einwohnern hängen).
    // Ab Stufe III kostet jeder Einwohner Konsumgüter: Wohnraum nur, solange die Konsumgüter mithalten (Karl 23.10.2026:
    // Wohnblock fasst 8× so viele Einwohner, „damit man regulieren kann“ – ohne diese Bremse wuchsen die Ökonomen auf
    // 600.000 Einwohner mit 44 % Konsumgütern und kamen nie auf Stufe IV).
    let _ = konsum_knapp;
    // Vor jedem Wohnblock: tragen Nahrung (Syntheten: Strom) und ab Stufe III die Konsumgüter auch die Einwohner, die er
    // aufnimmt? Sonst zuerst das Werk, das fehlt (Karl 23.10.2026: ein Wohnblock fasst 8× so viele Einwohner, „damit man
    // regulieren kann“ – ohne diese Prüfung wuchsen die Ökonomen auf 600.000 Einwohner mit 44 % Konsumgütern, die
    // Syntheten fraßen ihr Stromnetz leer, und keiner kam auf Stufe IV).
    let versorgung_fehlt = || -> Option<Gebaeude> {
        let mehr = r.stufenwert(r.geb(Wohnblock).ertrag, lvl(Wohnblock) + 1) - r.stufenwert(r.geb(Wohnblock).ertrag, lvl(Wohnblock));
        let ziel_bev = p.wohnraum + mehr;
        if ohne_nahrung {
            let je = r.wirtschaft.energie_je_1000_syntheten;
            if p.energie_erzeugung - p.energie_verbrauch + mal(p.bevoelkerung, je) / 1000 < mal(ziel_bev, je) / 1000 {
                return Some(Solarkraftwerk);
            }
        } else {
            let je = r.wirtschaft.nahrung_je_1000;
            if p.rate[Gut::Nahrung.idx()] + mal(p.bevoelkerung, je) / 1000 < mal(ziel_bev, je) / 1000 {
                return Some(Farm);
            }
        }
        if sp.stufe >= 3 && frei(Konsumgueterwerk) && lvl(Konsumgueterwerk) < kw_grenze {
            let je = r.wirtschaft.konsum_je_1000;
            if (p.rate[Gut::Konsumgut.idx()] + konsum_bedarf) * 10 < mal(ziel_bev, je) / 1000 * 8 {
                return Some(Konsumgueterwerk);
            }
        }
        None
    };
    if p.bevoelkerung > mal(p.wohnraum, WOHNEN_AB) && in_q(Wohnblock) == 0 && !module {
        match versorgung_fehlt() {
            None => return oder_lager(Wohnblock),
            Some(g) if in_q(g) == 0 => if let Some(b) = oder_lager(g) { return Some(b); },
            Some(_) => {}
        }
    }
    // Volle Lager nur erweitern, solange Felder übrig sind: die letzten Felder gehören Wohnraum und Stufengebäuden
    // (am Tag 30 lagen 11 Lagerstufen auf Heimatwelten, deren Erz niemand brauchte).
    let eng = [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Legierung, Gut::Elektronik, Gut::Xenokristall]
        .iter()
        .any(|g| bestand[g.idx()] >= mal(grenze[g.idx()], 0.85));
    // Aus dem echten Bestand: ein volles Lager ist genau das, was das Sparen auf die Rücklage verhindert.
    if eng && in_q(Lager) == 0 && bezahlbar(&bestand, &r.kosten_gebaeude(Lager, lvl(Lager) + 1)) && freie_felder > FELDER_FUER_WOHNRAUM {
        return Some(Lager);
    }
    let arbeit_knapp = p.arbeit_bedarf > mal(p.arbeit_verfuegbar, 0.93);

    // Generation 12: Fehlt für den nächsten Aufstieg nur noch Xenokristall, die Synthese auf jedem Planeten mit
    // freien Feldern bis Stufe 4 ausbauen; die Logistik bringt ihn zur Heimat (sonst wartet ein Reich ohne
    // Nebelkolonie und mit voller Heimatwelt wochenlang, Messung 17.10.: Krath).
    if welt.ausbau.industrie_version>0 && frei(Xenoextraktor) && lvl(Xenoextraktor) < 4 && in_q(Xenoextraktor) == 0 {
        if let Some(regel) = r.stufen.get(sp.stufe as usize - 1) {
            let xi = Gut::Xenokristall.idx();
            // Auch die verlangte Forschung kann Xenokristall kosten (Hyperraumantrieb 1 für Stufe V): ohne diesen Fall
            // warteten Reiche ohne Nebelkolonie auf erfüllte Bedingungen, die erst der Xenokristall erfüllt (Messung 18.10.).
            let forschung_xeno: i64 = regel.forschung.iter().filter(|(f, n)| sp.forschung[f.idx()] < **n)
                .map(|(f, _)| r.kosten_forschung(*f, sp.forschung[f.idx()] + 1)[xi]).sum();
            let xeno = preis_array(&regel.kosten)[xi] + forschung_xeno;
            // 20.10.2026: gleich mit der Freischaltung fördern (vorher erst ab der halben Einwohnerschwelle – mit dem
            // schnelleren Stufentempo fehlten am Tag 30 die 6.000 Xenokristall für Hyperraumantrieb und Stufe V).
            // Natürliche Vorkommen immer, Synthese nur, solange dem Reich insgesamt Xenokristall fehlt.
            let im_reich: i64 = sp.planeten.iter().map(|q| welt.bestand_jetzt(*q as usize)[xi]).sum();
            let bereit = welt.xeno_planet(p.koord) || im_reich < xeno;
            if xeno > 0 && bereit && bestand[xi] < xeno {
                if let Some(b) = oder_lager(Xenoextraktor) {
                    return Some(b);
                }
            }
        }
    }
    let mut ziel_wert = 0i64;
    let mut ziel_fehlt = [false; GUETER];
    for (g, min) in ziele_planet(welt, sid, pid, typ) {
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
        let k = r.kosten_gebaeude(g, lvl(g) + 1);
        ziel_wert = r.wert(&k);
        for i in 0..GUETER { ziel_fehlt[i] = bestand[i] < k[i]; }
        break;
    }
    if arbeit_knapp {
        return None;
    }
    // Ebenso für die Forschung, die die nächste Stufe verlangt (Spur 23.10.2026: Krath ohne Deuterium für Impulsantrieb 2,
    // 40 Tage Stufe III bei vollem Erz- und Kristalllager).
    if let Some(regel) = r.stufen.get(sp.stufe as usize - 1) {
        for (f, n) in &regel.forschung {
            if sp.forschung[f.idx()] < *n && sp.forschung[f.idx()] < MAX_FORSCHUNG {
                let k = r.kosten_forschung(*f, sp.forschung[f.idx()] + 1);
                for i in 0..GUETER { ziel_fehlt[i] |= bestand[i] < k[i]; }
            }
        }
    }
    // Mine mit der kürzesten Amortisation – nicht für ein Gut, dessen Lager schon zu drei Vierteln voll ist (die
    // Förderung liefe ins Leere), und nicht auf den letzten Feldern, die Wohnraum brauchen.
    // Fehlt dem Ziel genau dieses Gut, zuerst seine Mine – auch wenn sie mehr als ein Viertel des Ziels kostet (Spur
    // 23.10.2026: 40 Tage „Labor 7 fehlt Deuterium“ bei Deuteriumsynthesizer 7, Werft 4 dahinter kam nie dran).
    let mut beste: Option<((bool, i64), Gebaeude)> = None;
    for (g, gut, f) in [(Erzmine, Gut::Erz, F_ERZ), (Kristallmine, Gut::Kristall, F_KRISTALL), (Deuteriumsynthesizer, Gut::Deuterium, F_DEUTERIUM)] {
        let n = lvl(g);
        if n >= r.welt.max_gebaeudestufe || !kann(g) || bestand[gut.idx()] >= mal(grenze[gut.idx()], 0.75)
            || freie_felder <= FELDER_FUER_WOHNRAUM {
            continue;
        }
        let fuer_ziel = ziel_fehlt[gut.idx()];
        let kosten = r.wert(&r.kosten_gebaeude(g, n + 1));
        if ziel_wert > 0 && kosten > ziel_wert / 4 && !fuer_ziel {
            continue;
        }
        let mehr = anteil(r.stufenwert(r.geb(g).ertrag, n + 1) - r.stufenwert(r.geb(g).ertrag, n), p.faktor[f], 1000);
        let mut k = [0i64; GUETER];
        k[gut.idx()] = mehr.max(1);
        let rang = (!fuer_ziel, kosten / r.wert(&k).max(1));
        if beste.map(|(a, _)| rang < a).unwrap_or(true) {
            beste = Some((rang, g));
        }
    }
    // Sonst Wohnraum auf Vorrat: jeder freie Anteil beschleunigt das Wachstum, statt dass Rohstoffe im vollen
    // Lager verfallen.
    beste.map(|(_, g)| g).or_else(|| {
        (p.bevoelkerung > mal(p.wohnraum, WOHNEN_VORRAT_AB) && in_q(Wohnblock) == 0 && !module && versorgung_fehlt().is_none() && kann(Wohnblock)).then_some(Wohnblock)
    })
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
    for (g, min) in ziele_planet(welt, sid, pid, typ) {
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
    // Dysonsphäre bis auf die Einzahlung bereit: der offene Anteil bleibt für den Depot-Nachschub liegen (bot_dyson.rs).
    if p.heimat {
        let d = dyson_bot::sparziel(welt, sid);
        for i in 0..GUETER { reserve[i] = reserve[i].max(d[i]); }
    }
    // Die nächste Pflichtforschung des Aufstiegs (Forschung der Stufe, Astrophysik für ihre Kolonien) ab Stufe IV:
    // 20.10.2026 Balance-Abnahme klassisch: 14 von 24 Igeln fehlte am Epochenende die dritte Kolonie – ihre
    // Verteidigung (die Hälfte des Gebäudewerts) verbrauchte jeden Bestand, auf den die Forschung sparte.
    if p.heimat && sp.stufe >= 4 && sp.forschung_aktiv.is_none() {
        if let Some(f) = pflichtforschung(welt, sid) {
            let k = r.kosten_forschung(f, sp.forschung[f.idx()] + 1);
            for i in 0..GUETER {
                reserve[i] += k[i];
            }
        }
    }
    let k = kruemmer_spar(welt, sid, pid);
    for i in 0..GUETER { reserve[i] += k[i]; }
    reserve
}

/// Rücklage für Mondlieferungen (Transport und Fähre): wie `aufstiegs_ruecklage`, aber ohne den Dyson-Anteil – Mond und
/// Weltenkrümmer verhungerten sonst hinter der Sphäre (frische Welt 712, Tag 160: die Monde der Igel in Projektallianzen ohne
/// ein einziges Gebäude).
fn mond_ruecklage(welt: &Welt, sid: SpielerId, pid: usize) -> [i64; GUETER] {
    if !welt.planeten[pid].heimat { return [0; GUETER]; }
    if spart(welt, sid) { return aufstiegs_ruecklage(welt, sid, pid); }
    let mut v = kruemmer_ruecklage(welt, sid, pid);
    // Sind alle Bedingungen erfüllt, bleiben die Stufenkosten auch nach den 20 Spartagen daheim: Mondlieferung und Fähre
    // leerten sonst die Heimat (r58, Tag 60: Fähre Kristall 1.200 und Deuterium 600 je Stunde, daheim 949 Kristall).
    if aufstieg_bereit(welt, sid) {
        if let Some(regel) = welt.regeln.stufen.get(welt.spieler[sid as usize].stufe as usize - 1) {
            let k = preis_array(&regel.kosten);
            for i in 0..GUETER { v[i] += k[i]; }
        }
    }
    v
}

/// Alle Bedingungen der nächsten Stufe erfüllt (es fehlen höchstens die Kosten).
fn aufstieg_bereit(welt: &Welt, sid: SpielerId) -> bool {
    let b = welt.stufen_bedingungen(sid);
    !b.is_empty() && b.iter().all(|(_, ok)| *ok)
}

/// Weltenkrümmer: die nächste Doktrinstufe auf der Heimat mit der Deckung, die bot_monde.rs zum Einreihen verlangt (60 %, also
/// 5/3 der Kosten), solange sie sich beschaffen lässt. Gilt für Gebäude, Schiffe und Mondlieferungen: vorher verbaute oder
/// verflog ein Räuber jeden Überschuss, und die Mondfähre leerte die Heimat (Fortsetzung bot_lage 712, Tag 80: Mond 500.000
/// Erz, Heimat 3.509 Kristall, Hyperraumantrieb 4 blieb „unbezahlbar“).
fn kruemmer_ruecklage(welt: &Welt, sid: SpielerId, pid: usize) -> [i64; GUETER] {
    let mut v = [0i64; GUETER];
    if !welt.planeten[pid].heimat { return v; }
    let Some((f, st)) = monde_bot::kruemmer_forschung(welt, sid) else { return v };
    let k = welt.regeln.kosten_forschung(f, st);
    if beschaffbar(welt, sid, &welt.bestand_jetzt(pid), &k) { for i in 0..GUETER { v[i] = k[i] * 5 / 3; } }
    v
}

/// Krümmer-Rücklage für Gebäude, Schiffe und Forschung: die Doktrin (`kruemmer_ruecklage`) und danach die nächste Stufe des
/// Krümmermonds (`kruemmer_schritt_fehlt`). Mondlieferungen nehmen nur `kruemmer_ruecklage` – sie bringen den Schritt ja hin.
fn kruemmer_spar(welt: &Welt, sid: SpielerId, pid: usize) -> [i64; GUETER] {
    let mut v = kruemmer_ruecklage(welt, sid, pid);
    if welt.planeten[pid].heimat {
        let s = monde_bot::kruemmer_schritt_fehlt(welt, sid);
        for i in 0..GUETER { v[i] += s[i]; }
    }
    v
}

/// Die nächste noch fehlende Forschung, die der Aufstieg verlangt: erst die Forschung der Stufe, dann Astrophysik für
/// die verlangten Kolonien (Ausbau-Welten: je Stufe eine Kolonie, ältere Welten: je zwei Stufen eine).
///
/// Nur, was sich jetzt erforschen (Stufe, Labor) und beschaffen lässt: sonst hielt die Rücklage die Kosten des
/// Hyperraumantriebs fest, für den einem Reich ohne Nebelwelt der Xenokristall fehlte – die Heimat baute nichts mehr
/// und forschte nichts (Spur Igel 10, klassisch 20.10.2026: von Tag 60 bis 120 „Forschung None“, „Bau -“).
fn pflichtforschung(welt: &Welt, sid: SpielerId) -> Option<Forschung> {
    let sp = &welt.spieler[sid as usize];
    let regel = welt.regeln.stufen.get(sp.stufe as usize - 1)?;
    let mut plan: Vec<(Forschung, u8)> = regel.forschung.iter().map(|(f, n)| (*f, *n)).collect();
    if regel.kolonien > 0 {
        // Kolonieforschung der Welt (laufende Welten Astrophysik, neue Welten ab 0.4.7 Terraforming nach den freien Plätzen).
        plan.push((welt.kolonie_forschung(), welt.kolonie_stufe_roh(regel.kolonien)));
    }
    let heim = welt.bestand_jetzt(sp.heimat as usize);
    let labor = welt.planeten[sp.heimat as usize].gebaeude[Gebaeude::Labor.idx()];
    plan.into_iter().find(|(f, n)| {
        let Some(fr) = welt.regeln.forschung.get(f) else { return false };
        sp.forschung[f.idx()] < *n && fr.ab_stufe <= sp.stufe && labor >= fr.labor
            && beschaffbar(welt, sid, &heim, &welt.regeln.kosten_forschung(*f, sp.forschung[f.idx()] + 1))
    }).map(|(f, _)| f)
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
    // Unter Bombardierung oder Belagerung repariert der Kern nicht (bot_lage 20.10.2026: seit Bombardierungen wieder
    // vorkommen, die häufigste Ablehnung); dann wird normal gebaut.
    if welt.invasion_v1() && (welt.ausbau.boden.invasion.bombardements.contains_key(&(pid as PlanetId))
        || welt.kolonisation.besetzungen.contains_key(&(pid as PlanetId))) {
        return None;
    }
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
    // Recover a full bot world: one surplus mine level gives way to what the world needs and cannot place –
    // laboratory access, consumer goods for the next stage, food or power on the home world, and on every world the
    // next housing level (or the storage it no longer fits into). Never demolish life support, industry, research
    // or an occupied construction site.
    // 20.10.2026: a mine counts as surplus with a half-full store even at rate 0 (a full store stops the rate, and
    // two economists stayed on stage III with 32 % consumer goods); housing pressure is new (day 30 before: 65 mine
    // levels with full stores, every home world out of housing, growth stopped).
    // The freed field goes straight to that building in the same turn; otherwise a power plant took it (Syntheten
    // need power for their population) and the consumer goods never got their field.
    // A free queue slot is enough: waiting for an empty queue let consumer goods fall behind a growing population.
    let p=&welt.planeten[pid];
    // Das Feld, das die Heimat ab Stufe IV für den Orbitalring frei hält, zählt als belegt (sonst hungerte eine Heimat
    // mit 179/180 Feldern: keine Farm passte, und der Tausch sah die Welt nicht als voll).
    let ring_reserve=u16::from(p.heimat && welt.spieler[sid as usize].stufe>=4 && p.gebaeude[Gebaeude::Orbitalring.idx()]==0);
    if p.bauschleife.len()<2 && welt.felder_belegt(pid)+ring_reserve>=welt.felder_gesamt(pid) {
        let r=&welt.regeln;
        let stock=welt.bestand_jetzt(pid);let limit=welt.lagergrenze(pid);
        let naechste=|g:Gebaeude|r.kosten_gebaeude(g,p.gebaeude[g.idx()]+1);
        let passt=|g:Gebaeude|bezahlbar(&stock,&naechste(g));
        let heim=p.heimat && welt.spieler[sid as usize].stufe>=3;
        let ohne_nahrung=r.volk(welt.spieler[sid as usize].volk).ohne_nahrung;
        let ersatz=if heim && r.wirtschaft.terraforming_felder>0 && p.gebaeude[Gebaeude::Labor.idx()]<r.forsch(Forschung::Terraforming).labor {Some(Gebaeude::Labor)}
            else if heim && (p.konsum_deckung<800 || konsum_fuer_stufe_fehlt(welt,sid,pid)) && p.gebaeude[Gebaeude::Konsumgueterwerk.idx()]<20 {Some(Gebaeude::Konsumgueterwerk)}
            else if !ohne_nahrung && (p.rate[Gut::Nahrung.idx()]<0 || p.nahrung_deckung<1000) {Some(Gebaeude::Farm)}
            else if heim && p.energie_erzeugung<p.energie_verbrauch {Some(Gebaeude::Solarkraftwerk)}
            else if welt.ausbau.industrie_version>0 && welt.xeno_planet(p.koord) && welt.spieler[sid as usize].stufe>=r.geb(Gebaeude::Xenoextraktor).ab_stufe
                && p.gebaeude[Gebaeude::Xenoextraktor.idx()]<4 {Some(Gebaeude::Xenoextraktor)}
            else if p.bevoelkerung>mal(p.wohnraum,0.5) && p.gebaeude[Gebaeude::Wohnblock.idx()]>0 {
                let wb=naechste(Gebaeude::Wohnblock);
                let bh=naechste(Gebaeude::Bauhof);
                Some(if (0..GUETER).any(|g|wb[g]>mal(limit[g],0.85)) {Gebaeude::Lager}
                    else if welt.bauzeit(pid,&wb)>12*STUNDE && r.wert(&bh)*20<r.wert(&wb) && p.gebaeude[Gebaeude::Bauhof.idx()]<20 {Gebaeude::Bauhof}
                    else {Gebaeude::Wohnblock})
            } else {None};
        let schon=|g:Gebaeude|p.bauschleife.iter().any(|a|a.gebaeude==g) && !(g==Gebaeude::Konsumgueterwerk && konsum_fuer_stufe_fehlt(welt,sid,pid));
        if let Some(ersatz)=ersatz.filter(|g|passt(*g) && !schon(*g)) {
            let candidate=[(Gebaeude::Erzmine,Gut::Erz),(Gebaeude::Kristallmine,Gut::Kristall),(Gebaeude::Deuteriumsynthesizer,Gut::Deuterium)]
                .into_iter().filter(|(g,res)|p.gebaeude[g.idx()]>5
                    && ((p.rate[res.idx()]>0 && stock[res.idx()]>=10_000*M) || stock[res.idx()]>=limit[res.idx()]/2))
                .max_by_key(|(g,res)|if ersatz==Gebaeude::Labor {p.gebaeude[g.idx()] as i64} else {stock[res.idx()]*1000/limit[res.idx()].max(1)});
            let k=p.koord;
            if let Some((g,_))=candidate {
                if tu(welt,sid,json!({"typ":"abreissen","planet":k,"gebaeude":g.name()})) {
                    zaehle(match ersatz {Gebaeude::Wohnblock|Gebaeude::Lager=>"bau: mine weicht wohnraum",Gebaeude::Konsumgueterwerk=>"bau: mine weicht konsumgüterwerk",_=>"bau: mine weicht labor/versorgung"});
                    tu(welt,sid,json!({"typ":"bauen","planet":k.to_string(),"gebaeude":ersatz.name()}));
                }
            }
        }
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
    if p.gebaeude[Gebaeude::Xenoextraktor.idx()] > 0 && !p.prioritaeten.contains(&Gebaeude::Xenoextraktor) {
        let k = p.koord.to_string();
        tu(welt, sid, json!({"typ": "prioritaeten", "planet": k, "reihenfolge": ["solarkraftwerk", "fusionskraftwerk", "farm", "xenoextraktor"]}));
    }
    for _ in 0..2 {
        if welt.planeten[pid].bauschleife.len() >= 2 {
            return;
        }
        let Some(g) = waehle_bau(welt, sid, pid, typ) else {
            return;
        };
        // Ein beschädigtes Gebäude baut der Kern erst nach der Reparatur aus; unter Bombardierung ist keine möglich – dann
        // wartet der Bot (bot_lage 20.10.2026: sonst rund 850 Ablehnungen je Lauf).
        if welt.planeten[pid].gebaeude[g.idx()] > 0 && welt.integritaet(pid, g) < 1000 {
            zaehle("bau: beschädigt, wartet auf reparatur");
            return;
        }
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
    let bestand = welt.bestand_jetzt(pid);
    let mut ziel = "keins".to_string();
    for (g, min) in ziele_planet(welt, sid, pid, typ) {
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
            // Ein Xenoextraktor fördert in Schüben (gedrosselt, solange Legierung und Elektronik fehlen): trotzdem
            // beschaffbar, sonst belegte Terraforming den Forschungsplatz des Hyperraumantriebs.
            || (g == Gut::Xenokristall.idx() && sp.planeten.iter().any(|p| welt.planeten[*p as usize].gebaeude[Gebaeude::Xenoextraktor.idx()] > 0))
    })
}

/// Ab Stufe III ohne eigene Nebelwelt: das nächste unerfasste System des eigenen Sektors erkunden, bis ein Nebelsystem
/// bekannt ist. Jedes vierte Nicht-Heimatsystem hat natürlichen Xenokristall; ohne ihn reichte die teure Synthese am
/// Tag 30 nicht für Hyperraumantrieb und Stufe V (bot_lage 20.10.2026: 23 von 30 Bots fehlte der Hyperraumantrieb).
fn nebel_erkunden(welt: &mut Welt, sid: SpielerId) {
    let sp = &welt.spieler[sid as usize];
    if sp.stufe < 3 || !welt.aufklaerungsregeln() || welt.ausbau.industrie_version == 0 { return; }
    if sp.planeten.iter().any(|p| welt.xeno_planet(welt.planeten[*p as usize].koord)) { return; }
    let pid = sp.heimat as usize;
    let k = welt.planeten[pid].koord;
    let systeme = welt.regeln.welt.systeme_je_sektor;
    let bekannt = (1..=systeme).any(|n| welt.system_erfasst(sid, Koord::neu(k.sektor, n, 1)) && welt.xeno_planet(Koord::neu(k.sektor, n, 2)));
    if bekannt || welt.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::SystemErkunden) { return; }
    let Some(n) = (1..=systeme).filter(|n| !welt.system_erfasst(sid, Koord::neu(k.sektor, *n, 1)))
        .min_by_key(|n| ((*n as i64 - k.system as i64).abs(), *n)) else { return };
    let ziel = Koord::neu(k.sektor, n, 1);
    if let Some(a) = sondenauftrag(welt, sid, pid, ziel, "system_erkunden", 1) {
        if tu(welt, sid, a) { zaehle("kolonie: nebelsystem suchen"); }
    } else if welt.planeten[pid].einheiten[Einheit::Spionagesonde.idx()] == 0 && sonden_geplant(welt, sid, pid) == 0
        && bezahlbar(&welt.bestand_jetzt(pid), &welt.regeln.kosten_einheit(Einheit::Spionagesonde, sp.volk)) {
        let ks = k.to_string();
        tu(welt, sid, json!({"typ":"fertigen","planet":ks,"einheit":"spionagesonde","anzahl":1}));
    }
}

/// Sind alle Bedingungen der nächsten Stufe erfüllt, spart der Bot für ihre Kosten, höchstens 20 Tage
/// lang und nur, wenn sich jedes Gut der Kosten beschaffen lässt. Sonst stünde die Wirtschaft still.
fn spart(welt: &Welt, sid: SpielerId) -> bool {
    let sp = &welt.spieler[sid as usize];
    let b = welt.stufen_bedingungen(sid);
    if b.is_empty() || !(b.iter().all(|(_, ok)| *ok) || aufstieg_nah(welt, sid)) || sp.stufen_zaehler > 20 * TAG {
        return false;
    }
    let Some(regel) = welt.regeln.stufen.get(sp.stufe as usize - 1) else {
        return false;
    };
    beschaffbar(welt, sid, &welt.bestand_jetzt(sp.heimat as usize), &preis_array(&regel.kosten))
}

/// Ab Stufe IV ist der Aufstieg nah, wenn höchstens noch Forschung läuft, die er verlangt, Kolonieschiffe unterwegs sind
/// oder 15 % der Einwohner fehlen: dann sparen (20.10.2026: Bots mit fertigem Hyperraumantrieb hatten ihr Erz und
/// Kristall gerade in Wohnblöcke gesteckt und brauchten danach Tage für die 200.000 Erz der Stufe V).
fn aufstieg_nah(welt: &Welt, sid: SpielerId) -> bool {
    let sp = &welt.spieler[sid as usize];
    let Some(regel) = welt.regeln.stufen.get(sp.stufe as usize - 1) else { return false };
    if sp.stufe < 4 || welt.einwohner(sid) * 100 < regel.einwohner * M * 85 { return false; }
    let forschung_ok = regel.forschung.iter().all(|(f, n)| sp.forschung[f.idx()] >= *n
        || (sp.forschung[f.idx()] + 1 >= *n && sp.forschung_aktiv.as_ref().is_some_and(|a| a.forschung == *f)));
    let gebaeude_ok = regel.gebaeude.iter().all(|(g, n)| sp.planeten.iter().any(|p| welt.planeten[*p as usize].gebaeude[g.idx()] >= *n));
    let unterwegs = welt.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren).count();
    forschung_ok && gebaeude_ok && welt.kolonien(sid) + unterwegs >= regel.kolonien as usize
}

/// Rücklage für den nahen Aufstieg auf der Heimatwelt (beim Bauen abgezogen, siehe `waehle_bau`).
fn aufstiegs_ruecklage(welt: &Welt, sid: SpielerId, pid: usize) -> [i64; GUETER] {
    let sp = &welt.spieler[sid as usize];
    if !welt.planeten[pid].heimat { return [0; GUETER]; }
    // Stufe V: statt des Aufstiegs spart die Heimat den offenen Dyson-Anteil, sobald das Projekt sonst bereit ist, und die
    // nächste Stufe der Krümmer-Doktrin.
    if !spart(welt, sid) {
        let (d, k) = (dyson_bot::sparziel(welt, sid), kruemmer_spar(welt, sid, pid));
        return std::array::from_fn(|i| d[i] + k[i]);
    }
    // Fehlen noch Einwohner, baut die Heimat weiter Wohnraum: sonst wartete sie auf Einwohner, für die sie keinen
    // Wohnraum mehr bauen darf.
    // Ebenso, solange eine andere Bedingung fehlt (Werft, Forschung, Konsumgüter): mit 0,9 Grundwachstum und 8× Wohnraum
    // steht die Einwohnerschwelle sofort, und die Rücklage von 40.000 Erz über einem Lager von 20.480 blockierte jeden
    // Bau, auch den des Lagers (Spur 23.10.2026: Krath 50 Tage „Bau -“ auf Stufe III).
    let Some(r) = welt.regeln.stufen.get(sp.stufe as usize - 1) else { return [0; GUETER] };
    if welt.einwohner(sid) < r.einwohner * M || welt.stufen_bedingungen(sid).iter().any(|(_, ok)| !ok) { return [0; GUETER]; }
    preis_array(&r.kosten)
}

/// Bezugswert je Einheit in Credits, an dem sich alle Bot-Preise ausrichten (Marktorders, Händler, Handel per Brief):
/// die festen Kurse des Soforthandels (`Welt::soforthandel_angebot`), für die übrigen Güter das Wertungsgewicht.
/// 20.10.2026: vorher handelten Bots zu 5–12 % des Gewichts – billig beim Bot kaufen und über den Soforthandel
/// verkaufen brachte das Zwölffache.
fn marktwert(w: &Welt, g: Gut) -> f64 {
    sofortkurs(w, g).unwrap_or_else(|| w.regeln.wertung.gewichte.get(&g).copied().unwrap_or(1.0))
}

/// Preis auf Tausendstel Credits (die Auflösung des Kerns). Kurze Dezimalzahlen liest serde_json beim Nachspielen
/// bitgleich zurück; `1.5 * 0.99` = 1.4849999999999999 nicht immer – dann wich der Protokollhash ab (20.10.2026).
fn preis3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Fester Kurs des Soforthandels im Kern (2 % Gebühr); `None`: dort nicht tauschbar.
/// Kurse aus den Kernwerten der Welt (regeln/kernwerte.ron; Standard Erz 1, Kristall 1,5, Deuterium 3, Nahrung 0,5).
fn sofortkurs(w: &Welt, g: Gut) -> Option<f64> {
    w.regeln.sofortkurs(g)
}

/// Aktiver Handel aller Bots ab Stufe III mit Markt (Karl 20.10.2026: „Handel aktiv betreiben“): kaufen, was dem Reich
/// für den nächsten Aufstieg und dessen Forschung fehlt, und verkaufen, was die Heimat darüber hinaus hortet. Kauf und
/// Verkauf zum Marktwert – so treffen sich Bots untereinander, und Spieler finden ihre Aufträge im Markt, ohne dass sich
/// ein Umweg über den Soforthandel lohnt. Ohne Wirkung auf Lager, die der Bot selbst braucht.
/// Was der nächste Aufstieg kostet: die Stufenkosten und die nächste Stufe jeder noch fehlenden Stufenforschung
/// (ab Stufe V Xenokristall für Orbitalring und Großprojekte).
fn aufstiegsbedarf(w: &Welt, sid: SpielerId) -> [i64; GUETER] {
    let r = &w.regeln;
    let sp = &w.spieler[sid as usize];
    let mut bedarf = r.stufen.get(sp.stufe as usize - 1).map(|s| preis_array(&s.kosten)).unwrap_or([0; GUETER]);
    if let Some(s) = r.stufen.get(sp.stufe as usize - 1) {
        for (f, n) in &s.forschung {
            if sp.forschung[f.idx()] < *n { let k = r.kosten_forschung(*f, sp.forschung[f.idx()] + 1); for g in 0..GUETER { bedarf[g] += k[g]; } }
        }
    }
    if sp.stufe >= 5 { bedarf[Gut::Xenokristall.idx()] = bedarf[Gut::Xenokristall.idx()].max(5000 * M); }
    bedarf
}

/// Soforthandel (Karl 20.10.2026: „Handel aktiv betreiben“, alle Aktionen selbst ausführen): fehlt der Heimat ein Gut
/// des Soforthandels für den nächsten Aufstieg, tauscht der Bot Überschuss anderer Soforthandelsgüter direkt zu den
/// festen Kursen (2 % Gebühr), statt auf einen Käufer zu warten. Seit die Bots zum Marktwert handeln, kauft kaum einer
/// mehr zu; die Billigpreise vorher verschenkten Ware und verdeckten den Wachstumsvorsprung der Veyari. Nur wenn der Bot
/// auf den Aufstieg spart (`spart`): während des Aufbaus tauschte er sonst Baustoffe weg (Krath im naiven Ablauf 3 Tage
/// später auf Stufe IV). Überschuss heißt: über Bedarf und einem Drittel der Lagergrenze; Nahrung nur bei positiver
/// Rate. Höchstens ein Tausch je Zug.
fn soforthandel_nutzen(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
    let sp = &w.spieler[sid as usize];
    if sp.stufe < 2 || typ == Bottyp::Wehrlos || !spart(w, sid) { return; }
    let pid = sp.heimat as usize;
    let bedarf = aufstiegsbedarf(w, sid);
    let heim = w.bestand_jetzt(pid);
    let grenze = w.lagergrenze(pid);
    let fest = [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Nahrung];
    let kurse: Vec<f64> = Gut::ALLE.iter().map(|g| sofortkurs(w, *g).unwrap_or(1.0)).collect();
    let kurs = |g: Gut| kurse[g.idx()];
    let Some(ziel) = fest.into_iter()
        .filter(|g| bedarf[g.idx()] > heim[g.idx()] && grenze[g.idx()] > heim[g.idx()])
        .max_by_key(|g| (((bedarf[g.idx()] - heim[g.idx()]) as f64) * kurs(*g)) as i64) else { return };
    // Höchstens 95 % des freien Platzes: das Lager füllt sich bis zur Ausführung weiter (sonst „Im Ziellager ist nicht
    // genug Platz“, 30-mal je klassischer Epoche).
    let fehlt = ganz((bedarf[ziel.idx()] - heim[ziel.idx()]).min(mal(grenze[ziel.idx()] - heim[ziel.idx()], 0.95)));
    if fehlt < 100 { return; }
    let mut noetig = fehlt as f64 * kurs(ziel) / 0.98;
    let mut quellen: Vec<(Gut, i64)> = fest.into_iter()
        .filter(|g| *g != ziel && (*g != Gut::Nahrung || w.planeten[pid].rate[Gut::Nahrung.idx()] > 0))
        .map(|g| (g, ganz(heim[g.idx()] - bedarf[g.idx()].max(grenze[g.idx()] / 3))))
        .filter(|(_, u)| *u >= 100)
        .collect();
    quellen.sort_by_key(|(g, u)| -((*u as f64 * kurs(*g)) as i64));
    let mut angebot = serde_json::Map::new();
    for (g, ueber) in quellen {
        if noetig < 1.0 { break; }
        let menge = ((noetig / kurs(g)).floor() as i64).min(ueber);
        if menge < 1 { break; }
        angebot.insert(g.name().into(), json!(menge));
        noetig -= menge as f64 * kurs(g);
    }
    if angebot.is_empty() { return; }
    let k = w.planeten[pid].koord.to_string();
    if tu(w, sid, json!({"typ": "soforthandel", "planet": k, "angebot": angebot, "ziel": ziel.name()})) {
        zaehle("soforthandel: tausch für aufstieg");
    }
}

fn markt_handeln(w: &mut Welt, sid: SpielerId, typ: Bottyp) {
    let sp = &w.spieler[sid as usize];
    if sp.stufe < 3 || typ == Bottyp::Wehrlos { return; }
    let pid = sp.heimat as usize;
    let markt = w.planeten[pid].gebaeude[Gebaeude::Markt.idx()] as usize;
    if markt == 0 { return; }
    let r = w.regeln.clone();
    let mut bedarf = aufstiegsbedarf(w, sid);
    // Wer auf den Weltenkrümmer hinarbeitet, kauft Legierung, Elektronik und Xenokristall dafür (Doktrin und Krümmer) und
    // verkauft sie nicht als Überschuss.
    let kb = monde_bot::kruemmer_bedarf(w, sid, typ);
    for g in [Gut::Legierung, Gut::Elektronik, Gut::Xenokristall] { bedarf[g.idx()] = bedarf[g.idx()].max(kb[g.idx()]); }
    let sp = &w.spieler[sid as usize];
    let mut reich = [0i64; GUETER];
    for q in &sp.planeten { let b = w.bestand_jetzt(*q as usize); for g in 0..GUETER { reich[g] += b[g]; } }
    let heim = w.bestand_jetzt(pid);
    let k = w.planeten[pid].koord.to_string();
    let credits = sp.credits;
    // Eigene Orders aufräumen: Kauforders, deren Bedarf gedeckt ist (nach einem Tag, damit Händlerkäufe stehen bleiben),
    // und jede Order, die seit drei Tagen niemand bedient hat – sie wird mit Preis und Menge von heute neu gestellt.
    // 20.10.2026 klassisch: jeder Bot stellte früh eine Xenokristall-Kauforder, die für immer im Buch stand, und wer eine
    // Kauforder hat, verkauft dasselbe Gut nicht: in 365 Tagen keine einzige Xenokristall-Verkaufsorder, sieben Reiche
    // mit Millionen Credits blieben ohne Hyperraumantrieb auf Stufe IV.
    let jetzt = w.zeit;
    let alt: Vec<u32> = w.orders.iter().filter(|o| o.spieler == sid && (jetzt - o.zeit >= 3 * TAG
        || (o.seite == Marktseite::Kauf && jetzt - o.zeit >= TAG && bedarf[o.gut.idx()] <= reich[o.gut.idx()])))
        .map(|o| o.id).collect();
    for id in alt {
        if tu(w, sid, json!({"typ": "markt_storno", "order": id})) { zaehle("markt: alte order zurückgezogen"); }
    }
    let mut offen = w.orders.iter().filter(|o| o.spieler == sid).count();
    let grenze = markt * r.markt.orders_je_marktstufe;
    for g in [Gut::Xenokristall, Gut::Elektronik, Gut::Legierung, Gut::Deuterium, Gut::Kristall, Gut::Erz] {
        if offen >= grenze { break; }
        let i = g.idx();
        let wert = marktwert(w, g);
        let hat = |seite: Marktseite| w.orders.iter().any(|o| o.spieler == sid && o.gut == g && o.seite == seite);
        let fehlt = bedarf[i] - reich[i];
        if fehlt > 0 && !hat(Marktseite::Kauf) {
            let menge = ganz(fehlt).clamp(100, 5000);
            let preis = preis3(wert);
            if (menge as f64 * preis * 3.0) as i64 * M <= credits
                && tu(w, sid, json!({"typ": "markt_order", "planet": k, "gut": g.name(), "seite": "kauf", "menge": menge, "preis": preis})) {
                zaehle("markt: kauf für aufstieg"); offen += 1;
            }
            continue;
        }
        // Überschuss: über dem Bedarf und über einem Drittel der Lagergrenze (Xenokristall: über Bedarf + 2.000).
        let behalten = if g == Gut::Xenokristall { bedarf[i] + 2000 * M } else { bedarf[i].max(w.lagergrenze(pid)[i] / 3) };
        let ueber = ganz(heim[i] - behalten);
        if ueber >= 200 && !hat(Marktseite::Verkauf) && !hat(Marktseite::Kauf) && typ != Bottyp::Haendler {
            if tu(w, sid, json!({"typ": "markt_order", "planet": k, "gut": g.name(), "seite": "verkauf", "menge": ueber.min(3000), "preis": preis3(wert)})) {
                zaehle("markt: überschuss verkauft"); offen += 1;
            }
        }
    }
}

/// Der Kern startet nur den Kopf der Forschungsschlange (`forschung_schlange_starten`). Ist er nicht bezahlbar, wartet alles
/// dahinter – auch die Krümmer-Doktrin, und `forschen` reiht bei voller Schlange nichts nach (Fortsetzung der r48-Welt, Tag 186:
/// ein Räuber mit Schlange [Astrophysik, Hyperraumantrieb, Hyperraumantrieb], nichts aktiv, 23.000 Deuterium daheim, Doktrin bei
/// Hyperraumantrieb 1). Steht die Doktrin in der Schlange, rückt sie nach vorn (die Heimat spart auf sie); eine Doktrin an der
/// Spitze bleibt dort. Nur die Doktrin und erst ab der Stufe des Weltenkrümmers: Ein Vorziehen des ersten bezahlbaren Eintrags
/// ließ die billige Forschung jede Runde die Rücklage für den Aufstieg verbrauchen (frische Welt 712, r54–r57: zwei Bots blieben
/// von Tag 55 bis 180 auf Stufe IV, 9.986-mal „aufstieg: kosten fehlen“; r49 ohne dieses Vorziehen: alle 30 ab Tag 45 auf V).
fn schlange_entwirren(welt: &mut Welt, sid: SpielerId) {
    let sp = &welt.spieler[sid as usize];
    if sp.forschung_aktiv.is_some() || sp.forschung_schlange.len() < 2 { return; }
    if welt.regeln.einheiten.get(&Einheit::Weltentruemmer).is_none_or(|e| sp.stufe < e.ab_stufe) { return; }
    let r = &welt.regeln;
    let mut stufe = sp.forschung;
    let mut startbar = Vec::new();
    for (f, planet, _) in &sp.forschung_schlange {
        stufe[f.idx()] = stufe[f.idx()].saturating_add(1);
        let quelle = if welt.planeten.get(*planet as usize).is_some_and(|p| p.besitzer == sid) { *planet as usize } else { sp.heimat as usize };
        let k = r.kosten_forschung(*f, stufe[f.idx()]);
        startbar.push(bezahlbar(&welt.bestand_jetzt(quelle), &k) && welt.planeten[quelle].gebaeude[Gebaeude::Labor.idx()] >= r.forsch(*f).labor);
    }
    let doktrin: Vec<Forschung> = if monde_bot::kruemmer_forschung(welt, sid).is_some() {
        r.flottendoktrin.get(&Einheit::Weltentruemmer).map(|d| d.voraussetzungen.iter().filter(|(f, n)| sp.forschung[f.idx()] < **n).map(|(f, _)| *f).collect())
            .unwrap_or_default()
    } else { Vec::new() };
    let ist_doktrin = |i: usize| doktrin.contains(&sp.forschung_schlange[i].0);
    if doktrin.is_empty() || startbar[0] || ist_doktrin(0) { return; }
    let Some(i) = (1..startbar.len()).find(|i| ist_doktrin(*i)) else { return };
    let heimat = sp.heimat as usize;
    let kennung = welt.warteschlange_sicht(sid, "forschung", heimat)["kennung"].as_str().unwrap_or("").to_string();
    let koord = welt.planeten[heimat].koord.to_string();
    if tu(welt, sid, json!({"typ":"warteschlange_aendern","planet":koord,"art":"forschung","index":i,"ziel":0,"loeschen":false,"kennung":kennung})) {
        zaehle("forschung: schlange umgestellt");
    }
}

fn forschen(welt: &mut Welt, sid: SpielerId, typ: Bottyp) {
    use Forschung::*;
    schlange_entwirren(welt, sid);
    let sp = &welt.spieler[sid as usize];
    // Volle Warteschlange (Krieg und Monde reihen Doktrin und Mondforschung ein): nichts nachschieben, der Kern startet sie
    // selbst (bot_lage 20.10.2026: 230-mal „Forschungsschlange ist voll“).
    if sp.forschung_aktiv.is_some() || sp.forschung_schlange.len() >= welt.regeln.wirtschaft.forschung_warteschlange {
        return;
    }
    let r = welt.regeln.clone();
    // Weltenkrümmer: Die Doktrin geht vor. Ist ihre nächste Stufe noch nicht bezahlbar (aber beschaffbar), forscht der Bot nichts
    // anderes und spart – Schnellprobe 21.10.2026, Tag 90: Werftdock 10 stand seit Tag ~75, aber die billigste Forschung jeder
    // Runde leerte den Kristall der Heimat (4.434 statt der 160.000 für Hyperraumantrieb 4).
    if let Some((f, st)) = monde_bot::kruemmer_forschung(welt, sid) {
        let k = r.kosten_forschung(f, st);
        let heim = welt.bestand_jetzt(sp.heimat as usize);
        if bezahlbar(&heim, &k) && welt.planeten[sp.heimat as usize].gebaeude[Gebaeude::Labor.idx()] >= r.forsch(f).labor {
            if tu(welt, sid, json!({"typ":"forschen","forschung":f.name()})) { zaehle("forschung: krümmer-doktrin"); }
            return;
        }
        if beschaffbar(welt, sid, &heim, &k) { zaehle("forschung: spart für die krümmer-doktrin"); return; }
    }
    // Dysonsphäre bis auf die Einzahlung bereit: keine weitere Forschung, der Überschuss gehört dem Projekt – Schnellprobe
    // 21.10.2026, Tag 70–90: die Mitglieder zahlten nur ~20 % ihrer Förderung ein, 27.857-mal „nachschub zu klein“, weil die
    // billigste Forschung jeder Runde die Heimat leerte (Forschung prüft keine Rücklage).
    if dyson_bot::sparziel(welt, sid).iter().any(|x| *x > 0) { zaehle("forschung: pause für die dysonsphäre"); return; }
    // Zuerst, was die nächste Stufe verlangt, wie es im Regelwerk steht: ihre Forschung und für ihre Kolonien
    // Astrophysik (je zwei Stufen eine Kolonie mehr). So folgt der Bot auch geänderten Regeln.
    // 20.10.2026: die Bedingungen der nächsten Stufe vor Terraforming und Komfortforschung – mit dem schnelleren
    // Stufentempo verdrängten Terraforming (8.000 FP) und Astrophysik 5 den Hyperraumantrieb, kein Bot erreichte V.
    let mut plan: Vec<(Forschung, u8)> = Vec::new();
    if let Some(naechste) = r.stufen.get(sp.stufe as usize - 1) {
        plan.extend(naechste.forschung.iter().map(|(f, n)| (*f, *n)));
        if naechste.kolonien > 0 {
            // Ausbau-Welten: je Stufe der Kolonieforschung eine Kolonie (neue Welten nach den freien Plätzen);
            // ältere Welten: je zwei Astrophysikstufen eine.
            plan.push((welt.kolonie_forschung(), welt.kolonie_stufe_roh(naechste.kolonien)));
        }
    }
    // Kolonieschiff-Forschung wie in der Doktrin der Welt (neue Welten seit 25.10.2026 nur Impulsantrieb 2, laufende dazu Astrophysik 1).
    if sp.stufe>=3 && welt.ausbau.industrie_version>0 {
        plan.push((Impulsantrieb,2));
        if let Some(d)=r.flottendoktrin.get(&Einheit::Kolonieschiff) {plan.extend(d.voraussetzungen.iter().map(|(f,n)|(*f,*n)));}
        plan.extend([(Computertechnik,2),(Logistik,1)]);
    }
    if r.wirtschaft.terraforming_felder>0 && sp.stufe>=r.forsch(Terraforming).ab_stufe && sp.planeten.iter().any(|p|welt.felder_belegt(*p as usize)+8>=welt.felder_gesamt(*p as usize))
        && bezahlbar(&welt.bestand_jetzt(sp.heimat as usize),&r.kosten_forschung(Terraforming,sp.forschung[Terraforming.idx()].saturating_add(1))) {
        plan.push((Terraforming,sp.forschung[Terraforming.idx()].saturating_add(1)));
    }
    // Räuber ab Stufe III: Doktrin des schweren Jägers (Impulsantrieb 2 steht oben) und tiefere Spionage.
    if typ == Bottyp::Raeuber && sp.stufe >= 3 {
        plan.extend([(Waffentechnik, 2), (Schildtechnik, 3), (Spionagetechnik, 4)]);
    }
    // Eine zweite Kolonie schon auf Stufe III: eine junge Kolonie wächst mit billigem Wohnraum schneller als die volle
    // Heimatwelt, und ihre Einwohner zählen für Stufe IV mit.
    if sp.stufe == 3 && welt.ausbau.version > 0 {
        plan.push((welt.kolonie_forschung(), welt.kolonie_stufe_roh(2)));
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
            plan.extend([(welt.kolonie_forschung(), welt.kolonie_stufe_roh(3)), (Hyperraumantrieb, 1)]);
            if welt.ausbau.version == 0 { plan.push((Astrophysik, 5)); }
        }
        match typ {
            Bottyp::Raeuber => plan.extend([(Waffentechnik, 4), (Computertechnik, 4), (Verbrennungsantrieb, 4), (Schildtechnik, 3), (Logistik, 3)]),
            Bottyp::Igel => plan.extend([(Schildtechnik, 4), (Panzerung, 4), (Waffentechnik, 4)]),
            _ => plan.extend([(Werkstoffkunde, 4), (Agrarwissenschaft, 4)]),
        }
    }
    if sp.stufe >= 4 {
        // Technik v1: Plasmatechnik (braucht Energietechnik 5) vor den späten Ausbauforschungen.
        if r.forschung.contains_key(&Plasmatechnik) {plan.extend([(Energietechnik, 5), (Plasmatechnik, 2)]);}
        // Computertechnik 3: der vierte Flottenplatz. Mit dreien flog die Logistik dauerhaft auf zweien, und Händler,
        // Ökonomen und Igel bauten keinen Gürtel ab (5 von 40 Abbauflügen in 30 Tagen, Bericht Abbau 20.10.2026).
        plan.extend([(Computertechnik, 3), (Xenomaterialkunde, 1), (Soziologie, 6), (Energietechnik, 7), (Automatisierung, 6)]);
        if r.forschung.contains_key(&Plasmatechnik) {plan.push((Plasmatechnik, 4));}
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
            && (f!=Plasmatechnik || welt.plasmatechnik_fehlt(sid).is_empty())
    };
    // Pflichtforschung des Aufstiegs: was die nächste Stufe verlangt und Astrophysik für ihre Kolonien.
    let pflicht: Vec<Forschung> = r.stufen.get(stufe as usize - 1).map(|n| {
        let mut v: Vec<Forschung> = n.forschung.keys().copied().collect();
        if n.kolonien > 0 { v.push(welt.kolonie_forschung()); }
        v
    }).unwrap_or_default();
    let mut pflicht_wartet = false;
    for (f, min) in plan {
        if stufen[f.idx()] >= min || !moeglich(f) {
            continue;
        }
        // Wartet eine Pflichtforschung auf Mittel, geht andere bezahlbare Pflichtforschung vor, aber keine lange
        // Nebenforschung (20.10.2026: sonst warteten Igel auf den Hyperraumantrieb und forschten nie Astrophysik für
        // die dritte Kolonie – Stufe V klassisch 4 % statt 100 %; ohne die Wartegrenze belegte Terraforming den Platz).
        // Erst nur bei fehlendem Xenokristall, dann noch 38 %: auch Deuterium für den Hyperraumantrieb hielt das Labor auf.
        if pflicht_wartet && !pflicht.contains(&f) {
            return;
        }
        let kosten = r.kosten_forschung(f, stufen[f.idx()] + 1);
        if bezahlbar(&bestand, &kosten) {
            tu(welt, sid, json!({"typ": "forschen", "forschung": f.name()}));
            return;
        }
        // Darauf sparen, außer es fehlt ein Gut, das der Spieler weder hat noch herstellt.
        if beschaffbar(welt, sid, &bestand, &kosten) {
            if pflicht.contains(&f) {
                pflicht_wartet = true;
                continue;
            }
            return;
        }
    }
    if pflicht_wartet {
        return;
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
        // Ohne Haltezeit steigt der Bot im selben Zug auf, in dem alle Bedingungen erfüllt und die Kosten bezahlbar sind.
        let bedingungen = sp.stufen_zaehler >= r.stufen_haltezeit_stunden * STUNDE
            && (r.stufen_haltezeit_stunden > 0 || welt.stufen_bedingungen(sid).iter().all(|(_, ok)| *ok));
        if !bedingungen {
            zaehle("aufstieg: bedingungen offen");
        } else if !bezahlbar(&welt.bestand_jetzt(sp.heimat as usize), &preis_array(&regel.kosten)) {
            zaehle("aufstieg: kosten fehlen");
        } else if tu(welt, sid, json!({"typ": "stufenaufstieg"})) {
            zaehle("aufstieg: erfolgt");
        }
    }
}

/// Solarsatelliten (Kennung solarsonde, Mondgeneration, ab Stufe II): Energie ohne Bauplatz. Bei Strommangel,
/// wenn sie je Energieeinheit billiger sind als die nächste Solarstufe oder kaum Felder frei sind.
fn solarsatelliten(w: &mut Welt, sid: SpielerId) {
    let sat = Einheit::Solarsonde;
    let (Some(regel), Some(m)) = (w.regeln.einheiten.get(&sat).cloned(), w.regeln.monde.clone()) else { return };
    let sp = &w.spieler[sid as usize];
    if sp.stufe < regel.ab_stufe {
        return;
    }
    let (volk, et) = (sp.volk, sp.forschung[Forschung::Energietechnik.idx()]);
    for id in sp.planeten.clone() {
        let pid = id as usize;
        let p = &w.planeten[pid];
        if p.gebaeude[Gebaeude::Werft.idx()] < regel.werft || !p.fertigung[0].is_empty() {
            continue;
        }
        let bedarf = p.energie_verbrauch + p.energie_verbrauch / 10 + 40 * M;
        if p.energie_erzeugung >= bedarf {
            continue;
        }
        let je = kern::monde::solarsonde_energie(&w.regeln, &m, p.koord.position, et, w.regeln.volk(volk).energie, 0.0);
        if je <= 0 {
            continue;
        }
        let r = &w.regeln;
        let n_solar = p.gebaeude[Gebaeude::Solarkraftwerk.idx()];
        let solar_mehr = anteil(r.stufenwert(r.geb(Gebaeude::Solarkraftwerk).ertrag, n_solar + 1) - r.stufenwert(r.geb(Gebaeude::Solarkraftwerk).ertrag, n_solar), p.faktor[F_SOLAR], 1000)
            * (1000 + 50 * et as i64) / 1000;
        let kosten = r.kosten_einheit(sat, volk);
        let sat_preis = r.wert(&kosten) / je.max(1);
        let solar_preis = r.wert(&r.kosten_gebaeude(Gebaeude::Solarkraftwerk, n_solar + 1)) / solar_mehr.max(1);
        let felder_knapp = w.felder_belegt(pid) + 5 >= w.felder_gesamt(pid);
        if sat_preis >= solar_preis && !felder_knapp {
            continue;
        }
        let bestand = w.bestand_jetzt(pid);
        let leistbar = (0..GUETER).filter(|g| kosten[*g] > 0).map(|g| bestand[g] / kosten[g]).min().unwrap_or(0);
        let n = ((bedarf - p.energie_erzeugung + je - 1) / je).clamp(1, 20).min(leistbar);
        if n > 0 {
            tu(w, sid, json!({"typ": "fertigen", "planet": p.koord.to_string(), "einheit": sat.name(), "anzahl": n}));
        }
    }
}

/// V12 uses real, disjoint qualifications. Building an academy alone no longer
/// creates specialists; bots pay for training through the same commands as players.
fn industrie_personal(w:&mut Welt,sid:SpielerId){
    if w.ausbau.industrie_version==0{return;}
    for id in w.spieler[sid as usize].planeten.clone(){let pid=id as usize;let k=w.planeten[pid].koord;
        let order=[Gebaeude::Solarkraftwerk,Gebaeude::Farm,Gebaeude::Fusionskraftwerk,Gebaeude::Akademie,Gebaeude::Elektronikwerk,Gebaeude::Konsumgueterwerk,Gebaeude::Giesserei,Gebaeude::Labor,Gebaeude::Werft,Gebaeude::Orbitalwerft];
        if !w.planeten[pid].prioritaeten.starts_with(&order){tu(w,sid,json!({"typ":"prioritaeten","planet":k.to_string(),"reihenfolge":order.map(|g|g.name())}));}
        let e=w.industrie_berechnen(&w.planeten[pid]);
        let slots=w.planeten[pid].gebaeude[Gebaeude::Akademie.idx()] as i64*20;
        let booked=w.ausbau.ausbildungen.iter().filter(|a|a.planet==id).map(|a|a.anzahl).sum::<i64>();
        // Eine beschädigte Akademie (Bombardierung) bildet nicht aus, bis sie repariert ist (Kernregel „intakte Plätze“).
        if slots<=booked || w.integritaet(pid,Gebaeude::Akademie)<1000 {continue;}
        let people=&w.ausbau.personen[&id];
        let quote=w.regeln.wirtschaft.arbeitsquote;
        let demand=e.werke.iter().filter(|a|a.gebaeude!=Gebaeude::Labor).map(|a|a.fachkraefte_bedarf).sum::<i64>();
        let research=e.werke.iter().filter(|a|a.gebaeude==Gebaeude::Labor).map(|a|a.fachkraefte_bedarf).sum::<i64>();
        let targets=[(1,(demand*1000/milli(quote).max(1)+20*M-people.fachkraefte()).max(0)/M),(2,(research*1000/milli(quote).max(1)+10*M-people.forscher()).max(0)/M)];
        let volk=w.spieler[sid as usize].volk;
        for (q,needed) in targets {if q==2&&w.planeten[pid].gebaeude[Gebaeude::Labor.idx()]==0{continue;}let pending=w.ausbau.ausbildungen.iter().filter(|a|a.planet==id&&a.qualifikation==q).map(|a|a.anzahl).sum::<i64>();
            let mut stock=w.bestand_jetzt(pid);let free=slots-w.ausbau.ausbildungen.iter().filter(|a|a.planet==id).map(|a|a.anzahl).sum::<i64>();
            // Beim Sparen auf den Aufstieg bleibt dessen Elektronik auf der Heimatwelt liegen.
            if w.planeten[pid].heimat && spart(w,sid) {if let Some(regel)=w.regeln.stufen.get(w.spieler[sid as usize].stufe as usize-1){let kosten=preis_array(&regel.kosten);for g in 0..GUETER{stock[g]=(stock[g]-kosten[g]).max(0);}}}
            let n=(needed-pending).min(free).min(stock[Gut::Konsumgut.idx()]/(10*M)).min(stock[Gut::Elektronik.idx()]/(q as i64*5*M)).min(1000);
            if n>0{tu(w,sid,json!({"typ":"ausbilden","planet":k.to_string(),"volk":volk.name(),"qualifikation":q,"anzahl":n}));}
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
    // Grundschutz ab Stufe II für alle (19.10.2026): ohne ihn plünderten Räuber nur wehrlose Welten – 96 von 110
    // Bot-Kämpfen hinterließen keine Trümmer, es entstand nie ein Mond (Karl: „keiner hatte einen Mond“).
    // Igel 40 % des Gebäudewerts (bis 20.10.2026 50 %): mit der Räuberregel „Beute mindestens doppelter Verlust“ verloren
    // Igel klassisch nur noch 124.000 Punkte je Epoche an Plünderung, wuchsen aber ein Drittel langsamer als alle anderen
    // und erreichten Stufe V erst am Tag 130.
    let anteil_pm: i64 = match typ {
        Bottyp::Igel => 400,
        Bottyp::Oekonom | Bottyp::Haendler | Bottyp::Raeuber if gepluendert(welt, sid) => 150,
        _ => 50,
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
            if welt.ausbau.industrie_version>0&&welt.fertigungsplaetze(pid,0)<kern::parallel::platzbedarf(Produkt::Einheit(*e)){continue;}
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
    // Berichte älterer Layouts (weniger Schiffstypen) bleiben lesbar.
    for (i, n) in bs.iter().take(SCHIFFE).enumerate() { vert[i] = *n; }
    for (i, n) in bv.iter().take(EINHEITEN - SCHIFFE).enumerate() { vert[SCHIFFE + i] = *n; }
    if vert.iter().all(|n| *n == 0) {
        return Some((100, 0, beute));
    }
    let tech = |f: Forschung| 1.0 + r.kampf.tech_je_stufe * b.forschung.as_ref().map(|t| t[f.idx()]).unwrap_or(sp.forschung[f.idx()]) as f64;
    let a = Gruppe::neu(r, sp, *ang);
    let defender = r.volk(welt.spieler[b.besitzer as usize].volk);
    let v = Gruppe::mit_werten(r, b.besitzer, vert, tech(Forschung::Waffentechnik) * defender.waffen,
        tech(Forschung::Schildtechnik), tech(Forschung::Panzerung) * defender.panzerung);
    // Vorfilter: Schaden je Runde mal Haltbarkeit beider Seiten (Lanchester). Bleibt der Angreifer unter einem Viertel
    // des Verteidigers, gewinnt er in sechs Runden nicht, auch mit Schnellfeuer – ohne acht Simulatorläufe (im
    // 30-Tage-Lauf scheiterten über 30.000 Schätzungen an Siegchance < 75 %, die Rechenzeit verdoppelte sich).
    let wucht = |g: &Gruppe| {
        let (mut schaden, mut halt) = (0i128, 0i128);
        for e in 0..EINHEITEN {
            schaden += g.einheiten[e] as i128 * g.angriff[e] as i128;
            halt += g.einheiten[e] as i128 * (g.struktur[e] + g.schild[e]) as i128;
        }
        schaden * halt
    };
    if wucht(&a) * 4 < wucht(&v) {
        return Some((0, 0, beute));
    }
    const LAEUFE: u64 = 8;
    let (mut siege, mut verlust) = (0i64, 0i64);
    for i in 0..LAEUFE {
        let nr = (welt.zeit as u64) << 24 | (sid as u64) << 8 | 0x80 | i;
        let mut rng = strom(welt.startwert, KANAL_SIMULATOR, nr);
        let erg = kampf_ohne_protokoll(r, std::slice::from_ref(&a), std::slice::from_ref(&v), &mut rng);
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

/// Schiffe, mit denen der Räuber angreift: alles Bewaffnete außer Frachtern, Sonden, Kolonie- und Arbeitsschiffen.
const ANGRIFFSSCHIFFE: [Einheit; 13] = [Einheit::LeichterJaeger, Einheit::Abfangjaeger, Einheit::SchwererJaeger, Einheit::Sturmjaeger,
    Einheit::Kreuzer, Einheit::Raketenkreuzer, Einheit::Schildkreuzer, Einheit::Flottenkreuzer, Einheit::Schlachtschiff,
    Einheit::Schlachtkreuzer, Einheit::Zerstoerer, Einheit::Abfangzerstoerer, Einheit::Sturmschlachtschiff];
/// Sonden je Spähflug des Räubers: Scanstärke = Sonden + eigene minus fremde Spionagetechnik; Verteidigung steht
/// erst ab 3, Gebäude (Bunker) ab 5 im Bericht. Mit 2 Sonden fehlte die Verteidigung in fast jedem Bericht, der
/// Räuber konnte keinen Kampf durchrechnen und griff in 30 Tagen nie an (bot_lage 20.10.2026: 24.553-mal).
const SPAEH_SONDEN: i64 = 5;

/// Kann der Planet diese Einheit jetzt fertigen (Regel, Stufe, Werft, Gebäude, Doktrinforschung)?
fn baubar(welt: &Welt, sid: SpielerId, pid: usize, e: Einheit) -> bool {
    let Some(er) = welt.regeln.einheiten.get(&e) else { return false };
    let p = &welt.planeten[pid];
    welt.spieler[sid as usize].stufe >= er.ab_stufe
        && (!e.ist_schiff() || p.gebaeude[Gebaeude::Werft.idx()] >= er.werft.max(1))
        && er.braucht.iter().all(|(b, st)| p.gebaeude[b.idx()] >= *st)
        && welt.einheit_forschung_fehlend(sid, e).is_empty()
        && (welt.ausbau.industrie_version == 0 || welt.fertigungsplaetze(pid, 0) >= kern::parallel::platzbedarf(Produkt::Einheit(e)))
}

// ---------------------------------------------------------------------------------------------- Mitspieler
//
// Fairness gegenüber Mitspielern (Karl 22.10.2026: „die Bots können Schiffe bauen und mich angreifen, ich bin als
// menschlicher Spieler über die UI wehrlos – unfair“). Bots handeln alle zwei Spielstunden mit vollem Überblick, ein
// Mensch tippt am Handy und schläft nachts; bei Tempo 30 sind zwei Spielstunden Vorwarnung vier echte Minuten. Gegen
// Mitspieler (jedes aktive Reich, das kein Skriptbot ist) gilt darum mit aktivem Opferschutz:
// - Schonzeit nach Stärke statt nur Kalender: unter Stufe III, ohne Werft auf irgendeinem Planeten oder Heimatwelt
//   jünger als doppelte `neulinge_tage` – weder ausspähen noch angreifen.
// - Wer selbst angreift, verliert die Schonzeit für `VERGELTUNG` (Gegenschlag und Kriegserklärung bleiben möglich).
// - Doppelte Raubpause je Mitspieler, Bots haben als Ziel Vorrang, höchstens halbe Beute (Frachtraum begrenzt),
//   nur so viele Kampfschiffe wie für den sicheren Sieg nötig, Anflug mindestens `ANFLUG_MIN` (Zeit zum Reagieren).
// - Keine Eroberung (Bombardieren/Invasion als Raub) und keine Mondzerstörung gegen Mitspieler, außer im Krieg, den
//   der Mitspieler mit eigenen Angriffen ausgelöst hat.
// Bot gegen Bot bleibt unverändert (Balance-Messungen). Welche Reiche Mitspieler sind, weiß nur der Server: er ruft
// die Bots innerhalb von `mit_mitspielern` auf.

/// So lange nach seinem letzten eigenen Angriff gilt für einen Mitspieler keine Schonzeit.
pub const VERGELTUNG: i64 = 72 * STUNDE;
/// Kürzester Anflug eines Bots auf einen Mitspieler (Spielzeit): genug für Warnung, Saven oder Verteidigung.
pub const ANFLUG_MIN: i64 = 4 * STUNDE;
/// Anteil der gemeldeten Beute, den Bots einem Mitspieler höchstens abnehmen (über den Frachtraum).
const BEUTE_MITSPIELER: i64 = 2;

thread_local! {
    static MITSPIELER: std::cell::RefCell<std::collections::BTreeSet<SpielerId>> = const { std::cell::RefCell::new(std::collections::BTreeSet::new()) };
}

/// Führt `f` aus, während `menschen` als Mitspieler gelten (Server: alle aktiven Reiche ohne Skriptbot).
pub fn mit_mitspielern<R>(menschen: &std::collections::BTreeSet<SpielerId>, f: impl FnOnce() -> R) -> R {
    let alt = MITSPIELER.with(|m| std::mem::replace(&mut *m.borrow_mut(), menschen.clone()));
    let r = f();
    MITSPIELER.with(|m| *m.borrow_mut() = alt);
    r
}

/// Ist `ziel` ein Mitspieler, für den die Fairnessregeln gelten (nur mit aktivem Opferschutz)?
fn mitspieler(schutz: &Opferschutz, ziel: SpielerId) -> bool {
    schutz.aktiv && MITSPIELER.with(|m| m.borrow().contains(&ziel))
}

/// Hat das Reich in den letzten `VERGELTUNG` Sekunden selbst angegriffen?
fn greift_selbst_an(welt: &Welt, ziel: SpielerId) -> bool {
    welt.kampfberichte.iter().rev().take_while(|k| welt.zeit - k.zeit < VERGELTUNG).any(|k| k.angreifer == ziel)
}

/// Schonzeit eines Mitspielers: `None` = angreifbar, sonst der Grund (auch für die Anzeige im Spiel).
pub fn mitspieler_schonzeit(welt: &Welt, schutz: &Opferschutz, ziel: SpielerId) -> Option<&'static str> {
    if !schutz.aktiv { return None; }
    let g = &welt.spieler[ziel as usize];
    if greift_selbst_an(welt, ziel) { return None; }
    if g.stufe < 3 { return Some("unter Zivilisationsstufe III"); }
    if !g.planeten.iter().any(|p| welt.planeten[*p as usize].gebaeude[Gebaeude::Werft.idx()] > 0) { return Some("noch ohne Werft"); }
    if welt.zeit - welt.planeten[g.heimat as usize].gegruendet < 2 * schutz.neulinge_tage as i64 * TAG { return Some("Heimatwelt noch jung"); }
    None
}

/// Schutzstand eines Reichs gegenüber den Bots für die Spieleransicht.
pub fn schutz_sicht(welt: &Welt, schutz: &Opferschutz, sid: SpielerId, mensch: bool) -> Value {
    if !schutz.aktiv {
        return json!({"aktiv": false, "geschuetzt": false, "text": "Opferschutz ist in dieser Welt ausgeschaltet: Bots greifen jedes lohnende Ziel an."});
    }
    if !mensch {
        return json!({"aktiv": true, "geschuetzt": neuling(welt, schutz, sid), "text": ""});
    }
    let pause = 2 * schutz.pause_stunden as i64 * STUNDE;
    let letzter = welt.kampfberichte.iter().rev().take_while(|k| welt.zeit - k.zeit < pause)
        .find(|k| k.verteidiger.contains(&sid)).map(|k| k.zeit);
    let grund = mitspieler_schonzeit(welt, schutz, sid);
    let jung_bis = welt.planeten[welt.spieler[sid as usize].heimat as usize].gegruendet + 2 * schutz.neulinge_tage as i64 * TAG;
    let text = if let Some(g) = grund {
        format!("Schonzeit vor Bots: {g}. Bots spähen dich nicht aus und greifen nicht an. Sie endet ab Stufe III mit eigener Werft und frühestens am Spieltag {}. Greifst du selbst an, endet sie für drei Spieltage.", jung_bis / TAG + 1)
    } else if greift_selbst_an(welt, sid) {
        "Keine Schonzeit: Du hast in den letzten drei Spieltagen selbst angegriffen – Bots dürfen zurückschlagen.".to_string()
    } else if let Some(t) = letzter {
        format!("Raubpause: Nach dem letzten Angriff greift dich bis Spieltag {} kein Bot an.", (t + pause) / TAG + 1)
    } else {
        format!("Bots dürfen dich angreifen, aber fair: höchstens alle {} Spielstunden, höchstens halbe Beute, keine Eroberung und Anflug mindestens {} Spielstunden – genug Zeit zum Saven oder Verteidigen.", 2 * schutz.pause_stunden, ANFLUG_MIN / STUNDE)
    };
    json!({"aktiv": true, "mitspieler": true, "geschuetzt": grund.is_some() || letzter.is_some(), "grund": grund,
        "schonzeit_bis_sekunden": jung_bis, "raubpause_bis_sekunden": letzter.map(|t| t + pause), "text": text})
}

/// Neuling: unter Stufe II, oder Stufe II und die Heimatwelt jünger als `neulinge_tage` (Generation 12 hat keinen
/// Anfängerschutz im Kern: `schutz_bis` endet mit dem Beitritt). Ab Stufe III nie; ohne Opferschutz niemand.
/// Mitspieler: Schonzeit nach `mitspieler_schonzeit`.
fn neuling(welt: &Welt, schutz: &Opferschutz, ziel: SpielerId) -> bool {
    if mitspieler(schutz, ziel) { return mitspieler_schonzeit(welt, schutz, ziel).is_some(); }
    let g = &welt.spieler[ziel as usize];
    schutz.aktiv && (g.stufe < 2 || (g.stufe == 2 && welt.zeit - welt.planeten[g.heimat as usize].gegruendet < schutz.neulinge_tage as i64 * TAG))
}

/// Darf ein Räuber dieses Reich angreifen? Nie, solange schon ein eigener Angriff dorthin fliegt. Mit Opferschutz
/// keine Neulinge und kein Ort und kein Reich, an dem oder gegen das in den letzten `pause_stunden` irgendwer gekämpft
/// hat – frische Trümmerfelder sind öffentlich (sonst plünderten im Messlauf mehrere Räuber nacheinander dasselbe
/// reichste Reich, 13-mal in elf Tagen, verteilt auf seine Kolonien). Mitspieler: doppelte Pause.
fn raub_erlaubt(welt: &Welt, schutz: &Opferschutz, sid: SpielerId, ziel: SpielerId, ort: Koord) -> bool {
    let pause = schutz.pause_stunden as i64 * STUNDE * if mitspieler(schutz, ziel) { 2 } else { 1 };
    !neuling(welt, schutz, ziel)
        // Keine eigene Flotte mit Ziel oder Orbit dort: hält eine Blockade (Bombardieren, Kampfkolonisieren) den Orbit, findet
        // ein Angriff keinen Gegner (bot_lage 20.10.2026: 124-mal abgelehnt).
        && !welt.flotten.values().any(|f| f.besitzer == sid && f.ziel == ort
            && matches!(f.mission, Mission::Angriff | Mission::Bombardieren | Mission::Kampfkolonisieren))
        && !(schutz.aktiv && welt.kampfberichte.iter().rev().take_while(|k| welt.zeit - k.zeit < pause)
            .any(|k| k.ort == ort || k.verteidiger.contains(&ziel)))
}

/// Kleinste Raubflotte gegen einen Mitspieler: ein Viertel, die Hälfte, drei Viertel oder alle Kampfschiffe – der
/// erste Anteil, mit dem der Simulator weiter mindestens 75 % Siegchance verspricht. Ein Raub soll eine Lektion
/// sein, kein Auslöschen mit der ganzen Heimatflotte.
fn raubflotte_mitspieler(welt: &Welt, sid: SpielerId, b: &Spionagebericht, ang: &[i64; EINHEITEN]) -> [i64; EINHEITEN] {
    for teil in [4i64, 2] {
        let mut f = [0i64; EINHEITEN];
        for i in 0..EINHEITEN { f[i] = (ang[i] + teil - 1) / teil; }
        if f.iter().sum::<i64>() >= 5 && angriff_schaetzen(welt, sid, b, &f).is_some_and(|(chance, _, _)| chance >= 75) {
            return f;
        }
    }
    let mut f = [0i64; EINHEITEN];
    for i in 0..EINHEITEN { f[i] = (3 * ang[i] + 3) / 4; }
    if angriff_schaetzen(welt, sid, b, &f).is_some_and(|(chance, _, _)| chance >= 75) { f } else { *ang }
}

/// Fluggeschwindigkeit gegen einen Mitspieler: die schnellste, deren Anflug mindestens `ANFLUG_MIN` dauert und deren
/// Hin- und Rückflug der Vorrat trägt; ist jeder Anflug kürzer, die langsamste bezahlbare.
fn anflug_tempo_mitspieler(welt: &Welt, sid: SpielerId, pid: usize, ziel: Koord, flotte: &[i64; SCHIFFE], deut: i64) -> Option<f64> {
    let stufen = [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1];
    let plaene: Vec<(f64, i64)> = stufen.into_iter().filter_map(|s| {
        welt.flugplan(sid, pid, ziel, flotte, milli(s)).ok().filter(|p| 2 * p.treibstoff <= deut).map(|p| (s, p.dauer))
    }).collect();
    plaene.iter().find(|(_, d)| *d >= ANFLUG_MIN).or(plaene.last()).map(|(s, _)| *s)
}

/// Der Räuber späht Nachbarn aus und greift an, wenn der Simulator Beute über den Verlusten verspricht.
fn raeuber(welt: &mut Welt, sid: SpielerId, bot: &mut Bot) {
    let r = welt.regeln.clone();
    let sp = &welt.spieler[sid as usize];
    if sp.stufe < 3 {
        zaehle("räuber: stufe < 3");
        return;
    }
    let pid = sp.heimat as usize;
    let p = &welt.planeten[pid];
    if p.gebaeude[Gebaeude::Werft.idx()] < 1 || p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 {
        zaehle("räuber: ohne werft/raumhafen");
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
        let sondenbedarf=(2*SPAEH_SONDEN-sonden_geplant(welt,sid,pid)).max(0);
        // Schwere Jäger (Stufe III, Werft 3) durchschlagen die Schilde der Lasergeschütze; leichte Jäger (Angriff 50
        // gegen Schild 75) richteten gegen den Grundschutz anderer Bots nichts aus. Rapidfeuerfrei, also die
        // Hauptmasse, sobald die Doktrin (Impulsantrieb 2, Schildtechnik 3, Waffentechnik 2) erforscht ist.
        let schwer = [Einheit::SchwererJaeger, Einheit::Abfangjaeger].into_iter().find(|e| baubar(welt, sid, pid, *e));
        let bestellung = if sondenbedarf > 0 {
            Some((Einheit::Spionagesonde, stueck(Einheit::Spionagesonde, 1, sondenbedarf)))
        } else if n(welt, Einheit::KleinerTransporter) * 4 < jaeger + 8 {
            Some((Einheit::KleinerTransporter, stueck(Einheit::KleinerTransporter, 2, 10)))
        } else if welt.spieler[sid as usize].stufe >= 4 && p.gebaeude[Gebaeude::Werft.idx()] >= 4 && n(welt, Einheit::Kreuzer) * 6 < jaeger {
            Some((Einheit::Kreuzer, stueck(Einheit::Kreuzer, 2, 15)))
        } else if let Some(e) = schwer.filter(|e| n(welt, *e) < 2 * jaeger + 20) {
            Some((e, stueck(e, 2, 30)))
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

    // Angriff auf das beste frisch ausgespähte Ziel, mit allen Kampfschiffen der Heimatwelt.
    let mut ang = [0i64; EINHEITEN];
    for e in ANGRIFFSSCHIFFE { ang[e.idx()] = n(welt, e); }
    let kampfschiffe: i64 = ang.iter().sum();
    let transporter = n(welt, Einheit::KleinerTransporter);
    if frei(welt) <= 0 {
        zaehle("räuber: keine flottenplätze");
    } else if kampfschiffe < 5 {
        zaehle("räuber: < 5 kampfschiffe daheim");
    }
    if frei(welt) > 0 && kampfschiffe >= 5 {
        let alle = welt.spieler[sid as usize].berichte.iter().filter(|b| b.besitzer != sid).count();
        if alle == 0 { zaehle("räuber: keine berichte"); }
        // Nur die zwei Berichte mit dem meisten Lagerwert durchrechnen: der Simulator ist teuer.
        let mut berichte: Vec<(i64, Spionagebericht)> = welt.spieler[sid as usize]
            .berichte
            .iter()
            .filter(|b| jetzt - b.zeit < 8 * STUNDE && b.besitzer != sid && (welt.aufklaerungsregeln() || !geschuetzt(welt, b.besitzer)))
            .filter(|b| !welt.vertrag_zwischen(sid, b.besitzer, Vertragsart::Nichtangriffspakt))
            .filter(|b| raub_erlaubt(welt, &bot.opferschutz, sid, b.besitzer, b.ziel))
            .map(|b| {
                let mut lager = [0i64; GUETER];
                lager.copy_from_slice(&b.bestand[..GUETER]);
                (-r.wert(&lager), b.clone())
            })
            .collect();
        if alle > 0 && berichte.is_empty() { zaehle("räuber: berichte alt/pakt"); }
        berichte.sort_by_key(|(w, _)| *w);
        let mut bestes: Option<(bool, i64, Koord, i64, Spionagebericht)> = None;
        // Vier lohnendste Ziele prüfen, Kämpfe gegen Verteidigung annehmen, wenn der Sieg wahrscheinlich ist und die Beute
        // die Verluste trägt (19.10.2026: vorher nur wehrlose Welten – kaum Trümmer, nie ein Mond, „langweilig“ – Karl).
        for (lagerwert, b) in berichte.iter().take(4) {
            if -lagerwert < 20_000 * M {
                zaehle("bericht: lager < 20k");
                continue;
            }
            let Some((chance, verlust, beute)) = angriff_schaetzen(welt, sid, b, &ang) else {
                zaehle("bericht: ohne schiffe/verteidigung");
                continue;
            };
            let verlust = verlust / M;
            let menge: i64 = beute.iter().sum::<i64>() / M;
            let wert = r.wert(&beute) / M;
            // Lohnend erst, wenn die Beute den erwarteten Schiffsverlust mindestens verdoppelt: 20.10.2026 klassisch
            // griffen die stärkeren Räuber Igel 297-mal statt 16-mal an (Beute knapp über Verlust), die Igel wuchsen ab
            // Tag 100 kaum noch, Stufe V erreichten 58 % statt 100 %. Ein Plünderer setzt seine Flotte nicht für ein
            // Nullsummenspiel aufs Spiel; gegen Schwache und Reiche ändert sich nichts.
            let lohnend = wert > 2 * verlust + 2000;
            if chance < 75 { zaehle("bericht: siegchance < 75"); }
            else if !lohnend { zaehle("bericht: beute < 2 x verlust"); }
            else { zaehle("bericht: lohnend"); }
            // Bots haben als Ziel Vorrang vor Mitspielern; ohne Mitspieler bleibt die Wahl wie bisher.
            let vorrang = !mitspieler(&bot.opferschutz, b.besitzer);
            if chance >= 75 && lohnend && bestes.as_ref().map(|(v, w, _, _, _)| (vorrang, wert - verlust) > (*v, *w)).unwrap_or(true) {
                bestes = Some((vorrang, wert - verlust, b.ziel, menge, b.clone()));
            }
        }
        if let Some((vorrang, _, ziel, menge, bericht)) = bestes {
            let mensch = !vorrang;
            // Gegen Mitspieler nur so viele Kampfschiffe wie für den sicheren Sieg nötig und höchstens halbe Beute.
            let flotte_ang = if mensch { raubflotte_mitspieler(welt, sid, &bericht, &ang) } else { ang };
            let menge = if mensch { menge / BEUTE_MITSPIELER } else { menge };
            let ladung = r.einh(Einheit::KleinerTransporter).ladung;
            let mit = ((menge + ladung - 1) / ladung).clamp(1, transporter.max(1)).min(transporter);
            let mut schiffe = json!({});
            let mut flotte = [0i64; SCHIFFE];
            for e in ANGRIFFSSCHIFFE.into_iter().filter(|e| flotte_ang[e.idx()] > 0) {
                schiffe[e.name()] = json!(flotte_ang[e.idx()]);
                flotte[e.idx()] = flotte_ang[e.idx()];
            }
            if mit > 0 {
                schiffe["kleiner_transporter"] = json!(mit);
            }
            // Langsamer fliegen, wenn der Treibstoff für volle Fahrt nicht reicht: der Verbrauch fällt quadratisch.
            flotte[Einheit::KleinerTransporter.idx()] = mit;
            let deut = welt.bestand_jetzt(pid)[Gut::Deuterium.idx()];
            let tempo = if mensch { anflug_tempo_mitspieler(welt, sid, pid, ziel, &flotte, deut) } else { [1.0, 0.8, 0.6, 0.4, 0.3].into_iter().find(|s| {
                welt.flugplan(sid, pid, ziel, &flotte, milli(*s)).map(|p| 2 * p.treibstoff <= deut).unwrap_or(false)
            }) };
            if let Some(tempo) = tempo {
                let (ok, _) = welt.handeln(sid, Rolle::Alle, &json!({"typ": "flotte_senden", "start": ks, "ziel": ziel.to_string(), "mission": "angriff",
                    "schiffe": schiffe, "geschwindigkeit": tempo}));
                zaehle(if ok { "räuber: angriff gestartet" } else { "räuber: angriff abgelehnt" });
            } else {
                zaehle("räuber: treibstoff fehlt");
            }
        }
    }

    // Neue Ziele ausspähen: belegte Plätze im eigenen Sektor, die nächsten zuerst.
    let mut sonden = n(welt, Einheit::Spionagesonde);
    let mut kandidaten: Vec<(i64, Koord)> = if welt.aufklaerungsregeln() {
        let unbesetzt: std::collections::BTreeSet<String> = welt.spieler.iter().filter(|s| !welt.spieler_aktiv(s.id)).map(|s| s.name.clone()).collect();
        let aktive: BTreeMap<&str, SpielerId> = welt.spieler.iter().filter(|s| welt.spieler_aktiv(s.id)).map(|s| (s.name.as_str(), s.id)).collect();
        let mut known = Vec::new();
        for system in 1..=r.welt.systeme_je_sektor {
            if (system as i64-k.system as i64).abs()>15 {continue;}
            for position in 1..=r.welt.plaetze_je_system {
                let coord=Koord::neu(k.sektor,system,position);
                let view=welt.planetenwissen(sid,coord);
                if view["status"]=="eigen" || view["status"]=="frei" || view["status"]=="freund" {continue;}
                // Unclaimed start places are public knowledge and frozen: no target, no probes.
                if view["spieler"].as_str().is_some_and(|n|unbesetzt.contains(n)) {continue;}
                // Neulinge nicht mit Spionage überziehen: angreifen darf sie ohnehin niemand (`raub_erlaubt`).
                if view["spieler"].as_str().and_then(|n|aktive.get(n)).is_some_and(|id|neuling(welt,&bot.opferschutz,*id)) {continue;}
                if bot.gespaeht.get(&coord).is_some_and(|t|jetzt-*t<12*STUNDE) {continue;}
                known.push((welt.entfernung(k,coord),coord));
            }
        }
        known
    } else { welt
        .planeten
        .iter()
        .filter(|z| z.besitzer != sid && z.koord.sektor == k.sektor && (z.koord.system as i64 - k.system as i64).abs() <= 15)
        .filter(|z| !geschuetzt(welt, z.besitzer) && !neuling(welt, &bot.opferschutz, z.besitzer) && bot.gespaeht.get(&z.koord).map(|t| jetzt - *t >= 12 * STUNDE).unwrap_or(true))
        .map(|z| (welt.entfernung(k, z.koord), z.koord))
        .collect() };
    kandidaten.sort();
    if kandidaten.is_empty() { zaehle("spähen: keine kandidaten"); }
    for (_, ziel) in kandidaten {
        let mission=if welt.aufklaerungsregeln() && !welt.system_erfasst(sid,ziel) {"system_erkunden"} else {"spionage"};
        let anzahl = if mission == "spionage" { SPAEH_SONDEN } else { 2 };
        if frei(welt) <= 0 || sonden < anzahl {
            zaehle(if sonden < anzahl { "spähen: zu wenig sonden" } else { "spähen: keine flottenplätze" });
            break;
        }
        if welt.flotten.values().any(|f|f.besitzer==sid && f.ziel.sektor==ziel.sektor && f.ziel.system==ziel.system && f.mission==Mission::SystemErkunden) {continue;}
        if welt.flotten.values().any(|f|f.besitzer==sid && f.ziel==ziel && f.mission==Mission::Spionage && f.zustand==Flottenzustand::Hinflug) {continue;}
        let Some(auftrag)=sondenauftrag(welt,sid,pid,ziel,mission,anzahl) else{continue;};
        if tu(welt, sid, auftrag) {
            zaehle(if mission == "spionage" { "spähen: spionage gestartet" } else { "spähen: system erkunden" });
            bot.gespaeht.insert(ziel, jetzt);
            sonden -= anzahl;
        }
    }
}

/// Der Händler verkauft, was sein Lager füllt, und kauft, was ihm fehlt.
fn haendler(welt: &mut Welt, sid: SpielerId) {
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
        // Marktwert wie `markt_handeln`: der Händler verkauft knapp darunter (über dem Erlös des Soforthandels) und
        // kauft zum Marktwert – dort treffen ihn die Überschüsse der anderen Bots.
        let wert = marktwert(welt, g);
        let hat_order = welt.orders.iter().any(|o| o.spieler == sid && o.gut == g);
        if hat_order {
            continue;
        }
        if bestand[i] >= mal(grenze[i], 0.7) {
            let menge = ganz(bestand[i] / 5);
            if menge >= 100 {
                tu(welt, sid, json!({"typ": "markt_order", "planet": k, "gut": g.name(), "seite": "verkauf", "menge": menge, "preis": preis3(wert * 0.99)}));
                return;
            }
        } else if bestand[i] < mal(grenze[i], 0.1) && credits > 1500 * M {
            let preis = preis3(wert);
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
        // Nach einer Absage eine Woche kein neues Angebot an denselben Partner: der Räuber lehnt sofort ab, und im
        // Zwei-Tages-Fenster kam sonst jede Viertelstunde ein neues Angebot (Messlauf 20.10.2026: Vorfälle voller Angebote).
        if welt.vertraege.iter().any(|v| {
            v.art == Vertragsart::Nichtangriffspakt
                && (matches!(v.status, Vertragsstatus::Aktiv | Vertragsstatus::Angeboten)
                    || (v.status == Vertragsstatus::Abgelehnt && v.a == sid && welt.zeit - v.angeboten < 7 * TAG))
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
    // Ablauf v1: Kolonien mit dem meisten Xenokristall zuerst; sonst belegen Überschussfahrten die Flottenplätze und
    // der Xenokristall der Nebelkolonie erreicht die Heimat nie (Messung 17.10.: Syntheten warteten 300 h auf Stufe V).
    // Dyson-Depots (Planeten im Projektsystem) fahren nichts zur Heimat zurück: ihr Bestand gehört dem Projekt.
    let mut kolonien_rueck: Vec<usize> = kolonien.iter().copied().filter(|kp| !dyson_bot::im_projektsystem(welt, sid, *kp)).collect();
    if welt.regeln.ablauf >= 1 {
        kolonien_rueck.sort_by_key(|kp| std::cmp::Reverse(welt.bestand_jetzt(*kp)[Gut::Xenokristall.idx()]));
    }
    // Was die Heimat für ihre nächsten Schritte braucht und nicht hat: die Pflichtforschung (Stufenforschung, Astrophysik
    // für die Kolonien) und ein erlaubtes Kolonieschiff, dessen Bauteile schon bereitliegen. Die Kolonien bringen den
    // Fehlbetrag mit, abzüglich dessen, was schon zur Heimat fliegt. 20.10.2026 klassisch: Igel forschten 60 Tage nichts
    // und blieben bei einer Kolonie – 6.500 von 10.000 Deuterium für das Kolonieschiff zu Hause, 750.000 in der Kolonie.
    let mut heimziel = [0i64; GUETER];
    {
        let sp = &welt.spieler[sid as usize];
        if sp.forschung_aktiv.is_none() {
            if let Some(f) = pflichtforschung(welt, sid) {
                let k = r.kosten_forschung(f, sp.forschung[f.idx()] + 1);
                for g in 0..GUETER { heimziel[g] += k[g]; }
            }
        }
        // Dysonsphäre bis auf die Einzahlung bereit: die Kolonien bringen ihren Überschuss zur Heimat, die ihn zum Depot fährt
        // (Tag 121: die Heimatwelten förderten 9.000–100.000 Erz je Tag, die Wirtschaft steckte in den Kolonien).
        let d = dyson_bot::sparziel(welt, sid);
        for g in 0..GUETER { heimziel[g] += d[g]; }
        // Die nächste Stufe der Krümmer-Doktrin mit der Deckung, die das Einreihen verlangt (bot_monde.rs).
        if let Some((f, st)) = monde_bot::kruemmer_forschung(welt, sid) {
            let k = r.kosten_forschung(f, st);
            for g in 0..GUETER { heimziel[g] += k[g] * 5 / 3; }
        }
        let unterwegs = welt.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren).count();
        let schiff = &r.einh(Einheit::Kolonieschiff).kosten;
        let teile_da = [Gut::Habitatmodul, Gut::Antriebskern].iter()
            .all(|g| welt.bestand_jetzt(heimat)[g.idx()] >= schiff.get(g).copied().unwrap_or(0) * M);
        if sp.stufe >= if welt.ausbau.industrie_version > 0 { 3 } else { 4 } && welt.kolonien(sid) + unterwegs < welt.kolonien_erlaubt(sid)
            && welt.planeten[heimat].einheiten[Einheit::Kolonieschiff.idx()] == 0 && teile_da {
            let k = r.kosten_einheit(Einheit::Kolonieschiff, volk);
            for g in 0..GUETER { heimziel[g] += k[g]; }
        }
    }
    let mut kommt = [0i64; GUETER];
    for fl in welt.flotten.values().filter(|fl| fl.besitzer == sid && fl.ziel == hk && fl.mission == Mission::Transport) {
        for g in 0..GUETER { kommt[g] += fl.ladung[g]; }
    }
    for &kp in &kolonien_rueck {
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
        // Beim Sparen auf den Aufstieg, was der Heimat für die Stufenkosten fehlt (Legierung, Elektronik, Deuterium):
        // 20.10.2026 erfüllten zwölf Bots alle Bedingungen der Stufe V, die Kosten fehlten nur auf der Heimatwelt.
        // Auch Erz und Kristall und auch nach den 20 Spartagen, solange alle Bedingungen erfüllt sind: frische Welt 712, r58,
        // Tag 45–60 – ein Räuber mit allen Bedingungen der Stufe V hatte daheim 949 Kristall, die Kolonie 428.000.
        let mut fuer_aufstieg = false;
        if spart(welt, sid) || aufstieg_bereit(welt, sid) {
            if let Some(regel) = r.stufen.get(welt.spieler[sid as usize].stufe as usize - 1) {
                let kosten = preis_array(&regel.kosten);
                let heim = welt.bestand_jetzt(heimat);
                for g in [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Legierung, Gut::Elektronik] {
                    let fehlt = (kosten[g.idx()] - heim[g.idx()] - kommt[g.idx()]).max(0);
                    if fehlt > 0 {
                        menge[g.idx()] = menge[g.idx()].max(fehlt.min(b[g.idx()] / 2));
                        fuer_aufstieg |= menge[g.idx()] > 0;
                    }
                }
            }
        }
        // Heimziel (Pflichtforschung, Kolonieschiff, siehe oben): der Fehlbetrag, höchstens die halbe Kolonie.
        let heim = welt.bestand_jetzt(heimat);
        for g in [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Legierung, Gut::Elektronik] {
            let fehlt = (heimziel[g.idx()] - heim[g.idx()] - kommt[g.idx()]).max(0);
            if fehlt > 0 {
                menge[g.idx()] = menge[g.idx()].max(fehlt.min(b[g.idx()] / 2));
                fuer_aufstieg |= menge[g.idx()] > 0;
            }
        }
        if menge[Gut::Xenokristall.idx()] < 300 * M && r.wert(&menge) < 15_000 * M && konsum < habitat / 2 && !fuer_aufstieg {
            continue;
        }
        let Some(menge) = ladung_fuer(welt, sid, kp, hk, 1, menge) else {
            continue;
        };
        if tu(welt, sid, json!({"typ": "flotte_senden", "start": k.to_string(), "ziel": hk.to_string(), "mission": "transport",
            "schiffe": {gt.name(): 1}, "ladung": ladung_json(&menge)})) {
            for g in 0..GUETER { kommt[g] += menge[g]; }
        }
    }

    // Nebelkolonie ohne bezahlbaren Xenoextraktor: genau dessen Baukosten von der Heimat, vor allen anderen Fahrten.
    // 20.10.2026: neun Bots besaßen eine Nebelwelt mit Extraktor 0 – eine junge Kolonie hat nie 20.000 Erz,
    // 10.000 Kristall, 500 Legierung und 200 Elektronik zugleich, und ohne Extraktor fehlte Xenokristall für Stufe V.
    if welt.spieler[sid as usize].stufe >= r.geb(Gebaeude::Xenoextraktor).ab_stufe && frei(welt) >= 1 && welt.planeten[heimat].einheiten[gt.idx()] >= 1 {
        let ziel = kolonien.iter().copied().find(|kp| {
            let p = &welt.planeten[*kp];
            welt.xeno_planet(p.koord) && p.blockade.is_none() && p.gebaeude[Gebaeude::Xenoextraktor.idx()] < 4
                && !p.bauschleife.iter().any(|a| a.gebaeude == Gebaeude::Xenoextraktor)
                && !bezahlbar(&welt.bestand_jetzt(*kp), &r.kosten_gebaeude(Gebaeude::Xenoextraktor, p.gebaeude[Gebaeude::Xenoextraktor.idx()] + 1))
                && !welt.flotten.values().any(|f| f.besitzer == sid && f.ziel == p.koord && f.zustand == Flottenzustand::Hinflug)
        });
        if let Some(kp) = ziel {
            let p = &welt.planeten[kp];
            let kosten = r.kosten_gebaeude(Gebaeude::Xenoextraktor, p.gebaeude[Gebaeude::Xenoextraktor.idx()] + 1);
            let dort = welt.bestand_jetzt(kp);
            let heim = welt.bestand_jetzt(heimat);
            let mut menge = [0i64; GUETER];
            for g in [Gut::Erz, Gut::Kristall, Gut::Legierung, Gut::Elektronik] {
                let fehlt = (mal(kosten[g.idx()], 1.1) - dort[g.idx()]).max(0);
                menge[g.idx()] = fehlt.min(heim[g.idx()] / 2);
            }
            let ziel = p.koord;
            let anzahl = welt.planeten[heimat].einheiten[gt.idx()].min(2);
            if menge.iter().any(|m| *m > 0) {
                if let Some(menge) = ladung_fuer(welt, sid, heimat, ziel, anzahl, menge) {
                    if tu(welt, sid, json!({"typ": "flotte_senden", "start": hk.to_string(), "ziel": ziel.to_string(), "mission": "transport",
                        "schiffe": {gt.name(): anzahl}, "ladung": ladung_json(&menge)})) {
                        zaehle("logistik: baustoffe für xenoextraktor");
                        return;
                    }
                }
            }
        }
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
    // Ablauf v1: eine junge Kolonie ohne eigene Förderung (unter 30 belegten Feldern) darf den letzten freien
    // Flottenplatz nutzen; sonst belegen Rücktransporte alle Plätze und sie bekommt nie Baustoffe
    // (Messung 17.10.: Krath-Kolonien blieben bei 5 Feldern stehen).
    let jung = welt.regeln.ablauf >= 1 && welt.felder_belegt(kp) < 30;
    if frei(welt) <= if jung { 0 } else { 1 } || transporter < 1 {
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

/// Sammelsystem der Allianz (aus dem Allianzchat), solange dort ein Platz frei ist und dieses Reich hinsoll: ohne eigenen
/// Planeten dort immer, mit einem nur, wenn mehr Plätze frei sind, als Mitglieder ohne Planet dort nachkommen können. Die
/// Dysonsphäre braucht alle Plätze des Systems in Allianzhand und zehn Mitglieder mit Planet im System.
fn sammel_offen(welt: &Welt, sid: SpielerId) -> Option<(u8, u8)> {
    let aid = welt.spieler[sid as usize].allianz?;
    let (s, y) = allianz_bot::sammelsystem(welt, aid)?;
    let frei = (1..=welt.regeln.welt.plaetze_je_system).filter(|pos| !welt.belegung.contains_key(&Koord::neu(s, y, *pos))).count();
    if frei == 0 { return None; }
    let dort = |m: SpielerId| welt.spieler[m as usize].planeten.iter().filter(|x| { let q = welt.planeten[**x as usize].koord; q.sektor == s && q.system == y }).count();
    if dort(sid) == 0 { return Some((s, y)); }
    // Eine zweite Kolonie dort nur in einer Allianz, die die Mindestzahl der Projektmitglieder erreichen kann, und höchstens
    // zwei je Reich: sonst füllte in einer kleinen Allianz die Leitung das System allein (Test krieg_botlauf: sieben Kolonien).
    let al = welt.allianzen.iter().find(|a| a.id == aid)?;
    let min = welt.dyson_regeln().map(|r| r.mitglieder_min as usize).unwrap_or(10);
    if al.mitglieder.len() < min || dort(sid) >= 2 { return None; }
    let ohne = al.mitglieder.iter().filter(|m| **m != sid && dort(**m) == 0).count();
    (frei > ohne).then_some((s, y))
}

/// Ziele, an denen eine eigene Gründung in der letzten Woche scheiterte (aus den eigenen Vorfällen des Kerns).
fn kolonie_gescheitert(welt: &Welt, sid: SpielerId) -> std::collections::BTreeSet<Koord> {
    welt.spieler[sid as usize].vorfaelle.iter().rev()
        .take_while(|v| welt.zeit - v.zeit < KOLONIE_SPAEHPAUSE)
        .filter(|v| v.art == "kolonie" && ((v.text.starts_with("Kolonisierung von ") && v.text.contains("gescheitert"))
            || (v.text.starts_with("Gründung auf ") && v.text.contains("nicht möglich"))))
        .filter_map(|v| v.text.split_whitespace().nth(2).and_then(|k| k.parse().ok()))
        .collect()
}

/// Build, survey and settle using the faction climate and actual shared colony requirements.
/// `gespaeht`: the bot's probe memory. Under the reconnaissance rules a probed target that is
/// not confirmed free is skipped for a week: the report alone does not last (a bot keeps
/// only its latest 30 reports), and without this memory bots probed the same occupied or
/// reserved home every few minutes, flooding its owner with "Spionage bemerkt".
fn kolonisieren(welt: &mut Welt, sid: SpielerId, gespaeht: &mut BTreeMap<Koord, SimZeit>) {
    let r = welt.regeln.clone();
    let sp = &welt.spieler[sid as usize];
    if sp.stufe < if welt.ausbau.industrie_version>0{3}else{4} {
        zaehle("kolonie: stufe zu klein");
        return;
    }
    let pid = sp.heimat as usize;
    let p = &welt.planeten[pid];
    let k = p.koord;
    let ks = k.to_string();
    let unterwegs = welt.flotten.values().filter(|f| f.besitzer == sid && f.mission == Mission::Kolonisieren).count();
    if welt.kolonien(sid) + unterwegs >= welt.kolonien_erlaubt(sid) {
        zaehle(if welt.kolonien_erlaubt(sid) == 0 { "kolonie: astrophysik fehlt" } else { "kolonie: alle erlaubten da" });
        // Das Sammelsystem der Allianz braucht noch eine Kolonie dieses Reichs: Astrophysik hinter der laufenden Forschung
        // einreihen (bot_lage 712, 60 Tage: 134.227-mal „alle erlaubten da“, die Dyson-Projekte blieben bei 5 bzw. 4 von 10).
        let sp = &welt.spieler[sid as usize];
        let kf = welt.kolonie_forschung();
        if sammel_offen(welt, sid).is_some() && sp.forschung_schlange.is_empty()
            && sp.forschung_aktiv.as_ref().is_none_or(|a| a.forschung != kf)
            && (sp.forschung[kf.idx()] as usize) < r.wirtschaft.kolonien_max as usize
            && welt.planeten[pid].gebaeude[Gebaeude::Labor.idx()] >= r.forsch(kf).labor
            && tu(welt, sid, json!({"typ":"forschen","forschung":kf.name()})) {
            zaehle("kolonie: astrophysik fürs sammelsystem");
        }
        return;
    }
    if p.gebaeude[Gebaeude::Orbitalwerft.idx()] < 1 && (welt.ausbau.industrie_version==0||p.gebaeude[Gebaeude::Werft.idx()]<3) {
        zaehle("kolonie: werft < 3");
        return;
    }
    let bestand = welt.bestand_jetzt(pid);
    if p.einheiten[Einheit::Kolonieschiff.idx()] >= 1 {
        if welt.flotten.values().filter(|f| f.besitzer == sid).count() >= welt.flottenplaetze(sid) {
            return;
        }
        // Ziel: freier Platz der Lebenszone, zuerst im nächsten Nebelsystem, sonst so nah wie möglich.
        let in_nebel = sp.planeten.iter().any(|x| welt.xeno_planet(welt.planeten[*x as usize].koord));
        // Sammelsystem der Allianz (aus dem Allianzchat): eine Kolonie dort, solange noch keine eigene dort liegt.
        let sammel = sammel_offen(welt, sid);
        // Wo eine Gründung in der letzten Woche scheiterte (inzwischen besetzt, Klima oder Fracht abgelehnt), nicht
        // wieder hin: sonst pendelte das Kolonieschiff zwischen Heimat und demselben Ziel (Startwert 2026: ein Räuber
        // flog bis Tag 30 immer wieder dieselben zwei Plätze an und blieb ohne Kolonie).
        let gescheitert = kolonie_gescheitert(welt, sid);
        let lz = &r.zonen[&Zone::Leben];
        let mut bestes: Option<(i64, Koord)> = None;
        // Die Systeme des Heimatsektors und das Sammelsystem der Allianz, auch wenn es im anderen Sektor liegt (die Leitung
        // verkündet es in ihrem Heimatsektor; Mitglieder von dort drüben konnten es vorher nie anfliegen).
        let mut systeme: Vec<(u8, u8)> = (1..=r.welt.systeme_je_sektor).map(|n| (k.sektor, n)).collect();
        if let Some(sy) = sammel.filter(|(s, _)| *s != k.sektor) { systeme.push(sy); }
        for (sek, n) in systeme {
            let observed= !welt.aufklaerungsregeln() || welt.system_erfasst(sid,Koord::neu(sek,n,1));
            let nebel = observed && welt.system(Koord::neu(sek, n, 1)).map(|s| s.nebel).unwrap_or(false);
            let (von,bis)=if welt.aufklaerungsregeln() || welt.ausbau.version>0 {(1,r.welt.plaetze_je_system)}else{(lz.von,lz.bis)};
            for pos in von..=bis {
                let z = Koord::neu(sek, n, pos);
                let climate=welt.planet_eignung(sid,z,sp.volk);
                // Klimafaktor relativ zum Optimum der Tabelle (Version 0: 2000, Ablauf v1: 1000): mindestens 40 Prozent.
                let opt=kern::habitabilitaet::optimum(welt.regeln.ablauf);
                if welt.ausbau.version>0 && (!climate.bewohnbar||climate.wachstum_promille*5<opt*2) {continue;}
                if welt.flotten.values().any(|f|f.besitzer==sid&&f.mission==Mission::Kolonisieren&&f.ziel==z) || gescheitert.contains(&z) {continue;}
                if welt.aufklaerungsregeln() && !sp.erkundet.contains_key(&z) && gespaeht.get(&z).is_some_and(|t| welt.zeit - *t < KOLONIE_SPAEHPAUSE) {continue;}
                if if welt.aufklaerungsregeln() {
                    let status=welt.planetenwissen(sid,z)["status"].as_str().unwrap_or("unbekannt").to_string();
                    !matches!(status.as_str(),"frei"|"unbekannt")
                } else {welt.belegung.contains_key(&z)} {
                    continue;
                }
                // Climate follows public orbit/faction rules; deposits only count after reconnaissance.
                let xeno=nebel && (welt.ausbau.industrie_version==0||pos%2==0);
                // Die zweite Kolonie schon auf Stufe III ins Nebelsystem: natürlicher Xenokristall für Stufe V.
                let expansion=welt.kolonien(sid)>0 && !in_nebel && sp.stufe>=3;
                // Ablauf v1 (flache Klimatabelle): Planetengröße zählt mit, sonst siedeln Krath nur auf kleinen
                // Glutwelten (80–140 Felder) statt auf 0,95er Lebenswelten (150–230). Unerkundet: Zonenmittel.
                let groesse = if welt.regeln.ablauf>=1 {
                    let felder = sp.erkundet.get(&z).map(|e| e.felder as i64).unwrap_or_else(|| {
                        let zr = &r.zonen[&kern::habitabilitaet::zone(pos, r.welt.plaetze_je_system)];
                        (zr.felder_min as i64 + zr.felder_max as i64) / 2
                    });
                    felder * 300
                } else {0};
                let d = welt.entfernung(k,z) + if welt.ausbau.version>0 {(opt-climate.wachstum_promille).max(0)*200*2000/opt} else {0}
                    - if expansion&&xeno {150_000} else {0} - groesse
                    - if sammel == Some((z.sektor, z.system)) {400_000} else {0};
                if bestes.map(|(b, _)| d < b).unwrap_or(true) {
                    bestes = Some((d, z));
                }
            }
        }
        if bestes.is_none() { zaehle("kolonie: kein ziel"); }
        if let Some((_, ziel)) = bestes {
            if welt.aufklaerungsregeln() && !welt.system_erfasst(sid,ziel) {
                zaehle("kolonie: system erkunden");
                if !welt.flotten.values().any(|f| f.besitzer==sid && f.mission==Mission::SystemErkunden && f.ziel.sektor==ziel.sektor && f.ziel.system==ziel.system) {
                    if p.einheiten[Einheit::Spionagesonde.idx()]>0 {
                        if let Some(a)=sondenauftrag(welt,sid,pid,ziel,"system_erkunden",1){tu(welt,sid,a);}
                    } else if sonden_geplant(welt,sid,pid)==0 && bezahlbar(&bestand,&r.kosten_einheit(Einheit::Spionagesonde,sp.volk)) {
                        tu(welt,sid,json!({"typ":"fertigen","planet":ks,"einheit":"spionagesonde","anzahl":1}));
                    }
                }
                return;
            }
            if welt.kolonisation.aktiv {
                if !sp.erkundet.contains_key(&ziel) {
                    zaehle("kolonie: ziel ausspähen");
                    if !welt.flotten.values().any(|f| f.besitzer == sid && f.mission == Mission::Spionage && f.ziel == ziel) {
                        if p.einheiten[Einheit::Spionagesonde.idx()] > 0 {
                            if sondenauftrag(welt,sid,pid,ziel,"spionage",1).is_some_and(|a|tu(welt,sid,a)) {
                                gespaeht.insert(ziel, welt.zeit);
                            }
                        } else if sonden_geplant(welt,sid,pid)==0 && bezahlbar(&bestand,&r.kosten_einheit(Einheit::Spionagesonde,sp.volk)) {
                            tu(welt, sid, json!({"typ":"fertigen","planet":ks,"einheit":"spionagesonde","anzahl":1}));
                        }
                    }
                    return;
                }
                let escort = Einheit::ALLE[..SCHIFFE].iter().copied().find(|e| *e != Einheit::Spionagesonde && *e != Einheit::Kolonieschiff && p.einheiten[e.idx()] > 0 && r.einh(*e).angriff > 0);
                let Some(escort) = escort else {
                    zaehle("kolonie: begleitschiff fehlt");
                    if !p.fertigung[0].iter().any(|f| f.produkt == Produkt::Einheit(Einheit::LeichterJaeger)) && bezahlbar(&bestand,&r.kosten_einheit(Einheit::LeichterJaeger,sp.volk)) {
                        tu(welt, sid, json!({"typ":"fertigen","planet":ks,"einheit":"leichter_jaeger","anzahl":1}));
                    }
                    return;
                };
                let required = if welt.ausbau.version>0 {welt.koloniebedarf(sid,Einheit::Kolonieschiff,sp.volk).unwrap()} else {welt.koloniefracht(sid)};
                if required.iter().enumerate().any(|(i,n)| bestand[i] < *n) { zaehle("kolonie: fracht fehlt"); return; }
                let cargo: serde_json::Map<String, serde_json::Value> = Gut::ALLE.iter().filter(|g| required[g.idx()] > 0)
                    .map(|g| (g.name().to_string(), json!((required[g.idx()] + M - 1) / M))).collect();
                let settlers=r.flottendoktrin.get(&Einheit::Kolonieschiff).map(|d|d.siedler).unwrap_or(r.wirtschaft.siedler)*M;
                if p.bevoelkerung<settlers+1000*M {zaehle("kolonie: siedler fehlen");return;}
                if welt.ausbau.version>0 && welt.ausbau.personen.get(&(pid as PlanetId)).map(|p|p.auswahl_volk(sp.volk,settlers).gesamt()).unwrap_or(0)<settlers{zaehle("kolonie: siedler fehlen");return;}
                let mut fleet=[0;SCHIFFE];fleet[Einheit::Kolonieschiff.idx()]=1;fleet[escort.idx()]=1;
                let Ok(plan)=welt.flugplan(sid,pid,ziel,&fleet,1000) else{return;};
                if bestand[Gut::Deuterium.idx()]<required[Gut::Deuterium.idx()]+plan.treibstoff||plan.kapazitaet<required.iter().sum(){zaehle("kolonie: treibstoff/ladung");return;}
                let mut ships = serde_json::Map::new();
                ships.insert("kolonieschiff".into(), json!(1));
                ships.insert(escort.name().into(), json!(1));
                zaehle(if tu(welt, sid, json!({"typ":"flotte_senden","start":ks,"ziel":ziel.to_string(),"mission":"kolonisieren","schiffe":ships,"ladung":cargo})) {"kolonie: gestartet"} else {"kolonie: start abgelehnt"});
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
    if welt.ausbau.industrie_version>0 && p.gebaeude[Gebaeude::Orbitalwerft.idx()]==0 && bestand[Gut::Habitatmodul.idx()]<M {
        zaehle(if p.einheiten[Einheit::Recycler.idx()]==0 {"kolonie: recycler fürs habitatmodul"} else {"kolonie: habitatmodul"});
        if p.fertigung[0].is_empty(){
            if p.einheiten[Einheit::Recycler.idx()]==0 {if bezahlbar(&bestand,&r.kosten_einheit(Einheit::Recycler,sp.volk)){tu(welt,sid,json!({"typ":"fertigen","planet":ks,"einheit":"recycler","anzahl":1}));}}
            else if r.kosten_bauteil(Gut::Habitatmodul).is_some_and(|c|bezahlbar(&bestand,&c)){tu(welt,sid,json!({"typ":"fertigen","planet":ks,"bauteil":"habitatmodul","anzahl":1}));}
        }
        return;
    }
    let bestellt = |produkt: Produkt, schleife: usize| -> i64 { p.fertigung[schleife].iter().filter(|f| f.produkt == produkt).map(|f| f.rest).sum() };
    if bestellt(Produkt::Einheit(Einheit::Kolonieschiff), 0) > 0 {
        zaehle("kolonie: schiff in fertigung");
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
                let n=if welt.ausbau.industrie_version>0&&g==Gut::Habitatmodul&&welt.planeten[pid].gebaeude[Gebaeude::Orbitalwerft.idx()]==0{n.min(welt.planeten[pid].einheiten[Einheit::Recycler.idx()])}else{n};
                if n >= 1 && tu(welt, sid, json!({"typ": "fertigen", "planet": ks, "bauteil": g.name(), "anzahl": n})) {
                    for x in 0..GUETER {
                        rest[x] -= kosten[x] * n;
                    }
                }
            }
        }
    }
    if genug && bezahlbar(&welt.bestand_jetzt(pid), &r.kosten_einheit(Einheit::Kolonieschiff, volk)) && bevoelkerung > (r.flottendoktrin.get(&Einheit::Kolonieschiff).map(|p|p.siedler).unwrap_or(r.wirtschaft.siedler) + 1500) * M {
        zaehle(if tu(welt, sid, json!({"typ": "fertigen", "planet": ks, "einheit": "kolonieschiff", "anzahl": 1})) {"kolonie: schiff bestellt"} else {"kolonie: schiff abgelehnt"});
    } else {
        zaehle(if !genug {"kolonie: bauteile fehlen"} else if bevoelkerung <= (r.flottendoktrin.get(&Einheit::Kolonieschiff).map(|p|p.siedler).unwrap_or(r.wirtschaft.siedler) + 1500) * M {"kolonie: zu wenig einwohner fürs schiff"} else {"kolonie: schiff nicht bezahlbar"});
    }
}

/// A small mixed garrison uses the same paid training actions as human players.
fn bodensicherung(w:&mut Welt,sid:SpielerId){
 if w.ausbau.industrie_version==0{return;}
 for id in w.spieler[sid as usize].planeten.clone(){let pid=id as usize;let p=&w.planeten[pid];if p.bevoelkerung<2000*M||w.werk_ruht(pid,Gebaeude::Kaserne){continue;}
  let stock=w.bestand_jetzt(pid);let existing=w.ausbau.boden.bestand.get(&id).copied().unwrap_or_default();let queue=w.ausbau.boden.auftraege.get(&id);if queue.is_some_and(|q|q.len()>=8){continue;}
  let fp_alle:Vec<i64>=(0..kern::boden::ANZAHL).map(|i|w.regeln.kw().boden.fertigungsplaetze(i)).collect();let fp=&fp_alle;
  let used=queue.map(|q|q.iter().flat_map(|a|a.lose.iter().map(move|b|b.anzahl*fp[a.art])).sum::<i64>()).unwrap_or(0);
  for(i,target)in [(0,12),(2,4),(1,3),(4,3),(9,1),(11,1),(8,2)]{
   if p.gebaeude[Gebaeude::Kaserne.idx()]<w.regeln.kw().boden.kaserne(i)||w.spieler[sid as usize].stufe<w.regeln.kw().boden.ab_stufe(i)||p.gebaeude[Gebaeude::Kaserne.idx()] as i64-used<fp[i]{continue;}
   let pending=queue.map(|q|q.iter().filter(|a|a.art==i).map(|a|a.rest).sum::<i64>()).unwrap_or(0);if existing[i]+pending>=target{continue;}
   let cost=w.produktionskosten(pid,Gebaeude::Kaserne,w.regeln.kw().boden.kosten(i));if !bezahlbar(&stock,&cost.map(|n|n*3)){continue;}
   tu(w,sid,json!({"typ":"boden_bauen","planet":p.koord.to_string(),"art":kern::boden::NAMEN[i],"anzahl":1}));break;
  }
 }
}

/// Ein Zug eines Bots im aktuellen Fenster.
pub fn zug(welt: &mut Welt, sid: SpielerId, bot: &mut Bot) {
    zug_mit(welt, sid, bot, true)
}

/// Wie [`zug`]; mit `ausbilden = false` spielt der Bot wie ein Neuling, der weder Ausbildung bucht noch
/// Arbeitsprioritäten setzt (Messung der Ablauflogik ohne Spielerwissen, `examples/ablauf_messung.rs`).
pub fn zug_mit(welt: &mut Welt, sid: SpielerId, bot: &mut Bot, ausbilden: bool) {
    // A thirty-minute occupation cannot wait for the normal two-hour economy cycle.
    // Invasion v1: Miliz ausrufen, Nachschub und Bodenhilfe für Verbündete (bot_invasion.rs).
    invasion_bot::reaktion(welt, sid, bot.typ);
    if !welt.invasion_v1()&&!welt.kolonisation.besetzungen.is_empty()&&welt.spieler_aktiv(sid)&&bot.typ!=Bottyp::Wehrlos&&welt.ausbau.industrie_version>0{
        let threatened=welt.kolonisation.besetzungen.iter().filter(|(id,b)|b.verteidiger==sid&&welt.ausbau.boden.miliz.get(id).is_none_or(|m|m.personen.gesamt()==0)).map(|(id,_)|*id).collect::<Vec<_>>();
        for id in threatened{let pid=id as usize;let m=welt.miliz_sicht(pid);
            if m["belagert"]==true&&m["aktiv"]==0{let stock=welt.bestand_jetzt(pid);let n=m["verfuegbar"].as_i64().unwrap_or(0).min(1000).min(stock[Gut::Legierung.idx()]/(5*M)).min(stock[Gut::Elektronik.idx()]/M);
                if n>0{tu(welt,sid,json!({"typ":"miliz_aufrufen","planet":welt.planeten[pid].koord.to_string(),"anzahl":n}));}
            }
        }
    }
    if welt.zeit < bot.naechster || welt.beendet() || !welt.spieler_aktiv(sid) {
        return;
    }
    bot.naechster = welt.zeit + TAKT;
    allgemein(welt, sid);
    last_ausgleichen(welt,sid);
    if ausbilden {industrie_personal(welt,sid);}
    solarsatelliten(welt, sid);
    for pid in welt.spieler[sid as usize].planeten.clone() {
        bauen(welt, sid, pid as usize, bot.typ);
    }
    forschen(welt, sid, bot.typ);
    // Dysonsphäre (nur Dyson-Welten): Vorschlag, Beitritt, Einzahlung, Baubeginn, Anschlüsse (bot_dyson.rs).
    dyson_bot::zug(welt, sid, bot.typ, TAKT);
    if bot.typ!=Bottyp::Wehrlos{bodensicherung(welt,sid);}
    if bot.typ==Bottyp::Raeuber{invasion_bot::angriff(welt,sid,&bot.opferschutz);}
    // Kolonisieren vor Flotten- und Verteidigungskäufen: Recycler, Habitatmodul und Kolonieschiff brauchen eine freie
    // Werft, und Räuber füllten sie in jedem Zug zuerst mit Jägern (Startwert 2026: ein Räuber ohne Kolonie bis Tag 30).
    // Probe memory older than the longest pause (a week) no longer decides anything.
    let jetzt = welt.zeit;
    bot.gespaeht.retain(|_, t| jetzt - *t < KOLONIE_SPAEHPAUSE);
    nebel_erkunden(welt, sid);
    kolonisieren(welt, sid, &mut bot.gespaeht);
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
    krieg_bot::zug(welt, sid, bot.typ, &bot.opferschutz);
    // Abbau nach Kolonisieren, Räuber und Krieg, aber vor Logistik und Markt: die Logistik fliegt mit zwei Plätzen so lange,
    // wie welche frei sind, und ließe Recyclern und Bergbauschiffen keinen (bot_lage 20.10.2026: 11.000-mal „ohne Flottenplatz“).
    abbau_bot::zug(welt, sid, bot.typ);
    // Dyson-Depot: Einzahlen und Nachschub von der Heimat zum Planeten im System, jeden Takt (bot_dyson.rs).
    dyson_bot::nachschub(welt, sid, bot.typ);
    logistik(welt, sid, bot.typ);
    markt_handeln(welt, sid, bot.typ);
    soforthandel_nutzen(welt, sid, bot.typ);
    dialog::helfen(welt,sid,bot.typ);
    diplomatie(welt, sid, bot.typ);
    allianz_bot::zug(welt, sid, bot.typ);
    monde_bot::zug(welt, sid, bot.typ, &bot.opferschutz);
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
        // Abgelehnt: im selben Fenster und in der folgenden Woche kein neues Angebot an denselben Partner.
        let id=w.vertraege.iter().find(|v|v.a==100&&v.b==101).unwrap().id;
        assert!(w.handeln(101,Rolle::Alle,&json!({"typ":"vertrag_ablehnen","vertrag":id})).0);
        let vorher=w.vertraege.iter().filter(|v|v.a==100&&v.b==101).count();
        w.zeit+=15*60;diplomatie(&mut w,100,Bottyp::Haendler);
        w.zeit=4*TAG;diplomatie(&mut w,100,Bottyp::Haendler);
        assert_eq!(w.vertraege.iter().filter(|v|v.a==100&&v.b==101).count(),vorher,"nach einer Absage eine Woche kein neues Angebot");
        w.zeit=10*TAG;diplomatie(&mut w,100,Bottyp::Haendler);
        assert!(w.vertraege.iter().any(|v|v.a==100&&v.b==101&&v.status==Vertragsstatus::Angeboten),"nach einer Woche wieder");
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
    // Host runtimes disable `ki` and supply their authoritative registry separately.
    // Inferring an empty registry there would turn bot answers into endless reply chains.
    if w.spieler[sid as usize].ki {reagieren(w, sid, typ);}
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
    for id in w.dialogantworten(sid).into_iter().filter(|id|!scripts.contains(&w.nachrichten[*id as usize-1].von)).take(4).collect::<Vec<_>>() {
        let n=w.nachrichten[id as usize-1].clone();
        let betreff=w.kommunikation.briefe.get(&id).map(|b|b.betreff.clone()).unwrap_or_default();
        if scripts.contains(&n.von) {continue;}
        let detail=dialog::antwort(w,sid,n.von,&format!("{betreff} {}",n.text),typ);
        let (an,ich)=(w.spieler[n.von as usize].name.clone(),w.spieler[sid as usize].name.clone());
        tu(w,sid,json!({"typ":"brief_senden","kanal":"privat","an":an,"betreff":"Antwort","text":format!("Guten Tag, {an}. Ich bin {ich}. {detail}"),"antwort_auf":id}));
    }
    allianz_reagieren_typ(w,sid,scripts,typ);
    // Handel: hat ein Gesprächspartner die zum Angebot passende Order eingestellt, bedient der Bot sie sofort
    // (Aushandlung per Brief, bot_dialog.rs `handel`); sonst im nächsten Zug (`dialog::helfen`).
    dialog::markt_bedienen(w,sid,typ,scripts);
    let requests=w.kommunikation.anfragen.iter().filter(|a|a.zielallianz.is_none()&&a.an==sid&&a.status=="offen"&&a.vertrag.is_none_or(|id|w.vertraege.iter().any(|v|v.id==id&&v.status==Vertragsstatus::Angeboten))).map(|a|(a.id,a.art.clone())).take(4).collect::<Vec<_>>();
    // Mehrere Allianzeinladungen: die passendste annehmen (Größe, Beziehung zu den Mitgliedern), die übrigen ablehnen.
    let beste_einladung=requests.iter().filter(|(_,art)|art=="allianz")
        .filter_map(|(id,_)|w.kommunikation.anfragen.iter().find(|a|a.id==*id).and_then(|a|a.allianz).map(|aid|(allianz_bot::allianz_passung(w,sid,aid),*id)))
        .filter(|(p,_)|*p>=0).max().map(|(_,id)|id);
    for (id,art) in requests {
        let accept=typ!=Bottyp::Raeuber && (art!="allianz"||(w.spieler[sid as usize].allianz.is_none()&&beste_einladung==Some(id)));
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
    allianz_reagieren_typ(w,sid,scripts,Bottyp::Oekonom);
}
fn allianz_reagieren_typ(w:&mut Welt,sid:SpielerId,scripts:&std::collections::BTreeSet<SpielerId>,typ:Bottyp) {
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
                    let n=w.nachrichten[id as usize-1].clone();let sender=w.spieler[n.von as usize].name.clone();
                    let text=dialog::antwort(w,sid,n.von,&n.text,typ);
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
    // Gründen, Einladen und Wechseln stehen in bot_allianz.rs (nach Beziehungen, nicht nach Spielernummer).
    if w.spieler[sid as usize].allianz.is_none() && typ!=Bottyp::Raeuber {
        let invitation=w.allianzen.iter().filter(|a|a.eingeladen.contains(&sid)).map(|a|(allianz_bot::allianz_passung(w,sid,a.id),a.name.clone())).max().map(|(_,n)|n);
        if let Some(name)=invitation {tu(w,sid,json!({"typ":"allianz_beitreten","allianz":name}));}
    }
    let partner=(0..w.spieler.len() as u16).cycle().skip(sid as usize+1).take(w.spieler.len()-1).find(|s|w.spieler_aktiv(*s));
    if let Some(p)=partner {
        if let Some(aid)=w.spieler[sid as usize].allianz {
            // Allianzen v2 (bis 50): Bots laden nur bis bot_allianz_max ein (alte Welten unverändert).
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
            if let Some(id)=help {dialog::hilferuf(w,sid,id,typ);}
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

#[cfg(test)]
#[path = "bot_strategy_tests.rs"]
mod strategy_tests;
