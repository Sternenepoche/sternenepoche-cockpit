//! Orbitflotten / gestrandete Flotten (0.4.8, Karl 24.10.2026).
//!
//! „…unter 10 % ‚kein Rückflug‘, dann bleibt die Flotte da stehen, es gelten dieselben Regeln wie für gestrandet, da sie im
//! Orbit steht ohne geplanten Rückflugtreibstoff, es sei denn, man hat als Ladung im Laderaum der Flotte Deuterium – dieses
//! kann verwendet werden, um sich von dem Zielplaneten weiter bewegen zu können … als einzelne Flotte in der Flottenübersicht
//! … direkt befehligen oder anfliegen … wenn ich die Flotte von dort aus zu einem Angriff geschickt habe und sie zurückkehrt,
//! ist sie so lange zu sehen, bis sie sich wieder bewegt, muss dann aber einen der eigenen Planeten oder Übergabe als mögliche
//! Aktion haben und nicht nochmals von dort aus angreifen können … für alle Spieler, die das System erkundet haben, offen
//! sichtbar … von jedem als Ziel auswählbar.“
//!
//! Zustand (JSON-Erweiterung von `AusbauStand`, leer = nicht geschrieben, laufende Welten bytegleich):
//! - `orbitflotten`: gestrandete Flotten (`ImOrbit`, `orbit_ende = 0`, `ziel` = Ort) mit `seit` und `nur_heim`.
//! - `rueckziele`: Flüge ohne Heimkehr zum Startplaneten – `Stranden` (kein Rückflugtreibstoff) oder `Orbit` (zurück an den
//!   Orbitort, von dem sie mit Treibstoff aus der Ladung gestartet sind).
//! - `zielflotten`: Angriffe auf eine gestrandete Flotte (Angreiferflotte → Zielflotte).
use crate::flotte::Partei;
use crate::{typen::*, welt::*, Welt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Orbitlage {
    pub seit: SimZeit,
    /// Nach einem Angriff vom Orbitort aus: nur noch Stationieren auf einen eigenen Planeten oder Übergabe.
    #[serde(default)]
    pub nur_heim: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Rueckziel {
    Stranden,
    Orbit { ort: Koord, nur_heim: bool },
}

/// Woher ein Flug startet.
enum Quelle {
    Planet(usize),
    Orbit(FlottenId),
}

impl Welt {
    pub fn orbitflotten_aktiv(&self) -> bool {
        self.regeln.kw().flug.orbitflotten && self.kolonisationsregeln_v2()
    }
    pub fn gestrandet(&self, fid: FlottenId) -> bool {
        self.ausbau.orbitflotten.contains_key(&fid)
    }

    /// Flug ohne Rückflugtreibstoff (`flotte_planen` mit `rueckgeschwindigkeit: 0`): nach dem Auftrag bleibt die Flotte im Orbit.
    pub(crate) fn einweg_registrieren(&mut self, fid: FlottenId) {
        self.ausbau.rueckziele.insert(fid, Rueckziel::Stranden);
        if let Some(v) = self.ausbau.fluganzeigen.get_mut(&fid) { v["einweg"] = json!(true); v["kein_rueckflug"] = json!(true); }
    }

    /// Die Flotte bleibt an ihrem Ort (Ziel) im Orbit; auf einem eigenen Planeten landet sie stattdessen.
    pub(crate) fn stranden(&mut self, fid: FlottenId, nur_heim: bool) {
        let Some(f) = self.flotten.get(&fid).cloned() else { return };
        self.ausbau.rueckziele.remove(&fid);
        self.ausbau.rueckflugplaene.remove(&fid);
        self.kolonisation.rueckwege.remove(&fid);
        if let Some(pid) = self.belegung.get(&f.ziel).map(|p| *p as usize).filter(|p| self.planeten[*p].besitzer == f.besitzer) {
            self.orbitflotte_landen(fid, pid);
            return;
        }
        if let Some(fl) = self.flotten.get_mut(&fid) {
            fl.zustand = Flottenzustand::ImOrbit;
            fl.orbit_ende = 0;
            fl.verband = None;
        }
        self.ausbau.orbitflotten.insert(fid, Orbitlage { seit: self.zeit, nur_heim });
        let deut = f.ladung[Gut::Deuterium.idx()];
        self.vorfall(f.besitzer, "flotte", format!(
            "Flotte {fid} bleibt ohne Rückflugtreibstoff im Orbit von {} – gestrandet, für alle Kenner des Systems sichtbar und angreifbar. Weiterfliegen mit Deuterium aus der Ladung (an Bord {}) oder per Transport versorgen{}",
            f.ziel, ganz(deut), if nur_heim { "; von hier nur noch Heimflug oder Übergabe" } else { "" }));
        self.fachereignis("fleet_stranded", f.besitzer, json!({"fleet":fid,"at":f.ziel,"home_only":nur_heim}));
    }

    /// Haken in `heimflug`: Flüge mit eigenem Rückziel fliegen nicht zum Startplaneten. true = erledigt.
    pub(crate) fn heimflug_orbit(&mut self, fid: FlottenId) -> bool {
        let Some(rz) = self.ausbau.rueckziele.get(&fid).cloned() else { return false };
        let Some(f) = self.flotten.get(&fid).cloned() else { return false };
        match rz {
            Rueckziel::Stranden => { self.stranden(fid, false); true }
            Rueckziel::Orbit { ort, nur_heim } => {
                if f.ziel == ort { self.stranden(fid, nur_heim); return true; }
                let dauer = self.rueckdauer(fid, f.flugdauer);
                let zurueck = self.zeit + dauer;
                if let Some(fl) = self.flotten.get_mut(&fid) {
                    fl.zustand = Flottenzustand::Rueckflug;
                    fl.verband = None;
                    fl.rueckkehr = zurueck;
                }
                self.rueckflug_anzeigen(fid, 1.0, f.ziel);
                self.plane(zurueck, EreignisArt::FlotteRueckkehr { flotte: fid });
                true
            }
        }
    }

    /// Haken in der Rückkehr: zurück am Orbitort → wieder gestrandet (statt Landung auf dem Startplaneten). true = erledigt.
    pub(crate) fn rueckkehr_orbit(&mut self, fid: FlottenId) -> bool {
        let Some(Rueckziel::Orbit { ort, nur_heim }) = self.ausbau.rueckziele.get(&fid).cloned() else { return false };
        if let Some(fl) = self.flotten.get_mut(&fid) { fl.ziel = ort; }
        self.stranden(fid, nur_heim);
        true
    }

    /// Haken im Rückruf. Ok(true) = Rückruf hier erledigt/abgelehnt mit Text in Err.
    pub(crate) fn rueckruf_orbit(&mut self, fid: FlottenId) -> Result<bool, String> {
        if self.gestrandet(fid) {
            return Err("Gestrandete Flotte: mit Deuterium aus der Ladung zu einem eigenen Planeten fliegen (Weiterfliegen → Stationieren) – einen kostenlosen Rückruf gibt es nicht".into());
        }
        let Some(Rueckziel::Stranden) = self.ausbau.rueckziele.get(&fid).cloned() else { return Ok(false) };
        let Some(f) = self.flotten.get(&fid).cloned() else { return Ok(false) };
        if f.zustand != Flottenzustand::Hinflug { return Ok(false); }
        // Umkehren kostet die bisher geflogene Strecke an Treibstoff – aus der Ladung.
        let hin = self.ausbau.fluganzeigen.get(&fid).and_then(|v| v["treibstoff_hin"].as_f64()).map(|t| (t * M as f64).round() as i64).unwrap_or(0);
        let anteil = ((self.zeit - f.abflug).max(0) * 1000 / f.flugdauer.max(1)).clamp(0, 1000);
        let noetig = (hin * anteil + 999) / 1000;
        let deut = f.ladung[Gut::Deuterium.idx()];
        if deut < noetig {
            return Err(format!("Ohne Rückflugtreibstoff kein Rückruf: {} Deuterium in der Ladung nötig, an Bord {}", ganz(noetig), ganz(deut)));
        }
        if let Some(fl) = self.flotten.get_mut(&fid) { fl.ladung[Gut::Deuterium.idx()] -= noetig; }
        // Vom Orbit gestartet: zurück an den Orbitort; vom Planeten: zurück zum Startplaneten.
        let vom_orbit = self.ausbau.fluganzeigen.get(&fid).is_some_and(|v| v["vom_orbit"] == true);
        if vom_orbit { self.ausbau.rueckziele.insert(fid, Rueckziel::Orbit { ort: f.start_koord, nur_heim: false }); } else { self.ausbau.rueckziele.remove(&fid); }
        Ok(false)
    }

    /// Start von einer gestrandeten Flotte mit Treibstoff aus ihrer Ladung.
    pub fn flotte_starten(&mut self, sid: SpielerId, q: &Value) -> Result<String, String> {
        if !self.orbitflotten_aktiv() { return Err("Start aus dem Orbit gibt es in Welten mit Orbitflotten".into()); }
        let fid = q["flotte"].as_u64().ok_or("flotte fehlt")? as FlottenId;
        let lage = self.ausbau.orbitflotten.get(&fid).cloned().ok_or("Keine gestrandete eigene Flotte unter dieser Kennung")?;
        let f = self.flotten.get(&fid).cloned().ok_or("Flotte fehlt")?;
        if f.besitzer != sid { return Err("Keine gestrandete eigene Flotte unter dieser Kennung".into()); }
        self.flug_anlegen(sid, Quelle::Orbit(fid), q, f.schiffe, Some(lage))
    }

    /// Angriff von einem eigenen Planeten auf eine gestrandete fremde Flotte.
    pub fn orbitflotte_angreifen(&mut self, sid: SpielerId, q: &Value) -> Result<String, String> {
        if !self.orbitflotten_aktiv() { return Err("Angriffe auf gestrandete Flotten gibt es in Welten mit Orbitflotten".into()); }
        let start: Koord = q["start"].as_str().ok_or("start fehlt")?.parse()?;
        let pid = self.eigener_planet(sid, start)?;
        let mut s = [0i64; SCHIFFE];
        if let Some(m) = q["schiffe"].as_object() {
            for (k, v) in m {
                let e: Einheit = serde_json::from_value(json!(k)).map_err(|_| format!("Unbekanntes Schiff {k}"))?;
                if !e.ist_schiff() { return Err(format!("{e} ist kein Schiff")); }
                s[e.idx()] = v.as_i64().ok_or("Schiffszahl ungültig")?;
            }
        }
        let mut q = q.clone();
        q["mission"] = json!("angriff");
        self.flug_anlegen(sid, Quelle::Planet(pid), &q, s, None)
    }

    fn flug_anlegen(&mut self, sid: SpielerId, quelle: Quelle, q: &Value, schiffe: [i64; SCHIFFE], lage: Option<Orbitlage>) -> Result<String, String> {
        let ziel: Koord = q["ziel"].as_str().map(|s| s.parse()).transpose()?.or_else(|| {
            q["zielflotte"].as_u64().and_then(|z| self.flotten.get(&(z as FlottenId))).map(|z| z.ziel)
        }).ok_or("ziel fehlt")?;
        let mission: Mission = serde_json::from_value(q["mission"].clone()).map_err(|_| "Unbekannte Mission")?;
        let sigma = milli(q["geschwindigkeit"].as_f64().unwrap_or(1.0));
        if !(100..=1000).contains(&sigma) { return Err("Hinflugtempo: 10 bis 100 %".into()); }
        let rueck = q["rueckgeschwindigkeit"].as_f64().unwrap_or(1.0);
        if !(rueck == 0.0 || (0.1..=1.0).contains(&rueck)) { return Err("Rückflugtempo: 10 bis 100 % oder 0 = kein Rückflug".into()); }
        let zielflotte = q["zielflotte"].as_u64().map(|z| z as FlottenId);
        let haltedauer = q["haltedauer_stunden"].as_i64().unwrap_or(0) * STUNDE;
        if self.system(ziel).is_none() || ziel.position > self.regeln.welt.plaetze_je_system { return Err(format!("{ziel} liegt außerhalb der Galaxie")); }
        if !schiffe.iter().any(|n| *n > 0) { return Err("keine Schiffe gewählt".into()); }
        let ziel_pid = self.belegung.get(&ziel).map(|p| *p as usize);
        let eigen = ziel_pid.is_some_and(|p| self.planeten[p].besitzer == sid);
        // Erlaubte Aufträge (Punkt 3: nach einem Angriff vom Orbitort aus nur noch heim oder Übergabe).
        let uebergabe = q["uebergabe"] == true && mission == Mission::Transport;
        if lage.as_ref().is_some_and(|l| l.nur_heim) && mission != Mission::Stationieren && !uebergabe {
            return Err("Nach einem Angriff von hier aus: nur noch Heimflug (Stationieren auf einen eigenen Planeten) oder Flottenübergabe".into());
        }
        let mut gegner: Option<SpielerId> = None;
        match mission {
            Mission::Stationieren => if !eigen { return Err("Stationieren nur auf einem eigenen Planeten".into()); },
            Mission::Transport => if ziel_pid.is_none() && !self.eigene_orbitflotte_an(sid, ziel) { return Err(format!("auf {ziel} liegt kein Planet")); },
            Mission::Halten => {
                let zp = ziel_pid.ok_or_else(|| format!("auf {ziel} liegt kein Planet"))?;
                if !self.verbuendet(sid, self.planeten[zp].besitzer) { return Err("Halten geht nur bei einem Verbündeten".into()); }
                if haltedauer < STUNDE || haltedauer > self.regeln.flug.halten_max_stunden * STUNDE {
                    return Err(format!("haltedauer_stunden muss zwischen 1 und {} liegen", self.regeln.flug.halten_max_stunden));
                }
            }
            Mission::Spionage => {
                if (0..SCHIFFE).any(|e| e != Einheit::Spionagesonde.idx() && schiffe[e] > 0) { return Err("eine Spionageflotte besteht nur aus Spionagesonden".into()); }
                if !self.system_erfasst(sid, ziel) { return Err("Sonnensystem zuerst kartieren".into()); }
            }
            Mission::Angriff => {
                if let Some(z) = zielflotte {
                    let zf = self.flotten.get(&z).filter(|zf| self.gestrandet(z) && zf.ziel == ziel).ok_or("Am Ziel steht keine gestrandete Flotte unter dieser Kennung")?;
                    if !self.system_erfasst(sid, ziel) { return Err("Sonnensystem zuerst kartieren – gestrandete Flotten sieht, wer das System kennt".into()); }
                    self.gegner_angreifbar(sid, zf.besitzer)?;
                    gegner = Some(zf.besitzer);
                } else {
                    let zp = ziel_pid.ok_or_else(|| format!("auf {ziel} liegt kein Planet"))?;
                    if eigen { return Err("eigene Planeten kann man nicht angreifen".into()); }
                    let owner = self.planeten[zp].besitzer;
                    self.gegner_angreifbar(sid, owner)?;
                    gegner = Some(owner);
                }
            }
            _ => return Err("Von einer gestrandeten Flotte aus gehen Stationieren, Transport, Halten, Spionage und Angriff (Übergabe über das Angebot)".into()),
        }
        // Aufklärungspflicht für fremde Ziele (wie flotte_senden).
        if !eigen && zielflotte.is_none() && !matches!(mission, Mission::Spionage) && self.planetenwissen(sid, ziel)["bekannt"] != true
            && !self.eigene_orbitflotte_an(sid, ziel)
            && !(mission == Mission::Halten && ziel_pid.is_some_and(|p| self.verbuendet(sid, self.planeten[p].besitzer)))
            // Karl 28.10.2026: Allianzmitglieder beliefern einander ohne eigenen Bericht.
            && !(mission == Mission::Transport && ziel_pid.is_some_and(|p| self.gleiche_allianz(sid, self.planeten[p].besitzer))) {
            return Err("Ziel zuerst mit einer eigenen Planetensonde aufklären".into());
        }
        let (von, raumhafen) = match quelle {
            Quelle::Planet(pid) => (self.planeten[pid].koord, self.planeten[pid].gebaeude[Gebaeude::Raumhafen.idx()]),
            Quelle::Orbit(fid) => (self.flotten[&fid].ziel, 0),
        };
        let plan = self.flugplan_von(sid, von, raumhafen, ziel, &schiffe, sigma)?;
        let einweg = matches!(mission, Mission::Stationieren) || rueck == 0.0;
        let retour = if einweg { None } else { Some(self.flugplan_von(sid, ziel, 0, von, &schiffe, milli(rueck))?) };
        let treibstoff = plan.treibstoff + retour.as_ref().map(|r| r.treibstoff).unwrap_or(0);
        let id = match quelle {
            Quelle::Orbit(fid) => {
                let f = self.flotten[&fid].clone();
                let deut = f.ladung[Gut::Deuterium.idx()];
                if deut < treibstoff {
                    return Err(if einweg || deut < plan.treibstoff {
                        format!("Treibstoff reicht nicht: {} Deuterium nötig, an Bord {} – zuerst per Transport versorgen", ganz(treibstoff), ganz(deut))
                    } else {
                        format!("Treibstoff reicht nur für den Hinflug ({} von {} Deuterium) – „kein Rückflug“ wählen oder versorgen", ganz(deut), ganz(treibstoff))
                    });
                }
                let mut ladung = f.ladung;
                ladung[Gut::Deuterium.idx()] -= treibstoff;
                plan.pruefe_zuladung(ladung.iter().sum(), 0)?;
                if let Some(fl) = self.flotten.get_mut(&fid) {
                    fl.ladung = ladung;
                    fl.start_koord = von;
                    fl.start = self.spieler[sid as usize].heimat;
                    fl.ziel = ziel;
                    fl.mission = mission;
                    fl.abflug = self.zeit;
                    fl.ankunft = self.zeit + plan.dauer;
                    fl.flugdauer = plan.dauer;
                    fl.rueckkehr = 0;
                    fl.orbit_ende = 0;
                    fl.haltedauer = haltedauer;
                    fl.zustand = Flottenzustand::Hinflug;
                    fl.gemeldet = false;
                    fl.verband = None;
                }
                self.ausbau.orbitflotten.remove(&fid);
                fid
            }
            Quelle::Planet(pid) => {
                let p = &self.planeten[pid];
                if p.gebaeude[Gebaeude::Raumhafen.idx()] < 1 { return Err(format!("Flottenstart braucht einen Raumhafen auf {von}")); }
                for e in 0..SCHIFFE { if schiffe[e] < 0 || schiffe[e] > p.einheiten[e] { return Err(format!("auf {von} stehen nur {} x {}", p.einheiten[e], Einheit::ALLE[e])); } }
                let plaetze = self.flottenplaetze(sid);
                if self.flotten.values().filter(|f| f.besitzer == sid).count() >= plaetze { return Err(format!("alle {plaetze} Flottenplätze sind belegt")); }
                self.abrechnen(pid);
                let mut sprit = [0i64; GUETER];
                sprit[Gut::Deuterium.idx()] = treibstoff;
                self.zahlbar(pid, &sprit, None)?;
                self.zahlen(pid, &sprit, None);
                for e in 0..SCHIFFE { self.planeten[pid].einheiten[e] -= schiffe[e]; }
                let id = self.naechste_flotte;
                self.naechste_flotte += 1;
                let start = self.planeten[pid].id;
                self.flotten.insert(id, Flotte { id, besitzer: sid, start, start_koord: von, ziel, mission, schiffe, ladung: [0; GUETER], siedler: 0,
                    abflug: self.zeit, ankunft: self.zeit + plan.dauer, flugdauer: plan.dauer, rueckkehr: 0, orbit_ende: 0, haltedauer,
                    zustand: Flottenzustand::Hinflug, gemeldet: false, verband: None });
                self.raten_neu(pid);
                id
            }
        };
        let vom_orbit = matches!(quelle, Quelle::Orbit(_));
        if einweg {
            self.ausbau.rueckziele.insert(id, Rueckziel::Stranden);
        } else if let (Some(r), true) = (retour.as_ref(), vom_orbit) {
            self.ausbau.rueckziele.insert(id, Rueckziel::Orbit { ort: von, nur_heim: mission.feindlich() });
            self.ausbau.rueckflugplaene.insert(id, crate::flugkoordination::Rueckflugplan { dauer: r.dauer, tempo: rueck, treibstoff: r.treibstoff });
        } else if let Some(r) = retour.as_ref() {
            self.ausbau.rueckflugplaene.insert(id, crate::flugkoordination::Rueckflugplan { dauer: r.dauer, tempo: rueck, treibstoff: r.treibstoff });
        }
        if let Some(z) = zielflotte { self.ausbau.zielflotten.insert(id, z); }
        self.ausbau.fluganzeigen.insert(id, json!({"einweg":einweg,"kein_rueckflug":rueck==0.0,"vom_orbit":vom_orbit,"hin_tempo_prozent":sigma/10,"hin_dauer":plan.dauer,
            "treibstoff_hin":plan.treibstoff as f64/M as f64,"distanz":plan.entfernung,"reisetempo":plan.tempo}));
        let ankunft = self.zeit + plan.dauer;
        self.plane(ankunft, EreignisArt::FlotteAnkunft { flotte: id });
        if self.aufklaerungsregeln() {
            self.aufklaerung.abschirmung.insert(id, self.spieler[sid as usize].forschung[Forschung::Abschirmtechnik.idx()]);
            if mission.feindlich() && zielflotte.is_none() {
                if let Some(zp) = ziel_pid {
                    let kontakt = (ankunft - self.warnzeit(zp)).max(self.zeit);
                    if kontakt == self.zeit { self.sensor_kontakt(id); } else { self.plane(kontakt, EreignisArt::SensorKontakt { flotte: id }); }
                }
            }
        }
        if mission == Mission::Transport {
            if let Some(zp) = ziel_pid {
                self.kolonisation.transportziele.insert(id, crate::kolonisation::Transportziel::Planet { planet: zp as PlanetId, empfaenger: self.planeten[zp].besitzer });
            } else if let Some(of) = self.eigene_orbitflotte_bei(sid, ziel).filter(|of| *of != id) {
                self.kolonisation.transportziele.insert(id, crate::kolonisation::Transportziel::Flotte { flotte: of, empfaenger: sid });
            }
        }
        if let (Some(z), Some(g)) = (zielflotte, gegner) {
            let name = self.spieler[sid as usize].name.clone();
            self.vorfall(g, "flotte", format!("Angriff von {name} auf deine gestrandete Flotte {z} an {ziel} – Ankunft {}", zeittext(ankunft)));
        }
        if gegner.is_some() {
            let jetzt = self.zeit;
            let sp = &mut self.spieler[sid as usize];
            sp.schutz_bis = sp.schutz_bis.min(jetzt);
            if let Some(g) = gegner { if !sp.angegriffen.contains(&g) { sp.angegriffen.push(g); } }
        }
        self.fachereignis("fleet_launched", sid, json!({"fleet":id,"mission":mission,"target":ziel,"arrival":ankunft,"from_orbit":vom_orbit,"target_fleet":zielflotte}));
        Ok(format!("Flotte {id} startet{}: {mission} nach {ziel}, Ankunft {}, Treibstoff {}{}{}",
            if vom_orbit { format!(" von {von}") } else { String::new() }, zeittext(ankunft), ganz(treibstoff),
            if vom_orbit { " aus der Ladung" } else { "" },
            if einweg && mission != Mission::Stationieren { " – ohne Rückflug, sie bleibt danach am Ziel" } else if vom_orbit && !einweg { " (Rückflug zum Orbitort eingeplant)" } else { "" }))
    }

    /// Ankunft eines Angriffs auf eine gestrandete Flotte: Kampf nur gegen die gestrandeten Flotten dieses Spielers am Ort.
    /// true = Ankunft erledigt.
    pub(crate) fn orbitkampf_ankunft(&mut self, fid: FlottenId) -> bool {
        let Some(z) = self.ausbau.zielflotten.remove(&fid) else { return false };
        let Some(f) = self.flotten.get(&fid).cloned() else { return true };
        let Some(ziel) = self.flotten.get(&z).filter(|zf| self.gestrandet(z) && zf.ziel == f.ziel).cloned() else {
            self.vorfall(f.besitzer, "flotte", format!("Flotte {fid}: an {} steht keine gestrandete Zielflotte mehr – Rückflug", f.ziel));
            self.heimflug(fid);
            return true;
        };
        if self.gegner_angreifbar(f.besitzer, ziel.besitzer).is_err() {
            self.umkehren(fid, "das Ziel ist nicht mehr angreifbar");
            return true;
        }
        let gegner: Vec<Partei> = self.flotten.values()
            .filter(|x| x.besitzer == ziel.besitzer && x.ziel == f.ziel && self.gestrandet(x.id))
            .map(|x| Partei::Flotte(x.id)).collect();
        self.gefecht(&[fid], &gegner, f.ziel);
        for p in gegner { if let Partei::Flotte(id) = p { if !self.flotten.contains_key(&id) { self.ausbau.orbitflotten.remove(&id); } } }
        if self.flotten.contains_key(&fid) { self.heimflug(fid); }
        true
    }

    /// Steht an `k` eine eigene gestrandete Flotte? Dann kennt `sid` das Ziel (keine Planetensonde nötig).
    pub fn eigene_orbitflotte_an(&self, sid: SpielerId, k: Koord) -> bool {
        self.eigene_orbitflotte_bei(sid, k).is_some()
    }
    /// Die (erste) eigene gestrandete Flotte an `k` – Empfänger eines Transports ohne Planet am Ziel.
    pub fn eigene_orbitflotte_bei(&self, sid: SpielerId, k: Koord) -> Option<FlottenId> {
        if !self.orbitflotten_aktiv() { return None; }
        self.ausbau.orbitflotten.keys().copied().find(|id| self.flotten.get(id).is_some_and(|f| f.besitzer == sid && f.ziel == k))
    }

    /// Gestrandete Flotten an einer Koordinate für Kenner des Systems (Galaxie, Infokarte).
    pub fn orbitflotten_an(&self, sid: SpielerId, k: Koord) -> Value {
        if !self.system_erfasst(sid, k) { return json!([]); }
        json!(self.ausbau.orbitflotten.iter().filter_map(|(id, l)| self.flotten.get(id).filter(|f| f.ziel == k).map(|f| {
            json!({"flotte":id,"spieler":self.spieler[f.besitzer as usize].name,"eigen":f.besitzer==sid,"schiffe":f.schiffe.iter().sum::<i64>(),"seit":l.seit,"nur_heim":l.nur_heim})
        })).collect::<Vec<_>>())
    }

    /// Alle gestrandeten Flotten, die `sid` kennt (eigene und solche in erfassten Systemen) – für die Zielauswahl im Versand.
    pub fn orbitflotten_bekannt(&self, sid: SpielerId) -> Value {
        json!(self.ausbau.orbitflotten.iter().filter_map(|(id, l)| self.flotten.get(id)
            .filter(|f| f.besitzer == sid || self.system_erfasst(sid, f.ziel)).map(|f| {
            json!({"flotte":id,"koord":f.ziel.to_string(),"spieler":self.spieler[f.besitzer as usize].name,"eigen":f.besitzer==sid,
                "schiffe":f.schiffe.iter().sum::<i64>(),"seit":l.seit,"nur_heim":l.nur_heim})
        })).collect::<Vec<_>>())
    }

    /// Hilft diese gestrandete Flotte dem Besitzer des Planeten (Orbitkampf und Boden)? Nie gegen den Besitzer, nicht wer
    /// ihn angegriffen hat, nicht die Angreiferseite einer laufenden Besetzung.
    pub fn orbitflotte_hilft(&self, f: &Flotte, pid: usize, angreifer: Option<SpielerId>) -> bool {
        let p = &self.planeten[pid];
        self.gestrandet(f.id) && f.ziel == p.koord && f.besitzer != p.besitzer
            && !self.spieler[f.besitzer as usize].angegriffen.contains(&p.besitzer)
            && !angreifer.is_some_and(|a| a == f.besitzer || self.verbuendet(a, f.besitzer))
    }
}
