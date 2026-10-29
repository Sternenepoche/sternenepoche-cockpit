//! Stateful expeditions. Gameplay draws and narrative draws use separate streams.
use crate::{
    kampf::{kampf, Gruppe},
    typen::*,
    welt::*,
    Welt,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expedition {
    pub besitzer: SpielerId,
    pub ereignisse: u8,
    pub naechstes: SimZeit,
    pub scan: bool,
    pub zielart: u8,
    pub risikowurf: u32,
    pub totalwurf: u16,
    pub status: String,
    pub frist: Option<SimZeit>,
    pub rettungen: BTreeMap<FlottenId, SimZeit>,
    pub ruempfe: BTreeMap<Einheit, i64>,
    pub berichte: Vec<String>,
}
pub fn ereignislimit(pathfinder: i64) -> u8 {
    2 + [1, 2, 4, 8, 16, 32]
        .iter()
        .filter(|n| pathfinder >= **n)
        .count() as u8
}
pub fn rettung_limit(bergung: i64, pathfinder: i64) -> i64 {
    bergung.max(0) * (1 + pathfinder.clamp(0, 3))
}
pub fn risiko_ppm(pathfinder: i64) -> u32 {
    80_000 / (1 + pathfinder.clamp(0, 32) as u32)
}
impl Welt {
    fn expedition_bericht(&mut self, fid: FlottenId, fakt: &str) {
        let Some(e) = self.ausbau.expeditionen.get_mut(&fid) else {
            return;
        };
        let mut rng = strom(
            self.startwert,
            92,
            (fid as u64) << 32 | e.berichte.len() as u64,
        );
        let opener = [
            "Im blauen Schein der Instrumente verstummte die Brücke.",
            "Ein fernes Signal durchbrach das Rauschen.",
            "Staub zog wie ein Schleier am Sichtfenster vorbei.",
            "Das Logbuch verzeichnet eine weitere Begegnung fern der Heimat.",
        ];
        let ending = [
            "Die Navigation hat den Vorgang protokolliert.",
            "Der Bericht trägt die Signatur der Einsatzleitung.",
            "Die letzten Messdaten sind dem Logbuch beigefügt.",
        ];
        let text = format!(
            "{} {} {}",
            opener[zufall(&mut rng, 4) as usize],
            fakt,
            ending[zufall(&mut rng, 3) as usize]
        );
        e.berichte.push(text.clone());
        let sid = e.besitzer;
        self.vorfall(sid, "expedition", text);
    }
    pub fn expedition_starten(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        action: &serde_json::Value,
    ) -> Result<String, String> {
        if self.ausbau.version == 0 {
            return Err("Expedition benötigt Generation 12".into());
        }
        let a: crate::aktion::Aktion =
            serde_json::from_value(action.clone()).map_err(|e| e.to_string())?;
        let (start, ziel, ships) = match a {
            crate::aktion::Aktion::FlotteSenden {
                start,
                ziel,
                mission: Mission::Saven,
                schiffe,
                ..
            } => (start, ziel, schiffe),
            _ => return Err("Expedition benötigt einen Save-Flugauftrag als Flugplan".into()),
        };
        if ships.values().any(|n| !(0..=10000).contains(n)) || ships.values().sum::<i64>() > 10000 {
            return Err("Expedition: höchstens 10.000 Schiffe insgesamt".into());
        }
        let pf = ships.get(&Einheit::Pathfinder).copied().unwrap_or(0);
        if ships.get(&Einheit::Spionagesonde).copied().unwrap_or(0) > pf.saturating_mul(20) {
            return Err("Höchstens 20 Bordsonden je Pathfinder".into());
        }
        if ziel.position != 0 {
            return Err(
                "Expeditionen starten zu Position 0 eines Systems (Asteroiden/Deep Space)".into(),
            );
        }
        let pid = self.eigener_planet(sid, start)?;
        let mut next = self.clone();
        let fid = next.naechste_flotte;
        let (ok, why) = next.handeln(sid, rolle, action);
        if !ok {
            return Err(why);
        }
        let f = next.flotten[&fid].clone();
        // Reserve provisions for every passenger seat, so unexpected rescues can return.
        let seats: i64 = f
            .schiffe
            .iter()
            .enumerate()
            .map(|(i, n)| n * next.regeln.personenplaetze(Einheit::ALLE[i]))
            .sum();
        let crew: i64 = f
            .schiffe
            .iter()
            .enumerate()
            .map(|(i, n)| n * next.regeln.einh(Einheit::ALLE[i]).besatzung)
            .sum();
        let seats = seats + crew;
        let hours = (2 * f.flugdauer + 20 * STUNDE + STUNDE - 1) / STUNDE;
        let cost = next.bordversorgung(seats * M, seats * M, hours)?;
        next.abrechnen(pid);
        next.zahlbar(pid, &cost, None)?;
        next.zahlen(pid, &cost, None);
        let mut rng = strom(next.startwert, 91, fid as u64);
        next.ausbau.expeditionen.insert(
            fid,
            Expedition {
                besitzer: sid,
                ereignisse: 0,
                naechstes: f.ankunft,
                scan: false,
                zielart: 0,
                risikowurf: zufall(&mut rng, 1_000_000) as u32,
                totalwurf: zufall(&mut rng, 1000) as u16,
                status: "hinflug".into(),
                frist: None,
                rettungen: BTreeMap::new(),
                ruempfe: BTreeMap::new(),
                berichte: Vec::new(),
            },
        );
        next.expedition_bericht(fid,"Die Expedition ist gestartet. Vorräte, Bordsonden und Rückflugkraftstoff sind verbucht.");
        *self = next;
        Ok(format!(
            "Expedition {fid} gestartet; höchstens {} Ereignisse",
            ereignislimit(pf)
        ))
    }
    pub fn expedition_ankunft(&mut self, fid: FlottenId) -> bool {
        if let Some(target) = self.ausbau.rettungsziele.get(&fid).copied() {
            self.rettung_ankunft(fid, target);
            return true;
        }
        if !self.ausbau.expeditionen.contains_key(&fid) {
            return false;
        }
        if self.ausbau.expeditionen[&fid].status != "hinflug" {
            return true;
        }
        self.flotten.get_mut(&fid).unwrap().zustand = Flottenzustand::ImOrbit;
        self.ausbau.expeditionen.get_mut(&fid).unwrap().status = "aktiv".into();
        self.expedition_schritt(fid);
        true
    }
    pub fn expedition_schritt(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return;
        };
        let Some(e) = self.ausbau.expeditionen.get(&fid).cloned() else {
            return;
        };
        if e.naechstes != self.zeit || f.zustand != Flottenzustand::ImOrbit {
            return;
        }
        if e.status == "vermisst" {
            let last = e
                .rettungen
                .values()
                .copied()
                .max()
                .unwrap_or(0)
                .max(e.frist.unwrap_or(0))
                + 1;
            if self.zeit < last {
                self.ausbau.expeditionen.get_mut(&fid).unwrap().naechstes = last;
                self.plane(last, EreignisArt::ExpeditionSchritt { flotte: fid });
                return;
            }
            self.expedition_verlust(
                fid,
                "Das Rettungsfenster ist abgelaufen. Die Expedition gilt endgültig als verloren.",
            );
            return;
        }
        if e.status != "aktiv" {
            return;
        }
        let pf = f.schiffe[Einheit::Pathfinder.idx()];
        if !e.scan && e.ereignisse >= ereignislimit(pf) {
            self.expedition_abschluss(fid);
            return;
        }
        let mut rng = strom(self.startwert, 93, (fid as u64) << 32 | e.ereignisse as u64);
        if !e.scan {
            let kind = zufall(&mut rng, 8) as u8;
            let probes = f.schiffe[Einheit::Spionagesonde.idx()].min(4).min(pf * 20);
            let entry = self.ausbau.expeditionen.get_mut(&fid).unwrap();
            entry.scan = true;
            entry.zielart = kind;
            entry.naechstes = self.zeit + if probes > 0 { 15 * MINUTE } else { STUNDE };
            let next = entry.naechstes;
            if probes > 0 {
                let lost = if zufall(&mut rng, 100) < 5 { 1 } else { 0 };
                self.flotten.get_mut(&fid).unwrap().schiffe[Einheit::Spionagesonde.idx()] -= lost;
                self.expedition_bericht(fid,&format!("{probes} Bordsonden scannen das nächste Ziel. {} kehren zurück; {lost} gehen verloren. Der Scan dauert 15 Spielminuten.",probes-lost));
            }
            self.plane(next, EreignisArt::ExpeditionSchritt { flotte: fid });
            return;
        }
        // Every visited/skipped target consumes one event. The counter is never reset by finds.
        self.ausbau.expeditionen.get_mut(&fid).unwrap().ereignisse += 1;
        let amount = 100 + zufall(&mut rng, 401) as i64;
        match e.zielart {
            0|1=>{
                let good=if e.zielart==0{Gut::Erz}else{Gut::Kristall};
                let capacity=self.flugplan(f.besitzer,f.start as usize,f.ziel,&f.schiffe,1000).map(|p|p.kapazitaet).unwrap_or(0);
                let free=(capacity-f.ladung.iter().sum::<i64>()).max(0);let found=(amount*(1+pf.min(32))*M).min(free);
                self.flotten.get_mut(&fid).unwrap().ladung[good.idx()]+=found;
                self.expedition_bericht(fid,&format!("Ein Asteroidenfeld liefert {} {good}. Die Menge wurde im freien Frachtraum verstaut.",found/M));
            },
            2=>{
                let seats:i64=f.schiffe.iter().enumerate().map(|(i,n)|n*self.regeln.personenplaetze(Einheit::ALLE[i])).sum();
                let aboard=self.ausbau.passagiere.get(&fid).map(|p|p.gesamt()/M).unwrap_or(0);
                let found=(amount+f.schiffe[Einheit::Bergungsschiff.idx()]*100).min((seats-aboard).max(0));
                let volk=zufall(&mut rng,VOELKER as u64) as usize;let q=match zufall(&mut rng,10){0=>2,1|2=>1,_=>0};
                self.ausbau.passagiere.entry(fid).or_default().gruppen[volk][q]+=found*M;
                self.expedition_bericht(fid,&format!("{} Überlebende der {} wurden aufgenommen. Herkunft und Qualifikation bleiben erhalten; die verfügbaren Personenplätze begrenzen die Bergung.",found,Volk::ALLE[volk]));
            },
            3|4=>{
                let ship=if e.zielart==3{Einheit::Pathfinder}else{Einheit::Kreuzer};
                *self.ausbau.expeditionen.get_mut(&fid).unwrap().ruempfe.entry(ship).or_default()+=1;
                self.expedition_bericht(fid,&format!("Ein beschädigter {ship} wurde gesichert. Er benötigt Material, Besatzung und Rückflugtreibstoff; der Fund schaltet keinen Nachbau frei."));
            },
            5=>{
                let mut enemies=[0;EINHEITEN];enemies[Einheit::LeichterJaeger.idx()]=3+zufall(&mut rng,12) as i64;
                let mut own=[0;EINHEITEN];own[..SCHIFFE].copy_from_slice(&f.schiffe);
                let a=Gruppe::neu(&self.regeln,&self.spieler[f.besitzer as usize],own);
                let d=Gruppe::mit_werten(&self.regeln,f.besitzer,enemies,1.0,1.0,1.0);
                // A scan estimates strength; it is not an automatic exemption from every pirate encounter.
                let strength=|g:&Gruppe| (0..SCHIFFE).map(|i|g.einheiten[i] as i128*(g.angriff[i] as i128*10+g.schild[i] as i128+g.struktur[i] as i128/10)).sum::<i128>();
                if pf>0&&f.schiffe[Einheit::Spionagesonde.idx()]>0&&strength(&d)>strength(&a){
                    self.expedition_bericht(fid,"Die Aufklärung erkennt eine überlegene Piratenflotte. Das Ziel wird übersprungen und zählt trotzdem als Ereignis.");
                }else{
                    let result=kampf(&self.regeln,&[a],&[d],&mut rng);
                    self.flotten.get_mut(&fid).unwrap().schiffe.copy_from_slice(&result.angreifer[0][..SCHIFFE]);
                    if result.angreifer[0][..SCHIFFE].iter().sum::<i64>()==0{self.expedition_verlust(fid,"Piraten haben die Expedition vernichtet. Es gibt keine überlebenden Schiffe.");return;}
                    // Verluste je Schiffstyp nennen (vorher nur die Rundenzahl).
                    let verluste:Vec<String>=(0..SCHIFFE).filter(|i|f.schiffe[*i]>result.angreifer[0][*i]).map(|i|format!("{} {}",f.schiffe[i]-result.angreifer[0][i],Einheit::ALLE[i])).collect();
                    let piraten=enemies[Einheit::LeichterJaeger.idx()];
                    self.expedition_bericht(fid,&format!("Die Flotte übersteht ein Piratengefecht gegen {piraten} Piratenjäger über {} Runden; eigene Verluste: {}. Nur die tatsächlichen Überlebenden setzen den Flug fort.",result.runden,if verluste.is_empty(){"keine".to_string()}else{verluste.join(", ")}));
                }
            },
            6=>{
                let capacity=self.flugplan(f.besitzer,f.start as usize,f.ziel,&f.schiffe,1000).map(|p|p.kapazitaet).unwrap_or(0);
                if capacity-f.ladung.iter().sum::<i64>()>=M{self.flotten.get_mut(&fid).unwrap().ladung[Gut::Habitatmodul.idx()]+=M;self.expedition_bericht(fid,"Ein intaktes Habitatmodul wurde geborgen und als Fracht verbucht.");}
                else{self.expedition_bericht(fid,"Das Bauteil bleibt zurück: Der Frachtraum ist voll.");}
            },
            _=>self.expedition_bericht(fid,"Das Nebelfeld enthält keine verwertbaren Funde. Die Untersuchung ist abgeschlossen."),
        }
        self.expedition_kapazitaet(fid);
        let entry = self.ausbau.expeditionen.get_mut(&fid).unwrap();
        entry.scan = false;
        entry.naechstes = self.zeit + STUNDE;
        let next = entry.naechstes;
        self.plane(next, EreignisArt::ExpeditionSchritt { flotte: fid });
    }
    fn expedition_rueckweg_anpassen(&mut self, fid: FlottenId) {
        let Some(f) = self.flotten.get(&fid).cloned() else {
            return;
        };
        if let Ok(plan) = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000) {
            // Do not accelerate the original cruise; newly added slower hulls can extend it.
            self.flotten.get_mut(&fid).unwrap().flugdauer = f.flugdauer.max(plan.dauer);
        }
    }
    fn expedition_kapazitaet(&mut self, fid: FlottenId) {
        let f = &self.flotten[&fid];
        let seats: i64 = f
            .schiffe
            .iter()
            .enumerate()
            .map(|(i, n)| n * self.regeln.personenplaetze(Einheit::ALLE[i]))
            .sum();
        let mut verloren_personen = 0;
        if let Some(p) = self.ausbau.passagiere.get_mut(&fid) {
            let vorher = p.gesamt();
            p.begrenzen(seats * M);
            verloren_personen = (vorher - p.gesamt()) / M;
        }
        let capacity = self
            .flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000)
            .map(|p| p.kapazitaet)
            .unwrap_or(0);
        let f = self.flotten.get_mut(&fid).unwrap();
        let total: i64 = f.ladung.iter().sum();
        let mut zurueck = [0i64; GUETER];
        if total > capacity {
            for (i, g) in f.ladung.iter_mut().enumerate() {
                let neu = anteil(*g, capacity, total);
                zurueck[i] = *g - neu;
                *g = neu;
            }
        }
        // Still gekappte Fracht und Passagiere melden: Die gemeldete Fundmenge muss zur Ladung passen.
        if verloren_personen > 0 {
            self.expedition_bericht(fid, &format!("{verloren_personen} Personen fanden keinen Platz mehr an Bord (Personenplätze nach Verlusten zu knapp) und blieben zurück."));
        }
        if zurueck.iter().any(|n| *n >= M) {
            self.expedition_bericht(fid, &format!("Laderaum nach Verlusten zu klein – zurückgelassen: {}.", crate::berichte::fracht_text(&zurueck)));
        }
    }
    pub(crate) fn expedition_abschluss(&mut self, fid: FlottenId) {
        let pf = self.flotten[&fid].schiffe[Einheit::Pathfinder.idx()];
        let e = self.ausbau.expeditionen.get_mut(&fid).unwrap();
        if e.risikowurf < risiko_ppm(pf) {
            if e.totalwurf < 100 {
                self.expedition_verlust(
                    fid,
                    "Das letzte Signal bricht unwiederbringlich ab. Dieser Verlust ist unrettbar.",
                );
                return;
            }
            e.status = "vermisst".into();
            e.frist = Some(self.zeit + TAG);
            e.naechstes = self.zeit + TAG + 1;
            self.plane(
                self.zeit + TAG + 1,
                EreignisArt::ExpeditionSchritt { flotte: fid },
            );
            self.expedition_bericht(fid,"Die Expedition wird vermisst. Eine Rettungsflotte muss innerhalb von 24 Spielstunden starten; ihre direkte Anflugzeit wird zusätzlich gewährt.");
        } else {
            e.status = "rueckflug".into();
            self.expedition_bericht(fid,"Die Expedition beendet die Suche und tritt mit den verbuchten Funden den Rückflug an.");
            self.heimflug(fid);
        }
    }
    fn expedition_verlust(&mut self, fid: FlottenId, text: &str) {
        self.expedition_bericht(fid, text);
        self.ausbau.expeditionen.get_mut(&fid).unwrap().status = "verloren".into();
        self.flotten.remove(&fid);
        self.ausbau.passagiere.remove(&fid);
    }
    pub fn expedition_reparieren(
        &mut self,
        sid: SpielerId,
        fid: FlottenId,
        ship: Einheit,
    ) -> Result<String, String> {
        let f = self
            .flotten
            .get(&fid)
            .cloned()
            .filter(|f| f.besitzer == sid)
            .ok_or("Eigene Expedition fehlt")?;
        let e = self
            .ausbau
            .expeditionen
            .get(&fid)
            .ok_or("Keine Expedition")?;
        if e.status != "aktiv"
            || f.schiffe[Einheit::Bergungsschiff.idx()] == 0
            || e.ruempfe.get(&ship).copied().unwrap_or(0) == 0
        {
            return Err("Aktive Expedition, Bergungsschiff und reparierbarer Rumpf nötig".into());
        }
        let crew = self.regeln.einh(ship).besatzung * M;
        let mut people = self
            .ausbau
            .passagiere
            .get(&fid)
            .cloned()
            .unwrap_or_default();
        let taken = people.auswahl(crew);
        if taken.gesamt() != crew {
            return Err("Zu wenige gerettete Personen als neue Crew".into());
        }
        let mut cost = self
            .regeln
            .kosten_einheit(ship, self.spieler[sid as usize].volk)
            .map(|n| anteil(n, 1, 5));
        let mut single = [0; SCHIFFE];
        single[ship.idx()] = 1;
        cost[Gut::Deuterium.idx()] += self
            .flugplan(sid, f.start as usize, f.ziel, &single, 1000)?
            .treibstoff;
        if cost.iter().zip(f.ladung).any(|(need, have)| *need > have) {
            return Err("20 % Baukosten und Rückflugtreibstoff müssen an Bord liegen".into());
        }
        people.nehmen(&taken)?;
        self.ausbau.passagiere.insert(fid, people);
        self.ausbau
            .crew_abgaenge
            .entry(sid)
            .or_default()
            .add(&taken);
        let fleet = self.flotten.get_mut(&fid).unwrap();
        for i in 0..GUETER {
            fleet.ladung[i] -= cost[i];
        }
        fleet.schiffe[ship.idx()] += 1;
        self.expedition_rueckweg_anpassen(fid);
        let e = self.ausbau.expeditionen.get_mut(&fid).unwrap();
        *e.ruempfe.get_mut(&ship).unwrap() -= 1;
        e.naechstes += 15 * MINUTE;
        let next = e.naechstes;
        self.plane(next, EreignisArt::ExpeditionSchritt { flotte: fid });
        self.expedition_bericht(fid,&format!("Der {ship} ist nach einer Notreparatur einsatzbereit. Material, Besatzung und Treibstoff sind verbucht; die Reparatur kostet 15 Spielminuten."));
        Ok("Rumpf einsatzbereit; keine Baufreigabe erteilt".into())
    }
    pub fn rettung_starten(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        target: FlottenId,
        action: &serde_json::Value,
    ) -> Result<String, String> {
        let e = self
            .ausbau
            .expeditionen
            .get(&target)
            .filter(|e| e.besitzer == sid && e.status == "vermisst")
            .ok_or("Keine eigene vermisste Expedition")?;
        if self.zeit > e.frist.unwrap_or(0) {
            return Err("Die 24-Stunden-Startfrist ist abgelaufen".into());
        }
        let missing = self.flotten.get(&target).ok_or("Vermisste Flotte fehlt")?;
        let a: crate::aktion::Aktion =
            serde_json::from_value(action.clone()).map_err(|e| e.to_string())?;
        match a {
            crate::aktion::Aktion::FlotteSenden {
                ziel,
                mission: Mission::Saven,
                ref schiffe,
                ..
            } if ziel == missing.ziel
                && schiffe.get(&Einheit::Bergungsschiff).copied().unwrap_or(0) > 0 => {}
            _ => {
                return Err(
                    "Direkter Save-Flug mit Bergungsschiff zum Vermisstenziel erforderlich".into(),
                )
            }
        };
        let mut next = self.clone();
        let fid = next.naechste_flotte;
        let (ok, why) = next.handeln(sid, rolle, action);
        if !ok {
            return Err(why);
        }
        let fleet = next.flotten[&fid].clone();
        let arrival = fleet.ankunft;
        let seats: i64 = fleet
            .schiffe
            .iter()
            .enumerate()
            .map(|(i, n)| n * next.regeln.personenplaetze(Einheit::ALLE[i]))
            .sum();
        let crew: i64 = fleet
            .schiffe
            .iter()
            .enumerate()
            .map(|(i, n)| n * next.regeln.einh(Einheit::ALLE[i]).besatzung)
            .sum();
        let seats = seats + crew;
        let hours = (2 * fleet.flugdauer + STUNDE - 1) / STUNDE;
        let mut supply = next.bordversorgung(seats * M, seats * M, hours)?;
        // Recovery carries one still-crewed ship per salvage ship; fuel is prepaid for exactly these hulls.
        let mut tow = rettung_limit(
            fleet.schiffe[Einheit::Bergungsschiff.idx()],
            fleet.schiffe[Einheit::Pathfinder.idx()],
        );
        let mut recovered = [0; SCHIFFE];
        for ship in [Einheit::Pathfinder].into_iter().chain(
            Einheit::ALLE[..SCHIFFE]
                .iter()
                .copied()
                .filter(|e| *e != Einheit::Pathfinder),
        ) {
            let n = next.flotten[&target].schiffe[ship.idx()].min(tow);
            recovered[ship.idx()] = n;
            tow -= n;
        }
        if recovered.iter().any(|n| *n > 0) {
            supply[Gut::Deuterium.idx()] += next
                .flugplan(sid, fleet.start as usize, fleet.ziel, &recovered, 1000)?
                .treibstoff;
        }
        next.abrechnen(fleet.start as usize);
        next.zahlbar(fleet.start as usize, &supply, None)?;
        next.zahlen(fleet.start as usize, &supply, None);

        next.ausbau.rettungsziele.insert(fid, target);
        next.ausbau
            .expeditionen
            .get_mut(&target)
            .unwrap()
            .rettungen
            .insert(fid, arrival);
        *self = next;
        Ok(format!(
            "Rettung {fid} gestartet; direkte Ankunft {arrival} bleibt fristgerecht"
        ))
    }
    fn rettung_ankunft(&mut self, fid: FlottenId, target: FlottenId) {
        self.ausbau.rettungsziele.remove(&fid);
        let valid =
            self.ausbau.expeditionen.get(&target).is_some_and(|e| {
                e.status == "vermisst" && e.rettungen.get(&fid) == Some(&self.zeit)
            });
        if let Some(e) = self.ausbau.expeditionen.get_mut(&target) {
            e.rettungen.remove(&fid);
        }
        if !valid {
            self.heimflug(fid);
            return;
        }
        let rescue = self.flotten[&fid].clone();
        let lost = self.flotten[&target].clone();
        let seats: i64 = rescue
            .schiffe
            .iter()
            .enumerate()
            .map(|(i, n)| n * self.regeln.personenplaetze(Einheit::ALLE[i]))
            .sum();
        let remaining = self.ausbau.passagiere.entry(target).or_default();
        let taken = remaining.auswahl(seats * M);
        remaining.nehmen(&taken).expect("available survivors");
        self.ausbau.passagiere.entry(fid).or_default().add(&taken);
        let capacity = self
            .flugplan(
                rescue.besitzer,
                rescue.start as usize,
                rescue.ziel,
                &rescue.schiffe,
                1000,
            )
            .map(|p| p.kapazitaet)
            .unwrap_or(0);
        let free = (capacity - rescue.ladung.iter().sum::<i64>()).max(0);
        let total: i64 = lost.ladung.iter().sum();
        for g in 0..GUETER {
            let n = if total > free {
                anteil(lost.ladung[g], free, total)
            } else {
                lost.ladung[g]
            };
            self.flotten.get_mut(&target).unwrap().ladung[g] -= n;
            self.flotten.get_mut(&fid).unwrap().ladung[g] += n;
        }
        let mut tow = rettung_limit(
            rescue.schiffe[Einheit::Bergungsschiff.idx()],
            rescue.schiffe[Einheit::Pathfinder.idx()],
        );
        let mut recovered = 0;
        for ship in [Einheit::Pathfinder].into_iter().chain(
            Einheit::ALLE[..SCHIFFE]
                .iter()
                .copied()
                .filter(|e| *e != Einheit::Pathfinder),
        ) {
            let n = lost.schiffe[ship.idx()].min(tow);
            self.flotten.get_mut(&target).unwrap().schiffe[ship.idx()] -= n;
            self.flotten.get_mut(&fid).unwrap().schiffe[ship.idx()] += n;
            tow -= n;
            recovered += n;
        }
        self.expedition_rueckweg_anpassen(fid);
        self.expedition_bericht(target,&format!("{recovered} bereits bemannte Schiffe erhalten Navigationshilfe und kehren mit der Rettungsflotte zurück."));
        self.expedition_bericht(target,&format!("Die Rettungsflotte erreicht das Signal und übernimmt {} Personen sowie Fracht innerhalb ihrer Kapazität. Zurückbleibende Funde behalten die ursprüngliche Frist.",taken.gesamt()/M));
        self.heimflug(fid);
    }
}
