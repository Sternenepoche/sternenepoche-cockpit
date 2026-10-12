//! Irreversible elimination in the online epoch, with recoverable crisis windows.
use crate::{typen::*, welt::*, Welt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const REGELTEXT: &str = "Ausscheiden: Ein Reich wird für den Rest der Epoche besiegt, wenn alle seine bewohnten Planeten unter 50% lebensnotwendiger Versorgung bleiben (Standard: 72 ununterbrochene Spielstunden), oder wenn weder Wiederaufbau noch Handel, eigene Kolonien oder eine vorhandene Hilfs-/Rückkehrflotte die Rohstoffwirtschaft wieder starten können (Standard: 48 ununterbrochene Spielstunden). Bei Syntheten zählt Energie statt Nahrung. Erholung beendet die jeweilige Krise und setzt ihre Frist zurück. Die eigene Ansicht zeigt Grund und verbleibende Rettungsfrist. Nach dem Ausscheiden handeln Mensch, Skriptbot und Modellagent nicht mehr; Produktion, Aufträge und eigene Flotten werden stillgelegt. Das Konto bleibt zum Zuschauen erhalten. Ein ausgeschiedener Platz wird erst in der nächsten Epoche wieder vergeben. Die ursprüngliche Heimatwelt bleibt besiedelte, ausgegraute Ruine und kann niemals kolonisiert oder übernommen werden. Kolonien bleiben grundsätzlich eroberbar. Eine Lieferung nach der Niederlage belebt das Reich nicht wieder.";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AusscheidenRegeln {
    pub versorgung_stunden: i64,
    pub stillstand_stunden: i64,
}
impl Default for AusscheidenRegeln {
    fn default() -> Self {
        Self {
            versorgung_stunden: 72,
            stillstand_stunden: 48,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Krise {
    pub versorgung_seit: Option<SimZeit>,
    pub stillstand_seit: Option<SimZeit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Niederlage {
    pub zeit: SimZeit,
    pub grund: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AusscheidenZustand {
    pub regeln: AusscheidenRegeln,
    pub krisen: BTreeMap<SpielerId, Krise>,
    pub besiegt: BTreeMap<SpielerId, Niederlage>,
}
impl AusscheidenZustand {
    pub fn erweitert(&self) -> bool {
        !self.krisen.is_empty()
            || !self.besiegt.is_empty()
            || self.regeln != AusscheidenRegeln::default()
    }
    pub fn validieren(&self, w: &Welt) -> Result<(), String> {
        if !(1..=720).contains(&self.regeln.versorgung_stunden)
            || !(1..=720).contains(&self.regeln.stillstand_stunden)
        {
            return Err("Rettungsfristen müssen 1–720 Spielstunden sein".into());
        }
        if self.besiegt.iter().any(|(sid, n)| {
            *sid as usize >= w.spieler.len()
                || n.zeit < 0
                || n.zeit > w.zeit
                || w.aufklaerung.inaktive_spieler.contains(sid)
        }) || self.krisen.iter().any(|(sid, k)| {
            *sid as usize >= w.spieler.len()
                || w.aufklaerung.inaktive_spieler.contains(sid)
                || self.besiegt.contains_key(sid)
                || k.versorgung_seit
                    .into_iter()
                    .chain(k.stillstand_seit)
                    .any(|t| t < 0 || t > w.zeit)
        }) {
            return Err("Ungültiger Ausscheidestatus im Spielstand".into());
        }
        Ok(())
    }
}
impl Welt {
    pub fn ist_besiegt(&self, sid: SpielerId) -> bool {
        self.ausscheiden.besiegt.contains_key(&sid)
    }
    pub fn reich_status(&self, sid: SpielerId) -> Value {
        if let Some(n) = self.ausscheiden.besiegt.get(&sid) {
            return json!({"status":"besiegt","seit":n.zeit,"grund":n.grund});
        }
        let mut fristen = Vec::new();
        if let Some(k) = self.ausscheiden.krisen.get(&sid) {
            for (seit, stunden, grund) in [
                (
                    k.versorgung_seit,
                    self.ausscheiden.regeln.versorgung_stunden,
                    "Lebensnotwendige Versorgung unter 50%",
                ),
                (
                    k.stillstand_seit,
                    self.ausscheiden.regeln.stillstand_stunden,
                    "Rohstoffwirtschaft ohne erreichbaren Wiederaufbau",
                ),
            ] {
                if let Some(seit) = seit {
                    fristen.push(json!({"grund":grund,"seit":seit,"frist":seit+stunden*STUNDE,"rest_sekunden":(seit+stunden*STUNDE-self.zeit).max(0)}));
                }
            }
        }
        json!({"status":if fristen.is_empty(){"aktiv"}else{"kritisch"},"rettungsfristen":fristen})
    }
    /// Public defeat announcements do not expose crisis times, stocks or coordinates.
    pub fn ausgeschiedene(&self) -> Vec<Value> {
        self.ausscheiden.besiegt.iter().map(|(sid,n)|json!({"spieler":sid,"name":self.spieler[*sid as usize].name,"seit":n.zeit,"grund":n.grund})).collect()
    }
    fn hilfe_unterwegs(&self, sid: SpielerId) -> bool {
        self.flotten.values().any(|f| {
            if f.besitzer == sid {
                return f.ladung.iter().any(|n| *n > 0)
                    || Einheit::ALLE[..SCHIFFE]
                        .iter()
                        .any(|e| self.regeln.einh(*e).ladung > 0 && f.schiffe[e.idx()] > 0);
            }
            let ziel_eigen = self
                .belegung
                .get(&f.ziel)
                .is_some_and(|p| self.planeten[*p as usize].besitzer == sid);
            ziel_eigen
                && !self.ist_besiegt(f.besitzer)
                && !f.mission.feindlich()
                && f.ladung.iter().any(|n| *n > 0)
        }) || self.ereignisse.iter().any(|e| match &e.art {
            EreignisArt::MarktlieferungGebunden {
                empfaenger, menge, ..
            } => *empfaenger == sid && *menge > 0,
            EreignisArt::Marktlieferung { planet, menge, .. } => {
                self.planeten[*planet as usize].besitzer == sid && *menge > 0
            }
            _ => false,
        })
    }
    /// A finite closure over basic recovery steps. It includes reassignable workers,
    /// real repair costs, storage caps, fields and technical gates, not just today's rates.
    fn frachter_abflug_moeglich(&self, pid: usize) -> bool {
        let p = &self.planeten[pid];
        let sid = p.besitzer;
        if p.gebaeude[Gebaeude::Raumhafen.idx()] == 0
            || self.integritaet(pid, Gebaeude::Raumhafen) == 0
            || self.blockiert_fuer(pid, sid)
            || self.flotten.values().filter(|f| f.besitzer == sid).count()
                >= self.flottenplaetze(sid)
        {
            return false;
        }
        let dimensions = &self.regeln.welt;
        let mut ziel = p.koord;
        if dimensions.plaetze_je_system > 1 {
            ziel.position = if ziel.position > 1 {
                ziel.position - 1
            } else {
                2
            };
        } else if dimensions.systeme_je_sektor > 1 {
            ziel.system = if ziel.system > 1 { ziel.system - 1 } else { 2 };
        } else if dimensions.sektoren > 1 {
            ziel.sektor = if ziel.sektor > 1 { ziel.sektor - 1 } else { 2 };
        } else {
            return false;
        }
        let fuel = self.bestand_jetzt(pid)[Gut::Deuterium.idx()];
        Einheit::ALLE[..SCHIFFE].iter().any(|e| {
            if self.regeln.einh(*e).ladung <= 0 || p.einheiten[e.idx()] <= 0 {
                return false;
            }
            let mut ships = [0; SCHIFFE];
            ships[e.idx()] = 1;
            // A legal Save needs both directions; stationing on one's colony is one-way.
            self.flugplan(sid, pid, ziel, &ships, 100)
                .is_ok_and(|plan| fuel >= 2 * plan.treibstoff)
                || self.spieler[sid as usize].planeten.iter().any(|id| {
                    let target = self.planeten[*id as usize].koord;
                    target != p.koord
                        && !self.blockiert_fuer(*id as usize, sid)
                        && self
                            .flugplan(sid, pid, target, &ships, 100)
                            .is_ok_and(|plan| fuel >= plan.treibstoff)
                })
        })
    }
    fn wiederaufbau_moeglich(&self, pid: usize) -> bool {
        let p = &self.planeten[pid];
        let sp = &self.spieler[p.besitzer as usize];
        let r = &self.regeln;
        if p.gebaeude[Gebaeude::Markt.idx()] > 0
            && self.integritaet(pid, Gebaeude::Markt) > 0
            && sp.credits >= M
        {
            return true;
        }
        if self.frachter_abflug_moeglich(pid) {
            return true;
        }
        if p.bauschleife.iter().any(|a| a.fertig.is_some())
            || self
                .kolonisation
                .reparaturen
                .keys()
                .any(|(id, _)| *id == p.id)
        {
            return true;
        }
        let grund = [
            Gebaeude::Solarkraftwerk,
            Gebaeude::Erzmine,
            Gebaeude::Kristallmine,
            Gebaeude::Deuteriumsynthesizer,
            Gebaeude::Farm,
        ];
        let capacity = self.lagergrenze(pid);
        let mut supply = self.bestand_jetzt(pid);
        for g in 0..GUETER {
            if p.rate[g] > 0 {
                supply[g] = supply[g].max(capacity[g]);
            }
        }
        let mut works: Vec<bool> = grund
            .iter()
            .map(|g| p.gebaeude[g.idx()] > 0 && self.integritaet(pid, *g) > 0)
            .collect();
        let mut repaired = vec![false; grund.len()];
        let mut fields = self
            .felder_gesamt(pid)
            .saturating_sub(self.felder_belegt(pid));
        for _ in 0..10 {
            let before = (supply, works.clone(), repaired.clone());
            if (works[0] || p.energie_erzeugung > 0) && p.bevoelkerung > 0 {
                for (i, g) in [(1, Gut::Erz), (2, Gut::Kristall), (3, Gut::Deuterium)] {
                    if works[i] {
                        supply[g.idx()] = supply[g.idx()].max(capacity[g.idx()]);
                    }
                }
                if works[1] && works[2] {
                    return true;
                }
            }
            for (i, g) in grund.iter().enumerate() {
                if repaired[i] || (works[i] && self.integritaet(pid, *g) == 1000) {
                    continue;
                }
                let n = p.gebaeude[g.idx()];
                let gr = r.geb(*g);
                if n == 0
                    && (fields == 0
                        || gr.ab_stufe > sp.stufe
                        || gr
                            .braucht
                            .iter()
                            .any(|(need, level)| p.gebaeude[need.idx()] < *level))
                {
                    continue;
                }
                let cost = if n > 0 {
                    self.reparaturkosten(pid, *g).ok()
                } else {
                    Some(r.kosten_gebaeude(*g, 1))
                };
                if let Some(cost) = cost {
                    if cost.iter().zip(supply).all(|(need, have)| *need <= have) {
                        // Credit only reachable production; spent stocks cannot finance two independent fixes.
                        for j in 0..GUETER {
                            if p.rate[j] <= 0 && supply[j] <= self.bestand_jetzt(pid)[j] {
                                supply[j] -= cost[j];
                            }
                        }
                        works[i] = true;
                        repaired[i] = true;
                        if n == 0 {
                            fields -= 1;
                        }
                    }
                }
            }
            if before == (supply, works.clone(), repaired.clone()) {
                break;
            }
        }
        false
    }
    pub fn ausscheiden_pruefen(&mut self) {
        if !self.aufklaerungsregeln() {
            return;
        }
        for sid in 0..self.spieler.len() as SpielerId {
            if !self.spieler_aktiv(sid) {
                continue;
            }
            let planets = self.spieler[sid as usize].planeten.clone();
            let hunger = !planets.is_empty()
                && planets.iter().all(|id| {
                    let p = &self.planeten[*id as usize];
                    p.nahrung_deckung < 500
                });
            let stuck = !self.hilfe_unterwegs(sid)
                && !planets
                    .iter()
                    .any(|id| self.wiederaufbau_moeglich(*id as usize));
            if !hunger && !stuck {
                if self.ausscheiden.krisen.remove(&sid).is_some() {
                    self.vorfall(
                        sid,
                        "erholung",
                        "Existenzielle Krise beendet; Rettungsfrist zurückgesetzt".into(),
                    );
                }
                continue;
            }
            let neue_krise = !self.ausscheiden.krisen.contains_key(&sid);
            let k = self.ausscheiden.krisen.entry(sid).or_default();
            if hunger {
                k.versorgung_seit.get_or_insert(self.zeit);
            } else {
                k.versorgung_seit = None;
            }
            if stuck {
                k.stillstand_seit.get_or_insert(self.zeit);
            } else {
                k.stillstand_seit = None;
            }
            let grund = if k.versorgung_seit.is_some_and(|t| {
                self.zeit - t >= self.ausscheiden.regeln.versorgung_stunden * STUNDE
            }) {
                Some("Versorgungskollaps")
            } else if k.stillstand_seit.is_some_and(|t| {
                self.zeit - t >= self.ausscheiden.regeln.stillstand_stunden * STUNDE
            }) {
                Some("Wirtschaftlicher Stillstand")
            } else {
                None
            };
            if neue_krise {
                self.vorfall(sid,"existenzkrise","Existenzielle Krise: Rettungsfrist läuft; Versorgung oder Wiederaufbau sichern".into());
                self.wecke(
                    sid,
                    Rolle::Verwalter,
                    "Existenzielle Versorgungskrise",
                    true,
                );
                self.wecke(sid, Rolle::Stratege, "Ausscheiden droht", true);
            }
            if let Some(grund) = grund {
                self.reich_besiegen(sid, grund);
            }
        }
    }
    fn reich_besiegen(&mut self, sid: SpielerId, grund: &str) {
        let planets = self.spieler[sid as usize].planeten.clone();
        for id in &planets {
            self.abrechnen(*id as usize);
            self.orders_stornieren_planet(*id);
        }
        self.ausscheiden.krisen.remove(&sid);
        self.ausscheiden.besiegt.insert(
            sid,
            Niederlage {
                zeit: self.zeit,
                grund: grund.into(),
            },
        );
        let sp = &mut self.spieler[sid as usize];
        sp.ki = false;
        sp.forschung_aktiv = None;
        sp.forschung_schlange.clear();
        for id in planets {
            let p = &mut self.planeten[id as usize];
            p.rate = [0; GUETER];
            p.fp_rate = 0;
            p.bauschleife.clear();
            p.fertigung = [vec![], vec![]];
            p.einheiten[..SCHIFFE].fill(0);
            p.raketen = [0; 2];
        }
        let fleets: Vec<_> = self
            .flotten
            .values()
            .filter(|f| f.besitzer == sid)
            .map(|f| f.id)
            .collect();
        for id in &fleets {
            self.flotten.remove(id);
        }
        let blockades: Vec<_> = self
            .planeten
            .iter()
            .filter(|p| p.blockade.is_some_and(|id| fleets.contains(&id)))
            .map(|p| p.id)
            .collect();
        for id in blockades {
            self.planeten[id as usize].blockade = None;
            self.raten_neu(id as usize);
        }
        self.kolonisation
            .besetzungen
            .retain(|_, b| !fleets.contains(&b.flotte));
        self.kolonisation
            .reparaturen
            .retain(|(id, _), _| self.planeten[*id as usize].besitzer != sid);
        self.faellig.retain(|f| f.spieler != sid);
        self.vorfall(sid,"besiegt",format!("Besiegt: {grund}. Für diese Epoche ausgeschieden; Heimatwelt bleibt vor Kolonisation geschützt."));
    }
}
