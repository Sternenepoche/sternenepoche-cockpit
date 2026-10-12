//! Online V1: private sensor contacts, physical fleet probes and timestamped intelligence.
use crate::{
    flotte::Flugauftrag,
    sicht::{einheiten, gueter},
    typen::*,
    welt::*,
    Welt,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Aufklaerungszustand {
    pub neues_layout: bool,
    pub aktiv: bool,
    pub inaktive_spieler: BTreeSet<SpielerId>,
    /// Hardware fitted when the fleet leaves; later research does not refit it in flight.
    pub abschirmung: BTreeMap<FlottenId, u8>,
    pub sondenziele: BTreeMap<FlottenId, FlottenId>,
    pub flottenberichte: BTreeMap<(SpielerId, FlottenId), Flottenbericht>,
    pub systemberichte: BTreeMap<(SpielerId, u8, u8), Systembericht>,
}
impl Aufklaerungszustand {
    pub fn neu(online: bool) -> Self {
        Self {
            neues_layout: online,
            aktiv: online,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flottenbericht {
    pub zeit: SimZeit,
    pub flotte: FlottenId,
    pub ankunft: SimZeit,
    pub ziel: Koord,
    pub besitzer: Option<SpielerId>,
    pub schiffe: Option<[i64; SCHIFFE]>,
    pub ladung: Option<[i64; GUETER]>,
    pub abgeschirmt: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Systembericht {
    pub zeit: SimZeit,
    pub nebel: bool,
    pub guertel: bool,
}
impl Welt {
    pub fn spieler_aktiv(&self, sid: SpielerId) -> bool {
        (sid as usize) < self.spieler.len() && !self.aufklaerung.inaktive_spieler.contains(&sid) && !self.ist_besiegt(sid)
    }
    /// Reserved lobby homes neither grow nor act until their account claims them.
    pub fn startplatz_reservieren(&mut self, sid: SpielerId) -> Result<(), String> {
        if !self.aufklaerungsregeln() || sid as usize >= self.spieler.len() || self.zeit != 0 {
            return Err("Startplätze nur beim Anlegen einer Onlinewelt reservieren".into());
        }
        self.aufklaerung.inaktive_spieler.insert(sid);
        let sp = &mut self.spieler[sid as usize];
        sp.ki = false;
        sp.schutz_bis = i64::MAX;
        for &pid in &sp.planeten {
            self.planeten[pid as usize].rate = [0; GUETER];
        }
        self.punkte_neu();
        self.fenster_vorbereiten();
        Ok(())
    }
    pub fn startplatz_aktivieren(
        &mut self,
        sid: SpielerId,
        name: String,
        volk: Volk,
    ) -> Result<(), String> {
        if !self.aufklaerung.inaktive_spieler.remove(&sid) {
            return Err("Startplatz bereits belegt".into());
        }
        let sp = &mut self.spieler[sid as usize];
        sp.name = name;
        sp.volk = volk;
        sp.ki = false;
        sp.schutz_bis = self.zeit + self.regeln.diplomatie.anfaengerschutz_tage * TAG;
        let planets = sp.planeten.clone();
        for pid in planets {
            self.planeten[pid as usize].stand = self.zeit;
            self.planeten[pid as usize].gegruendet = self.zeit;
            self.raten_neu(pid as usize);
        }
        self.punkte_neu();
        self.fenster_vorbereiten();
        Ok(())
    }
    pub fn aufklaerungsregeln(&self) -> bool {
        self.aufklaerung.aktiv
    }
    pub fn system_erfasst(&self, sid: SpielerId, k: Koord) -> bool {
        self.spieler[sid as usize].planeten.iter().any(|pid| {
            let p = &self.planeten[*pid as usize];
            p.koord.sektor == k.sektor && p.koord.system == k.system
        }) || self
            .aufklaerung
            .systemberichte
            .contains_key(&(sid, k.sektor, k.system))
    }
    pub(crate) fn system_erkunden_ankunft(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return;
        };
        if let Some(system) = self.system(f.ziel) {
            let kontakte = (1..=self.regeln.welt.plaetze_je_system).map(|position| {
                let k = Koord::neu(f.ziel.sektor, f.ziel.system, position);
                let owner = self.belegung.get(&k).map(|pid|self.planeten[*pid as usize].besitzer)
                    .filter(|sid|!self.aufklaerung.inaktive_spieler.contains(sid));
                (position, owner)
            }).collect();
            let (nebel, guertel) = (system.nebel, system.guertel);
            self.kommunikation.systemkontakte.insert(format!("{}:{}:{}",f.besitzer,f.ziel.sektor,f.ziel.system),kontakte);
            self.aufklaerung.systemberichte.insert(
                (f.besitzer, f.ziel.sektor, f.ziel.system),
                Systembericht {
                    zeit: self.zeit,
                    nebel,
                    guertel,
                },
            );
            self.vorfall(
                f.besitzer,
                "systemaufklaerung",
                format!(
                    "System {}:{} kartiert; Bewohner erkannt, Planetentypen und Ressourcen brauchen Planetensonden",
                    f.ziel.sektor, f.ziel.system
                ),
            );
        }
        self.heimflug(fid);
    }

    /// Upgrade bonuses are gated by ALL required infrastructure and research.
    pub(crate) fn sensorstaerke(&self, sid: SpielerId, pid: usize) -> i64 {
        let p = &self.planeten[pid];
        let sp = &self.spieler[sid as usize];
        let intelligence = p.gebaeude[Gebaeude::Geheimdienst.idx()] as i64
            * self.integritaet(pid, Gebaeude::Geheimdienst) as i64
            / 1000;
        let antenna = p.gebaeude[Gebaeude::Sensorphalanx.idx()] as i64
            * self.integritaet(pid, Gebaeude::Sensorphalanx) as i64
            / 1000;
        let level = intelligence
            .min(antenna)
            .min(sp.forschung[Forschung::Ueberwachungstechnik.idx()] as i64)
            .min(sp.forschung[Forschung::Spionagetechnik.idx()] as i64);
        level
            + if level > 0 && sp.volk == Volk::Syntheten {
                1
            } else {
                0
            }
    }
    pub(crate) fn online_warnzeit(&self, pid: usize) -> i64 {
        let p = &self.planeten[pid];
        // Shielding conceals details, never suppresses the two-hour safety warning.
        (self.regeln.kampf.warnzeit_basis_minuten
            + self.regeln.kampf.warnzeit_je_phalanx_minuten * self.sensorstaerke(p.besitzer, pid))
            * MINUTE
    }
    pub(crate) fn sensor_kontakt(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid) else {
            return;
        };
        if f.gemeldet || f.zustand != Flottenzustand::Hinflug || !f.mission.feindlich() {
            return;
        }
        let Some(&pid) = self.belegung.get(&f.ziel) else {
            return;
        };
        if self.zeit < f.ankunft - self.warnzeit(pid as usize) {
            return;
        }
        let target = self.planeten[pid as usize].besitzer;
        if f.besitzer == target || self.verbuendet(f.besitzer, target) {
            return;
        }
        self.flotten.get_mut(&fid).unwrap().gemeldet = true;
        self.wecke(target, Rolle::Feldherr, "Feindliche Flotte im Anflug", true);
        for sid in 0..self.spieler.len() as SpielerId {
            if sid != target && self.verbuendet(sid, target) {
                self.wecke(
                    sid,
                    Rolle::Feldherr,
                    "Feindliche Flotte im Anflug auf einen Partner",
                    true,
                );
            }
        }
    }

    /// This projection is the sole source for human UI and LLM fleet warnings.
    pub fn angriffswissen(&self, sid: SpielerId, fid: FlottenId) -> Option<Value> {
        if !self.sichtbare_angriffe(sid).contains(&fid) {
            return None;
        }
        let f = &self.flotten[&fid];
        let mut v = json!({"flotte":fid,"ziel":f.ziel.to_string(),"ankunft":zeittext(f.ankunft),
            "ankunft_sekunden":f.ankunft,"in_min":(f.ankunft-self.zeit)/MINUTE,
            "von":null,"schiffe":null,"schiffstypen":null,"schiffe_spanne":null,
            "mission":"feindlicher_anflug","sichtstufe":0});
        if !self.aufklaerungsregeln() {
            v["von"] = json!(self.spieler[f.besitzer as usize].name);
            v["schiffe"] = json!(f.schiffe.iter().sum::<i64>());
            v["mission"] = json!(f.mission.name());
            return Some(v);
        }
        let pid = *self.belegung.get(&f.ziel)? as usize;
        // Allied sensor contacts share timing. Details still require the viewer's own training.
        let own_pid = if self.planeten[pid].besitzer == sid {
            pid
        } else {
            self.spieler[sid as usize]
                .planeten
                .iter()
                .map(|p| *p as usize)
                .max_by_key(|p| self.sensorstaerke(sid, *p))
                .unwrap_or(self.spieler[sid as usize].heimat as usize)
        };
        let shield = self.aufklaerung.abschirmung.get(&fid).copied().unwrap_or(0) as i64;
        let strength = (self.sensorstaerke(sid, own_pid) - shield).max(0);
        let total = f.schiffe.iter().sum::<i64>();
        if strength >= 1 {
            v["sichtstufe"] = json!(1);
            v["von"] = json!(self.spieler[f.besitzer as usize].name);
            v["schiffe_spanne"] = json!(crate::sternkarte::intervall(total, 10));
        }
        if strength >= 3 {
            v["sichtstufe"] = json!(2);
            v["schiffe"] = json!(total);
        }
        if strength >= 6 {
            v["sichtstufe"] = json!(3);
            v["schiffstypen"] = einheiten(&f.schiffe, 0);
            v["mission"] = json!(f.mission.name());
        }
        // Active reports remain historical. Never silently refresh their contents from live state.
        if let Some(b) = self.aufklaerung.flottenberichte.get(&(sid, fid)) {
            v["sondenbericht"] = self.flottenbericht_sicht(b);
        }
        Some(v)
    }
    fn flottenbericht_sicht(&self, b: &Flottenbericht) -> Value {
        json!({"flotte":b.flotte,"zeit":b.zeit,"alter_sekunden":self.zeit-b.zeit,
            "ankunft_sekunden_beim_scan":b.ankunft,"ziel_beim_scan":b.ziel.to_string(),
            "von":b.besitzer.map(|s| self.spieler[s as usize].name.clone()),
            "schiffstypen":b.schiffe.as_ref().map(|s|einheiten(s,0)),
            "schiffe":b.schiffe.as_ref().map(|s|s.iter().sum::<i64>()),
            "ladung":b.ladung.as_ref().map(|s|gueter(s,false)),"abgeschirmt":b.abgeschirmt,"historisch":true})
    }
    pub fn eigene_flottenberichte(&self, sid: SpielerId) -> Vec<Value> {
        self.aufklaerung
            .flottenberichte
            .iter()
            .filter(|((owner, _), _)| *owner == sid)
            .map(|(_, b)| self.flottenbericht_sicht(b))
            .collect()
    }
    pub fn flotte_ausspaehen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        start: Koord,
        ziel: FlottenId,
        sonden: i64,
        sigma_pm: i64,
    ) -> Result<String, String> {
        if !self.aufklaerungsregeln() {
            return Err("Flottenaufklärung benötigt Online-Regeln v1".into());
        }
        // Use identical rejection for guessed IDs, hidden fleets and foreign non-contacts.
        if !self.sichtbare_angriffe(sid).contains(&ziel) {
            return Err("Kein erfasster feindlicher Anflug für diese Sonde".into());
        }
        if !(1..=1000).contains(&sonden) {
            return Err("1 bis 1000 Spionagesonden wählen".into());
        }
        let target = self.flotten[&ziel].clone();
        let pid = self.eigener_planet(sid, start)?;
        let mut ships = [0; SCHIFFE];
        ships[Einheit::Spionagesonde.idx()] = sonden;
        let plan = self.flugplan(sid, pid, target.ziel, &ships, sigma_pm)?;
        if self.zeit + plan.dauer >= target.ankunft {
            return Err("Sonde erreicht den Anflugkorridor erst nach dem Angriff".into());
        }
        self.flotte_senden_intern(
            sid,
            rolle,
            Flugauftrag {
                start,
                ziel: target.ziel,
                mission: Mission::FlottenSpionage,
                schiffe: ships,
                sigma_pm,
                ladung: [0; GUETER],
                haltedauer: 0,
            },
            Some(ziel),
        )
    }
    pub(crate) fn flotten_spionage_ankunft(&mut self, probe: FlottenId) {
        let Some(p) = self.flotten.get(&probe).cloned() else {
            return;
        };
        let Some(target_id) = self.aufklaerung.sondenziele.remove(&probe) else {
            self.heimflug(probe);
            return;
        };
        let target = self.flotten.get(&target_id).cloned().filter(|f| {
            f.zustand == Flottenzustand::Hinflug && f.ankunft > self.zeit && f.ziel == p.ziel
        });
        let Some(f) = target else {
            self.vorfall(
                p.besitzer,
                "flottenaufklaerung",
                "Anflugkontakt verloren; Sonde kehrt zurück".into(),
            );
            self.heimflug(probe);
            return;
        };
        if !self.sichtbare_angriffe(p.besitzer).contains(&target_id) {
            self.heimflug(probe);
            return;
        }
        let player = &self.spieler[p.besitzer as usize];
        let spy = player.forschung[Forschung::Spionagetechnik.idx()] as i64;
        let g = self.planeten[p.start as usize].gebaeude[Gebaeude::Geheimdienst.idx()] as i64
            * self.integritaet(p.start as usize, Gebaeude::Geheimdienst) as i64
            / 1000;
        let advantage = spy
            + g
            + if spy > 0 && player.volk == Volk::Veyari {
                1
            } else {
                0
            };
        let shield = self
            .aufklaerung
            .abschirmung
            .get(&target_id)
            .copied()
            .unwrap_or(0) as i64;
        let success = shield == 0 || advantage > shield;
        let b = Flottenbericht {
            zeit: self.zeit,
            flotte: target_id,
            ankunft: f.ankunft,
            ziel: f.ziel,
            besitzer: success.then_some(f.besitzer),
            schiffe: success.then_some(f.schiffe),
            ladung: (success && advantage >= shield + 5).then_some(f.ladung),
            abgeschirmt: !success,
        };
        self.aufklaerung
            .flottenberichte
            .insert((p.besitzer, target_id), b);
        // Bound historical storage per player, independently of fleet IDs.
        let mut reports: Vec<_> = self
            .aufklaerung
            .flottenberichte
            .iter()
            .filter(|((s, _), _)| *s == p.besitzer)
            .map(|(key, b)| (b.zeit, *key))
            .collect();
        reports.sort();
        for (_, key) in reports.iter().take(reports.len().saturating_sub(100)) {
            self.aufklaerung.flottenberichte.remove(key);
        }
        self.vorfall(
            p.besitzer,
            "flottenaufklaerung",
            if success {
                "Flottenbericht eingetroffen"
            } else {
                "Flottenbericht durch Abschirmung verdeckt"
            }
            .into(),
        );
        self.wecke(
            p.besitzer,
            Rolle::Feldherr,
            "Flottenbericht eingetroffen",
            true,
        );
        self.vorfall(
            f.besitzer,
            "flottenaufklaerung",
            "Eine Sonde hat die Flotte untersucht".into(),
        );
        let mut rng = strom(self.startwert, KANAL_SPIONAGE, self.spionage_nr);
        self.spionage_nr += 1;
        let risk = if success { 100 } else { 500 };
        if zufall(&mut rng, 1000) < risk {
            self.flotten.remove(&probe);
        } else {
            self.heimflug(probe);
        }
    }
}
