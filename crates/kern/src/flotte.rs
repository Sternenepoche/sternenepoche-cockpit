//! Flotten: Entfernung, Flugzeit, Treibstoff, Missionen, Kampf am Ziel, Plünderung,
//! Blockade, Invasion, Spionage, Kolonisierung, Recycling und Abbau.

use crate::kampf::{kampf, Gruppe};
use crate::kolonisation::{Rueckweg, Transportziel};
use crate::typen::*;
use crate::welt::*;

#[derive(Clone, Debug)]
pub struct Flugauftrag {
    pub start: Koord,
    pub ziel: Koord,
    pub mission: Mission,
    pub schiffe: [i64; SCHIFFE],
    /// Geschwindigkeitsstufe in Promille, 100 bis 1000.
    pub sigma_pm: i64,
    pub ladung: [i64; GUETER],
    /// Sekunden im Orbit für Halten und Abbau.
    pub haltedauer: i64,
}

#[derive(Clone, Debug)]
pub struct Flugplan {
    pub entfernung: i64,
    pub dauer: i64,
    /// Treibstoff für eine Strecke, in Tausendsteln.
    pub treibstoff: i64,
    pub kapazitaet: i64,
    pub tempo: i64,
}

enum Partei {
    Planet(usize),
    Flotte(FlottenId),
}

impl Welt {
    pub fn entfernung(&self, a: Koord, b: Koord) -> i64 {
        let f = &self.regeln.flug;
        if a.sektor != b.sektor {
            f.je_sektor * (a.sektor as i64 - b.sektor as i64).abs()
        } else if a.system != b.system {
            f.sektor_basis + f.je_system * (a.system as i64 - b.system as i64).abs()
        } else {
            f.system_basis + f.je_position * (a.position as i64 - b.position as i64).abs()
        }
    }

    pub fn tempo(&self, sid: SpielerId, e: Einheit) -> i64 {
        let er = self.regeln.einh(e);
        let Some(antrieb) = er.antrieb else {
            return 0;
        };
        let bonus = self
            .regeln
            .flug
            .antrieb_bonus
            .get(&antrieb)
            .copied()
            .unwrap_or(0.0);
        mal(
            er.tempo,
            1.0 + bonus * self.spieler[sid as usize].forschung[antrieb.idx()] as f64,
        )
    }

    /// Flugzeit in Spielsekunden: max(min, k * (10 + 350/sigma * sqrt(10 d / v))).
    pub fn flugdauer(&self, d: i64, v: i64, sigma_pm: i64) -> i64 {
        let f = &self.regeln.flug;
        let w = wurzel(10 * d as i128 * 1_000_000 / v.max(1) as i128) as i64;
        (f.zeitfaktor * (10 + 350 * w / sigma_pm.max(1))).max(f.min_sekunden)
    }

    pub fn flugplan(
        &self,
        sid: SpielerId,
        start_pid: usize,
        ziel: Koord,
        schiffe: &[i64; SCHIFFE],
        sigma_pm: i64,
    ) -> Result<Flugplan, String> {
        if !(100..=1000).contains(&sigma_pm) || schiffe.iter().any(|n| *n < 0 || *n > 1_000_000) {
            return Err(
                "ungültige Geschwindigkeit oder Schiffszahl (maximal 1.000.000 je Typ)".into(),
            );
        }
        let r = &self.regeln;
        let sp = &self.spieler[sid as usize];
        let p = &self.planeten[start_pid];
        let mut tempo = i64::MAX;
        let mut verbrauch = 0i64;
        let mut ladung = 0i64;
        for e in 0..SCHIFFE {
            if schiffe[e] > 0 {
                let einheit = Einheit::ALLE[e];
                let er = r.einh(einheit);
                tempo = tempo.min(self.tempo(sid, einheit));
                verbrauch += schiffe[e] * er.verbrauch;
                ladung += schiffe[e] * er.ladung;
            }
        }
        if tempo == i64::MAX || tempo <= 0 {
            return Err("keine Schiffe gewählt".into());
        }
        let d = self.entfernung(p.koord, ziel);
        let dauer = self.flugdauer(d, tempo, sigma_pm);
        let ersparnis = (r.flug.raumhafen_ersparnis * p.gebaeude[Gebaeude::Raumhafen.idx()] as f64)
            .min(r.flug.raumhafen_ersparnis_max);
        let roh = M as i128
            + verbrauch as i128 * M as i128 * d as i128 * (sigma_pm as i128 * sigma_pm as i128)
                / (r.flug.treibstoff_teiler as i128 * 1_000_000);
        let treibstoff = mal(roh as i64, 1.0 - ersparnis);
        let logistik = 1.0 + r.flug.logistik_bonus * sp.forschung[Forschung::Logistik.idx()] as f64;
        let kapazitaet = mal(ladung * M, r.volk(sp.volk).ladung * logistik);
        Ok(Flugplan {
            entfernung: d,
            dauer,
            treibstoff,
            kapazitaet,
            tempo,
        })
    }

    pub fn flottenplaetze(&self, sid: SpielerId) -> usize {
        1 + self.spieler[sid as usize].forschung[Forschung::Computertechnik.idx()] as usize
    }

    pub fn kolonien_erlaubt(&self, sid: SpielerId) -> usize {
        let astro = self.spieler[sid as usize].forschung[Forschung::Astrophysik.idx()] as usize;
        if astro == 0 {
            0
        } else {
            (1 + (astro - 1) / 2).min(self.regeln.wirtschaft.kolonien_max as usize)
        }
    }

    /// Vorwarnzeit eines Planeten in Sekunden.
    pub fn warnzeit(&self, pid: usize) -> i64 {
        if self.aufklaerungsregeln() { return self.online_warnzeit(pid); }
        let k = &self.regeln.kampf;
        (k.warnzeit_basis_minuten
            + k.warnzeit_je_phalanx_minuten
                * self.planeten[pid].gebaeude[Gebaeude::Sensorphalanx.idx()] as i64)
            * MINUTE
    }

    /// Feindliche Flotten im Sensorbereich eigener Planeten und der Planeten von Bündnispartnern.
    pub fn sichtbare_angriffe(&self, sid: SpielerId) -> Vec<FlottenId> {
        let mut v = Vec::new();
        for f in self.flotten.values() {
            if !f.mission.feindlich() || f.zustand != Flottenzustand::Hinflug || f.besitzer == sid {
                continue;
            }
            let Some(zp) = self.belegung.get(&f.ziel) else {
                continue;
            };
            let ziel = &self.planeten[*zp as usize];
            if (ziel.besitzer == sid || self.verbuendet(sid, ziel.besitzer))
                && self.zeit >= f.ankunft - self.warnzeit(*zp as usize)
            {
                v.push(f.id);
            }
        }
        v
    }

    pub(crate) fn blockiert_fuer(&self, pid: usize, sid: SpielerId) -> bool {
        match self.planeten[pid].blockade {
            Some(bf) => self
                .flotten
                .get(&bf)
                .map(|f| {
                    f.besitzer != sid
                        && (!self.kolonisationsregeln_v2()
                            || (f.zustand == Flottenzustand::ImOrbit
                                && !self.verbuendet(sid, f.besitzer)))
                })
                .unwrap_or(false),
            None => false,
        }
    }

    pub fn flotte_senden(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        a: Flugauftrag,
    ) -> Result<String, String> {
        self.flotte_senden_intern(sid, rolle, a, None)
    }

    pub(crate) fn flotte_senden_intern(&mut self, sid: SpielerId, rolle: Rolle,
        a: Flugauftrag, sondenziel: Option<FlottenId>) -> Result<String, String> {
        let r = self.regeln.clone();
        let pid = self.eigener_planet(sid, a.start)?;
        let p = &self.planeten[pid];
        let sp = &self.spieler[sid as usize];
        if p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 || self.integritaet(pid, Gebaeude::Raumhafen) == 0 {
            return Err(format!(
                "Flottenstart braucht einen funktionsfähigen Raumhafen auf {}",
                a.start
            ));
        }
        if !a.schiffe.iter().any(|n| *n > 0) {
            return Err("keine Schiffe gewählt".into());
        }
        for e in 0..SCHIFFE {
            if a.schiffe[e] < 0 || a.schiffe[e] > p.einheiten[e] {
                return Err(format!(
                    "auf {} stehen nur {} x {}",
                    a.start,
                    p.einheiten[e],
                    Einheit::ALLE[e]
                ));
            }
        }
        let plaetze = self.flottenplaetze(sid);
        if self.flotten.values().filter(|f| f.besitzer == sid).count() >= plaetze {
            return Err(format!(
                "alle {plaetze} Flottenplätze sind belegt, Computertechnik erhöht die Zahl"
            ));
        }
        if !(100..=1000).contains(&a.sigma_pm) {
            return Err("geschwindigkeit muss zwischen 0.1 und 1.0 liegen".into());
        }
        if self.system(a.ziel).is_none() || a.ziel.position > r.welt.plaetze_je_system {
            return Err(format!("{} liegt außerhalb der Galaxie", a.ziel));
        }
        if (a.ziel.position == 0) != (a.mission == Mission::Abbau) {
            return Err("Position 0 ist der Asteroidengürtel: nur die Mission abbau fliegt dorthin, und nur dorthin".into());
        }
        if self.blockiert_fuer(pid, sid) && !(a.mission == Mission::Angriff && a.ziel == a.start) {
            return Err(format!(
                "{} ist blockiert: möglich ist nur ein Angriff auf die Blockadeflotte (Ziel {})",
                a.start, a.start
            ));
        }
        if self.aufklaerungsregeln() {
            let own = sp.planeten.iter().any(|id|self.planeten[*id as usize].koord==a.ziel);
            if a.mission == Mission::Abbau && !self.system_erfasst(sid,a.ziel) {
                return Err("Sonnensystem zuerst mit einer Systemsonde kartieren".into());
            }
            if !own && !matches!(a.mission,Mission::SystemErkunden|Mission::Spionage|Mission::FlottenSpionage|Mission::Saven|Mission::Abbau)
                && self.planetenwissen(sid,a.ziel)["bekannt"] != true {
                return Err("Ziel zuerst mit einer eigenen Planetensonde aufklären".into());
            }
        }
        let ziel_pid = self.belegung.get(&a.ziel).map(|x| *x as usize);
        let hat = |e: Einheit| a.schiffe[e.idx()] > 0;
        let mut einweg = false;
        let mut siedler = 0i64;
        let mut haltedauer = 0i64;
        let mut feindlich_gegen: Option<SpielerId> = None;
        match a.mission {
            Mission::SystemErkunden => {
                if !self.aufklaerungsregeln() { return Err("Systemaufklärung benötigt Online-Regeln v1".into()); }
                if (0..SCHIFFE).any(|e|e != Einheit::Spionagesonde.idx() && a.schiffe[e]>0) {
                    return Err("Systemaufklärung verwendet ausschließlich Spionagesonden".into());
                }
            }
            Mission::Saven => {
                if !self.aufklaerungsregeln() { return Err("Saven benötigt Online-Regeln v1".into()); }
                if a.ziel == a.start { return Err("Save-Flug benötigt eine andere Zielkoordinate".into()); }
                if !(0..=72 * STUNDE).contains(&a.haltedauer) { return Err("Save-Wartezeit: 0 bis 72 Spielstunden".into()); }
                haltedauer = a.haltedauer;
            }
            Mission::FlottenSpionage => {
                if !self.aufklaerungsregeln() || sondenziel.is_none() {
                    return Err("Flottensonden mit flotte_ausspaehen starten".into());
                }
                if (0..SCHIFFE).any(|e|e != Einheit::Spionagesonde.idx() && a.schiffe[e]>0) {
                    return Err("Flottenaufklärung verwendet ausschließlich Sonden".into());
                }
            }
            Mission::Angriff
            | Mission::Blockade
            | Mission::Invasion
            | Mission::Bombardieren
            | Mission::Kampfkolonisieren => {
                if !self.kolonisation.aktiv
                    && matches!(
                        a.mission,
                        Mission::Bombardieren | Mission::Kampfkolonisieren
                    )
                {
                    return Err("Mission benötigt Kolonisationsregeln v1".into());
                }
                if self.kolonisation.aktiv && a.mission == Mission::Invasion {
                    return Err("Stabilitäts-Invasion ersetzt: bombardieren und kampfkolonisieren verwenden".into());
                }
                let Some(zp) = ziel_pid else {
                    return Err(format!("auf {} liegt kein Planet", a.ziel));
                };
                let ziel = &self.planeten[zp];
                if self.kolonisationsregeln_v2() {
                    self.offensive_pruefen(sid, zp, a.mission, &a.schiffe)?;
                    feindlich_gegen = self.tatsaechlicher_gegner(zp, sid);
                } else {
                    if ziel.besitzer == sid {
                        if a.mission != Mission::Angriff || !self.blockiert_fuer(zp, sid) {
                            return Err("eigene Planeten kann man nicht angreifen".into());
                        }
                    } else {
                        let gegner = &self.spieler[ziel.besitzer as usize];
                        if self.zeit < gegner.schutz_bis {
                            return Err(format!(
                                "{} steht bis {} unter Anfängerschutz",
                                gegner.name,
                                zeittext(gegner.schutz_bis)
                            ));
                        }
                        let schwach = r.diplomatie.schwachenschutz_anteil;
                        if schwach > 0.0
                            && gegner.punkte.gesamt() < mal(sp.punkte.gesamt(), schwach)
                            && !gegner.angegriffen.contains(&sid)
                        {
                            return Err(format!(
                                "{} ist zu schwach für einen Angriff und hat dich nie angegriffen",
                                gegner.name
                            ));
                        }
                        if a.mission != Mission::Angriff && sp.stufe < 4 {
                            return Err(
                                "Blockade und Invasion gibt es ab Zivilisationsstufe 4".into()
                            );
                        }
                        if a.mission == Mission::Invasion {
                            if ziel.heimat {
                                return Err("Heimatwelten können nicht erobert werden".into());
                            }
                            if !hat(Einheit::Truppentransporter) {
                                return Err("eine Invasion braucht Truppentransporter".into());
                            }
                        }
                        if a.mission == Mission::Kampfkolonisieren {
                            if ziel.heimat {
                                return Err(
                                    "Der ursprüngliche Heimatplanet ist vor Übernahme geschützt"
                                        .into(),
                                );
                            }
                            if !hat(Einheit::Kolonieschiff) {
                                return Err("Kampfkolonisierung braucht ein Kolonieschiff".into());
                            }
                            if self.kolonien(sid) >= self.kolonien_erlaubt(sid) {
                                return Err("Kolonielimit erreicht".into());
                            }
                        }
                        feindlich_gegen = Some(ziel.besitzer);
                    }
                }
            }
            Mission::Transport => {
                if ziel_pid.is_none() {
                    return Err(format!("auf {} liegt kein Planet", a.ziel));
                }
            }
            Mission::Stationieren => {
                let zp = self.eigener_planet(sid, a.ziel)?;
                if zp == pid {
                    return Err("Start und Ziel sind derselbe Planet".into());
                }
                einweg = true;
            }
            Mission::Halten => {
                let Some(zp) = ziel_pid else {
                    return Err(format!("auf {} liegt kein Planet", a.ziel));
                };
                let partner = self.planeten[zp].besitzer;
                if partner == sid || !self.verbuendet(sid, partner) {
                    return Err("Halten geht nur bei einem Partner mit Verteidigungsbündnis oder in derselben Allianz".into());
                }
                haltedauer = a.haltedauer;
                if haltedauer < STUNDE || haltedauer > r.flug.halten_max_stunden * STUNDE {
                    return Err(format!(
                        "haltedauer_stunden muss zwischen 1 und {} liegen",
                        r.flug.halten_max_stunden
                    ));
                }
            }
            Mission::Spionage => {
                if self.aufklaerungsregeln() && !self.system_erfasst(sid,a.ziel) {
                    return Err("Sonnensystem zuerst mit system_erkunden und einer Spionagesonde kartieren".into());
                }
                if (0..SCHIFFE).any(|e| e != Einheit::Spionagesonde.idx() && a.schiffe[e] > 0) {
                    return Err("eine Spionageflotte besteht nur aus Spionagesonden".into());
                }
                if ziel_pid
                    .map(|zp| self.planeten[zp].besitzer == sid)
                    .unwrap_or(false)
                {
                    return Err("eigene Planeten muss man nicht ausspionieren".into());
                }
            }
            Mission::Kolonisieren => {
                self.koloniefracht_pruefen(sid, a.ziel, &a.schiffe, &a.ladung)?;
                if !hat(Einheit::Kolonieschiff) {
                    return Err("Kolonisieren braucht ein Kolonieschiff".into());
                }
                if ziel_pid.is_some() && !self.aufklaerungsregeln() {
                    return Err(format!("{} ist schon besiedelt", a.ziel));
                }
                let unterwegs = self
                    .flotten
                    .values()
                    .filter(|f| {
                        f.besitzer == sid
                            && f.mission == Mission::Kolonisieren
                            && f.zustand == Flottenzustand::Hinflug
                    })
                    .count();
                let erlaubt = self.kolonien_erlaubt(sid);
                if if self.kolonisationsregeln_v2() {
                    !self.koloniekapazitaet(sid, a.ziel)
                } else {
                    self.kolonien(sid) + unterwegs >= erlaubt
                } {
                    return Err(format!(
                        "Astrophysik erlaubt {erlaubt} Kolonien, höhere Stufen erlauben mehr"
                    ));
                }
                siedler = r.wirtschaft.siedler * M;
                if p.bevoelkerung - siedler < 1000 * M {
                    return Err(format!(
                        "für {} Siedler müssen auf {} mindestens 1000 Einwohner zurückbleiben",
                        r.wirtschaft.siedler, a.start
                    ));
                }
                einweg = true;
            }
            Mission::Recyceln => {
                if !hat(Einheit::Recycler) {
                    return Err("Recyceln braucht Recycler".into());
                }
            }
            Mission::Abbau => {
                if !hat(Einheit::Bergbauschiff) {
                    return Err("Abbau braucht Bergbauschiffe".into());
                }
                if !self.system(a.ziel).map(|s| s.guertel).unwrap_or(false) {
                    return Err(format!(
                        "System {}:{} hat keinen Asteroidengürtel",
                        a.ziel.sektor, a.ziel.system
                    ));
                }
                haltedauer = a.haltedauer;
                if haltedauer < STUNDE || haltedauer > r.flug.abbau_max_stunden * STUNDE {
                    return Err(format!(
                        "haltedauer_stunden muss zwischen 1 und {} liegen",
                        r.flug.abbau_max_stunden
                    ));
                }
            }
        }

        let plan = self.flugplan(sid, pid, a.ziel, &a.schiffe, a.sigma_pm)?;
        let fracht = a.ladung.iter().try_fold(0i64,|sum,n|sum.checked_add(*n)).ok_or("Ladung ist zu groß")?;
        if a.ladung.iter().any(|m| *m < 0) {
            return Err("ladung darf nicht negativ sein".into());
        }
        if fracht > plan.kapazitaet {
            return Err(format!(
                "Ladung {} übersteigt die Kapazität {}",
                ganz(fracht),
                ganz(plan.kapazitaet)
            ));
        }
        if matches!(a.mission, Mission::Spionage | Mission::FlottenSpionage | Mission::SystemErkunden) && fracht > 0 {
            return Err("Sonden tragen keine Ladung".into());
        }
        let treibstoff = plan.treibstoff * if einweg { 1 } else { 2 };
        self.abrechnen(pid);
        let mut bedarf = a.ladung;
        bedarf[Gut::Deuterium.idx()] = bedarf[Gut::Deuterium.idx()].checked_add(treibstoff).ok_or("Ladung und Treibstoff sind zu groß")?;
        let mut sprit = [0i64; GUETER];
        sprit[Gut::Deuterium.idx()] = treibstoff;
        let topf = Self::topf_fuer(rolle, Topf::Wirtschaft);
        // Bestand muss Ladung und Treibstoff decken, der Topf nur den Treibstoff.
        self.zahlbar(pid, &bedarf, None)?;
        self.zahlbar(pid, &sprit, topf)?;
        self.zahlen(pid, &sprit, topf);
        let p = &mut self.planeten[pid];
        for g in 0..GUETER {
            p.bestand[g] -= a.ladung[g];
        }
        for e in 0..SCHIFFE {
            p.einheiten[e] -= a.schiffe[e];
        }
        p.bevoelkerung -= siedler;
        let id = self.naechste_flotte;
        self.naechste_flotte += 1;
        let ankunft = self.zeit + plan.dauer;
        self.flotten.insert(
            id,
            Flotte {
                id,
                besitzer: sid,
                start: self.planeten[pid].id,
                start_koord: a.start,
                ziel: a.ziel,
                mission: a.mission,
                schiffe: a.schiffe,
                ladung: a.ladung,
                siedler,
                abflug: self.zeit,
                ankunft,
                flugdauer: plan.dauer,
                rueckkehr: 0,
                orbit_ende: 0,
                haltedauer,
                zustand: Flottenzustand::Hinflug,
                gemeldet: false,
                verband: None,
            },
        );
        self.plane(ankunft, EreignisArt::FlotteAnkunft { flotte: id });
        if self.aufklaerungsregeln() {
            self.aufklaerung.abschirmung.insert(id, self.spieler[sid as usize].forschung[Forschung::Abschirmtechnik.idx()]);
            if let Some(target) = sondenziel { self.aufklaerung.sondenziele.insert(id, target); }
            if a.mission.feindlich() {
                if let Some(zp) = ziel_pid {
                    let kontakt = (ankunft - self.warnzeit(zp)).max(self.zeit);
                    if kontakt == self.zeit { self.sensor_kontakt(id); }
                    else { self.plane(kontakt, EreignisArt::SensorKontakt { flotte:id }); }
                }
            }
        }
        if self.kolonisationsregeln_v2() && a.mission == Mission::Transport {
            if let Some(zp) = ziel_pid {
                self.kolonisation.transportziele.insert(
                    id,
                    Transportziel::Planet {
                        planet: zp as PlanetId,
                        empfaenger: self.planeten[zp].besitzer,
                    },
                );
            }
        }
        self.fachereignis(
            "fleet_launched",
            sid,
            serde_json::json!({"fleet":id,"mission":a.mission,"target":a.ziel,"arrival":ankunft}),
        );
        if feindlich_gegen.is_some() {
            // Wer selbst angreift, verliert den Anfängerschutz sofort.
            let jetzt = self.zeit;
            let sp = &mut self.spieler[sid as usize];
            sp.schutz_bis = sp.schutz_bis.min(jetzt);
        }
        if siedler > 0 {
            self.raten_neu(pid);
        }
        Ok(format!(
            "Flotte {id} gestartet: {} nach {}, Ankunft {}, Treibstoff {}",
            a.mission,
            a.ziel,
            zeittext(ankunft),
            ganz(treibstoff)
        ))
    }

    /// Eine eigene ausfliegende Angriffsflotte freiwillig für Allianzpartner öffnen.
    fn tatsaechlicher_gegner(&self, pid: usize, sid: SpielerId) -> Option<SpielerId> {
        if let Some(f) = self.planeten[pid]
            .blockade
            .and_then(|id| self.flotten.get(&id))
            .filter(|f| f.zustand == Flottenzustand::ImOrbit)
        {
            return (f.besitzer != sid && !self.verbuendet(sid, f.besitzer)).then_some(f.besitzer);
        }
        let owner = self.planeten[pid].besitzer;
        (owner != sid && !self.verbuendet(sid, owner)).then_some(owner)
    }
    fn gegner_angreifbar(&self, sid: SpielerId, gegner: SpielerId) -> Result<(), String> {
        if sid == gegner || self.verbuendet(sid, gegner) {
            return Err("Gegner ist inzwischen eigener oder verbündeter Spieler".into());
        }
        let g = &self.spieler[gegner as usize];
        let sp = &self.spieler[sid as usize];
        if self.zeit < g.schutz_bis {
            return Err(format!("{} steht unter Anfängerschutz", g.name));
        }
        let schwach = self.regeln.diplomatie.schwachenschutz_anteil;
        if schwach > 0.0
            && g.punkte.gesamt() < mal(sp.punkte.gesamt(), schwach)
            && !g.angegriffen.contains(&sid)
        {
            return Err("Tatsächlicher Gegner steht unter Schwachenschutz".into());
        }
        Ok(())
    }
    fn offensive_pruefen(
        &self,
        sid: SpielerId,
        pid: usize,
        mission: Mission,
        schiffe: &[i64; SCHIFFE],
    ) -> Result<(), String> {
        let p = &self.planeten[pid];
        let freund = p.besitzer == sid || self.verbuendet(sid, p.besitzer);
        let eigene_blockade = p
            .blockade
            .and_then(|id| self.flotten.get(&id))
            .is_some_and(|f| f.besitzer == sid && f.zustand == Flottenzustand::ImOrbit);
        if freund && (mission != Mission::Angriff || self.tatsaechlicher_gegner(pid, sid).is_none())
        {
            return Err("Eigene oder verbündete Planeten erlauben nur Entsatz gegen eine feindliche Blockade".into());
        }
        if !freund && self.verbuendet(sid, p.besitzer) {
            return Err("Bündnis zuerst ausdrücklich beenden".into());
        }
        if mission != Mission::Angriff && self.spieler[sid as usize].stufe < 4 {
            return Err("Orbitbesetzung benötigt Zivilisationsstufe 4".into());
        }
        if mission == Mission::Kampfkolonisieren {
            if p.heimat {
                return Err("Der ursprüngliche Heimatplanet ist vor Übernahme geschützt".into());
            }
            if schiffe[Einheit::Kolonieschiff.idx()] < 1 {
                return Err("Kampfkolonisierung braucht ein Kolonieschiff".into());
            }
            if !self.koloniekapazitaet(sid, p.koord) {
                return Err("Koloniekapazität bereits belegt oder reserviert".into());
            }
        }
        if let Some(g) = self.tatsaechlicher_gegner(pid, sid) {
            self.gegner_angreifbar(sid, g)?;
        } else if eigene_blockade && mission == Mission::Kampfkolonisieren && !freund {
            self.gegner_angreifbar(sid, p.besitzer)?;
        } else {
            return Err("Kein feindlicher Gegner am Ziel".into());
        }
        Ok(())
    }
    pub fn flotte_versorgen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        a: Flugauftrag,
        versorgungsflotte: FlottenId,
    ) -> Result<String, String> {
        if !self.kolonisationsregeln_v2() {
            return Err("Flottenversorgung benötigt V4-Regeln".into());
        }
        let f = self
            .flotten
            .get(&versorgungsflotte)
            .ok_or("Versorgungsflotte fehlt")?;
        if f.besitzer != sid
            || f.zustand != Flottenzustand::ImOrbit
            || f.ziel != a.ziel
            || a.mission != Mission::Transport
        {
            return Err(
                "Versorgungsziel muss die gebundene eigene Orbitflotte am Flugziel sein".into(),
            );
        }
        let id = self.naechste_flotte;
        let result = self.flotte_senden(sid, rolle, a)?;
        self.kolonisation.transportziele.insert(
            id,
            Transportziel::Flotte {
                flotte: versorgungsflotte,
                empfaenger: sid,
            },
        );
        Ok(result)
    }
    fn transport_ankunft_v2(&mut self, fid: FlottenId) {
        let f = self.flotten[&fid].clone();
        let mut zugestellt = false;
        match self.kolonisation.transportziele.remove(&fid) {
            Some(Transportziel::Planet { planet, empfaenger }) => {
                let pid = planet as usize;
                if self
                    .planeten
                    .get(pid)
                    .is_some_and(|p| p.besitzer == empfaenger && p.koord == f.ziel)
                    && !self.blockiert_fuer(pid, empfaenger)
                {
                    self.abrechnen(pid);
                    for g in 0..GUETER {
                        self.planeten[pid].bestand[g] += f.ladung[g];
                    }
                    let wert = self.regeln.wert(&f.ladung);
                    if empfaenger != f.besitzer {
                        self.spieler[f.besitzer as usize].statistik.geschenkt += wert;
                        self.spieler[empfaenger as usize].statistik.erhalten += wert;
                    }
                    self.raten_neu(pid);
                    zugestellt = true;
                }
            }
            Some(Transportziel::Flotte { flotte, empfaenger }) => {
                if let Some(z) = self.flotten.get(&flotte).cloned().filter(|z| {
                    z.besitzer == empfaenger
                        && empfaenger == f.besitzer
                        && z.ziel == f.ziel
                        && z.zustand == Flottenzustand::ImOrbit
                        && z.id != fid
                }) {
                    let kap = self
                        .flugplan(z.besitzer, z.start as usize, z.ziel, &z.schiffe, 1000)
                        .map(|p| p.kapazitaet)
                        .unwrap_or(0);
                    let neue = z
                        .ladung
                        .iter()
                        .zip(f.ladung)
                        .map(|(a, b)| a.checked_add(b))
                        .collect::<Option<Vec<_>>>();
                    if let Some(neue) =
                        neue.filter(|n| n.iter().map(|v| *v as i128).sum::<i128>() <= kap as i128)
                    {
                        self.flotten
                            .get_mut(&flotte)
                            .unwrap()
                            .ladung
                            .copy_from_slice(&neue);
                        zugestellt = true;
                    }
                }
            }
            None => {}
        }
        if zugestellt {
            self.flotten.get_mut(&fid).unwrap().ladung = [0; GUETER];
        }
        self.fachereignis(
            if zugestellt {
                "transport_delivered"
            } else {
                "transport_returned"
            },
            f.besitzer,
            serde_json::json!({"fleet":fid,"target":f.ziel,"cargo":f.ladung}),
        );
        self.heimflug(fid);
    }
    pub fn verband_oeffnen(&mut self, sid: SpielerId, fid: FlottenId) -> Result<String, String> {
        let f = self.flotten.get_mut(&fid).ok_or("Flotte fehlt")?;
        if f.besitzer != sid
            || f.mission != Mission::Angriff
            || f.zustand != Flottenzustand::Hinflug
            || f.ankunft <= self.zeit
        {
            return Err(
                "Nur eigene ausfliegende Angriffsflotten können einen Verband öffnen".into(),
            );
        }
        if f.verband.is_some_and(|v| v != fid) {
            return Err("Flotte ist bereits einem anderen Verband beigetreten".into());
        }
        f.verband = Some(fid);
        Ok(format!("Angriffsverband {fid} für Allianzpartner geöffnet"))
    }

    /// Niemals beschleunigen: alle Teilnehmer erreichen das Ziel zur spätesten Ankunft.
    /// Die gesamte Verzögerung gegenüber dem ursprünglichen Flug bleibt <= 6 Stunden.
    pub fn verband_beitreten(
        &mut self,
        sid: SpielerId,
        fid: FlottenId,
        fuehrung: FlottenId,
    ) -> Result<String, String> {
        let f = self.flotten.get(&fid).ok_or("Beitrittsflotte fehlt")?;
        let leader = self.flotten.get(&fuehrung).ok_or("Führungsflotte fehlt")?;
        if fid == fuehrung || f.besitzer != sid || f.verband.is_some() {
            return Err("Nur eine eigene ungebundene Flotte kann beitreten".into());
        }
        if leader.verband != Some(fuehrung) {
            return Err("Führung hat keinen Verband geöffnet".into());
        }
        let gleiche_allianz = self.spieler[sid as usize].allianz.is_some()
            && self.spieler[sid as usize].allianz == self.spieler[leader.besitzer as usize].allianz;
        if sid != leader.besitzer && !gleiche_allianz {
            return Err("Verbandsangriffe erfordern dieselbe Allianz".into());
        }
        if [f, leader].iter().any(|x| {
            x.mission != Mission::Angriff
                || x.zustand != Flottenzustand::Hinflug
                || x.ankunft <= self.zeit
        }) || f.ziel != leader.ziel
        {
            return Err("Beide Flotten müssen im Hinflug dasselbe Angriffsziel haben".into());
        }
        let mut ids: Vec<_> = self
            .flotten
            .values()
            .filter(|x| x.verband == Some(fuehrung) && x.zustand == Flottenzustand::Hinflug)
            .map(|x| x.id)
            .collect();
        if ids.len() >= 16 {
            return Err("Ein Verband umfasst höchstens 16 Flotten".into());
        }
        ids.push(fid);
        let ankunft = ids.iter().map(|id| self.flotten[id].ankunft).max().unwrap();
        if ids
            .iter()
            .any(|id| ankunft - (self.flotten[id].abflug + self.flotten[id].flugdauer) > 6 * STUNDE)
        {
            return Err(
                "Verband würde ursprünglichen Flug um mehr als sechs Stunden verzögern".into(),
            );
        }
        for id in ids {
            let fl = self.flotten.get_mut(&id).unwrap();
            fl.verband = Some(fuehrung);
            fl.ankunft = ankunft;
            self.plane(ankunft, EreignisArt::FlotteAnkunft { flotte: id });
        }
        Ok(format!(
            "Flotte {fid} tritt Verband {fuehrung} bei; gemeinsame Ankunft {}",
            zeittext(ankunft)
        ))
    }

    pub fn flotte_zurueckrufen(
        &mut self,
        sid: SpielerId,
        fid: FlottenId,
    ) -> Result<String, String> {
        let jetzt = self.zeit;
        let Some(f) = self.flotten.get(&fid) else {
            if self.aufklaerungsregeln() {return Err("Keine eigene rückrufbare Flotte unter dieser Kennung".into());}
            return Err(format!("Flotte {fid} gibt es nicht"));
        };
        if f.besitzer != sid {
            if self.aufklaerungsregeln() {return Err("Keine eigene rückrufbare Flotte unter dieser Kennung".into());}
            return Err(format!("Flotte {fid} gehört dir nicht"));
        }
        if self.kolonisationsregeln_v2()
            && self
                .kolonisation
                .rueckwege
                .get(&fid)
                .is_some_and(|r| r.wartend)
        {
            return Err("Rückkehrflotte wartet bereits auf die Freigabe ihres Landeorbits".into());
        }
        match f.zustand {
            Flottenzustand::Rueckflug => Err(format!("Flotte {fid} ist schon auf dem Rückflug")),
            Flottenzustand::Hinflug => {
                let zurueck = jetzt + (jetzt - f.abflug).max(MINUTE);
                let start = f.start;
                let f = self.flotten.get_mut(&fid).unwrap();
                f.zustand = Flottenzustand::Rueckflug;
                f.verband = None;
                f.rueckkehr = zurueck;
                if self.kolonisationsregeln_v2() {
                    self.kolonisation.rueckwege.insert(
                        fid,
                        Rueckweg {
                            ziel: start,
                            wartend: false,
                        },
                    );
                }
                self.plane(zurueck, EreignisArt::FlotteRueckkehr { flotte: fid });
                Ok(format!(
                    "Flotte {fid} kehrt um, Rückkehr {}",
                    zeittext(zurueck)
                ))
            }
            Flottenzustand::ImOrbit => {
                let (mission, ziel, seit) = (f.mission, f.ziel, f.ankunft);
                if mission == Mission::Abbau {
                    self.abbau_ertrag(fid, jetzt - seit);
                }
                if let Some(zp) = self.belegung.get(&ziel).map(|x| *x as usize) {
                    if self.planeten[zp].blockade == Some(fid) {
                        self.planeten[zp].blockade = None;
                        let besitzer = self.planeten[zp].besitzer;
                        self.vorfall(
                            besitzer,
                            "blockade",
                            format!("Die Blockade von {ziel} wurde aufgehoben"),
                        );
                    }
                }
                self.heimflug(fid);
                Ok(format!("Flotte {fid} verlässt den Orbit von {ziel}"))
            }
        }
    }

    pub(crate) fn heimflug(&mut self, fid: FlottenId) {
        if self.kolonisationsregeln_v2() {
            let Some(f) = self.flotten.get(&fid).cloned() else {
                return;
            };
            if f.schiffe.iter().sum::<i64>() <= 0 {
                self.flotten.remove(&fid);
                return;
            }
            let besetzungen = self
                .kolonisation
                .besetzungen
                .iter()
                .filter(|(_, b)| b.flotte == fid)
                .map(|(pid, _)| *pid)
                .collect::<Vec<_>>();
            for pid in besetzungen {
                self.besetzung_abbrechen(pid, "Besetzungsflotte verlässt den Orbit");
            }
            let start = f.start as usize;
            if self.planeten[start].besitzer == f.besitzer {
                let retour = self.zeit + f.flugdauer;
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::Rueckflug;
                fl.verband = None;
                fl.rueckkehr = retour;
                self.kolonisation.rueckwege.insert(
                    fid,
                    Rueckweg {
                        ziel: f.start,
                        wartend: false,
                    },
                );
                self.plane(retour, EreignisArt::FlotteRueckkehr { flotte: fid });
            } else {
                self.rueckflug_umleiten(fid, f.ziel, self.spieler[f.besitzer as usize].heimat);
            }
            return;
        }
        let jetzt = self.zeit;
        let Some(f) = self.flotten.get_mut(&fid) else {
            return;
        };
        if f.schiffe.iter().sum::<i64>() <= 0 {
            self.flotten.remove(&fid);
            return;
        }
        f.zustand = Flottenzustand::Rueckflug;
        f.verband = None;
        f.rueckkehr = jetzt + f.flugdauer;
        let zurueck = f.rueckkehr;
        self.plane(zurueck, EreignisArt::FlotteRueckkehr { flotte: fid });
    }

    pub fn flotte_rueckkehr(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return;
        };
        if f.zustand != Flottenzustand::Rueckflug || f.rueckkehr != self.zeit {
            return;
        }
        if self.kolonisationsregeln_v2() {
            let pid = self
                .kolonisation
                .rueckwege
                .get(&fid)
                .map(|r| r.ziel)
                .unwrap_or(f.start) as usize;
            if self.planeten[pid].besitzer != f.besitzer {
                let home = self.spieler[f.besitzer as usize].heimat;
                self.rueckflug_umleiten(fid, self.planeten[pid].koord, home);
                return;
            }
            if self.blockiert_fuer(pid, f.besitzer) {
                let ende = self.zeit + self.regeln.fenster();
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::ImOrbit;
                fl.ziel = self.planeten[pid].koord;
                fl.orbit_ende = ende;
                self.kolonisation.rueckwege.insert(
                    fid,
                    Rueckweg {
                        ziel: pid as PlanetId,
                        wartend: true,
                    },
                );
                self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                self.fachereignis(
                    "return_waiting",
                    f.besitzer,
                    serde_json::json!({"fleet":fid,"planet":pid}),
                );
                return;
            }
            self.rueckflug_landen(fid, pid);
            return;
        }
        // Ist der Startplanet verloren, landet die Flotte auf der Heimatwelt.
        let mut pid = f.start as usize;
        if self.planeten[pid].besitzer != f.besitzer {
            pid = self.spieler[f.besitzer as usize].heimat as usize;
        }
        self.abrechnen(pid);
        let p = &mut self.planeten[pid];
        for e in 0..SCHIFFE {
            p.einheiten[e] += f.schiffe[e];
        }
        for g in 0..GUETER {
            p.bestand[g] += f.ladung[g];
        }
        p.bevoelkerung += f.siedler;
        let k = p.koord;
        self.flotten.remove(&fid);
        let fracht: Vec<String> = Gut::ALLE
            .iter()
            .filter(|g| f.ladung[g.idx()] >= M)
            .map(|g| format!("{} {}", ganz(f.ladung[g.idx()]), g))
            .collect();
        let text = if fracht.is_empty() {
            format!("Flotte {fid} ({}) ist auf {k} zurück", f.mission)
        } else {
            format!(
                "Flotte {fid} ({}) ist auf {k} zurück mit {}",
                f.mission,
                fracht.join(", ")
            )
        };
        self.vorfall(f.besitzer, "flotte", text);
        self.raten_neu(pid);
    }

    fn rueckflug_umleiten(&mut self, fid: FlottenId, von: Koord, ziel: PlanetId) {
        let f = self.flotten[&fid].clone();
        let k = self.planeten[ziel as usize].koord;
        let tempo = Einheit::ALLE[..SCHIFFE]
            .iter()
            .filter(|e| f.schiffe[e.idx()] > 0)
            .map(|e| self.tempo(f.besitzer, *e))
            .min()
            .unwrap_or(1);
        let dauer = self.flugdauer(self.entfernung(von, k), tempo, 1000);
        let ende = self.zeit + dauer;
        let fl = self.flotten.get_mut(&fid).unwrap();
        fl.zustand = Flottenzustand::Rueckflug;
        fl.verband = None;
        fl.rueckkehr = ende;
        fl.ziel = k;
        self.kolonisation.rueckwege.insert(
            fid,
            Rueckweg {
                ziel,
                wartend: false,
            },
        );
        self.plane(ende, EreignisArt::FlotteRueckkehr { flotte: fid });
        self.fachereignis(
            "return_redirected",
            f.besitzer,
            serde_json::json!({"fleet":fid,"from":von,"target":k,"arrival":ende}),
        );
    }
    fn rueckflug_landen(&mut self, fid: FlottenId, pid: usize) {
        let f = self.flotten[&fid].clone();
        self.abrechnen(pid);
        let p = &mut self.planeten[pid];
        for e in 0..SCHIFFE {
            p.einheiten[e] += f.schiffe[e];
        }
        for g in 0..GUETER {
            p.bestand[g] += f.ladung[g];
        }
        p.bevoelkerung += f.siedler;
        self.flotten.remove(&fid);
        self.kolonisation.rueckwege.remove(&fid);
        self.kolonisation.transportziele.remove(&fid);
        self.fachereignis(
            "fleet_returned",
            f.besitzer,
            serde_json::json!({"fleet":fid,"planet":pid}),
        );
        self.vorfall(
            f.besitzer,
            "flotte",
            format!("Flotte {fid} auf {} zurück", self.planeten[pid].koord),
        );
        self.raten_neu(pid);
    }
    fn abbau_ertrag(&mut self, fid: FlottenId, dauer: i64) {
        let r = self.regeln.clone();
        let Some(f) = self.flotten.get(&fid) else {
            return;
        };
        let Some(sys) = self.system(f.ziel) else {
            return;
        };
        let schiffe = f.schiffe[Einheit::Bergbauschiff.idx()];
        let erz = anteil(
            anteil(schiffe * r.flug.abbau_erz_je_stunde * M, dauer, STUNDE),
            sys.reich_erz,
            1000,
        );
        let kristall = anteil(
            anteil(schiffe * r.flug.abbau_kristall_je_stunde * M, dauer, STUNDE),
            sys.reich_kristall,
            1000,
        );
        let Ok(plan) = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000) else {
            return;
        };
        let frei = (plan.kapazitaet - f.ladung.iter().sum::<i64>()).max(0);
        let summe = erz + kristall;
        let (erz, kristall) = if summe > frei {
            (anteil(erz, frei, summe), anteil(kristall, frei, summe))
        } else {
            (erz, kristall)
        };
        let f = self.flotten.get_mut(&fid).unwrap();
        f.ladung[Gut::Erz.idx()] += erz;
        f.ladung[Gut::Kristall.idx()] += kristall;
    }

    pub fn orbit_ende(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid) else {
            return;
        };
        if f.zustand != Flottenzustand::ImOrbit || f.orbit_ende != self.zeit {
            return;
        }
        if self.kolonisationsregeln_v2()
            && self
                .kolonisation
                .rueckwege
                .get(&fid)
                .is_some_and(|r| r.wartend)
        {
            let fl = self.flotten.get_mut(&fid).unwrap();
            fl.zustand = Flottenzustand::Rueckflug;
            fl.rueckkehr = self.zeit;
            self.flotte_rueckkehr(fid);
            return;
        }
        if f.mission == Mission::Abbau {
            let dauer = f.haltedauer;
            self.abbau_ertrag(fid, dauer);
        }
        self.heimflug(fid);
    }

    pub fn flotte_ankunft(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return;
        };
        if f.zustand != Flottenzustand::Hinflug || f.ankunft != self.zeit {
            return;
        }
        if f.mission == Mission::Angriff && f.verband.is_some() {
            self.verbandskampf(fid);
            return;
        }
        let jetzt = self.zeit;
        let ziel_pid = self.belegung.get(&f.ziel).map(|x| *x as usize);
        match f.mission {
            Mission::SystemErkunden => self.system_erkunden_ankunft(fid),
            Mission::FlottenSpionage => self.flotten_spionage_ankunft(fid),
            Mission::Saven => {
                if f.haltedauer > 0 {
                    let end = jetzt + f.haltedauer;
                    let fleet = self.flotten.get_mut(&fid).unwrap();
                    fleet.zustand = Flottenzustand::ImOrbit;
                    fleet.orbit_ende = end;
                    self.plane(end, EreignisArt::OrbitEnde { flotte:fid });
                } else { self.heimflug(fid); }
            }
            Mission::Transport => {
                if self.kolonisationsregeln_v2() {
                    self.transport_ankunft_v2(fid);
                    return;
                }
                if let Some(zp) = ziel_pid {
                    if self.blockiert_fuer(zp, f.besitzer) {
                        self.vorfall(
                            f.besitzer,
                            "flotte",
                            format!("Transport nach {} kehrt um: Blockade", f.ziel),
                        );
                    } else {
                        self.abrechnen(zp);
                        let empfaenger = self.planeten[zp].besitzer;
                        for g in 0..GUETER {
                            self.planeten[zp].bestand[g] += f.ladung[g];
                        }
                        let wert = self.regeln.wert(&f.ladung);
                        self.flotten.get_mut(&fid).unwrap().ladung = [0; GUETER];
                        if empfaenger != f.besitzer && wert > 0 {
                            let (von, an) = (
                                self.spieler[f.besitzer as usize].name.clone(),
                                self.spieler[empfaenger as usize].name.clone(),
                            );
                            self.spieler[f.besitzer as usize].statistik.geschenkt += wert;
                            self.spieler[empfaenger as usize].statistik.erhalten += wert;
                            self.vorfall(
                                empfaenger,
                                "lieferung",
                                format!(
                                    "Lieferung von {von} auf {} im Wert von {}",
                                    f.ziel,
                                    ganz(wert)
                                ),
                            );
                            self.vorfall(
                                f.besitzer,
                                "lieferung",
                                format!("Lieferung an {an} auf {} zugestellt", f.ziel),
                            );
                            self.wecke(empfaenger, Rolle::Diplomat, "Lieferung erhalten", false);
                        }
                        self.raten_neu(zp);
                    }
                }
                self.heimflug(fid);
            }
            Mission::Stationieren => match ziel_pid {
                Some(zp)
                    if self.planeten[zp].besitzer == f.besitzer
                        && !self.blockiert_fuer(zp, f.besitzer) =>
                {
                    self.abrechnen(zp);
                    let p = &mut self.planeten[zp];
                    for e in 0..SCHIFFE {
                        p.einheiten[e] += f.schiffe[e];
                    }
                    for g in 0..GUETER {
                        p.bestand[g] += f.ladung[g];
                    }
                    self.flotten.remove(&fid);
                    self.vorfall(
                        f.besitzer,
                        "flotte",
                        format!("Flotte {fid} auf {} stationiert", f.ziel),
                    );
                    self.raten_neu(zp);
                }
                _ => self.heimflug(fid),
            },
            Mission::Halten => match ziel_pid {
                Some(zp)
                    if self.verbuendet(f.besitzer, self.planeten[zp].besitzer)
                        && !self.blockiert_fuer(zp, f.besitzer) =>
                {
                    let fl = self.flotten.get_mut(&fid).unwrap();
                    fl.zustand = Flottenzustand::ImOrbit;
                    fl.orbit_ende = jetzt + fl.haltedauer;
                    let ende = fl.orbit_ende;
                    self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
                    let partner = self.planeten[zp].besitzer;
                    let name = self.spieler[f.besitzer as usize].name.clone();
                    self.vorfall(
                        partner,
                        "flotte",
                        format!(
                            "Flotte von {name} hält auf {} bis {}",
                            f.ziel,
                            zeittext(ende)
                        ),
                    );
                }
                _ => self.heimflug(fid),
            },
            Mission::Spionage => self.spionieren(fid),
            Mission::Angriff
            | Mission::Blockade
            | Mission::Invasion
            | Mission::Bombardieren
            | Mission::Kampfkolonisieren => self.kampf_am_planeten(fid),
            Mission::Kolonisieren => self.kolonisieren(fid),
            Mission::Recyceln => {
                if let Some(t) = self.truemmer.get(&f.ziel).copied() {
                    let r = self.regeln.clone();
                    let sp = &self.spieler[f.besitzer as usize];
                    let logistik = 1.0
                        + r.flug.logistik_bonus * sp.forschung[Forschung::Logistik.idx()] as f64;
                    let kap = mal(
                        f.schiffe[Einheit::Recycler.idx()] * r.einh(Einheit::Recycler).ladung * M,
                        r.volk(sp.volk).ladung * logistik,
                    );
                    let frei = (kap - f.ladung.iter().sum::<i64>()).max(0);
                    let summe = t[0] + t[1];
                    let (erz, kristall) = if summe > frei {
                        (anteil(t[0], frei, summe), anteil(t[1], frei, summe))
                    } else {
                        (t[0], t[1])
                    };
                    let fl = self.flotten.get_mut(&fid).unwrap();
                    fl.ladung[Gut::Erz.idx()] += erz;
                    fl.ladung[Gut::Kristall.idx()] += kristall;
                    let rest = [t[0] - erz, t[1] - kristall];
                    if rest[0] + rest[1] < M {
                        self.truemmer.remove(&f.ziel);
                    } else {
                        self.truemmer.insert(f.ziel, rest);
                    }
                }
                self.heimflug(fid);
            }
            Mission::Abbau => {
                let fl = self.flotten.get_mut(&fid).unwrap();
                fl.zustand = Flottenzustand::ImOrbit;
                fl.orbit_ende = jetzt + fl.haltedauer;
                let ende = fl.orbit_ende;
                self.plane(ende, EreignisArt::OrbitEnde { flotte: fid });
            }
        }
    }

    // ---------------------------------------------------------------- Kampf

    /// Trägt einen Kampf aus und schreibt Überlebende, Trümmer, Bericht und Statistik zurück.
    fn gefecht(&mut self, fids: &[FlottenId], parteien: &[Partei], ort: Koord) -> Sieger {
        let r = self.regeln.clone();
        let nr = self.kampf_nr;
        self.kampf_nr += 1;
        let mut rng = strom(self.startwert, KANAL_KAMPF, nr);
        let f = self.flotten[&fids[0]].clone();
        let flotten: Vec<_> = fids.iter().map(|id| self.flotten[id].clone()).collect();
        let mut koloniestand = std::collections::BTreeMap::new();
        if self.kolonisationsregeln_v2() {
            for fl in &flotten {
                koloniestand.insert(fl.id, fl.schiffe[Einheit::Kolonieschiff.idx()]);
            }
            for p in parteien {
                if let Partei::Flotte(id) = p {
                    koloniestand
                        .insert(*id, self.flotten[id].schiffe[Einheit::Kolonieschiff.idx()]);
                }
            }
        }
        let mut ang_e = [0i64; EINHEITEN];
        let ang: Vec<_> = flotten
            .iter()
            .map(|fl| {
                let mut e = [0; EINHEITEN];
                e[..SCHIFFE].copy_from_slice(&fl.schiffe);
                for i in 0..EINHEITEN {
                    ang_e[i] += e[i];
                }
                Gruppe::neu(&r, &self.spieler[fl.besitzer as usize], e)
            })
            .collect();
        let mut vert = Vec::new();
        for p in parteien {
            match p {
                Partei::Planet(zp) => {
                    let pl = &self.planeten[*zp];
                    vert.push(Gruppe::neu(
                        &r,
                        &self.spieler[pl.besitzer as usize],
                        pl.einheiten,
                    ));
                }
                Partei::Flotte(id) => {
                    let fl = &self.flotten[id];
                    let mut e = [0i64; EINHEITEN];
                    e[..SCHIFFE].copy_from_slice(&fl.schiffe);
                    vert.push(Gruppe::neu(&r, &self.spieler[fl.besitzer as usize], e));
                }
            }
        }
        let erg = kampf(&r, &ang, &vert, &mut rng);

        let mut truemmer = [0i64; 2];
        let mut truemmer_dazu = |e: usize, verloren: i64, volk: Volk| {
            let k = r.kosten_einheit(Einheit::ALLE[e], volk);
            truemmer[0] += mal(k[Gut::Erz.idx()] * verloren, r.kampf.truemmer_anteil);
            truemmer[1] += mal(k[Gut::Kristall.idx()] * verloren, r.kampf.truemmer_anteil);
        };
        let wert_von =
            |e: usize, n: i64, volk: Volk| n * r.wert(&r.kosten_einheit(Einheit::ALLE[e], volk));

        // Angreifer.
        let mut ang_nachher = [0i64; EINHEITEN];
        let mut ang_verluste = Vec::new();
        let mut angreifer_gruppen = Vec::new();
        for (gi, fl) in flotten.iter().enumerate() {
            let ang_volk = self.spieler[fl.besitzer as usize].volk;
            let mut verlust = 0;
            for e in 0..SCHIFFE {
                let rest = erg.angreifer[gi][e];
                let verloren = fl.schiffe[e] - rest;
                ang_nachher[e] += rest;
                if verloren > 0 {
                    truemmer_dazu(e, verloren, ang_volk);
                    verlust += wert_von(e, verloren, ang_volk);
                }
            }
            ang_verluste.push((fl.besitzer, verlust));
            angreifer_gruppen.push(Kampfteilnehmer {
                flotte: fl.id,
                spieler: fl.besitzer,
                vorher: ang[gi].einheiten,
                nachher: erg.angreifer[gi],
            });
            if erg.angreifer[gi].iter().sum::<i64>() == 0 {
                self.flotten.remove(&fl.id);
            } else {
                self.flotten
                    .get_mut(&fl.id)
                    .unwrap()
                    .schiffe
                    .copy_from_slice(&erg.angreifer[gi][..SCHIFFE]);
            }
        }

        // Verteidiger.
        let mut vert_vorher = [0i64; EINHEITEN];
        let mut vert_nachher = [0i64; EINHEITEN];
        let mut vert_spieler: Vec<SpielerId> = Vec::new();
        let mut vert_verlust: Vec<(SpielerId, i64)> = Vec::new();
        for (gi, p) in parteien.iter().enumerate() {
            let sid = vert[gi].spieler;
            let volk = self.spieler[sid as usize].volk;
            let mut verlust = 0i64;
            if !vert_spieler.contains(&sid) {
                vert_spieler.push(sid);
            }
            match p {
                Partei::Planet(zp) => {
                    for e in 0..EINHEITEN {
                        let vorher = vert[gi].einheiten[e];
                        let mut rest = erg.verteidiger[gi][e];
                        let verloren = vorher - rest;
                        vert_vorher[e] += vorher;
                        if verloren > 0 {
                            if e < SCHIFFE {
                                truemmer_dazu(e, verloren, volk);
                                verlust += wert_von(e, verloren, volk);
                            } else {
                                // Zerstörte Verteidigung wird zum Teil wiederhergestellt.
                                let neu = if self.kolonisation.aktiv
                                    && matches!(
                                        f.mission,
                                        Mission::Bombardieren | Mission::Kampfkolonisieren
                                    ) {
                                    0
                                } else {
                                    ((verloren as i128 * fx(r.kampf.verteidigung_wiederaufbau)
                                        + FX / 2)
                                        / FX) as i64
                                };
                                rest += neu;
                                verlust += wert_von(e, verloren - neu, volk);
                            }
                        }
                        vert_nachher[e] += rest;
                        self.planeten[*zp].einheiten[e] = rest;
                    }
                }
                Partei::Flotte(id) => {
                    let mut neu = [0i64; SCHIFFE];
                    for e in 0..SCHIFFE {
                        let vorher = vert[gi].einheiten[e];
                        let rest = erg.verteidiger[gi][e];
                        vert_vorher[e] += vorher;
                        vert_nachher[e] += rest;
                        neu[e] = rest;
                        if vorher > rest {
                            truemmer_dazu(e, vorher - rest, volk);
                            verlust += wert_von(e, vorher - rest, volk);
                        }
                    }
                    if neu.iter().sum::<i64>() == 0 {
                        self.flotten.remove(id);
                    } else {
                        self.flotten.get_mut(id).unwrap().schiffe = neu;
                    }
                }
            }
            vert_verlust.push((sid, verlust));
        }

        if truemmer[0] + truemmer[1] > 0 {
            let t = self.truemmer.entry(ort).or_insert([0, 0]);
            t[0] += truemmer[0];
            t[1] += truemmer[1];
        }
        self.kampfberichte.push(Kampfbericht {
            nr,
            zeit: self.zeit,
            ort,
            mission: f.mission,
            angreifer: f.besitzer,
            angreifer_gruppen,
            verteidiger: vert_spieler.clone(),
            runden: erg.runden,
            sieger: erg.sieger,
            ang_vorher: ang_e,
            ang_nachher,
            vert_vorher,
            vert_nachher,
            beute: [0; GUETER],
            truemmer,
        });

        let ang_name = self.spieler[f.besitzer as usize].name.clone();
        let ausgang = match erg.sieger {
            Sieger::Angreifer => "Sieg des Angreifers",
            Sieger::Verteidiger => "Sieg des Verteidigers",
            Sieger::Unentschieden => "unentschieden",
        };
        for (sid, verlust) in ang_verluste {
            let st = &mut self.spieler[sid as usize].statistik;
            st.angriffe += 1;
            st.verluste += verlust;
            self.vorfall(sid,"kampf",format!("Kampf {nr} bei {ort}: {ausgang} nach {} Runden, eigene Verluste im Wert von {}",erg.runden,ganz(verlust)));
            self.wecke(sid, Rolle::Feldherr, "Kampfbericht", false);
            self.wecke(sid, Rolle::Stratege, "Kampfbericht", false);
        }
        for (sid, verlust) in vert_verlust {
            let st = &mut self.spieler[sid as usize].statistik;
            st.verteidigungen += 1;
            st.verluste += verlust;
            self.vorfall(
                sid,
                "kampf",
                format!("Kampf {nr} bei {ort}: Angriff von {ang_name}, {ausgang} nach {} Runden, eigene Verluste im Wert von {}", erg.runden, ganz(verlust)),
            );
            self.wecke(sid, Rolle::Feldherr, "Kampfbericht", true);
            self.wecke(sid, Rolle::Stratege, "Kampfbericht", false);
        }
        for (id, vorher) in koloniestand {
            let nachher = self
                .flotten
                .get(&id)
                .map(|f| f.schiffe[Einheit::Kolonieschiff.idx()])
                .unwrap_or(0);
            if nachher < vorher {
                let pids = self
                    .kolonisation
                    .besetzungen
                    .iter()
                    .filter(|(_, b)| b.flotte == id)
                    .map(|(pid, _)| *pid)
                    .collect::<Vec<_>>();
                for pid in pids {
                    self.besetzung_abbrechen(pid, "Kolonieschiffbestand im Gefecht gesunken");
                }
            }
        }
        self.fachereignis(
            "combat_resolved",
            f.besitzer,
            serde_json::json!({"combat":nr,"target":ort,"attackers":fids,"winner":erg.sieger}),
        );
        erg.sieger
    }

    fn kampf_am_planeten(&mut self, fid: FlottenId) {
        if self.kolonisationsregeln_v2() {
            self.kampf_am_planeten_v2(&[fid]);
            return;
        }
        let f = self.flotten[&fid].clone();
        let Some(zp) = self.belegung.get(&f.ziel).map(|x| *x as usize) else {
            self.heimflug(fid);
            return;
        };
        self.abrechnen(zp);
        let ziel_besitzer = self.planeten[zp].besitzer;

        // Hält eine fremde Flotte den Orbit, geht der Kampf gegen sie.
        if let Some(bf) = self.planeten[zp].blockade {
            match self.flotten.get(&bf).map(|b| b.besitzer) {
                Some(b_sid) if b_sid != f.besitzer => {
                    let sieger = self.gefecht(&[fid], &[Partei::Flotte(bf)], f.ziel);
                    if !self.flotten.contains_key(&bf) {
                        self.planeten[zp].blockade = None;
                        self.vorfall(
                            ziel_besitzer,
                            "blockade",
                            format!("Die Blockade von {} ist gebrochen", f.ziel),
                        );
                    }
                    let _ = sieger;
                    self.heimflug(fid);
                    return;
                }
                Some(_) => {
                    if self.kolonisation.aktiv
                        && f.mission == Mission::Kampfkolonisieren
                        && self.flotten[&bf].besitzer == f.besitzer
                        && !self.planeten[zp].heimat
                    {
                        if self.flotten[&bf]
                            .schiffe
                            .iter()
                            .zip(f.schiffe)
                            .any(|(a, b)| a.checked_add(b).is_none())
                            || self.flotten[&bf]
                                .ladung
                                .iter()
                                .zip(f.ladung)
                                .any(|(a, b)| a.checked_add(b).is_none())
                        {
                            self.heimflug(fid);
                            return;
                        }
                        let block = self.flotten.get_mut(&bf).unwrap();
                        for e in 0..SCHIFFE {
                            block.schiffe[e] += f.schiffe[e];
                        }
                        for g in 0..GUETER {
                            block.ladung[g] += f.ladung[g];
                        }
                        block.mission = Mission::Kampfkolonisieren;
                        self.flotten.remove(&fid);
                        self.besetzung_beginnen(zp, bf);
                        return;
                    }
                    self.heimflug(fid);
                    return;
                }
                None => self.planeten[zp].blockade = None,
            }
        }
        if ziel_besitzer == f.besitzer {
            self.heimflug(fid);
            return;
        }

        self.bruch_pruefen(f.besitzer, ziel_besitzer);
        if !self.spieler[f.besitzer as usize]
            .angegriffen
            .contains(&ziel_besitzer)
        {
            self.spieler[f.besitzer as usize]
                .angegriffen
                .push(ziel_besitzer);
        }
        let mut parteien = vec![Partei::Planet(zp)];
        for fl in self.flotten.values() {
            if fl.zustand == Flottenzustand::ImOrbit
                && fl.mission == Mission::Halten
                && fl.ziel == f.ziel
            {
                parteien.push(Partei::Flotte(fl.id));
            }
        }
        let sieger = self.gefecht(&[fid], &parteien, f.ziel);
        if sieger == Sieger::Angreifer && self.flotten.contains_key(&fid) {
            match f.mission {
                Mission::Angriff => {
                    self.pluendern(fid, zp);
                    self.heimflug(fid);
                }
                _ => {
                    self.flotten.get_mut(&fid).unwrap().zustand = Flottenzustand::ImOrbit;
                    self.planeten[zp].blockade = Some(fid);
                    let name = self.spieler[f.besitzer as usize].name.clone();
                    let art = if f.mission == Mission::Invasion {
                        "belagert"
                    } else {
                        "blockiert"
                    };
                    self.vorfall(
                        ziel_besitzer,
                        "blockade",
                        format!("{name} {art} {}", f.ziel),
                    );
                    self.vorfall(
                        f.besitzer,
                        "blockade",
                        format!("Flotte {fid} hält den Orbit von {}", f.ziel),
                    );
                    self.wecke(ziel_besitzer, Rolle::Stratege, "Blockade", true);
                }
            }
        } else {
            self.heimflug(fid);
        }
        if sieger == Sieger::Angreifer
            && self.flotten.contains_key(&fid)
            && matches!(
                f.mission,
                Mission::Bombardieren | Mission::Kampfkolonisieren
            )
        {
            self.bombardement_abschluss(zp, fid);
        }
        self.raten_neu(zp);
        self.punkte_neu();
    }

    fn planet_verteidiger_v2(&self, pid: usize, fid: FlottenId) -> Vec<Partei> {
        let p = &self.planeten[pid];
        let mut out = vec![Partei::Planet(pid)];
        for f in self.flotten.values() {
            if f.id != fid
                && f.zustand == Flottenzustand::ImOrbit
                && f.mission == Mission::Halten
                && f.ziel == p.koord
                && (f.besitzer == p.besitzer || self.verbuendet(f.besitzer, p.besitzer))
                && self.bewaffnet(&f.schiffe)
            {
                out.push(Partei::Flotte(f.id));
            }
        }
        out
    }
    pub(crate) fn besetzung_verteidigung_bekaempfen(&mut self, pid: usize, fid: FlottenId) -> bool {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return false;
        };
        let parteien = self.planet_verteidiger_v2(pid, fid);
        if !self.bewaffnet(&self.planeten[pid].einheiten) && parteien.len() == 1 {
            return true;
        }
        // A current ally in the defending orbit must never be shot automatically.
        if parteien.iter().any(|p|matches!(p,Partei::Flotte(id) if self.verbuendet(f.besitzer,self.flotten[id].besitzer))) {return false;}
        self.abrechnen(pid);
        let gewonnen =
            self.gefecht(&[fid], &parteien, self.planeten[pid].koord) == Sieger::Angreifer;
        if !gewonnen && self.flotten.contains_key(&fid) {
            self.planeten[pid].blockade = None;
            self.heimflug(fid);
        }
        if !self.flotten.contains_key(&fid) {
            self.planeten[pid].blockade = None;
        }
        gewonnen && self.flotten.contains_key(&fid)
    }
    /// Shared dispatch for one fleet and a synchronized attack group.
    fn kampf_am_planeten_v2(&mut self, ids: &[FlottenId]) {
        if ids.is_empty() {
            return;
        }
        let f = self.flotten[&ids[0]].clone();
        let Some(pid) = self.belegung.get(&f.ziel).map(|p| *p as usize) else {
            for id in ids {
                self.heimflug(*id);
            }
            return;
        };
        self.abrechnen(pid);
        let owner = self.planeten[pid].besitzer;
        let freundplanet = ids.iter().any(|id| {
            self.flotten[id].besitzer == owner || self.verbuendet(self.flotten[id].besitzer, owner)
        });
        let block = self.planeten[pid].blockade.filter(|id| {
            self.flotten
                .get(id)
                .is_some_and(|b| b.zustand == Flottenzustand::ImOrbit)
        });
        if block.is_none() {
            self.planeten[pid].blockade = None;
        }
        if let Some(bf) = block {
            let b = self.flotten[&bf].clone();
            if ids.len() == 1
                && b.besitzer == f.besitzer
                && !freundplanet
                && f.mission == Mission::Kampfkolonisieren
                && !self.planeten[pid].heimat
            {
                if self.gegner_angreifbar(f.besitzer, owner).is_err() {
                    self.heimflug(f.id);
                    return;
                }
                if b.schiffe
                    .iter()
                    .zip(f.schiffe)
                    .any(|(a, v)| a.checked_add(v).is_none_or(|n| n > 1_000_000))
                    || b.ladung
                        .iter()
                        .zip(f.ladung)
                        .any(|(a, v)| a.checked_add(v).is_none())
                {
                    self.heimflug(f.id);
                    return;
                }
                let block = self.flotten.get_mut(&bf).unwrap();
                for e in 0..SCHIFFE {
                    block.schiffe[e] += f.schiffe[e];
                }
                for g in 0..GUETER {
                    block.ladung[g] += f.ladung[g];
                }
                block.mission = Mission::Kampfkolonisieren;
                self.flotten.remove(&f.id);
                self.fachereignis(
                    "blockade_reinforced",
                    f.besitzer,
                    serde_json::json!({"fleet":bf,"reinforcement":f.id}),
                );
                // Existing occupation clocks remain intact; an earlier casualty already removed them.
                self.besetzung_beginnen(pid, bf);
                return;
            }
            if ids.iter().any(|id| {
                self.gegner_angreifbar(self.flotten[id].besitzer, b.besitzer)
                    .is_err()
            }) {
                for id in ids {
                    self.heimflug(*id);
                }
                return;
            }
            if freundplanet
                && ids
                    .iter()
                    .any(|id| self.flotten[id].mission != Mission::Angriff)
            {
                for id in ids {
                    self.heimflug(*id);
                }
                return;
            }
            for id in ids {
                let sid = self.flotten[id].besitzer;
                self.bruch_pruefen(sid, b.besitzer);
                if !self.spieler[sid as usize].angegriffen.contains(&b.besitzer) {
                    self.spieler[sid as usize].angegriffen.push(b.besitzer);
                }
            }
            self.gefecht(ids, &[Partei::Flotte(bf)], f.ziel);
            if !self.flotten.contains_key(&bf) {
                self.planeten[pid].blockade = None;
                self.fachereignis(
                    "blockade_broken",
                    owner,
                    serde_json::json!({"planet":pid,"fleet":bf}),
                );
            }
            for id in ids {
                self.heimflug(*id);
            }
            self.raten_neu(pid);
            self.punkte_neu();
            return;
        }
        if freundplanet
            || ids.iter().any(|id| {
                self.gegner_angreifbar(self.flotten[id].besitzer, owner)
                    .is_err()
            })
        {
            for id in ids {
                self.heimflug(*id);
            }
            return;
        }
        let parteien = self.planet_verteidiger_v2(pid, f.id);
        if parteien.iter().any(|p|matches!(p,Partei::Flotte(bf) if ids.iter().any(|id|self.flotten[id].besitzer==self.flotten[bf].besitzer || self.verbuendet(self.flotten[id].besitzer,self.flotten[bf].besitzer)))) {for id in ids {self.heimflug(*id);}return;}
        for id in ids {
            let sid = self.flotten[id].besitzer;
            self.bruch_pruefen(sid, owner);
            if !self.spieler[sid as usize].angegriffen.contains(&owner) {
                self.spieler[sid as usize].angegriffen.push(owner);
            }
        }
        let sieger = self.gefecht(ids, &parteien, f.ziel);
        if sieger == Sieger::Angreifer && f.mission == Mission::Angriff {
            if ids.len() > 1 {
                self.verband_pluendern(ids, pid);
            } else if self.flotten.contains_key(&f.id) {
                self.pluendern(f.id, pid);
            }
        }
        if sieger == Sieger::Angreifer
            && f.mission != Mission::Angriff
            && self.flotten.contains_key(&f.id)
        {
            self.flotten.get_mut(&f.id).unwrap().zustand = Flottenzustand::ImOrbit;
            self.planeten[pid].blockade = Some(f.id);
            self.fachereignis(
                "blockade_started",
                f.besitzer,
                serde_json::json!({"planet":pid,"fleet":f.id}),
            );
            if matches!(
                f.mission,
                Mission::Bombardieren | Mission::Kampfkolonisieren
            ) {
                self.bombardement_abschluss(pid, f.id);
            }
        } else {
            for id in ids {
                self.heimflug(*id);
            }
        }
        self.raten_neu(pid);
        self.punkte_neu();
    }
    fn verbandskampf(&mut self, fid: FlottenId) {
        let f = self.flotten[&fid].clone();
        let ids: Vec<_> = self
            .flotten
            .values()
            .filter(|fl| {
                fl.verband == f.verband
                    && fl.mission == Mission::Angriff
                    && fl.zustand == Flottenzustand::Hinflug
                    && fl.ankunft == self.zeit
                    && fl.ziel == f.ziel
            })
            .map(|fl| fl.id)
            .collect();
        if self.kolonisationsregeln_v2() {
            self.kampf_am_planeten_v2(&ids);
            return;
        }
        let Some(zp) = self.belegung.get(&f.ziel).map(|x| *x as usize) else {
            for id in ids {
                self.heimflug(id);
            }
            return;
        };
        self.abrechnen(zp);
        let ziel_sid = self.planeten[zp].besitzer;
        // Ein inzwischen übernommenes eigenes oder verbündetes Ziel wird nicht beschossen.
        if ids.iter().any(|id| {
            let sid = self.flotten[id].besitzer;
            sid == ziel_sid || self.verbuendet(sid, ziel_sid)
        }) {
            for id in ids {
                self.heimflug(id);
            }
            return;
        }
        let mut parteien = Vec::new();
        let mut gegen_blockade = false;
        if let Some(bf) = self.planeten[zp].blockade {
            if let Some(blockade) = self.flotten.get(&bf) {
                if ids.iter().any(|id| {
                    self.flotten[id].besitzer == blockade.besitzer
                        || self.verbuendet(self.flotten[id].besitzer, blockade.besitzer)
                }) {
                    for id in ids {
                        self.heimflug(id);
                    }
                    return;
                }
                parteien.push(Partei::Flotte(bf));
                gegen_blockade = true;
            } else {
                self.planeten[zp].blockade = None;
            }
        }
        if !gegen_blockade {
            parteien.push(Partei::Planet(zp));
            for fl in self.flotten.values() {
                if fl.zustand == Flottenzustand::ImOrbit
                    && fl.mission == Mission::Halten
                    && fl.ziel == f.ziel
                {
                    parteien.push(Partei::Flotte(fl.id));
                }
            }
            let mut besitzer = std::collections::BTreeSet::new();
            for id in &ids {
                besitzer.insert(self.flotten[id].besitzer);
            }
            for sid in besitzer {
                self.bruch_pruefen(sid, ziel_sid);
                if !self.spieler[sid as usize].angegriffen.contains(&ziel_sid) {
                    self.spieler[sid as usize].angegriffen.push(ziel_sid);
                }
            }
        }
        let sieger = self.gefecht(&ids, &parteien, f.ziel);
        if gegen_blockade {
            if self.planeten[zp]
                .blockade
                .is_some_and(|id| !self.flotten.contains_key(&id))
            {
                self.planeten[zp].blockade = None;
            }
        } else if sieger == Sieger::Angreifer {
            self.verband_pluendern(&ids, zp);
        }
        for id in ids {
            self.heimflug(id);
        }
        self.raten_neu(zp);
        self.punkte_neu();
    }

    /// Ein gemeinsamer Beutepool, einmaliger Bunkerschutz und eine einmalige Moralwirkung.
    /// Volksquoten werden nach freiem überlebendem Frachtraum gewichtet.
    fn verband_pluendern(&mut self, ids: &[FlottenId], zp: usize) {
        let r = self.regeln.clone();
        let mut traeger = Vec::new();
        for id in ids {
            if let Some(f) = self.flotten.get(id) {
                if let Ok(plan) =
                    self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000)
                {
                    let frei = (plan.kapazitaet - f.ladung.iter().sum::<i64>()).max(0);
                    if frei > 0 {
                        let quote = r
                            .volk(self.spieler[f.besitzer as usize].volk)
                            .pluenderquote
                            .unwrap_or(r.kampf.pluenderquote);
                        traeger.push((*id, f.besitzer, frei, fx(quote)));
                    }
                }
            }
        }
        let gesamt: i64 = traeger.iter().map(|t| t.2).sum();
        if gesamt == 0 {
            return;
        }
        let quote: i128 = traeger.iter().map(|t| t.2 as i128 * t.3).sum::<i128>() / gesamt as i128;
        let schutz = r.bunkerschutz(self.planeten[zp].gebaeude[Gebaeude::Bunker.idx()]);
        let mut pool = [0; GUETER];
        for g in [
            Gut::Erz,
            Gut::Kristall,
            Gut::Deuterium,
            Gut::Nahrung,
            Gut::Legierung,
            Gut::Elektronik,
            Gut::Konsumgut,
            Gut::Xenokristall,
        ] {
            pool[g.idx()] =
                (((self.planeten[zp].bestand[g.idx()] - schutz[g.idx()]).max(0) as i128 * quote)
                    / FX) as i64;
        }
        let summe: i64 = pool.iter().sum();
        if summe > gesamt {
            for g in &mut pool {
                *g = anteil(*g, gesamt, summe);
            }
        }
        let mut ladungen = vec![[0; GUETER]; traeger.len()];
        let mut frei: Vec<_> = traeger.iter().map(|t| t.2).collect();
        for g in 0..GUETER {
            let mut rest = pool[g];
            for i in 0..traeger.len() {
                let n = anteil(pool[g], traeger[i].2, gesamt).min(frei[i]);
                ladungen[i][g] = n;
                frei[i] -= n;
                rest -= n;
            }
            for i in 0..traeger.len() {
                let n = rest.min(frei[i]);
                ladungen[i][g] += n;
                frei[i] -= n;
                rest -= n;
            }
            debug_assert_eq!(rest, 0);
            self.planeten[zp].bestand[g] -= pool[g];
        }
        for (i, (id, sid, _, _)) in traeger.iter().enumerate() {
            for g in 0..GUETER {
                self.flotten.get_mut(id).unwrap().ladung[g] += ladungen[i][g];
            }
            let wert = r.wert(&ladungen[i]);
            self.spieler[*sid as usize].statistik.beute += wert;
            self.vorfall(
                *sid,
                "beute",
                format!(
                    "Verbandsbeute bei {}: eigener Anteil im Wert von {}",
                    self.planeten[zp].koord,
                    ganz(wert)
                ),
            );
        }
        if let Some(bericht) = self.kampfberichte.last_mut() {
            bericht.beute = pool;
        }
        let jetzt = self.zeit;
        self.planeten[zp].mali.push(Malus {
            betrag: milli(r.stabilitaet.pluenderung_malus),
            start: jetzt,
            ende: jetzt + r.stabilitaet.pluenderung_tage * TAG,
        });
        self.vorfall(
            self.planeten[zp].besitzer,
            "beute",
            format!(
                "Verbandsangriff auf {}: einmalige Beute im Wert von {}",
                self.planeten[zp].koord,
                ganz(r.wert(&pool))
            ),
        );
    }

    fn pluendern(&mut self, fid: FlottenId, zp: usize) {
        let r = self.regeln.clone();
        let f = self.flotten[&fid].clone();
        let quote = r
            .volk(self.spieler[f.besitzer as usize].volk)
            .pluenderquote
            .unwrap_or(r.kampf.pluenderquote);
        let Ok(plan) = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000) else {
            return;
        };
        let frei = (plan.kapazitaet - f.ladung.iter().sum::<i64>()).max(0);
        let schutz = r.bunkerschutz(self.planeten[zp].gebaeude[Gebaeude::Bunker.idx()]);
        let mut beute = [0i64; GUETER];
        for g in [
            Gut::Erz,
            Gut::Kristall,
            Gut::Deuterium,
            Gut::Nahrung,
            Gut::Legierung,
            Gut::Elektronik,
            Gut::Konsumgut,
            Gut::Xenokristall,
        ] {
            beute[g.idx()] = mal(
                (self.planeten[zp].bestand[g.idx()] - schutz[g.idx()]).max(0),
                quote,
            );
        }
        let summe: i64 = beute.iter().sum();
        if summe > frei {
            for g in 0..GUETER {
                beute[g] = anteil(beute[g], frei, summe);
            }
        }
        for g in 0..GUETER {
            self.planeten[zp].bestand[g] -= beute[g];
            self.flotten.get_mut(&fid).unwrap().ladung[g] += beute[g];
        }
        let wert = r.wert(&beute);
        let st = &r.stabilitaet;
        let jetzt = self.zeit;
        self.planeten[zp].mali.push(Malus {
            betrag: milli(st.pluenderung_malus),
            start: jetzt,
            ende: jetzt + st.pluenderung_tage * TAG,
        });
        if let Some(b) = self.kampfberichte.last_mut() {
            b.beute = beute;
        }
        let opfer = self.planeten[zp].besitzer;
        self.spieler[f.besitzer as usize].statistik.beute += wert;
        let liste: Vec<String> = Gut::ALLE
            .iter()
            .filter(|g| beute[g.idx()] >= M)
            .map(|g| format!("{} {}", ganz(beute[g.idx()]), g))
            .collect();
        let ort = self.planeten[zp].koord;
        self.vorfall(
            f.besitzer,
            "beute",
            format!(
                "Beute bei {ort}: {}",
                if liste.is_empty() {
                    "nichts".into()
                } else {
                    liste.join(", ")
                }
            ),
        );
        self.vorfall(
            opfer,
            "beute",
            format!(
                "{ort} wurde geplündert: {}",
                if liste.is_empty() {
                    "nichts".into()
                } else {
                    liste.join(", ")
                }
            ),
        );
    }

    // ---------------------------------------------------------------- Spionage

    fn spionieren(&mut self, fid: FlottenId) {
        let f = self.flotten[&fid].clone();
        self.scan_aufzeichnen(f.besitzer, f.ziel);
        let nr = self.spionage_nr;
        self.spionage_nr += 1;
        let mut rng = strom(self.startwert, KANAL_SPIONAGE, nr);
        let sonden = f.schiffe[Einheit::Spionagesonde.idx()];
        let jetzt = self.zeit;
        let Some(zp) = self.belegung.get(&f.ziel).map(|x| *x as usize) else {
            // Erkundung eines freien Platzes.
            if let (Some(platz), Some(sys)) =
                (self.platz(f.ziel).cloned(), self.system(f.ziel).cloned())
            {
                self.spieler[f.besitzer as usize].erkundet.insert(
                    f.ziel,
                    Erkundung {
                        zeit: jetzt,
                        felder: platz.felder,
                        zone: platz.zone,
                        reich_erz: sys.reich_erz,
                        reich_kristall: sys.reich_kristall,
                        nebel: sys.nebel,
                    },
                );
                self.vorfall(
                    f.besitzer,
                    "erkundung",
                    format!(
                        "{} erkundet: {} Felder, Zone {}",
                        f.ziel, platz.felder, platz.zone
                    ),
                );
            }
            self.heimflug(fid);
            return;
        };
        self.abrechnen(zp);
        let ziel = &self.planeten[zp];
        let gegner = ziel.besitzer;
        let tech = |sid: SpielerId| {
            self.spieler[sid as usize].forschung[Forschung::Spionagetechnik.idx()] as i64
        };
        let diff = tech(f.besitzer) - tech(gegner);
        let info = sonden + diff;
        let bericht = Spionagebericht {
            zeit: jetzt,
            ziel: f.ziel,
            besitzer: gegner,
            bestand: ziel.bestand.to_vec(),
            schiffe: (info >= 2).then(|| ziel.einheiten[..SCHIFFE].to_vec()),
            verteidigung: (info >= 3).then(|| ziel.einheiten[SCHIFFE..].to_vec()),
            gebaeude: (info >= 5).then(|| ziel.gebaeude.iter().take(if self.aufklaerungsregeln(){GEBAEUDE}else{28}).copied().collect()),
            forschung: (info >= 7).then(|| self.spieler[gegner as usize].forschung.iter().take(if self.aufklaerungsregeln(){FORSCHUNGEN}else{17}).copied().collect()),
        };
        let ziel_schiffe: i64 = ziel.einheiten[..SCHIFFE].iter().sum();
        let mut chance = (ziel_schiffe * sonden * 2).min(1000);
        if diff > 0 {
            chance >>= diff.min(10);
        } else {
            chance = (chance << (-diff).min(10)).min(1000);
        }
        let abgeschossen = (zufall(&mut rng, 1000) as i64) < chance;
        let sp = &mut self.spieler[f.besitzer as usize];
        sp.berichte.retain(|b| b.ziel != f.ziel);
        sp.berichte.push(bericht);
        if sp.berichte.len() > 30 {
            sp.berichte.remove(0);
        }
        let name = sp.name.clone();
        self.vorfall(
            gegner,
            "spionage",
            format!("Spionage durch {name} auf {} bemerkt", f.ziel),
        );
        self.wecke(gegner, Rolle::Feldherr, "Spionage bemerkt", false);
        self.wecke(f.besitzer, Rolle::Feldherr, "Spionagebericht", false);
        if abgeschossen {
            self.flotten.remove(&fid);
            self.vorfall(
                f.besitzer,
                "spionage",
                format!(
                    "Spionagebericht von {} erhalten, die Sonden wurden abgeschossen",
                    f.ziel
                ),
            );
        } else {
            self.vorfall(
                f.besitzer,
                "spionage",
                format!("Spionagebericht von {} erhalten", f.ziel),
            );
            self.heimflug(fid);
        }
    }

    // ---------------------------------------------------------------- Kolonisierung

    fn kolonisieren(&mut self, fid: FlottenId) {
        let r = self.regeln.clone();
        let f = self.flotten[&fid].clone();
        let sid = f.besitzer;
        let (Some(platz), Some(sys)) = (self.platz(f.ziel).cloned(), self.system(f.ziel).cloned())
        else {
            self.heimflug(fid);
            return;
        };
        if self.belegung.contains_key(&f.ziel)
            || self.kolonien(sid) >= self.kolonien_erlaubt(sid)
            || self
                .koloniefracht_pruefen(sid, f.ziel, &f.schiffe, &f.ladung)
                .is_err()
        {
            self.vorfall(
                sid,
                "kolonie",
                format!(
                    "Kolonisierung von {} gescheitert, die Flotte kehrt um",
                    f.ziel
                ),
            );
            self.heimflug(fid);
            return;
        }
        let zr = &r.zonen[&platz.zone];
        let habitate = r
            .einh(Einheit::Kolonieschiff)
            .kosten
            .get(&Gut::Habitatmodul)
            .copied()
            .unwrap_or(0);
        let mut einheiten = [0i64; EINHEITEN];
        einheiten[..SCHIFFE].copy_from_slice(&f.schiffe);
        // Das Kolonieschiff wird verbraucht: seine Habitatmodule werden der Grundwohnraum.
        einheiten[Einheit::Kolonieschiff.idx()] -= 1;
        let pid = self.planeten.len();
        self.planeten.push(Planet {
            id: pid as PlanetId,
            besitzer: sid,
            koord: f.ziel,
            zone: platz.zone,
            felder: platz.felder,
            heimat: false,
            faktor: [
                sys.reich_erz,
                sys.reich_kristall,
                milli(zr.deuterium),
                milli(zr.nahrung),
                milli(zr.solar),
            ],
            bestand: f.ladung,
            rate: [0; GUETER],
            stand: self.zeit,
            gebaeude: [0; GEBAEUDE],
            einheiten,
            raketen: [0; 2],
            bevoelkerung: f.siedler,
            stabilitaet: milli(r.welt.start_stabilitaet),
            grundwohnraum: habitate * r.wirtschaft.habitat_wohnraum * M,
            bauschleife: Vec::new(),
            fertigung: [Vec::new(), Vec::new()],
            fertigung_naechste: [-1, -1],
            prioritaeten: Vec::new(),
            mali: Vec::new(),
            belagerungsmalus: 0,
            blockade: None,
            leer_gemeldet: false,
            gegruendet: self.zeit,
            energie_erzeugung: 0,
            energie_verbrauch: 0,
            arbeit_bedarf: 0,
            arbeit_verfuegbar: 0,
            fk_bedarf: 0,
            fk_verfuegbar: 0,
            nahrung_deckung: 1000,
            konsum_deckung: 0,
            fp_rate: 0,
            stab_ziel: milli(r.welt.start_stabilitaet),
            wohnraum: 0,
        });
        self.belegung.insert(f.ziel, pid as PlanetId);
        self.spieler[sid as usize].planeten.push(pid as PlanetId);
        self.flotten.remove(&fid);
        self.fachereignis(
            "colony_founded",
            sid,
            serde_json::json!({"planet":pid,"fleet":fid,"target":f.ziel}),
        );
        self.raten_neu(pid);
        self.vorfall(
            sid,
            "kolonie",
            format!(
                "Kolonie auf {} gegründet: {} Felder, Zone {}",
                f.ziel, platz.felder, platz.zone
            ),
        );
        self.wecke(sid, Rolle::Stratege, "Kolonie gegründet", false);
        self.wecke(sid, Rolle::Verwalter, "Kolonie gegründet", false);
        self.punkte_neu();
    }

    // ---------------------------------------------------------------- Eroberung

    pub fn bodentruppen(&self, f: &Flotte) -> i64 {
        let k = &self.regeln.kampf;
        let waffen =
            self.spieler[f.besitzer as usize].forschung[Forschung::Waffentechnik.idx()] as f64;
        mal(
            f.schiffe[Einheit::Truppentransporter.idx()] * k.truppen_je_transporter,
            1.0 + k.tech_je_stufe * waffen,
        )
    }

    pub fn garnison(&self, pid: usize) -> i64 {
        let k = &self.regeln.kampf;
        let p = &self.planeten[pid];
        let panzer = self.spieler[p.besitzer as usize].forschung[Forschung::Panzerung.idx()] as f64;
        mal(
            p.gebaeude[Gebaeude::Kaserne.idx()] as i64 * k.garnison_je_kaserne,
            1.0 + k.tech_je_stufe * panzer,
        ) + p.bevoelkerung / (100 * M) * k.garnison_je_100_einwohner
    }

    /// Belagerte Kolonien wechseln den Besitzer, wenn die Stabilität gefallen ist und
    /// die Bodentruppen des Angreifers der Garnison überlegen sind.
    pub fn eroberung_pruefen(&mut self) {
        if self.kolonisation.aktiv {
            return;
        }
        let schwelle = milli(self.regeln.stabilitaet.uebernahme_unter);
        for pid in 0..self.planeten.len() {
            let Some(bf) = self.planeten[pid].blockade else {
                continue;
            };
            let Some(f) = self.flotten.get(&bf).cloned() else {
                self.planeten[pid].blockade = None;
                continue;
            };
            if f.mission != Mission::Invasion || self.planeten[pid].heimat {
                continue;
            }
            if self.planeten[pid].stabilitaet < schwelle
                && self.bodentruppen(&f) > self.garnison(pid)
            {
                self.erobern(pid, bf);
            }
        }
    }

    pub(crate) fn erobern(&mut self, pid: usize, fid: FlottenId) {
        let r = self.regeln.clone();
        let f = self.flotten[&fid].clone();
        self.abrechnen(pid);
        // Offene Marktorders auf diesem Planeten enden vor dem Besitzerwechsel.
        let planet_id = self.planeten[pid].id;
        self.orders_stornieren_planet(planet_id);
        let alt = self.planeten[pid].besitzer;
        let neu = f.besitzer;
        let regeln_v2 = self.kolonisationsregeln_v2();
        let p = &mut self.planeten[pid];
        let neuartig = self.kolonisation.aktiv && f.mission == Mission::Kampfkolonisieren;
        if !neuartig {
            for g in 0..GEBAEUDE {
                p.gebaeude[g] =
                    mal(p.gebaeude[g] as i64, 1.0 - r.kampf.eroberung_stufenverlust) as u8;
            }
        }
        if !regeln_v2 {
            p.bevoelkerung = mal(p.bevoelkerung, r.kampf.eroberung_bevoelkerung).max(100 * M);
        }
        p.einheiten = [0; EINHEITEN];
        for e in 0..SCHIFFE {
            p.einheiten[e] = f.schiffe[e];
        }
        if neuartig {
            p.einheiten[Einheit::Kolonieschiff.idx()] -= 1;
        }
        for g in 0..GUETER {
            p.bestand[g] += f.ladung[g];
        }
        p.bauschleife.clear();
        p.fertigung = [Vec::new(), Vec::new()];
        p.fertigung_naechste = [-1, -1];
        p.prioritaeten.clear();
        p.mali.clear();
        p.belagerungsmalus = 0;
        p.blockade = None;
        p.stabilitaet = milli(r.stabilitaet.unruhen_unter);
        p.besitzer = neu;
        p.raketen = [0; 2];
        let k = p.koord;
        let id = p.id;
        self.kolonisation
            .reparaturen
            .retain(|(planet, _), _| *planet != id);
        self.ereignisse.retain(
            |e| !matches!(e.art, EreignisArt::RaketenFertig { planet, .. } if planet == id)
                && !(regeln_v2 && matches!(e.art,EreignisArt::BauFertig{planet}|EreignisArt::FertigungFertig{planet,..} if planet==id)),
        );
        self.flotten.remove(&fid);
        self.kolonisation.besetzungen.remove(&id);
        self.kolonisation.besetzungsmarken.remove(&id);
        self.kolonisation.besetzungsangreifer.remove(&id);
        self.fachereignis(
            "planet_captured",
            neu,
            serde_json::json!({"planet":id,"fleet":fid,"previous_owner":alt,"owner":neu}),
        );
        self.spieler[alt as usize].planeten.retain(|x| *x != id);
        self.spieler[neu as usize].planeten.push(id);
        let (alt_name, neu_name) = (
            self.spieler[alt as usize].name.clone(),
            self.spieler[neu as usize].name.clone(),
        );
        self.vorfall(
            alt,
            "eroberung",
            format!("{k} wurde von {neu_name} erobert"),
        );
        self.vorfall(neu, "eroberung", format!("{k} von {alt_name} erobert"));
        self.wecke(alt, Rolle::Stratege, "Kolonie verloren", true);
        self.wecke(neu, Rolle::Stratege, "Kolonie erobert", false);
        self.wecke(neu, Rolle::Verwalter, "Kolonie erobert", false);
        self.raten_neu(pid);
        self.punkte_neu();
    }
}
