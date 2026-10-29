//! Invasion v1 (Karl, 18.10.2026): Verteidigung brechen → nur Bomber senken die Integrität aller
//! Gebäude schrittweise bis genau 30 % → Bodentruppen landen → 48 Spielstunden Belagerung mit
//! Bodenrunden alle 15 Spielminuten → Übernahme nur bei gebrochener Bodenverteidigung und gehaltenem
//! Orbit. Während der Belagerung: eigener Nachschub landet trotz Blockade, Verbündete verstärken am
//! Boden (Mission halten/„verstärken“) und im Orbit, die vorher freigeschaltete Volksmiliz bewaffnet
//! alle Bewohner und kapituliert nach 30 % Verlusten.
//!
//! Aktivierung nur für neue Welten (`invasion_aktivieren`, Zustand in `ausbau.boden.invasion`,
//! JSON-Erweiterung der STERNE12/13-Hülle, leer = nicht serialisiert). Welten mit version 0
//! laden bytegleich und behalten die bisherigen Regeln.
use crate::boden_katalog::{ANZAHL, KAMPFZAHL, NAMEN};
use crate::flotte::Partei;
use crate::{kolonisation::Besetzung, typen::*, welt::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Abstand der Bomben- und Bodenrunden.
pub const RUNDE: SimZeit = 15 * MINUTE;
/// Belagerungsdauer ab Landung (2 Spieltage; Online-Tempo 1× = 2 Echttage).
pub const BELAGERUNG: SimZeit = 48 * STUNDE;
/// Untergrenze der Bombardierung und Bedingung für die Landung (Promille).
pub const INTEGRITAET_GRENZE: i64 = 300;
/// Strukturpunkte je Gebäudestufe gegen Bomben.
pub const STRUKTUR_JE_STUFE: i64 = 700;
/// Bombenlast je Bombenschacht und Runde.
pub const BOMBENLAST_JE_SCHACHT: i64 = 50;
/// Volksmiliz: Waffenwert 0,3 (Promille-Schaden je Person) und Rüstwert 5 je Bewohner.
pub const MILIZ_WAFFEN_PROMILLE: i64 = 300;
pub const MILIZ_RUESTUNG: i64 = 5;
/// Die Miliz kapituliert, sobald dieser Anteil des Aufgebots gefallen ist.
pub const MILIZ_KAPITULATION_PROMILLE: i64 = 300;
/// Anteil der Bewohner, der jeweils im Milizdienst steht und in Produktion und Forschung fehlt.
pub const MILIZ_DIENST_PROMILLE: i64 = 500;
/// Ausrüstung je bewaffnetem Bewohner (nicht erstattet).
pub const MILIZ_LEGIERUNG_JE_BEWOHNER: i64 = 1;
/// Freischaltung: Kaserne dieser Stufe auf dem Planeten.
pub const MILIZ_KASERNE: u8 = 1;
const BERICHTE_MAX: usize = 200;

/// Maßgeblicher Regeltext der Invasion v1; ergänzt in solchen Welten REGELTEXT_V2 und ersetzt dort
/// dessen Sätze zu Bombardement, Besetzungsfrist und Miliz.
pub const REGELTEXT_INVASION: &str = "Invasion v1 (Welten ohne Verteidigungslinien; ersetzt die Angaben zu Bombardement, 30-Minuten-Frist und 25-%-Miliz): 1. Verteidigung brechen: bombardieren (mindestens ein Bomber) oder kampfkolonisieren (Kolonieschiff) kämpfen zuerst gegen alle bewaffneten Schiffe, Verteidigungsanlagen und Planetenschilde, auch verbündete Flotten mit halten im Orbit; nur der Sieger hält den Orbit. 2. Bombardierung: nur Bomber wirken, andere Schiffe kämpfen ausschließlich gegen Flotten und Verteidigung. Die erste Bombenrunde fällt sofort, danach alle 15 Spielminuten. Bombenlast je Runde: Bomber 100, Strategischer Bomber 150, jede weitere Einheit der Klasse bomber 50. Strukturpunkte des Planeten: 700 je Gebäudestufe (Summe aller Stufen). Alle Gebäude und Anlagen verlieren je Runde gleich viel Integrität: aufgerundet 1000 × Bombenlast ÷ Strukturpunkte Promille, nie unter 30 %. Neue bewaffnete Verteidigung unterbricht: die Runde wird für den Kampf verbraucht, ein verlorener Kampf beendet die Bombardierung. Unter Bombardierung und Belagerung sind keine Reparaturen möglich; der Bauauftrag wird mit der ersten Runde storniert. Weitere bombardieren- und kampfkolonisieren-Flotten des Angreifers vereinigen sich mit der eigenen Blockade. 3. Landung: erst wenn alle Gebäude höchstens 30 % Integrität haben und die kampfkolonisieren-Flotte ein Kolonieschiff sowie Bodentruppen in Truppentransportern (100 Plätze je Transporter, boden_senden) trägt. Ohne Bodentruppen keine Landung. 4. Belagerung: 48 Spielstunden ab Landung (Online-Tempo 1x: 2 Echttage). Bodenrunden alle 15 Spielminuten mit gleichzeitigem Feuer und den Kontern der Bodentruppen. Übernahme frühestens nach Ablauf der 48 Stunden und nur, wenn keine Bodenverteidiger mehr leben und der Angreifer den Orbit mit Kolonieschiff hält. Sterben alle angreifenden Bodentruppen, gehen Orbit, Kolonieschiff oder Beziehung verloren, bricht die Belagerung ab. Die Bevölkerung bleibt bei Übernahme erhalten. Der ursprüngliche Heimatplanet ist dauerhaft geschützt. 5. Hilfe: eigene Bodentruppen landen während der Belagerung trotz Blockade (boden_senden mit transport oder stationieren; die Schiffe kehren um). Verbündete (Allianz oder Verteidigungsbündnis) schicken Bodentruppen mit boden_senden Mission verstaerken (wird zu halten): reine Truppenkonvois landen ohne Gefecht, ihre Truppen verteidigen mit, bleiben Eigentum des Absenders und kehren nach Ende der Belagerung heim. Verbündete Kampfflotten mit halten kämpfen bei Ankunft gegen die Besatzungsflotte; fällt sie, endet die Belagerung. 6. Miliz: freigeschaltet durch Kaserne Stufe 1 auf dem Planeten; miliz_ausrufen nur während einer Belagerung, einmal je Belagerung. Alle Bewohner erhalten Waffenwert 0,3 und Rüstwert 5 (Sturminfanterie 10 und 50). Kosten 1 Legierung je Bewohner, nicht erstattet; die Hälfte der Bewohner steht jeweils im Dienst und fehlt in Produktion und Forschung. Gefallene sind tote Bewohner. Nach 30 % Gefallenen des Aufgebots kapituliert die Miliz, die Überlebenden bleiben Bevölkerung. Ohne ausgerufene Miliz leisten Bewohner keinen Widerstand.";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Invasion {
    #[serde(default)]
    pub version: u8,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bombardements: BTreeMap<PlanetId, Bombardement>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub milizen: BTreeMap<PlanetId, Aufgebot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub berichte: Vec<Value>,
    /// Invasion v2 (0.4.8): Verteidigungslinien und Wellen je Planet.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub wellen: BTreeMap<PlanetId, crate::invasion2::Wellenkampf>,
    /// Invasion v2: Rückhalt gebrochen – der Besitzer hat die Kontrolle verloren („Verwüstet“).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub verwuestet: BTreeMap<PlanetId, crate::invasion2::Verwuestung>,
}
impl Invasion {
    pub fn leer(&self) -> bool {
        self.version == 0 && self.bombardements.is_empty() && self.milizen.is_empty() && self.berichte.is_empty()
            && self.wellen.is_empty() && self.verwuestet.is_empty()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bombardement {
    pub flotte: FlottenId,
    pub angreifer: SpielerId,
    pub verteidiger: SpielerId,
    pub seit: SimZeit,
    pub letzte_runde: SimZeit,
    pub runden: u32,
    #[serde(default)]
    pub unterbrochen: u32,
    #[serde(default)]
    pub abgeschlossen: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Aufgebot {
    pub besitzer: SpielerId,
    pub ausgerufen: SimZeit,
    pub aufgebot: i64,
    pub gefallen: i64,
    pub kapituliert: bool,
    /// Invasion v2: Ausbildung der Miliz von `beginn` bis `bereit` (0 = sofort einsatzbereit wie in v1).
    #[serde(default, skip_serializing_if = "ist_null")]
    pub beginn: SimZeit,
    #[serde(default, skip_serializing_if = "ist_null")]
    pub bereit: SimZeit,
}
fn ist_null(n: &SimZeit) -> bool { *n == 0 }

/// Bombenschächte laut Steckbrief (flottenausbau.json „motiv“): Bomber 2, Strategischer Bomber 3,
/// jede weitere Einheit der Klasse „bomber“ 1.
pub fn bombenschaechte(e: Einheit, klasse: Option<&str>) -> i64 {
    match e {
        Einheit::Bomber => 2,
        Einheit::StrategischerBomber => 3,
        _ if klasse == Some("bomber") => 1,
        _ => 0,
    }
}

/// Verbündeter Truppenkonvoi aus reinen Truppentransportern: seine Bodentruppen gelten als gelandet,
/// die Transporter sind keine Orbitverteidigung.
pub fn nur_truppenkonvoi(schiffe: &[i64; SCHIFFE]) -> bool {
    schiffe[Einheit::Truppentransporter.idx()] > 0
        && (0..SCHIFFE).all(|i| i == Einheit::Truppentransporter.idx() || schiffe[i] == 0)
}

impl Welt {
    pub fn invasion_v1(&self) -> bool {
        self.ausbau.boden.invasion.version >= 1 && self.kolonisationsregeln_v2()
    }
    /// Neue Welten (Server, lokale Online-Partie, Botläufe). Laufende Welten bleiben unverändert.
    pub fn invasion_aktivieren(&mut self) -> Result<(), String> {
        if self.ausbau.industrie_version == 0 || !self.kolonisationsregeln_v2() {
            return Err("Invasion v1 benötigt Kolonisationsregeln v2 und Bodentruppen (Generation 12)".into());
        }
        self.ausbau.boden.invasion.version = 1;
        Ok(())
    }
    /// Takt der Bomben- und Bodenrunden (Kernwerte; Standard `self.iv_runde()`).
    pub fn iv_runde(&self) -> SimZeit { self.regeln.kw().boden.runde() }
    /// Belagerung bis zur frühesten Übernahme (Kernwerte; Standard `self.iv_belagerung()`).
    pub fn iv_belagerung(&self) -> SimZeit { self.regeln.kw().boden.belagerung() }
    /// Untergrenze der Bombardierung in Promille (Kernwerte; Standard `self.iv_integritaet_grenze()`).
    pub fn iv_integritaet_grenze(&self) -> i64 { self.regeln.kw().boden.integritaet_grenze_promille }
    pub(crate) fn iv(&self) -> &crate::kernwerte::BodenWerte { &self.regeln.kw().boden }
    pub fn bombenlast(&self, e: Einheit) -> i64 {
        let klasse = self.regeln.flottendoktrin.get(&e).map(|d| d.klasse.as_str());
        bombenschaechte(e, klasse) * self.iv().bombenlast_je_schacht
    }
    pub fn bombenlast_flotte(&self, schiffe: &[i64; SCHIFFE]) -> i64 {
        (0..SCHIFFE).map(|i| schiffe[i].max(0) * self.bombenlast(Einheit::ALLE[i])).sum()
    }
    /// Strukturpunkte aller Gebäude: 700 je Gebäudestufe.
    pub fn gebaeudestruktur(&self, pid: usize) -> i64 {
        self.planeten[pid].gebaeude.iter().map(|n| *n as i64).sum::<i64>() * self.iv().bombardement_struktur_je_stufe
    }
    /// Integritätsverlust je Runde in Promille: ⌈1000 × Bombenlast / Strukturpunkte⌉.
    pub fn bombenschaden_promille(&self, pid: usize, last: i64) -> i64 {
        let s = self.gebaeudestruktur(pid);
        if last <= 0 || s <= 0 { return 0; }
        (1000 * last + s - 1) / s
    }
    fn hoechste_integritaet(&self, pid: usize) -> i64 {
        Gebaeude::ALLE.iter().filter(|g| self.planeten[pid].gebaeude[g.idx()] > 0)
            .map(|g| self.integritaet(pid, *g)).max().unwrap_or(0)
    }
    #[allow(dead_code)]
    fn bombardement_fertig(&self, pid: usize) -> bool {
        self.hoechste_integritaet(pid) <= self.iv_integritaet_grenze()
    }
    pub(crate) fn orbit_unbewaffnet(&self, pid: usize, fid: FlottenId) -> bool {
        !self.bewaffnet(&self.planeten[pid].einheiten) && self.planet_verteidiger_v2(pid, fid).len() == 1
    }
    fn bombardement_gueltig(&self, pid: usize, b: &Bombardement) -> bool {
        let p = &self.planeten[pid];
        self.flotten.get(&b.flotte).is_some_and(|f| {
            p.blockade == Some(b.flotte)
                && f.zustand == Flottenzustand::ImOrbit
                && matches!(f.mission, Mission::Bombardieren | Mission::Kampfkolonisieren)
                && f.besitzer != p.besitzer
                && !self.verbuendet(f.besitzer, p.besitzer)
        }) && p.besitzer == b.verteidiger
    }
    fn invasionsbericht(&mut self, bericht: Value) {
        let l = &mut self.ausbau.boden.invasion.berichte;
        l.push(bericht);
        if l.len() > BERICHTE_MAX { l.remove(0); }
    }

    /// Ersetzt in Invasion v1 das sofortige Absenken auf 30 %: der Orbit ist gewonnen, die Bomber
    /// werfen die erste Runde sofort, danach alle 15 Spielminuten.
    pub(crate) fn bombardement_beginnen(&mut self, pid: usize, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else { return };
        let id = pid as PlanetId;
        if !self.orbit_unbewaffnet(pid, fid) { return; }
        if self.ausbau.boden.invasion.bombardements.get(&id).is_some_and(|b| b.flotte == fid) {
            self.besetzung_beginnen(pid, fid);
            return;
        }
        let verteidiger = self.planeten[pid].besitzer;
        self.ausbau.boden.invasion.bombardements.insert(id, Bombardement {
            flotte: fid, angreifer: f.besitzer, verteidiger, seit: self.zeit, letzte_runde: self.zeit, runden: 0, unterbrochen: 0, abgeschlossen: false,
        });
        let ziel = self.planeten[pid].koord;
        let last = self.bombenlast_flotte(&f.schiffe);
        let text = if last > 0 {
            format!("{ziel}: Orbit gewonnen, Bombardierung beginnt ({} Bombenlast je Runde, {}‰ Integritätsverlust je {} Spielminuten)", last, self.bombenschaden_promille(pid, last), self.iv().runde_minuten)
        } else {
            format!("{ziel}: Orbit gewonnen, aber keine Bomber in der Flotte – Bombardierung wirkungslos")
        };
        for sid in [verteidiger, f.besitzer] { self.vorfall(sid, "bombardement", text.clone()); }
        self.fachereignis("bombardment_started", f.besitzer, json!({"planet":pid,"fleet":fid,"bomb_load":last}));
        self.bombenrunde(pid, fid, self.zeit);
    }

    /// Eine Bombenrunde: alle Gebäude mit Integrität über 30 % verlieren denselben Promillewert.
    fn bombenrunde(&mut self, pid: usize, fid: FlottenId, t: SimZeit) {
        let id = pid as PlanetId;
        let Some(mut b) = self.ausbau.boden.invasion.bombardements.get(&id).cloned() else { return };
        let schiffe = self.flotten[&fid].schiffe;
        let last = self.bombenlast_flotte(&schiffe);
        let schaden = self.bombenschaden_promille(pid, last);
        let vorher = self.hoechste_integritaet(pid);
        b.letzte_runde = t;
        if schaden > 0 && vorher > self.iv_integritaet_grenze() {
            self.abrechnen(pid);
            for g in Gebaeude::ALLE {
                if self.planeten[pid].gebaeude[g.idx()] == 0 { continue; }
                let alt = self.integritaet(pid, g);
                if alt > self.iv_integritaet_grenze() {
                    self.kolonisation.integritaet.insert((id, g), (alt - schaden).max(self.iv_integritaet_grenze()) as u16);
                }
            }
            if b.runden == 0 {
                // Bereits bezahlte Aufträge bleiben storniert (wie bisher beim Bombardement).
                self.planeten[pid].bauschleife.clear();
                self.ereignisse.retain(|e| !matches!(e.art, EreignisArt::BauFertig{planet} if planet == id));
            }
            self.kolonisation.reparaturen.retain(|(p, _), _| *p != id);
            b.runden += 1;
            self.raten_neu(pid);
        }
        let nachher = self.hoechste_integritaet(pid);
        let bomber: BTreeMap<&str, i64> = (0..SCHIFFE).filter(|i| schiffe[*i] > 0 && self.bombenlast(Einheit::ALLE[*i]) > 0)
            .map(|i| (Einheit::ALLE[i].name(), schiffe[i])).collect();
        let fertig = nachher <= self.iv_integritaet_grenze();
        if schaden > 0 && vorher > self.iv_integritaet_grenze() {
            self.invasionsbericht(json!({"art":"bombenrunde","zeit":t,"planet":self.planeten[pid].koord.to_string(),"flotte":fid,
                "angreifer":b.angreifer,"verteidiger":b.verteidiger,"bomber":bomber,"bombenlast":last,
                "strukturpunkte":self.gebaeudestruktur(pid),"schaden_promille":schaden,
                "integritaet_vorher_prozent":vorher as f64/10.0,"integritaet_nachher_prozent":nachher as f64/10.0,"runde":b.runden,"abgeschlossen":fertig}));
        }
        let neu_fertig = fertig && !b.abgeschlossen;
        b.abgeschlossen = fertig;
        self.ausbau.boden.invasion.bombardements.insert(id, b.clone());
        if neu_fertig {
            let ziel = self.planeten[pid].koord;
            let kk = self.flotten.get(&fid).is_some_and(|f| f.mission == Mission::Kampfkolonisieren);
            let text = format!("{ziel}: Bombardierung abgeschlossen nach {} Runden – alle Gebäude bei höchstens {} % Integrität{}", b.runden, self.iv_integritaet_grenze() / 10,
                // Invasion v2: Bomber mit Bodentruppen landen selbst (boden.rs bomber_landung) – der alte Satz widersprach dem.
                if kk { "; Bodentruppen können landen" } else if self.invasion_v2() { "; mitgeführte Bodentruppen landen jetzt, das Kolonieschiff braucht es erst für die Übernahme" } else { "; Landung nur mit kampfkolonisieren (Kolonieschiff, Truppentransporter mit Bodentruppen)" });
            for sid in [b.verteidiger, b.angreifer] {
                self.vorfall(sid, "bombardement", text.clone());
                self.wecke(sid, Rolle::Feldherr, "Bombardierung abgeschlossen", true);
            }
            self.fachereignis("bombardment_completed", b.angreifer, json!({"planet":pid,"fleet":fid,"rounds":b.runden}));
            self.punkte_neu();
        }
        if fertig { self.besetzung_beginnen(pid, fid); }
    }

    /// Fenstertakt: fällige Bombenrunden nachholen; neue Verteidigung unterbricht und wird bekämpft.
    pub(crate) fn bombardement_runden(&mut self) {
        for (id, b) in self.ausbau.boden.invasion.bombardements.clone() {
            let pid = id as usize;
            if !self.bombardement_gueltig(pid, &b) {
                self.ausbau.boden.invasion.bombardements.remove(&id);
                if !b.abgeschlossen {
                    let text = format!("{}: Bombardierung beendet – Orbit, Besitzer oder Beziehung verändert", self.planeten[pid].koord);
                    for sid in [b.verteidiger, b.angreifer] { self.vorfall(sid, "bombardement", text.clone()); }
                }
                continue;
            }
            let mut letzte = b.letzte_runde;
            while letzte + self.iv_runde() <= self.zeit {
                letzte += self.iv_runde();
                if !self.orbit_unbewaffnet(pid, b.flotte) {
                    if let Some(x) = self.ausbau.boden.invasion.bombardements.get_mut(&id) { x.unterbrochen += 1; x.letzte_runde = letzte; }
                    let text = format!("{}: neue Verteidigung unterbricht die Bombardierung", self.planeten[pid].koord);
                    for sid in [b.verteidiger, b.angreifer] { self.vorfall(sid, "bombardement", text.clone()); }
                    if !self.besetzung_verteidigung_bekaempfen(pid, b.flotte) {
                        self.ausbau.boden.invasion.bombardements.remove(&id);
                        if self.kolonisation.besetzungen.contains_key(&id) {
                            self.besetzung_abbrechen(id, "Bewaffnete Verteidigung nicht ausgeschaltet");
                        }
                        break;
                    }
                    continue;
                }
                self.bombenrunde(pid, b.flotte, letzte);
                if !self.ausbau.boden.invasion.bombardements.contains_key(&id) { break; }
            }
        }
    }

    /// Bodentruppen der Besatzungsflotte, die angreifen können.
    pub fn landetruppen(&self, fid: FlottenId) -> i64 {
        let a = self.ausbau.boden.flotten.get(&fid).copied().unwrap_or_default();
        let platz = self.flotten.get(&fid).map(|f| f.schiffe[Einheit::Truppentransporter.idx()] * self.iv().transporter_plaetze).unwrap_or(0);
        if platz == 0 { return 0; }
        (0..ANZAHL).filter(|i| self.iv().angriff(*i) > 0).map(|i| a[i]).sum()
    }
    /// Verbündete Flotten im Orbit (Mission halten), die Bodentruppen für diesen Planeten tragen – in neuen Welten auch
    /// gestrandete Flotten, deren Truppen den Besitzer automatisch unterstützen (0.4.8).
    pub fn hilfsflotten(&self, pid: usize) -> Vec<FlottenId> {
        let p = &self.planeten[pid];
        let angreifer = self.kolonisation.besetzungsangreifer.get(&(pid as PlanetId)).copied();
        self.flotten.values().filter(|f| {
            (self.orbitflotte_hilft(f, pid, angreifer) && self.ausbau.boden.flotten.get(&f.id).is_some_and(|a| a.iter().sum::<i64>() > 0)) ||
            f.zustand == Flottenzustand::ImOrbit && f.mission == Mission::Halten && f.ziel == p.koord
                && !self.monde.flottenziel.contains_key(&f.id)
                && (f.besitzer == p.besitzer || self.verbuendet(f.besitzer, p.besitzer))
                && self.ausbau.boden.flotten.get(&f.id).is_some_and(|a| a.iter().sum::<i64>() > 0)
        }).map(|f| f.id).collect()
    }
    /// Kampfstärke der ausgerufenen Miliz: alle Bewohner (ganze Personen) einschließlich Dienst – in neuen Welten nur der
    /// Anteil je Volk (`miliz_anteil_promille`).
    pub fn miliz_staerke(&self, pid: usize) -> i64 {
        let id = pid as PlanetId;
        match self.ausbau.boden.invasion.milizen.get(&id) {
            Some(a) if !a.kapituliert && a.besitzer == self.planeten[pid].besitzer => {
                let voll = self.miliz_potenzial(pid);
                if a.bereit > self.zeit && a.bereit > a.beginn {
                    voll * (self.zeit - a.beginn).max(0) / (a.bereit - a.beginn)
                } else { voll }
            }
            _ => 0,
        }
    }
    /// Bewohner, die als Miliz kämpfen würden (einschließlich der schon Dienenden).
    pub fn miliz_potenzial(&self, pid: usize) -> i64 {
        let id = pid as PlanetId;
        let dienst = self.ausbau.boden.miliz.get(&id).map(|m| m.personen.gesamt()).unwrap_or(0);
        let anteile = &self.iv().miliz_anteil_promille;
        if anteile.is_empty() { return (self.planeten[pid].bevoelkerung + dienst) / M; }
        let mut je_volk = [0i64; VOELKER];
        if let Some(p) = self.ausbau.personen.get(&id) { for v in 0..VOELKER { je_volk[v] += p.gruppen[v].iter().sum::<i64>(); } }
        if let Some(m) = self.ausbau.boden.miliz.get(&id) { for v in 0..VOELKER { je_volk[v] += m.personen.gruppen[v].iter().sum::<i64>(); } }
        (0..VOELKER).map(|v| je_volk[v] / M * self.miliz_anteil(Volk::ALLE[v]) / 1000).sum()
    }
    /// Kampfanteil eines Volkes in der Miliz (Promille).
    pub fn miliz_anteil(&self, volk: Volk) -> i64 {
        self.iv().miliz_anteil_promille.get(volk.name()).copied().unwrap_or(1000).clamp(0, 1000)
    }
    /// Alle Bodenverteidiger: Garnison, verbündete Bodentruppen und ausgerufene Miliz.
    pub fn bodenverteidiger(&self, pid: usize) -> i64 {
        let g = self.ausbau.boden.bestand.get(&(pid as PlanetId)).map(|a| a.iter().sum::<i64>()).unwrap_or(0);
        let h: i64 = self.hilfsflotten(pid).iter().map(|f| self.ausbau.boden.flotten[f].iter().sum::<i64>()).sum();
        g + h + self.miliz_staerke(pid)
    }

    /// Belagerung je Fenster: neue Verteidigung bekämpfen, fällige Bodenrunden nachholen,
    /// nach 48 Spielstunden übernehmen, wenn niemand mehr am Boden verteidigt.
    pub(crate) fn belagerung_schritt(&mut self, id: PlanetId, b: Besetzung) {
        let pid = id as usize;
        if !self.orbit_unbewaffnet(pid, b.flotte) {
            if !self.besetzung_verteidigung_bekaempfen(pid, b.flotte) {
                self.besetzung_abbrechen(id, "Bewaffnete Verteidigung nicht ausgeschaltet");
                return;
            }
            if !self.kolonisation.besetzungen.contains_key(&id) { return; }
        }
        loop {
            let last = self.ausbau.boden.letzte_runde.get(&b.flotte).copied().unwrap_or(b.seit);
            if self.zeit - last < self.iv_runde() { break; }
            if !self.bodenrunde_v1(pid, b.flotte, last + self.iv_runde()) { return; }
        }
        if self.zeit >= b.fruehestens && self.bodenverteidiger(pid) == 0 && self.landetruppen(b.flotte) > 0 {
            self.belagerung_beenden(pid);
            self.miliz_entlassen(pid);
            self.kolonisation.besetzungen.remove(&id);
            self.kolonisation.besetzungsmarken.remove(&id);
            self.kolonisation.besetzungsangreifer.remove(&id);
            self.ausbau.boden.invasion.bombardements.remove(&id);
            self.erobern(pid, b.flotte);
        }
    }

    /// Eine Bodenrunde mit gleichzeitigem Feuer. false = Belagerung beendet.
    fn bodenrunde_v1(&mut self, pid: usize, fid: FlottenId, t: SimZeit) -> bool {
        use crate::boden_kampf::{aufgebot_bw, feuern_bw, name, struktur_bw, unterstuetzung_bw};
        let rg = self.regeln.clone();
        let bw = &rg.kw().boden;
        let aufgebot = aufgebot_bw(bw);
        let id = pid as PlanetId;
        self.boden_kapazitaet(fid);
        let a0 = self.ausbau.boden.flotten.get(&fid).copied().unwrap_or_default();
        let garnison = self.ausbau.boden.bestand.get(&id).copied().unwrap_or_default();
        let hilfe: Vec<(FlottenId, [i64; ANZAHL])> = self.hilfsflotten(pid).into_iter().map(|f| (f, self.ausbau.boden.flotten[&f])).collect();
        let miliz = self.miliz_staerke(pid);
        let a: [i64; KAMPFZAHL] = std::array::from_fn(|i| if i < ANZAHL { a0[i] } else { 0 });
        let d: [i64; KAMPFZAHL] = std::array::from_fn(|i| if i < ANZAHL { garnison[i] + hilfe.iter().map(|(_, h)| h[i]).sum::<i64>() } else { miliz });
        self.ausbau.boden.letzte_runde.insert(fid, t);
        if d.iter().sum::<i64>() == 0 { return true; }
        if a.iter().sum::<i64>() == 0 {
            self.besetzung_abbrechen(id, "Angreifer ohne einsatzfähige Bodentruppen; die Bodenverteidigung hält");
            return false;
        }
        let mut wounds = self.ausbau.boden.wunden.get(&fid).copied().unwrap_or_default();
        let vorher_wunden = wounds;
        let asupport = unterstuetzung_bw(bw, &a, &mut wounds[0]);
        let dsupport = unterstuetzung_bw(bw, &d, &mut wounds[1]);
        let (mut dl, af, dw) = feuern_bw(bw, &a, &d, wounds[1], aufgebot);
        // Kapitulation mitten in der Runde: höchstens bis zur 30-%-Grenze fallen Milizionäre.
        if let Some(m) = self.ausbau.boden.invasion.milizen.get(&id).filter(|m| !m.kapituliert) {
            let grenze = (m.aufgebot * self.iv().miliz_kapitulation_promille + 999) / 1000 - m.gefallen;
            dl[ANZAHL] = dl[ANZAHL].min(grenze.max(0));
        }
        let (al, df, aw) = feuern_bw(bw, &d, &a, wounds[0], aufgebot);
        let an: [i64; ANZAHL] = std::array::from_fn(|i| a[i] - al[i]);
        self.ausbau.boden.flotten.insert(fid, an);
        // Verluste anteilig auf Garnison und Hilfsflotten; Rest der Reihe nach (Garnison zuerst).
        let mut quellen: Vec<[i64; ANZAHL]> = std::iter::once(garnison).chain(hilfe.iter().map(|(_, h)| *h)).collect();
        for i in 0..ANZAHL {
            if dl[i] == 0 || d[i] == 0 { continue; }
            let mut rest = dl[i];
            let anteile: Vec<i64> = quellen.iter().map(|q| dl[i] * q[i] / d[i]).collect();
            for (q, n) in quellen.iter_mut().zip(&anteile) { q[i] -= n; rest -= n; }
            for q in quellen.iter_mut() { let n = rest.min(q[i]); q[i] -= n; rest -= n; }
        }
        self.ausbau.boden.bestand.insert(id, quellen[0]);
        for (k, (f, _)) in hilfe.iter().enumerate() { self.ausbau.boden.flotten.insert(*f, quellen[k + 1]); }
        let mut dw = dw;
        if dl[ANZAHL] > 0 { self.miliz_verluste_v1(pid, dl[ANZAHL]); }
        if self.miliz_staerke(pid) == 0 { dw[ANZAHL] = 0; }
        self.ausbau.boden.wunden.insert(fid, [aw, dw]);
        let angreifer = self.flotten[&fid].besitzer;
        let verteidiger = self.planeten[pid].besitzer;
        let quellen_json: Vec<Value> = std::iter::once(json!({"quelle":"garnison","spieler":verteidiger,"vorher":garnison,"nachher":quellen[0]}))
            .chain(hilfe.iter().enumerate().map(|(k, (f, h))| json!({"quelle":"verbuendete","flotte":f,"spieler":self.flotten[f].besitzer,"name":self.spieler[self.flotten[f].besitzer as usize].name,"vorher":h,"nachher":quellen[k + 1]})))
            .chain((miliz > 0).then(|| json!({"quelle":"miliz","spieler":verteidiger,"vorher":miliz,"nachher":self.miliz_staerke(pid)})))
            .collect();
        self.ausbau.boden.berichte.push(json!({"zeit":t,"regeln":"invasion-v1","planet":self.planeten[pid].koord.to_string(),"angreifer":angreifer,"verteidiger":verteidiger,
            "typen":(0..KAMPFZAHL).map(name).collect::<Vec<_>>(),"angreifer_name":self.spieler[angreifer as usize].name,"verteidiger_name":self.spieler[verteidiger as usize].name,
            "struktur_je_einheit":(0..KAMPFZAHL).map(|i| struktur_bw(bw, i, aufgebot)).collect::<Vec<_>>(),"wunden_vorher":vorher_wunden,"wunden_nachher":[aw,dw],
            "angreifer_vorher":a,"verteidiger_vorher":d,"angreifer_feuer":af,"verteidiger_feuer":df,"angreifer_unterstuetzung":asupport,"verteidiger_unterstuetzung":dsupport,
            "angreifer_verluste":al,"verteidiger_verluste":dl,"angreifer_nachher":(0..KAMPFZAHL).map(|i| a[i]-al[i]).collect::<Vec<_>>(),
            "verteidiger_nachher":(0..KAMPFZAHL).map(|i| d[i]-dl[i]).collect::<Vec<_>>(),"verteidiger_quellen":quellen_json}));
        self.bodenberichte_kappen();
        let rest = self.bodenverteidiger(pid);
        if an.iter().sum::<i64>() == 0 && rest > 0 {
            self.besetzung_abbrechen(id, "Bodenverteidigung hat die Invasion zurückgeschlagen");
            return false;
        }
        true
    }

    pub(crate) fn miliz_verluste_v1(&mut self, pid: usize, n: i64) {
        let id = pid as PlanetId;
        let mut rest = n * M;
        if let Some(m) = self.ausbau.boden.miliz.get_mut(&id) {
            let tot = m.personen.auswahl(rest);
            rest -= tot.gesamt();
            m.personen.nehmen(&tot).expect("Milizverluste begrenzt");
        }
        if rest > 0 {
            self.personen_abgleichen(pid);
            if let Some(p) = self.ausbau.personen.get_mut(&id) {
                let tot = p.auswahl(rest);
                p.nehmen(&tot).expect("Milizverluste begrenzt");
                self.planeten[pid].bevoelkerung = (self.planeten[pid].bevoelkerung - tot.gesamt()).max(0);
            }
        }
        let koord = self.planeten[pid].koord;
        let kapitulation = self.iv().miliz_kapitulation_promille;
        let Some(a) = self.ausbau.boden.invasion.milizen.get_mut(&id) else { return };
        a.gefallen += n;
        if a.gefallen * 1000 >= a.aufgebot * kapitulation && !a.kapituliert {
            a.kapituliert = true;
            let (besitzer, gefallen, aufgebot) = (a.besitzer, a.gefallen, a.aufgebot);
            self.miliz_entlassen(pid);
            self.raten_neu(pid);
            let text = format!("{koord}: Miliz kapituliert nach {gefallen} Gefallenen von {aufgebot}; die Überlebenden bleiben Bevölkerung");
            self.vorfall(besitzer, "miliz", text.clone());
            if let Some(sid) = self.kolonisation.besetzungsangreifer.get(&id).copied() { self.vorfall(sid, "miliz", text); }
            self.fachereignis("militia_capitulated", besitzer, json!({"planet":pid,"fallen":gefallen,"levy":aufgebot}));
        }
    }

    /// Ende einer Belagerung (Übernahme oder Abbruch): verbündete Bodentruppen kehren heim.
    pub(crate) fn belagerung_beenden(&mut self, pid: usize) {
        if !self.invasion_v1() { return; }
        for f in self.hilfsflotten(pid) {
            let sid = self.flotten[&f].besitzer;
            self.vorfall(sid, "flotte", format!("Belagerung von {} beendet: Bodentruppen der Flotte {f} kehren heim", self.planeten[pid].koord));
            self.heimflug(f);
        }
        self.ausbau.boden.invasion.milizen.remove(&(pid as PlanetId));
    }

    /// Verbündete Haltetruppen bleiben bis zum Ende der Belagerung.
    pub(crate) fn invasion_orbit_verlaengern(&mut self, fid: FlottenId) -> bool {
        if !self.invasion_v1() { return false; }
        let Some(f) = self.flotten.get(&fid) else { return false };
        let Some(pid) = self.belegung.get(&f.ziel).map(|p| *p as usize) else { return false };
        if f.mission != Mission::Halten || !self.kolonisation.besetzungen.contains_key(&(pid as PlanetId))
            || !(self.hilfsflotten(pid).contains(&fid) || self.angriffshelfer(pid).contains(&fid)) {
            return false;
        }
        let ende = self.zeit + self.iv_runde();
        self.flotten.get_mut(&fid).unwrap().orbit_ende = ende;
        self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
        true
    }

    /// Ankunft unter Blockade: Bodentruppen des Besitzers landen während der Belagerung trotzdem
    /// (die Schiffe kehren danach wie gewohnt um); verbündete Halteflotten kämpfen bewaffnet gegen die
    /// Besatzungsflotte und gehen mit Bodentruppen in den Orbit. true = Ankunft vollständig erledigt.
    pub(crate) fn invasion_ankunft(&mut self, fid: FlottenId, ziel_pid: Option<usize>) -> bool {
        if !self.invasion_v1() { return false; }
        let (Some(zp), Some(f)) = (ziel_pid, self.flotten.get(&fid).cloned()) else { return false };
        let id = zp as PlanetId;
        let owner = self.planeten[zp].besitzer;
        let belagert = self.kolonisation.besetzungen.contains_key(&id);
        if !self.blockiert_fuer(zp, f.besitzer) { return false; }
        let truppen = self.ausbau.boden.flotten.get(&fid).is_some_and(|a| a.iter().sum::<i64>() > 0);
        match f.mission {
            Mission::Transport | Mission::Stationieren if f.besitzer == owner && belagert && truppen => {
                self.boden_kapazitaet(fid);
                let n: i64 = self.ausbau.boden.flotten.get(&fid).map(|a| a.iter().sum()).unwrap_or(0);
                self.boden_landen(fid, zp);
                self.vorfall(owner, "bodentruppen", format!("{}: {n} Bodeneinheiten landen trotz Blockade und verstärken die Garnison", f.ziel));
                self.fachereignis("ground_reinforcement_landed", owner, json!({"planet":zp,"fleet":fid,"units":n}));
                false
            }
            Mission::Halten if self.verbuendet(f.besitzer, owner) => {
                if let Some(bf) = self.planeten[zp].blockade.filter(|b| self.flotten.contains_key(b)) {
                    let gegner = self.flotten[&bf].besitzer;
                    // Reine Truppenkonvois (nur Truppentransporter) landen ohne Gefecht; Kampfschiffe greifen an.
                    if !nur_truppenkonvoi(&f.schiffe) && self.bewaffnet(&f.schiffe) && self.gegner_angreifbar(f.besitzer, gegner).is_ok() {
                        self.bruch_pruefen(f.besitzer, gegner);
                        if !self.spieler[f.besitzer as usize].angegriffen.contains(&gegner) { self.spieler[f.besitzer as usize].angegriffen.push(gegner); }
                        self.gefecht(&[fid], &[Partei::Flotte(bf)], f.ziel);
                        if !self.flotten.contains_key(&bf) {
                            self.planeten[zp].blockade = None;
                            self.fachereignis("blockade_broken", owner, json!({"planet":zp,"fleet":bf,"by":fid}));
                        }
                    }
                }
                if !self.flotten.contains_key(&fid) { return true; }
                let truppen = self.ausbau.boden.flotten.get(&fid).is_some_and(|a| a.iter().sum::<i64>() > 0);
                if self.blockiert_fuer(zp, f.besitzer) && !(belagert && truppen) {
                    self.heimflug(fid);
                    return true;
                }
                let jetzt = self.zeit;
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::ImOrbit;
                fl.orbit_ende = jetzt + fl.haltedauer;
                let ende = fl.orbit_ende;
                self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                let name = self.spieler[f.besitzer as usize].name.clone();
                self.vorfall(owner, "flotte", format!("Verbündete Flotte von {name} verstärkt {} während der Belagerung", f.ziel));
                self.vorfall(f.besitzer, "flotte", format!("Flotte {fid} verstärkt den belagerten Planeten {}", f.ziel));
                true
            }
            _ => false,
        }
    }

    // ------------------------------------------------------------------ Miliz v1

    pub fn miliz_freigeschaltet(&self, pid: usize) -> bool {
        self.planeten[pid].gebaeude[Gebaeude::Kaserne.idx()] >= MILIZ_KASERNE
    }
    /// Ruft die Volksmiliz aus: alle Bewohner erhalten Waffen- und Rüstwert, die Hälfte steht jeweils
    /// im Dienst und fehlt in Produktion und Forschung.
    pub fn miliz_ausrufen(&mut self, sid: SpielerId, rolle: Rolle, k: Koord) -> Result<String, String> {
        if !self.invasion_v1() { return Err("miliz_ausrufen gibt es in Welten mit Invasionsregeln v1; sonst miliz_aufrufen".into()); }
        let pid = self.eigener_planet(sid, k)?;
        let id = pid as PlanetId;
        if !self.miliz_freigeschaltet(pid) {
            return Err(format!("Miliz nicht freigeschaltet: Kaserne Stufe {MILIZ_KASERNE} auf diesem Planeten nötig"));
        }
        if let Some((f, n)) = self.iv().miliz_forschung.iter().find(|(f, n)| self.spieler[sid as usize].forschung[f.idx()] < **n) {
            return Err(format!("Miliz noch nicht erforscht: {} Stufe {n} nötig", crate::regeltext::forschung_name(*f)));
        }
        if !self.iv().miliz_jederzeit && !self.kolonisation.besetzungen.get(&id).is_some_and(|b| b.verteidiger == sid) {
            return Err("Miliz kann nur während einer laufenden Belagerung ausgerufen werden".into());
        }
        if let Some(a) = self.ausbau.boden.invasion.milizen.get(&id) {
            return Err(if a.kapituliert { "Die Miliz hat in dieser Belagerung bereits kapituliert".into() } else { "Miliz ist bereits ausgerufen".into() });
        }
        self.abrechnen(pid);
        self.personen_abgleichen(pid);
        let staerke = self.miliz_potenzial(pid);
        if staerke < 1 { return Err("Keine Bewohner auf dem Planeten, die kämpfen können".into()); }
        let mut kosten = [0; GUETER];
        kosten[Gut::Legierung.idx()] = self.iv().miliz_legierung_je_bewohner * staerke * M;
        let topf = Self::topf_fuer(rolle, Topf::Militaer);
        self.zahlbar(pid, &kosten, topf)?;
        self.zahlen(pid, &kosten, topf);
        let leute = self.ausbau.personen.get(&id).cloned().unwrap_or_default();
        let mut dienst = crate::ausbau::Personen::default();
        for v in 0..VOELKER { let anteil = self.miliz_anteil(Volk::ALLE[v]); for q in 0..3 { dienst.gruppen[v][q] = leute.gruppen[v][q] / M * anteil / 1000 * self.iv().miliz_dienst_promille / 1000 * M; } }
        self.ausbau.personen.get_mut(&id).map(|p| p.nehmen(&dienst)).transpose()?;
        self.planeten[pid].bevoelkerung -= dienst.gesamt();
        self.ausbau.boden.miliz.entry(id).or_insert(crate::boden_katalog::Miliz { besitzer: sid, personen: Default::default() }).personen.add(&dienst);
        let ausbildung = (staerke * self.iv().miliz_ausbildung_sekunden_je_1000 + 999) / 1000;
        let (beginn, bereit) = if ausbildung > 0 { (self.zeit, self.zeit + ausbildung) } else { (0, 0) };
        self.ausbau.boden.invasion.milizen.insert(id, Aufgebot { besitzer: sid, ausgerufen: self.zeit, aufgebot: staerke, gefallen: 0, kapituliert: false, beginn, bereit });
        self.raten_neu(pid);
        if let Some(a) = self.kolonisation.besetzungsangreifer.get(&id).copied() {
            self.vorfall(a, "miliz", format!("{k}: Der Verteidiger ruft die Volksmiliz aus ({staerke} bewaffnete Bewohner)"));
        }
        self.fachereignis("militia_called", sid, json!({"planet":pid,"strength":staerke}));
        Ok(format!("Miliz auf {k} ausgerufen: {staerke} Bewohner mit Waffenwert {} und Rüstwert {}; {} im Dienst fehlen in der Produktion; Kapitulation nach {} Gefallenen",
            dezimal(self.iv().miliz_waffen_promille), self.iv().miliz_ruestung, dienst.gesamt() / M, (staerke * self.iv().miliz_kapitulation_promille + 999) / 1000))
    }
    pub fn miliz_sicht_v1(&self, pid: usize) -> Value {
        let p = &self.planeten[pid];
        let id = p.id;
        let belagert = self.kolonisation.besetzungen.get(&id).is_some_and(|b| b.verteidiger == p.besitzer);
        let a = self.ausbau.boden.invasion.milizen.get(&id);
        let staerke = self.miliz_staerke(pid);
        let potenzial = self.miliz_potenzial(pid);
        let jederzeit = self.iv().miliz_jederzeit;
        let anteile: serde_json::Map<String, Value> = Volk::ALLE.iter().map(|v| (v.name().to_string(), json!(self.miliz_anteil(*v) / 10))).collect();
        let frei = self.miliz_freigeschaltet(pid);
        let dienst = self.ausbau.boden.miliz.get(&id).map(|m| m.personen.gesamt() / M).unwrap_or(0);
        json!({"regeln":if self.invasion_v2() {"invasion-v2"} else {"invasion-v1"},"freigeschaltet":frei,"freischaltung":format!("Kaserne Stufe {MILIZ_KASERNE} auf diesem Planeten"),
            "belagert":belagert,"ausgerufen":a.is_some_and(|a|!a.kapituliert),"kapituliert":a.is_some_and(|a|a.kapituliert),
            "aktiv":staerke,"staerke":staerke,"im_dienst":dienst,"aufgebot":a.map(|a|a.aufgebot),"gefallen":a.map(|a|a.gefallen),
            "kapitulation_bei_gefallenen":a.map(|a|(a.aufgebot*self.iv().miliz_kapitulation_promille+999)/1000),
            "verfuegbar":if (belagert||jederzeit)&&frei&&a.is_none(){potenzial}else{0},"jederzeit":jederzeit,"anteil_prozent":anteile,
            "entlassbar":jederzeit&&a.is_some_and(|a|!a.kapituliert),
            "waffenwert":self.iv().miliz_waffen_promille as f64/1000.0,"ruestwert":self.iv().miliz_ruestung,"angriff":self.iv().miliz_waffen_promille as f64/1000.0,"struktur":self.iv().miliz_ruestung,
            "kapitulation_prozent":self.iv().miliz_kapitulation_promille/10,"arbeitsausfall_prozent":self.iv().miliz_dienst_promille/10,
            "kosten":{"legierung_je_bewohner":self.iv().miliz_legierung_je_bewohner,"gesamt_legierung":potenzial*self.iv().miliz_legierung_je_bewohner},
            // 0.4.8: Forschung und Ausbildungszeit aus den Kernwerten (die Oberfläche rechnet nicht mit eigenen Konstanten).
            "forschung":self.iv().miliz_forschung.iter().map(|(f,n)|(serde_json::to_value(f).ok().and_then(|v|v.as_str().map(String::from)).unwrap_or_default(),json!(n))).collect::<serde_json::Map<_,_>>(),
            "ausbildung_sekunden_je_1000":self.iv().miliz_ausbildung_sekunden_je_1000,
            "beschreibung":if jederzeit {format!("Mit {}Kaserne Stufe 1 jederzeit ausrufbar, auch vorbeugend: je nach Volk kämpft ein Anteil der Bewohner (Krath alle, Veyari 90 %, Syntheten 80 %, Aurelianer 70 %) mit Waffenwert {} und Rüstwert {} (Sturminfanterie {}/{}). Ausbildung {} Spielsekunden je 1.000. {} der Kämpfer steht im Dienst und fehlt in Produktion und Forschung. {} Legierung je Milizionär, nicht erstattet. Die Miliz kapituliert nach {} % Gefallenen; Überlebende bleiben Bevölkerung. Außerhalb einer Invasion jederzeit entlassbar.",
                self.iv().miliz_forschung.iter().map(|(f,n)|format!("{} {n} und ",crate::regeltext::forschung_name(*f))).collect::<String>(),
                dezimal(self.iv().miliz_waffen_promille),self.iv().miliz_ruestung,self.iv().angriff(0),self.iv().struktur(0),self.iv().miliz_ausbildung_sekunden_je_1000,
                dienstanteil(self.iv().miliz_dienst_promille),self.iv().miliz_legierung_je_bewohner,self.iv().miliz_kapitulation_promille/10)} else {format!("Nur während einer Belagerung und nach Freischaltung (Kaserne Stufe 1): alle Bewohner erhalten Waffenwert {} und Rüstwert {} (Sturminfanterie {}/{}). {} steht jeweils im Dienst und fehlt in Produktion und Forschung. {} Legierung je Bewohner, nicht erstattet. Die Miliz kapituliert nach {} % Gefallenen; Überlebende bleiben Bevölkerung. Ohne Miliz leisten Bewohner keinen Widerstand.",dezimal(self.iv().miliz_waffen_promille),self.iv().miliz_ruestung,self.iv().angriff(0),self.iv().struktur(0),dienstanteil(self.iv().miliz_dienst_promille),self.iv().miliz_legierung_je_bewohner,self.iv().miliz_kapitulation_promille/10)}})
    }

    // ------------------------------------------------------------------ Sicht

    fn bombardement_sicht(&self, pid: usize) -> Value {
        let Some(b) = self.ausbau.boden.invasion.bombardements.get(&(pid as PlanetId)) else { return Value::Null };
        let last = self.flotten.get(&b.flotte).map(|f| self.bombenlast_flotte(&f.schiffe)).unwrap_or(0);
        let schaden = self.bombenschaden_promille(pid, last);
        let hoch = self.hoechste_integritaet(pid);
        let unbewaffnet = self.orbit_unbewaffnet(pid, b.flotte);
        let status = if hoch <= self.iv_integritaet_grenze() { "abgeschlossen" } else if last == 0 { "ohne_bomber" } else if !unbewaffnet { "unterbrochen" } else { "laeuft" };
        let runden_rest = if schaden > 0 { ((hoch - self.iv_integritaet_grenze()).max(0) + schaden - 1) / schaden } else { -1 };
        json!({"flotte":b.flotte,"angreifer_name":self.spieler[b.angreifer as usize].name,"verteidiger_name":self.spieler[b.verteidiger as usize].name,
            "seit":b.seit,"runden":b.runden,"unterbrochen":b.unterbrochen,"status":status,
            "bombenlast_je_runde":last,"strukturpunkte":self.gebaeudestruktur(pid),"schaden_promille_je_runde":schaden,
            "integritaet_hoechste_prozent":hoch as f64/10.0,"ziel_prozent":self.iv_integritaet_grenze()/10,
            "fortschritt_prozent":({let spanne=(1000-self.iv_integritaet_grenze()).max(1);(1000-hoch).clamp(0,spanne)*100/spanne}),
            "runden_bis_ziel":runden_rest,"naechste_runde":if status=="laeuft"{Some(b.letzte_runde+self.iv_runde())}else{None},
            "restzeit_sekunden":if runden_rest>0{Some((b.letzte_runde+self.iv_runde()-self.zeit).max(0)+(runden_rest-1)*self.iv_runde())}else{None},
            "formel":format!("Integritätsverlust je {} Spielminuten = ⌈1000 × Bombenlast ÷ ({} × Summe aller Gebäudestufen)⌉ Promille; Bomber {}, Strategischer Bomber {} Bombenlast; Untergrenze {} %",self.iv().runde_minuten,self.iv().bombardement_struktur_je_stufe,2*self.iv().bombenlast_je_schacht,3*self.iv().bombenlast_je_schacht,self.iv_integritaet_grenze()/10)})
    }
    fn belagerung_sicht(&self, pid: usize) -> Value {
        let id = pid as PlanetId;
        let Some(b) = self.kolonisation.besetzungen.get(&id) else { return Value::Null };
        let angreifer = self.flotten.get(&b.flotte).map(|f| f.besitzer);
        let hilfe: Vec<Value> = self.hilfsflotten(pid).iter().map(|f| {
            let s = self.flotten[f].besitzer;
            json!({"flotte":f,"name":self.spieler[s as usize].name,"truppen":self.ausbau.boden.flotten[f]})
        }).collect();
        let last = self.ausbau.boden.letzte_runde.get(&b.flotte).copied().unwrap_or(b.seit);
        json!({"flotte":b.flotte,"angreifer_name":angreifer.map(|s|self.spieler[s as usize].name.clone()),"verteidiger_name":self.spieler[b.verteidiger as usize].name,
            "gelandet":b.seit,"ende":b.fruehestens,"dauer_sekunden":self.iv_belagerung(),"restzeit_sekunden":(b.fruehestens-self.zeit).max(0),
            "angreifer_truppen":self.ausbau.boden.flotten.get(&b.flotte).copied().unwrap_or_default(),"truppentypen":NAMEN,
            "verteidiger":{"garnison":self.ausbau.boden.bestand.get(&id).copied().unwrap_or_default(),"verbuendete":hilfe,"miliz":self.miliz_staerke(pid)},
            "verteidiger_gesamt":self.bodenverteidiger(pid),"bodenverteidigung_gebrochen":self.bodenverteidiger(pid)==0,
            "letzte_bodenrunde":last,"naechste_bodenrunde":last+self.iv_runde(),
            "hinweis":if self.invasion_v2(){"Invasion: Wellen brechen die Verteidigungslinien nacheinander; fällt auch der Rückhalt, ist der Planet verwüstet und das Kolonieschiff des Besatzers darf landen. Eigene Bodentruppen landen trotz Blockade im Rückhalt, Verbündete helfen mit Bodenhilfe.".to_string()}else{format!("Übernahme frühestens nach {} Spielstunden ab Landung, nur bei gebrochener Bodenverteidigung und gehaltenem Orbit. Eigene Bodentruppen landen trotz Blockade (transport/stationieren), Verbündete verstärken mit boden_senden Mission verstaerken.",self.iv().belagerung_stunden)}})
    }
    /// Feld `invasion` der Planetensicht (Besitzer).
    pub fn invasion_planet_sicht(&self, pid: usize) -> Value {
        if !self.invasion_v1() { return Value::Null; }
        json!({"regeln":if self.invasion_v2(){"invasion-v2"}else{"invasion-v1"},"bombardement":self.bombardement_sicht(pid),"belagerung":self.belagerung_sicht(pid),"miliz":self.miliz_sicht_v1(pid),
            "wellen":self.wellen_sicht(pid, self.planeten[pid].besitzer),"verwuestet":self.verwuestet(pid)})
    }
    /// Alle Bombardements und Belagerungen, an denen der Spieler als Angreifer, Verteidiger oder
    /// Verbündeter des Verteidigers beteiligt ist.
    pub fn invasion_sicht(&self, sid: SpielerId) -> Value {
        if !self.invasion_v1() { return Value::Null; }
        let mut ids: Vec<PlanetId> = self.ausbau.boden.invasion.bombardements.keys().copied().collect();
        ids.extend(self.kolonisation.besetzungen.keys().copied());
        ids.extend(self.ausbau.boden.invasion.verwuestet.keys().copied());
        ids.sort();
        ids.dedup();
        json!(ids.into_iter().filter_map(|id| {
            let pid = id as usize;
            let owner = self.planeten[pid].besitzer;
            let angreifer = self.ausbau.boden.invasion.bombardements.get(&id).map(|b| b.angreifer)
                .or_else(|| self.kolonisation.besetzungen.get(&id).and_then(|b| self.flotten.get(&b.flotte)).map(|f| f.besitzer));
            let rolle = if owner == sid { "verteidiger" } else if angreifer == Some(sid) { "angreifer" } else if self.verbuendet(sid, owner) { "verbuendeter" }
                else if angreifer.is_some_and(|a| self.verbuendet(sid, a)) { "angriffshelfer" } else { return None };
            // Angreifer und ihre Helfer sehen keine Verteidigerzahlen (Funktionsprüfung 25.10.2026: Garnison und Miliz des
            // Verteidigers standen in ihrer Sicht) – nur, ob die Bodenverteidigung gebrochen ist.
            let mut belagerung = self.belagerung_sicht(pid);
            if matches!(rolle, "angreifer" | "angriffshelfer") { if let Some(o) = belagerung.as_object_mut() { o.remove("verteidiger"); o.remove("verteidiger_gesamt"); } }
            Some(json!({"planet":self.planeten[pid].koord.to_string(),"rolle":rolle,"besitzer":self.spieler[owner as usize].name,
                "regeln":if self.invasion_v2(){"invasion-v2"}else{"invasion-v1"},
                "bombardement":self.bombardement_sicht(pid),"belagerung":belagerung,
                // Rundenberichte der Bombardierung (wurden geschrieben, aber nie gezeigt); Namen statt Spielernummern.
                "bombenrunden":self.ausbau.boden.invasion.berichte.iter().rev()
                    .filter(|r| r["planet"] == self.planeten[pid].koord.to_string().as_str()).take(20)
                    .map(|r| { let mut v = r.clone(); if let Some(o) = v.as_object_mut() {
                        for k in ["angreifer", "verteidiger"] { if let Some(s) = o.remove(k).and_then(|x| x.as_u64()) { o.insert(format!("{k}_name"), json!(self.spieler[s as usize].name)); } }
                    } v }).collect::<Vec<_>>(),
                "miliz":if matches!(rolle,"angreifer"|"angriffshelfer"){json!({"ausgerufen":self.miliz_staerke(pid)>0})}else{self.miliz_sicht_v1(pid)},
                "wellen":self.wellen_sicht(pid, sid),"verwuestet":self.verwuestet(pid)}))
        }).collect::<Vec<_>>())
    }
    /// Feste Regelwerte für Katalog, Doku und Agenten.
    pub fn invasion_katalog() -> Value {
        Self::invasion_katalog_bw(&crate::kernwerte::standard().boden)
    }
    /// Regelwerte mit den Kernwerten einer Welt (`Regelwerk::kw().boden`).
    pub fn invasion_katalog_bw(b: &crate::kernwerte::BodenWerte) -> Value {
        // Neue Welten spielen Invasion v2 (Wellen gegen Linien, keine Belagerungsfrist): Kodex, Technologiebaum und Website lasen
        // bis 26.10.2026 hier noch die v1-Werte („48 Spielstunden Belagerung“, Miliz nur mit Kaserne 1).
        if b.wellen > 0 {
            return json!({"regeln":"invasion-v2","runde_sekunden":b.runde(),"integritaet_grenze_prozent":b.integritaet_grenze_promille/10,
                "strukturpunkte_je_gebaeudestufe":b.bombardement_struktur_je_stufe,"bombenlast":{"bomber":2*b.bombenlast_je_schacht,"strategischer_bomber":3*b.bombenlast_je_schacht,"weitere_klasse_bomber":b.bombenlast_je_schacht},
                "transporter_plaetze":b.transporter_plaetze,"linien":b.wellen,"wellen_abstand_sekunden":b.wellen_abstand_stunden*3600,"wellen_runden_max":b.wellen_runden_max,
                "ausbildung_waehrend_invasion_prozent":b.invasion_ausbildung_promille/10,"reparatur_prozent":b.reparatur_stufe_promille/10,
                "miliz":{"freischaltung":"kaserne 1","forschung":b.miliz_forschung,"jederzeit":b.miliz_jederzeit,
                    "ausbildung_sekunden_je_1000":b.miliz_ausbildung_sekunden_je_1000,"anteil_promille":b.miliz_anteil_promille,
                    "waffenwert":b.miliz_waffen_promille as f64/1000.0,"ruestwert":b.miliz_ruestung,"kapitulation_prozent":b.miliz_kapitulation_promille/10,
                    "arbeitsausfall_prozent":b.miliz_dienst_promille/10,"legierung_je_bewohner":b.miliz_legierung_je_bewohner}});
        }
        json!({"regeln":"invasion-v1","runde_sekunden":b.runde(),"belagerung_sekunden":b.belagerung(),"integritaet_grenze_prozent":b.integritaet_grenze_promille/10,
            "strukturpunkte_je_gebaeudestufe":b.bombardement_struktur_je_stufe,"bombenlast":{"bomber":2*b.bombenlast_je_schacht,"strategischer_bomber":3*b.bombenlast_je_schacht,"weitere_klasse_bomber":b.bombenlast_je_schacht},
            "miliz":{"freischaltung":"kaserne 1","waffenwert":b.miliz_waffen_promille as f64/1000.0,"ruestwert":b.miliz_ruestung,"kapitulation_prozent":b.miliz_kapitulation_promille/10,
                "arbeitsausfall_prozent":b.miliz_dienst_promille/10,"legierung_je_bewohner":b.miliz_legierung_je_bewohner}})
    }
}

/// Promille als deutsche Dezimalzahl (300 → „0,3“).
fn dezimal(promille: i64) -> String {
    format!("{}", promille as f64 / 1000.0).replace('.', ",")
}
/// Dienstanteil der Miliz als Satzanfang (500 ‰ → „Die Hälfte“, sonst „N %“).
fn dienstanteil(promille: i64) -> String {
    if promille == 500 { "Die Hälfte".into() } else { format!("{} % der Bewohner", promille as f64 / 10.0) }
}
