//! Dysonsphäre – das Allianz-Großprojekt – und Allianzen v2 (bis 50 Mitglieder).
//!
//! Konzept und Beschlüsse: `docs/DYSONSPHAERE.md` (Abschnitt „Beschlossen“). Nur neue Welten:
//! `Welt::dyson_aktivieren` und `Welt::allianz_v2_aktivieren` (Server `fresh_with`, lokale Online-Partien).
//! Der Zustand liegt in `ausbau.dyson` (JSON-Erweiterung des Snapshots, leer = nicht serialisiert), die Regeln
//! zusätzlich gespiegelt in `regeln.dyson` (`#[serde(skip)]`). Laufende Welten bleiben bytegleich.
//!
//! Ablauf: Vorschlag (Allianzleitung) → Beitritt → Einzahlung in die zweckgebundene Projektkasse (nur von eigenen
//! Planeten/Monden im System, Teilzahlungen, genau ein Anteil je Mitglied) → Baubeginn (alle 12 Plätze von der
//! Allianz besiedelt, ≥ 10 Mitglieder voll eingezahlt, Stufe V, 14 Tage Mitgliedschaft) → 30 Spieltage Bau (ruht bei
//! verletzten Bedingungen) → Wirkung: Sonnenfaktor × 1,6 für die Eigner-Allianz im System, bis 4 Anschlüsse mit
//! Einspeisung, „nie Mangel“, Strahlsynthese, Strombad. Verwaist unter 6 gehaltenen Plätzen; Übernahme nach 7 Tagen.

use crate::typen::*;
use crate::welt::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const REGELN_RON: &str = include_str!("../../../regeln/dyson.ron");
/// Dysonschatten (Ablauf v2, Karl 19.10.2026): eine vollendete Sphäre senkt die Sonne aller Planeten und Mondsatelliten
/// im System, die nicht der Eigner-Allianz gehören, auf 50 Prozent; die Eigner behalten `sonnenfaktor` und Anschlüsse.
pub const SCHATTEN: f64 = 0.5;
pub const ALLIANZ_RON: &str = include_str!("../../../regeln/allianz.ron");
/// Die drei Grundrohstoffe des Anteils.
pub const ROHSTOFFE: [Gut; 3] = [Gut::Erz, Gut::Kristall, Gut::Deuterium];

// ---------------------------------------------------------------- Regeln

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DysonRegeln {
    pub version: u32,
    pub mitglieder_min: usize,
    pub stufe_min: u8,
    pub mitgliedschaft_min_tage: i64,
    pub halten_min_plaetze: u8,
    pub anteil_voll: i64,
    pub anteil_365: i64,
    pub voll_ab_tage: i64,
    pub min_epoche_tage: i64,
    pub mindestbuchung_voll: i64,
    pub mindestbuchung_min: i64,
    pub rueckzahlung_austritt: f64,
    pub rueckzahlung_abbruch_vor_bau: f64,
    pub rueckzahlung_abbruch_im_bau: f64,
    pub bau_sekunden: i64,
    pub ruhe_frist_tage: i64,
    pub sperre_nach_abbruch_tage: i64,
    pub uebernahme_tage: i64,
    pub sonnenfaktor: f64,
    pub anschluesse: usize,
    pub anschluesse_je_spieler: usize,
    pub anschluss_vorlauf_stunden: i64,
    pub anschluss_sperre_tage: i64,
    pub einspeisung: f64,
    pub strahlsynthese_deuterium: f64,
    pub strahlsynthese_xenokristall: f64,
    pub strombad: f64,
    pub xenosynthese_ohne_malus: bool,
    pub xenosynthese_rezept_faktor: f64,
}

impl DysonRegeln {
    pub fn laden(text: &str) -> Result<DysonRegeln, String> {
        let r: DysonRegeln = ron::from_str(text).map_err(|e| format!("Dysonregeln nicht lesbar: {e}"))?;
        if r.mitglieder_min == 0 || r.anteil_voll <= 0 || r.anteil_365 <= 0 || r.anteil_365 > r.anteil_voll
            || r.bau_sekunden <= 0 || r.anschluesse == 0 || r.anschluesse_je_spieler == 0 || r.voll_ab_tage <= 365
            || r.sonnenfaktor < 1.0 || !(0.0..=1.0).contains(&r.rueckzahlung_austritt)
            || !(0.0..=1.0).contains(&r.rueckzahlung_abbruch_vor_bau) || !(0.0..=1.0).contains(&r.rueckzahlung_abbruch_im_bau) {
            return Err("Dysonregeln: ungültige Grenzwerte".into());
        }
        Ok(r)
    }
    pub fn standard() -> DysonRegeln { Self::laden(REGELN_RON).expect("geprüfte Dysonregeln") }
    /// Anteil je Gut und Mitglied (ganze Einheiten) für eine Epochenlänge; `None` = keine Sphäre.
    /// `s(E) = min(1, a365/a_voll · (a_voll/a365)^((E − 365)/365))`, auf volle Million gerundet.
    pub fn anteil_fuer(&self, epoche_tage: i64) -> Option<i64> {
        if epoche_tage < self.min_epoche_tage { return None; }
        if epoche_tage >= self.voll_ab_tage { return Some(self.anteil_voll); }
        let a365 = self.anteil_365 as f64;
        let voll = self.anteil_voll as f64;
        let s = ((a365 / voll) * (voll / a365).powf((epoche_tage - 365) as f64 / 365.0)).min(1.0);
        let gerundet = ((voll * s / 1_000_000.0).round() as i64).max(1) * 1_000_000;
        Some(gerundet.min(self.anteil_voll))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AllianzRegeln {
    pub version: u32,
    pub allianz_max: usize,
    pub bot_allianz_max: usize,
}

impl AllianzRegeln {
    pub fn laden(text: &str) -> Result<AllianzRegeln, String> {
        let r: AllianzRegeln = ron::from_str(text).map_err(|e| format!("Allianzregeln nicht lesbar: {e}"))?;
        if r.allianz_max < 2 || r.bot_allianz_max < 2 { return Err("Allianzregeln: ungültige Grenzwerte".into()); }
        Ok(r)
    }
    pub fn standard() -> AllianzRegeln { Self::laden(ALLIANZ_RON).expect("geprüfte Allianzregeln") }
}

// ---------------------------------------------------------------- Zustand

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DysonPhase { Sammeln, Bau, Vollendet, Verwaist, Abgebrochen }

impl DysonPhase {
    pub fn name(self) -> &'static str {
        match self { Self::Sammeln => "sammeln", Self::Bau => "bau", Self::Vollendet => "vollendet", Self::Verwaist => "verwaist", Self::Abgebrochen => "abgebrochen" }
    }
    pub fn offen(self) -> bool { matches!(self, Self::Sammeln | Self::Bau) }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Beitrag {
    pub beigetreten: SimZeit,
    /// Erz, Kristall, Deuterium in Tausendsteln.
    pub eingezahlt: [i64; 3],
    #[serde(default)]
    pub voll_seit: Option<SimZeit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anschluss {
    pub planet: PlanetId,
    pub besitzer: SpielerId,
    pub gewaehlt: SimZeit,
    pub aktiv_ab: SimZeit,
    #[serde(default)]
    pub angewendet: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DysonProjekt {
    pub id: u32,
    pub allianz: u32,
    pub sektor: u8,
    pub system: u8,
    pub vorgeschlagen: SimZeit,
    pub vorschlag_von: SpielerId,
    pub phase: DysonPhase,
    #[serde(default)]
    pub bau_begonnen: Option<SimZeit>,
    #[serde(default)]
    pub bau_erledigt: i64,
    #[serde(default)]
    pub bau_stand: SimZeit,
    #[serde(default)]
    pub ruht_seit: Option<SimZeit>,
    #[serde(default)]
    pub ruhe_summe: i64,
    #[serde(default)]
    pub ruhe_grund: String,
    #[serde(default)]
    pub phase_gemeldet: u8,
    #[serde(default)]
    pub vollendet: Option<SimZeit>,
    #[serde(default)]
    pub verwaist_seit: Option<SimZeit>,
    #[serde(default)]
    pub uebernahme: Option<(u32, SimZeit)>,
    #[serde(default)]
    pub abgebrochen: Option<SimZeit>,
    #[serde(default)]
    pub gesperrt_bis: SimZeit,
    pub mitglieder: BTreeMap<SpielerId, Beitrag>,
    pub anschluesse: Vec<Option<Anschluss>>,
    pub slot_sperre_bis: Vec<SimZeit>,
}

impl DysonProjekt {
    pub fn koord(&self) -> String { format!("{}:{}", self.sektor, self.system) }
    pub fn im_system(&self, k: Koord) -> bool { k.sektor == self.sektor && k.system == self.system }
    pub fn kasse(&self) -> [i64; 3] {
        let mut k = [0; 3];
        for b in self.mitglieder.values() { for i in 0..3 { k[i] += b.eingezahlt[i]; } }
        k
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dysonzustand {
    pub aktiv: bool,
    #[serde(default)]
    pub regeln: Option<DysonRegeln>,
    /// Anteil je Gut und Mitglied in ganzen Einheiten (aus der Epochenlänge, bei der Aktivierung festgelegt).
    #[serde(default)]
    pub anteil: i64,
    #[serde(default)]
    pub projekte: BTreeMap<u32, DysonProjekt>,
    #[serde(default)]
    pub naechstes: u32,
    /// Beitrittszeit je Spieler (Allianz, Zeit) – der Kern speichert sie sonst nicht.
    #[serde(default)]
    pub mitglied_seit: BTreeMap<SpielerId, (u32, SimZeit)>,
    /// Rückzahlungsguthaben Erz, Kristall, Deuterium in Tausendsteln.
    #[serde(default)]
    pub guthaben: BTreeMap<SpielerId, [i64; 3]>,
    #[serde(default)]
    pub pruef_stand: SimZeit,
    /// Allianzen v2: Regeln (nur gesetzt nach `allianz_v2_aktivieren`).
    #[serde(default)]
    pub allianz: Option<AllianzRegeln>,
}

impl Dysonzustand {
    pub fn leer(&self) -> bool { !self.aktiv && self.allianz.is_none() }
}

/// Wirkung der Sphäre auf einen Planeten (für `planer::wirtschaft`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DysonWirkung {
    pub sonnenfaktor: f64,
    pub anschluss: bool,
    /// Tausendstel je Spielstunde.
    pub einspeisung: i64,
    pub deuterium: i64,
    pub xenokristall: i64,
    pub strombad: f64,
    pub xeno_ohne_malus: bool,
    pub rezept_faktor: f64,
}

/// Eine Bedingung für den Baubeginn (für Oberfläche und Ablehnungstext).
#[derive(Clone, Debug, Serialize)]
pub struct Bedingung {
    pub kennung: &'static str,
    pub text: String,
    pub ist: i64,
    pub soll: i64,
    pub erfuellt: bool,
}

// ---------------------------------------------------------------- Aktivierung und Grundlagen

impl Welt {
    pub fn dyson_regeln(&self) -> Option<&DysonRegeln> {
        if self.ausbau.dyson.aktiv { self.ausbau.dyson.regeln.as_ref() } else { None }
    }
    /// Skalenfaktor der absoluten Mengen in Millionsteln (Anteil / voller Anteil).
    pub fn dyson_skala_ppm(&self) -> i64 {
        let Some(r) = self.dyson_regeln() else { return 0 };
        (self.ausbau.dyson.anteil as i128 * 1_000_000 / r.anteil_voll.max(1) as i128) as i64
    }
    fn dyson_skaliert(&self, wert: f64) -> i64 {
        // Tausendstel: wert · M · s
        (wert * M as f64 * self.dyson_skala_ppm() as f64 / 1_000_000.0).round() as i64
    }
    pub fn dyson_mindestbuchung(&self) -> i64 {
        let Some(r) = self.dyson_regeln() else { return 0 };
        ((r.mindestbuchung_voll as i128 * self.dyson_skala_ppm() as i128 / 1_000_000) as i64).max(r.mindestbuchung_min)
    }

    /// Schaltet die Dysonsphäre ein (neue Welten). Unter `min_epoche_tage` bleibt sie aus (keine Wirkung, kein Hash).
    pub fn dyson_aktivieren(&mut self) -> Result<(), String> {
        self.dyson_aktivieren_aus(REGELN_RON)
    }
    /// Wie [`Welt::dyson_aktivieren`], mit gegebenem Text von `regeln/dyson.ron` (ConfigWelt).
    pub fn dyson_aktivieren_aus(&mut self, text: &str) -> Result<(), String> {
        if self.ausbau.dyson.aktiv { return Ok(()); }
        if self.ausbau.version == 0 { return Err("Die Dysonsphäre benötigt Welten der Generation 12 (ausbau_aktivieren)".into()); }
        let regeln = DysonRegeln::laden(text)?;
        let Some(anteil) = regeln.anteil_fuer(self.regeln.welt.epoche_tage) else { return Ok(()); };
        {
            use sha2::{Digest, Sha256};
            let r = std::sync::Arc::make_mut(&mut self.regeln);
            let mut h = Sha256::new();
            h.update(r.hash.as_bytes());
            h.update(b"dyson-v1");
            h.update(text.replace("\r\n", "\n").as_bytes());
            r.hash = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
            r.dyson = Some(std::sync::Arc::new(regeln.clone()));
        }
        let mut seit = BTreeMap::new();
        for a in &self.allianzen { for m in &a.mitglieder { seit.insert(*m, (a.id, self.zeit)); } }
        let alt_allianz = self.ausbau.dyson.allianz.clone();
        self.ausbau.dyson = Dysonzustand { aktiv: true, regeln: Some(regeln), anteil, naechstes: 1, mitglied_seit: seit,
            pruef_stand: self.zeit, allianz: alt_allianz, ..Default::default() };
        Ok(())
    }

    /// Allianzen v2 (neue Welten): `diplomatie.allianz_max` der Welt auf den Wert aus `regeln/allianz.ron`.
    pub fn allianz_v2_aktivieren(&mut self) -> Result<(), String> {
        self.allianz_v2_aktivieren_aus(ALLIANZ_RON)
    }
    /// Wie [`Welt::allianz_v2_aktivieren`], mit gegebenem Text von `regeln/allianz.ron` (ConfigWelt).
    pub fn allianz_v2_aktivieren_aus(&mut self, text: &str) -> Result<(), String> {
        if self.ausbau.dyson.allianz.is_some() { return Ok(()); }
        if self.ausbau.version == 0 { return Err("Allianzen v2 benötigen Welten der Generation 12 (ausbau_aktivieren)".into()); }
        let a = AllianzRegeln::laden(text)?;
        {
            use sha2::{Digest, Sha256};
            let r = std::sync::Arc::make_mut(&mut self.regeln);
            r.diplomatie.allianz_max = a.allianz_max;
            let mut h = Sha256::new();
            h.update(r.hash.as_bytes());
            h.update(b"allianz-v2");
            h.update(text.replace("\r\n", "\n").as_bytes());
            r.hash = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        }
        self.ausbau.dyson.allianz = Some(a);
        Ok(())
    }
    /// Bot-Grenze für Einladungen (alte Welten: allianz_max des Profils).
    pub fn bot_allianz_max(&self) -> usize {
        let max = self.regeln.diplomatie.allianz_max;
        self.ausbau.dyson.allianz.as_ref().map_or(max, |a| a.bot_allianz_max.min(max))
    }

    /// Nach dem Laden: Regeln spiegeln, Zustand prüfen.
    pub(crate) fn dyson_nach_laden(&mut self) -> Result<(), String> {
        if self.ausbau.dyson.aktiv {
            let r = self.ausbau.dyson.regeln.clone().ok_or("Dysonzustand ohne Regeln")?;
            std::sync::Arc::make_mut(&mut self.regeln).dyson = Some(std::sync::Arc::new(r));
            for p in self.ausbau.dyson.projekte.values() {
                if p.mitglieder.keys().any(|s| *s as usize >= self.spieler.len()) || p.anschluesse.iter().flatten().any(|a| a.planet as usize >= self.planeten.len()) {
                    return Err("Ungültiges Dysonprojekt im Spielstand".into());
                }
            }
        }
        Ok(())
    }

    fn dyson_system_planeten(&self, sektor: u8, system: u8) -> Vec<usize> {
        let n = self.regeln.welt.plaetze_je_system;
        (1..=n).filter_map(|p| self.belegung.get(&Koord::neu(sektor, system, p)).map(|pid| *pid as usize))
            .filter(|pid| self.planet_aktiv(*pid)).collect()
    }
    /// Von Mitgliedern der Allianz besiedelte Plätze im System.
    fn dyson_allianz_plaetze(&self, aid: u32, sektor: u8, system: u8) -> usize {
        self.dyson_system_planeten(sektor, system).into_iter()
            .filter(|pid| self.spieler[self.planeten[*pid].besitzer as usize].allianz == Some(aid)).count()
    }
    fn dyson_hat_planet(&self, sid: SpielerId, sektor: u8, system: u8) -> bool {
        self.dyson_system_planeten(sektor, system).into_iter().any(|pid| self.planeten[pid].besitzer == sid)
    }
    fn dyson_in_allianz(&self, sid: SpielerId, aid: u32) -> bool {
        self.spieler.get(sid as usize).is_some_and(|s| s.allianz == Some(aid)) && self.spieler_aktiv(sid)
    }
    fn dyson_voll(&self, b: &Beitrag) -> bool {
        let soll = self.ausbau.dyson.anteil.saturating_mul(M);
        b.eingezahlt.iter().all(|x| *x >= soll)
    }
    fn dyson_projekt_von_allianz(&self, aid: u32) -> Option<u32> {
        self.ausbau.dyson.projekte.values().filter(|p| p.allianz == aid && p.phase != DysonPhase::Abgebrochen)
            .max_by_key(|p| (p.phase.offen(), p.id)).map(|p| p.id)
    }
    fn dyson_projekt_im_system(&self, sektor: u8, system: u8) -> Option<&DysonProjekt> {
        self.ausbau.dyson.projekte.values().filter(|p| p.sektor == sektor && p.system == system).max_by_key(|p| p.id)
    }
    fn dyson_projekt_holen(&self, a: &Value, sid: SpielerId) -> Result<u32, String> {
        if let Some(id) = a.get("projekt").and_then(Value::as_u64) { return Ok(id as u32); }
        let aid = self.spieler[sid as usize].allianz.ok_or("Du bist in keiner Allianz")?;
        self.dyson_projekt_von_allianz(aid).ok_or_else(|| "Deine Allianz hat kein Dysonprojekt".into())
    }
    fn dyson_ereignis(&mut self, p: u32, art: &str, text: String, auch_allianz: bool) {
        let Some(pr) = self.ausbau.dyson.projekte.get(&p) else { return };
        let mut an: Vec<SpielerId> = pr.mitglieder.keys().copied().collect();
        if auch_allianz {
            if let Some(a) = self.allianzen.iter().find(|a| a.id == pr.allianz) { for m in &a.mitglieder { if !an.contains(m) { an.push(*m); } } }
        }
        for s in an { if (s as usize) < self.spieler.len() { self.vorfall(s, art, text.clone()); } }
    }
    fn dyson_register(&mut self, p: u32, vorgang: &str) {
        let Some(pr) = self.ausbau.dyson.projekte.get(&p) else { return };
        let name = self.allianzen.iter().find(|a| a.id == pr.allianz).map(|a| a.name.clone()).unwrap_or_default();
        self.register.push(Registereintrag { zeit: self.zeit, vertrag: 0, art: "dysonsphaere".into(), a: name.clone(), b: pr.koord(), vorgang: vorgang.to_string() });
    }
    fn dyson_raten_system(&mut self, sektor: u8, system: u8) {
        for pid in self.dyson_system_planeten(sektor, system) { self.raten_neu(pid); }
    }
    fn dyson_guthaben(&mut self, sid: SpielerId, betrag: [i64; 3]) {
        let g = self.ausbau.dyson.guthaben.entry(sid).or_insert([0; 3]);
        for i in 0..3 { g[i] += betrag[i]; }
        if g.iter().all(|x| *x == 0) { self.ausbau.dyson.guthaben.remove(&sid); }
    }

    // ------------------------------------------------------------ Bedingungen

    /// Bedingungen B1–B5 für den Baubeginn (Reihenfolge wie im Konzept).
    pub fn dyson_bedingungen(&self, pid: u32) -> Vec<Bedingung> {
        let (Some(r), Some(p)) = (self.dyson_regeln(), self.ausbau.dyson.projekte.get(&pid)) else { return Vec::new() };
        let n = self.regeln.welt.plaetze_je_system as i64;
        let plaetze = self.dyson_allianz_plaetze(p.allianz, p.sektor, p.system) as i64;
        let gueltig: Vec<SpielerId> = p.mitglieder.keys().copied()
            .filter(|s| self.dyson_in_allianz(*s, p.allianz) && self.dyson_hat_planet(*s, p.sektor, p.system)).collect();
        let voll = p.mitglieder.iter().filter(|(s, b)| gueltig.contains(s) && self.dyson_voll(b)).count() as i64;
        let alle_voll = p.mitglieder.values().all(|b| self.dyson_voll(b));
        let stufe = gueltig.iter().filter(|s| self.spieler[**s as usize].stufe >= r.stufe_min).count() as i64;
        let tage = r.mitgliedschaft_min_tage * TAG;
        let lange = gueltig.iter().filter(|s| self.ausbau.dyson.mitglied_seit.get(s).is_some_and(|(a, t)| *a == p.allianz && self.zeit - *t >= tage)).count() as i64;
        let g = gueltig.len() as i64;
        let min = r.mitglieder_min as i64;
        vec![
            Bedingung { kennung: "plaetze", text: format!("Alle {n} Plätze des Systems von Allianzmitgliedern besiedelt"), ist: plaetze, soll: n, erfuellt: plaetze >= n },
            Bedingung { kennung: "mitglieder", text: format!("Mindestens {min} Projektmitglieder mit Planet im System"), ist: g, soll: min, erfuellt: g >= min },
            Bedingung { kennung: "eingezahlt", text: "Alle Projektmitglieder haben ihren Anteil voll eingezahlt".into(), ist: voll, soll: (p.mitglieder.len() as i64).max(min), erfuellt: alle_voll && voll >= min },
            Bedingung { kennung: "stufe", text: format!("Alle Projektmitglieder auf Zivilisationsstufe {}", r.stufe_min), ist: stufe, soll: g.max(min), erfuellt: stufe == g && g >= min },
            Bedingung { kennung: "mitgliedschaft", text: format!("Alle Projektmitglieder seit mindestens {} Spieltagen in der Allianz", r.mitgliedschaft_min_tage), ist: lange, soll: g.max(min), erfuellt: lange == g && g >= min },
        ]
    }
    /// Bedingungen während des Baus: alle Plätze und ≥ Mindestzahl voll eingezahlter Mitglieder mit Planet im System.
    fn dyson_bau_grund(&self, pid: u32) -> Option<String> {
        let (r, p) = (self.dyson_regeln()?, self.ausbau.dyson.projekte.get(&pid)?);
        let n = self.regeln.welt.plaetze_je_system as usize;
        let plaetze = self.dyson_allianz_plaetze(p.allianz, p.sektor, p.system);
        if plaetze < n { return Some(format!("System nicht mehr voll besetzt ({plaetze}/{n} Plätze)")); }
        let g = p.mitglieder.iter().filter(|(s, b)| self.dyson_in_allianz(**s, p.allianz) && self.dyson_hat_planet(**s, p.sektor, p.system) && self.dyson_voll(b)).count();
        if g < r.mitglieder_min { return Some(format!("Nur noch {g}/{} Projektmitglieder mit Planet im System", r.mitglieder_min)); }
        None
    }
    fn dyson_leitung(&self, sid: SpielerId, aid: u32) -> Result<(), String> {
        if self.spieler[sid as usize].allianz != Some(aid) { return Err("Das Projekt gehört nicht zu deiner Allianz".into()); }
        if !self.allianz_leitung(sid, aid) { return Err("Nur die Allianzleitung (die obersten vier Mitglieder) darf das".into()); }
        Ok(())
    }

    // ------------------------------------------------------------ Aktionen

    pub fn dyson_vorschlagen(&mut self, sid: SpielerId, sektor: u8, system: u8) -> Result<String, String> {
        if self.dyson_regeln().is_none() { return Err("Die Dysonsphäre gibt es in dieser Welt nicht".into()); }
        if self.system(Koord::neu(sektor, system, 1)).is_none() { return Err(format!("System {sektor}:{system} gibt es nicht")); }
        let aid = self.spieler[sid as usize].allianz.ok_or("Du bist in keiner Allianz")?;
        self.dyson_leitung(sid, aid)?;
        if self.dyson_allianz_plaetze(aid, sektor, system) == 0 { return Err("Deine Allianz braucht mindestens einen Planeten in diesem System".into()); }
        if let Some(p) = self.dyson_projekt_im_system(sektor, system) {
            if p.phase != DysonPhase::Abgebrochen { return Err(format!("Im System {} gibt es bereits ein Dysonprojekt ({})", p.koord(), p.phase.name())); }
            if p.gesperrt_bis > self.zeit { return Err(format!("System {} ist nach dem Abbruch gesperrt bis {}", p.koord(), zeittext(p.gesperrt_bis))); }
        }
        if self.ausbau.dyson.projekte.values().any(|p| p.allianz == aid && p.phase.offen()) { return Err("Deine Allianz hat bereits ein offenes Dysonprojekt".into()); }
        let slots = self.dyson_regeln().unwrap().anschluesse;
        let id = self.ausbau.dyson.naechstes.max(1);
        self.ausbau.dyson.naechstes = id + 1;
        self.ausbau.dyson.projekte.insert(id, DysonProjekt { id, allianz: aid, sektor, system, vorgeschlagen: self.zeit, vorschlag_von: sid,
            phase: DysonPhase::Sammeln, bau_begonnen: None, bau_erledigt: 0, bau_stand: self.zeit, ruht_seit: None, ruhe_summe: 0, ruhe_grund: String::new(),
            phase_gemeldet: 0, vollendet: None, verwaist_seit: None, uebernahme: None, abgebrochen: None, gesperrt_bis: 0,
            mitglieder: BTreeMap::new(), anschluesse: vec![None; slots], slot_sperre_bis: vec![0; slots] });
        self.dyson_ereignis(id, "dyson_vorgeschlagen", format!("Dysonsphäre im System {sektor}:{system} vorgeschlagen – Mitglieder mit Planet im System können beitreten"), true);
        Ok(format!("Dysonprojekt {id} im System {sektor}:{system} vorgeschlagen"))
    }

    pub fn dyson_beitreten(&mut self, sid: SpielerId, pid: u32) -> Result<String, String> {
        let p = self.ausbau.dyson.projekte.get(&pid).ok_or("Dysonprojekt gibt es nicht")?;
        if !p.phase.offen() { return Err("Beitreten geht nur, solange gesammelt oder gebaut wird".into()); }
        if self.spieler[sid as usize].allianz != Some(p.allianz) { return Err("Nur Mitglieder der Eigner-Allianz können beitreten".into()); }
        if !self.dyson_hat_planet(sid, p.sektor, p.system) { return Err(format!("Du brauchst einen eigenen Planeten im System {}", p.koord())); }
        if p.mitglieder.contains_key(&sid) { return Err("Du bist bereits Projektmitglied".into()); }
        if self.ausbau.dyson.projekte.values().any(|q| q.id != pid && q.phase.offen() && q.mitglieder.contains_key(&sid)) { return Err("Du bist schon Mitglied eines anderen offenen Dysonprojekts".into()); }
        let koord = p.koord();
        self.ausbau.dyson.projekte.get_mut(&pid).unwrap().mitglieder.insert(sid, Beitrag { beigetreten: self.zeit, eingezahlt: [0; 3], voll_seit: None });
        let name = self.spieler[sid as usize].name.clone();
        self.dyson_ereignis(pid, "dyson_beitritt", format!("{name} ist dem Dysonprojekt {koord} beigetreten"), false);
        Ok(format!("Dem Dysonprojekt {koord} beigetreten – Anteil je {} Erz, Kristall und Deuterium", self.ausbau.dyson.anteil))
    }

    pub fn dyson_austreten(&mut self, sid: SpielerId, pid: u32) -> Result<String, String> {
        let r = self.dyson_regeln().ok_or("Die Dysonsphäre gibt es in dieser Welt nicht")?.clone();
        let p = self.ausbau.dyson.projekte.get(&pid).ok_or("Dysonprojekt gibt es nicht")?;
        if p.phase != DysonPhase::Sammeln { return Err("Austreten geht nur vor dem Baubeginn – verbaute Paneele sind nicht zurückzuholen".into()); }
        let b = p.mitglieder.get(&sid).ok_or("Du bist kein Projektmitglied")?.clone();
        let rueck = b.eingezahlt.map(|x| mal(x, r.rueckzahlung_austritt));
        self.ausbau.dyson.projekte.get_mut(&pid).unwrap().mitglieder.remove(&sid);
        self.dyson_guthaben(sid, rueck);
        Ok(format!("Aus dem Dysonprojekt ausgetreten; {} % der Einzahlung ({} Erz, {} Kristall, {} Deuterium) als Guthaben – mit dyson_auszahlen abrufen",
            (r.rueckzahlung_austritt * 100.0).round(), rueck[0] / M, rueck[1] / M, rueck[2] / M))
    }

    /// Einzahlung in die Projektkasse von einem eigenen Planeten (oder Mond mit `mond = true`) im Dyson-System.
    pub fn dyson_einzahlen(&mut self, sid: SpielerId, pid: u32, von: Koord, mond: bool, mengen: [i64; 3]) -> Result<String, String> {
        if self.dyson_regeln().is_none() { return Err("Die Dysonsphäre gibt es in dieser Welt nicht".into()); }
        let p = self.ausbau.dyson.projekte.get(&pid).ok_or("Dysonprojekt gibt es nicht")?;
        if !p.phase.offen() { return Err("Einzahlen geht nur, solange gesammelt oder gebaut wird".into()); }
        let b = p.mitglieder.get(&sid).ok_or("Erst dem Projekt beitreten (dyson_beitreten)")?.clone();
        if !p.im_system(von) { return Err(format!("Einzahlen geht nur von eigenen Planeten oder Monden im System {}", p.koord())); }
        if mengen.iter().any(|x| *x < 0) || mengen.iter().all(|x| *x == 0) { return Err("Mengen für erz, kristall oder deuterium angeben (ganze Einheiten, nicht negativ)".into()); }
        let min = self.dyson_mindestbuchung();
        let soll = self.ausbau.dyson.anteil.saturating_mul(M);
        for (i, g) in ROHSTOFFE.iter().enumerate() {
            let x = mengen[i];
            if x == 0 { continue; }
            let rest = (soll - b.eingezahlt[i]).max(0);
            if x * M > rest { return Err(format!("{}: offener Rest nur {} (Anteil {}, keine Überzahlung)", g.name(), rest / M, soll / M)); }
            if x < min && x * M != rest { return Err(format!("{}: Mindestbuchung {min} (oder genau der Rest)", g.name())); }
        }
        let milli = mengen.map(|x| x * M);
        if mond {
            let mid = self.eigener_mond(sid, von)?;
            let m = &self.monde.monde[&mid];
            for (i, g) in ROHSTOFFE.iter().enumerate() { if m.bestand[g.idx()] < milli[i] { return Err(format!("Auf dem Mond {von} liegen nur {} {}", m.bestand[g.idx()] / M, g.name())); } }
            let m = self.monde.monde.get_mut(&mid).unwrap();
            for (i, g) in ROHSTOFFE.iter().enumerate() { m.bestand[g.idx()] -= milli[i]; }
        } else {
            let planet = self.eigener_planet(sid, von)?;
            self.abrechnen(planet);
            for (i, g) in ROHSTOFFE.iter().enumerate() { if self.planeten[planet].bestand[g.idx()] < milli[i] { return Err(format!("Auf {von} liegen nur {} {}", self.planeten[planet].bestand[g.idx()] / M, g.name())); } }
            for (i, g) in ROHSTOFFE.iter().enumerate() { self.planeten[planet].bestand[g.idx()] -= milli[i]; }
            self.raten_neu(planet);
        }
        let zeit = self.zeit;
        let bb = self.ausbau.dyson.projekte.get_mut(&pid).unwrap().mitglieder.get_mut(&sid).unwrap();
        for i in 0..3 { bb.eingezahlt[i] += milli[i]; }
        let jetzt_voll = bb.eingezahlt.iter().all(|x| *x >= soll) && bb.voll_seit.is_none();
        if jetzt_voll { bb.voll_seit = Some(zeit); }
        let stand = bb.eingezahlt;
        if jetzt_voll {
            let name = self.spieler[sid as usize].name.clone();
            self.dyson_ereignis(pid, "dyson_anteil_voll", format!("{name} hat den Anteil am Dysonprojekt voll eingezahlt"), false);
        }
        Ok(format!("In die Projektkasse eingezahlt: {} Erz, {} Kristall, {} Deuterium; eigener Stand {}/{}/{} von je {}",
            mengen[0], mengen[1], mengen[2], stand[0] / M, stand[1] / M, stand[2] / M, soll / M))
    }

    pub fn dyson_bau_starten(&mut self, sid: SpielerId, pid: u32) -> Result<String, String> {
        let p = self.ausbau.dyson.projekte.get(&pid).ok_or("Dysonprojekt gibt es nicht")?;
        self.dyson_leitung(sid, p.allianz)?;
        if p.phase != DysonPhase::Sammeln { return Err(format!("Das Projekt ist in Phase {}", p.phase.name())); }
        let fehlt: Vec<String> = self.dyson_bedingungen(pid).into_iter().filter(|b| !b.erfuellt).map(|b| format!("{} ({}/{})", b.text, b.ist, b.soll)).collect();
        if !fehlt.is_empty() { return Err(format!("Baubeginn nicht möglich: {}", fehlt.join("; "))); }
        let zeit = self.zeit;
        let p = self.ausbau.dyson.projekte.get_mut(&pid).unwrap();
        p.phase = DysonPhase::Bau;
        p.bau_begonnen = Some(zeit);
        p.bau_stand = zeit;
        let koord = p.koord();
        self.dyson_ereignis(pid, "dyson_bau_begonnen", format!("Baubeginn der Dysonsphäre im System {koord} – {} Spieltage", self.dyson_regeln().map_or(15, |r| r.bau_sekunden / TAG)), true);
        self.dyson_register(pid, "Baubeginn der Dysonsphäre");
        Ok(format!("Bau der Dysonsphäre im System {koord} begonnen"))
    }

    pub fn dyson_abbrechen(&mut self, sid: SpielerId, pid: u32, bestaetigt: bool) -> Result<String, String> {
        let p = self.ausbau.dyson.projekte.get(&pid).ok_or("Dysonprojekt gibt es nicht")?;
        self.dyson_leitung(sid, p.allianz)?;
        if !p.phase.offen() { return Err("Abbrechen geht nur vor oder während des Baus".into()); }
        if !bestaetigt { return Err("Abbruch bitte mit bestaetigt: true bestätigen".into()); }
        Ok(self.dyson_abbruch(pid, "Abbruch durch die Allianzleitung"))
    }

    fn dyson_abbruch(&mut self, pid: u32, grund: &str) -> String {
        let r = self.dyson_regeln().cloned().expect("aktiv");
        let p = self.ausbau.dyson.projekte.get(&pid).cloned().expect("Projekt");
        let (quote, sperre) = if p.phase == DysonPhase::Bau {
            let f = (p.bau_erledigt as f64 / r.bau_sekunden as f64).clamp(0.0, 1.0);
            (r.rueckzahlung_abbruch_im_bau * (1.0 - f), r.sperre_nach_abbruch_tage * TAG)
        } else { (r.rueckzahlung_abbruch_vor_bau, 0) };
        for (s, b) in &p.mitglieder { let rueck = b.eingezahlt.map(|x| mal(x, quote)); self.dyson_guthaben(*s, rueck); }
        let zeit = self.zeit;
        let q = self.ausbau.dyson.projekte.get_mut(&pid).unwrap();
        q.phase = DysonPhase::Abgebrochen;
        q.abgebrochen = Some(zeit);
        q.gesperrt_bis = zeit + sperre;
        q.ruht_seit = None;
        let text = format!("Dysonprojekt {} abgebrochen ({grund}); Rückzahlung {} % als Guthaben", p.koord(), (quote * 100.0).round());
        self.dyson_ereignis(pid, "dyson_abgebrochen", text.clone(), true);
        if p.phase == DysonPhase::Bau { self.dyson_register(pid, "Abbruch der Dysonsphäre"); }
        text
    }

    pub fn dyson_anschluss_waehlen(&mut self, sid: SpielerId, pid: u32, slot: usize, k: Koord) -> Result<String, String> {
        let r = self.dyson_regeln().ok_or("Die Dysonsphäre gibt es in dieser Welt nicht")?.clone();
        let p = self.ausbau.dyson.projekte.get(&pid).ok_or("Dysonprojekt gibt es nicht")?;
        self.dyson_leitung(sid, p.allianz)?;
        if p.phase != DysonPhase::Vollendet { return Err("Anschlüsse gibt es erst an der vollendeten Sphäre".into()); }
        if slot == 0 || slot > r.anschluesse { return Err(format!("Anschluss-Platz 1 bis {}", r.anschluesse)); }
        let i = slot - 1;
        if p.anschluesse[i].is_some() { return Err(format!("Anschluss {slot} ist belegt – erst dyson_anschluss_loesen")); }
        if p.slot_sperre_bis[i] > self.zeit { return Err(format!("Anschluss {slot} ist gesperrt bis {}", zeittext(p.slot_sperre_bis[i]))); }
        if !p.im_system(k) { return Err(format!("Nur Planeten im System {} können angeschlossen werden", p.koord())); }
        let planet = *self.belegung.get(&k).ok_or_else(|| format!("Auf {k} liegt kein Planet"))? as usize;
        if !self.planet_aktiv(planet) { return Err(format!("{k} ist nicht besiedelt")); }
        let besitzer = self.planeten[planet].besitzer;
        if !p.mitglieder.contains_key(&besitzer) || !self.dyson_in_allianz(besitzer, p.allianz) { return Err("Der Besitzer muss Projektmitglied der Eigner-Allianz sein".into()); }
        let eigene = p.anschluesse.iter().flatten().filter(|a| a.besitzer == besitzer).count();
        if eigene >= r.anschluesse_je_spieler { return Err(format!("Höchstens {} Anschluss je Spieler", r.anschluesse_je_spieler)); }
        let zeit = self.zeit;
        let aktiv_ab = zeit + r.anschluss_vorlauf_stunden * STUNDE;
        let q = self.ausbau.dyson.projekte.get_mut(&pid).unwrap();
        q.anschluesse[i] = Some(Anschluss { planet: planet as PlanetId, besitzer, gewaehlt: zeit, aktiv_ab, angewendet: false });
        q.slot_sperre_bis[i] = aktiv_ab + r.anschluss_sperre_tage * TAG;
        let name = self.spieler[besitzer as usize].name.clone();
        self.dyson_ereignis(pid, "dyson_anschluss", format!("Anschluss {slot}: {k} ({name}) – Spiegel werden ausgerichtet, aktiv ab {}", zeittext(aktiv_ab)), false);
        Ok(format!("Anschluss {slot} auf {k} gesetzt, aktiv ab {}", zeittext(aktiv_ab)))
    }

    pub fn dyson_anschluss_loesen(&mut self, sid: SpielerId, pid: u32, slot: usize) -> Result<String, String> {
        let p = self.ausbau.dyson.projekte.get(&pid).ok_or("Dysonprojekt gibt es nicht")?;
        self.dyson_leitung(sid, p.allianz)?;
        if slot == 0 || slot > p.anschluesse.len() { return Err("Unbekannter Anschluss-Platz".into()); }
        let i = slot - 1;
        let a = p.anschluesse[i].clone().ok_or_else(|| format!("Anschluss {slot} ist frei"))?;
        if p.slot_sperre_bis[i] > self.zeit { return Err(format!("Anschluss {slot} ist gesperrt bis {}", zeittext(p.slot_sperre_bis[i]))); }
        self.ausbau.dyson.projekte.get_mut(&pid).unwrap().anschluesse[i] = None;
        self.raten_neu(a.planet as usize);
        Ok(format!("Anschluss {slot} gelöst"))
    }

    /// Rückzahlungsguthaben auf einen eigenen Planeten buchen (höchstens bis zur Lagergrenze).
    pub fn dyson_auszahlen(&mut self, sid: SpielerId, k: Koord, mengen: [i64; 3]) -> Result<String, String> {
        if self.dyson_regeln().is_none() { return Err("Die Dysonsphäre gibt es in dieser Welt nicht".into()); }
        let g = self.ausbau.dyson.guthaben.get(&sid).copied().ok_or("Kein Rückzahlungsguthaben")?;
        let planet = self.eigener_planet(sid, k)?;
        self.abrechnen(planet);
        let grenze = self.lagergrenze(planet);
        let mut ab = [0i64; 3];
        for (i, gut) in ROHSTOFFE.iter().enumerate() {
            let wunsch = if mengen[i] < 0 { 0 } else { mengen[i].saturating_mul(M) };
            let frei = (grenze[gut.idx()] - self.planeten[planet].bestand[gut.idx()]).max(0);
            ab[i] = wunsch.min(g[i]).min(frei);
        }
        if ab.iter().all(|x| *x == 0) { return Err("Nichts auszuzahlen (Guthaben leer oder Lager voll)".into()); }
        for (i, gut) in ROHSTOFFE.iter().enumerate() { self.planeten[planet].bestand[gut.idx()] += ab[i]; }
        self.dyson_guthaben(sid, ab.map(|x| -x));
        self.raten_neu(planet);
        Ok(format!("Auf {k} ausgezahlt: {} Erz, {} Kristall, {} Deuterium", ab[0] / M, ab[1] / M, ab[2] / M))
    }

    /// Allianzbeitritt oder -austritt (aus `diplomatie.rs`): Mitgliedschaftszeit, Projektfolgen, Raten im System.
    pub(crate) fn dyson_allianzwechsel(&mut self, sid: SpielerId) {
        if !self.ausbau.dyson.aktiv { return; }
        match self.spieler[sid as usize].allianz {
            Some(a) => { self.ausbau.dyson.mitglied_seit.insert(sid, (a, self.zeit)); }
            None => { self.ausbau.dyson.mitglied_seit.remove(&sid); }
        }
        let r = self.dyson_regeln().cloned().expect("aktiv");
        let ids: Vec<u32> = self.ausbau.dyson.projekte.keys().copied().collect();
        for id in ids {
            let p = &self.ausbau.dyson.projekte[&id];
            if p.mitglieder.contains_key(&sid) && self.spieler[sid as usize].allianz != Some(p.allianz) {
                match p.phase {
                    DysonPhase::Sammeln => {
                        let rueck = p.mitglieder[&sid].eingezahlt.map(|x| mal(x, r.rueckzahlung_austritt));
                        self.ausbau.dyson.projekte.get_mut(&id).unwrap().mitglieder.remove(&sid);
                        self.dyson_guthaben(sid, rueck);
                    }
                    DysonPhase::Bau => { self.ausbau.dyson.projekte.get_mut(&id).unwrap().mitglieder.remove(&sid); }
                    _ => {
                        let q = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
                        for a in q.anschluesse.iter_mut() { if a.as_ref().is_some_and(|a| a.besitzer == sid) { *a = None; } }
                    }
                }
            }
            let (s, y) = { let p = &self.ausbau.dyson.projekte[&id]; (p.sektor, p.system) };
            if self.dyson_hat_planet(sid, s, y) && matches!(self.ausbau.dyson.projekte[&id].phase, DysonPhase::Vollendet) {
                for pid in self.dyson_system_planeten(s, y) { if self.planeten[pid].besitzer == sid { self.raten_neu(pid); } }
            }
        }
    }

    // ------------------------------------------------------------ Takt

    pub(crate) fn dyson_naechster_termin(&self) -> Option<SimZeit> {
        let r = self.dyson_regeln()?;
        let mut t = self.ausbau.dyson.pruef_stand + STUNDE;
        for p in self.ausbau.dyson.projekte.values() {
            if p.phase == DysonPhase::Bau && p.ruht_seit.is_none() { t = t.min(p.bau_stand + (r.bau_sekunden - p.bau_erledigt).max(1)); }
            for a in p.anschluesse.iter().flatten() { if !a.angewendet { t = t.min(a.aktiv_ab); } }
        }
        Some(t.max(self.zeit + 1))
    }

    pub(crate) fn dyson_tick(&mut self) {
        let Some(r) = self.dyson_regeln().cloned() else { return };
        let jetzt = self.zeit;
        let ids: Vec<u32> = self.ausbau.dyson.projekte.keys().copied().collect();
        for id in &ids {
            let mut fertig = false;
            let mut phase_neu = None;
            {
                let p = self.ausbau.dyson.projekte.get_mut(id).unwrap();
                if p.phase == DysonPhase::Bau && p.ruht_seit.is_none() && jetzt > p.bau_stand {
                    p.bau_erledigt = (p.bau_erledigt + jetzt - p.bau_stand).min(r.bau_sekunden);
                    p.bau_stand = jetzt;
                    let viertel = (p.bau_erledigt * 4 / r.bau_sekunden) as u8;
                    if viertel > p.phase_gemeldet && viertel < 4 { p.phase_gemeldet = viertel; phase_neu = Some(viertel); }
                    fertig = p.bau_erledigt >= r.bau_sekunden;
                }
            }
            if let Some(v) = phase_neu {
                let k = self.ausbau.dyson.projekte[id].koord();
                let name = ["", "Äquatorgürtel", "Halbschale", "Polkappen"][v as usize];
                self.dyson_ereignis(*id, "dyson_phase", format!("Dysonsphäre {k}: {} % – {name}", v as u32 * 25), true);
                self.dyson_register(*id, &format!("Dysonsphäre {} %", v as u32 * 25));
            }
            if fertig { self.dyson_vollenden(*id); }
            // Anschlüsse aktiv
            let faellig: Vec<(usize, PlanetId)> = self.ausbau.dyson.projekte[id].anschluesse.iter().enumerate()
                .filter_map(|(i, a)| a.as_ref().filter(|a| !a.angewendet && a.aktiv_ab <= jetzt).map(|a| (i, a.planet))).collect();
            for (i, planet) in faellig {
                self.ausbau.dyson.projekte.get_mut(id).unwrap().anschluesse[i].as_mut().unwrap().angewendet = true;
                self.raten_neu(planet as usize);
                let k = self.planeten[planet as usize].koord;
                self.dyson_ereignis(*id, "dyson_anschluss_aktiv", format!("Anschluss {} auf {k} ist aktiv", i + 1), false);
            }
        }
        while self.ausbau.dyson.pruef_stand + STUNDE <= jetzt {
            self.ausbau.dyson.pruef_stand += STUNDE;
            self.dyson_pruefen();
        }
    }

    fn dyson_vollenden(&mut self, id: u32) {
        let zeit = self.zeit;
        let (s, y, k) = {
            let p = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
            p.phase = DysonPhase::Vollendet;
            p.vollendet = Some(zeit);
            p.phase_gemeldet = 4;
            (p.sektor, p.system, p.koord())
        };
        self.dyson_ereignis(id, "dyson_vollendet", format!("Die Dysonsphäre im System {k} ist vollendet: Sonnenfaktor × {} für die Allianz, Anschlüsse wählbar", self.dyson_regeln().map_or(1.6, |r| r.sonnenfaktor).to_string().replace('.', ",")), true);
        self.dyson_register(id, "Dysonsphäre vollendet");
        self.dyson_raten_system(s, y);
    }

    /// Stündliche Prüfung: Ruhen/Fortsetzen, Ruhefrist, Verwaisung, Übernahme, erloschene Anschlüsse.
    fn dyson_pruefen(&mut self) {
        let Some(r) = self.dyson_regeln().cloned() else { return };
        let jetzt = self.zeit;
        let ids: Vec<u32> = self.ausbau.dyson.projekte.keys().copied().collect();
        for id in ids {
            let p = self.ausbau.dyson.projekte[&id].clone();
            let allianz_lebt = self.allianzen.iter().any(|a| a.id == p.allianz && !a.mitglieder.is_empty());
            match p.phase {
                DysonPhase::Sammeln => { if !allianz_lebt { self.dyson_abbruch(id, "Allianz aufgelöst"); } }
                DysonPhase::Bau => {
                    if !allianz_lebt { self.dyson_abbruch(id, "Allianz aufgelöst"); continue; }
                    let grund = self.dyson_bau_grund(id);
                    match (grund, p.ruht_seit) {
                        (Some(g), None) => {
                            let q = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
                            q.ruht_seit = Some(jetzt);
                            q.ruhe_grund = g.clone();
                            self.dyson_ereignis(id, "dyson_bau_ruht", format!("Bau der Dysonsphäre {} ruht: {g}", p.koord()), true);
                        }
                        (Some(_), Some(seit)) => {
                            if p.ruhe_summe + (jetzt - seit) > r.ruhe_frist_tage * TAG { self.dyson_abbruch(id, "Ruhefrist abgelaufen"); }
                        }
                        (None, Some(seit)) => {
                            let q = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
                            q.ruhe_summe += jetzt - seit;
                            q.ruht_seit = None;
                            q.ruhe_grund.clear();
                            q.bau_stand = jetzt;
                            self.dyson_ereignis(id, "dyson_bau_fortgesetzt", format!("Bau der Dysonsphäre {} läuft weiter", p.koord()), true);
                        }
                        (None, None) => {}
                    }
                }
                DysonPhase::Vollendet => {
                    // Erloschene Anschlüsse: Besitzer gewechselt, nicht mehr in der Allianz oder Planet aufgegeben.
                    let mut geaendert = Vec::new();
                    for (i, a) in p.anschluesse.iter().enumerate() {
                        if let Some(a) = a {
                            let pl = a.planet as usize;
                            if !self.planet_aktiv(pl) || self.planeten[pl].besitzer != a.besitzer || !self.dyson_in_allianz(a.besitzer, p.allianz) { geaendert.push((i, pl)); }
                        }
                    }
                    for (i, pl) in geaendert {
                        let q = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
                        q.anschluesse[i] = None;
                        q.slot_sperre_bis[i] = 0;
                        self.raten_neu(pl);
                        self.dyson_ereignis(id, "dyson_anschluss_erloschen", format!("Anschluss {} erloschen (Planet nicht mehr in Allianzhand)", i + 1), false);
                    }
                    let halt = self.dyson_allianz_plaetze(p.allianz, p.sektor, p.system);
                    if !allianz_lebt || halt < r.halten_min_plaetze as usize {
                        let q = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
                        q.phase = DysonPhase::Verwaist;
                        q.verwaist_seit = Some(jetzt);
                        q.uebernahme = None;
                        for a in q.anschluesse.iter_mut() { *a = None; }
                        self.dyson_ereignis(id, "dyson_verwaist", format!("Die Dysonsphäre {} ist verwaist ({halt} Plätze gehalten) – keine Wirkung", p.koord()), true);
                        self.dyson_register(id, "Dysonsphäre verwaist");
                        self.dyson_raten_system(p.sektor, p.system);
                    }
                }
                DysonPhase::Verwaist => {
                    // Übernahme: eine Allianz hält alle Plätze mit ≥ Mindestzahl Mitgliedern mit Planet im System.
                    let n = self.regeln.welt.plaetze_je_system as usize;
                    let planeten = self.dyson_system_planeten(p.sektor, p.system);
                    let kandidat = if planeten.len() == n {
                        let allianzen: Vec<Option<u32>> = planeten.iter().map(|pl| self.spieler[self.planeten[*pl].besitzer as usize].allianz).collect();
                        allianzen.first().copied().flatten().filter(|a| allianzen.iter().all(|x| *x == Some(*a))).filter(|a| {
                            let mut sp: Vec<SpielerId> = planeten.iter().map(|pl| self.planeten[*pl].besitzer).collect();
                            sp.sort(); sp.dedup();
                            sp.len() >= r.mitglieder_min && self.allianzen.iter().any(|x| x.id == *a)
                        })
                    } else { None };
                    match (kandidat, p.uebernahme) {
                        (Some(a), Some((b, seit))) if a == b => {
                            if jetzt - seit >= r.uebernahme_tage * TAG {
                                let q = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
                                q.allianz = a;
                                q.phase = DysonPhase::Vollendet;
                                q.verwaist_seit = None;
                                q.uebernahme = None;
                                let mut mitglieder = BTreeMap::new();
                                for pl in &planeten { mitglieder.entry(self.planeten[*pl].besitzer).or_insert(Beitrag { beigetreten: jetzt, eingezahlt: [0; 3], voll_seit: None }); }
                                let alt = std::mem::take(&mut self.ausbau.dyson.projekte.get_mut(&id).unwrap().mitglieder);
                                let q = self.ausbau.dyson.projekte.get_mut(&id).unwrap();
                                for (s, b) in mitglieder { q.mitglieder.insert(s, alt.get(&s).cloned().unwrap_or(b)); }
                                self.dyson_ereignis(id, "dyson_uebernommen", format!("Die Dysonsphäre {} gehört jetzt eurer Allianz – Spiegel neu ausgerichtet", p.koord()), true);
                                self.dyson_register(id, "Dysonsphäre übernommen");
                                self.dyson_raten_system(p.sektor, p.system);
                            }
                        }
                        (Some(a), _) => { self.ausbau.dyson.projekte.get_mut(&id).unwrap().uebernahme = Some((a, jetzt)); }
                        (None, Some(_)) => { self.ausbau.dyson.projekte.get_mut(&id).unwrap().uebernahme = None; }
                        (None, None) => {}
                    }
                }
                DysonPhase::Abgebrochen => {}
            }
        }
    }

    // ------------------------------------------------------------ Wirkung

    fn dyson_wirksam_fuer(&self, besitzer: SpielerId, k: Koord) -> Option<&DysonProjekt> {
        if !self.ausbau.dyson.aktiv { return None; }
        let aid = self.spieler.get(besitzer as usize)?.allianz?;
        self.ausbau.dyson.projekte.values().find(|p| p.phase == DysonPhase::Vollendet && p.allianz == aid && p.im_system(k))
    }
    /// Vollendete Sphäre im System, die nicht der Allianz des Besitzers gehört (Ablauf v2): sie fängt dessen Sonne ab.
    fn dyson_beschattet(&self, besitzer: SpielerId, k: Koord) -> bool {
        if !self.ausbau.dyson.aktiv || !self.ablauf_v2() || self.dyson_wirksam_fuer(besitzer, k).is_some() { return false; }
        let aid = self.spieler.get(besitzer as usize).and_then(|s| s.allianz);
        self.ausbau.dyson.projekte.values().any(|p| p.phase == DysonPhase::Vollendet && Some(p.allianz) != aid && p.im_system(k))
    }
    /// Sonnenfaktor in Promille für einen Spieler an einer Stelle: Eigner-Allianz `sonnenfaktor`, beschattet `SCHATTEN`,
    /// sonst 1000 (Anzeige `klima_sicht.dyson_promille`).
    pub fn dyson_sonnen_promille(&self, sid: SpielerId, k: Koord) -> i64 {
        match self.dyson_regeln() {
            Some(r) if self.dyson_wirksam_fuer(sid, k).is_some() => (r.sonnenfaktor * 1000.0).round() as i64,
            Some(_) if self.dyson_beschattet(sid, k) => (self.regeln.kw().sonne.dyson_schatten * 1000.0).round() as i64,
            _ => 1000,
        }
    }
    /// Wirkung auf einen Planeten (`None` = keine; dann rechnet die Wirtschaft exakt wie ohne Sphäre).
    pub fn dyson_wirkung(&self, p: &Planet) -> Option<DysonWirkung> {
        let r = self.dyson_regeln()?;
        if self.dyson_beschattet(p.besitzer, p.koord) {
            return Some(DysonWirkung { sonnenfaktor: self.regeln.kw().sonne.dyson_schatten, anschluss: false, einspeisung: 0, deuterium: 0, xenokristall: 0,
                strombad: 1.0, xeno_ohne_malus: false, rezept_faktor: 1.0 });
        }
        let proj = self.dyson_wirksam_fuer(p.besitzer, p.koord)?;
        let anschluss = proj.anschluesse.iter().flatten().any(|a| a.planet == p.id && a.besitzer == p.besitzer && a.aktiv_ab <= self.zeit);
        Some(DysonWirkung {
            sonnenfaktor: r.sonnenfaktor,
            anschluss,
            einspeisung: if anschluss { self.dyson_skaliert(r.einspeisung) } else { 0 },
            deuterium: if anschluss { self.dyson_skaliert(r.strahlsynthese_deuterium) } else { 0 },
            xenokristall: if anschluss { self.dyson_skaliert(r.strahlsynthese_xenokristall) } else { 0 },
            strombad: if anschluss { r.strombad } else { 1.0 },
            xeno_ohne_malus: anschluss && r.xenosynthese_ohne_malus,
            rezept_faktor: if anschluss { r.xenosynthese_rezept_faktor } else { 1.0 },
        })
    }
    /// Faktor für Solarsatelliten auf Monden im Dyson-System der Eigner-Allianz.
    pub fn dyson_sonnenfaktor(&self, besitzer: SpielerId, k: Koord) -> f64 {
        match (self.dyson_regeln(), self.dyson_wirksam_fuer(besitzer, k)) {
            (Some(r), Some(_)) => r.sonnenfaktor,
            (Some(_), None) if self.dyson_beschattet(besitzer, k) => self.regeln.kw().sonne.dyson_schatten,
            _ => 1.0,
        }
    }

    // ------------------------------------------------------------ Sicht

    fn dyson_projekt_sicht(&self, sid: SpielerId, p: &DysonProjekt) -> Value {
        let r = self.dyson_regeln().expect("aktiv");
        let soll = self.ausbau.dyson.anteil;
        let tag = self.allianz_tag(p.allianz);
        let rest = match p.phase { DysonPhase::Bau => (r.bau_sekunden - p.bau_erledigt).max(0), _ => 0 };
        let mitglieder: Vec<Value> = p.mitglieder.iter().map(|(s, b)| {
            let sp = &self.spieler[*s as usize];
            json!({"name":sp.name,"volk":sp.volk.name(),"eingezahlt":b.eingezahlt.map(|x| x / M),"voll":self.dyson_voll(b),"beigetreten":b.beigetreten,
                "in_allianz":sp.allianz == Some(p.allianz),"planet_im_system":self.dyson_hat_planet(*s, p.sektor, p.system),"stufe":sp.stufe,"eigene":*s == sid})
        }).collect();
        let anschluesse: Vec<Value> = p.anschluesse.iter().enumerate().map(|(i, a)| match a {
            Some(a) => json!({"slot":i+1,"planet":self.planeten[a.planet as usize].koord.to_string(),"besitzer":self.spieler[a.besitzer as usize].name,"aktiv_ab":a.aktiv_ab,"aktiv":a.aktiv_ab<=self.zeit,"sperre_bis":p.slot_sperre_bis[i]}),
            None => json!({"slot":i+1,"planet":null,"sperre_bis":p.slot_sperre_bis[i]}),
        }).collect();
        let eigene_planeten: Vec<String> = self.dyson_system_planeten(p.sektor, p.system).into_iter().filter(|pl| self.planeten[*pl].besitzer == sid).map(|pl| self.planeten[pl].koord.to_string()).collect();
        let eigene_monde: Vec<String> = self.monde.monde.values().filter(|m| m.besitzer == sid && p.im_system(m.koord)).map(|m| m.koord.to_string()).collect();
        let ruhe_rest = p.ruht_seit.map(|seit| (r.ruhe_frist_tage * TAG - p.ruhe_summe - (self.zeit - seit)).max(0));
        json!({
            "id":p.id,"system":p.koord(),"sektor":p.sektor,"systemnummer":p.system,"allianz_tag":tag,"phase":p.phase.name(),
            "fortschritt_promille":match p.phase { DysonPhase::Vollendet | DysonPhase::Verwaist => 1000, _ => (p.bau_erledigt * 1000 / r.bau_sekunden.max(1)).min(1000) },
            "rest_sekunden":rest,"ruht":p.ruht_seit.is_some(),"ruhe_grund":p.ruhe_grund,"ruhe_rest_sekunden":ruhe_rest,
            "bau_begonnen":p.bau_begonnen,"vollendet":p.vollendet,"verwaist_seit":p.verwaist_seit,"gesperrt_bis":p.gesperrt_bis,
            "bedingungen":if p.phase == DysonPhase::Sammeln { json!(self.dyson_bedingungen(p.id)) } else { json!([]) },
            "kasse":p.kasse().map(|x| x / M),"anteil":soll,"mindestbuchung":self.dyson_mindestbuchung(),
            "mitglieder":mitglieder,"anschluesse":anschluesse,
            "leitung":self.spieler[sid as usize].allianz == Some(p.allianz) && self.allianz_leitung(sid, p.allianz),
            "mitglied":p.mitglieder.contains_key(&sid),"eigene_planeten":eigene_planeten,"eigene_monde":eigene_monde,
            "plaetze":{"allianz":self.dyson_allianz_plaetze(p.allianz, p.sektor, p.system),"gesamt":self.regeln.welt.plaetze_je_system,"halten_min":r.halten_min_plaetze},
        })
    }

    /// Lagebild der Dysonsphäre für einen Spieler (nur in Dyson-Welten).
    pub fn dyson_sicht(&self, sid: SpielerId) -> Value {
        let Some(r) = self.dyson_regeln() else { return json!({"aktiv":false}) };
        let aid = self.spieler[sid as usize].allianz;
        let eigene: Vec<Value> = self.ausbau.dyson.projekte.values().filter(|p| Some(p.allianz) == aid && p.phase != DysonPhase::Abgebrochen)
            .map(|p| self.dyson_projekt_sicht(sid, p)).collect();
        let oeffentlich: Vec<Value> = self.ausbau.dyson.projekte.values().filter(|p| p.phase != DysonPhase::Abgebrochen && p.phase != DysonPhase::Sammeln)
            .map(|p| self.dyson_oeffentlich(sid, p)).collect();
        let s = self.dyson_skala_ppm();
        // Nach einem Abbruch gesperrte Systeme vorab zeigen (Funktionsprüfung 25.10.2026, dyson#13: die Sperre kam erst als
        // Fehlermeldung nach dem Absenden). Nur Systeme, in denen die eigene Allianz vorschlagen könnte – dieselbe Auskunft
        // gibt der Kern sonst beim Vorschlag.
        let gesperrt: Vec<Value> = match aid {
            Some(a) => self.ausbau.dyson.projekte.values()
                .filter(|p| p.phase == DysonPhase::Abgebrochen && p.gesperrt_bis > self.zeit && (p.allianz == a || self.dyson_allianz_plaetze(a, p.sektor, p.system) > 0))
                .map(|p| json!({"system":p.koord(),"bis":p.gesperrt_bis,"bis_text":zeittext(p.gesperrt_bis)})).collect(),
            None => Vec::new(),
        };
        json!({
            "aktiv":true,"gesperrt":gesperrt,"anteil":self.ausbau.dyson.anteil,"skala_ppm":s,"mindestbuchung":self.dyson_mindestbuchung(),
            "guthaben":self.ausbau.dyson.guthaben.get(&sid).copied().unwrap_or([0; 3]).map(|x| x / M),
            "projekte":eigene,"sphaeren":oeffentlich,
            "eigene_anschluesse":self.ausbau.dyson.projekte.values().flat_map(|p| p.anschluesse.iter().flatten()).filter(|a| a.besitzer == sid).map(|a| self.planeten[a.planet as usize].koord.to_string()).collect::<Vec<_>>(),
            "regeln":{"mitglieder_min":r.mitglieder_min,"stufe_min":r.stufe_min,"mitgliedschaft_min_tage":r.mitgliedschaft_min_tage,"bau_tage":r.bau_sekunden / TAG,
                "sonnenfaktor":r.sonnenfaktor,"anschluesse":r.anschluesse,"anschluesse_je_spieler":r.anschluesse_je_spieler,"anschluss_vorlauf_stunden":r.anschluss_vorlauf_stunden,
                "anschluss_sperre_tage":r.anschluss_sperre_tage,"einspeisung":self.dyson_skaliert(r.einspeisung) / M,"strahlsynthese_deuterium":self.dyson_skaliert(r.strahlsynthese_deuterium) / M,
                "strahlsynthese_xenokristall":self.dyson_skaliert(r.strahlsynthese_xenokristall) / M,"strombad":r.strombad,"halten_min_plaetze":r.halten_min_plaetze,
                "ruhe_frist_tage":r.ruhe_frist_tage,"uebernahme_tage":r.uebernahme_tage,"rueckzahlung_austritt":r.rueckzahlung_austritt,"rueckzahlung_abbruch_im_bau":r.rueckzahlung_abbruch_im_bau},
        })
    }
    fn dyson_oeffentlich(&self, sid: SpielerId, p: &DysonProjekt) -> Value {
        let r = self.dyson_regeln().expect("aktiv");
        let eigen = self.spieler[sid as usize].allianz == Some(p.allianz);
        // Vor Baubeginn 0 % (Funktionsprüfung 25.10.2026: „sammeln“ zeigte 100 % und eine fertige Sphäre).
        let prozent = match p.phase { DysonPhase::Bau => (p.bau_erledigt * 100 / r.bau_sekunden.max(1)) / 5 * 5, DysonPhase::Sammeln => 0, _ => 100 };
        let mut v = json!({"system":p.koord(),"allianz_tag":self.allianz_tag(p.allianz),"phase":p.phase.name(),"fortschritt_prozent":prozent,"ruht":p.ruht_seit.is_some(),"eigen":eigen});
        if eigen {
            v["rest_sekunden"] = json!(if p.phase == DysonPhase::Bau { (r.bau_sekunden - p.bau_erledigt).max(0) } else { 0 });
            v["anschluesse"] = json!(p.anschluesse.iter().flatten().map(|a| self.planeten[a.planet as usize].koord.to_string()).collect::<Vec<_>>());
        }
        v
    }
    /// Eintrag für die Galaxie-Zeile über Platz 1 (öffentlich, sobald gebaut wird).
    pub fn dyson_im_system(&self, sid: SpielerId, sektor: u8, system: u8) -> Option<Value> {
        self.dyson_regeln()?;
        let p = self.dyson_projekt_im_system(sektor, system)?;
        if matches!(p.phase, DysonPhase::Abgebrochen) { return None; }
        if p.phase == DysonPhase::Sammeln && self.spieler[sid as usize].allianz != Some(p.allianz) { return None; }
        Some(self.dyson_oeffentlich(sid, p))
    }

    // ------------------------------------------------------------ Aktionsverteilung

    pub(crate) fn dyson_aktion(&mut self, sid: SpielerId, rolle: Rolle, a: &Value) -> Option<Result<String, String>> {
        let typ = a.get("typ").and_then(Value::as_str)?;
        if !DYSON_AKTIONEN.contains(&typ) { return None; }
        if self.dyson_regeln().is_none() { return Some(Err("Die Dysonsphäre gibt es in dieser Welt nicht".into())); }
        if rolle != Rolle::Alle && !dyson_zustaendig(typ).contains(&rolle) {
            let wer: Vec<&str> = dyson_zustaendig(typ).iter().map(|r| r.name()).collect();
            return Some(Err(format!("{typ} gehört nicht zur Rolle {rolle}, zuständig: {}", wer.join(", "))));
        }
        let k = |f: &str| -> Result<Koord, String> { a.get(f).and_then(Value::as_str).ok_or(format!("Feld {f} fehlt"))?.parse::<Koord>() };
        let menge = |f: &str| -> Result<i64, String> {
            match a.get(f) { None | Some(Value::Null) => Ok(0), Some(v) => { let x = v.as_f64().ok_or(format!("{f} ist keine Zahl"))?; if !x.is_finite() || x < 0.0 { return Err(format!("{f} ungültig")); } Ok(x.floor() as i64) } }
        };
        Some((|| -> Result<String, String> {
            match typ {
                "dyson_vorschlagen" => {
                    let s = a.get("system").and_then(Value::as_str).ok_or("Feld system fehlt (Sektor:System)")?;
                    let t: Vec<&str> = s.split(':').collect();
                    if t.len() < 2 { return Err("system als Sektor:System angeben".into()); }
                    let z = |x: &str| x.trim().parse::<u8>().map_err(|_| "system als Sektor:System angeben".to_string());
                    self.dyson_vorschlagen(sid, z(t[0])?, z(t[1])?)
                }
                "dyson_beitreten" => { let p = self.dyson_projekt_holen(a, sid)?; self.dyson_beitreten(sid, p) }
                "dyson_austreten" => { let p = self.dyson_projekt_holen(a, sid)?; self.dyson_austreten(sid, p) }
                "dyson_einzahlen" => {
                    let p = self.dyson_projekt_holen(a, sid)?;
                    let mond = a.get("zielart").and_then(Value::as_str) == Some("mond");
                    self.dyson_einzahlen(sid, p, k("von")?, mond, [menge("erz")?, menge("kristall")?, menge("deuterium")?])
                }
                "dyson_bau_starten" => { let p = self.dyson_projekt_holen(a, sid)?; self.dyson_bau_starten(sid, p) }
                "dyson_abbrechen" => { let p = self.dyson_projekt_holen(a, sid)?; self.dyson_abbrechen(sid, p, a.get("bestaetigt").and_then(Value::as_bool).unwrap_or(false)) }
                "dyson_anschluss_waehlen" => {
                    let p = self.dyson_projekt_holen(a, sid)?;
                    let slot = a.get("slot").and_then(Value::as_u64).ok_or("Feld slot fehlt")? as usize;
                    self.dyson_anschluss_waehlen(sid, p, slot, k("planet")?)
                }
                "dyson_anschluss_loesen" => {
                    let p = self.dyson_projekt_holen(a, sid)?;
                    let slot = a.get("slot").and_then(Value::as_u64).ok_or("Feld slot fehlt")? as usize;
                    self.dyson_anschluss_loesen(sid, p, slot)
                }
                "dyson_auszahlen" => self.dyson_auszahlen(sid, k("planet")?, [menge("erz")?, menge("kristall")?, menge("deuterium")?]),
                _ => Err("unbekannte Dysonaktion".into()),
            }
        })())
    }
}

pub const DYSON_AKTIONEN: [&str; 9] = ["dyson_vorschlagen", "dyson_beitreten", "dyson_austreten", "dyson_einzahlen", "dyson_bau_starten",
    "dyson_abbrechen", "dyson_anschluss_waehlen", "dyson_anschluss_loesen", "dyson_auszahlen"];

pub fn dyson_zustaendig(typ: &str) -> &'static [Rolle] {
    use Rolle::*;
    match typ {
        "dyson_einzahlen" | "dyson_auszahlen" => &[Verwalter, Stratege],
        "dyson_beitreten" | "dyson_austreten" => &[Stratege, Diplomat],
        _ => &[Diplomat, Stratege],
    }
}

/// JSON-Schemata der Dysonaktionen für Agenten (nur in Dyson-Welten).
pub fn dyson_aktionsschemas(rolle: Rolle) -> Vec<Value> {
    let objekt = |typ: &str, felder: Vec<(&str, Value, bool)>| {
        let mut props = serde_json::Map::new();
        props.insert("typ".into(), json!({"type":"string","enum":[typ]}));
        let mut pflicht = vec![json!("typ")];
        for (k, v, p) in felder { if p { pflicht.push(json!(k)); } props.insert(k.into(), v); }
        json!({"type":"object","properties":props,"required":pflicht,"additionalProperties":false})
    };
    let projekt = || (("projekt"), json!({"type":"integer","minimum":1,"description":"Projektnummer; ohne Angabe das Projekt der eigenen Allianz"}), false);
    let koord = |d: &str| json!({"type":"string","description":d});
    let zahl = || json!({"type":"number","minimum":0,"description":"ganze Einheiten"});
    let alle = vec![
        objekt("dyson_vorschlagen", vec![("system", koord("Sektor:System des Dyson-Systems"), true)]),
        objekt("dyson_beitreten", vec![projekt()]),
        objekt("dyson_austreten", vec![projekt()]),
        objekt("dyson_einzahlen", vec![projekt(), ("von", koord("eigener Planet oder Mond im Dyson-System"), true), ("zielart", json!({"type":"string","enum":["planet","mond"]}), false),
            ("erz", zahl(), false), ("kristall", zahl(), false), ("deuterium", zahl(), false)]),
        objekt("dyson_bau_starten", vec![projekt()]),
        objekt("dyson_abbrechen", vec![projekt(), ("bestaetigt", json!({"type":"boolean"}), true)]),
        objekt("dyson_anschluss_waehlen", vec![projekt(), ("slot", json!({"type":"integer","minimum":1,"maximum":8}), true), ("planet", koord("Planet im Dyson-System"), true)]),
        objekt("dyson_anschluss_loesen", vec![projekt(), ("slot", json!({"type":"integer","minimum":1,"maximum":8}), true)]),
        objekt("dyson_auszahlen", vec![("planet", koord("eigener Planet"), true), ("erz", zahl(), false), ("kristall", zahl(), false), ("deuterium", zahl(), false)]),
    ];
    alle.into_iter().filter(|v| {
        let typ = v["properties"]["typ"]["enum"][0].as_str().unwrap_or("");
        rolle == Rolle::Alle || dyson_zustaendig(typ).contains(&rolle)
    }).collect()
}

/// Regeltext der Dysonsphäre für Agenten und Kodex (Anteil und absolute Mengen dieser Welt).
pub fn regeltext(r: &DysonRegeln, anteil: i64) -> String {
    let s = anteil as f64 / r.anteil_voll as f64;
    let sk = |x: f64| ((x * s).round() as i64).to_string();
    format!("Dysonsphäre (Allianz-Großprojekt, eine je System): Die Allianzleitung schlägt ein System vor (dyson_vorschlagen), Allianzmitglieder mit eigenem Planet dort treten bei (dyson_beitreten) und zahlen ihren Anteil genau einmal in die zweckgebundene Projektkasse: je {anteil} Erz, Kristall und Deuterium (in dieser Welt; voller Anteil {} ab {} Tagen Epochendauer), in Teilzahlungen von eigenen Planeten oder Monden im System (dyson_einzahlen, Mindestbuchung {} oder der Rest; keine Überzahlung, nicht für andere, nicht plünderbar). Baubeginn (dyson_bau_starten) erst, wenn alle Plätze des Systems von Allianzmitgliedern besiedelt sind und mindestens {} Projektmitglieder mit Planet im System voll eingezahlt haben, alle auf Zivilisationsstufe {} und seit {} Spieltagen in der Allianz. Bauzeit {} Spieltage; fällt eine Bedingung weg, ruht der Bau, nach insgesamt {} Tagen Ruhe Abbruch. Rückzahlung als Guthaben (dyson_auszahlen): Austritt vor Baubeginn {} %, Abbruch vor Baubeginn {} %, im Bau {} % des nicht verbauten Teils. Wirkung erst bei Fertigstellung: Solarkraftwerke und Solarsatelliten der Allianz im System rechnen mit Sonnenfaktor × {}; bis zu {} angeschlossene Planeten im System (dyson_anschluss_waehlen, höchstens {} je Spieler, aktiv nach {} Stunden, danach {} Tage gesperrt) erhalten {} Energie je Stunde, volle Energiedeckung, Strahlsynthese {} Deuterium und {} Xenokristall je Stunde, Deuteriumsynthesizer und Xenoextraktor × {} und Xenosynthese ohne Abschlag. Die Sphäre ist kein Kampfziel; sie wirkt, solange die Eigner-Allianz mindestens {} Plätze hält, sonst ist sie verwaist und geht an eine Allianz über, die {} Tage alle Plätze mit mindestens {} Mitgliedern hält.",
        r.anteil_voll, r.voll_ab_tage, (r.mindestbuchung_voll as f64 * s).round().max(r.mindestbuchung_min as f64) as i64, r.mitglieder_min, r.stufe_min, r.mitgliedschaft_min_tage,
        r.bau_sekunden / TAG, r.ruhe_frist_tage, (r.rueckzahlung_austritt * 100.0).round(), (r.rueckzahlung_abbruch_vor_bau * 100.0).round(), (r.rueckzahlung_abbruch_im_bau * 100.0).round(),
        r.sonnenfaktor, r.anschluesse, r.anschluesse_je_spieler, r.anschluss_vorlauf_stunden, r.anschluss_sperre_tage, sk(r.einspeisung), sk(r.strahlsynthese_deuterium),
        sk(r.strahlsynthese_xenokristall), r.strombad, r.halten_min_plaetze, r.uebernahme_tage, r.mitglieder_min)
}
