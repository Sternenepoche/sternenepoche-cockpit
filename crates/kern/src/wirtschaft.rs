//! Wirtschaftskern: Raten, Stundentick, Bauschleifen, Fertigung, Forschung, Stufen.

use crate::typen::*;
use crate::welt::*;

impl Welt {
    pub fn lagergrenze(&self, pid: usize) -> [i64; GUETER] {
        let base = self.regeln.lagergrenze(0);
        let mut capacity = self
            .regeln
            .lagergrenze(self.planeten[pid].gebaeude[Gebaeude::Lager.idx()]);
        let integritaet = self.integritaet(pid, Gebaeude::Lager);
        for i in 0..GUETER {
            capacity[i] = base[i] + anteil(capacity[i] - base[i], integritaet, 1000);
        }
        capacity
    }

    /// Geschlossene Ressourcenrechnung: Bestand seit der letzten Änderung fortschreiben.
    /// Produktion endet an der Lagergrenze, Bestände werden nie negativ.
    pub fn abrechnen(&mut self, pid: usize) {
        let jetzt = self.zeit;
        if !self.planet_aktiv(pid) {
            self.planeten[pid].stand = jetzt;
            return;
        }
        let grenze = self.lagergrenze(pid);
        let p = &mut self.planeten[pid];
        let dt = jetzt - p.stand;
        if dt <= 0 {
            return;
        }
        let ergebnis = if self.ausbau.industrie_version>0 {
            let rest=self.ausbau.bestandsrest.get(&p.id).copied().unwrap_or([0;GUETER]);
            let (result,next)=crate::planer::bestand::fortschreiben_mit_rest(&p.bestand,&p.rate,&grenze,dt,&rest);
            if next.iter().any(|n|*n!=0){self.ausbau.bestandsrest.insert(p.id,next);}else{self.ausbau.bestandsrest.remove(&p.id);}
            result
        }else{crate::planer::bestand::fortschreiben(&p.bestand, &p.rate, &grenze, dt)};
        p.bestand = ergebnis.bestand;
        p.stand = jetzt;
        let besitzer = p.besitzer as usize;
        let wert = self.regeln.wert(&ergebnis.lagerverlust);
        self.spieler[besitzer].statistik.lagerverlust += wert;
    }

    /// Reihenfolge, in der Gebäude Arbeitskräfte bekommen: erst die Liste des Agenten, dann der Rest.
    pub fn reihenfolge(p: &Planet) -> Vec<Gebaeude> {
        let mut v = p.prioritaeten.clone();
        // Ohne Vorgabe zuerst die Versorgung: sonst nimmt die erste Mine dem Kraftwerk die Leute weg.
        let versorgung = [
            Gebaeude::Farm,
            Gebaeude::Solarkraftwerk,
            Gebaeude::Fusionskraftwerk,
        ];
        for g in versorgung.into_iter().chain(Gebaeude::ALLE) {
            if !v.contains(&g) {
                v.push(g);
            }
        }
        v
    }

    /// Rechnet Bestand ab und bestimmt alle Raten des Planeten neu.
    pub fn raten_neu(&mut self, pid: usize) {
        if !self.planet_aktiv(pid) {
            self.planeten[pid].rate = [0; GUETER];
            return;
        }
        self.abrechnen(pid);
        self.personen_abgleichen(pid);
        if self.ausbau.version>0 {
            let p=&mut self.planeten[pid];
            p.zone=crate::habitabilitaet::zone(p.koord.position,self.regeln.welt.plaetze_je_system);
            p.faktor=crate::habitabilitaet::faktoren(&self.regeln,p);
        }
        let ergebnis = self.industrie_berechnen(&self.planeten[pid]);
        let p = &mut self.planeten[pid];
        p.rate = ergebnis.rate;
        p.energie_erzeugung = ergebnis.energie_erzeugung;
        p.energie_verbrauch = ergebnis.energie_verbrauch;
        p.arbeit_bedarf = ergebnis.arbeit_bedarf;
        p.arbeit_verfuegbar = ergebnis.arbeit_verfuegbar;
        p.fk_bedarf = ergebnis.fk_bedarf;
        p.fk_verfuegbar = ergebnis.fk_verfuegbar;
        p.nahrung_deckung = ergebnis.nahrung_deckung;
        p.konsum_deckung = ergebnis.konsum_deckung;
        p.fp_rate = ergebnis.fp_rate;
        p.wohnraum = ergebnis.wohnraum;
        self.orbital_automatik(pid);
    }

    pub fn kolonien(&self, sid: SpielerId) -> usize {
        self.spieler[sid as usize].planeten.len().saturating_sub(1)
    }

    /// Kolonien, die die Verwaltungszentren ohne Stabilitätsverlust tragen.
    pub fn verwaltungsgrenze(&self, sid: SpielerId) -> usize {
        self.spieler[sid as usize]
            .planeten
            .iter()
            .map(|pid| {
                self.planeten[*pid as usize].gebaeude[Gebaeude::Verwaltungszentrum.idx()] as usize
                    / 2
            })
            .sum()
    }

    fn bevoelkerung_tick(&mut self, pid: usize) {
        if self.ausbau.version>0 {self.personen_tick(pid);return;}
        let r = self.regeln.clone();
        let w = &r.wirtschaft;
        let st = &r.stabilitaet;
        let p = &self.planeten[pid];
        let volk = r.volk(self.spieler[p.besitzer as usize].volk);
        let bev = p.bevoelkerung as i128;
        let wohn = p.wohnraum.max(M) as i128;
        let r_h = fx(w.wachstum_je_tag * volk.wachstum) / 24;
        let hmin = milli(w.hunger_min);
        let v_pm = (hmin + anteil(1000 - hmin, p.nahrung_deckung, 1000)) as i128;
        let voll = milli(st.wachstum_voll_ab);
        let null = milli(st.wachstum_null_unter);
        let s_pm = ((p.stabilitaet - null) * 1000 / (voll - null).max(1)).clamp(0, 1000) as i128;
        let mut d: i128 = 0;
        if bev > wohn {
            d = bev * r_h * (wohn - bev) / wohn / FX;
        } else if v_pm >= 0 {
            d = bev * r_h * (wohn - bev) / wohn * v_pm / 1000 * s_pm / 1000 / FX;
        }
        if v_pm < 0 {
            d = d.min(bev * r_h * v_pm / 1000 / FX);
        }
        let p = &mut self.planeten[pid];
        p.bevoelkerung = ((bev + d) as i64).max(100 * M);
    }

    fn stabilitaet_tick(&mut self, pid: usize) {
        let r = self.regeln.clone();
        let st = &r.stabilitaet;
        let jetzt = self.zeit;
        let besitzer = self.planeten[pid].besitzer;
        let kolonien = self.kolonien(besitzer) as i64;
        let getragen = self.verwaltungsgrenze(besitzer) as i64;
        let belagert = match self.planeten[pid].blockade {
            Some(fid) => self
                .flotten
                .get(&fid)
                .map(|f| f.mission == Mission::Invasion)
                .unwrap_or(false),
            None => false,
        };
        let sp = &self.spieler[besitzer as usize];
        let sozio = sp.forschung[Forschung::Soziologie.idx()] as i64;
        let steuer = sp.steuersatz as i64;
        let p = &mut self.planeten[pid];

        if belagert {
            p.belagerungsmalus += milli(st.belagerung_je_tag) / 24;
        } else {
            p.belagerungsmalus =
                (p.belagerungsmalus - milli(st.belagerung_erholung_je_tag) / 24).max(0);
        }
        p.mali.retain(|m| m.ende > jetzt);

        let mut z = milli(st.ziel_basis);
        z += anteil(milli(st.konsum_bonus), p.konsum_deckung, 1000);
        let frei_pm = if p.wohnraum > 0 {
            (p.wohnraum - p.bevoelkerung).max(0) * 1000 / p.wohnraum
        } else {
            0
        };
        let voll_pm = milli(st.wohnraum_frei_voll).max(1);
        z += anteil(milli(st.wohnraum_bonus), frei_pm.min(voll_pm), voll_pm);
        z += milli(st.soziologie_je_stufe) * sozio;
        if steuer > st.steuer_frei_bis as i64 {
            z -= milli(st.steuer_malus_je_punkt) * (steuer - st.steuer_frei_bis as i64);
        }
        if kolonien > getragen {
            z -= milli(st.kolonie_ueber_grenze) * (kolonien - getragen);
        }
        let mali: i64 = p
            .mali
            .iter()
            .map(|m| anteil(m.betrag, m.ende - jetzt, (m.ende - m.start).max(1)))
            .sum();
        z -= mali.min(milli(st.pluenderung_malus_max));
        z -= p.belagerungsmalus;
        z = z.clamp(0, 100 * M);
        p.stab_ziel = z;
        p.stabilitaet =
            (p.stabilitaet + mal(z - p.stabilitaet, st.annaeherung_je_stunde)).clamp(0, 100 * M);
    }

    /// Wirtschaftstick einmal pro Spielstunde.
    pub fn tick(&mut self) {
        self.klima_angleichen();
        let r = self.regeln.clone();
        let w = &r.wirtschaft;
        let mut einkommen = vec![0i64; self.spieler.len()];
        for pid in 0..self.planeten.len() {
            if !self.planet_aktiv(pid) { continue; }
            self.bevoelkerung_tick(pid);
            self.stabilitaet_tick(pid);
            let p = &self.planeten[pid];
            let sid = p.besitzer as usize;
            let volk = r.volk(self.spieler[sid].volk);
            let steuer = mal(
                anteil(p.bevoelkerung, self.spieler[sid].steuersatz as i64, 100),
                w.steuer_je_einwohner_stunde * volk.steuer,
            );
            self.spieler[sid].credits += steuer;
            self.raten_neu(pid);
            // Ablauf-Version 1: Was an der Lagergrenze verfällt, füllt keine Töpfe.
            let grenze = if self.ablauf_v1() { Some(self.lagergrenze(pid)) } else { None };
            let p = &self.planeten[pid];
            let mut plus = [0i64; GUETER];
            for g in 0..GUETER {
                plus[g] = p.rate[g].max(0);
                if grenze.is_some_and(|gr| p.bestand[g] >= gr[g]) {
                    plus[g] = 0;
                }
            }
            einkommen[sid] += r.wert(&plus) + steuer;
        }
        for sid in 0..self.spieler.len() {
            if !self.spieler_aktiv(sid as SpielerId) { continue; }
            // Einkommen nach den Anteilen der Doktrin auf die Töpfe verteilen.
            for t in 0..TOEPFE {
                let teil = einkommen[sid] * self.spieler[sid].anteile[t] as i64 / 100;
                self.spieler[sid].toepfe[t] += teil;
            }
            self.forschung_tick(sid as SpielerId);
            self.stufen_tick(sid as SpielerId);
        }
        for pid in 0..self.planeten.len() {
            if !self.planet_aktiv(pid) { continue; }
            self.bau_starten(pid);
        }
        self.eroberung_pruefen();
        self.ausscheiden_pruefen();
        self.punkte_neu();
    }

    // ---------------------------------------------------------------- Töpfe und Bezahlen

    /// Hat der Spieler eine sichtbare feindliche Flotte im Anflug?
    pub fn warnung_aktiv(&self, sid: SpielerId) -> bool {
        !self.sichtbare_angriffe(sid).is_empty()
    }

    pub(crate) fn topf_guthaben(&self, sid: SpielerId, topf: Topf) -> i64 {
        let sp = &self.spieler[sid as usize];
        let mut g = sp.toepfe[topf.idx()];
        if topf == Topf::Militaer && self.warnung_aktiv(sid) {
            g += sp.toepfe[Topf::Reserve.idx()].max(0);
        }
        g
    }

    fn topf_abbuchen(&mut self, sid: SpielerId, topf: Topf, betrag: i64) {
        let sp = &mut self.spieler[sid as usize];
        let aus_topf = betrag.min(sp.toepfe[topf.idx()].max(0));
        sp.toepfe[topf.idx()] -= aus_topf;
        let rest = betrag - aus_topf;
        if rest > 0 {
            // Nur der Feldherr kommt hierher, wenn bei einer Warnung die Reserve mitzahlt.
            let t = if topf == Topf::Militaer {
                Topf::Reserve
            } else {
                topf
            };
            sp.toepfe[t.idx()] -= rest;
        }
    }

    /// Bucht vorhandenes Guthaben sofort zwischen zwei Töpfen um (Stratege; Karl 26.10.2026). `menge` in Tausendsteln.
    pub(crate) fn topf_umbuchen(&mut self, sid: SpielerId, von: Topf, nach: Topf, menge: i64) -> Result<String, String> {
        if von == nach { return Err("von und nach müssen verschiedene Töpfe sein".into()); }
        if menge <= 0 { return Err("menge muss über 0 liegen".into()); }
        let sp = &mut self.spieler[sid as usize];
        let hat = sp.toepfe[von.idx()].max(0);
        if menge > hat {
            return Err(format!("Topf {von} hat nur {} Werteinheiten (lage.toepfe.{von}.guthaben)", ganz(hat)));
        }
        sp.toepfe[von.idx()] -= menge;
        sp.toepfe[nach.idx()] += menge;
        Ok(format!("{} Werteinheiten von {von} nach {nach} umgebucht; {nach} hat jetzt {}", ganz(menge), ganz(sp.toepfe[nach.idx()])))
    }

    /// Prüft Bestand und Topf. Der Fehlertext nennt genau, was fehlt.
    pub fn zahlbar(
        &self,
        pid: usize,
        kosten: &[i64; GUETER],
        topf: Option<Topf>,
    ) -> Result<(), String> {
        let p = &self.planeten[pid];
        let mut fehlt = Vec::new();
        for g in Gut::ALLE {
            let d = kosten[g.idx()] - p.bestand[g.idx()];
            if d > 0 {
                fehlt.push(format!("{} {}", (d + M - 1) / M, g));
            }
        }
        if !fehlt.is_empty() {
            return Err(format!("auf {} fehlen {}", p.koord, fehlt.join(", ")));
        }
        if let Some(t) = topf {
            let wert = self.regeln.wert(kosten);
            let hat = self.topf_guthaben(p.besitzer, t);
            if wert > hat {
                return Err(format!(
                    "Topf {} reicht nicht: {} Werteinheiten nötig, {} vorhanden (lage.toepfe); der Stratege bucht mit topf_umbuchen sofort um oder ändert die Anteile mit doktrin – Anfrage an ihn über die Pinwand",
                    t,
                    // aufgerundet: eine Umbuchung über genau den genannten Fehlbetrag muss reichen (Rat-Probe 27.10.2026:
                    // „63 nötig“ waren 63,4, die Umbuchung von 63 scheiterte erneut)
                    (wert + M - 1) / M,
                    ganz(hat.max(0))
                ));
            }
        }
        Ok(())
    }

    pub fn zahlen(&mut self, pid: usize, kosten: &[i64; GUETER], topf: Option<Topf>) {
        let besitzer = self.planeten[pid].besitzer;
        for g in 0..GUETER {
            self.planeten[pid].bestand[g] -= kosten[g];
        }
        self.spieler[besitzer as usize].xeno_verbaut += kosten[Gut::Xenokristall.idx()];
        if let Some(t) = topf {
            let wert = self.regeln.wert(kosten);
            self.topf_abbuchen(besitzer, t, wert);
        }
    }

    /// Topf, aus dem eine Rolle für einen Zweck zahlt. Bots zahlen ohne Topf.
    pub fn topf_fuer(rolle: Rolle, zweck: Topf) -> Option<Topf> {
        match rolle {
            Rolle::Alle => None,
            Rolle::Feldherr => Some(Topf::Militaer),
            Rolle::Diplomat => Some(Topf::Reserve),
            _ => Some(zweck),
        }
    }

    // ---------------------------------------------------------------- Gebäude

    pub fn felder_gesamt(&self, pid: usize) -> u16 {
        let p = &self.planeten[pid];
        let terra =
            self.spieler[p.besitzer as usize].forschung[Forschung::Terraforming.idx()] as u16;
        p.felder
            + terra * self.regeln.wirtschaft.terraforming_felder
            + p.gebaeude[Gebaeude::Orbitalring.idx()] as u16 * self.regeln.zusatz.orbitalring_felder
            // Terraformer-Schiffe (0.4.7): geformte Felder über der Grundgröße (neue Welten; dort Forschungsterm 0).
            + self.terraform_felder(p.koord)
    }

    pub fn felder_belegt(&self, pid: usize) -> u16 {
        let p = &self.planeten[pid];
        p.gebaeude.iter().map(|s| *s as u16).sum::<u16>() + p.bauschleife.len() as u16
    }

    /// Bauzeit in Spielsekunden.
    pub fn zeitberechnung(&self, pid: usize, kosten: &[i64; GUETER], fertigung: Option<(usize,Volk)>) -> crate::planer::zeiten::ZeitBerechnung {
        let mut plant=self.planeten[pid].clone();if self.ausbau.industrie_version>0{if let Some((n,_))=fertigung{plant.gebaeude[if n==0{Gebaeude::Werft}else{Gebaeude::Orbitalwerft}.idx()]=1;}}
        let mut result=crate::planer::zeiten::berechnen(&self.regeln, &plant, kosten,
            &std::array::from_fn(|i| self.integritaet(pid, Gebaeude::ALLE[i])), fertigung);
        if fertigung.is_some()&&self.ausbau.industrie_version>0{result.sekunden=result.sekunden.faktor("produktionsleistung",100.0/self.werk_tempo(pid,result.werk) as f64).mindestens("minimum",1);}
        result
    }

    pub fn bauzeit(&self, pid: usize, kosten: &[i64; GUETER]) -> i64 {
        self.zeitberechnung(pid, kosten, None).sekunden.wert
    }

    pub fn bauen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        k: Koord,
        g: Gebaeude,
    ) -> Result<String, String> {
        if !self.regeln.gebaeude.contains_key(&g) { return Err("Gebäude gehört nicht zu den Regeln dieser Partie".into()); }
        let pid = self.eigener_planet(sid, k)?;
        if self.integritaet(pid, g) < 1000 {
            return Err("Beschädigtes Gebäude vor weiterem Ausbau reparieren".into());
        }
        let r = self.regeln.clone();
        let gr = r.geb(g);
        if self.spieler[sid as usize].stufe < gr.ab_stufe {
            return Err(format!(
                "{g} gibt es erst ab Zivilisationsstufe {}",
                gr.ab_stufe
            ));
        }
        if self.ausbau.version==0 && g == Gebaeude::Xenoextraktor && !self.system(k).map(|s| s.nebel).unwrap_or(false) {
            return Err("Xenoextraktor geht nur auf Planeten in Nebelsystemen".into());
        }
        let p = &self.planeten[pid];
        for (b, st) in &gr.braucht {
            if p.gebaeude[b.idx()] < *st {
                return Err(format!("{g} braucht {b} Stufe {st} auf {k}"));
            }
        }
        if p.bauschleife.len() >= r.wirtschaft.warteschlange {
            return Err(format!(
                "Bauschleife auf {k} ist voll ({} Aufträge)",
                r.wirtschaft.warteschlange
            ));
        }
        let in_schleife = p.bauschleife.iter().filter(|a| a.gebaeude == g).count() as u8;
        let ziel = p.gebaeude[g.idx()] + in_schleife + 1;
        if matches!(
            g,
            Gebaeude::Orbitalring | Gebaeude::Forschungsarchiv | Gebaeude::Versorgungsnetz
        ) && ziel > 1
        {
            return Err(format!("Großprojekt {g} ist einmal pro Planet möglich"));
        }
        if ziel > r.welt.max_gebaeudestufe {
            return Err(format!("{g} hat die höchste Stufe erreicht"));
        }
        if self.felder_belegt(pid) >= self.felder_gesamt(pid) {
            return Err(format!(
                "auf {k} sind alle {} Felder belegt",
                self.felder_gesamt(pid)
            ));
        }
        let topf = Self::topf_fuer(rolle, Topf::Wirtschaft);
        let kosten = r.kosten_gebaeude(g, ziel);
        if self.planeten[pid].bauschleife.is_empty() && self.ausbau.industrie_version == 0 {
            // Der Auftrag würde sofort beginnen: gleich genau sagen, woran es scheitert.
            if self.planeten[pid].stabilitaet < milli(r.stabilitaet.unruhen_unter) {
                return Err(format!(
                    "Unruhen auf {k}: Bauaufträge ruhen, solange die Stabilität unter {} liegt",
                    r.stabilitaet.unruhen_unter
                ));
            }
            self.abrechnen(pid);
            self.zahlbar(pid, &kosten, topf)?;
        }
        self.planeten[pid].bauschleife.push(Bauauftrag {
            gebaeude: g,
            stufe: ziel,
            fertig: None,
            dauer: 0,
            topf,
        });
        self.planeten[pid].leer_gemeldet = false;
        self.bau_starten(pid);
        let dauer = self.bauzeit(pid, &kosten);
        Ok(format!(
            "{g} Stufe {ziel} auf {k} eingereiht, Bauzeit {} min",
            dauer / 60
        ))
    }

    /// Startet den vordersten Auftrag, sobald er bezahlt werden kann.
    pub fn bau_starten(&mut self, pid: usize) {
        if self.auftrag_pausiert("bau", self.planeten[pid].id) { return; }
        if self.kolonisationsregeln_v2()
            && self
                .kolonisation
                .reparaturen
                .keys()
                .any(|(planet, _)| *planet == pid as PlanetId)
        {
            return;
        }
        let Some(kopf) = self.planeten[pid].bauschleife.first().cloned() else {
            return;
        };
        if kopf.fertig.is_some() || self.bau_ruht_unruhen(pid) {
            return;
        }
        let kosten = self.regeln.kosten_gebaeude(kopf.gebaeude, kopf.stufe);
        self.abrechnen(pid);
        if self.zahlbar(pid, &kosten, kopf.topf).is_err() {
            return;
        }
        self.zahlen(pid, &kosten, kopf.topf);
        let dauer = self.bauzeit(pid, &kosten);
        let fertig = self.zeit + dauer;
        self.planeten[pid].bauschleife[0].dauer = dauer;
        self.planeten[pid].bauschleife[0].fertig = Some(fertig);
        let id = self.planeten[pid].id;
        self.plane(fertig, EreignisArt::BauFertig { planet: id });
    }

    pub fn bau_fertig(&mut self, pid: usize) {
        let jetzt = self.zeit;
        let Some(kopf) = self.planeten[pid].bauschleife.first().cloned() else {
            return;
        };
        if kopf.fertig != Some(jetzt) {
            // Unrest preserves the remaining work by moving the finish timestamp.
            // Only replace the old event when it is consumed, avoiding duplicate
            // queue entries on every simulated time step.
            if let Some(fertig) = kopf.fertig.filter(|t| *t > jetzt) {
                self.plane(
                    fertig,
                    EreignisArt::BauFertig {
                        planet: self.planeten[pid].id,
                    },
                );
            }
            return;
        }
        self.abrechnen(pid);
        let p = &mut self.planeten[pid];
        p.bauschleife.remove(0);
        p.gebaeude[kopf.gebaeude.idx()] = kopf.stufe;
        let (besitzer, k) = (p.besitzer, p.koord);
        if self.ausbau.industrie_version>0 {match kopf.gebaeude{
            Gebaeude::Werft=>self.parallel_sync(pid,0),Gebaeude::Orbitalwerft=>self.parallel_sync(pid,1),
            Gebaeude::Kaserne=>self.boden_verteilen(pid),Gebaeude::Raketensilo=>self.silo_parallel_sync(pid),_=>{}
        }}
        self.vorfall(
            besitzer,
            "bau",
            format!("{} Stufe {} auf {k} fertig", kopf.gebaeude, kopf.stufe),
        );
        self.raten_neu(pid);
        self.bau_starten(pid);
    }

    pub fn abreissen(&mut self, sid: SpielerId, k: Koord, g: Gebaeude) -> Result<String, String> {
        let pid = self.eigener_planet(sid, k)?;
        let p = &self.planeten[pid];
        if p.gebaeude[g.idx()] == 0 {
            return Err(format!("{g} steht nicht auf {k}"));
        }
        if self.ausbau.industrie_version>0{
            let occupied=match g{Gebaeude::Werft=>self.parallel_sicht(pid,0)["belegt"].as_i64().unwrap(),Gebaeude::Orbitalwerft=>self.parallel_sicht(pid,1)["belegt"].as_i64().unwrap(),Gebaeude::Raketensilo=>self.silo_parallel_sicht(pid)["belegt"].as_i64().unwrap(),Gebaeude::Kaserne=>self.ausbau.boden.auftraege.get(&p.id).map(|q|q.iter().map(|a|a.lose.iter().map(|b|b.anzahl*self.regeln.kw().boden.fertigungsplaetze(a.art)).sum::<i64>()).sum()).unwrap_or(0),_=>0};
            if occupied>p.gebaeude[g.idx()] as i64-1{return Err("Fertigungsplätze sind belegt; zuerst betroffene Aufträge fertigstellen oder abbrechen".into());}
        }
        if p.bauschleife.iter().any(|a| a.gebaeude == g) {
            return Err(format!("{g} ist in der Bauschleife, Abriss nicht möglich"));
        }
        // Ablauf-Version 1: Voraussetzungen gelten bis zum Baubeginn – kein Abriss unter die Stufe, die ein Auftrag
        // in der Bauschleife braucht (sonst entstünde z. B. eine Nanofabrik ohne Bauhof).
        if self.ablauf_v1() {
            let rest = p.gebaeude[g.idx()] - 1;
            if let Some(a) = p.bauschleife.iter().find(|a| self.regeln.geb(a.gebaeude).braucht.iter().any(|(b, st)| *b == g && rest < *st)) {
                return Err(format!("{g} wird von {} in der Bauschleife gebraucht, Abriss nicht möglich", a.gebaeude));
            }
        }
        if g == Gebaeude::Raketensilo
            && self.raketen_belegt(pid)
                > (p.gebaeude[g.idx()] as i64 - 1) * self.regeln.zusatz.silo_plaetze_je_stufe
        {
            return Err("Silo enthält fertige oder im Bau befindliche Raketen; Abriss würde Kapazität unterschreiten".into());
        }
        if g == Gebaeude::Orbitalring
            && self.felder_belegt(pid).saturating_sub(1)
                > self
                    .felder_gesamt(pid)
                    .saturating_sub(self.regeln.zusatz.orbitalring_felder)
        {
            return Err(
                "Orbitalring kann nicht abgerissen werden: zusätzliche Felder sind belegt".into(),
            );
        }
        self.abrechnen(pid);
        self.planeten[pid].gebaeude[g.idx()] -= 1;
        if self.kolonisationsregeln_v2() && self.planeten[pid].gebaeude[g.idx()] == 0 {
            self.kolonisation.integritaet.remove(&(pid as PlanetId, g));
            if self
                .kolonisation
                .reparaturen
                .remove(&(pid as PlanetId, g))
                .is_some()
            {
                self.vorfall(sid,"reparatur",format!("Reparatur von {g} auf {k} wegen vollständigem Abriss abgebrochen; ohne Erstattung"));
                self.bau_starten(pid);
            }
        }
        self.raten_neu(pid);
        Ok(format!(
            "{g} auf {k} um eine Stufe abgerissen, ohne Erstattung"
        ))
    }

    pub fn schleife_leeren(&mut self, sid: SpielerId, k: Koord) -> Result<String, String> {
        let pid = self.eigener_planet(sid, k)?;
        let p = &mut self.planeten[pid];
        let vorher = p.bauschleife.len();
        p.bauschleife.retain(|a| a.fertig.is_some());
        Ok(format!(
            "{} wartende Aufträge auf {k} entfernt",
            vorher - p.bauschleife.len()
        ))
    }

    // ---------------------------------------------------------------- Fertigung

    pub fn fertigungszeit(&self, pid: usize, kosten: &[i64; GUETER], schleife: usize) -> i64 {
        self.zeitberechnung(pid, kosten, Some((schleife, self.spieler[self.planeten[pid].besitzer as usize].volk))).sekunden.wert
    }

    pub fn fertigen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        k: Koord,
        produkt: Produkt,
        anzahl: i64,
    ) -> Result<String, String> {
        let pid = self.eigener_planet(sid, k)?;
        let r = self.regeln.clone();
        if !(1..=10_000).contains(&anzahl) {
            return Err("Anzahl muss zwischen 1 und 10000 liegen".into());
        }
        let stufe = self.spieler[sid as usize].stufe;
        let volk = self.spieler[sid as usize].volk;
        let p = &self.planeten[pid];
        let (stueck, schleife, besatzung, name, zweck) = match produkt {
            Produkt::Einheit(e) => {
                if !r.einheiten.contains_key(&e){return Err("Einheit ist in dieser Partie nicht verfügbar".into());}
                if e==Einheit::Weltentruemmer{return Err(match self.monde_regeln(){Some(m)=>format!("Weltenkrümmer entstehen nur im Werftdock (Stufe {}) eines Mondes: mond_fertigen",m.weltentruemmer_werftdock),None=>"Weltenkrümmer entstehen nur im Werftdock eines Mondes: mond_fertigen".into()});}
                let fehlt=self.einheit_forschung_fehlend(sid,e);
                if !fehlt.is_empty(){return Err(format!("{e} benötigt {}",fehlt.join(", ")));}
                if !r.flottendoktrin.is_empty() && matches!(e,Einheit::Planetenschild|Einheit::PlanetenschildII|Einheit::PlanetenschildIII){
                    let shields=[Einheit::Planetenschild,Einheit::PlanetenschildII,Einheit::PlanetenschildIII];
                    if shields.iter().any(|s|s.idx()>=e.idx()&&p.einheiten[s.idx()]>0){return Err("Gleiche oder höhere Planetenschildstufe bereits aktiv".into());}
                    if p.fertigung[0].iter().any(|a|matches!(a.produkt,Produkt::Einheit(t)if shields.contains(&t))){return Err("Ein Planetenschild ist bereits in Fertigung".into());}
                }
                let er = r.einh(e);
                if stufe < er.ab_stufe {
                    return Err(format!(
                        "{e} gibt es erst ab Zivilisationsstufe {}",
                        er.ab_stufe
                    ));
                }
                if e.ist_schiff() && p.gebaeude[Gebaeude::Werft.idx()] < er.werft.max(1) {
                    return Err(format!(
                        "{e} braucht Werft Stufe {} auf {k}",
                        er.werft.max(1)
                    ));
                }
                for (b, st) in &er.braucht {
                    if p.gebaeude[b.idx()] < *st {
                        return Err(format!("{e} braucht {b} Stufe {st} auf {k}"));
                    }
                }
                if er.max_anzahl > 0 {
                    let offen: i64 = p.fertigung[0]
                        .iter()
                        .filter(|f| f.produkt == produkt)
                        .map(|f| f.rest)
                        .sum();
                    if p.einheiten[e.idx()] + offen + anzahl > er.max_anzahl {
                        return Err(format!(
                            "von {e} sind höchstens {} je Planet erlaubt",
                            er.max_anzahl
                        ));
                    }
                }
                let zivil = zivile_einheit(e);
                if rolle == Rolle::Verwalter && !zivil {
                    return Err(format!("{e} bestellt der Feldherr, nicht der Verwalter"));
                }
                let zweck = if zivil && rolle != Rolle::Feldherr {
                    Topf::Wirtschaft
                } else {
                    Topf::Militaer
                };
                (
                    r.kosten_einheit(e, volk),
                    0usize,
                    er.besatzung * M,
                    e.name(),
                    zweck,
                )
            }
            Produkt::Bauteil(g) => {
                let Some(kosten) = r.kosten_bauteil(g) else {
                    return Err(format!("{g} ist kein Bauteil der Orbitalwerft"));
                };
                let frueh=self.ausbau.version>0 && g==Gut::Habitatmodul && p.gebaeude[Gebaeude::Orbitalwerft.idx()]==0;
                if frueh {
                    if stufe<3 || p.gebaeude[Gebaeude::Werft.idx()]<3 {return Err("Habitatmodule benötigen Stufe III und Werft 3".into());}
                    if p.einheiten[Einheit::Recycler.idx()]<anzahl {return Err("Ein vorhandener Recycler je Habitatmodul wird umgebaut".into());}
                } else if p.gebaeude[Gebaeude::Orbitalwerft.idx()] < 1 {
                    return Err(format!("Antriebskerne brauchen eine Orbitalwerft auf {k}"));
                }
                (kosten, usize::from(!frueh), 0, g.name(), Topf::Wirtschaft)
            }
        };
        if p.fertigung[schleife].len() >= r.wirtschaft.warteschlange {
            return Err(format!("Fertigungsschleife auf {k} ist voll"));
        }
        let work=if schleife==1 {Gebaeude::Orbitalwerft}else{Gebaeude::Werft};
        if self.ausbau.industrie_version>0 && crate::parallel::platzbedarf(produkt)>self.fertigungsplaetze(pid,schleife){return Err(format!("Dieses Produkt belegt {} Fertigungsplätze; {work} ausbauen",crate::parallel::platzbedarf(produkt)));}
        let paid_stueck=self.produktionskosten(pid,work,stueck);
        let mut kosten = [0i64; GUETER];
        for g in 0..GUETER {
            kosten[g] = paid_stueck[g].saturating_mul(anzahl);
        }
        let crew = besatzung * anzahl;
        if self.ausbau.version>0 && self.ausbau.personen.get(&(pid as PlanetId)).is_none_or(|p|p.auswahl(crew).gesamt()!=crew){return Err("Nicht genügend ganze Personen für die Besatzung".into());}
        if crew > 0 && p.bevoelkerung - crew < 500 * M {
            return Err(format!(
                "Besatzung von {} fehlt: auf {k} müssen mindestens 500 Einwohner bleiben",
                ganz(crew)
            ));
        }
        let topf = Self::topf_fuer(rolle, zweck);
        self.abrechnen(pid);
        self.zahlbar(pid, &kosten, topf)?;
        self.zahlen(pid, &kosten, topf);
        if produkt==Produkt::Bauteil(Gut::Habitatmodul) && schleife==0 {self.planeten[pid].einheiten[Einheit::Recycler.idx()]-=anzahl;}
        self.crew_reservieren(pid,produkt,crew);
        if self.ausbau.industrie_version>0 {self.ausbau.fertigung_zahlung.entry(format!("{pid}:{schleife}")).or_default().push((topf,paid_stueck));}
        if self.ausbau.industrie_version>0{self.parallel_sync(pid,schleife);}
        let dauer = self.fertigungszeit(pid, &stueck, schleife);
        let p = &mut self.planeten[pid];
        p.bevoelkerung -= crew;
        let leer = p.fertigung[schleife].is_empty();
        p.fertigung[schleife].push(Fertigung {
            produkt,
            rest: anzahl,
            dauer,
        });
        if leer && self.ausbau.industrie_version==0 {
            let fertig = self.zeit + dauer;
            self.planeten[pid].fertigung_naechste[schleife] = fertig;
            let id = self.planeten[pid].id;
            self.plane(
                fertig,
                EreignisArt::FertigungFertig {
                    planet: id,
                    schleife: schleife as u8,
                },
            );
        }
        if self.ausbau.industrie_version>0{self.parallel_sync(pid,schleife);}
        self.raten_neu(pid);
        Ok(format!(
            "{anzahl} x {name} auf {k} bestellt, {dauer} s je Stück"
        ))
    }

    pub fn fertigung_fertig(&mut self, pid: usize, schleife: usize) {
        if self.ausbau.industrie_version>0{if !self.ausbau.parallel.contains_key(&crate::parallel::key(pid,schleife)){self.parallel_sync(pid,schleife);}self.parallel_fertig(pid,schleife);return;}
        let jetzt = self.zeit;
        let naechste = self.planeten[pid].fertigung_naechste[schleife];
        if naechste > jetzt && !self.planeten[pid].fertigung[schleife].is_empty() {
            // Pause hat die Fertigstellung verschoben: Ereignis erst beim Verbrauch ersetzen.
            let id = self.planeten[pid].id;
            self.plane(naechste, EreignisArt::FertigungFertig { planet: id, schleife: schleife as u8 });
            return;
        }
        if naechste != jetzt
            || self.planeten[pid].fertigung[schleife].is_empty()
        {
            return;
        }
        self.abrechnen(pid);
        let produkt = self.planeten[pid].fertigung[schleife][0].produkt;
        self.crew_fertig(pid,produkt);
        let p = &mut self.planeten[pid];
        match produkt {
            Produkt::Einheit(e) => {
                if !self.regeln.flottendoktrin.is_empty()&&matches!(e,Einheit::Planetenschild|Einheit::PlanetenschildII|Einheit::PlanetenschildIII){
                    for t in [Einheit::Planetenschild,Einheit::PlanetenschildII,Einheit::PlanetenschildIII]{p.einheiten[t.idx()]=0;}
                }
                p.einheiten[e.idx()]+=1;
            },
            Produkt::Bauteil(g) => p.bestand[g.idx()] += M,
        }
        p.fertigung[schleife][0].rest -= 1;
        if p.fertigung[schleife][0].rest <= 0 {
            p.fertigung[schleife].remove(0);
            if let Some(paid)=self.ausbau.fertigung_zahlung.get_mut(&format!("{pid}:{schleife}")){if !paid.is_empty(){paid.remove(0);}}
            let (besitzer, k) = (p.besitzer, p.koord);
            let name = match produkt {
                Produkt::Einheit(e) => e.name(),
                Produkt::Bauteil(g) => g.name(),
            };
            self.vorfall(
                besitzer,
                "fertigung",
                format!("Auftrag {name} auf {k} abgeschlossen"),
            );
        }
        let p = &mut self.planeten[pid];
        if let Some(naechster) = p.fertigung[schleife].first() {
            let fertig = jetzt + naechster.dauer;
            p.fertigung_naechste[schleife] = fertig;
            let id = p.id;
            self.plane(
                fertig,
                EreignisArt::FertigungFertig {
                    planet: id,
                    schleife: schleife as u8,
                },
            );
        } else {
            p.fertigung_naechste[schleife] = -1;
        }
        self.raten_neu(pid);
    }

    // ---------------------------------------------------------------- Forschung

    pub fn forschen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        f: Forschung,
        planet: Option<Koord>,
    ) -> Result<String, String> {
        if !self.regeln.forschung.contains_key(&f) { return Err("Forschung gehört nicht zu den Regeln dieser Partie".into()); }
        if matches!(f,Forschung::Ueberwachungstechnik|Forschung::Abschirmtechnik) && self.spieler[sid as usize].forschung[Forschung::Spionagetechnik.idx()]==0 {
            return Err("Überwachung und Abschirmtechnik benötigen Spionagetechnik Stufe 1".into());
        }
        if f == Forschung::Ueberwachungstechnik && !self.spieler[sid as usize].planeten.iter()
            .any(|p|self.planeten[*p as usize].gebaeude[Gebaeude::Geheimdienst.idx()] > 0) {
            return Err("Überwachungstechnik benötigt einen eigenen Geheimdienst".into());
        }
        if f == Forschung::Plasmatechnik {
            let fehlt = self.plasmatechnik_fehlt(sid);
            if !fehlt.is_empty() { return Err(format!("Plasmatechnik benötigt {}", fehlt.join(", "))); }
        }
        let r = self.regeln.clone();
        let fr = r.forsch(f);
        let sp = &self.spieler[sid as usize];
        let geplant = u16::from(sp.forschung[f.idx()])
            + u16::from(sp.forschung_aktiv.as_ref().is_some_and(|a|a.forschung==f))
            + sp.forschung_schlange.iter().filter(|(t,_,_)|*t==f).count() as u16;
        if geplant >= crate::regeln::MAX_FORSCHUNG as u16 { return Err("Höchste Forschungsstufe bereits erreicht oder eingeplant".into()); }
        if sp.stufe < fr.ab_stufe {
            return Err(format!(
                "{f} gibt es erst ab Zivilisationsstufe {}",
                fr.ab_stufe
            ));
        }
        let pid = match planet {
            Some(k) => self.eigener_planet(sid, k)?,
            None => sp.heimat as usize,
        };
        let k = self.planeten[pid].koord;
        if self.planeten[pid].gebaeude[Gebaeude::Labor.idx()] < fr.labor {
            return Err(format!("{f} braucht Labor Stufe {} auf {k}", fr.labor));
        }
        let topf = Self::topf_fuer(rolle, Topf::Forschung);
        if self.ausbau.industrie_version > 0 {
            if sp.forschung_schlange.len() >= r.wirtschaft.forschung_warteschlange {
                return Err("Forschungsschlange ist voll".into());
            }
            self.spieler[sid as usize].forschung_schlange.push((f, self.planeten[pid].id, topf));
            self.forschung_schlange_starten(sid);
            return Ok(format!("Forschung {f} eingereiht; startet automatisch, sobald der vorherige Auftrag abgeschlossen ist und die Mittel reichen"));
        }
        if sp.forschung_aktiv.is_none() {
            self.forschung_starten(sid, f, pid, topf)?;
            let stufe = self.spieler[sid as usize].forschung[f.idx()] + 1;
            Ok(format!("Forschung {f} Stufe {stufe} begonnen"))
        } else if sp.forschung_schlange.len() < r.wirtschaft.forschung_warteschlange {
            self.spieler[sid as usize]
                .forschung_schlange
                .push((f, self.planeten[pid].id, topf));
            Ok(format!("Forschung {f} eingereiht"))
        } else {
            Err("Forschungsschlange ist voll".into())
        }
    }

    fn forschung_starten(
        &mut self,
        sid: SpielerId,
        f: Forschung,
        pid: usize,
        topf: Option<Topf>,
    ) -> Result<(), String> {
        if self.kolonisationsregeln_v2() {
            let p = self.planeten.get(pid).ok_or("Forschungsplanet fehlt")?;
            let fr = self.regeln.forsch(f);
            if p.besitzer != sid {
                return Err("Forschungsplanet gehört inzwischen einem anderen Spieler".into());
            }
            if self.spieler[sid as usize].stufe < fr.ab_stufe
                || p.gebaeude[Gebaeude::Labor.idx()] < fr.labor
            {
                return Err(format!(
                    "Forschung {f} braucht weiterhin Zivilisationsstufe {} und Labor {} auf {}",
                    fr.ab_stufe, fr.labor, p.koord
                ));
            }
        }
        if self.spieler[sid as usize].forschung[f.idx()] >= crate::regeln::MAX_FORSCHUNG { return Err("Höchste Forschungsstufe erreicht".into()); }
        let stufe = self.spieler[sid as usize].forschung[f.idx()] + 1;
        let kosten = self.regeln.kosten_forschung(f, stufe);
        self.abrechnen(pid);
        self.zahlbar(pid, &kosten, topf)?;
        self.zahlen(pid, &kosten, topf);
        let fp_rest = self.regeln.fp_forschung(f, stufe);
        self.ausbau.forschung_zahlung.insert(sid,(pid as PlanetId,topf,kosten));
        self.spieler[sid as usize].forschung_aktiv = Some(Forschungsauftrag {
            forschung: f,
            stufe,
            fp_rest,
            begonnen: Some(self.zeit),
        });
        Ok(())
    }

    pub fn fp_rate(&self, sid: SpielerId) -> i64 {
        self.spieler[sid as usize]
            .planeten
            .iter()
            .map(|pid| self.planeten[*pid as usize].fp_rate)
            .sum()
    }

    fn forschung_tick(&mut self, sid: SpielerId) {
        if self.auftrag_pausiert("forschung", sid as u32) { return; }
        let fp = self.fp_rate(sid);
        let sp = &mut self.spieler[sid as usize];
        let fertig = match &mut sp.forschung_aktiv {
            Some(a) => {
                a.fp_rest -= fp;
                if a.fp_rest <= 0 {
                    Some((a.forschung, a.stufe))
                } else {
                    None
                }
            }
            None => None,
        };
        if let Some((f, stufe)) = fertig {
            sp.forschung[f.idx()] = stufe;
            sp.forschung_aktiv = None;
            self.ausbau.forschung_zahlung.remove(&sid);
            self.vorfall(
                sid,
                "forschung",
                format!("Forschung {f} Stufe {stufe} abgeschlossen"),
            );
            for pid in self.spieler[sid as usize].planeten.clone() {
                self.raten_neu(pid as usize);
            }
        }
        self.forschung_schlange_starten(sid);
    }

    pub(crate) fn forschung_schlange_starten(&mut self, sid: SpielerId) {
        if self.auftrag_pausiert("forschung", sid as u32) { return; }
        // Keep unpaid/unavailable projects in order and retry after supply changes.
        // A queue is a plan, not a collection of disposable requests.
        if self.spieler[sid as usize].forschung_aktiv.is_none()
            && !self.spieler[sid as usize].forschung_schlange.is_empty() {
            let (f, planet, topf) = self.spieler[sid as usize].forschung_schlange[0];
            let mut pid = planet as usize;
            if self.planeten.get(pid).is_none_or(|p|p.besitzer != sid) {
                pid = self.spieler[sid as usize].heimat as usize;
            }
            if self.forschung_starten(sid, f, pid, topf).is_ok() {
                self.spieler[sid as usize].forschung_schlange.remove(0);
            }
        }
    }

    // ---------------------------------------------------------------- Zivilisationsstufen

    pub fn einwohner(&self, sid: SpielerId) -> i64 {
        self.spieler[sid as usize]
            .planeten
            .iter()
            .map(|pid| self.planeten[*pid as usize].bevoelkerung)
            .sum()
    }

    /// Bedingungen für den nächsten Aufstieg mit Stand. Leer auf der höchsten Stufe.
    pub fn stufen_bedingungen(&self, sid: SpielerId) -> Vec<(String, bool)> {
        let sp = &self.spieler[sid as usize];
        let Some(regel) = self.regeln.stufen.get(sp.stufe as usize - 1) else {
            return Vec::new();
        };
        let heimat = &self.planeten[sp.heimat as usize];
        let mut v = Vec::new();
        let ew = self.einwohner(sid);
        v.push((
            format!("{} Einwohner (jetzt {})", regel.einwohner, ganz(ew)),
            ew >= regel.einwohner * M,
        ));
        for (g, st) in &regel.gebaeude {
            let hat = sp
                .planeten
                .iter()
                .map(|p| self.planeten[*p as usize].gebaeude[g.idx()])
                .max()
                .unwrap_or(0);
            v.push((format!("{g} Stufe {st} (jetzt {hat})"), hat >= *st));
        }
        for (f, st) in &regel.forschung {
            let hat = sp.forschung[f.idx()];
            v.push((format!("{f} Stufe {st} (jetzt {hat})"), hat >= *st));
        }
        if regel.versorgung_plus {
            let ohne = self.regeln.volk(sp.volk).ohne_nahrung;
            let ok = sp.planeten.iter().all(|p| {
                let p = &self.planeten[*p as usize];
                (ohne || p.rate[Gut::Nahrung.idx()] >= 0)
                    && p.energie_erzeugung >= p.energie_verbrauch
            });
            v.push(("Nahrung und Energie im Plus".to_string(), ok));
        }
        if regel.stabilitaet > 0.0 {
            v.push((
                format!(
                    "Stabilität der Heimatwelt ab {} (jetzt {})",
                    regel.stabilitaet,
                    heimat.stabilitaet / M
                ),
                heimat.stabilitaet >= milli(regel.stabilitaet),
            ));
        }
        if regel.konsum_deckung > 0.0 {
            v.push((
                format!(
                    "Konsumgüter zu {} % gedeckt (jetzt {} %)",
                    (regel.konsum_deckung * 100.0).round(),
                    heimat.konsum_deckung / 10
                ),
                heimat.konsum_deckung >= milli(regel.konsum_deckung),
            ));
        }
        if regel.kolonien > 0 {
            let hat = self.kolonien(sid);
            v.push((
                format!("{} Kolonien (jetzt {hat})", regel.kolonien),
                hat >= regel.kolonien as usize,
            ));
        }
        v
    }

    fn stufen_tick(&mut self, sid: SpielerId) {
        let bed = self.stufen_bedingungen(sid);
        let haltezeit = self.regeln.stufen_haltezeit_stunden * STUNDE;
        if bed.is_empty() {
            return;
        }
        let erfuellt = bed.iter().all(|(_, ok)| *ok);
        let vorher = self.spieler[sid as usize].stufen_zaehler;
        let jetzt = if erfuellt { vorher + STUNDE } else { 0 };
        self.spieler[sid as usize].stufen_zaehler = jetzt;
        // Ohne Haltezeit meldet der erste Tick, in dem alles erfüllt ist; mit Haltezeit (alte Welten) wie bisher.
        let schwelle = haltezeit.max(STUNDE);
        if vorher < schwelle && jetzt >= schwelle {
            let text = if haltezeit == 0 {
                "Alle Bedingungen für den Stufenaufstieg sind erfüllt: Aufstieg jetzt möglich".to_string()
            } else {
                format!(
                    "Alle Bedingungen für den Stufenaufstieg sind seit {} Stunden erfüllt",
                    self.regeln.stufen_haltezeit_stunden
                )
            };
            self.vorfall(sid, "stufe", text);
            self.wecke(sid, Rolle::Stratege, "Stufenaufstieg möglich", false);
            self.wecke(sid, Rolle::Verwalter, "Stufenaufstieg möglich", false);
        }
    }

    pub fn stufenaufstieg(&mut self, sid: SpielerId) -> Result<String, String> {
        let r = self.regeln.clone();
        let sp = &self.spieler[sid as usize];
        let Some(regel) = r.stufen.get(sp.stufe as usize - 1) else {
            return Err("Die höchste Zivilisationsstufe ist erreicht".into());
        };
        let offen: Vec<String> = self
            .stufen_bedingungen(sid)
            .into_iter()
            .filter(|(_, ok)| !ok)
            .map(|(t, _)| t)
            .collect();
        if !offen.is_empty() {
            return Err(format!("Bedingungen offen: {}", offen.join("; ")));
        }
        let haltezeit = r.stufen_haltezeit_stunden * STUNDE;
        if sp.stufen_zaehler < haltezeit {
            return Err(format!(
                "Bedingungen erst seit {} von {} Stunden ohne Unterbrechung erfüllt",
                sp.stufen_zaehler / STUNDE,
                r.stufen_haltezeit_stunden
            ));
        }
        let pid = sp.heimat as usize;
        let kosten = crate::regeln::preis_array(&regel.kosten);
        self.abrechnen(pid);
        self.zahlbar(pid, &kosten, None)?;
        self.zahlen(pid, &kosten, None);
        let sp = &mut self.spieler[sid as usize];
        sp.stufe += 1;
        sp.stufen_zaehler = 0;
        if sp.stufe >= r.diplomatie.anfaengerschutz_bis_stufe {
            sp.schutz_bis = sp.schutz_bis.min(self.zeit);
        }
        let stufe = sp.stufe;
        self.vorfall(
            sid,
            "stufe",
            format!("Aufstieg zur Zivilisationsstufe {}", regel.name),
        );
        self.wecke(sid, Rolle::Stratege, "Stufe erreicht", false);
        self.punkte_neu();
        Ok(format!(
            "Zivilisationsstufe {} erreicht (Stufe {stufe}). Neu: {}",
            regel.name, self.regeln.freischaltungen(stufe)
        ))
    }

    // ---------------------------------------------------------------- Tageswechsel

    /// Wert aller Schiffe eines Spielers, auf Planeten und in Flotten.
    pub fn flottenwert(&self, sid: SpielerId) -> i64 {
        let sp = &self.spieler[sid as usize];
        let mut zahl = [0i64; SCHIFFE];
        for pid in &sp.planeten {
            for e in 0..SCHIFFE {
                zahl[e] += self.planeten[*pid as usize].einheiten[e];
            }
        }
        for f in self.flotten.values().filter(|f| f.besitzer == sid) {
            for e in 0..SCHIFFE {
                zahl[e] += f.schiffe[e];
            }
        }
        let mut wert = 0i64;
        for e in 0..SCHIFFE {
            if zahl[e] > 0 {
                wert += zahl[e]
                    * self
                        .regeln
                        .wert(&self.regeln.kosten_einheit(Einheit::ALLE[e], sp.volk));
            }
        }
        wert
    }

    pub fn tageswechsel(&mut self) {
        let r = self.regeln.clone();
        let w = &r.wirtschaft;
        let jetzt = self.zeit;
        let grenze = jetzt - r.agenten.chronik_tage * TAG;
        for sid in 0..self.spieler.len() as SpielerId {
            // The chronicle limit applies to every seat. Reserved start places and defeated
            // empires still receive incidents (probes, attacks); skipping them let their
            // chronicle grow for the whole epoch (megabytes in every saved world).
            let sp = &mut self.spieler[sid as usize];
            sp.vorfaelle.retain(|v| v.zeit >= grenze);
            sp.meldungen.retain(|m| m.zeit >= grenze);
            if !self.spieler_aktiv(sid) { continue; }
            // Unterhalt für Schiffe und Verwaltung der Kolonien.
            // Ablauf-Version 1: auch Schiffe auf Monden kosten Unterhalt (Punkte zählen sie über mond_punkte).
            let mondschiffe = if self.ablauf_v1() && self.monde.aktiv {
                let volk = self.spieler[sid as usize].volk;
                self.monde.monde.values().filter(|m| m.besitzer == sid)
                    .map(|m| Einheit::ALLE.iter().filter(|e| e.ist_schiff()).map(|e| m.einheiten[e.idx()] * self.regeln.wert(&self.regeln.kosten_einheit(*e, volk))).sum::<i64>())
                    .sum::<i64>()
            } else { 0 };
            let kosten = mal(self.flottenwert(sid) + mondschiffe, w.unterhalt_schiffe_je_tag)
                + self.kolonien(sid) as i64 * w.verwaltung_je_kolonie_tag * M;
            let sp = &mut self.spieler[sid as usize];
            sp.nachrichten_heute = 0;
            if sp.credits >= kosten {
                sp.credits -= kosten;
                sp.schuld_seit = None;
            } else {
                sp.credits = 0;
                let seit = *sp.schuld_seit.get_or_insert(jetzt);
                if jetzt - seit >= w.desertion_nach_tagen * TAG {
                    self.desertion(sid);
                } else {
                    self.vorfall(
                        sid,
                        "unterhalt",
                        "Unterhalt nicht bezahlt, Credits fehlen".into(),
                    );
                }
            }
        }
        self.tribute_zahlen();
        self.punkte_neu();
        let tag = jetzt / TAG;
        for sid in 0..self.spieler.len() as SpielerId {
            if !self.spieler_aktiv(sid) { continue; }
            let sp = &self.spieler[sid as usize];
            let produktion: i64 = sp
                .planeten
                .iter()
                .map(|pid| {
                    let p = &self.planeten[*pid as usize];
                    let mut plus = [0i64; GUETER];
                    for g in 0..GUETER {
                        plus[g] = p.rate[g].max(0);
                    }
                    r.wert(&plus) * 24
                })
                .sum();
            let zeile = Tageswerte {
                tag,
                spieler: sid,
                stufe: sp.stufe,
                punkte: sp.punkte,
                bevoelkerung: self.einwohner(sid),
                stabilitaet: self.planeten[sp.heimat as usize].stabilitaet,
                kolonien: self.kolonien(sid) as u8,
                flottenwert: self.flottenwert(sid),
                produktion,
                credits: sp.credits,
            };
            self.tageswerte.push(zeile);
        }
    }

    fn desertion(&mut self, sid: SpielerId) {
        let f = self.regeln.wirtschaft.desertion_anteil;
        let weg = |n: i64| if n > 0 { mal(n, f).max(1) } else { 0 };
        let mut summe = 0i64;
        let mut je_typ = [0i64; SCHIFFE];
        for pid in self.spieler[sid as usize].planeten.clone() {
            for e in 0..SCHIFFE {
                let d = weg(self.planeten[pid as usize].einheiten[e]);
                self.planeten[pid as usize].einheiten[e] -= d;
                summe += d;
                je_typ[e] += d;
            }
        }
        let mut gekappt = [0i64; GUETER];
        let mut fids = Vec::new();
        for fl in self.flotten.values_mut().filter(|f| f.besitzer == sid) {
            for e in 0..SCHIFFE {
                // Das letzte Schiff einer Flotte bleibt, damit sie heimkehren kann.
                let d = weg(fl.schiffe[e]).min((fl.schiffe.iter().sum::<i64>() - 1).max(0));
                fl.schiffe[e] -= d;
                summe += d;
                je_typ[e] += d;
            }
            fids.push(fl.id);
        }
        // Weniger Schiffe tragen weniger: Fracht über dem neuen Laderaum geht mit den Deserteuren verloren.
        for fid in fids {
            let f = self.flotten[&fid].clone();
            let kap = self.flugplan(f.besitzer, f.start as usize, f.ziel, &f.schiffe, 1000).map_or(i64::MAX, |p| p.kapazitaet);
            let mut frei = kap;
            let fl = self.flotten.get_mut(&fid).unwrap();
            for g in 0..GUETER {
                let behalten = fl.ladung[g].min(frei.max(0));
                gekappt[g] += fl.ladung[g] - behalten;
                fl.ladung[g] = behalten;
                frei -= behalten;
            }
        }
        if summe > 0 {
            let typen: Vec<String> = (0..SCHIFFE).filter(|e| je_typ[*e] > 0).map(|e| format!("{} {}", je_typ[e], Einheit::ALLE[e])).collect();
            self.vorfall(
                sid,
                "unterhalt",
                format!(
                    "{summe} Schiffe desertiert, weil der Unterhalt seit Tagen fehlt: {}{}",
                    typen.join(", "),
                    if gekappt.iter().any(|n| *n >= M) { format!("; mitgenommene Fracht: {}", crate::berichte::fracht_text(&gekappt)) } else { String::new() }
                ),
            );
            self.wecke(sid, Rolle::Stratege, "Desertion", false);
        }
    }
}

/// Zivile Einheiten: Verwalter dürfen sie fertigen und zahlen dafür aus dem Topf Wirtschaft (auch das Antwortschema
/// des Verwalters bietet nur sie an).
pub fn zivile_einheit(e: Einheit) -> bool {
    matches!(
        e,
        Einheit::KleinerTransporter
            | Einheit::GrosserTransporter
            | Einheit::Bergbauschiff
            | Einheit::Recycler
            | Einheit::Kolonieschiff
            | Einheit::GrossesKolonieschiff
            | Einheit::Schnellfrachter
            | Einheit::Schwerlastfrachter
            | Einheit::Handelsfrachter
            | Einheit::Solarsonde
            | Einheit::Terraformer
    )
}
