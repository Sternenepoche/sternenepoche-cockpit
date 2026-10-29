//! Simulationsschleife: Ereigniswarteschlange, Entscheidungsfenster, Wecklogik, Zustandshash.

use crate::typen::*;
use crate::welt::*;
use sha2::{Digest, Sha256};
use std::collections::BinaryHeap;

impl Welt {
    /// Interactive/model clients must finish every due role before advancing.
    /// Offline replay and baseline bots retain the lower-level `schritt` primitive.
    pub fn schritt_wenn_bereit(&mut self) -> Result<bool, String> {
        if self
            .faellig
            .iter()
            .any(|f| self.spieler[f.spieler as usize].weck[f.rolle.idx()].letzter < self.zeit)
        {
            return Err("Entscheidungsfenster wartet noch auf Rollen-Antworten".into());
        }
        Ok(self.schritt())
    }

    fn zeit_vorruecken(&mut self, ziel: SimZeit) {
        let dt = ziel - self.zeit;
        self.boden_vorruecken(dt);
        self.pausen_vorruecken(dt);
        if dt > 0 {
            let ruht: Vec<bool> = (0..self.planeten.len()).map(|pid| self.bau_ruht_unruhen(pid)).collect();
            for (p, ruht) in self.planeten.iter_mut().zip(ruht) {
                if ruht {
                    if let Some(auftrag) = p.bauschleife.first_mut() {
                        if let Some(fertig) = &mut auftrag.fertig {
                            *fertig += dt;
                        }
                    }
                }
            }
        }
        self.zeit = ziel;
    }
    pub fn beendet(&self) -> bool {
        self.zeit >= self.regeln.epochenende()
    }

    /// Rückt die Welt um ein Entscheidungsfenster vor. Während die Agenten entscheiden,
    /// steht die Weltuhr still: gerechnet wird nur hier.
    pub fn schritt(&mut self) -> bool {
        let fenster = self.regeln.fenster().max(1);
        self.schritt_bis((self.zeit / fenster + 1) * fenster)
    }

    /// Rückt höchstens bis zur nächsten Fenstergrenze vor (für sekundengenaue Echtzeit).
    /// Fensterbeginn und Fensterende laufen genau einmal je Fenster; bei ausgerichteter
    /// Zeit ist ein voller Aufruf identisch mit dem bisherigen Fensterschritt.
    pub fn schritt_bis(&mut self, ziel: SimZeit) -> bool {
        if self.beendet() || ziel <= self.zeit {
            return false;
        }
        self.wartende_auftraege_starten();
        self.produktionsserien_tick();
        self.flugtermine_tick();
        self.monde_tick();
        self.dyson_tick();
        let fenster = self.regeln.fenster().max(1);
        if self.zeit % fenster == 0 {
            self.kolonisation_fensterabschluss();
        }
        let grenze = (self.zeit / fenster + 1) * fenster;
        let ziel = ziel.min(grenze).min(self.regeln.epochenende());
        loop {
            let event=self.ereignisse.peek().map(|e|e.zeit);
            let ground=self.boden_naechste_fertigstellung();
            let flight=self.ausbau.flugtermine.values().map(|a|a.abflug).min();
            let moon=self.mond_naechster_termin();
            let dyson=self.dyson_naechster_termin();
            let Some(next)=event.into_iter().chain(ground).chain(flight).chain(moon).chain(dyson).min().filter(|t|*t<=ziel)else{break;};
            if event==Some(next){
                // Paused missile jobs move with time while still in the heap.
                self.zeit_vorruecken(next);
                if self.ereignisse.peek().is_some_and(|e|e.zeit<=self.zeit){
                    let mut e=self.ereignisse.pop().unwrap();
                    if matches!(e.art,EreignisArt::RaketenFertig{planet,..} if self.werk_ruht(planet as usize,Gebaeude::Raketensilo)) {
                        // Preserve the job identity and payment receipt if supply changed
                        // at this exact event time. Paused work remains cancellable.
                        if let Some(p)=self.ausbau.raketen_zahlung.get_mut(&e.seq){for b in &mut p.lose{b.ende+=60;}}e.zeit=self.zeit+60;self.ereignisse.push(e);
                    }else if matches!(e.art,EreignisArt::RaketenFertig{..})&&self.ausbau.raketen_zahlung.get(&e.seq).is_some_and(|p|p.basis>0){self.silo_parallel_fertig(e);}else{self.ausfuehren(e.art);}
                }
            }else{self.zeit_vorruecken(next);}
            self.wartende_auftraege_starten();
            self.produktionsserien_tick();
            self.flugtermine_tick();
            self.monde_tick();
            self.dyson_tick();
        }
        self.zeit_vorruecken(ziel);
        self.ausbau_aufraeumen();
        self.wartende_auftraege_starten();
        self.produktionsserien_tick();
        self.flugtermine_tick();
        self.monde_tick();
        self.dyson_tick();
        if self.aufklaerungsregeln() {
            self.aufklaerung.abschirmung.retain(|id,_|self.flotten.contains_key(id));
            self.aufklaerung.sondenziele.retain(|id,_|self.flotten.contains_key(id));
        }
        if self.beendet() {
            self.punkte_neu();
        }
        if ziel == grenze || self.beendet() {
            self.kommunikation_tick();
            self.fenster_vorbereiten();
        }
        true
    }

    fn ausfuehren(&mut self, art: EreignisArt) {
        match art {
            EreignisArt::ExpeditionSchritt {flotte}=>self.expedition_schritt(flotte),
            EreignisArt::SensorKontakt { flotte } => self.sensor_kontakt(flotte),
            EreignisArt::InterneLieferung { angebot } => self.intern_liefern(angebot),
            EreignisArt::Tick => {
                self.tick();
                self.plane(self.zeit + STUNDE, EreignisArt::Tick);
            }
            EreignisArt::Tag => {
                self.tageswechsel();
                self.plane(self.zeit + TAG, EreignisArt::Tag);
            }
            EreignisArt::BauFertig { planet } => {if self.planet_aktiv(planet as usize){self.bau_fertig(planet as usize)}},
            EreignisArt::FertigungFertig { planet, schleife } => {
                if self.planet_aktiv(planet as usize){self.fertigung_fertig(planet as usize, schleife as usize)}
            }
            EreignisArt::FlotteAnkunft { flotte } => self.flotte_ankunft(flotte),
            EreignisArt::FlotteRueckkehr { flotte } => self.flotte_rueckkehr(flotte),
            EreignisArt::OrbitEnde { flotte } => self.orbit_ende(flotte),
            EreignisArt::Marktlieferung { planet, gut, menge } => {
                self.marktlieferung(planet as usize, gut, menge)
            }
            EreignisArt::MarktlieferungGebunden {
                planet,
                empfaenger,
                gut,
                menge,
            } => self.marktlieferung_gebunden(planet as usize, empfaenger, gut, menge),
            EreignisArt::VertragEnde { vertrag } => self.vertrag_ende(vertrag),
            EreignisArt::RaketenFertig {
                planet,
                besitzer,
                art,
                anzahl,
            } => {if self.spieler_aktiv(besitzer){if self.werk_ruht(planet as usize,Gebaeude::Raketensilo){self.plane(self.zeit+60,EreignisArt::RaketenFertig{planet,besitzer,art,anzahl});}else{self.raketen_fertig(planet as usize, besitzer, art, anzahl)}}},
            EreignisArt::TaktischeRakete {salve} => self.taktisch_ankunft(salve),
            EreignisArt::RaketenAnkunft {
                von,
                ziel,
                anzahl,
                zieltyp,
            } => {if self.spieler_aktiv(von){self.raketen_ankunft(von, ziel, anzahl, zieltyp)}},
        }
    }

    /// Lost die Reihenfolge der Spieler für dieses Fenster aus und bestimmt die fälligen Rollen.
    pub fn fenster_vorbereiten(&mut self) {
        let r = self.regeln.clone();
        let jetzt = self.zeit;
        let fenster_nr = (jetzt / r.fenster()) as u64;
        let mut rng = strom(self.startwert, KANAL_REIHENFOLGE, fenster_nr);
        let mut reihenfolge: Vec<SpielerId> = (0..self.spieler.len() as SpielerId).collect();
        mischen(&mut rng, &mut reihenfolge);

        // Feindliche Flotten, die neu in den Sensorbereich kommen, wecken den Feldherrn sofort.
        let mut warnungen: Vec<(FlottenId, SpielerId)> = Vec::new();
        for f in self.flotten.values() {
            if f.gemeldet || !f.mission.feindlich() || f.zustand != Flottenzustand::Hinflug {
                continue;
            }
            if let Some(zp) = self.belegung.get(&f.ziel) {
                if self.aufklaerungsregeln() && (f.besitzer==self.planeten[*zp as usize].besitzer || self.verbuendet(f.besitzer,self.planeten[*zp as usize].besitzer)){continue;}
                if jetzt >= f.ankunft - self.warnzeit(*zp as usize) {
                    warnungen.push((f.id, self.planeten[*zp as usize].besitzer));
                }
            }
        }
        for (fid, ziel) in warnungen {
            self.flotten.get_mut(&fid).unwrap().gemeldet = true;
            self.wecke(ziel, Rolle::Feldherr, "Feindliche Flotte im Anflug", true);
            for sid in 0..self.spieler.len() as SpielerId {
                if self.verbuendet(sid, ziel) {
                    self.wecke(
                        sid,
                        Rolle::Feldherr,
                        "Feindliche Flotte im Anflug auf einen Partner",
                        true,
                    );
                }
            }
        }
        // Leere Bauschleifen wecken den Verwalter, je Leerstand einmal.
        for pid in 0..self.planeten.len() {
            let p = &self.planeten[pid];
            if p.bauschleife.is_empty() && !p.leer_gemeldet {
                let besitzer = p.besitzer;
                self.planeten[pid].leer_gemeldet = true;
                self.wecke(besitzer, Rolle::Verwalter, "Bauschleife leer", false);
            }
        }

        let mut faellig = Vec::new();
        for sid in &reihenfolge {
            let sp = &self.spieler[*sid as usize];
            if !sp.ki || !self.spieler_aktiv(*sid) || self.beendet() {
                continue;
            }
            for rolle in [
                Rolle::Stratege,
                Rolle::Verwalter,
                Rolle::Feldherr,
                Rolle::Diplomat,
            ] {
                let w = &sp.weck[rolle.idx()];
                let takt = r.agenten.takt_stunden[&rolle] * STUNDE;
                let mut gruende: Vec<String> = Vec::new();
                if jetzt - w.letzter >= takt {
                    gruende.push("Regeltakt".into());
                }
                if w.wecker.map(|t| t <= jetzt).unwrap_or(false) {
                    gruende.push("Wecker".into());
                }
                if !w.ausloeser.is_empty()
                    && (w.dringend || jetzt - w.letzter >= r.agenten.frueheste_sekunden(rolle))
                {
                    gruende.extend(w.ausloeser.iter().cloned());
                }
                if !gruende.is_empty() {
                    faellig.push(Faellig {
                        spieler: *sid,
                        name: sp.name.clone(),
                        rolle,
                        gruende,
                    });
                }
            }
        }
        self.reihenfolge = reihenfolge;
        self.faellig = faellig;
    }

    /// SHA-256 über den gesamten Zustand. Zwei Läufe mit gleichem Startwert und
    /// gleichem Aktionsprotokoll liefern denselben Hash.
    pub fn hash(&self) -> String {
        crate::snapshot_layout::with_forschungen(self.forschungszahl(),||crate::snapshot_layout::with_fleets(self.flottenlayout(),||crate::snapshot_layout::with_layout(!self.aufklaerung.neues_layout, || self.hash_intern())))
    }
    fn hash_intern(&self) -> String {
        let mut kopie = self.clone();
        let ereignisse =
            std::mem::replace(&mut kopie.ereignisse, BinaryHeap::new()).into_sorted_vec();
        let mut h = Sha256::new();
        h.update(bincode::serialize(&kopie).expect("Zustand serialisierbar"));
        h.update(bincode::serialize(&ereignisse).expect("Ereignisse serialisierbar"));
        if self.ausbau.version>0 {h.update(b"ausbau-v1");h.update(serde_json::to_vec(&self.ausbau).expect("Ausbau"));}
        if !self.regeln.flottendoktrin.is_empty(){h.update(serde_json::to_vec(&self.regeln.flottendoktrin).expect("Flottendoktrin"));}
        if self.kolonisation.aktiv {
            if self.kolonisationsregeln_v2() {
                h.update(b"kolonisation-v2");
                h.update(bincode::serialize(&self.kolonisation).expect("Kolonisationszustand"));
            } else {
                h.update(b"kolonisation-v1");
                h.update(
                    bincode::serialize(&crate::kolonisation::KolonisationszustandV1::from(
                        &self.kolonisation,
                    ))
                    .expect("Kolonisationszustand"),
                );
            }
        }
        if !self.scans.is_empty() {
            h.update(b"sternkarte-v1");
            h.update(bincode::serialize(&self.scans).expect("Scans"));
        }
        if self.aufklaerung.neues_layout {
            h.update(b"aufklaerung-online-v1");
            h.update(bincode::serialize(&self.aufklaerung).expect("Aufklärungszustand"));
        }
        if self.ausscheiden.erweitert() {
            h.update(b"ausscheiden-v1");
            h.update(bincode::serialize(&self.ausscheiden).expect("Ausscheidestatus"));
        }
        if self.kommunikation.ereignisse_gesamt() > 0 || !self.kommunikation.systemkontakte.is_empty() || !self.kommunikation.allianz_tags.is_empty() {
            h.update(b"kommunikation-v1");
            h.update(serde_json::to_vec(&self.kommunikation).expect("Kommunikation"));
        }
        if self.monde.aktiv {
            h.update(b"monde-v1");
            h.update(serde_json::to_vec(&(&self.regeln.monde, &self.monde)).expect("Mondzustand"));
        }
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    fn flottenlayout(&self)->u8 {
        // STERNE14 (35 Schiffe) nur für Welten mit Terraformer im Regelwerk; alle anderen Mondwelten bleiben STERNE13.
        if self.monde.aktiv{return if self.terraformer_welt(){crate::snapshot_layout::AKTUELL}else{crate::snapshot_layout::MONDE};}
        if self.ausbau.version>0{return 3;}
        if self.regeln.flottendoktrin.contains_key(&Einheit::Handelsfrachter){2}
        else if !self.regeln.flottendoktrin.is_empty(){1}else{0}
    }
    /// Vollständiger Schnappschuss als Bytes.
    pub fn zu_bytes(&self) -> Vec<u8> {
        // Technik v1 speichert 20 Forschungen im Binärteil, alle anderen Welten wie bisher 19 (bzw. 17 im Altlayout).
        crate::snapshot_layout::with_forschungen(self.forschungszahl(), || self.zu_bytes_huelle())
    }
    fn zu_bytes_huelle(&self) -> Vec<u8> {
        if self.monde.aktiv {
            // STERNE13: Mondgeneration mit 34 Schiffen / 43 Einheiten; STERNE14: dieselbe Hülle mit Terraformer
            // (35 Schiffe / 44 Einheiten). Erweiterungen als JSON.
            let v=self.flottenlayout();
            let base=crate::snapshot_layout::with_fleets(v,||self.zu_bytes_v9());
            let mut out=if v==crate::snapshot_layout::AKTUELL{b"STERNE14".to_vec()}else{b"STERNE13".to_vec()};out.extend_from_slice(&(base.len() as u64).to_le_bytes());out.extend(base);
            out.extend(crate::snapshot_layout::with_fleets(v,||serde_json::to_vec(&(&self.regeln.flottendoktrin,&self.ausbau,&self.regeln.monde,&self.monde)).expect("V13 extension")));return out;
        }
        if self.ausbau.version>0 {
            let base=crate::snapshot_layout::with_fleets(3,||self.zu_bytes_v9());
            let mut out=b"STERNE12".to_vec();out.extend_from_slice(&(base.len() as u64).to_le_bytes());out.extend(base);
            out.extend(crate::snapshot_layout::with_fleets(3,||serde_json::to_vec(&(&self.regeln.flottendoktrin,&self.ausbau)).expect("V12 extension")));return out;
        }

        if !self.regeln.flottendoktrin.is_empty(){
            let base=crate::snapshot_layout::with_fleets(self.flottenlayout(),||self.zu_bytes_v9());
            let mut out=if self.flottenlayout()==2{b"STERNE11".to_vec()}else{b"STERNE10".to_vec()};out.extend_from_slice(&(base.len()as u64).to_le_bytes());out.extend(base);
            out.extend(serde_json::to_vec(&self.regeln.flottendoktrin).expect("Flottendoktrin"));return out;
        }
        self.zu_bytes_v9()
    }
    fn zu_bytes_v9(&self) -> Vec<u8> {
        let base=self.zu_bytes_v8();
        if self.kommunikation.ereignisse_gesamt()==0 && self.kommunikation.systemkontakte.is_empty() && self.kommunikation.allianz_tags.is_empty() {return base;}
        let mut out=b"STERNEP9".to_vec();
        out.extend_from_slice(&(base.len() as u64).to_le_bytes());out.extend(base);
        out.extend(serde_json::to_vec(&self.kommunikation).expect("Kommunikation"));out
    }
    fn zu_bytes_v8(&self) -> Vec<u8> {
        let base=self.zu_bytes_v7();
        if !self.ausscheiden.erweitert() {return base;}
        let mut out=b"STERNEP8".to_vec();
        out.extend_from_slice(&(base.len() as u64).to_le_bytes());out.extend(base);
        out.extend(bincode::serialize(&self.ausscheiden).expect("Ausscheidestatus"));out
    }
    fn zu_bytes_v7(&self) -> Vec<u8> {
        let base=self.zu_bytes_v6();
        if !self.aufklaerung.neues_layout || (!self.planeten.iter().any(|p|p.bauschleife.iter().any(|a|a.dauer>0)) && !self.spieler.iter().any(|s|s.forschung_aktiv.as_ref().is_some_and(|a|a.begonnen.is_some()))){return base;}
        let durations: Vec<Vec<i64>>=self.planeten.iter().map(|p|p.bauschleife.iter().map(|a|a.dauer).collect()).collect();
        let mut out=b"STERNEP7".to_vec();
        out.extend_from_slice(&(base.len() as u64).to_le_bytes());out.extend(base);
        out.extend(bincode::serialize(&durations).expect("Bauzeiten"));
        let research:Vec<Option<i64>>=self.spieler.iter().map(|s|s.forschung_aktiv.as_ref().and_then(|a|a.begonnen)).collect();
        out.extend(bincode::serialize(&research).expect("Forschungsbeginn"));out
    }
    fn zu_bytes_v6(&self) -> Vec<u8> {
        let base = crate::snapshot_layout::with_layout(!self.aufklaerung.neues_layout, || self.zu_bytes_mit_scans());
        if !self.aufklaerung.neues_layout { return base; }
        let mut out = b"STERNEP6".to_vec();
        out.extend_from_slice(&(base.len() as u64).to_le_bytes());
        out.extend(base);
        out.extend(bincode::serialize(&self.aufklaerung).expect("Aufklärungszustand"));
        out
    }
    fn zu_bytes_mit_scans(&self) -> Vec<u8> {
        let base = self.zu_bytes_legacy();
        if self.scans.is_empty() { return base; }
        let mut out = b"STERNEP5".to_vec();
        out.extend_from_slice(&(base.len() as u64).to_le_bytes());
        out.extend(base);
        out.extend(bincode::serialize(&self.scans).expect("Scans"));
        out
    }

    fn zu_bytes_legacy(&self) -> Vec<u8> {
        let base = bincode::serialize(self).expect("Zustand serialisierbar");
        if !self.kolonisation.aktiv {
            return base;
        }
        let mut out = if self.kolonisationsregeln_v2() {
            b"STERNEP4".to_vec()
        } else {
            b"STERNEP3".to_vec()
        };
        out.extend_from_slice(&(base.len() as u64).to_le_bytes());
        out.extend(base);
        if self.kolonisationsregeln_v2() {
            out.extend(bincode::serialize(&self.kolonisation).expect("Kolonisationszustand"));
        } else {
            out.extend(
                bincode::serialize(&crate::kolonisation::KolonisationszustandV1::from(
                    &self.kolonisation,
                ))
                .expect("Kolonisationszustand"),
            );
        }
        out
    }

    pub fn aus_bytes(b: &[u8]) -> Result<Welt, String> {
        if b.starts_with(b"STERNE13") || b.starts_with(b"STERNE14") {
            let len=u64::from_le_bytes(b.get(8..16).ok_or("V13 Kopf fehlt")?.try_into().unwrap()) as usize;
            let end=16usize.checked_add(len).filter(|n|*n<=b.len()).ok_or("V13 Länge ungültig")?;
            if [b"STERNE14",b"STERNE13",b"STERNE12",b"STERNE11",b"STERNE10"].iter().any(|h|b[16..end].starts_with(*h)){return Err("Verschachtelter Flottensnapshot".into());}
            let v=if b.starts_with(b"STERNE14"){crate::snapshot_layout::AKTUELL}else{crate::snapshot_layout::MONDE};
            type Ext=(std::collections::BTreeMap<Einheit,crate::flottenausbau::FlottenDoktrin>,crate::ausbau::Ausbau,Option<std::sync::Arc<crate::monde::MondRegeln>>,crate::monde::Mondzustand);
            // Erweiterung zuerst: `ausbau.technik` legt die Länge des Forschungsfelds im Binärteil fest.
            let (doctrine,expansion,regeln,monde):Ext=serde_json::from_slice(&b[end..]).map_err(|e|format!("V13 Erweiterung: {e}"))?;
            let forschungen=if expansion.technik.is_some(){FORSCHUNGEN}else{crate::snapshot_layout::FORSCHUNGEN_V6};
            let mut w=crate::snapshot_layout::with_forschungen(forschungen,||crate::snapshot_layout::with_fleets(v,||Self::aus_bytes(&b[16..end])))?;
            let r=std::sync::Arc::make_mut(&mut w.regeln);r.flottendoktrin=doctrine;r.monde=regeln;r.ablauf=expansion.ablauf;r.technik=expansion.technik.clone();
            w.ausbau=expansion;w.monde=monde;
            // Die Hülle muss zum Regelwerk passen: STERNE14 genau dann, wenn der Terraformer zum Regelwerk gehört.
            if (v==crate::snapshot_layout::AKTUELL)!=w.terraformer_welt(){return Err("Snapshot-Hülle passt nicht zum Terraformer im Regelwerk".into());}
            w.personen_validieren()?;w.monde_validieren()?;w.dyson_nach_laden()?;w.terraform_validieren()?;return Ok(w);
        }
        if b.starts_with(b"STERNE12") {
            let len=u64::from_le_bytes(b.get(8..16).ok_or("V12 Kopf fehlt")?.try_into().unwrap()) as usize;
            let end=16usize.checked_add(len).filter(|n|*n<=b.len()).ok_or("V12 Länge ungültig")?;
            if [b"STERNE14",b"STERNE13",b"STERNE12",b"STERNE11",b"STERNE10"].iter().any(|h|b[16..end].starts_with(*h)){return Err("Verschachtelter Flottensnapshot".into());}
            let (doctrine,expansion):(_,crate::ausbau::Ausbau)=serde_json::from_slice(&b[end..]).map_err(|e|format!("V12 Erweiterung: {e}"))?;
            let forschungen=if expansion.technik.is_some(){FORSCHUNGEN}else{crate::snapshot_layout::FORSCHUNGEN_V6};
            let mut w=crate::snapshot_layout::with_forschungen(forschungen,||crate::snapshot_layout::with_fleets(3,||Self::aus_bytes(&b[16..end])))?;
            let r=std::sync::Arc::make_mut(&mut w.regeln);r.flottendoktrin=doctrine;r.ablauf=expansion.ablauf;r.technik=expansion.technik.clone();w.ausbau=expansion;
            w.personen_validieren()?;w.dyson_nach_laden()?;return Ok(w);
        }

        if b.starts_with(b"STERNE10") || b.starts_with(b"STERNE11") {
            let len=u64::from_le_bytes(b.get(8..16).ok_or("Snapshotkopf fehlt")?.try_into().unwrap())as usize;
            let end=16usize.checked_add(len).filter(|n|*n<=b.len()).ok_or("Snapshotlänge ungültig")?;
            if b[16..end].starts_with(b"STERNE10")||b[16..end].starts_with(b"STERNE11"){return Err("Verschachtelter Flottensnapshot".into());}
            let mut w=crate::snapshot_layout::with_fleets(if b.starts_with(b"STERNE11"){2}else{1},||Self::aus_bytes(&b[16..end]))?;
            std::sync::Arc::make_mut(&mut w.regeln).flottendoktrin=serde_json::from_slice(&b[end..]).map_err(|e|e.to_string())?;
            return Ok(w);
        }
        if b.starts_with(b"STERNEP9") {
            let len=u64::from_le_bytes(b.get(8..16).ok_or("Snapshotkopf fehlt")?.try_into().unwrap()) as usize;
            let end=16usize.checked_add(len).filter(|n|*n<=b.len()).ok_or("Snapshotlänge ungültig")?;
            if b[16..end].starts_with(b"STERNEP9") {return Err("Verschachtelter V9-Snapshot".into());}
            let mut w=Self::aus_bytes(&b[16..end])?;
            w.kommunikation=serde_json::from_slice(&b[end..]).map_err(|e|e.to_string())?;
            if w.kommunikation.briefe.keys().any(|id|*id==0||*id>w.nachrichten.len() as u64) {return Err("Ungültige Briefreferenz".into());}
            return Ok(w);
        }
        if b.starts_with(b"STERNEP8") {
            let len=u64::from_le_bytes(b.get(8..16).ok_or("Snapshotkopf fehlt")?.try_into().unwrap()) as usize;
            let end=16usize.checked_add(len).filter(|n|*n<=b.len()).ok_or("Snapshotlänge ungültig")?;
            if !b[16..end].starts_with(b"STERNEP6") && !b[16..end].starts_with(b"STERNEP7") {return Err("V8 benötigt V6/V7-Basis".into());}
            let mut w=Self::aus_bytes(&b[16..end])?;
            w.ausscheiden=bincode::deserialize(&b[end..]).map_err(|e|e.to_string())?;
            w.ausscheiden.validieren(&w)?;return Ok(w);
        }
        if b.starts_with(b"STERNEP7") {
            let len=u64::from_le_bytes(b.get(8..16).ok_or("Snapshotkopf fehlt")?.try_into().unwrap()) as usize;
            let end=16usize.checked_add(len).filter(|n|*n<=b.len()).ok_or("Snapshotlänge ungültig")?;
            if !b[16..end].starts_with(b"STERNEP6"){return Err("V7 benötigt V6-Basis".into());}
            let mut w=Self::aus_bytes(&b[16..end])?;
            let mut timing=std::io::Cursor::new(&b[end..]);
            let durations:Vec<Vec<i64>>=bincode::deserialize_from(&mut timing).map_err(|e|e.to_string())?;
            if durations.len()!=w.planeten.len(){return Err("Bauzeiten passen nicht zu Planeten".into());}
            for(p,d) in w.planeten.iter_mut().zip(durations){
                if p.bauschleife.len()!=d.len() || d.iter().any(|n|*n<0){return Err("Ungültige Bauzeiten".into());}
                for(a,n) in p.bauschleife.iter_mut().zip(d){a.dauer=n;}
            }
            // Optional appended extension: the earliest V7 snapshots only had building durations.
            if timing.position()<(b.len()-end) as u64 {
                let research:Vec<Option<i64>>=bincode::deserialize_from(&mut timing).map_err(|e|e.to_string())?;
                if research.len()!=w.spieler.len() || research.iter().flatten().any(|n|*n<0||*n>w.zeit){return Err("Ungültiger Forschungsbeginn".into());}
                for(s,begin) in w.spieler.iter_mut().zip(research){if let Some(a)=s.forschung_aktiv.as_mut(){a.begonnen=begin;}}
            }
            return Ok(w);
        }
        if b.starts_with(b"STERNEP6") {
            let len = u64::from_le_bytes(b.get(8..16).ok_or("Snapshotkopf fehlt")?.try_into().unwrap()) as usize;
            let end = 16usize.checked_add(len).filter(|n|*n<=b.len()).ok_or("Snapshotlänge ungültig")?;
            if b[16..end].starts_with(b"STERNEP6") { return Err("Verschachtelter V6-Snapshot".into()); }
            let mut w = crate::snapshot_layout::with_layout(false, ||Self::aus_bytes_intern(&b[16..end]))?;
            w.aufklaerung = bincode::deserialize(&b[end..]).map_err(|e|e.to_string())?;
            if !w.aufklaerung.neues_layout { return Err("V6-Snapshot ohne erweitertes Layout".into()); }
            return Ok(w);
        }
        crate::snapshot_layout::with_layout(true, ||Self::aus_bytes_intern(b))
    }
    fn aus_bytes_intern(b: &[u8]) -> Result<Welt, String> {
        if b.starts_with(b"STERNEP5") {
            let len = u64::from_le_bytes(b.get(8..16).ok_or("Snapshotkopf fehlt")?.try_into().unwrap()) as usize;
            let end = 16usize.checked_add(len).filter(|n| *n <= b.len()).ok_or("Snapshotlänge ungültig")?;
            if b[16..end].starts_with(b"STERNEP5") { return Err("Verschachtelter V5-Snapshot".into()); }
            let mut w = Self::aus_bytes_intern(&b[16..end])?;
            w.scans = bincode::deserialize(&b[end..]).map_err(|e| e.to_string())?;
            return Ok(w);
        }
        if b.starts_with(b"STERNEP3") || b.starts_with(b"STERNEP4") {
            let len = u64::from_le_bytes(
                b.get(8..16)
                    .ok_or("Unvollständiger Snapshotkopf")?
                    .try_into()
                    .unwrap(),
            ) as usize;
            let end = 16usize
                .checked_add(len)
                .filter(|end| *end <= b.len())
                .ok_or("Snapshotlänge ungültig")?;
            let mut w: Welt = bincode::deserialize(&b[16..end]).map_err(|e| e.to_string())?;
            if b.starts_with(b"STERNEP4") {
                w.kolonisation = bincode::deserialize(&b[end..]).map_err(|e| e.to_string())?;
                if !w.kolonisationsregeln_v2() {
                    return Err("V4-Snapshot benötigt Kolonisationsregeln v2".into());
                }
            } else {
                let old: crate::kolonisation::KolonisationszustandV1 =
                    bincode::deserialize(&b[end..]).map_err(|e| e.to_string())?;
                w.kolonisation = old.into();
            }
            return Ok(w);
        }
        bincode::deserialize(b).map_err(|e| format!("Schnappschuss nicht lesbar: {e}"))
    }

    /// Holt die seit dem letzten Aufruf angefallenen Protokolleinträge ab.
    pub fn log_abholen(&mut self) -> Vec<Logeintrag> {
        std::mem::take(&mut self.logpuffer)
    }
}
