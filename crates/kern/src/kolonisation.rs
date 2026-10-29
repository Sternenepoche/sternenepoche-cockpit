//! Versioned expedition, bombardment and occupation rules. Legacy snapshots keep legacy rules.
use crate::{typen::*, welt::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const REGELTEXT_V2: &str = "Kolonisation: Freie Ziele müssen zuerst durch eine eigene Spionagesonde erkundet sein. Eine freie Kolonie benötigt ein Kolonieschiff, mindestens ein bewaffnetes Begleitschiff und mitgeführte Baustoffe für je eine Erzmine, Kristallmine, Farm und ein Solarkraftwerk sowie einen Tag Nahrung für die organischen Siedler. Startfracht bleibt als Vorrat erhalten. Die feste Schiffsausstattung, das gewählte Siedlervolk und die Klimavoraussetzungen werden im Core geprüft; habitabilitaet und koloniebedarf zeigen die gültigen Werte. Ziel und weiteren Aufbauplan wählt der Spieler. Bombardieren besiegt die bewaffnete Verteidigung einschließlich Planetenschild; danach sinken bestehende Gebäude auf höchstens 30% Integrität. Kampfkolonisieren benötigt zusätzlich ein Kolonieschiff und hält den Orbit. Ausbaustufen und technische Freischaltungen bleiben erhalten; Produktion, Forschung, Wohnraum, Lagerausbau und Bau-/Werftboni werden durch Integrität vermindert. Reparieren kostet den beschädigten Anteil der kumulierten Baukosten und dauert mindestens 15 Spielminuten. Reparatur und normaler Bau teilen eine planetare Baustelle. Transporte sind an den Empfänger bei Abflug gebunden; eigene Orbitflotten werden ausschließlich mit flotte_versorgen versorgt. Einzel- und Verbandsangriffe kämpfen gegen die feindliche Blockadeflotte, verbündete Planeten bleiben unangetastet. Verlorene Rückkehrziele erfordern einen zusätzlichen Flug zur Heimat; feindliche Blockaden verhindern die Landung. Kampfkolonisation beginnt bei höchstens 30% Gebäudeintegrität, benötigt danach fortlaufende Orbitkontrolle, ein Kolonieschiff, mindestens 1800 Sekunden und zwei volle Reaktionsfenster. Zivile Reparaturen oder Sonden setzen die Frist nicht zurück; neue bewaffnete Verteidigung wird vor der Übernahme bekämpft. Sinkender Kolonieschiffbestand bricht die Besetzung ab. Freie und kämpferische Kolonisation teilen die Koloniekapazität. Nur ursprüngliche Heimatplaneten sind dauerhaft vor Übernahme geschützt. Bodenkampf, Garnison, Volksmiliz und Belagerung während der Besetzung regelt der Abschnitt Invasion. Überlebende Bevölkerung bleibt bei Übernahme erhalten. Bereits gekaufte neutrale Marktware bleibt an den Käufer gebunden; bei Besitzerwechsel fliegt sie erst vom erreichten Ziel zur Käuferheimat und wartet dort bei Blockade, ohne zusätzliche Abfangmechanik.";

pub const REGELTEXT:&str = "Kolonisationsregeln v1: Freie Ziele müssen zuerst durch eine eigene Spionagesonde erkundet sein. Eine Kolonie benötigt ein Kolonieschiff, mindestens ein bewaffnetes Begleitschiff und mitgeführte Baustoffe für je eine Erzmine, Kristallmine, Farm und ein Solarkraftwerk sowie einen Tag Nahrung für die Siedler. Diese Fracht wird nicht automatisch verbaut. Ziel und Aufbauplan wählt der Spieler. bombardieren: starke Flotte besiegt alle Verteidiger und alle Planetenschild-Einheiten; danach sinken bestehende Gebäude auf höchstens 30 Prozent Integrität. kampfkolonisieren benötigt zusätzlich ein Kolonieschiff und hält den Orbit. Übernahme erst nach mindestens 30 Spielminuten und zwei abgeschlossenen Reaktionsfenstern des Verteidigers. Rückruf, Verlust des Kolonieschiffs/Orbits oder wiederhergestellte Verteidigung brechen den Vorgang ab. Nur der ursprüngliche Heimatplanet ist dauerhaft vor Übernahme geschützt. Ausbaustufen bleiben erhalten; reparieren kostet den beschädigten Anteil der kumulierten Baukosten und braucht mindestens einen 15-Minuten-Tick. Produktion, Forschung, Wohnraum, Lagerausbau und Bau-/Werftboni sind mit der Integrität vermindert; Stufen und technische Freischaltungen bleiben erhalten. Die frühere Invasion über Stabilität ist in diesem Regelmodus abgeschaltet.";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Kolonisationszustand {
    pub aktiv: bool,
    pub integritaet: BTreeMap<(PlanetId, Gebaeude), u16>,
    pub besetzungen: BTreeMap<PlanetId, Besetzung>,
    pub reparaturen: BTreeMap<(PlanetId, Gebaeude), Reparatur>,
    pub version: u8,
    pub transportziele: BTreeMap<FlottenId, Transportziel>,
    pub rueckwege: BTreeMap<FlottenId, Rueckweg>,
    pub besetzungsmarken: BTreeMap<PlanetId, SimZeit>,
    pub besetzungsangreifer: BTreeMap<PlanetId, SpielerId>,
}
/// Exact extension layout used by STERNEP3 and its historical hashes.
#[derive(Serialize, Deserialize)]
pub(crate) struct KolonisationszustandV1 {
    aktiv: bool,
    integritaet: BTreeMap<(PlanetId, Gebaeude), u16>,
    besetzungen: BTreeMap<PlanetId, Besetzung>,
    reparaturen: BTreeMap<(PlanetId, Gebaeude), Reparatur>,
}
impl From<&Kolonisationszustand> for KolonisationszustandV1 {
    fn from(s: &Kolonisationszustand) -> Self {
        Self {
            aktiv: s.aktiv,
            integritaet: s.integritaet.clone(),
            besetzungen: s.besetzungen.clone(),
            reparaturen: s.reparaturen.clone(),
        }
    }
}
impl From<KolonisationszustandV1> for Kolonisationszustand {
    fn from(s: KolonisationszustandV1) -> Self {
        Self {
            aktiv: s.aktiv,
            integritaet: s.integritaet,
            besetzungen: s.besetzungen,
            reparaturen: s.reparaturen,
            version: 1,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Transportziel {
    Planet {
        planet: PlanetId,
        empfaenger: SpielerId,
    },
    Flotte {
        flotte: FlottenId,
        empfaenger: SpielerId,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rueckweg {
    pub ziel: PlanetId,
    pub wartend: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Besetzung {
    pub flotte: FlottenId,
    pub verteidiger: SpielerId,
    pub seit: SimZeit,
    pub fruehestens: SimZeit,
    pub reaktionsfenster: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Reparatur {
    pub besitzer: SpielerId,
    pub fertig: SimZeit,
}

impl Welt {
    /// Shared by the repair command and its planning quote; never charges resources.
    pub fn reparaturkosten(&self, pid: usize, g: Gebaeude) -> Result<[i64; GUETER], String> {
        let hp = self.integritaet(pid, g);
        let mut kosten = [0i64; GUETER];
        // 0.4.8 (Karl 24.10.2026): „kostet 75 % der Baukosten der aktuellen Stufe des Gebäudes … dann funktioniert es wieder“ –
        // anteilig zum Schaden bis zur Bombengrenze (bei 30 % Integrität genau der volle Anteil).
        let anteil = self.regeln.kw().boden.reparatur_stufe_promille;
        if anteil > 0 {
            let stufe = self.planeten[pid].gebaeude[g.idx()];
            if stufe == 0 { return Ok(kosten); }
            let spanne = (1000 - self.iv_integritaet_grenze()).max(1) as i128;
            let schaden = (1000 - hp).clamp(0, spanne as i64) as i128;
            for (i, wert) in self.regeln.kosten_gebaeude(g, stufe).iter().enumerate() {
                kosten[i] = ((*wert as i128 * anteil as i128 * schaden + 1000 * spanne - 1) / (1000 * spanne)) as i64;
            }
            return Ok(kosten);
        }
        for level in 1..=self.planeten[pid].gebaeude[g.idx()] {
            for (i, value) in self.regeln.kosten_gebaeude(g, level).iter().enumerate() {
                kosten[i] = kosten[i]
                    .checked_add(*value)
                    .ok_or("Reparaturkosten zu groß")?;
            }
        }
        for v in &mut kosten {
            *v = ((*v as i128 * (1000 - hp) as i128 + 999) / 1000) as i64;
        }
        Ok(kosten)
    }
    pub fn kolonisationsregeln_aktivieren(&mut self) {
        self.kolonisation.aktiv = true;
        self.kolonisation.version = 1;
    }
    pub fn kolonisationsregeln_v2_aktivieren(&mut self) {
        self.kolonisation.aktiv = true;
        self.kolonisation.version = 2;
    }
    pub fn kolonisationsregeln_v2(&self) -> bool {
        self.kolonisation.aktiv && self.kolonisation.version == 2
    }
    pub fn fachereignisse_abholen(&mut self) -> Vec<serde_json::Value> {
        std::mem::take(&mut self.fachereignisse)
    }
    pub(crate) fn fachereignis(&mut self, typ: &str, sid: SpielerId, payload: serde_json::Value) {
        if self.kolonisationsregeln_v2() {
            self.fachereignisse.push(serde_json::json!({"event_type":typ,"time":self.zeit,"player_id":sid,"payload":payload}));
        }
    }
    pub(crate) fn besetzung_abbrechen(&mut self, pid: PlanetId, grund: &str) {
        self.ausbau.boden.invasion.wellen.remove(&pid);
        self.ausbau.boden.invasion.verwuestet.remove(&pid);
        if let Some(b) = self.kolonisation.besetzungen.remove(&pid) {
            self.miliz_entlassen(pid as usize);
            self.belagerung_beenden(pid as usize);
            self.kolonisation.besetzungsmarken.remove(&pid);
            let attacker = self
                .kolonisation
                .besetzungsangreifer
                .remove(&pid)
                .or_else(|| self.flotten.get(&b.flotte).map(|f| f.besitzer));
            self.fachereignis("occupation_aborted", b.verteidiger, serde_json::json!({"planet":pid,"fleet":b.flotte,"attacker":attacker,"defender":b.verteidiger,"reason":grund}));
            let mut beteiligt = vec![b.verteidiger];
            if let Some(sid) = attacker.filter(|sid| *sid != b.verteidiger) {
                beteiligt.push(sid);
            }
            for sid in beteiligt {
                self.vorfall(
                    sid,
                    "kampfkolonisation",
                    format!(
                        "Übernahme von {} abgebrochen: {grund}",
                        self.planeten[pid as usize].koord
                    ),
                );
                self.wecke(sid, Rolle::Feldherr, "Kampfkolonisation abgebrochen", true);
                self.wecke(sid, Rolle::Stratege, "Kampfkolonisation abgebrochen", true);
            }
        }
    }
    pub(crate) fn bewaffnet(&self, einheiten: &[i64]) -> bool {
        einheiten.iter().enumerate().any(|(i, n)| {
            *n > 0
                && (self.regeln.einh(Einheit::ALLE[i]).angriff > 0
                    || matches!(Einheit::ALLE[i],Einheit::Planetenschild|Einheit::PlanetenschildII|Einheit::PlanetenschildIII))
        })
    }
    /// Pending free and hostile colonies share one reservation pool per distinct target.
    pub(crate) fn koloniekapazitaet(&self, sid: SpielerId, ziel: Koord) -> bool {
        let mut ziele = std::collections::BTreeSet::new();
        for f in self.flotten.values() {
            if f.besitzer == sid
                && matches!(
                    f.mission,
                    Mission::Kolonisieren | Mission::Kampfkolonisieren
                )
                && Self::kolonieschiffe(&f.schiffe) > 0
                && matches!(f.zustand, Flottenzustand::Hinflug | Flottenzustand::ImOrbit)
                && !self
                    .belegung
                    .get(&f.ziel)
                    .is_some_and(|p| self.planeten[*p as usize].besitzer == sid)
            {
                ziele.insert(f.ziel);
            }
        }
        ziele.insert(ziel);
        self.kolonien(sid) + ziele.len() <= self.kolonien_erlaubt(sid)
    }
    fn besetzung_kontrolliert(&self, pid: usize, fid: FlottenId) -> bool {
        let p = &self.planeten[pid];
        self.flotten.get(&fid).is_some_and(|f| {
            !p.heimat
                && p.blockade == Some(fid)
                && f.zustand == Flottenzustand::ImOrbit
                && (f.mission == Mission::Kampfkolonisieren || (self.invasion_v2() && f.mission == Mission::Bombardieren))
                && f.besitzer != p.besitzer
                && !self.verbuendet(f.besitzer, p.besitzer)
                && (self.invasion_v2() || Self::kolonieschiffe(&f.schiffe) > 0)
                // Neue Welten: der Platz zählt erst bei der Übernahme (erobern), die Belagerung läuft auch ohne ihn.
                && (self.kolonieplatz_erst_bei_gruendung() || self.kolonien(f.besitzer) < self.kolonien_erlaubt(f.besitzer))
        })
    }
    pub fn integritaet(&self, pid: usize, g: Gebaeude) -> i64 {
        if !self.kolonisation.aktiv
            || (self.kolonisationsregeln_v2() && self.planeten[pid].gebaeude[g.idx()] == 0)
        {
            return 1000;
        }
        i64::from(
            *self
                .kolonisation
                .integritaet
                .get(&(pid as PlanetId, g))
                .unwrap_or(&1000),
        )
    }
    /// `integritaet` für alle Gebäude eines Planeten mit einem Bereichsdurchlauf statt einer Suche je Gebäude
    /// (Sekundenpfade über `werk_ruht`); Werte gleich `integritaet(pid, g)`.
    pub fn integritaeten(&self, pid: usize) -> [i64; GEBAEUDE] {
        let mut a = [1000i64; GEBAEUDE];
        if !self.kolonisation.aktiv {
            return a;
        }
        let v2 = self.kolonisationsregeln_v2();
        let id = pid as PlanetId;
        for (&(_, g), &wert) in self.kolonisation.integritaet.range((id, Gebaeude::ALLE[0])..=(id, Gebaeude::ALLE[GEBAEUDE - 1])) {
            if !(v2 && self.planeten[pid].gebaeude[g.idx()] == 0) {
                a[g.idx()] = i64::from(wert);
            }
        }
        a
    }
    pub fn koloniefracht(&self, sid: SpielerId) -> [i64; GUETER] {
        let mut kosten = [0; GUETER];
        for g in [
            Gebaeude::Erzmine,
            Gebaeude::Kristallmine,
            Gebaeude::Farm,
            Gebaeude::Solarkraftwerk,
        ] {
            for (i, k) in self.regeln.kosten_gebaeude(g, 1).iter().enumerate() {
                kosten[i] += k;
            }
        }
        if !self
            .regeln
            .volk(self.spieler[sid as usize].volk)
            .ohne_nahrung
        {
            kosten[Gut::Nahrung.idx()] += mal(
                self.regeln.wirtschaft.siedler * M,
                self.regeln.wirtschaft.nahrung_je_1000,
            ) * 24
                / 1000;
        }
        kosten
    }
    pub fn koloniefracht_pruefen(
        &self,
        sid: SpielerId,
        ziel: Koord,
        schiffe: &[i64; SCHIFFE],
        ladung: &[i64; GUETER],
    ) -> Result<(), String> {
        if !self.kolonisation.aktiv {
            return Ok(());
        }
        // Invasion v2: Den verwüsteten Planeten hält die eigene Seite am Boden – er ist bekannt.
        if !self.spieler[sid as usize].erkundet.contains_key(&ziel) && !self.verwuestet_fuer(sid, ziel) {
            return Err("Ziel zuerst mit eigener Spionagesonde erkunden".into());
        }
        if !Einheit::ALLE[..SCHIFFE].iter().any(|e| {
            *e != Einheit::Spionagesonde
                && *e != Einheit::Kolonieschiff
                && *e != Einheit::GrossesKolonieschiff
                && schiffe[e.idx()] > 0
                && self.regeln.einh(*e).angriff > 0
        }) {
            return Err("Kolonisation braucht mindestens ein bewaffnetes Begleitschiff".into());
        }
        let mut bedarf = self.koloniefracht(sid);
        if self.ausbau.version>0 {bedarf[Gut::Nahrung.idx()]=0;}
        let fehlt = Gut::ALLE
            .iter()
            .filter(|g| ladung[g.idx()] < bedarf[g.idx()])
            .map(|g| format!("{} {}", (bedarf[g.idx()] + M - 1) / M, g))
            .collect::<Vec<_>>();
        if !fehlt.is_empty() {
            return Err(format!(
                "Kolonie-Startfracht benötigt mindestens: {}",
                fehlt.join(", ")
            ));
        }
        Ok(())
    }
    pub fn bombardement_abschluss(&mut self, pid: usize, fid: FlottenId) {
        if !self.kolonisation.aktiv
            || (if self.kolonisationsregeln_v2() {
                self.bewaffnet(&self.planeten[pid].einheiten)
            } else {
                self.planeten[pid].einheiten.iter().any(|n| *n > 0)
            })
        {
            return;
        }
        if self.invasion_v1() {
            // Invasion v1: nur Bomber senken die Integrität, schrittweise je 15 Spielminuten.
            self.bombardement_beginnen(pid, fid);
            return;
        }
        self.abrechnen(pid);
        for g in Gebaeude::ALLE {
            if self.planeten[pid].gebaeude[g.idx()] > 0 {
                let alt = self.integritaet(pid, g);
                self.kolonisation
                    .integritaet
                    .insert((pid as PlanetId, g), if self.aufklaerungsregeln(){(alt*300/1000) as u16}else{alt.min(300) as u16});
            }
        }
        self.kolonisation
            .reparaturen
            .retain(|(p, _), _| *p != pid as PlanetId);
        // Already paid orders remain cancelled; damage cannot finish a pristine upgrade for free.
        self.planeten[pid].bauschleife.clear();
        if self.kolonisationsregeln_v2() {
            self.ereignisse.retain(
                |e| !matches!(e.art,EreignisArt::BauFertig{planet} if planet==pid as PlanetId),
            );
        }
        let owner = self.planeten[pid].besitzer;
        let target = self.planeten[pid].koord;
        self.vorfall(owner,"bombardement",format!("{target}: Verteidigung und Planetenschilde ausgeschaltet, Gebäude höchstens 30% intakt"));
        if let Some(f) = self.flotten.get(&fid) {
            let attacker = f.besitzer;
            self.vorfall(
                attacker,
                "bombardement",
                format!("{target}: Übernahmebedingungen durch Bombardement vorbereitet"),
            );
        }
        self.raten_neu(pid);
        self.punkte_neu();
        if let Some(f) = self.flotten.get(&fid) {
            self.fachereignis(
                "bombardment_completed",
                f.besitzer,
                serde_json::json!({"planet":pid,"fleet":fid}),
            );
        }
        self.besetzung_beginnen(pid, fid);
    }
    pub fn besetzung_beginnen(&mut self, pid: usize, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid) else {
            return;
        };
        // Invasion v2: auch Bomber mit Bodentruppen landen; das Kolonieschiff braucht es erst für die Übernahme.
        let v2 = self.invasion_v2();
        if !self.kolonisation.aktiv
            || self.planeten[pid].heimat
            || !(f.mission == Mission::Kampfkolonisieren || (v2 && f.mission == Mission::Bombardieren))
            || (!v2 && Self::kolonieschiffe(&f.schiffe) < 1)
            || !self.uebernahme_bereit(pid, fid)
        {
            return;
        }
        if self
            .kolonisation
            .besetzungen
            .get(&(pid as PlanetId))
            .is_some_and(|b| b.flotte == fid)
        {
            return;
        }
        let invasion = self.invasion_v1();
        if invasion && self.landetruppen(fid) == 0 {
            // Landung nur mit Bodentruppen in Truppentransportern.
            return;
        }
        let verteidiger = self.planeten[pid].besitzer;
        let attacker = f.besitzer;
        let ziel = self.planeten[pid].koord;
        if invasion {
            let ende = self.zeit + self.iv_belagerung();
            self.kolonisation.besetzungen.insert(pid as PlanetId, Besetzung { flotte: fid, verteidiger, seit: self.zeit, fruehestens: ende, reaktionsfenster: 0 });
            self.kolonisation.besetzungsmarken.insert(pid as PlanetId, self.zeit);
            self.kolonisation.besetzungsangreifer.insert(pid as PlanetId, attacker);
            self.ausbau.boden.letzte_runde.insert(fid, self.zeit);
            let truppen = self.landetruppen(fid);
            if self.invasion_v2() {
                self.wellenkampf_beginnen(pid, fid);
                let (n, abstand) = (self.linien_anzahl(), self.wellen_abstand() / STUNDE);
                self.fachereignis("occupation_started", attacker, serde_json::json!({"planet":pid,"fleet":fid,"defender":verteidiger,"troops":truppen,"rules":"invasion-v2","lines":n}));
                self.vorfall(verteidiger, "invasion", format!("{ziel}: INVASION – {truppen} Bodeneinheiten gelandet. {n} Verteidigungslinien, die erste Welle in {abstand} Spielstunden: Truppen den Linien zuteilen, Miliz rufen, Nachschub landen"));
                self.vorfall(attacker, "invasion", format!("{ziel}: {truppen} Bodeneinheiten gelandet. {n} Verteidigungslinien, je eine Welle alle {abstand} Spielstunden; ist der Rückhalt gebrochen, kann ein Kolonieschiff landen"));
                for sid in [verteidiger, attacker] {
                    self.wecke(sid, Rolle::Feldherr, "Invasion: Landung", true);
                    self.wecke(sid, Rolle::Stratege, "Invasion: Landung", true);
                }
                return;
            }
            self.fachereignis("occupation_started", attacker, serde_json::json!({"planet":pid,"fleet":fid,"defender":verteidiger,"earliest":ende,"troops":truppen,"rules":"invasion-v1"}));
            for sid in [verteidiger, attacker] {
                self.vorfall(sid, "kampfkolonisation", format!("{ziel}: {truppen} Bodeneinheiten gelandet – Belagerung {} Spielstunden bis frühestens {}; Bodenrunden alle {} Spielminuten", self.regeln.kw().boden.belagerung_stunden, zeittext(ende), self.regeln.kw().boden.runde_minuten));
                self.wecke(sid, Rolle::Feldherr, "Invasion: Belagerung begonnen", true);
                self.wecke(sid, Rolle::Stratege, "Invasion: Belagerung begonnen", true);
            }
            return;
        }
        self.kolonisation.besetzungen.insert(
            pid as PlanetId,
            Besetzung {
                flotte: fid,
                verteidiger,
                seit: self.zeit,
                fruehestens: self.zeit + 2 * 15 * MINUTE,
                reaktionsfenster: 0,
            },
        );
        if self.kolonisationsregeln_v2() {
            self.kolonisation
                .besetzungsmarken
                .insert(pid as PlanetId, self.zeit);
            self.kolonisation
                .besetzungsangreifer
                .insert(pid as PlanetId, attacker);
            self.fachereignis("occupation_started",attacker,serde_json::json!({"planet":pid,"fleet":fid,"defender":verteidiger,"earliest":self.zeit+1800}));
        }
        for sid in [verteidiger, attacker] {
            self.vorfall(sid,"kampfkolonisation",format!("{ziel}: Kolonieschiff angesetzt; mindestens zwei Reaktionsfenster und 30 Spielminuten bis zur Übernahme"));
            self.wecke(
                sid,
                Rolle::Feldherr,
                "Kampfkolonisation: Reaktionsfenster",
                true,
            );
            self.wecke(
                sid,
                Rolle::Stratege,
                "Kampfkolonisation: Reaktionsfenster",
                true,
            );
        }
    }
    pub fn uebernahme_bereit(&self, pid: usize, fid: FlottenId) -> bool {
        if self.kolonisationsregeln_v2() {
            let p = &self.planeten[pid];
            return self.besetzung_kontrolliert(pid, fid)
                && !self.bewaffnet(&p.einheiten)
                && Gebaeude::ALLE
                    .iter()
                    .all(|g| p.gebaeude[g.idx()] == 0 || self.integritaet(pid, *g) <= self.iv_integritaet_grenze())
                && !self.flotten.values().any(|a| {
                    a.id != fid
                        && a.ziel == p.koord
                        && a.zustand == Flottenzustand::ImOrbit
                        && a.mission == Mission::Halten
                        && (a.besitzer == p.besitzer || self.verbuendet(a.besitzer, p.besitzer))
                        && self.bewaffnet(&a.schiffe)
                        // Wie im Orbitkampf (flotte.rs): reine Truppenkonvois (Bodenhilfe) werden nicht bekämpft und sperren
                        // die Landung daher auch nicht – Einsatzprobe 25.10.2026: ein Truppentransporter hielt die Landung
                        // 39,5 Spielstunden auf, seine Truppen kämpften nie.
                        && !(self.invasion_v1() && crate::invasion::nur_truppenkonvoi(&a.schiffe))
                });
        }
        let p = &self.planeten[pid];
        let Some(f) = self.flotten.get(&fid) else {
            return false;
        };
        !p.heimat
            && p.blockade == Some(fid)
            && f.zustand == Flottenzustand::ImOrbit
            && f.besitzer != p.besitzer
            && self.kolonien(f.besitzer) < self.kolonien_erlaubt(f.besitzer)
            && Self::kolonieschiffe(&f.schiffe) > 0
            && p.einheiten.iter().all(|n| *n == 0)
            && Gebaeude::ALLE
                .iter()
                .all(|g| p.gebaeude[g.idx()] == 0 || self.integritaet(pid, *g) <= self.iv_integritaet_grenze())
            && !self.flotten.values().any(|a| {
                a.id != fid
                    && a.ziel == p.koord
                    && a.zustand == Flottenzustand::ImOrbit
                    && a.mission == Mission::Halten
            })
    }
    /// Called after players committed their reactions and before advancing the world clock.
    pub fn kolonisation_fensterabschluss(&mut self) {
        if !self.kolonisation.aktiv {
            return;
        }
        let ready = self
            .kolonisation
            .reparaturen
            .iter()
            .filter(|(_, r)| r.fertig <= self.zeit)
            .map(|(k, r)| (*k, r.clone()))
            .collect::<Vec<_>>();
        for ((pid, g), r) in ready {
            self.kolonisation.reparaturen.remove(&(pid, g));
            if self
                .planeten
                .get(pid as usize)
                .is_some_and(|p| p.besitzer == r.besitzer && p.gebaeude[g.idx()] > 0)
            {
                self.abrechnen(pid as usize);
                self.kolonisation.integritaet.remove(&(pid, g));
                self.raten_neu(pid as usize);
                self.vorfall(
                    r.besitzer,
                    "reparatur",
                    format!(
                        "{} auf {} wieder vollständig intakt",
                        g, self.planeten[pid as usize].koord
                    ),
                );
            }
            if self.kolonisationsregeln_v2() {
                self.bau_starten(pid as usize);
            }
        }
        if self.invasion_v1() {
            self.bombardement_runden();
        }
        for (pid, mut b) in self.kolonisation.besetzungen.clone() {
            if self.invasion_v1() && !self.kolonisation.besetzungen.contains_key(&pid) {
                continue;
            }
            if self.kolonisationsregeln_v2() {
                if self.planeten[pid as usize].besitzer != b.verteidiger
                    || !self.besetzung_kontrolliert(pid as usize, b.flotte)
                {
                    self.besetzung_abbrechen(
                        pid,
                        "Orbit, Besitzer, Beziehung oder Kolonieschiff verloren",
                    );
                    continue;
                }
                let attacker = self.flotten[&b.flotte].besitzer;
                for sid in [b.verteidiger, attacker] {
                    for rolle in [Rolle::Feldherr, Rolle::Stratege] {
                        self.wecke(sid, rolle, "Kampfkolonisation: Reaktionsfenster", true);
                    }
                }
                if self.invasion_v2() {
                    self.wellen_schritt(pid, b);
                    continue;
                }
                if self.invasion_v1() {
                    self.belagerung_schritt(pid, b);
                    continue;
                }
                let letzte = *self
                    .kolonisation
                    .besetzungsmarken
                    .get(&pid)
                    .unwrap_or(&b.seit);
                // Players have just completed a full frozen decision phase. Count a
                // boundary once, independently of when arrival happened in its predecessor.
                if self.zeit <= b.seit
                    || self.zeit <= letzte
                    || self.zeit / self.regeln.fenster() <= letzte / self.regeln.fenster()
                {
                    continue;
                }
                if !self.besetzung_verteidigung_bekaempfen(pid as usize, b.flotte) {
                    self.besetzung_abbrechen(pid, "Bewaffnete Verteidigung nicht ausgeschaltet");
                    continue;
                }
                // A colony-ship casualty inside that battle invalidated the old timer.
                if !self.kolonisation.besetzungen.contains_key(&pid) {
                    continue;
                }
                b.reaktionsfenster = b.reaktionsfenster.saturating_add(1);
                self.kolonisation.besetzungsmarken.insert(pid, self.zeit);
                self.fachereignis(
                    "occupation_window_completed",
                    b.verteidiger,
                    serde_json::json!({"planet":pid,"fleet":b.flotte,"windows":b.reaktionsfenster}),
                );
                if !self.boden_abwehr(pid as usize,b.flotte){
                    if self.kolonisation.besetzungen.contains_key(&pid){self.kolonisation.besetzungen.insert(pid,b);}
                    continue;
                }
                if b.reaktionsfenster >= 2 && self.zeit >= b.fruehestens {
                    self.kolonisation.besetzungen.remove(&pid);
                    self.kolonisation.besetzungsmarken.remove(&pid);
                    self.kolonisation.besetzungsangreifer.remove(&pid);
                    self.erobern(pid as usize, b.flotte);
                } else {
                    self.kolonisation.besetzungen.insert(pid, b);
                }
                continue;
            }
            if self.planeten[pid as usize].besitzer != b.verteidiger
                || !self.uebernahme_bereit(pid as usize, b.flotte)
            {
                self.kolonisation.besetzungen.remove(&pid);
                self.vorfall(
                    b.verteidiger,
                    "kampfkolonisation",
                    format!(
                        "Übernahme von {} abgebrochen",
                        self.planeten[pid as usize].koord
                    ),
                );
                continue;
            }
            if self.zeit <= b.seit {
                continue;
            }
            b.reaktionsfenster = b.reaktionsfenster.saturating_add(1);
            if b.reaktionsfenster >= 2 && self.zeit >= b.fruehestens {
                self.kolonisation.besetzungen.remove(&pid);
                self.erobern(pid as usize, b.flotte);
            } else {
                self.kolonisation.besetzungen.insert(pid, b.clone());
                self.wecke(
                    b.verteidiger,
                    Rolle::Feldherr,
                    "Kampfkolonisation: letztes Reaktionsfenster",
                    true,
                );
            }
        }
    }
    pub fn reparieren(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        k: Koord,
        g: Gebaeude,
    ) -> Result<String, String> {
        if !self.kolonisation.aktiv {
            return Err("Reparatur benötigt Kolonisationsregeln v1".into());
        }
        let pid = self.eigener_planet(sid, k)?;
        if self.invasion_v1()
            && (self.ausbau.boden.invasion.bombardements.contains_key(&(pid as PlanetId))
                || self.kolonisation.besetzungen.contains_key(&(pid as PlanetId)))
        {
            return Err("Unter Bombardierung oder Belagerung sind keine Reparaturen möglich".into());
        }
        // Invasion v2: Ein verwüsteter Planet ist besetzt (Kolonieschiff des Besatzers darf landen) – wie die Bauseite es sagt.
        if self.verwuestet(pid) {
            return Err("Verwüstet: Solange der Planet besetzt ist, sind keine Reparaturen möglich".into());
        }
        let p = &self.planeten[pid];
        if self.kolonisationsregeln_v2()
            && (!p.bauschleife.is_empty()
                || self
                    .kolonisation
                    .reparaturen
                    .keys()
                    .any(|(planet, _)| *planet == pid as PlanetId))
        {
            return Err("Planetare Baustelle bereits durch Bau oder Reparatur belegt".into());
        }
        let hp = self.integritaet(pid, g);
        if p.gebaeude[g.idx()] == 0 || hp >= 1000 {
            return Err("Kein beschädigtes Gebäude dieses Typs".into());
        }
        if self
            .kolonisation
            .reparaturen
            .contains_key(&(pid as PlanetId, g))
            || p.bauschleife.iter().any(|b| b.gebaeude == g)
        {
            return Err("Gebäude bereits in Bau oder Reparatur".into());
        }
        let kosten = self.reparaturkosten(pid, g)?;
        self.abrechnen(pid);
        let topf = Self::topf_fuer(rolle, Topf::Wirtschaft);
        self.zahlbar(pid, &kosten, topf)?;
        let dauer = self.bauzeit(pid, &kosten).max(15 * MINUTE);
        self.zahlen(pid, &kosten, topf);
        self.kolonisation.reparaturen.insert(
            (pid as PlanetId, g),
            Reparatur {
                besitzer: sid,
                fertig: self.zeit + dauer,
            },
        );
        Ok(format!(
            "{g} auf {k} wird repariert; fertig frühestens {}",
            zeittext(self.zeit + dauer)
        ))
    }
}
