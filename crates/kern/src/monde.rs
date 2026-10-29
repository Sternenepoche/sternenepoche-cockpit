//! Mondgeneration (STERNE13): Monde aus großen Schlachten, Mondgebäude und Mondökonomie,
//! Tiefenphalanx, Sprungtor mit öffentlichem Hyperraumloch, Weltenkrümmer und Solarsonden.
//!
//! Konzept: `docs/MONDKONZEPT.md`. Alle Würfe laufen über den eigenen Zufallskanal
//! `KANAL_MOND` und hängen an der Kampfnummer; bestehende Kampfprotokolle bleiben bitgleich.
//! Der Zustand liegt außerhalb des historischen bincode-Layouts in der STERNE13-Hülle.

use crate::flotte::{Flugauftrag, Partei};
use crate::regeln::{preis_array, Preis};
use crate::typen::*;
use crate::welt::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub type MondId = u32;
pub const KANAL_MOND: u64 = 6;
pub const REGELN_RON: &str = include_str!("../../../regeln/monde.ron");

macro_rules! namen_enum {
    ($name:ident, $anz:ident, [$($v:ident = $s:tt),+ $(,)?]) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename = $s)] $v),+ }
        pub const $anz: usize = [$($name::$v),+].len();
        impl $name {
            pub const ALLE: [$name; $anz] = [$($name::$v),+];
            pub fn idx(self) -> usize { self as usize }
            pub fn name(self) -> &'static str { match self { $($name::$v => $s),+ } }
            pub fn aus_name(s: &str) -> Option<$name> { match s { $($s => Some($name::$v),)+ _ => None } }
        }
    };
}

namen_enum!(Mondtyp, MONDTYPEN, [
    KraterGrau = "krater_grau", BasaltDunkel = "basalt_dunkel", EisRissig = "eis_rissig",
    MineralBlau = "mineral_blau", OrganischGruen = "organisch_gruen", RostStaub = "rost_staub",
    SchluchtTief = "schluucht_tief", VulkanGlut = "vulkan_glut", AmethystViolett = "amethyst_violett",
    KarminRot = "karmin_rot",
]);

namen_enum!(MondGebaeude, MONDGEBAEUDE, [
    Mondbasis = "mondbasis", Tiefenphalanx = "tiefenphalanx", Sprungtor = "sprungtor",
    Regolithfoerderung = "regolithfoerderung", Kryolager = "kryolager",
    Werftdock = "werftdock", Ortungsschleier = "ortungsschleier",
]);

// ---------------------------------------------------------------- Regeln

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MondtypRegel {
    pub glut: u32,
    pub leben: u32,
    pub frost: u32,
    #[serde(default)] pub regolith_erz: f64,
    #[serde(default)] pub regolith_kristall: f64,
    #[serde(default)] pub zerstoerung_mod: f64,
    #[serde(default)] pub solar_mod: f64,
    #[serde(default)] pub kryolager_mod: f64,
    #[serde(default)] pub deuterium_spur: f64,
    #[serde(default)] pub nahrung_je_stufe: f64,
    #[serde(default)] pub unterhalt_mod: f64,
    #[serde(default)] pub spionage_mod: i64,
    #[serde(default)] pub felder_bonus: f64,
    #[serde(default)] pub grundenergie_mod: f64,
    #[serde(default)] pub xeno_spur: f64,
    #[serde(default)] pub xeno_spur_nebel: f64,
    #[serde(default)] pub phalanx_stufe: u8,
    #[serde(default)] pub scan_mod: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MondGebaeudeRegel {
    pub kosten: Preis,
    pub faktor: f64,
    #[serde(default)] pub energie: f64,
    pub ab_stufe: u8,
    /// 0: unbegrenzt (bis zur Felderobergrenze).
    #[serde(default)] pub max_stufe: u8,
    #[serde(default)] pub braucht: BTreeMap<MondGebaeude, u8>,
    #[serde(default)] pub forschung: BTreeMap<Forschung, u8>,
    #[serde(default)] pub ertrag: f64,
    #[serde(default)] pub ertrag_erz: f64,
    #[serde(default)] pub ertrag_kristall: f64,
    pub wirkung: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhalanxRegel { pub scan_deuterium: i64, pub abklingzeit_sekunden: i64, pub reichweite_teiler: i64, pub berichte_max: usize }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LochRegel {
    pub offen_sekunden: i64,
    pub abklingzeit_basis_minuten: i64,
    pub abklingzeit_min_minuten: i64,
    pub abklingzeit_je_stufe: f64,
    pub reichweite_je_stufe: i64,
    pub sektorwechsel_ab_stufe: u8,
    pub sektorwechsel_aufschlag: i64,
    pub deuterium_basis: i64,
    pub deuterium_je_system: i64,
    pub kapazitaet_je_stufe: i64,
    pub gebuehr_anteil: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZerstoerungRegel { pub fallout_lager_anteil: f64, pub fallout_gebaeude_anteil: f64, pub fallout_malus: f64, pub fallout_stunden: i64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MondRegeln {
    pub version: u32,
    pub truemmer_je_prozent: i64,
    pub chance_max_prozent: u8,
    pub truemmer_groesse_max: i64,
    pub groesse_x_min: u8,
    pub groesse_x_max: u8,
    pub durchmesser_max: i64,
    pub grundfelder: u16,
    pub felder_je_mondbasis: u16,
    pub nebel_amethyst_faktor: u32,
    pub typen: BTreeMap<Mondtyp, MondtypRegel>,
    pub grundenergie_je_mondbasis: f64,
    pub lager_anteil: f64,
    pub kryolager_basis: i64,
    pub kryolager_faktor: f64,
    pub kryolager_geschuetzt: f64,
    pub faehre_je_mondbasis_stunde: i64,
    pub unterhalt_basis_tag: i64,
    pub unterhalt_je_stufe_tag: i64,
    pub werftdock_zeitaufschlag: f64,
    pub weltentruemmer_werftdock: u8,
    pub reparatur_je_stufe: f64,
    pub reparatur_max: f64,
    pub reparatur_kosten_anteil: f64,
    pub reparatur_zeit_anteil: f64,
    pub solarsonde_energie: f64,
    pub gebaeude: BTreeMap<MondGebaeude, MondGebaeudeRegel>,
    pub tiefenphalanx: PhalanxRegel,
    pub hyperraumloch: LochRegel,
    pub zerstoerung: ZerstoerungRegel,
}

impl MondRegeln {
    pub fn laden(text: &str) -> Result<MondRegeln, String> {
        let r: MondRegeln = ron::from_str(text).map_err(|e| format!("Mondregeln nicht lesbar: {e}"))?;
        for t in Mondtyp::ALLE { if !r.typen.contains_key(&t) { return Err(format!("Mondregeln: Typ {} fehlt", t.name())); } }
        for g in MondGebaeude::ALLE { if !r.gebaeude.contains_key(&g) { return Err(format!("Mondregeln: Gebäude {} fehlt", g.name())); } }
        if r.truemmer_je_prozent <= 0 || r.chance_max_prozent > 100 || r.groesse_x_min > r.groesse_x_max
            || r.hyperraumloch.offen_sekunden <= 0 || r.tiefenphalanx.reichweite_teiler <= 0 {
            return Err("Mondregeln: ungültige Grenzwerte".into());
        }
        Ok(r)
    }
    pub fn standard() -> MondRegeln { Self::laden(REGELN_RON).expect("geprüfte Mondregeln") }
    pub fn typ(&self, t: Mondtyp) -> &MondtypRegel { &self.typen[&t] }
    pub fn geb(&self, g: MondGebaeude) -> &MondGebaeudeRegel { &self.gebaeude[&g] }
    /// Kosten für die Stufe `stufe`: `kosten · faktor^(stufe−1)`.
    pub fn kosten(&self, g: MondGebaeude, stufe: u8) -> [i64; GUETER] {
        let r = self.geb(g);
        let p = pot(r.faktor, stufe.saturating_sub(1) as u32);
        let mut a = [0i64; GUETER];
        for (gut, m) in &r.kosten { a[gut.idx()] = ((*m as i128 * M as i128 * p) / FX).min(i64::MAX as i128 / 4) as i64; }
        a
    }
    /// Abklingzeit in Sekunden für die (niedrigere) Torstufe.
    pub fn abklingzeit(&self, stufe: u8) -> i64 {
        let l = &self.hyperraumloch;
        let basis = l.abklingzeit_basis_minuten * MINUTE;
        let faktor = (1.0 - l.abklingzeit_je_stufe * (stufe.max(1) - 1) as f64).max(0.0);
        mal(basis, faktor).max(l.abklingzeit_min_minuten * MINUTE)
    }
    pub fn phalanx_reichweite(&self, stufe_eff: u8) -> i64 {
        if stufe_eff == 0 { return -1; }
        let s = stufe_eff as i64;
        (s * s - 1) / self.tiefenphalanx.reichweite_teiler
    }
    /// Würfelt den Mondtyp aus `w` in `0..Gewichtssumme`.
    pub fn typ_gewichte(&self, zone: Zone, nebel: bool) -> Vec<(Mondtyp, u64)> {
        Mondtyp::ALLE.iter().map(|t| {
            let r = self.typ(*t);
            let mut g = match zone { Zone::Glut => r.glut, Zone::Leben => r.leben, Zone::Frost => r.frost } as u64;
            if nebel && *t == Mondtyp::AmethystViolett { g *= self.nebel_amethyst_faktor as u64; }
            (*t, g)
        }).collect()
    }
    pub fn typ_wahl(&self, zone: Zone, nebel: bool, w: u64) -> Mondtyp {
        let mut rest = w;
        for (t, g) in self.typ_gewichte(zone, nebel) { if rest < g { return t; } rest -= g; }
        Mondtyp::KraterGrau
    }
    pub fn gewichtssumme(&self, zone: Zone, nebel: bool) -> u64 { self.typ_gewichte(zone, nebel).iter().map(|(_, g)| *g).sum() }
    /// Durchmesser in km: `⌊√(x·10⁶ + 30·T)⌋`, gedeckelt.
    pub fn durchmesser(&self, x: u8, truemmer: i64) -> i64 {
        let t = truemmer.clamp(0, self.truemmer_groesse_max);
        (wurzel(x as i128 * 1_000_000 + 30 * t as i128) as i64).min(self.durchmesser_max)
    }
    pub fn felder_obergrenze(&self, durchmesser: i64, typ: Mondtyp) -> u16 {
        let basis = (durchmesser as i128 * durchmesser as i128 / 1_000_000) as i64;
        mal(basis, 1.0 + self.typ(typ).felder_bonus).clamp(0, u16::MAX as i64) as u16
    }
    pub fn chance_prozent(&self, truemmer: i64) -> u8 {
        (truemmer / self.truemmer_je_prozent).clamp(0, self.chance_max_prozent as i64) as u8
    }
    /// Zerstörungs- und Verlustchance in Promille (Konzept 8.2).
    pub fn zerstoerung_promille(&self, durchmesser: i64, typ: Mondtyp, weltentruemmer: i64) -> (i64, i64) {
        self.zerstoerung_promille_mit(&crate::kernwerte::standard().mond, durchmesser, typ, weltentruemmer)
    }
    /// Wie [`MondRegeln::zerstoerung_promille`] mit den Kernwerten der Welt (Grenze der Wurzel, Verlustteiler).
    pub fn zerstoerung_promille_mit(&self, kw: &crate::kernwerte::MondWerte, durchmesser: i64, typ: Mondtyp, weltentruemmer: i64) -> (i64, i64) {
        let d_eff = mal(durchmesser, 1.0 + self.typ(typ).zerstoerung_mod);
        let w_d = wurzel(d_eff as i128 * 1_000_000) as i64;
        let w_n = wurzel(weltentruemmer.max(0) as i128 * 1_000_000) as i64;
        let grenze = kw.zerstoerung_wurzel_grenze.max(1) * 1000;
        let zerst = (((grenze - w_d) as i128 * w_n as i128) / 100_000).clamp(0, 1000) as i64;
        let verl = (w_d / (kw.verlust_teiler.max(1) * 10)).clamp(0, 1000);
        (zerst, verl)
    }
}

/// Energie einer Solarsonde in Tausendsteln je Stunde.
pub fn solarsonde_energie(r: &crate::Regelwerk, m: &MondRegeln, position: u8, energietechnik: u8, volk_energie: f64, typ_mod: f64) -> i64 {
    let solar = crate::habitabilitaet::solar_v(position, r.welt.plaetze_je_system, r.ablauf);
    let basis = milli(m.solarsonde_energie) * solar / 1000;
    mal(basis, (1.0 + r.wirtschaft.energietechnik_bonus * energietechnik as f64) * volk_energie * (1.0 + typ_mod))
}
/// Wie [`solarsonde_energie`] an einer Koordinate: mit den Kernwerten der Welt (Solarkurve, Strahlkraft des Sterns).
pub fn solarsonde_energie_an(r: &crate::Regelwerk, m: &MondRegeln, k: Koord, energietechnik: u8, volk_energie: f64, typ_mod: f64) -> i64 {
    let solar = crate::habitabilitaet::solar_r(r, k);
    let basis = milli(m.solarsonde_energie) * solar / 1000;
    mal(basis, (1.0 + r.wirtschaft.energietechnik_bonus * energietechnik as f64) * volk_energie * (1.0 + typ_mod))
}

// ---------------------------------------------------------------- Zustand

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mondbauauftrag { pub gebaeude: MondGebaeude, pub stufe: u8, pub fertig: Option<SimZeit>, pub dauer: i64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mondfertigung { pub einheit: Einheit, pub rest: i64, pub dauer: i64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reparaturlos { pub nr: u64, pub schiffe: BTreeMap<Einheit, i64>, pub kosten: Vec<i64>, pub fertig: SimZeit }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mond {
    pub id: MondId,
    pub mutter: PlanetId,
    pub besitzer: SpielerId,
    pub koord: Koord,
    pub name: String,
    pub typ: Mondtyp,
    pub durchmesser: i64,
    pub felder_max: u16,
    pub entstanden: SimZeit,
    pub kampf_nr: u64,
    pub bestand: [i64; GUETER],
    pub gebaeude: [u8; MONDGEBAEUDE],
    #[serde(with = "crate::snapshot_layout::units")]
    pub einheiten: [i64; EINHEITEN],
    pub bauschleife: Vec<Mondbauauftrag>,
    pub fertigung: Vec<Mondfertigung>,
    pub fertigung_naechste: SimZeit,
    pub reparatur: Vec<Reparaturlos>,
    pub blockade: Option<FlottenId>,
    pub sprungtor_bereit: SimZeit,
    pub phalanx_bereit: SimZeit,
    pub unterhalt_verzug: bool,
    pub stunde_stand: SimZeit,
    pub tag_stand: SimZeit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mondwurf {
    pub kampf_nr: u64,
    pub zeit: SimZeit,
    pub ort: Koord,
    pub truemmer: i64,
    pub chance_prozent: u8,
    pub wurf: u8,
    pub x: u8,
    pub typwurf: u64,
    pub typ: Mondtyp,
    pub durchmesser: i64,
    pub mond: Option<MondId>,
    pub beteiligte: Vec<SpielerId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Durchreise {
    pub zeit: SimZeit,
    pub spieler: SpielerId,
    pub vorwaerts: bool,
    pub herkunft: String,
    pub flotte: Option<FlottenId>,
    pub schiffe: BTreeMap<Einheit, i64>,
    pub struktur: i64,
    pub gebuehr: i64,
    pub gelandet: bool,
    pub torwache_kampf: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hyperraumloch {
    pub id: u64,
    pub oeffner: SpielerId,
    pub von: MondId,
    pub nach: MondId,
    pub von_koord: Koord,
    pub nach_koord: Koord,
    pub geoeffnet: SimZeit,
    pub schliesst: SimZeit,
    pub stufe: u8,
    pub kapazitaet: i64,
    pub kapazitaet_rest: i64,
    pub durchreisen: Vec<Durchreise>,
    /// Herkunftsorte und Flotten, die dieses Loch schon benutzt haben.
    pub benutzt: BTreeSet<String>,
    /// Ortungsschleier je Ende beim Öffnen (für ferne Tiefenphalanxen).
    pub schleier: [u8; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phalanxbericht { pub nr: u64, pub spieler: SpielerId, pub zeit: SimZeit, pub mond: MondId, pub ziel: Koord, pub kontakte: Vec<Value> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mondspionagebericht {
    pub zeit: SimZeit,
    pub spieler: SpielerId,
    pub mond: MondId,
    pub koord: Koord,
    pub besitzer: SpielerId,
    pub staerke: i64,
    pub schwellen: BTreeMap<String, i64>,
    pub bestand: Vec<i64>,
    pub schiffe: Option<BTreeMap<String, i64>>,
    pub verteidigung: Option<BTreeMap<String, i64>>,
    pub gebaeude: Option<BTreeMap<String, u8>>,
    pub forschung: Option<BTreeMap<String, u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Faehre { pub mond: MondId, pub gut: Gut, pub menge_je_stunde: i64 }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Mondzustand {
    pub aktiv: bool,
    pub monde: BTreeMap<MondId, Mond>,
    pub naechster_mond: MondId,
    pub flottenziel: BTreeMap<FlottenId, MondId>,
    pub flottenstart: BTreeMap<FlottenId, MondId>,
    pub loecher: BTreeMap<u64, Hyperraumloch>,
    pub loecher_zu: Vec<Hyperraumloch>,
    pub naechstes_loch: u64,
    pub phalanxberichte: Vec<Phalanxbericht>,
    pub naechster_scan: u64,
    pub wuerfe: BTreeMap<u64, Mondwurf>,
    pub faehren: Vec<Faehre>,
    pub freigaben: BTreeSet<MondId>,
    pub spionageberichte: Vec<Mondspionagebericht>,
    /// Taktische Raketensalven mit Zielart Mond (Salvennummer).
    #[serde(default)]
    pub raketenziel: BTreeSet<u64>,
    /// Überlebende Besatzungen zerstörter Monde je Trümmerfeld; Recycler und Bergungsschiffe nehmen sie als
    /// Passagiere auf (Karl 19.10.2026: „Trümmerring … um Material und Besatzung zu bergen“). JSON in der
    /// STERNE13-Hülle, darum als Liste statt Map mit Koordinatenschlüssel.
    #[serde(default)]
    pub ueberlebende: Vec<(Koord, crate::ausbau::Personen)>,
    /// Trümmerfelder aus Mondzerstörungen (Trümmerring statt Wrackwolke); fällt weg, wenn das Feld abgebaut ist.
    #[serde(default)]
    pub truemmerringe: Vec<Koord>,
}

impl crate::welt::Welt {
    /// Zusatzangaben eines Trümmerfelds für die Oberfläche: Trümmerring (Mond) und wartende Überlebende.
    pub fn truemmer_zusatz(&self, k: Koord, v: &mut serde_json::Value) {
        if self.monde.truemmerringe.contains(&k) { v["ring"] = serde_json::json!(true); }
        if let Some((_, p)) = self.monde.ueberlebende.iter().find(|(x, _)| *x == k) { v["ueberlebende"] = serde_json::json!(p.gesamt() / M); }
    }
}

/// Grundanteil des Trümmerrings aus der Mondmasse je Kilometer Durchmesser (Erz, Kristall) – ohne Regelfeld,
/// damit Spielstände mit eingebettetem Regelwerk lesbar bleiben. Ein 5000-km-Mond ergibt 100 000 Erz, 50 000 Kristall.
pub const TRUEMMER_JE_KM: [i64; 2] = [20, 10];
/// Anteil der Besatzung stationierter Schiffe, der die Zerstörung eines Mondes überlebt (Promille).
pub const UEBERLEBENDE_PROMILLE: i64 = 500;

fn gueter_text(k: &[i64; GUETER]) -> BTreeMap<&'static str, f64> {
    Gut::ALLE.iter().filter(|g| k[g.idx()] != 0).map(|g| (g.name(), k[g.idx()] as f64 / M as f64)).collect()
}
fn einheiten_text(a: &[i64; EINHEITEN], schiffe: bool) -> BTreeMap<&'static str, i64> {
    Einheit::ALLE.iter().filter(|e| e.ist_schiff() == schiffe && a[e.idx()] > 0).map(|e| (e.name(), a[e.idx()])).collect()
}
fn spanne(n: i64) -> [i64; 2] {
    if n <= 0 { return [0, 0]; }
    let mut p = 1i64;
    while p * 10 <= n { p *= 10; }
    [p, p * 10 - 1]
}

// ---------------------------------------------------------------- Welt: Grundlagen

impl Welt {
    /// Aktive Mondregeln dieser Partie.
    pub fn monde_regeln(&self) -> Option<std::sync::Arc<MondRegeln>> {
        if self.monde.aktiv { self.regeln.monde.clone() } else { None }
    }

    /// Schaltet die Mondgeneration ein (neue Partien; einmaliger Inhaltsschnitt wie `ausbau_aktivieren`).
    pub fn monde_aktivieren(&mut self) -> Result<(), String> {
        self.monde_aktivieren_aus(REGELN_RON)
    }
    /// Wie [`Welt::monde_aktivieren`], mit gegebenem Text von `regeln/monde.ron` (ConfigWelt).
    pub fn monde_aktivieren_aus(&mut self, text: &str) -> Result<(), String> {
        if self.monde.aktiv { return Ok(()); }
        if self.ausbau.version == 0 { self.ausbau_aktivieren()?; }
        let regeln = MondRegeln::laden(text)?;
        let r = std::sync::Arc::make_mut(&mut self.regeln);
        r.mondinhalt_aktivieren();
        r.monde = Some(std::sync::Arc::new(regeln));
        {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(r.hash.as_bytes());
            h.update(b"monde-v1");
            h.update(text.replace("\r\n", "\n").as_bytes());
            let neu: BTreeMap<_, _> = r.einheiten.iter().filter(|(e, _)| e.ist_mondinhalt()).collect();
            h.update(serde_json::to_vec(&neu).expect("Mondinhalte"));
            r.hash = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        }
        self.monde = Mondzustand { aktiv: true, naechster_mond: 1, naechstes_loch: 1, naechster_scan: 1, ..Default::default() };
        Ok(())
    }

    pub fn mond_an(&self, k: Koord) -> Option<MondId> {
        self.monde.monde.values().find(|m| m.koord == k).map(|m| m.id)
    }
    pub fn mond_von_planet(&self, pid: usize) -> Option<MondId> {
        self.monde.monde.values().find(|m| m.mutter as usize == pid).map(|m| m.id)
    }
    pub fn eigener_mond(&self, sid: SpielerId, k: Koord) -> Result<MondId, String> {
        if self.monde_regeln().is_none() { return Err("Monde sind in dieser Partie nicht aktiv".into()); }
        if self.ist_besiegt(sid) { return Err("Reich besiegt; keine weiteren Spielaktionen".into()); }
        match self.mond_an(k) {
            Some(mid) if self.monde.monde[&mid].besitzer == sid => Ok(mid),
            Some(_) => Err(format!("Der Mond bei {k} gehört dir nicht")),
            None => Err(format!("Bei {k} gibt es keinen Mond")),
        }
    }
    fn mond_typ_regel(&self, mid: MondId) -> MondtypRegel {
        let m = &self.monde.monde[&mid];
        self.regeln.monde.as_ref().map(|r| r.typ(m.typ).clone()).unwrap_or_default()
    }

    /// (belegt, verfügbar, Obergrenze) in Feldern.
    pub fn mond_felder(&self, mid: MondId) -> (u16, u16, u16) {
        let Some(r) = self.monde_regeln() else { return (0, 0, 0) };
        let m = &self.monde.monde[&mid];
        let belegt = m.gebaeude.iter().map(|s| *s as u16).sum::<u16>() + m.bauschleife.len() as u16;
        let basis = m.gebaeude[MondGebaeude::Mondbasis.idx()] as u16;
        let verfuegbar = (r.grundfelder + r.felder_je_mondbasis * basis).min(m.felder_max);
        (belegt, verfuegbar, m.felder_max)
    }

    /// (Erzeugung, Verbrauch) in Tausendsteln.
    pub fn mond_energie(&self, mid: MondId) -> (i64, i64) {
        let Some(r) = self.monde_regeln() else { return (0, 0) };
        let m = &self.monde.monde[&mid];
        let typ = r.typ(m.typ);
        let n = |g: MondGebaeude| m.gebaeude[g.idx()];
        let grund = mal(self.regeln.stufenwert(r.grundenergie_je_mondbasis, n(MondGebaeude::Mondbasis)), 1.0 + typ.grundenergie_mod);
        let sp = &self.spieler[m.besitzer as usize];
        let sonde = solarsonde_energie_an(&self.regeln, &r, m.koord, sp.forschung[Forschung::Energietechnik.idx()], self.regeln.volk(sp.volk).energie, typ.solar_mod);
        let sonde = if self.ausbau.dyson.aktiv { mal(sonde, self.dyson_sonnenfaktor(m.besitzer, m.koord)) } else { sonde };
        let erzeugung = grund + sonde * m.einheiten[Einheit::Solarsonde.idx()];
        let verbrauch = MondGebaeude::ALLE.iter().filter(|g| **g != MondGebaeude::Mondbasis)
            .map(|g| self.regeln.stufenwert(r.geb(*g).energie, n(*g))).sum();
        (erzeugung, verbrauch)
    }
    pub fn mond_energiefaktor(&self, mid: MondId) -> i64 {
        let (e, v) = self.mond_energie(mid);
        if v <= e { 1000 } else { e * 1000 / v.max(1) }
    }
    /// Wirkung aller Mondgebäude in Promille (Energie, Unterhaltsverzug).
    pub fn mond_wirkung(&self, mid: MondId) -> i64 {
        let f = self.mond_energiefaktor(mid);
        if self.monde.monde[&mid].unterhalt_verzug { f / 2 } else { f }
    }
    /// Sprungtor und Tiefenphalanx brauchen volle Versorgung und bezahlten Unterhalt.
    pub fn mond_voll_versorgt(&self, mid: MondId) -> bool {
        self.mond_energiefaktor(mid) >= 1000 && !self.monde.monde[&mid].unterhalt_verzug
    }
    pub fn mond_lager(&self, mid: MondId) -> [i64; GUETER] {
        let Some(r) = self.monde_regeln() else { return [0; GUETER] };
        let m = &self.monde.monde[&mid];
        let mut a = self.regeln.lagergrenze(m.gebaeude[MondGebaeude::Mondbasis.idx()]);
        for x in &mut a { *x = mal(*x, r.lager_anteil); }
        a[Gut::Deuterium.idx()] += self.kryolager_kapazitaet(mid);
        a
    }
    pub fn kryolager_kapazitaet(&self, mid: MondId) -> i64 {
        let Some(r) = self.monde_regeln() else { return 0 };
        let m = &self.monde.monde[&mid];
        let n = m.gebaeude[MondGebaeude::Kryolager.idx()];
        if n == 0 { return 0; }
        let basis = ((r.kryolager_basis as i128 * M as i128 * pot(r.kryolager_faktor, (n - 1) as u32)) / FX) as i64;
        mal(basis, 1.0 + r.typ(m.typ).kryolager_mod)
    }
    /// Regolithförderung je Spielstunde in Tausendsteln (nach Energie und Verzug).
    pub fn mond_foerderung(&self, mid: MondId) -> [i64; GUETER] {
        let mut a = [0i64; GUETER];
        let Some(r) = self.monde_regeln() else { return a };
        let m = &self.monde.monde[&mid];
        let n = m.gebaeude[MondGebaeude::Regolithfoerderung.idx()];
        if n == 0 { return a; }
        let typ = r.typ(m.typ);
        let geb = r.geb(MondGebaeude::Regolithfoerderung);
        let erz = mal(self.regeln.stufenwert(geb.ertrag_erz, n), (1.0 + typ.regolith_erz).max(0.0));
        let kristall = mal(self.regeln.stufenwert(geb.ertrag_kristall, n), (1.0 + typ.regolith_kristall).max(0.0));
        a[Gut::Erz.idx()] = erz;
        a[Gut::Kristall.idx()] = kristall;
        a[Gut::Deuterium.idx()] = mal(erz, typ.deuterium_spur);
        a[Gut::Nahrung.idx()] = self.regeln.stufenwert(typ.nahrung_je_stufe, n);
        let nebel = self.system(m.koord).is_some_and(|s| s.nebel);
        a[Gut::Xenokristall.idx()] = mal(kristall, if nebel { typ.xeno_spur_nebel } else { typ.xeno_spur });
        let w = self.mond_wirkung(mid);
        for x in &mut a { *x = anteil(*x, w, 1000); }
        a
    }
    pub fn mond_unterhalt_tag(&self, mid: MondId) -> i64 {
        let Some(r) = self.monde_regeln() else { return 0 };
        let m = &self.monde.monde[&mid];
        let stufen: i64 = m.gebaeude.iter().map(|s| *s as i64).sum();
        mal((r.unterhalt_basis_tag + r.unterhalt_je_stufe_tag * stufen) * M, (1.0 + r.typ(m.typ).unterhalt_mod).max(0.0))
    }
    pub(crate) fn mond_blockiert_fuer(&self, mid: MondId, sid: SpielerId) -> bool {
        let m = &self.monde.monde[&mid];
        m.blockade.and_then(|f| self.flotten.get(&f)).is_some_and(|f| f.zustand == Flottenzustand::ImOrbit && f.ziel == m.koord && f.besitzer != sid && !self.verbuendet(sid, f.besitzer))
    }
    fn feind_im_orbit(&self, mid: MondId) -> bool {
        let m = &self.monde.monde[&mid];
        self.flotten.values().any(|f| f.zustand == Flottenzustand::ImOrbit && f.ziel == m.koord && f.besitzer != m.besitzer
            && !self.verbuendet(f.besitzer, m.besitzer) && self.monde.flottenziel.get(&f.id) == Some(&mid))
    }
    fn mond_bauzeit(&self, mid: MondId, kosten: &[i64; GUETER], werk: MondGebaeude) -> i64 {
        let m = &self.monde.monde[&mid];
        let masse = kosten[Gut::Erz.idx()] + kosten[Gut::Kristall.idx()] + 2 * kosten[Gut::Legierung.idx()];
        let stufe = m.gebaeude[werk.idx()] as i128;
        let teiler = (self.regeln.wirtschaft.bauzeit_teiler as i128 * M as i128 * (1 + stufe)).max(1);
        (masse as i128 * STUNDE as i128 / teiler) as i64
    }
    fn mond_fertigungszeit(&self, mid: MondId, kosten: &[i64; GUETER]) -> i64 {
        let Some(r) = self.monde_regeln() else { return 30 };
        let volk = self.spieler[self.monde.monde[&mid].besitzer as usize].volk;
        mal(mal(self.mond_bauzeit(mid, kosten, MondGebaeude::Werftdock), self.regeln.volk(volk).werftzeit), 1.0 + r.werftdock_zeitaufschlag).max(30)
    }
}

// ---------------------------------------------------------------- Entstehung

impl Welt {
    /// Mondwurf nach einem Kampf an einem besiedelten Planeten (Konzept 3.1/3.2).
    pub fn mondwurf(&mut self, nr: u64, ort: Koord, truemmer_milli: i64, beteiligte: Vec<SpielerId>) {
        let Some(r) = self.monde_regeln() else { return };
        let Some(&pid) = self.belegung.get(&ort) else { return };
        let pid = pid as usize;
        if self.mond_von_planet(pid).is_some() { return; }
        // Mehrere Gefechte derselben Sekunde am selben Planeten würfeln nur einmal.
        if self.monde.wuerfe.values().any(|w| w.ort == ort && w.zeit == self.zeit) { return; }
        let t = truemmer_milli / M;
        if t < r.truemmer_je_prozent { return; }
        let chance = r.chance_prozent(t);
        let mut rng = strom(self.startwert, KANAL_MOND, nr);
        let zone = if self.ausbau.version > 0 { crate::habitabilitaet::zone(ort.position, self.regeln.welt.plaetze_je_system) } else { self.planeten[pid].zone };
        let nebel = self.system(ort).is_some_and(|s| s.nebel);
        let w1 = zufall(&mut rng, 100) as u8;
        let w2 = zufall(&mut rng, (r.groesse_x_max - r.groesse_x_min) as u64 + 1) as u8;
        let w3 = zufall(&mut rng, r.gewichtssumme(zone, nebel));
        let x = r.groesse_x_min + w2;
        let typ = r.typ_wahl(zone, nebel, w3);
        let durchmesser = r.durchmesser(x, t);
        let entsteht = w1 < chance;
        let mut mond = None;
        if entsteht {
            let id = self.monde.naechster_mond.max(1);
            self.monde.naechster_mond = id + 1;
            let besitzer = self.planeten[pid].besitzer;
            let felder_max = r.felder_obergrenze(durchmesser, typ);
            let stunde = (self.zeit / STUNDE) * STUNDE;
            self.monde.monde.insert(id, Mond {
                id, mutter: pid as PlanetId, besitzer, koord: ort, name: format!("Mond {ort}"), typ, durchmesser, felder_max,
                entstanden: self.zeit, kampf_nr: nr, bestand: [0; GUETER], gebaeude: [0; MONDGEBAEUDE], einheiten: [0; EINHEITEN],
                bauschleife: Vec::new(), fertigung: Vec::new(), fertigung_naechste: -1, reparatur: Vec::new(), blockade: None,
                sprungtor_bereit: 0, phalanx_bereit: 0, unterhalt_verzug: false, stunde_stand: stunde, tag_stand: (self.zeit / TAG) * TAG,
            });
            mond = Some(id);
        }
        while self.monde.wuerfe.len() >= 300 { let k = *self.monde.wuerfe.keys().next().unwrap(); self.monde.wuerfe.remove(&k); }
        self.monde.wuerfe.insert(nr, Mondwurf { kampf_nr: nr, zeit: self.zeit, ort, truemmer: t, chance_prozent: chance, wurf: w1, x, typwurf: w3, typ, durchmesser, mond, beteiligte: beteiligte.clone() });
        let text = if entsteht {
            format!("Gefecht bei {ort}: Mondchance {chance} % – ein Mond ist entstanden: Typ {}, {durchmesser} km", typ.name())
        } else {
            format!("Gefecht bei {ort}: Mondchance {chance} % – kein Mond")
        };
        let mut alle = beteiligte;
        alle.push(self.planeten[pid].besitzer);
        alle.sort();
        alle.dedup();
        for sid in alle {
            self.vorfall(sid, if entsteht { "mond_entstanden" } else { "mondwurf" }, text.clone());
            if entsteht { self.wecke(sid, Rolle::Stratege, "Mond entstanden", false); }
        }
        self.fachereignis("moon_roll", self.planeten[pid].besitzer, json!({"combat":nr,"target":ort,"chance":chance,"moon":mond,"type":typ.name(),"diameter":durchmesser}));
    }

    /// Reparaturdock am Kampfort: (Mond, Besitzer, Promille).
    pub fn reparaturdock(&self, ort: Koord) -> Option<(MondId, SpielerId, i64)> {
        let r = self.monde_regeln()?;
        let mid = self.mond_an(ort)?;
        let m = &self.monde.monde[&mid];
        let stufe = m.gebaeude[MondGebaeude::Werftdock.idx()];
        if stufe == 0 { return None; }
        let pm = milli((r.reparatur_je_stufe * stufe as f64).min(r.reparatur_max));
        Some((mid, m.besitzer, pm))
    }
    pub(crate) fn reparatur_vormerken(&mut self, mid: MondId, nr: u64, schiffe: &[i64; SCHIFFE]) {
        let Some(r) = self.monde_regeln() else { return };
        if !self.monde.monde.contains_key(&mid) || schiffe.iter().all(|n| *n <= 0) { return; }
        let volk = self.spieler[self.monde.monde[&mid].besitzer as usize].volk;
        let mut kosten = [0i64; GUETER];
        let mut dauer = 0i64;
        let mut liste = BTreeMap::new();
        for e in Einheit::ALLE.iter().filter(|e| e.ist_schiff() && schiffe[e.idx()] > 0) {
            let n = schiffe[e.idx()];
            let k = self.regeln.kosten_einheit(*e, volk);
            for g in [Gut::Legierung, Gut::Elektronik] { kosten[g.idx()] += mal(k[g.idx()], r.reparatur_kosten_anteil) * n; }
            dauer += mal(self.mond_fertigungszeit(mid, &k), r.reparatur_zeit_anteil) * n;
            liste.insert(*e, n);
        }
        let fertig = self.zeit + dauer.max(60);
        let m = self.monde.monde.get_mut(&mid).unwrap();
        m.reparatur.push(Reparaturlos { nr, schiffe: liste, kosten: kosten.to_vec(), fertig });
        let (besitzer, k) = (m.besitzer, m.koord);
        self.vorfall(besitzer, "reparaturdock", format!("Werftdock {k}: beschädigte Schiffe aus Kampf {nr} geborgen, Reparatur bis {}", zeittext(fertig)));
    }
}

// ---------------------------------------------------------------- Bauen, Fertigen, Takt

impl Welt {
    pub fn mond_bauen(&mut self, sid: SpielerId, k: Koord, g: MondGebaeude) -> Result<String, String> {
        let mid = self.eigener_mond(sid, k)?;
        let r = self.monde_regeln().unwrap();
        let regel = r.geb(g).clone();
        let sp = &self.spieler[sid as usize];
        if sp.stufe < regel.ab_stufe { return Err(format!("{} gibt es erst ab Zivilisationsstufe {}", g.name(), regel.ab_stufe)); }
        let m = &self.monde.monde[&mid];
        for (b, st) in &regel.braucht { if m.gebaeude[b.idx()] < *st { return Err(format!("{} braucht {} Stufe {st}", g.name(), b.name())); } }
        let fehlt: Vec<String> = regel.forschung.iter().filter(|(f, n)| sp.forschung[f.idx()] < **n).map(|(f, n)| format!("{f} Stufe {n}")).collect();
        if !fehlt.is_empty() { return Err(format!("{} benötigt {}", g.name(), fehlt.join(", "))); }
        let geplant = m.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;
        let stufe = m.gebaeude[g.idx()] + geplant + 1;
        if regel.max_stufe > 0 && stufe > regel.max_stufe { return Err(format!("{} höchstens Stufe {}", g.name(), regel.max_stufe)); }
        if stufe > self.regeln.welt.max_gebaeudestufe { return Err("Höchste Ausbaustufe erreicht".into()); }
        let (belegt, verfuegbar, _) = self.mond_felder(mid);
        // Ablauf v3: die Mondbasis schafft selbst Felder und darf auf vollem Mond gebaut werden (eine Stufe zur Zeit) – sonst
        // sperrte das letzte belegte Feld den Mond für immer (Mond-Sitzung 22.10.2026).
        if belegt >= verfuegbar && !(self.ablauf_v3() && g == MondGebaeude::Mondbasis && geplant == 0) { return Err(format!("Keine freien Felder auf dem Mond ({belegt}/{verfuegbar}); Mondbasis ausbauen, solange die Obergrenze es erlaubt")); }
        if m.bauschleife.len() >= self.regeln.wirtschaft.warteschlange { return Err("Mondbauschleife ist voll".into()); }
        let kosten = r.kosten(g, stufe);
        if let Some(gut) = Gut::ALLE.iter().find(|x| m.bestand[x.idx()] < kosten[x.idx()]) {
            return Err(format!("Mondbestand reicht nicht: {} fehlt (Mond {k}; Nachschub per Transport oder Fähre)", gut));
        }
        let dauer = self.mond_bauzeit(mid, &kosten, MondGebaeude::Mondbasis).max(self.regeln.wirtschaft.min_bauzeit_sekunden);
        let jetzt = self.zeit;
        let m = self.monde.monde.get_mut(&mid).unwrap();
        for i in 0..GUETER { m.bestand[i] -= kosten[i]; }
        let fertig = if m.bauschleife.is_empty() { Some(jetzt + dauer) } else { None };
        m.bauschleife.push(Mondbauauftrag { gebaeude: g, stufe, fertig, dauer });
        self.punkte_neu();
        Ok(format!("{} Stufe {stufe} auf Mond {k} beauftragt, {} min", g.name(), (dauer + 59) / 60))
    }

    pub fn mond_fertigen(&mut self, sid: SpielerId, rolle: Rolle, k: Koord, e: Einheit, anzahl: i64) -> Result<String, String> {
        let mid = self.eigener_mond(sid, k)?;
        let r = self.monde_regeln().unwrap();
        if !(1..=10_000).contains(&anzahl) { return Err("Anzahl muss zwischen 1 und 10000 liegen".into()); }
        if !self.regeln.einheiten.contains_key(&e) { return Err("Einheit ist in dieser Partie nicht verfügbar".into()); }
        let er = self.regeln.einh(e).clone();
        let sp = &self.spieler[sid as usize];
        if sp.stufe < er.ab_stufe { return Err(format!("{e} gibt es erst ab Zivilisationsstufe {}", er.ab_stufe)); }
        let fehlt = self.einheit_forschung_fehlend(sid, e);
        if !fehlt.is_empty() { return Err(format!("{e} benötigt {}", fehlt.join(", "))); }
        if matches!(e, Einheit::Planetenschild | Einheit::PlanetenschildII | Einheit::PlanetenschildIII) { return Err("Planetenschilde gibt es nur auf Planeten".into()); }
        let m = &self.monde.monde[&mid];
        let dock = m.gebaeude[MondGebaeude::Werftdock.idx()];
        let noetig = if e == Einheit::Weltentruemmer { r.weltentruemmer_werftdock } else { er.werft.max(1) };
        if dock < noetig { return Err(format!("{e} braucht Werftdock Stufe {noetig} auf dem Mond")); }
        if m.fertigung.len() >= self.regeln.wirtschaft.warteschlange { return Err("Werftdock-Schleife ist voll".into()); }
        let volk = sp.volk;
        let stueck = self.regeln.kosten_einheit(e, volk);
        let mut kosten = [0i64; GUETER];
        for g in 0..GUETER { kosten[g] = stueck[g].saturating_mul(anzahl); }
        // Bezahlt wird aus dem Mondbestand, der Rest vom eigenen Mutterplaneten, der das Werftdock beliefert
        // (19.10.2026: ein Weltenkrümmer kostet 5 Mio. Erz, das Mondlager fasst erst bei Mondbasis 16 so viel – deren
        // Ausbau passte selbst nicht ins Lager; Karl: „sonst kann ich es nie benutzen“).
        let pid = m.mutter as usize;
        let mutter_eigen = pid < self.planeten.len() && self.planeten[pid].besitzer == sid;
        let planet = if mutter_eigen { self.bestand_jetzt(pid) } else { [0; GUETER] };
        if let Some(gut) = Gut::ALLE.iter().find(|x| m.bestand[x.idx()] + planet[x.idx()] < kosten[x.idx()]) {
            let fehlt = (kosten[gut.idx()] - m.bestand[gut.idx()] - planet[gut.idx()]) / M;
            return Err(format!("Mondbestand{} reicht nicht: es fehlen {fehlt} {gut}", if mutter_eigen { " und Mutterplanet" } else { "" }));
        }
        // Besatzung kommt vom Mutterplaneten (der Mond hat keine Bevölkerung).
        let crew = er.besatzung * M * anzahl;
        if crew > 0 {
            if self.planeten[pid].besitzer != sid { return Err("Besatzung braucht den eigenen Mutterplaneten".into()); }
            if self.ausbau.version > 0 && self.ausbau.personen.get(&(pid as PlanetId)).is_none_or(|p| p.auswahl(crew).gesamt() != crew) { return Err("Nicht genügend ganze Personen für die Besatzung auf dem Mutterplaneten".into()); }
            if self.planeten[pid].bevoelkerung - crew < 500 * M { return Err("Besatzung fehlt: auf dem Mutterplaneten müssen mindestens 500 Einwohner bleiben".into()); }
        }
        if rolle == Rolle::Verwalter && e != Einheit::Solarsonde { return Err(format!("{e} bestellt der Feldherr, nicht der Verwalter")); }
        let dauer = self.mond_fertigungszeit(mid, &stueck);
        if crew > 0 {
            self.abrechnen(pid);
            self.crew_reservieren(pid, Produkt::Einheit(e), crew);
            self.planeten[pid].bevoelkerung -= crew;
            self.raten_neu(pid);
        }
        let jetzt = self.zeit;
        let vom_mond: [i64; GUETER] = std::array::from_fn(|i| kosten[i].min(self.monde.monde[&mid].bestand[i]));
        if (0..GUETER).any(|i| kosten[i] > vom_mond[i]) {
            self.abrechnen(pid);
            for i in 0..GUETER { self.planeten[pid].bestand[i] -= kosten[i] - vom_mond[i]; }
            self.raten_neu(pid);
        }
        let m = self.monde.monde.get_mut(&mid).unwrap();
        for i in 0..GUETER { m.bestand[i] -= vom_mond[i]; }
        if m.fertigung.is_empty() { m.fertigung_naechste = jetzt + dauer; }
        m.fertigung.push(Mondfertigung { einheit: e, rest: anzahl, dauer });
        Ok(format!("{anzahl} x {e} im Werftdock {k} bestellt, {dauer} s je Stück"))
    }

    pub fn mond_umbenennen(&mut self, sid: SpielerId, k: Koord, name: &str) -> Result<String, String> {
        let mid = self.eigener_mond(sid, k)?;
        let n: String = name.trim().chars().filter(|c| !c.is_control()).take(40).collect();
        if n.is_empty() { return Err("Name darf nicht leer sein".into()); }
        self.monde.monde.get_mut(&mid).unwrap().name = n.clone();
        Ok(format!("Mond {k} heißt jetzt {n}"))
    }

    pub fn mond_faehre(&mut self, sid: SpielerId, k: Koord, gut: Gut, menge: i64) -> Result<String, String> {
        let mid = self.eigener_mond(sid, k)?;
        let r = self.monde_regeln().unwrap();
        if matches!(gut, Gut::Antriebskern | Gut::Habitatmodul) { return Err("Bauteile fahren nicht mit der Fähre".into()); }
        if menge.abs() > 10_000_000 { return Err("Fährmenge zu groß".into()); }
        let basis = self.monde.monde[&mid].gebaeude[MondGebaeude::Mondbasis.idx()] as i64;
        let deckel = r.faehre_je_mondbasis_stunde * basis;
        let andere: i64 = self.monde.faehren.iter().filter(|f| f.mond == mid && f.gut != gut).map(|f| f.menge_je_stunde.abs()).sum();
        if andere + menge.abs() > deckel { return Err(format!("Fähre gedeckelt auf {deckel} Einheiten je Spielstunde (2.000 je Mondbasisstufe); belegt {andere}")); }
        self.monde.faehren.retain(|f| !(f.mond == mid && f.gut == gut));
        if menge == 0 { return Ok(format!("Fähre für {gut} zu Mond {k} beendet")); }
        self.monde.faehren.push(Faehre { mond: mid, gut, menge_je_stunde: menge });
        Ok(if menge > 0 { format!("Fähre: {menge} {gut} je Stunde vom Mutterplaneten zum Mond {k}") } else { format!("Fähre: {} {gut} je Stunde vom Mond {k} zum Mutterplaneten", -menge) })
    }

    /// Nächster Termin der Mondmechanik (Bau, Werftdock, Reparatur, Stunde, Tag, Lochschluss).
    pub(crate) fn mond_naechster_termin(&self) -> Option<SimZeit> {
        if !self.monde.aktiv { return None; }
        let mut t: Option<SimZeit> = None;
        let mut nimm = |x: SimZeit| { t = Some(t.map_or(x, |a: SimZeit| a.min(x))); };
        for m in self.monde.monde.values() {
            nimm(m.stunde_stand + STUNDE);
            nimm(m.tag_stand + TAG);
            if let Some(f) = m.bauschleife.first().and_then(|a| a.fertig) { nimm(f); }
            if m.fertigung_naechste >= 0 && !m.fertigung.is_empty() { nimm(m.fertigung_naechste); }
            for l in &m.reparatur { nimm(l.fertig); }
        }
        for l in self.monde.loecher.values() { nimm(l.schliesst); }
        t
    }

    pub(crate) fn monde_tick(&mut self) {
        if !self.monde.aktiv { return; }
        let jetzt = self.zeit;
        let ids: Vec<MondId> = self.monde.monde.keys().copied().collect();
        for mid in ids {
            self.mond_bau_tick(mid, jetzt);
            self.mond_fertigung_tick(mid, jetzt);
            self.mond_reparatur_tick(mid, jetzt);
            while self.monde.monde.get(&mid).is_some_and(|m| m.stunde_stand + STUNDE <= jetzt) {
                self.monde.monde.get_mut(&mid).unwrap().stunde_stand += STUNDE;
                self.mond_stunde(mid);
            }
            while self.monde.monde.get(&mid).is_some_and(|m| m.tag_stand + TAG <= jetzt) {
                self.monde.monde.get_mut(&mid).unwrap().tag_stand += TAG;
                self.mond_tag(mid);
            }
            let blockade_weg = {
                let m = &self.monde.monde[&mid];
                m.blockade.is_some_and(|f| !self.flotten.get(&f).is_some_and(|fl| fl.zustand == Flottenzustand::ImOrbit && fl.ziel == m.koord))
            };
            if blockade_weg { self.monde.monde.get_mut(&mid).unwrap().blockade = None; }
        }
        let zu: Vec<u64> = self.monde.loecher.values().filter(|l| l.schliesst <= jetzt).map(|l| l.id).collect();
        for id in zu { self.loch_schliessen(id, "Zeit abgelaufen"); }
        let flotten = &self.flotten;
        self.monde.flottenziel.retain(|f, _| flotten.contains_key(f));
        self.monde.flottenstart.retain(|f, _| flotten.contains_key(f));
    }

    fn mond_bau_tick(&mut self, mid: MondId, jetzt: SimZeit) {
        loop {
            let Some(m) = self.monde.monde.get_mut(&mid) else { return };
            let Some(fertig) = m.bauschleife.first().and_then(|a| a.fertig) else { return };
            if fertig > jetzt { return; }
            let a = m.bauschleife.remove(0);
            m.gebaeude[a.gebaeude.idx()] = a.stufe.max(m.gebaeude[a.gebaeude.idx()]);
            if let Some(n) = m.bauschleife.first_mut() { n.fertig = Some(fertig + n.dauer); }
            let (besitzer, k) = (m.besitzer, m.koord);
            self.vorfall(besitzer, "mondbau", format!("Mond {k}: {} Stufe {} fertig", a.gebaeude.name(), a.stufe));
            self.wecke(besitzer, Rolle::Verwalter, "Mondbau fertig", false);
            self.punkte_neu();
        }
    }
    fn mond_fertigung_tick(&mut self, mid: MondId, jetzt: SimZeit) {
        loop {
            let Some(m) = self.monde.monde.get(&mid) else { return };
            if m.fertigung.is_empty() || m.fertigung_naechste < 0 || m.fertigung_naechste > jetzt { return; }
            let e = m.fertigung[0].einheit;
            let pid = m.mutter as usize;
            self.crew_fertig(pid, Produkt::Einheit(e));
            let m = self.monde.monde.get_mut(&mid).unwrap();
            let t = m.fertigung_naechste;
            m.einheiten[e.idx()] += 1;
            m.fertigung[0].rest -= 1;
            if m.fertigung[0].rest <= 0 {
                m.fertigung.remove(0);
                let (besitzer, k) = (m.besitzer, m.koord);
                if let Some(n) = self.monde.monde.get_mut(&mid).unwrap().fertigung.first() { let d = n.dauer; self.monde.monde.get_mut(&mid).unwrap().fertigung_naechste = t + d; } else { self.monde.monde.get_mut(&mid).unwrap().fertigung_naechste = -1; }
                self.vorfall(besitzer, "fertigung", format!("Werftdock {k}: Auftrag {e} abgeschlossen"));
            } else {
                m.fertigung_naechste = t + m.fertigung[0].dauer;
            }
        }
    }
    fn mond_reparatur_tick(&mut self, mid: MondId, jetzt: SimZeit) {
        let Some(m) = self.monde.monde.get_mut(&mid) else { return };
        let mut fertig = Vec::new();
        for los in m.reparatur.iter_mut().filter(|l| l.fertig <= jetzt) {
            if (0..GUETER).all(|g| m.bestand[g] >= los.kosten.get(g).copied().unwrap_or(0)) {
                for g in 0..GUETER { m.bestand[g] -= los.kosten.get(g).copied().unwrap_or(0); }
                for (e, n) in &los.schiffe { m.einheiten[e.idx()] += n; }
                fertig.push(los.nr);
            } else {
                los.fertig = jetzt + STUNDE;
            }
        }
        m.reparatur.retain(|l| !fertig.contains(&l.nr));
        let (besitzer, k) = (m.besitzer, m.koord);
        for nr in fertig { self.vorfall(besitzer, "reparaturdock", format!("Werftdock {k}: Schiffe aus Kampf {nr} repariert")); }
    }
    fn mond_stunde(&mut self, mid: MondId) {
        let foerderung = self.mond_foerderung(mid);
        let lager = self.mond_lager(mid);
        {
            let m = self.monde.monde.get_mut(&mid).unwrap();
            for g in 0..GUETER {
                if foerderung[g] > 0 && m.bestand[g] < lager[g] { m.bestand[g] = (m.bestand[g] + foerderung[g]).min(lager[g]); }
            }
        }
        // Versorgungsfähre: gedeckelte, stündliche Verschiebung, ruht bei Blockade von Planet oder Mond.
        let faehren: Vec<Faehre> = self.monde.faehren.iter().filter(|f| f.mond == mid).cloned().collect();
        if faehren.is_empty() { return; }
        let (pid, besitzer) = { let m = &self.monde.monde[&mid]; (m.mutter as usize, m.besitzer) };
        if self.planeten[pid].besitzer != besitzer || self.blockiert_fuer(pid, besitzer) || self.mond_blockiert_fuer(mid, besitzer) { return; }
        self.abrechnen(pid);
        for f in faehren {
            let g = f.gut.idx();
            let menge = f.menge_je_stunde.abs() * M;
            if f.menge_je_stunde > 0 {
                let frei = (lager[g] - self.monde.monde[&mid].bestand[g]).max(0);
                let n = menge.min(self.planeten[pid].bestand[g].max(0)).min(frei);
                self.planeten[pid].bestand[g] -= n;
                self.monde.monde.get_mut(&mid).unwrap().bestand[g] += n;
            } else {
                let mut n = menge.min(self.monde.monde[&mid].bestand[g].max(0));
                // Ablauf-Version 1: wie in Gegenrichtung nur bis zur Lagergrenze des Planeten.
                if self.ablauf_v1() { n = n.min((self.lagergrenze(pid)[g] - self.planeten[pid].bestand[g]).max(0)); }
                self.monde.monde.get_mut(&mid).unwrap().bestand[g] -= n;
                self.planeten[pid].bestand[g] += n;
            }
        }
        self.raten_neu(pid);
    }
    fn mond_tag(&mut self, mid: MondId) {
        let kosten = self.mond_unterhalt_tag(mid);
        let besitzer = self.monde.monde[&mid].besitzer;
        let sp = &mut self.spieler[besitzer as usize];
        let verzug = sp.credits < kosten;
        if !verzug { sp.credits -= kosten; }
        let war = self.monde.monde[&mid].unterhalt_verzug;
        self.monde.monde.get_mut(&mid).unwrap().unterhalt_verzug = verzug;
        if verzug && !war {
            let k = self.monde.monde[&mid].koord;
            self.vorfall(besitzer, "mondunterhalt_verzug", format!("Mond {k}: Unterhalt {} Credits nicht bezahlt – halbe Wirkung, Hyperraumtor-Generator und Tiefenphalanx gesperrt", ganz(kosten)));
            self.wecke(besitzer, Rolle::Verwalter, "Mondunterhalt im Verzug", false);
        }
    }
}

// ---------------------------------------------------------------- Flüge, Kampf, Zerstörung

fn feld<'a>(a: &'a Value, k: &str) -> Option<&'a Value> { a.get(k).filter(|v| !v.is_null()) }
fn koord_feld(a: &Value, k: &str) -> Result<Koord, String> {
    feld(a, k).and_then(Value::as_str).ok_or_else(|| format!("Feld {k} fehlt"))?.parse()
}

impl Welt {
    /// `flotte_senden` mit `startart` und/oder `zielart` = `mond`.
    pub fn mond_flotte_senden(&mut self, sid: SpielerId, rolle: Rolle, a: &Value) -> Result<String, String> {
        let r = self.monde_regeln().ok_or("Monde sind in dieser Partie nicht aktiv")?;
        let start = koord_feld(a, "start")?;
        let ziel = koord_feld(a, "ziel")?;
        let mission: Mission = serde_json::from_value(feld(a, "mission").cloned().ok_or("Feld mission fehlt")?).map_err(|e| format!("mission: {e}"))?;
        let schiffe_map: BTreeMap<Einheit, i64> = serde_json::from_value(feld(a, "schiffe").cloned().unwrap_or(json!({}))).map_err(|e| format!("schiffe: {e}"))?;
        let ladung_map: BTreeMap<Gut, f64> = serde_json::from_value(feld(a, "ladung").cloned().unwrap_or(json!({}))).map_err(|e| format!("ladung: {e}"))?;
        let tempo = feld(a, "geschwindigkeit").and_then(Value::as_f64).unwrap_or(1.0);
        let halten = feld(a, "haltedauer_stunden").and_then(Value::as_i64).unwrap_or(0);
        let startart = feld(a, "startart").and_then(Value::as_str).unwrap_or("planet");
        let mut zielart = feld(a, "zielart").and_then(Value::as_str).unwrap_or("planet");
        if mission == Mission::MondZerstoeren { zielart = "mond"; }
        if !["planet", "mond"].contains(&startart) || !["planet", "mond"].contains(&zielart) { return Err("startart und zielart: planet oder mond".into()); }
        let mut schiffe = [0i64; SCHIFFE];
        for (e, n) in &schiffe_map {
            if !e.ist_schiff() { return Err(format!("{e} ist stationär und kann nicht fliegen")); }
            if *n < 0 || *n > 1_000_000 { return Err("Schiffszahl ungültig".into()); }
            schiffe[e.idx()] = *n;
        }
        let mut ladung = [0i64; GUETER];
        for (g, x) in &ladung_map {
            if !x.is_finite() || *x < 0.0 || *x > 1e12 { return Err("ladung muss eine nichtnegative Menge sein".into()); }
            ladung[g.idx()] = milli(*x);
        }
        if !(0.1..=1.0).contains(&tempo) { return Err("geschwindigkeit muss zwischen 0.1 und 1.0 liegen".into()); }
        let sigma = milli(tempo);
        let auftrag = Flugauftrag { start, ziel, mission, schiffe, sigma_pm: sigma, ladung, haltedauer: halten * STUNDE };
        let start_mond = if startart == "mond" { Some(self.eigener_mond(sid, start)?) } else { None };
        if zielart == "planet" {
            let Some(mid) = start_mond else { return self.flotte_senden(sid, rolle, auftrag) };
            return self.mond_start_ueber_planet(sid, rolle, mid, auftrag);
        }
        let zm = self.mond_an(ziel).ok_or_else(|| format!("Bei {ziel} gibt es keinen Mond"))?;
        let zbesitzer = self.monde.monde[&zm].besitzer;
        let freund = zbesitzer == sid || self.verbuendet(sid, zbesitzer);
        let hat = |e: Einheit| schiffe[e.idx()] > 0;
        if !schiffe.iter().any(|n| *n > 0) { return Err("keine Schiffe gewählt".into()); }
        let mut einweg = false;
        let mut haltedauer = 0;
        match mission {
            Mission::Transport => { }
            Mission::Stationieren => {
                if zbesitzer != sid { return Err("Stationieren nur auf eigenen Monden".into()); }
                if start_mond == Some(zm) { return Err("Start und Ziel sind derselbe Mond".into()); }
                einweg = true;
            }
            Mission::Halten => {
                if !freund { return Err("Halten nur an eigenen oder verbündeten Monden".into()); }
                haltedauer = auftrag.haltedauer;
                if haltedauer < STUNDE || haltedauer > self.regeln.flug.halten_max_stunden * STUNDE { return Err(format!("haltedauer_stunden muss zwischen 1 und {} liegen", self.regeln.flug.halten_max_stunden)); }
            }
            Mission::Spionage => {
                if zbesitzer == sid { return Err("eigene Monde muss man nicht ausspionieren".into()); }
                if (0..SCHIFFE).any(|e| e != Einheit::Spionagesonde.idx() && schiffe[e] > 0) { return Err("eine Spionageflotte besteht nur aus Spionagesonden".into()); }
                if self.aufklaerungsregeln() && !self.system_erfasst(sid, ziel) { return Err("Sonnensystem zuerst mit system_erkunden und einer Spionagesonde kartieren".into()); }
            }
            Mission::Angriff | Mission::Blockade | Mission::MondZerstoeren => {
                if freund { return Err("Eigene oder verbündete Monde kann man nicht angreifen".into()); }
                self.gegner_angreifbar(sid, zbesitzer)?;
                if mission == Mission::MondZerstoeren && !hat(Einheit::Weltentruemmer) { return Err("Mondzerstörung braucht mindestens einen Weltenkrümmer".into()); }
                if mission == Mission::Blockade {
                    if self.spieler[sid as usize].stufe < 4 { return Err("Blockade gibt es ab Zivilisationsstufe 4".into()); }
                    haltedauer = auftrag.haltedauer.clamp(STUNDE, self.regeln.flug.halten_max_stunden * STUNDE);
                }
            }
            _ => return Err(format!("Mission {mission} ist gegen Monde nicht möglich")),
        }
        if hat(Einheit::Weltentruemmer) && !matches!(mission, Mission::Angriff | Mission::MondZerstoeren | Mission::Stationieren | Mission::Transport) {
            return Err("Weltenkrümmer fliegen nur Angriff, Mondzerstörung, Transport oder Stationieren".into());
        }
        if self.aufklaerungsregeln() && !freund && mission != Mission::Spionage && !self.system_erfasst(sid, ziel) {
            return Err("Ziel zuerst aufklären".into());
        }
        let plaetze = self.flottenplaetze(sid);
        if self.flotten.values().filter(|f| f.besitzer == sid).count() >= plaetze { return Err(format!("alle {plaetze} Flottenplätze sind belegt, Computertechnik erhöht die Zahl")); }
        // Startort und Bestände.
        let (basis_pid, bestand_quelle) = match start_mond {
            Some(mid) => {
                let m = &self.monde.monde[&mid];
                if m.gebaeude[MondGebaeude::Mondbasis.idx()] < 1 { return Err("Flottenstart vom Mond braucht Mondbasis Stufe 1".into()); }
                if self.mond_blockiert_fuer(mid, sid) { return Err(format!("Mond {start} ist blockiert")); }
                for e in 0..SCHIFFE { if schiffe[e] > m.einheiten[e] { return Err(format!("auf Mond {start} stehen nur {} x {}", m.einheiten[e], Einheit::ALLE[e])); } }
                (m.mutter as usize, Some(mid))
            }
            None => {
                let pid = self.eigener_planet(sid, start)?;
                let p = &self.planeten[pid];
                if p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 { return Err(format!("Flottenstart braucht einen Raumhafen auf {start}")); }
                if self.blockiert_fuer(pid, sid) { return Err(format!("{start} ist blockiert")); }
                for e in 0..SCHIFFE { if schiffe[e] > p.einheiten[e] { return Err(format!("auf {start} stehen nur {} x {}", p.einheiten[e], Einheit::ALLE[e])); } }
                (pid, None)
            }
        };
        let plan = self.flugplan(sid, basis_pid, ziel, &schiffe, sigma)?;
        let fracht: i64 = ladung.iter().sum();
        plan.pruefe_zuladung(fracht, 0)?;
        if mission == Mission::Spionage && fracht > 0 { return Err("Sonden tragen keine Ladung".into()); }
        let treibstoff = plan.treibstoff * if einweg { 1 } else { 2 };
        let mut bedarf = ladung;
        bedarf[Gut::Deuterium.idx()] += treibstoff;
        match bestand_quelle {
            Some(mid) => {
                let m = &self.monde.monde[&mid];
                if let Some(g) = Gut::ALLE.iter().find(|g| m.bestand[g.idx()] < bedarf[g.idx()]) { return Err(format!("Mondbestand reicht nicht für Ladung und Treibstoff: {g}")); }
                let m = self.monde.monde.get_mut(&mid).unwrap();
                for g in 0..GUETER { m.bestand[g] -= bedarf[g]; }
                for e in 0..SCHIFFE { m.einheiten[e] -= schiffe[e]; }
            }
            None => {
                self.abrechnen(basis_pid);
                let mut sprit = [0i64; GUETER];
                sprit[Gut::Deuterium.idx()] = treibstoff;
                let topf = Self::topf_fuer(rolle, Topf::Wirtschaft);
                self.zahlbar(basis_pid, &bedarf, None)?;
                self.zahlbar(basis_pid, &sprit, topf)?;
                self.zahlen(basis_pid, &sprit, topf);
                let p = &mut self.planeten[basis_pid];
                for g in 0..GUETER { p.bestand[g] -= ladung[g]; }
                for e in 0..SCHIFFE { p.einheiten[e] -= schiffe[e]; }
            }
        }
        let id = self.naechste_flotte;
        self.naechste_flotte += 1;
        let ankunft = self.zeit + plan.dauer;
        self.flotten.insert(id, Flotte {
            id, besitzer: sid, start: self.planeten[basis_pid].id, start_koord: start, ziel, mission, schiffe, ladung, siedler: 0,
            abflug: self.zeit, ankunft, flugdauer: plan.dauer, rueckkehr: 0, orbit_ende: 0, haltedauer, zustand: Flottenzustand::Hinflug,
            gemeldet: false, verband: None,
        });
        self.monde.flottenziel.insert(id, zm);
        if let Some(mid) = start_mond { self.monde.flottenstart.insert(id, mid); }
        if self.ausbau.version > 0 { self.ausbau.fluganzeigen.insert(id, json!({"einweg":einweg,"hin_tempo_prozent":sigma/10,"hin_dauer":plan.dauer,"treibstoff_hin":plan.treibstoff as f64/M as f64,"distanz":plan.entfernung,"reisetempo":plan.tempo,"zielart":"mond","startart":startart})); }
        self.plane(ankunft, EreignisArt::FlotteAnkunft { flotte: id });
        if self.aufklaerungsregeln() {
            self.aufklaerung.abschirmung.insert(id, self.spieler[sid as usize].forschung[Forschung::Abschirmtechnik.idx()]);
        }
        if mission.feindlich() {
            // Basiswarnung für den Mondbesitzer, Details nach der Sensorstärke des Mutterplaneten.
            let mutter = self.monde.monde[&zm].mutter as usize;
            let kontakt = (ankunft - self.warnzeit(mutter)).max(self.zeit);
            if kontakt == self.zeit { self.sensor_kontakt(id); } else { self.plane(kontakt, EreignisArt::SensorKontakt { flotte: id }); }
            let jetzt = self.zeit;
            let sp = &mut self.spieler[sid as usize];
            sp.schutz_bis = sp.schutz_bis.min(jetzt);
        }
        if bestand_quelle.is_none() && fracht > 0 { self.raten_neu(basis_pid); }
        self.fachereignis("fleet_launched", sid, json!({"fleet":id,"mission":mission,"target":ziel,"target_kind":"moon","arrival":ankunft}));
        let _ = r;
        Ok(format!("Flotte {id} gestartet: {mission} zum Mond {ziel}, Ankunft {}, Treibstoff {}", zeittext(ankunft), ganz(treibstoff)))
    }

    /// Start vom Mond zu einem Planeten: der Flug läuft durch die normale Missionsprüfung des
    /// Mutterplaneten (gleiche Koordinate), bezahlt aber aus dem Mond; die Flotte landet wieder auf dem Mond.
    fn mond_start_ueber_planet(&mut self, sid: SpielerId, rolle: Rolle, mid: MondId, mut auftrag: Flugauftrag) -> Result<String, String> {
        if matches!(auftrag.mission, Mission::Kolonisieren | Mission::Kampfkolonisieren | Mission::Invasion | Mission::Bombardieren | Mission::MondZerstoeren | Mission::Terraformen) {
            return Err(format!("Mission {} startet nicht von einem Mond", auftrag.mission));
        }
        let m = self.monde.monde[&mid].clone();
        if m.gebaeude[MondGebaeude::Mondbasis.idx()] < 1 { return Err("Flottenstart vom Mond braucht Mondbasis Stufe 1".into()); }
        if self.mond_blockiert_fuer(mid, sid) { return Err(format!("Mond {} ist blockiert", m.koord)); }
        for e in 0..SCHIFFE { if auftrag.schiffe[e] > m.einheiten[e] { return Err(format!("auf Mond {} stehen nur {} x {}", m.koord, m.einheiten[e], Einheit::ALLE[e])); } }
        let pid = m.mutter as usize;
        if self.planeten[pid].besitzer != sid { return Err("Der Mutterplanet gehört dir nicht mehr".into()); }
        auftrag.start = self.planeten[pid].koord;
        self.abrechnen(pid);
        let (alt_e, alt_b) = (self.planeten[pid].einheiten, self.planeten[pid].bestand);
        {
            let p = &mut self.planeten[pid];
            for e in 0..SCHIFFE { p.einheiten[e] += auftrag.schiffe[e]; }
            for g in 0..GUETER { p.bestand[g] += m.bestand[g]; }
        }
        let vorher = self.naechste_flotte;
        let res = self.flotte_senden(sid, rolle, auftrag);
        let p = &mut self.planeten[pid];
        let verbraucht: Vec<i64> = (0..GUETER).map(|g| alt_b[g] + m.bestand[g] - p.bestand[g]).collect();
        p.bestand = alt_b;
        p.einheiten = alt_e;
        match res {
            Ok(text) => {
                let fid = vorher;
                let fl = self.flotten[&fid].clone();
                let mm = self.monde.monde.get_mut(&mid).unwrap();
                for g in 0..GUETER { mm.bestand[g] -= verbraucht[g]; }
                for e in 0..SCHIFFE { mm.einheiten[e] -= fl.schiffe[e]; }
                self.monde.flottenstart.insert(fid, mid);
                if let Some(v) = self.ausbau.fluganzeigen.get_mut(&fid) { v["startart"] = json!("mond"); }
                self.raten_neu(pid);
                Ok(format!("{text} (Start vom Mond)"))
            }
            Err(e) => { self.raten_neu(pid); Err(e) }
        }
    }

    /// Rückkehr einer vom Mond gestarteten Flotte. `true`: erledigt.
    pub(crate) fn mond_rueckkehr(&mut self, fid: FlottenId) -> bool {
        self.monde.flottenziel.remove(&fid);
        let Some(mid) = self.monde.flottenstart.get(&fid).copied() else { return false };
        let Some(f) = self.flotten.get(&fid).cloned() else { return false };
        let ok = self.monde.monde.get(&mid).is_some_and(|m| m.besitzer == f.besitzer) && !self.mond_blockiert_fuer(mid, f.besitzer);
        self.monde.flottenstart.remove(&fid);
        if !ok { return false; }
        let m = self.monde.monde.get_mut(&mid).unwrap();
        for e in 0..SCHIFFE { m.einheiten[e] += f.schiffe[e]; }
        for g in 0..GUETER { m.bestand[g] += f.ladung[g]; }
        let k = m.koord;
        self.flotten.remove(&fid);
        self.kolonisation.rueckwege.remove(&fid);
        self.kolonisation.transportziele.remove(&fid);
        self.vorfall(f.besitzer, "flotte", format!("Flotte {fid} ({}) ist auf Mond {k} zurück", f.mission));
        true
    }

    /// Ankunft einer Flotte mit Zielart Mond. `true`: erledigt.
    pub(crate) fn mond_ankunft(&mut self, fid: FlottenId) -> bool {
        let Some(&mid) = self.monde.flottenziel.get(&fid) else { return false };
        let f = self.flotten[&fid].clone();
        if !self.monde.monde.contains_key(&mid) {
            self.monde.flottenziel.remove(&fid);
            if f.mission.feindlich() || f.mission == Mission::Spionage { self.heimflug(fid); return true; }
            return false; // eigene Lieferungen landen auf dem Mutterplaneten
        }
        let m = self.monde.monde[&mid].clone();
        let freund = m.besitzer == f.besitzer || self.verbuendet(f.besitzer, m.besitzer);
        let jetzt = self.zeit;
        match f.mission {
            Mission::Transport if !self.mond_blockiert_fuer(mid, m.besitzer) => {
                let mm = self.monde.monde.get_mut(&mid).unwrap();
                for g in 0..GUETER { mm.bestand[g] += f.ladung[g]; }
                self.flotten.get_mut(&fid).unwrap().ladung = [0; GUETER];
                if m.besitzer != f.besitzer { self.vorfall(m.besitzer, "lieferung", format!("Lieferung auf Mond {} erhalten", m.koord)); }
                self.monde.flottenziel.remove(&fid);
                self.heimflug(fid);
            }
            Mission::Stationieren if m.besitzer == f.besitzer && !self.mond_blockiert_fuer(mid, f.besitzer) => {
                let mm = self.monde.monde.get_mut(&mid).unwrap();
                for e in 0..SCHIFFE { mm.einheiten[e] += f.schiffe[e]; }
                for g in 0..GUETER { mm.bestand[g] += f.ladung[g]; }
                self.flotten.remove(&fid);
                self.monde.flottenziel.remove(&fid);
                self.monde.flottenstart.remove(&fid);
                self.vorfall(f.besitzer, "flotte", format!("Flotte {fid} auf Mond {} stationiert", m.koord));
            }
            Mission::Halten if freund && !self.mond_blockiert_fuer(mid, f.besitzer) => {
                let ende = jetzt + f.haltedauer;
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::ImOrbit;
                fl.orbit_ende = ende;
                self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                if m.besitzer != f.besitzer { let n = self.spieler[f.besitzer as usize].name.clone(); self.vorfall(m.besitzer, "flotte", format!("Flotte von {n} hält am Mond {} bis {}", m.koord, zeittext(ende))); }
            }
            Mission::Spionage => { self.mond_spionieren(fid, mid); }
            Mission::Angriff | Mission::Blockade | Mission::MondZerstoeren if !freund => {
                if self.gegner_angreifbar(f.besitzer, m.besitzer).is_err() { self.monde.flottenziel.remove(&fid); self.heimflug(fid); return true; }
                let (sieger, nr) = self.mond_kampf(fid, mid);
                if !self.flotten.contains_key(&fid) { self.monde.flottenziel.remove(&fid); return true; }
                if sieger == Sieger::Angreifer {
                    match f.mission {
                        Mission::Angriff => { self.mond_pluendern(fid, mid); }
                        Mission::Blockade => {
                            let ende = jetzt + f.haltedauer.max(STUNDE);
                            let fl = self.flotten.get_mut(&fid).unwrap();
                            fl.zustand = Flottenzustand::ImOrbit;
                            fl.orbit_ende = ende;
                            self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                            self.monde.monde.get_mut(&mid).unwrap().blockade = Some(fid);
                            self.vorfall(m.besitzer, "blockade", format!("Mond {} wird bis {} blockiert", m.koord, zeittext(ende)));
                            return true;
                        }
                        _ => { self.mond_zerstoerung_versuch(fid, mid, nr); }
                    }
                }
                if self.flotten.contains_key(&fid) { self.monde.flottenziel.remove(&fid); self.heimflug(fid); }
            }
            _ => { self.monde.flottenziel.remove(&fid); self.heimflug(fid); }
        }
        true
    }

    /// Gefecht am Mond (Partei::Mond); liefert Sieger und Kampfnummer.
    pub(crate) fn mond_kampf(&mut self, fid: FlottenId, mid: MondId) -> (Sieger, u64) {
        let m = self.monde.monde[&mid].clone();
        let mut parteien = vec![Partei::Mond(mid)];
        for f in self.flotten.values() {
            if f.id != fid && f.zustand == Flottenzustand::ImOrbit && f.mission == Mission::Halten && f.ziel == m.koord
                && self.monde.flottenziel.get(&f.id) == Some(&mid)
                && (f.besitzer == m.besitzer || self.verbuendet(f.besitzer, m.besitzer)) && self.bewaffnet(&f.schiffe) {
                parteien.push(Partei::Flotte(f.id));
            }
        }
        let angreifer = self.flotten[&fid].besitzer;
        if !self.spieler[angreifer as usize].angegriffen.contains(&m.besitzer) { self.spieler[angreifer as usize].angegriffen.push(m.besitzer); }
        self.bruch_pruefen(angreifer, m.besitzer);
        let nr = self.kampf_nr;
        let sieger = self.gefecht(&[fid], &parteien, m.koord);
        (sieger, nr)
    }

    fn mond_pluendern(&mut self, fid: FlottenId, mid: MondId) {
        let Some(r) = self.monde_regeln() else { return };
        let f = self.flotten[&fid].clone();
        let regeln = self.regeln.clone();
        let quote = regeln.volk(self.spieler[f.besitzer as usize].volk).pluenderquote.unwrap_or(regeln.kampf.pluenderquote);
        let Ok(plan) = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000) else { return };
        let frei = (plan.kapazitaet - f.ladung.iter().sum::<i64>()).max(0);
        let m = &self.monde.monde[&mid];
        let mut schutz = [0i64; GUETER];
        if m.gebaeude[MondGebaeude::Kryolager.idx()] > 0 {
            schutz[Gut::Deuterium.idx()] = mal(m.bestand[Gut::Deuterium.idx()].min(self.kryolager_kapazitaet(mid)), r.kryolager_geschuetzt);
        }
        let beute = crate::planer::kampf::beute(&regeln, &m.bestand, &schutz, quote, frei);
        let mm = self.monde.monde.get_mut(&mid).unwrap();
        for g in 0..GUETER { mm.bestand[g] -= beute[g]; }
        let (besitzer, k) = (mm.besitzer, mm.koord);
        let fl = self.flotten.get_mut(&fid).unwrap();
        for g in 0..GUETER { fl.ladung[g] += beute[g]; }
        if let Some(b) = self.kampfberichte.last_mut() { b.beute = beute; }
        let wert = regeln.wert(&beute);
        self.spieler[f.besitzer as usize].statistik.beute += wert;
        let text = format!("Mond {k} geplündert: {} (Wert {})", crate::berichte::fracht_text(&beute), ganz(wert));
        self.vorfall(f.besitzer, "beute", text.clone());
        self.vorfall(besitzer, "beute", text);
    }

    fn mond_spionieren(&mut self, fid: FlottenId, mid: MondId) {
        let f = self.flotten[&fid].clone();
        let m = self.monde.monde[&mid].clone();
        let tech = |sid: SpielerId| self.spieler[sid as usize].forschung[Forschung::Spionagetechnik.idx()] as i64;
        let staerke = f.schiffe[Einheit::Spionagesonde.idx()] + tech(f.besitzer) - tech(m.besitzer);
        let aufschlag = m.gebaeude[MondGebaeude::Ortungsschleier.idx()] as i64 + self.mond_typ_regel(mid).spionage_mod;
        let schwellen: BTreeMap<String, i64> = [("schiffe", 2), ("verteidigung", 3), ("gebaeude", 5), ("forschung", 7)].into_iter().map(|(k, v)| (k.to_string(), v + aufschlag)).collect();
        let sieht = |k: &str| staerke >= schwellen[k];
        let gegner = &self.spieler[m.besitzer as usize];
        let bericht = Mondspionagebericht {
            zeit: self.zeit, spieler: f.besitzer, mond: mid, koord: m.koord, besitzer: m.besitzer, staerke, schwellen: schwellen.clone(),
            bestand: m.bestand.to_vec(),
            schiffe: sieht("schiffe").then(|| einheiten_text(&m.einheiten, true).into_iter().map(|(k, v)| (k.to_string(), v)).collect()),
            verteidigung: sieht("verteidigung").then(|| einheiten_text(&m.einheiten, false).into_iter().map(|(k, v)| (k.to_string(), v)).collect()),
            gebaeude: sieht("gebaeude").then(|| MondGebaeude::ALLE.iter().map(|g| (g.name().to_string(), m.gebaeude[g.idx()])).collect()),
            forschung: sieht("forschung").then(|| self.forschungen().iter().map(|x| (x.name().to_string(), gegner.forschung[x.idx()])).collect()),
        };
        self.monde.spionageberichte.retain(|b| !(b.spieler == f.besitzer && b.mond == mid));
        self.monde.spionageberichte.push(bericht);
        while self.monde.spionageberichte.iter().filter(|b| b.spieler == f.besitzer).count() > 30 {
            let i = self.monde.spionageberichte.iter().position(|b| b.spieler == f.besitzer).unwrap();
            self.monde.spionageberichte.remove(i);
        }
        let name = self.spieler[f.besitzer as usize].name.clone();
        self.vorfall(m.besitzer, "spionage", format!("Spionage durch {name} am Mond {} bemerkt; Sonde gestartet von {}", m.koord, f.start_koord));
        self.vorfall(f.besitzer, "spionage", format!("Mondspionagebericht von {} erhalten (Scanstärke {staerke})", m.koord));
        self.wecke(f.besitzer, Rolle::Feldherr, "Spionagebericht", false);
        self.wecke(m.besitzer, Rolle::Feldherr, "Spionage bemerkt", false);
        self.monde.flottenziel.remove(&fid);
        self.heimflug(fid);
    }

    fn mond_zerstoerung_versuch(&mut self, fid: FlottenId, mid: MondId, nr: u64) {
        let Some(r) = self.monde_regeln() else { return };
        let Some(f) = self.flotten.get(&fid).cloned() else { return };
        let n = f.schiffe[Einheit::Weltentruemmer.idx()];
        if n <= 0 { return; }
        let m = self.monde.monde[&mid].clone();
        let (zerst, verl) = r.zerstoerung_promille_mit(&self.regeln.kw().mond, m.durchmesser, m.typ, n);
        let mut rng = strom(self.startwert, KANAL_MOND, nr);
        let w_z = zufall(&mut rng, 1000) as i64;
        let w_v = zufall(&mut rng, 1000) as i64;
        let zerstoert = w_z < zerst;
        let verloren = w_v < verl;
        // In Prozent wie Lage und Handbuch (Funktionsprüfung 25.10.2026: „904 ‰“).
        let prozent = |pm: i64| if pm % 10 == 0 { format!("{} %", pm / 10) } else { format!("{},{} %", pm / 10, pm % 10) };
        let text = format!("Zerstörungsversuch am Mond {} ({} km) mit {n} Weltenkrümmern: Chance {}, Verlustrisiko {} – {}{}",
            m.koord, crate::schiffhandel::credits_text(m.durchmesser), prozent(zerst), prozent(verl), if zerstoert { "Mond zerstört" } else { "Mond hält" }, if verloren { "; alle Weltenkrümmer verloren" } else { "" });
        self.vorfall(f.besitzer, if zerstoert { "mond_zerstoert" } else { "mond_zerstoerung_fehlgeschlagen" }, text.clone());
        self.vorfall(m.besitzer, if zerstoert { "mond_zerstoert" } else { "mond_zerstoerung_fehlgeschlagen" }, text);
        self.wecke(m.besitzer, Rolle::Feldherr, "Mondzerstörung", true);
        self.wecke(f.besitzer, Rolle::Feldherr, "Mondzerstörung", false);
        if verloren {
            let fl = self.flotten.get_mut(&fid).unwrap();
            fl.schiffe[Einheit::Weltentruemmer.idx()] = 0;
            let leer = fl.schiffe.iter().all(|x| *x == 0);
            self.vorfall(f.besitzer, "weltentruemmer_verloren", format!("{n} Weltenkrümmer beim Mond {} verloren", m.koord));
            if leer { self.flotten.remove(&fid); self.monde.flottenziel.remove(&fid); }
        }
        if zerstoert { self.mond_loeschen(mid, true); }
    }

    /// Recycler/Bergungsschiff am Trümmerfeld eines zerstörten Mondes: Überlebende als Passagiere aufnehmen,
    /// begrenzt durch die freien Personenplätze der Flotte (Rest wartet auf den nächsten Flug).
    pub fn ueberlebende_bergen(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else { return };
        let Some(i) = self.monde.ueberlebende.iter().position(|(k, _)| *k == f.ziel) else { return };
        let plaetze: i64 = f.schiffe.iter().enumerate().map(|(e, n)| n * self.regeln.personenplaetze(Einheit::ALLE[e])).sum();
        let an_bord = self.ausbau.passagiere.get(&fid).map(|p| p.gesamt() / M).unwrap_or(0);
        let frei = (plaetze - an_bord).max(0);
        if frei == 0 {
            self.vorfall(f.besitzer, "ueberlebende_warten", format!("Am Trümmerring {} warten Überlebende – die Flotte hat keine freien Personenplätze (Bergungsschiff mitschicken).", f.ziel));
            return;
        }
        let warten = self.monde.ueberlebende[i].1.gesamt() / M;
        let mut teil = self.monde.ueberlebende[i].1.clone();
        teil.begrenzen(frei.min(warten) * M);
        let geborgen = teil.gesamt() / M;
        if geborgen <= 0 { return; }
        let _ = self.monde.ueberlebende[i].1.nehmen(&teil);
        if self.monde.ueberlebende[i].1.gesamt() <= 0 { self.monde.ueberlebende.remove(i); }
        self.ausbau.passagiere.entry(fid).or_default().add(&teil);
        self.vorfall(f.besitzer, "ueberlebende_geborgen", format!("{geborgen} Überlebende vom Trümmerring {} an Bord genommen{}.", f.ziel,
            if warten > geborgen { format!(" – {} warten noch", warten - geborgen) } else { String::new() }));
    }

    /// Entfernt einen Mond. `fallout`: Zerstörung mit Trümmerregen (sonst Aufgabe des Planeten).
    pub fn mond_loeschen(&mut self, mid: MondId, fallout: bool) {
        let Some(m) = self.monde.monde.remove(&mid) else { return };
        let r = self.monde_regeln();
        if fallout {
            if let Some(r) = r.as_ref() {
                let z = &r.zerstoerung;
                let mut t = [mal(m.bestand[Gut::Erz.idx()], z.fallout_lager_anteil), mal(m.bestand[Gut::Kristall.idx()], z.fallout_lager_anteil)];
                for g in MondGebaeude::ALLE {
                    for s in 1..=m.gebaeude[g.idx()] {
                        let k = r.kosten(g, s);
                        t[0] += mal(k[Gut::Erz.idx()], z.fallout_gebaeude_anteil);
                        t[1] += mal(k[Gut::Kristall.idx()], z.fallout_gebaeude_anteil);
                    }
                }
                // Der Mond zerfällt zu einem Trümmerring: Grundanteil aus der Mondmasse, auch wenn Lager und
                // Gebäude leer sind (vorher 0/0 – Recycler fanden nichts), dazu Wracks der stationierten Einheiten
                // wie bei Gefechtstrümmern.
                t[0] += m.durchmesser.max(0) * self.regeln.kw().mond.truemmer_je_km_erz * M;
                t[1] += m.durchmesser.max(0) * self.regeln.kw().mond.truemmer_je_km_kristall * M;
                let volk = self.spieler.get(m.besitzer as usize).map(|s| s.volk).unwrap_or(Volk::Aurelianer);
                let mut besatzung = 0i64;
                for e in Einheit::ALLE {
                    let n = m.einheiten[e.idx()];
                    if n <= 0 { continue; }
                    let k = self.regeln.kosten_einheit(e, volk);
                    t[0] += mal(k[Gut::Erz.idx()] * n, self.regeln.kampf.truemmer_anteil);
                    t[1] += mal(k[Gut::Kristall.idx()] * n, self.regeln.kampf.truemmer_anteil);
                    if e.ist_schiff() { besatzung += n * self.regeln.einh(e).besatzung; }
                }
                let e = self.truemmer.entry(m.koord).or_insert([0, 0]);
                e[0] += t[0];
                e[1] += t[1];
                if !self.monde.truemmerringe.contains(&m.koord) { self.monde.truemmerringe.push(m.koord); }
                let ueberlebend = besatzung * self.regeln.kw().mond.ueberlebende_promille / 1000;
                if ueberlebend > 0 {
                    let mut p = crate::ausbau::Personen::default();
                    p.gruppen[volk.idx()][1] = ueberlebend * M;
                    if let Some((_, alt)) = self.monde.ueberlebende.iter_mut().find(|(k, _)| *k == m.koord) { alt.add(&p); } else { self.monde.ueberlebende.push((m.koord, p)); }
                }
                self.vorfall(m.besitzer, "mond_truemmerring", format!(
                    "Über {} zieht ein Trümmerring: {} Erz, {} Kristall{} – Recycler und Bergungsschiffe können ihn abbauen.",
                    m.koord, t[0] / M, t[1] / M,
                    if ueberlebend > 0 { format!("; {ueberlebend} Überlebende der Mondbesatzung warten auf Bergung") } else { String::new() }));
                let pid = m.mutter as usize;
                if pid < self.planeten.len() {
                    let jetzt = self.zeit;
                    self.planeten[pid].mali.push(Malus { betrag: milli(z.fallout_malus), start: jetzt, ende: jetzt + z.fallout_stunden * STUNDE });
                    self.raten_neu(pid);
                }
            }
        }
        // Flotten: eigene Lieferungen zum Planeten, feindliche Missionen kehren um, Orbitflotten fliegen heim.
        let ziel: Vec<FlottenId> = self.monde.flottenziel.iter().filter(|(_, x)| **x == mid).map(|(f, _)| *f).collect();
        for fid in ziel {
            self.monde.flottenziel.remove(&fid);
            let Some(f) = self.flotten.get(&fid).cloned() else { continue };
            if f.zustand == Flottenzustand::ImOrbit || (f.zustand == Flottenzustand::Hinflug && (f.mission.feindlich() || f.mission == Mission::Spionage || f.mission == Mission::Halten)) {
                self.heimflug(fid);
            } else if f.mission == Mission::Transport && self.kolonisationsregeln_v2() && (m.mutter as usize) < self.planeten.len() {
                let empfaenger = self.planeten[m.mutter as usize].besitzer;
                self.kolonisation.transportziele.insert(fid, crate::kolonisation::Transportziel::Planet { planet: m.mutter, empfaenger });
            }
        }
        let start: Vec<FlottenId> = self.monde.flottenstart.iter().filter(|(_, x)| **x == mid).map(|(f, _)| *f).collect();
        for fid in start { self.monde.flottenstart.remove(&fid); }
        let loecher: Vec<u64> = self.monde.loecher.values().filter(|l| l.von == mid || l.nach == mid).map(|l| l.id).collect();
        for id in loecher { self.loch_schliessen(id, "Mond vernichtet"); }
        self.monde.faehren.retain(|f| f.mond != mid);
        self.monde.freigaben.remove(&mid);
        self.punkte_neu();
        self.fachereignis("moon_removed", m.besitzer, json!({"moon":mid,"target":m.koord,"destroyed":fallout}));
    }

    /// Eroberung des Mutterplaneten: Der Mond geht mit; fremde Schiffe und Aufträge des Vorbesitzers enden.
    pub(crate) fn mond_planet_erobert(&mut self, pid: usize, neu: SpielerId) {
        let Some(mid) = self.mond_von_planet(pid) else { return };
        let alt = self.monde.monde[&mid].besitzer;
        if alt == neu { return; }
        let m = self.monde.monde.get_mut(&mid).unwrap();
        m.besitzer = neu;
        for e in 0..SCHIFFE { m.einheiten[e] = 0; }
        m.bauschleife.clear();
        m.fertigung.clear();
        m.fertigung_naechste = -1;
        m.reparatur.clear();
        m.unterhalt_verzug = false;
        let k = m.koord;
        self.monde.faehren.retain(|f| f.mond != mid);
        self.monde.freigaben.remove(&mid);
        let loecher: Vec<u64> = self.monde.loecher.values().filter(|l| l.von == mid || l.nach == mid).map(|l| l.id).collect();
        for id in loecher { self.loch_schliessen(id, "Torbesitzer gewechselt"); }
        let start: Vec<FlottenId> = self.monde.flottenstart.iter().filter(|(_, x)| **x == mid).map(|(f, _)| *f).collect();
        for fid in start { self.monde.flottenstart.remove(&fid); }
        self.vorfall(alt, "mond", format!("Mond {k} ging mit dem Planeten verloren"));
        self.vorfall(neu, "mond", format!("Mond {k} mit dem Planeten übernommen"));
    }
    pub(crate) fn mond_aufgeben_grund(&self, pid: usize) -> Option<String> {
        let mid = self.mond_von_planet(pid)?;
        let m = &self.monde.monde[&mid];
        (m.einheiten[..SCHIFFE].iter().any(|n| *n > 0) || self.monde.flottenstart.values().any(|x| *x == mid))
            .then(|| format!("Mond {} hat noch Schiffe oder Flotten unterwegs; zuerst abziehen", m.koord))
    }
    pub(crate) fn mond_planet_aufgegeben(&mut self, pid: usize) {
        if let Some(mid) = self.mond_von_planet(pid) { self.mond_loeschen(mid, false); }
    }

    /// Werte für die Wertung: (Wirtschaft, Militär) in Werteinheiten.
    pub(crate) fn mond_punkte(&self, sid: SpielerId) -> (i64, i64) {
        let Some(r) = self.monde_regeln() else { return (0, 0) };
        let volk = self.spieler[sid as usize].volk;
        let mut w = 0i64;
        let mut mil = 0i64;
        for m in self.monde.monde.values().filter(|m| m.besitzer == sid) {
            for g in MondGebaeude::ALLE { for s in 1..=m.gebaeude[g.idx()] { w = w.saturating_add(self.regeln.wert(&r.kosten(g, s))); } }
            for e in Einheit::ALLE { let n = m.einheiten[e.idx()]; if n > 0 { mil = mil.saturating_add(n * self.regeln.wert(&self.regeln.kosten_einheit(e, volk))); } }
        }
        (w, mil)
    }
}

// ---------------------------------------------------------------- Tiefenphalanx

impl Welt {
    pub fn phalanx_stufe_eff(&self, mid: MondId) -> u8 {
        let m = &self.monde.monde[&mid];
        let s = m.gebaeude[MondGebaeude::Tiefenphalanx.idx()];
        if s == 0 { 0 } else { s + self.mond_typ_regel(mid).phalanx_stufe }
    }
    pub fn phalanx_scannen(&mut self, sid: SpielerId, k: Koord, ziel: Koord, zielart_mond: bool) -> Result<String, String> {
        let mid = self.eigener_mond(sid, k)?;
        let r = self.monde_regeln().unwrap();
        if zielart_mond { return Err("Die Tiefenphalanx kann keine Monde scannen".into()); }
        let m = self.monde.monde[&mid].clone();
        if m.gebaeude[MondGebaeude::Tiefenphalanx.idx()] == 0 { return Err("Auf diesem Mond steht keine Tiefenphalanx".into()); }
        if !self.mond_voll_versorgt(mid) { return Err("Tiefenphalanx braucht volle Energie und bezahlten Unterhalt".into()); }
        if self.zeit < m.phalanx_bereit { return Err(format!("Tiefenphalanx lädt noch bis {}", zeittext(m.phalanx_bereit))); }
        let Some(&zp) = self.belegung.get(&ziel) else { return Err(format!("Bei {ziel} liegt kein Planet")); };
        let reichweite = r.phalanx_reichweite(self.phalanx_stufe_eff(mid));
        if ziel.sektor != m.koord.sektor || (ziel.system as i64 - m.koord.system as i64).abs() > reichweite {
            return Err(format!("Ziel außerhalb der Reichweite ({reichweite} Systeme im eigenen Sektor)"));
        }
        let kosten = mal(r.tiefenphalanx.scan_deuterium * M, 1.0 + self.mond_typ_regel(mid).scan_mod);
        if m.bestand[Gut::Deuterium.idx()] < kosten { return Err(format!("Scan kostet {} Deuterium aus dem Mondbestand", ganz(kosten))); }
        let ueberwachung = self.spieler[sid as usize].forschung[Forschung::Ueberwachungstechnik.idx()] as i64;
        let phalanx = self.phalanx_stufe_eff(mid) as i64;
        let detail = |fid: FlottenId| -> i64 {
            if !self.aufklaerungsregeln() { return 99; }
            phalanx.min(ueberwachung) - self.aufklaerung.abschirmung.get(&fid).copied().unwrap_or(0) as i64
        };
        let planet_koord = self.planeten[zp as usize].koord;
        let mut kontakte = Vec::new();
        // Bis Ablauf-Version 0 blieben auch Flüge vom Planeten zu einem Mond unsichtbar (alte Welten unverändert).
        let mond_sichtbar = self.ablauf_v1();
        for f in self.flotten.values() {
            let mondziel = self.monde.flottenziel.contains_key(&f.id);
            let mondstart = self.monde.flottenstart.contains_key(&f.id);
            let richtung = if f.zustand == Flottenzustand::Hinflug && f.ziel == planet_koord && !mondziel { Some(("anflug", f.ankunft)) }
                else if f.zustand == Flottenzustand::ImOrbit && f.ziel == planet_koord && !mondziel { Some(("orbit", f.orbit_ende)) }
                // Nur Flüge zwischen zwei Monden bleiben unsichtbar: Ein Flug vom gescannten Planeten zu einem Mond
                // (und seine Rückkehr) erscheint, wie der Anflug von einem Mond auf diesen Planeten.
                else if f.start_koord == planet_koord && f.start == zp && !mondstart && (mond_sichtbar || !mondziel) && f.zustand == Flottenzustand::Hinflug { Some(("abflug", f.ankunft)) }
                else if f.start_koord == planet_koord && f.start == zp && !mondstart && (mond_sichtbar || !mondziel) && f.zustand == Flottenzustand::Rueckflug { Some(("rueckkehr", f.rueckkehr)) }
                else { None };
            let Some((richtung, zeit)) = richtung else { continue };
            let d = detail(f.id);
            let anzahl: i64 = f.schiffe.iter().sum();
            let mut v = json!({"flotte":f.id,"richtung":richtung,"herkunft":f.start_koord.to_string(),"ziel":f.ziel.to_string(),"ankunft":zeit,"ankunft_text":zeittext(zeit),"detailstufe":d.clamp(0, 99)});
            if d >= 1 { v["besitzer"] = json!(self.spieler[f.besitzer as usize].name); v["schiffe_spanne"] = json!(spanne(anzahl)); }
            if d >= 3 { v["schiffe"] = json!(anzahl); }
            if d >= 6 {
                v["mission"] = json!(f.mission.name());
                v["schiffstypen"] = json!(Einheit::ALLE.iter().filter(|e| e.ist_schiff() && f.schiffe[e.idx()] > 0).map(|e| (e.name(), f.schiffe[e.idx()])).collect::<BTreeMap<_, _>>());
            }
            kontakte.push(v);
        }
        let nr = self.monde.naechster_scan.max(1);
        self.monde.naechster_scan = nr + 1;
        let anzahl = kontakte.len();
        let mm = self.monde.monde.get_mut(&mid).unwrap();
        mm.bestand[Gut::Deuterium.idx()] -= kosten;
        mm.phalanx_bereit = self.zeit + r.tiefenphalanx.abklingzeit_sekunden;
        self.monde.phalanxberichte.push(Phalanxbericht { nr, spieler: sid, zeit: self.zeit, mond: mid, ziel, kontakte });
        while self.monde.phalanxberichte.iter().filter(|b| b.spieler == sid).count() > r.tiefenphalanx.berichte_max {
            let i = self.monde.phalanxberichte.iter().position(|b| b.spieler == sid).unwrap();
            self.monde.phalanxberichte.remove(i);
        }
        Ok(format!("Phalanxbericht {nr} für {ziel}: {anzahl} Kontakte"))
    }
}

// ---------------------------------------------------------------- Sprungtor und Hyperraumloch

impl Welt {
    pub fn sprungtor_freigabe(&mut self, sid: SpielerId, k: Koord, frei: bool) -> Result<String, String> {
        let mid = self.eigener_mond(sid, k)?;
        if self.monde.monde[&mid].gebaeude[MondGebaeude::Sprungtor.idx()] == 0 { return Err("Auf diesem Mond steht kein Hyperraumtor-Generator".into()); }
        if frei { self.monde.freigaben.insert(mid); } else { self.monde.freigaben.remove(&mid); }
        Ok(format!("Hyperraumtor-Generator {k} für Allianz und Verteidigungsbündnis {}", if frei { "freigegeben" } else { "gesperrt" }))
    }

    fn tor_bereit(&self, mid: MondId, sid: SpielerId) -> Result<(), String> {
        let m = &self.monde.monde[&mid];
        if m.gebaeude[MondGebaeude::Sprungtor.idx()] == 0 { return Err(format!("Mond {} hat keinen Hyperraumtor-Generator", m.koord)); }
        if !self.mond_voll_versorgt(mid) { return Err(format!("Hyperraumtor-Generator {} braucht volle Energie und bezahlten Unterhalt", m.koord)); }
        if self.zeit < m.sprungtor_bereit { return Err(format!("Hyperraumtor-Generator {} lädt bis {}", m.koord, zeittext(m.sprungtor_bereit))); }
        if self.mond_blockiert_fuer(mid, sid) || self.mond_blockiert_fuer(mid, m.besitzer) { return Err(format!("Mond {} ist blockiert", m.koord)); }
        if self.feind_im_orbit(mid) { return Err(format!("Feindliche Flotte im Orbit von {}", m.koord)); }
        if self.monde.loecher.values().any(|l| l.von == mid || l.nach == mid) { return Err(format!("Am Tor {} ist bereits ein Hyperraumloch offen", m.koord)); }
        Ok(())
    }

    /// Abstand zweier Tore in Systemen; Sektorwechsel zählt `aufschlag + |Δsystem|`.
    pub fn tor_abstand(&self, a: Koord, b: Koord) -> i64 {
        let r = self.monde_regeln();
        let d = (a.system as i64 - b.system as i64).abs();
        if a.sektor == b.sektor { d } else { r.map(|r| r.hyperraumloch.sektorwechsel_aufschlag).unwrap_or(30) + d }
    }

    pub fn hyperraumloch_oeffnen(&mut self, sid: SpielerId, rolle: Rolle, k: Koord, gegenstelle: Koord, erst: Option<&Value>) -> Result<String, String> {
        let von = self.eigener_mond(sid, k)?;
        let r = self.monde_regeln().unwrap();
        let nach = self.mond_an(gegenstelle).ok_or_else(|| format!("Bei {gegenstelle} gibt es keinen Mond"))?;
        if nach == von { return Err("Gegenstelle ist derselbe Mond".into()); }
        let gb = self.monde.monde[&nach].besitzer;
        if gb != sid && !(self.verbuendet(sid, gb) && self.monde.freigaben.contains(&nach)) {
            return Err("Gegenstelle muss ein eigener oder freigegebener Mond eines Allianz- oder Bündnispartners sein".into());
        }
        self.tor_bereit(von, sid)?;
        self.tor_bereit(nach, sid)?;
        let (sv, sn) = (self.monde.monde[&von].gebaeude[MondGebaeude::Sprungtor.idx()], self.monde.monde[&nach].gebaeude[MondGebaeude::Sprungtor.idx()]);
        let stufe = sv.min(sn);
        let l = &r.hyperraumloch;
        if k.sektor != gegenstelle.sektor && stufe < l.sektorwechsel_ab_stufe { return Err(format!("Sektorwechsel erst ab Torstufe {}", l.sektorwechsel_ab_stufe)); }
        let abstand = self.tor_abstand(k, gegenstelle);
        let reichweite = l.reichweite_je_stufe * stufe as i64;
        if abstand > reichweite { return Err(format!("Gegenstelle {abstand} Systeme entfernt; Reichweite {reichweite}")); }
        let kosten = (l.deuterium_basis + l.deuterium_je_system * abstand) * M;
        if self.monde.monde[&von].bestand[Gut::Deuterium.idx()] < kosten { return Err(format!("Öffnen kostet {} Deuterium aus dem Mondbestand", ganz(kosten))); }
        let jetzt = self.zeit;
        let abkling = r.abklingzeit(stufe);
        self.monde.monde.get_mut(&von).unwrap().bestand[Gut::Deuterium.idx()] -= kosten;
        for mid in [von, nach] { self.monde.monde.get_mut(&mid).unwrap().sprungtor_bereit = jetzt + abkling; }
        let id = self.monde.naechstes_loch.max(1);
        self.monde.naechstes_loch = id + 1;
        let kap = l.kapazitaet_je_stufe * stufe as i64;
        let schleier = [self.monde.monde[&von].gebaeude[MondGebaeude::Ortungsschleier.idx()], self.monde.monde[&nach].gebaeude[MondGebaeude::Ortungsschleier.idx()]];
        self.monde.loecher.insert(id, Hyperraumloch {
            id, oeffner: sid, von, nach, von_koord: k, nach_koord: gegenstelle, geoeffnet: jetzt, schliesst: jetzt + l.offen_sekunden,
            stufe, kapazitaet: kap, kapazitaet_rest: kap, durchreisen: Vec::new(), benutzt: BTreeSet::new(), schleier,
        });
        // Alle, die das Loch sehen, erfahren davon (Sichtregel 7.3).
        for s in 0..self.spieler.len() as SpielerId {
            if !self.spieler_aktiv(s) || !self.loch_sichtbar(id, s) { continue; }
            let eigene_schiffe = self.schiffe_im_system(s, k) || self.schiffe_im_system(s, gegenstelle);
            self.vorfall(s, "hyperraumloch_offen", format!("Hyperraumloch {id} offen: {k} ↔ {gegenstelle}, schließt {}", zeittext(jetzt + l.offen_sekunden)));
            self.wecke(s, Rolle::Feldherr, "Hyperraumloch offen", eigene_schiffe && s != sid);
        }
        self.fachereignis("wormhole_opened", sid, json!({"wormhole":id,"from":k,"to":gegenstelle,"closes":jetzt + l.offen_sekunden}));
        let mut text = format!("Hyperraumtor {id} geöffnet: {k} ↔ {gegenstelle}, offen bis {}, Kapazität {} Struktur, Abklingzeit {} min", zeittext(jetzt + l.offen_sekunden), kap, abkling / MINUTE);
        if let Some(e) = erst {
            let mut a = e.clone();
            a["loch"] = json!(id);
            if a.get("von").is_none() { a["von"] = json!({"art":"mond","koord":k.to_string()}); }
            match self.hyperraumloch_durchreisen(sid, rolle, &a) {
                Ok(t) => { text.push_str("; Erstdurchreise: "); text.push_str(&t); }
                Err(e) => { text.push_str("; Erstdurchreise abgelehnt: "); text.push_str(&e); }
            }
        }
        Ok(text)
    }

    fn schiffe_im_system(&self, sid: SpielerId, k: Koord) -> bool {
        let gleich = |x: Koord| x.sektor == k.sektor && x.system == k.system;
        self.spieler[sid as usize].planeten.iter().any(|p| { let p = &self.planeten[*p as usize]; gleich(p.koord) && p.einheiten[..SCHIFFE].iter().any(|n| *n > 0) })
            || self.monde.monde.values().any(|m| m.besitzer == sid && gleich(m.koord) && m.einheiten[..SCHIFFE].iter().any(|n| *n > 0))
            || self.flotten.values().any(|f| f.besitzer == sid && f.zustand == Flottenzustand::ImOrbit && gleich(f.ziel))
    }

    /// Sichtmenge eines Lochs (Konzept 7.3), dynamisch ausgewertet.
    pub fn loch_sichtbar(&self, id: u64, sid: SpielerId) -> bool {
        let Some(l) = self.monde.loecher.get(&id) else { return false };
        let Some(r) = self.monde_regeln() else { return false };
        if sid == l.oeffner || self.verbuendet(sid, l.oeffner) { return true; }
        if self.monde.monde.get(&l.nach).is_some_and(|m| m.besitzer == sid) || self.monde.monde.get(&l.von).is_some_and(|m| m.besitzer == sid) { return true; }
        for (i, k) in [l.von_koord, l.nach_koord].into_iter().enumerate() {
            let gleich = |x: Koord| x.sektor == k.sektor && x.system == k.system;
            if self.spieler[sid as usize].planeten.iter().any(|p| gleich(self.planeten[*p as usize].koord)) { return true; }
            if self.monde.monde.values().any(|m| m.besitzer == sid && gleich(m.koord)) { return true; }
            if self.flotten.values().any(|f| f.besitzer == sid && f.zustand == Flottenzustand::ImOrbit && gleich(f.ziel)) { return true; }
            for m in self.monde.monde.values().filter(|m| m.besitzer == sid) {
                let s = self.phalanx_stufe_eff(m.id);
                if s == 0 || s <= l.schleier[i] { continue; }
                let reichweite = r.phalanx_reichweite(s);
                if m.koord.sektor == k.sektor && (m.koord.system as i64 - k.system as i64).abs() <= reichweite { return true; }
            }
        }
        false
    }

    pub fn hyperraumloch_durchreisen(&mut self, sid: SpielerId, _rolle: Rolle, a: &Value) -> Result<String, String> {
        let r = self.monde_regeln().ok_or("Monde sind in dieser Partie nicht aktiv")?;
        let id = feld(a, "loch").and_then(Value::as_u64).ok_or("Feld loch fehlt")?;
        let l = self.monde.loecher.get(&id).cloned().ok_or("Dieses Hyperraumloch ist nicht offen")?;
        if self.zeit >= l.schliesst { return Err("Das Hyperraumloch hat sich geschlossen".into()); }
        if !self.loch_sichtbar(id, sid) { return Err("Dieses Hyperraumloch ist nicht offen".into()); }
        if self.zeit < self.spieler[sid as usize].schutz_bis { return Err("Unter Anfängerschutz ist keine Durchreise möglich".into()); }
        let danach = feld(a, "danach").and_then(Value::as_str).unwrap_or("orbit");
        if !["landen", "orbit"].contains(&danach) { return Err("danach: landen oder orbit".into()); }
        let von = feld(a, "von").ok_or("Feld von fehlt")?;
        let art = von.get("art").and_then(Value::as_str).unwrap_or("");
        let schiffe_map: BTreeMap<Einheit, i64> = serde_json::from_value(feld(a, "schiffe").cloned().unwrap_or(json!({}))).map_err(|e| format!("schiffe: {e}"))?;
        let mut schiffe = [0i64; SCHIFFE];
        for (e, n) in &schiffe_map {
            if !e.ist_schiff() { return Err("Verteidigung und Solarsatelliten reisen nicht durch Hyperraumlöcher".into()); }
            if *e == Einheit::Weltentruemmer && *n > 0 { return Err("Weltenkrümmer passen durch kein Hyperraumloch".into()); }
            if *n < 0 || *n > 1_000_000 { return Err("Schiffszahl ungültig".into()); }
            schiffe[e.idx()] = *n;
        }
        // Mitgeführte Ladung bei Durchreise vom Planeten oder Mond, vor allem Deuterium für Weiter- und Rückflug
        // (Karl, 18.10.2026: "man sollte Deuterium dabei haben"). Orbitflotten reisen mit ihrer vorhandenen Ladung.
        let ladung_map: BTreeMap<Gut, f64> = serde_json::from_value(feld(a, "ladung").cloned().unwrap_or(json!({}))).map_err(|e| format!("ladung: {e}"))?;
        let mut mit = [0i64; GUETER];
        for (g, x) in &ladung_map {
            if !x.is_finite() || *x < 0.0 || *x > 1e12 { return Err("ladung muss eine nichtnegative Menge sein".into()); }
            mit[g.idx()] = milli(*x);
        }
        let gleich = |x: Koord, k: Koord| x.sektor == k.sektor && x.system == k.system;
        // Herkunft bestimmen.
        let (herkunft, ort, flotte): (String, Koord, Option<FlottenId>) = match art {
            "planet" => {
                let k: Koord = von.get("koord").and_then(Value::as_str).ok_or("von.koord fehlt")?.parse()?;
                let pid = self.eigener_planet(sid, k)?;
                (format!("planet:{pid}"), k, None)
            }
            "mond" => {
                let k: Koord = von.get("koord").and_then(Value::as_str).ok_or("von.koord fehlt")?.parse()?;
                let mid = self.eigener_mond(sid, k)?;
                (format!("mond:{mid}"), k, None)
            }
            "flotte" => {
                let fid = von.get("flotte").and_then(Value::as_u64).ok_or("von.flotte fehlt")? as FlottenId;
                let f = self.flotten.get(&fid).filter(|f| f.besitzer == sid).ok_or("Keine berechtigte Flotte unter dieser Kennung")?;
                if f.zustand != Flottenzustand::ImOrbit { return Err("Nur Flotten im Orbit dürfen mitreisen; Flotten im Flug sind zwischen den Systemen".into()); }
                if self.ausbau.expeditionen.contains_key(&fid) { return Err("Expeditionen reisen nicht durch Hyperraumlöcher".into()); }
                for e in 0..SCHIFFE { schiffe[e] = f.schiffe[e]; }
                if schiffe[Einheit::Weltentruemmer.idx()] > 0 { return Err("Weltenkrümmer passen durch kein Hyperraumloch".into()); }
                (format!("flotte:{fid}"), f.ziel, Some(fid))
            }
            _ => return Err("von.art: planet, mond oder flotte".into()),
        };
        let vorwaerts = if gleich(ort, l.von_koord) { true } else if gleich(ort, l.nach_koord) { false } else {
            return Err("Mitreisen darf nur, wer im Start- oder Zielsystem des Lochs steht".into());
        };
        if l.benutzt.contains(&herkunft) { return Err("Diese Flotte bzw. dieser Ort hat das Loch schon benutzt".into()); }
        if !schiffe.iter().any(|n| *n > 0) { return Err("keine Schiffe gewählt".into()); }
        // Verfügbarkeit prüfen.
        match art {
            "planet" => { let pid = self.eigener_planet(sid, ort)?; for e in 0..SCHIFFE { if schiffe[e] > self.planeten[pid].einheiten[e] { return Err(format!("auf {ort} stehen nur {} x {}", self.planeten[pid].einheiten[e], Einheit::ALLE[e])); } } }
            "mond" => { let mid = self.eigener_mond(sid, ort)?; for e in 0..SCHIFFE { if schiffe[e] > self.monde.monde[&mid].einheiten[e] { return Err(format!("auf Mond {ort} stehen nur {} x {}", self.monde.monde[&mid].einheiten[e], Einheit::ALLE[e])); } } }
            _ => {}
        }
        let struktur: i64 = (0..SCHIFFE).map(|e| schiffe[e] * self.regeln.einh(Einheit::ALLE[e]).struktur).sum();
        if struktur > l.kapazitaet_rest { return Err(format!("Kapazität des Lochs überschritten: {struktur} Struktur, frei {}", l.kapazitaet_rest)); }
        let (aus_koord, aus_mond) = if vorwaerts { (l.nach_koord, l.nach) } else { (l.von_koord, l.von) };
        if mit.iter().any(|x| *x > 0) {
            let pid = match art {
                "planet" => self.eigener_planet(sid, ort)?,
                "mond" => self.monde.monde[&self.eigener_mond(sid, ort)?].mutter as usize,
                _ => return Err("Eine Orbitflotte reist mit ihrer vorhandenen Ladung; ladung nur bei Herkunft Planet oder Mond".into()),
            };
            let frei = self.flugplan(sid, pid, aus_koord, &schiffe, 1000)?.kapazitaet;
            if mit.iter().sum::<i64>() > frei { return Err(format!("Ladung {} übersteigt die Ladekapazität {}", ganz(mit.iter().sum::<i64>()), ganz(frei))); }
        }
        let aus_besitzer = self.monde.monde.get(&aus_mond).map(|m| m.besitzer).ok_or("Austrittsmond existiert nicht mehr")?;
        if danach == "landen" && aus_besitzer != sid { return Err("Landen nur am eigenen Austrittsmond; sonst danach = orbit".into()); }
        let flotten_neu = flotte.is_none() && danach == "orbit";
        if flotten_neu {
            let plaetze = self.flottenplaetze(sid);
            if self.flotten.values().filter(|f| f.besitzer == sid).count() >= plaetze { return Err(format!("alle {plaetze} Flottenplätze sind belegt")); }
        }
        // Gebühr: Anteil des normalen Einwegtreibstoffs zwischen den Toren.
        let d = self.entfernung(l.von_koord, l.nach_koord);
        // Technik v1: Verbrauch nach Antriebswechsel (ohne Technik v1 wie die Einheitenregel).
        let verbrauch: i64 = (0..SCHIFFE).map(|e| schiffe[e] * self.verbrauch(sid, Einheit::ALLE[e])).sum();
        let roh = M as i128 + verbrauch as i128 * M as i128 * d as i128 / self.regeln.flug.treibstoff_teiler.max(1) as i128;
        let mut gebuehr = mal(roh as i64, r.hyperraumloch.gebuehr_anteil);
        // Ablauf-Version 1: Bleibt die Flotte nach der Durchreise im Orbit, fliegt sie später von der Gegenstelle heim –
        // dieser Rückflug wird gleich mitbezahlt (normaler Einwegtreibstoff), sonst wäre er kostenlos.
        if self.ablauf_v1() && danach != "landen" {
            let heim_k = match art {
                "planet" | "mond" => ort,
                _ => { let f = &self.flotten[&flotte.unwrap()]; self.monde.flottenstart.get(&f.id).and_then(|m| self.monde.monde.get(m)).map(|m| m.koord).unwrap_or(f.start_koord) }
            };
            let d2 = self.entfernung(aus_koord, heim_k);
            gebuehr += (M as i128 + verbrauch as i128 * M as i128 * d2 as i128 / self.regeln.flug.treibstoff_teiler.max(1) as i128) as i64;
        }
        let deut = Gut::Deuterium.idx();
        match art {
            "planet" => {
                let pid = self.eigener_planet(sid, ort)?;
                self.abrechnen(pid);
                if self.planeten[pid].bestand[deut] < gebuehr + mit[deut] { return Err(format!("Gebühr {} und Ladung {} Deuterium fehlen auf {ort}", ganz(gebuehr), ganz(mit[deut]))); }
                if let Some(g) = (0..GUETER).find(|g| *g != deut && self.planeten[pid].bestand[*g] < mit[*g]) { return Err(format!("Ladung {} {} fehlt auf {ort}", ganz(mit[g]), Gut::ALLE[g])); }
            }
            "mond" => {
                let mid = self.eigener_mond(sid, ort)?;
                let b = &self.monde.monde[&mid].bestand;
                if b[deut] < gebuehr + mit[deut] { return Err(format!("Gebühr {} und Ladung {} Deuterium fehlen auf Mond {ort}", ganz(gebuehr), ganz(mit[deut]))); }
                if let Some(g) = (0..GUETER).find(|g| *g != deut && b[*g] < mit[*g]) { return Err(format!("Ladung {} {} fehlt auf Mond {ort}", ganz(mit[g]), Gut::ALLE[g])); }
            }
            _ => { if self.flotten[&flotte.unwrap()].ladung[deut] < gebuehr { return Err(format!("Gebühr {} Deuterium fehlt in der Ladung", ganz(gebuehr))); } }
        }
        // Ab hier keine Ablehnung mehr: abbuchen und reisen.
        let jetzt = self.zeit;
        let heim_pid: usize;
        let mut heim_mond: Option<MondId> = None;
        let ladung: [i64; GUETER];
        match art {
            "planet" => {
                let pid = self.eigener_planet(sid, ort)?;
                let p = &mut self.planeten[pid];
                p.bestand[deut] -= gebuehr;
                for g in 0..GUETER { p.bestand[g] -= mit[g]; }
                ladung = mit;
                for e in 0..SCHIFFE { p.einheiten[e] -= schiffe[e]; }
                heim_pid = pid;
                self.raten_neu(pid);
            }
            "mond" => {
                let mid = self.eigener_mond(sid, ort)?;
                let m = self.monde.monde.get_mut(&mid).unwrap();
                m.bestand[deut] -= gebuehr;
                for g in 0..GUETER { m.bestand[g] -= mit[g]; }
                ladung = mit;
                for e in 0..SCHIFFE { m.einheiten[e] -= schiffe[e]; }
                heim_pid = m.mutter as usize;
                heim_mond = Some(mid);
            }
            _ => {
                let f = self.flotten.get_mut(&flotte.unwrap()).unwrap();
                f.ladung[deut] -= gebuehr;
                ladung = f.ladung;
                heim_pid = f.start as usize;
                heim_mond = self.monde.flottenstart.get(&f.id).copied();
            }
        }
        let mut benutzt = vec![herkunft.clone()];
        let mut neue_flotte = None;
        let mut torwache = None;
        if danach == "landen" {
            let m = self.monde.monde.get_mut(&aus_mond).unwrap();
            for e in 0..SCHIFFE { m.einheiten[e] += schiffe[e]; }
            for g in 0..GUETER { m.bestand[g] += ladung[g]; }
            if let Some(fid) = flotte {
                self.flotten.remove(&fid);
                self.monde.flottenziel.remove(&fid);
                self.monde.flottenstart.remove(&fid);
                self.kolonisation.rueckwege.remove(&fid);
            }
            benutzt.push(format!("mond:{aus_mond}"));
        } else {
            let freund = aus_besitzer == sid || self.verbuendet(sid, aus_besitzer);
            let mission = if freund { Mission::Halten } else { Mission::Saven };
            let heim = heim_mond.and_then(|m| self.monde.monde.get(&m)).map(|m| m.koord).unwrap_or(self.planeten[heim_pid].koord);
            let tempo = (0..SCHIFFE).filter(|e| schiffe[*e] > 0).map(|e| self.tempo(sid, Einheit::ALLE[e])).min().unwrap_or(1).max(1);
            let rueck = self.flugdauer(self.entfernung(aus_koord, heim), tempo, 1000);
            // Ablauf-Version 1: Orbit bei einem fremden Mond gilt als Saven und endet wie dort nach höchstens 72 h.
            let ende = jetzt + if self.ablauf_v1() && !freund { 72 } else { self.regeln.flug.halten_max_stunden } * STUNDE;
            let fid = match flotte {
                Some(fid) => {
                    let f = self.flotten.get_mut(&fid).unwrap();
                    f.ziel = aus_koord;
                    f.mission = mission;
                    f.flugdauer = rueck;
                    f.orbit_ende = ende;
                    f.haltedauer = ende - jetzt;
                    fid
                }
                None => {
                    let fid = self.naechste_flotte;
                    self.naechste_flotte += 1;
                    self.flotten.insert(fid, Flotte {
                        id: fid, besitzer: sid, start: self.planeten[heim_pid].id, start_koord: ort, ziel: aus_koord, mission, schiffe, ladung, siedler: 0,
                        abflug: jetzt, ankunft: jetzt, flugdauer: rueck, rueckkehr: 0, orbit_ende: ende, haltedauer: ende - jetzt, zustand: Flottenzustand::ImOrbit,
                        gemeldet: true, verband: None,
                    });
                    if let Some(mid) = heim_mond { self.monde.flottenstart.insert(fid, mid); }
                    if self.aufklaerungsregeln() { self.aufklaerung.abschirmung.insert(fid, self.spieler[sid as usize].forschung[Forschung::Abschirmtechnik.idx()]); }
                    neue_flotte = Some(fid);
                    fid
                }
            };
            self.ausbau.rueckflugplaene.remove(&fid);
            self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
            if freund { self.monde.flottenziel.insert(fid, aus_mond); } else { self.monde.flottenziel.remove(&fid); }
            benutzt.push(format!("flotte:{fid}"));
            // Torwache: Kampf gegen alle ohne Bündnis oder Nichtangriffspakt.
            if !freund && !self.vertrag_zwischen(sid, aus_besitzer, Vertragsart::Nichtangriffspakt) {
                let nr = self.kampf_nr;
                self.monde.flottenziel.insert(fid, aus_mond);
                let _ = self.mond_kampf(fid, aus_mond);
                self.monde.flottenziel.remove(&fid);
                torwache = Some(nr);
            }
        }
        let l = self.monde.loecher.get_mut(&id).unwrap();
        l.kapazitaet_rest -= struktur;
        for b in benutzt { l.benutzt.insert(b); }
        l.durchreisen.push(Durchreise {
            zeit: jetzt, spieler: sid, vorwaerts, herkunft: herkunft.clone(), flotte: neue_flotte.or(flotte), schiffe: schiffe_map.into_iter().filter(|(_, n)| *n > 0).collect(),
            struktur, gebuehr, gelandet: danach == "landen", torwache_kampf: torwache,
        });
        self.fachereignis("wormhole_transit", sid, json!({"wormhole":id,"forward":vorwaerts,"exit":aus_koord,"structure":struktur,"gate_guard_combat":torwache}));
        Ok(format!("Durchreise durch Hyperraumtor {id} nach {aus_koord}: {struktur} Struktur, Gebühr {} Deuterium{}{}", ganz(gebuehr),
            if danach == "landen" { ", gelandet" } else { ", im Orbit" }, torwache.map(|n| format!(", Torwache-Kampf {n}")).unwrap_or_default()))
    }

    /// Raketenschlag auf die Anlagen eines Mondes (Prüfung über den Mutterplaneten an derselben Koordinate).
    pub fn mond_raketen_starten(&mut self, sid: SpielerId, a: &Value) -> Result<String, String> {
        let start = koord_feld(a, "start")?;
        let ziel = koord_feld(a, "ziel")?;
        let anzahl = feld(a, "anzahl").and_then(Value::as_i64).ok_or("Feld anzahl fehlt")?;
        let zieltyp = feld(a, "zieltyp").and_then(Value::as_str).and_then(Einheit::aus_name).ok_or("Feld zieltyp fehlt")?;
        let art = feld(a, "art").and_then(Value::as_str).unwrap_or("interplanetar");
        let mid = self.mond_an(ziel).ok_or_else(|| format!("Bei {ziel} gibt es keinen Mond"))?;
        if zieltyp.ist_schiff() { return Err("Raketen treffen auf Monden nur Anlagen und Solarsatelliten".into()); }
        if self.belegung.get(&ziel).is_none_or(|p| self.planeten[*p as usize].besitzer != self.monde.monde[&mid].besitzer) {
            return Err("Mond und Mutterplanet haben verschiedene Besitzer; Ziel nicht eindeutig".into());
        }
        let vorher = self.seq;
        let text = self.taktisch_starten(sid, start, ziel, anzahl, zieltyp, art, None)?;
        let id = vorher + 1;
        if self.ausbau.raketensystem.salven.contains_key(&id) { self.monde.raketenziel.insert(id); }
        Ok(format!("{text} (Ziel: Mond {ziel})"))
    }

    /// Einschlag einer Salve mit Zielart Mond. `true`: erledigt.
    pub(crate) fn mond_rakete(&mut self, id: u64) -> bool {
        if !self.monde.raketenziel.remove(&id) { return false; }
        let Some(s) = self.ausbau.raketensystem.salven.remove(&id) else { return true };
        let r = crate::raketen::regeln(&self.regeln).remove(s.art as usize);
        let mid = self.mond_an(s.ziel);
        let gueltig = mid.is_some_and(|m| self.monde.monde[&m].besitzer == s.gegner) && self.spieler_aktiv(s.von) && self.spieler_aktiv(s.gegner)
            && self.zeit >= self.spieler[s.gegner as usize].schutz_bis && !self.verbuendet(s.von, s.gegner);
        if !gueltig {
            let msg = format!("Die {}-Salve auf den Mond {} findet kein zulässiges Ziel mehr und zerstört sich ohne Treffer.", r.name, s.ziel);
            self.vorfall(s.von, "raketen", msg.clone());
            self.vorfall(s.gegner, "raketen", msg);
            return true;
        }
        let mid = mid.unwrap();
        let owner = &self.spieler[s.gegner as usize];
        let armor = crate::raketen::panzerung(&self.regeln, s.zieltyp, owner.forschung[Forschung::Panzerung.idx()], owner.volk);
        let vorhanden = self.monde.monde[&mid].einheiten[s.zieltyp.idx()];
        // Monde haben kein Raketensilo: keine Abfangraketen.
        let zerstoert = crate::raketen::zerstoert(&r, s.art, s.anzahl, s.schaden, s.zieltyp, armor, vorhanden);
        let kosten = self.regeln.kosten_einheit(s.zieltyp, owner.volk);
        self.monde.monde.get_mut(&mid).unwrap().einheiten[s.zieltyp.idx()] -= zerstoert;
        self.spieler[s.gegner as usize].statistik.verluste += self.regeln.wert(&kosten) * zerstoert;
        self.spieler[s.gegner as usize].statistik.verteidigungen += 1;
        if zerstoert > 0 {
            let t = self.truemmer.entry(s.ziel).or_insert([0, 0]);
            t[0] += mal(kosten[Gut::Erz.idx()] * zerstoert, self.regeln.kampf.truemmer_anteil);
            t[1] += mal(kosten[Gut::Kristall.idx()] * zerstoert, self.regeln.kampf.truemmer_anteil);
        }
        let msg = format!("{} auf den Mond {}: {} gestartet, {} {} zerstört. Keine Beute, kein Wiederaufbau.", r.name, s.ziel, s.anzahl, zerstoert, s.zieltyp);
        self.vorfall(s.von, "raketen", msg.clone());
        self.vorfall(s.gegner, "raketen", msg);
        self.wecke(s.gegner, Rolle::Feldherr, "Raketeneinschlag", true);
        self.punkte_neu();
        true
    }

    fn loch_schliessen(&mut self, id: u64, grund: &str) {
        let Some(l) = self.monde.loecher.remove(&id) else { return };
        self.vorfall(l.oeffner, "hyperraumloch_zu", format!("Hyperraumloch {id} ({} ↔ {}) geschlossen: {grund}", l.von_koord, l.nach_koord));
        self.monde.loecher_zu.push(l);
        if self.monde.loecher_zu.len() > 20 { self.monde.loecher_zu.remove(0); }
    }
}

// ---------------------------------------------------------------- Aktionen, Sicht, Schema

pub const MOND_AKTIONEN: [&str; 8] = ["mond_bauen", "mond_fertigen", "mond_umbenennen", "mond_faehre", "phalanx_scannen", "sprungtor_freigabe", "hyperraumloch_oeffnen", "hyperraumloch_durchreisen"];

pub fn mond_zustaendig(typ: &str) -> &'static [Rolle] {
    use Rolle::*;
    match typ {
        "mond_bauen" | "mond_faehre" => &[Verwalter],
        "mond_fertigen" => &[Verwalter, Feldherr],
        "mond_umbenennen" => &[Stratege, Verwalter],
        "sprungtor_freigabe" => &[Feldherr, Diplomat],
        _ => &[Feldherr],
    }
}

impl Welt {
    /// Mondaktionen; `None`, wenn `typ` keine ist.
    pub(crate) fn mond_aktion(&mut self, sid: SpielerId, rolle: Rolle, a: &Value) -> Option<Result<String, String>> {
        let typ = a.get("typ").and_then(Value::as_str)?;
        if typ == "raketen_starten" && a.get("zielart").and_then(Value::as_str) == Some("mond") {
            if self.monde_regeln().is_none() { return Some(Err("Monde sind in dieser Partie nicht aktiv".into())); }
            if rolle != Rolle::Alle && rolle != Rolle::Feldherr { return Some(Err(format!("raketen_starten gehört nicht zur Rolle {rolle}, zuständig: feldherr"))); }
            return Some(self.mond_raketen_starten(sid, a));
        }
        let flug_mond = typ == "flotte_senden" && (a.get("zielart").and_then(Value::as_str) == Some("mond") || a.get("startart").and_then(Value::as_str) == Some("mond") || a.get("mission").and_then(Value::as_str) == Some("mond_zerstoeren"));
        if !MOND_AKTIONEN.contains(&typ) && !flug_mond { return None; }
        if self.monde_regeln().is_none() { return Some(Err("Monde sind in dieser Partie nicht aktiv".into())); }
        if flug_mond {
            let mission = a.get("mission").and_then(Value::as_str).unwrap_or("");
            let rollen: &[Rolle] = if ["transport", "stationieren"].contains(&mission) { &[Rolle::Verwalter, Rolle::Feldherr] } else { &[Rolle::Feldherr] };
            if rolle != Rolle::Alle && !rollen.contains(&rolle) { return Some(Err(format!("flotte_senden mit Mission {mission} gehört nicht zur Rolle {rolle}"))); }
            return Some(self.mond_flotte_senden(sid, rolle, a));
        }
        if rolle != Rolle::Alle && !mond_zustaendig(typ).contains(&rolle) {
            let wer: Vec<&str> = mond_zustaendig(typ).iter().map(|r| r.name()).collect();
            return Some(Err(format!("{typ} gehört nicht zur Rolle {rolle}, zuständig: {}", wer.join(", "))));
        }
        let k = |f: &str| koord_feld(a, f);
        Some((|| -> Result<String, String> {
            match typ {
                "mond_bauen" => {
                    let g = a.get("gebaeude").and_then(Value::as_str).and_then(MondGebaeude::aus_name).ok_or("Unbekanntes Mondgebäude")?;
                    self.mond_bauen(sid, k("mond")?, g)
                }
                "mond_fertigen" => {
                    let e = a.get("einheit").and_then(Value::as_str).and_then(Einheit::aus_name).ok_or("Unbekannte Einheit")?;
                    self.mond_fertigen(sid, rolle, k("mond")?, e, a.get("anzahl").and_then(Value::as_i64).unwrap_or(1))
                }
                "mond_umbenennen" => self.mond_umbenennen(sid, k("mond")?, a.get("name").and_then(Value::as_str).unwrap_or("")),
                "mond_faehre" => {
                    let g = a.get("gut").and_then(Value::as_str).and_then(Gut::aus_name).ok_or("Unbekanntes Gut")?;
                    let menge = a.get("menge_je_stunde").and_then(Value::as_f64).ok_or("Feld menge_je_stunde fehlt")?;
                    if !menge.is_finite() { return Err("menge_je_stunde ungültig".into()); }
                    self.mond_faehre(sid, k("mond")?, g, menge.round() as i64)
                }
                "phalanx_scannen" => self.phalanx_scannen(sid, k("mond")?, k("ziel")?, a.get("zielart").and_then(Value::as_str) == Some("mond")),
                "sprungtor_freigabe" => self.sprungtor_freigabe(sid, k("mond")?, a.get("freigegeben").and_then(Value::as_bool).unwrap_or(true)),
                "hyperraumloch_oeffnen" => self.hyperraumloch_oeffnen(sid, rolle, k("mond")?, k("gegenstelle")?, a.get("erstdurchreise").filter(|v| !v.is_null())),
                "hyperraumloch_durchreisen" => self.hyperraumloch_durchreisen(sid, rolle, a),
                _ => Err("unbekannte Mondaktion".into()),
            }
        })())
    }

    /// Volle Mondansicht für den Besitzer (gleiche Form wie `monde_sicht`.eigene[]).
    pub fn mond_sicht(&self, mid: MondId) -> Value {
        if self.monde_regeln().is_none() || !self.monde.monde.contains_key(&mid) { return Value::Null; }
        self.mond_sicht_eigen(mid)
    }
    fn mond_sicht_eigen(&self, mid: MondId) -> Value {
        let r = self.monde_regeln().unwrap();
        let m = &self.monde.monde[&mid];
        let sp = &self.spieler[m.besitzer as usize];
        let (belegt, verfuegbar, obergrenze) = self.mond_felder(mid);
        let (erz, verb) = self.mond_energie(mid);
        let baubar: Vec<Value> = MondGebaeude::ALLE.iter().map(|g| {
            let regel = r.geb(*g);
            let geplant = m.bauschleife.iter().filter(|a| a.gebaeude == *g).count() as u8;
            let stufe = m.gebaeude[g.idx()] + geplant + 1;
            let kosten = r.kosten(*g, stufe);
            let mut fehlt: Vec<String> = Vec::new();
            if sp.stufe < regel.ab_stufe { fehlt.push(format!("Zivilisationsstufe {}", regel.ab_stufe)); }
            for (b, st) in &regel.braucht { if m.gebaeude[b.idx()] < *st { fehlt.push(format!("{} {st}", b.name())); } }
            for (f, n) in &regel.forschung { if sp.forschung[f.idx()] < *n { fehlt.push(format!("{f} {n}")); } }
            if regel.max_stufe > 0 && stufe > regel.max_stufe { fehlt.push(format!("höchstens Stufe {}", regel.max_stufe)); }
            // Gleiche Ausnahme wie mond_bauen: Mondbasis auf vollem Mond (Ablauf v3, eine Stufe zur Zeit) – Funktionsprüfung 25.10.2026.
            if belegt >= verfuegbar && !(self.ablauf_v3() && *g == MondGebaeude::Mondbasis && geplant == 0) { fehlt.push("freies Feld".into()); }
            let rohstoffe: Vec<&str> = Gut::ALLE.iter().filter(|x| m.bestand[x.idx()] < kosten[x.idx()]).map(|x| x.name()).collect();
            json!({"gebaeude":g.name(),"stufe":stufe,"kosten":gueter_text(&kosten),"bauzeit_sekunden":self.mond_bauzeit(mid,&kosten,MondGebaeude::Mondbasis).max(self.regeln.wirtschaft.min_bauzeit_sekunden),
                "energie":regel.energie,"wirkung":regel.wirkung,"fehlt":fehlt,"fehlende_rohstoffe":rohstoffe,"moeglich":fehlt.is_empty() && rohstoffe.is_empty()})
        }).collect();
        let tor = m.gebaeude[MondGebaeude::Sprungtor.idx()];
        let phalanx = self.phalanx_stufe_eff(mid);
        let werft: Vec<Value> = Einheit::ALLE.iter().filter(|e| self.regeln.einheiten.contains_key(e) && !matches!(e, Einheit::Planetenschild | Einheit::PlanetenschildII | Einheit::PlanetenschildIII)).map(|e| {
            let er = self.regeln.einh(*e);
            let noetig = if *e == Einheit::Weltentruemmer { r.weltentruemmer_werftdock } else { er.werft.max(1) };
            let k = self.regeln.kosten_einheit(*e, sp.volk);
            json!({"einheit":e.name(),"schiff":e.ist_schiff(),"werftdock":noetig,"kosten":gueter_text(&k),"bauzeit_sekunden":self.mond_fertigungszeit(mid,&k),
                "baubar":m.gebaeude[MondGebaeude::Werftdock.idx()]>=noetig && sp.stufe>=er.ab_stufe && self.einheit_forschung_fehlend(m.besitzer,*e).is_empty()})
        }).collect();
        json!({
            "id":m.id,"koord":m.koord.to_string(),"name":m.name,"typ":m.typ.name(),"karte":format!("moons.{}.map",m.typ.name()),
            "karte_volk":format!("moons.{}.{}.map",m.typ.name(),sp.volk.name()),"durchmesser":m.durchmesser,"mutterplanet":self.planeten[m.mutter as usize].koord.to_string(),
            "entstanden":m.entstanden,"alter_sekunden":(self.zeit-m.entstanden).max(0),"volk":sp.volk.name(),"kampf":m.kampf_nr,
            "felder":{"belegt":belegt,"verfuegbar":verfuegbar,"obergrenze":obergrenze},
            "gebaeude":MondGebaeude::ALLE.iter().map(|g|(g.name(),m.gebaeude[g.idx()])).collect::<BTreeMap<_,_>>(),
            "bauschleife":m.bauschleife.iter().map(|a|json!({"gebaeude":a.gebaeude.name(),"stufe":a.stufe,"fertig":a.fertig,"dauer":a.dauer})).collect::<Vec<_>>(),
            "baubar":baubar,
            "bestand":gueter_text(&m.bestand),"lager":gueter_text(&self.mond_lager(mid)),"kryolager_geschuetzt_anteil":if m.gebaeude[MondGebaeude::Kryolager.idx()]>0 {r.kryolager_geschuetzt} else {0.0},
            "foerderung_je_stunde":gueter_text(&self.mond_foerderung(mid)),
            "energie":{"erzeugung":erz as f64/M as f64,"verbrauch":verb as f64/M as f64,"faktor_promille":self.mond_energiefaktor(mid),"je_solarsonde":solarsonde_energie_an(&self.regeln,&r,m.koord,sp.forschung[Forschung::Energietechnik.idx()],self.regeln.volk(sp.volk).energie,r.typ(m.typ).solar_mod) as f64/M as f64},
            "wirkung_promille":self.mond_wirkung(mid),
            "schiffe":einheiten_text(&m.einheiten,true),"verteidigung":einheiten_text(&m.einheiten,false),
            "werftdock":{"stufe":m.gebaeude[MondGebaeude::Werftdock.idx()],"schleife":m.fertigung.iter().map(|f|json!({"einheit":f.einheit.name(),"rest":f.rest,"dauer":f.dauer})).collect::<Vec<_>>(),"naechste":m.fertigung_naechste,"einheiten":werft},
            "reparatur":m.reparatur.iter().map(|l|json!({"kampf":l.nr,"schiffe":l.schiffe.iter().map(|(e,n)|(e.name(),*n)).collect::<BTreeMap<_,_>>(),"fertig":l.fertig})).collect::<Vec<_>>(),
            "sprungtor":{"stufe":tor,"bereit_ab":m.sprungtor_bereit,"bereit":tor>0 && self.zeit>=m.sprungtor_bereit && self.mond_voll_versorgt(mid),"freigegeben":self.monde.freigaben.contains(&mid),
                "reichweite_systeme":r.hyperraumloch.reichweite_je_stufe*tor as i64,"kapazitaet":r.hyperraumloch.kapazitaet_je_stufe*tor as i64,"abklingzeit_minuten":if tor>0 {r.abklingzeit(tor)/MINUTE} else {0},
                "ziele":self.monde.monde.values().filter(|z|z.id!=mid && z.gebaeude[MondGebaeude::Sprungtor.idx()]>0 && (z.besitzer==m.besitzer || (self.verbuendet(m.besitzer,z.besitzer) && self.monde.freigaben.contains(&z.id)))).map(|z|json!({"koord":z.koord.to_string(),"name":z.name,"stufe":z.gebaeude[MondGebaeude::Sprungtor.idx()],"abstand":self.tor_abstand(m.koord,z.koord),"bereit_ab":z.sprungtor_bereit,
                    // Öffnungskosten aus dem Mondlager (wie hyperraumloch_oeffnen), damit die Oberfläche sie vor dem Tipp nennt.
                    "deuterium":r.hyperraumloch.deuterium_basis + r.hyperraumloch.deuterium_je_system * self.tor_abstand(m.koord,z.koord)})).collect::<Vec<_>>()},
            "tiefenphalanx":{"stufe":m.gebaeude[MondGebaeude::Tiefenphalanx.idx()],"stufe_wirksam":phalanx,"reichweite_systeme":r.phalanx_reichweite(phalanx).max(0),"bereit_ab":m.phalanx_bereit,
                "scan_deuterium":mal(r.tiefenphalanx.scan_deuterium,1.0+r.typ(m.typ).scan_mod)},
            "faehren":self.monde.faehren.iter().filter(|f|f.mond==mid).map(|f|json!({"gut":f.gut.name(),"menge_je_stunde":f.menge_je_stunde})).collect::<Vec<_>>(),
            "faehre_deckel_je_stunde":r.faehre_je_mondbasis_stunde*m.gebaeude[MondGebaeude::Mondbasis.idx()] as i64,
            "unterhalt":{"credits_je_tag":self.mond_unterhalt_tag(mid) as f64/M as f64,"verzug":m.unterhalt_verzug},
            "blockiert":m.blockade.is_some(),"typ_regel":r.typ(m.typ),
        })
    }

    /// Kartensicht eines Mondes an `k` für `sid` (Existenz, Typ, Durchmesser, Besitzer).
    pub fn mond_kartensicht(&self, sid: SpielerId, k: Koord) -> Option<Value> {
        self.monde_regeln()?;
        let mid = self.mond_an(k)?;
        let m = &self.monde.monde[&mid];
        if m.besitzer != sid && self.aufklaerungsregeln() && !self.system_erfasst(sid, k) { return None; }
        let volk = self.spieler[m.besitzer as usize].volk;
        // `bedroht`: feindlicher Anflug auf diesen Mond, nur für Besitzer und Verbündete sichtbar (Basiswarnung).
        let bedroht = (m.besitzer == sid || self.verbuendet(sid, m.besitzer)) && self.flotten.values().any(|f| f.zustand == Flottenzustand::Hinflug
            && f.mission.feindlich() && f.gemeldet && f.besitzer != m.besitzer && !self.verbuendet(f.besitzer, m.besitzer) && self.monde.flottenziel.get(&f.id) == Some(&mid));
        Some(json!({"id":mid,"koord":k.to_string(),"name":m.name,"typ":m.typ.name(),"karte":format!("moons.{}.map",m.typ.name()),"karte_volk":format!("moons.{}.{}.map",m.typ.name(),volk.name()),
            "bild":format!("moons.{}.map",m.typ.name()),"groesse":m.durchmesser,
            // Darstellung: Eigendrehung klingt in den ersten 30 Tagen ab (web-client/planet-kugel.js).
            "alter_sekunden":(self.zeit-m.entstanden).max(0),"volk":volk.name(),
            "durchmesser":m.durchmesser,"besitzer":self.spieler[m.besitzer as usize].name,"eigen":m.besitzer==sid,"bedroht":bedroht,
            "sprungtor":m.gebaeude[MondGebaeude::Sprungtor.idx()]>0 && (m.besitzer==sid || self.verbuendet(sid,m.besitzer))}))
    }

    /// Sichtbares Hyperraumloch mit einem Ende in diesem System: `{id, ziel, offen_bis, von, nach}`.
    pub fn loch_im_system(&self, sid: SpielerId, k: Koord) -> Option<Value> {
        if !self.monde.aktiv { return None; }
        let gleich = |x: Koord| x.sektor == k.sektor && x.system == k.system;
        self.monde.loecher.values().filter(|l| self.loch_sichtbar(l.id, sid)).find_map(|l| {
            let ziel = if gleich(l.von_koord) { l.nach_koord } else if gleich(l.nach_koord) { l.von_koord } else { return None };
            Some(json!({"id":l.id,"ziel":ziel.to_string(),"offen_bis":l.schliesst,"von":l.von_koord.to_string(),"nach":l.nach_koord.to_string(),"kapazitaet_rest":l.kapazitaet_rest}))
        })
    }

    pub fn loecher_sicht(&self, sid: SpielerId) -> Vec<Value> {
        self.monde.loecher.values().filter(|l| self.loch_sichtbar(l.id, sid)).map(|l| {
            let gleich = |x: Koord, k: Koord| x.sektor == k.sektor && x.system == k.system;
            let ende = |k: Koord| self.spieler[sid as usize].planeten.iter().any(|p| gleich(self.planeten[*p as usize].koord, k))
                || self.monde.monde.values().any(|m| m.besitzer == sid && gleich(m.koord, k))
                || self.flotten.values().any(|f| f.besitzer == sid && f.zustand == Flottenzustand::ImOrbit && gleich(f.ziel, k));
            json!({"id":l.id,"von":l.von_koord.to_string(),"nach":l.nach_koord.to_string(),"oeffner":self.spieler[l.oeffner as usize].name,
                "geoeffnet":l.geoeffnet,"schliesst":l.schliesst,"rest_sekunden":(l.schliesst-self.zeit).max(0),"stufe":l.stufe,
                "kapazitaet":l.kapazitaet,"kapazitaet_rest":l.kapazitaet_rest,"durchreisen":l.durchreisen.len(),
                "mitreisen_moeglich_von":([l.von_koord, l.nach_koord]).iter().filter(|k| ende(**k)).map(|k| k.to_string()).collect::<Vec<_>>(),
                "eigene_herkuenfte_benutzt":l.benutzt.iter().cloned().collect::<Vec<_>>()})
        }).collect()
    }

    /// Lagebild der Mondgeneration für einen Spieler.
    pub fn monde_sicht(&self, sid: SpielerId) -> Value {
        let Some(r) = self.monde_regeln() else { return json!({"aktiv":false}) };
        let eigene: Vec<Value> = self.monde.monde.values().filter(|m| m.besitzer == sid).map(|m| self.mond_sicht_eigen(m.id)).collect();
        let bekannt: Vec<Value> = self.monde.monde.values().filter(|m| m.besitzer != sid).filter_map(|m| self.mond_kartensicht(sid, m.koord)).collect();
        let eigene_ids: BTreeSet<MondId> = self.monde.monde.values().filter(|m| m.besitzer == sid).map(|m| m.id).collect();
        let anfluege: Vec<Value> = self.flotten.values().filter(|f| f.zustand == Flottenzustand::Hinflug && f.besitzer != sid && f.mission.feindlich()
            && self.monde.flottenziel.get(&f.id).is_some_and(|m| eigene_ids.contains(m))).filter_map(|f| {
            let mid = self.monde.flottenziel[&f.id];
            let mutter = self.monde.monde[&mid].mutter as usize;
            (self.zeit >= f.ankunft - self.warnzeit(mutter)).then(|| json!({"flotte":f.id,"mond":f.ziel.to_string(),"ankunft":f.ankunft,"ankunft_text":zeittext(f.ankunft)}))
        }).collect();
        json!({
            "aktiv":true,
            "eigene":eigene,
            "bekannt":bekannt,
            "anfluege_auf_monde":anfluege,
            "hyperraumloecher":self.loecher_sicht(sid),
            "phalanxberichte":self.monde.phalanxberichte.iter().filter(|b|b.spieler==sid).rev().take(20).map(|b|json!({"nr":b.nr,"zeit":b.zeit,"zeit_text":zeittext(b.zeit),"mond":self.monde.monde.get(&b.mond).map(|m|m.koord.to_string()),"ziel":b.ziel.to_string(),"kontakte":b.kontakte})).collect::<Vec<_>>(),
            // Lesbar wie Planetenspionage: Namen statt Spielernummern, ganze Güter statt Tausendstel.
            "spionageberichte":self.monde.spionageberichte.iter().filter(|b|b.spieler==sid).rev().take(20).map(|b|json!({
                "art":"mond","ziel":b.koord.to_string(),"mond":true,"zeit":b.zeit,"zeit_text":zeittext(b.zeit),"alter_stunden":(self.zeit-b.zeit)/STUNDE,
                "besitzer":self.spieler[b.besitzer as usize].name,"volk":self.spieler[b.besitzer as usize].volk.name(),
                "scan_metadaten":{"scanstaerke":b.staerke,"schwellen":b.schwellen},
                "bestand":Gut::ALLE.iter().enumerate().map(|(i,g)|(g.name(),crate::typen::ganz(b.bestand.get(i).copied().unwrap_or(0)))).collect::<BTreeMap<_,_>>(),
                "schiffe":b.schiffe,"verteidigung":b.verteidigung,"gebaeude":b.gebaeude,"forschung":b.forschung})).collect::<Vec<_>>(),
            "mondwuerfe":self.monde.wuerfe.values().filter(|w|w.beteiligte.contains(&sid) || w.mond.and_then(|m|self.monde.monde.get(&m)).is_some_and(|m|m.besitzer==sid)).rev().take(20)
                .map(|w|json!({"kampf_nr":w.kampf_nr,"zeit":w.zeit,"zeit_text":zeittext(w.zeit),"ort":w.ort.to_string(),"truemmer":crate::typen::ganz(w.truemmer),
                    "chance_prozent":w.chance_prozent,"wurf":w.wurf,"entstanden":w.mond.is_some(),"typ":w.typ.name(),"durchmesser":w.durchmesser,
                    "beteiligte":w.beteiligte.iter().map(|s|self.spieler[*s as usize].name.clone()).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "flotten_mondziel":self.flotten.values().filter(|f|f.besitzer==sid).filter_map(|f|self.monde.flottenziel.get(&f.id).map(|m|json!({"flotte":f.id,"mond":m}))).collect::<Vec<_>>(),
            "flotten_mondstart":self.flotten.values().filter(|f|f.besitzer==sid).filter_map(|f|self.monde.flottenstart.get(&f.id).map(|m|json!({"flotte":f.id,"mond":m}))).collect::<Vec<_>>(),
            "regeln":{"weltentruemmer_werftdock":r.weltentruemmer_werftdock,"chance_je_100000_truemmer_prozent":100000/r.truemmer_je_prozent.max(1),"zerstoerung_mod":r.typen.iter().map(|(t,x)|(t.name(),x.zerstoerung_mod)).collect::<BTreeMap<_,_>>(),"truemmer_je_prozent":r.truemmer_je_prozent,"chance_max_prozent":r.chance_max_prozent,"loch_offen_sekunden":r.hyperraumloch.offen_sekunden,
                "gebaeude":r.gebaeude.iter().map(|(g,x)|(g.name(),json!({"kosten":x.kosten,"faktor":x.faktor,"energie":x.energie,"ab_stufe":x.ab_stufe,"max_stufe":x.max_stufe,"wirkung":x.wirkung}))).collect::<BTreeMap<_,_>>()},
        })
    }

    pub(crate) fn monde_validieren(&self) -> Result<(), String> {
        if !self.monde.aktiv { return Ok(()); }
        if self.regeln.monde.is_none() { return Err("STERNE13 ohne Mondregeln".into()); }
        for m in self.monde.monde.values() {
            if m.mutter as usize >= self.planeten.len() || m.besitzer as usize >= self.spieler.len() { return Err("Ungültiger Mond im Spielstand".into()); }
            if m.einheiten.iter().any(|n| *n < 0) || m.bestand.iter().any(|n| *n < -1_000_000) { return Err("Ungültige Mondbestände".into()); }
        }
        Ok(())
    }
}

/// JSON-Schemata der Mondaktionen für Agenten (nur in Partien mit aktiven Monden).
pub fn mond_aktionsschemas(rolle: Rolle) -> Vec<Value> {
    let koord = || json!({"type":"string","description":"Koordinate Sektor:System:Position des Mondes (gleich der Planetenkoordinate)"});
    let objekt = |typ: &str, felder: Vec<(&str, Value)>| {
        let mut props = serde_json::Map::new();
        props.insert("typ".into(), json!({"type":"string","enum":[typ]}));
        let mut pflicht = vec![json!("typ")];
        for (k, v) in felder { pflicht.push(json!(k)); props.insert(k.into(), v); }
        json!({"type":"object","properties":props,"required":pflicht,"additionalProperties":false})
    };
    let gebaeude: Vec<&str> = MondGebaeude::ALLE.iter().map(|g| g.name()).collect();
    // Der Verwalter fertigt auf Monden nur Solarsonden (`mond_fertigen`); sein Schema bietet nur sie an.
    let einheiten: Vec<&str> = Einheit::ALLE.iter().filter(|e| !matches!(e, Einheit::Planetenschild | Einheit::PlanetenschildII | Einheit::PlanetenschildIII))
        .filter(|e| rolle != Rolle::Verwalter || **e == Einheit::Solarsonde).map(|e| e.name()).collect();
    let schiffe: serde_json::Map<String, Value> = Einheit::ALLE.iter().filter(|e| e.ist_schiff()).map(|e| (e.name().to_string(), json!({"type":"integer","minimum":0}))).collect();
    let alle = vec![
        objekt("mond_bauen", vec![("mond", koord()), ("gebaeude", json!({"type":"string","enum":gebaeude}))]),
        objekt("mond_fertigen", vec![("mond", koord()), ("einheit", json!({"type":"string","enum":einheiten})), ("anzahl", json!({"type":"integer","minimum":1,"maximum":10000}))]),
        objekt("mond_umbenennen", vec![("mond", koord()), ("name", json!({"type":"string","maxLength":40}))]),
        objekt("mond_faehre", vec![("mond", koord()), ("gut", json!({"type":"string","enum":Gut::ALLE.iter().map(|g|g.name()).collect::<Vec<_>>()})), ("menge_je_stunde", json!({"type":"integer","description":"positiv zum Mond, negativ zum Planeten, 0 beendet"}))]),
        objekt("phalanx_scannen", vec![("mond", koord()), ("ziel", json!({"type":"string","description":"Planetenkoordinate in Reichweite; Monde sind nicht scannbar"}))]),
        objekt("sprungtor_freigabe", vec![("mond", koord()), ("freigegeben", json!({"type":"boolean"}))]),
        objekt("hyperraumloch_oeffnen", vec![("mond", koord()), ("gegenstelle", koord()), ("erstdurchreise", json!({"type":["object","null"],"properties":{"schiffe":{"type":"object","properties":schiffe.clone()},"danach":{"type":"string","enum":["landen","orbit"]}}}))]),
        // ladung ist optional (ältere Aufrufe senden sie nicht)
        { let mut o = objekt("hyperraumloch_durchreisen", vec![("loch", json!({"type":"integer","minimum":1})), ("von", json!({"type":"object","properties":{"art":{"type":"string","enum":["planet","mond","flotte"]},"koord":{"type":"string"},"flotte":{"type":"integer"}},"required":["art"]})),
            ("schiffe", json!({"type":"object","properties":schiffe})), ("danach", json!({"type":"string","enum":["landen","orbit"]})),
            ("ladung", json!({"type":"object","description":"Fracht von Planet oder Mond, vor allem Deuterium für Weiter- und Rückflug; {} bei Orbitflotten","properties":Gut::ALLE.iter().map(|g|(g.name().to_string(),json!({"type":"number","minimum":0}))).collect::<serde_json::Map<String,Value>>()}))]);
          if let Some(p) = o["required"].as_array_mut() { p.retain(|v| v != "ladung"); } o },
    ];
    alle.into_iter().filter(|v| {
        let typ = v["properties"]["typ"]["enum"][0].as_str().unwrap_or("");
        rolle == Rolle::Alle || mond_zustaendig(typ).contains(&rolle)
    }).collect()
}

#[allow(dead_code)]
fn _preis(p: &Preis) -> [i64; GUETER] { preis_array(p) }

/// Regeltext der Mondgeneration für Agenten und Kodex.
pub fn regeltext(r: &MondRegeln) -> String {
    let l = &r.hyperraumloch;
    format!("Monde entstehen nur nach einem Kampf an einem besiedelten Planeten ohne Mond: Mondchance {} % je {} Trümmer (Erz und Kristall) dieses einen Kampfes, höchstens {} %; vorhandene Trümmer zählen nicht, Kämpfe am Mond, Raketen, Bodenkämpfe und Expeditionen nie. Der Mond gehört dem Planetenbesitzer, geht bei Eroberung mit und wird beim Aufgeben gelöscht. Durchmesser ⌊√(x·10⁶ + 30·T)⌋ km mit x 10–20 und T höchstens {}; Felderobergrenze ⌊d²/10⁶⌋ (Typ schluucht_tief +25 %), 1 Grundfeld und +3 je Mondbasis-Stufe; die Mondbasis lässt sich auch auf einem vollen Mond ausbauen (eine Stufe zur Zeit). Zehn Mondtypen nach Zone mit je einem Vorteil und einem Nachteil.
Mondgebäude (ohne Arbeiter, Energie aus Mondbasis und Solarsatelliten): mondbasis, regolithfoerderung, kryolager (Deuterium-Kühllager), werftdock (ab Stufe {} Weltenkrümmer), tiefenphalanx, sprungtor (Hyperraumtor-Generator, höchstens 5), ortungsschleier (höchstens 5). Bezahlt wird aus dem Mondbestand; Nachschub per Transport (zielart mond) oder Versorgungsfähre (mond_faehre, höchstens {} Einheiten je Stunde und Mondbasis-Stufe). Unterhalt {} + {} Credits je Gebäudestufe und Tag; im Verzug halbe Wirkung, Hyperraumtor-Generator und Tiefenphalanx gesperrt. Ein Mond belegt keinen Kolonieplatz.
Flüge: flotte_senden mit zielart mond (transport, stationieren, halten, spionage, angriff, blockade, mond_zerstoeren) und startart mond. Die Tiefenphalanx (phalanx_scannen) zeigt Flüge eines Planeten in Reichweite ⌊(Stufe²−1)/{}⌋ Systeme im eigenen Sektor, {} Deuterium je Scan, {} s Abklingzeit; Monde sind nicht scannbar; nur Flüge zwischen zwei Monden und Hyperraumloch-Durchreisen bleiben unsichtbar – ein Flug von einem Mond auf den gescannten Planeten oder von ihm zu einem Mond erscheint. Der Gescannte erfährt nichts.
Hyperraumtor-Generator (Schlüssel sprungtor): hyperraumloch_oeffnen öffnet ein Hyperraumtor neben dem Mond zu einem zweiten Generator (eigenes oder freigegebenes eines Partners), {} Spielsekunden offen, nicht vorzeitig schließbar. Reichweite {} Systeme je Stufe (Sektorwechsel ab Stufe {}), Kosten {} + {} Deuterium je System, Abklingzeit {} min −10 % je Stufe (mindestens {} min) für beide Tore. Jeder, der das Loch sieht (Besitz oder Orbitflotte im Start- oder Zielsystem, Partner, ferne Phalanx über dem Ortungsschleier), darf mit Schiffen aus Start- oder Zielsystem in beide Richtungen reisen (hyperraumloch_durchreisen): Kapazität {} Struktur je Stufe, {}, jede Flotte und jeder Ort einmal je Loch. Am Austritt kämpft die Torwache gegen alle ohne Bündnis oder Nichtangriffspakt. Weltenkrümmer, Solarsatelliten, Verteidigung und Reiche unter Anfängerschutz reisen nicht.
Weltenkrümmer zerstören Monde nach vollständigem Sieg: Chance (100 − √d)·√n %, Verlust aller Weltenkrümmer √d/2 %. Der zerstörte Mond zerfällt zu einem Trümmerring (20 Erz und 10 Kristall je km Durchmesser, dazu Lager-, Gebäude- und Wrackanteil); die Hälfte der Besatzung stationierter Schiffe überlebt und wird mit recyceln von Recyclern und Bergungsschiffen als Passagiere geborgen (Personenplätze). Solarsatelliten (Kennung solarsonde, ab Zivilisationsstufe II, auch über Planeten) liefern {} × Sonnenfaktor der Bahn Energie, fliegen nie und hinterlassen Trümmer.",
        1, r.truemmer_je_prozent, r.chance_max_prozent, r.truemmer_groesse_max, r.weltentruemmer_werftdock,
        r.faehre_je_mondbasis_stunde, r.unterhalt_basis_tag, r.unterhalt_je_stufe_tag, r.tiefenphalanx.reichweite_teiler, r.tiefenphalanx.scan_deuterium,
        r.tiefenphalanx.abklingzeit_sekunden, l.offen_sekunden, l.reichweite_je_stufe, l.sektorwechsel_ab_stufe, l.deuterium_basis, l.deuterium_je_system,
        l.abklingzeit_basis_minuten, l.abklingzeit_min_minuten, l.kapazitaet_je_stufe, if l.gebuehr_anteil <= 0.0 { "Durchreise ohne Gebühr (wer danach im Orbit bleibt, zahlt den späteren Heimflug gleich mit)".to_string() } else { format!("Gebühr {} % des Einwegtreibstoffs", (l.gebuehr_anteil * 100.0).round()) }, r.solarsonde_energie)
}
