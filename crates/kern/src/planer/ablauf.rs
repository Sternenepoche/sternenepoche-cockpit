//! Bounded, private economic replay. Commands and decision windows use the live core.
use crate::{aktion::Aktion, typen::*, welt::*, Welt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanAnfrage {
    pub schritte: Vec<Value>,
    pub horizont_stunden: u16,
    #[serde(default = "alle")]
    pub rolle: Rolle,
}
fn alle() -> Rolle {
    Rolle::Alle
}

pub fn validieren(q: &PlanAnfrage) -> Result<(), String> {
    if q.schritte.len() > 32 || !(1..=168).contains(&q.horizont_stunden) {
        return Err("Plan: höchstens 32 Schritte, Horizont 1–168 Spielstunden".into());
    }
    for v in &q.schritte {
        schritt(v)?;
    }
    Ok(())
}
fn schritt(v: &Value) -> Result<Aktion, String> {
    let a: Aktion = serde_json::from_value(v.clone()).map_err(|e| format!("Planschritt: {e}"))?;
    match &a {
        Aktion::Bauen { .. }
        | Aktion::Reparieren { .. }
        | Aktion::Abreissen { .. }
        | Aktion::Forschen { .. }
        | Aktion::Steuersatz { .. }
        | Aktion::Prioritaeten { .. }
        | Aktion::Stufenaufstieg {} => {}
        Aktion::Fertigen { anzahl, .. } | Aktion::RaketenBauen { anzahl, .. }
            if (1..=10000).contains(anzahl) => {}
        _ => return Err("Dieser Befehl gehört nicht zur Wirtschaftsplanung".into()),
    }
    Ok(a)
}

impl Welt {
    fn planer_eigen(&self, sid: SpielerId) -> BTreeSet<PlanetId> {
        self.planeten
            .iter()
            .filter(|p| p.besitzer == sid)
            .map(|p| p.id)
            .collect()
    }
    fn planer_ereignisse(&self, sid: SpielerId) -> Vec<Ereignis> {
        let own = self.planer_eigen(sid);
        let mut events: Vec<_> = self
            .ereignisse
            .iter()
            .filter(|e| match e.art {
                EreignisArt::Tick | EreignisArt::Tag => true,
                EreignisArt::BauFertig { planet }
                | EreignisArt::FertigungFertig { planet, .. }
                | EreignisArt::Marktlieferung { planet, .. } => own.contains(&planet),
                EreignisArt::RaketenFertig { besitzer, .. }
                | EreignisArt::MarktlieferungGebunden {
                    empfaenger: besitzer,
                    ..
                } => besitzer == sid,
                EreignisArt::FlotteAnkunft { flotte }
                | EreignisArt::FlotteRueckkehr { flotte }
                | EreignisArt::OrbitEnde { flotte } => {
                    self.flotten.get(&flotte).is_some_and(|f| f.besitzer == sid)
                }
                EreignisArt::InterneLieferung { angebot } => self
                    .kommunikation
                    .angebote
                    .iter()
                    .any(|a| a.id == angebot && a.kaeufer == Some(sid)),
                _ => false,
            })
            .cloned()
            .collect();
        events.sort_by_key(|e| (e.zeit, e.prio, e.seq));
        // Preserve relative order, but hide unrelated global event counts.
        for (i, e) in events.iter_mut().enumerate() {
            e.seq = i as u64;
        }
        events
    }
    pub fn plan_revision(&self, sid: SpielerId) -> Result<String, String> {
        let sp = self.spieler.get(sid as usize).ok_or("Spieler fehlt")?;
        let own = self.planer_eigen(sid);
        let planets: Vec<_> = own.iter().map(|p| &self.planeten[*p as usize]).collect();
        let fleets: Vec<_> = self
            .flotten
            .values()
            .filter(|f| f.besitzer == sid)
            .collect();
        let integrity: Vec<_> = self
            .kolonisation
            .integritaet
            .iter()
            .filter(|((p, _), _)| own.contains(p))
            .collect();
        let repairs: Vec<_> = self
            .kolonisation
            .reparaturen
            .iter()
            .filter(|((p, _), _)| own.contains(p))
            .collect();
        let offers: Vec<_> = self
            .kommunikation
            .angebote
            .iter()
            .filter(|a| a.kaeufer == Some(sid))
            .collect();
        let people: Vec<_> = self
            .ausbau
            .personen
            .iter()
            .filter(|(p, _)| own.contains(p))
            .collect();
        let schooling: Vec<_> = self
            .ausbau
            .ausbildungen
            .iter()
            .filter(|a| own.contains(&a.planet))
            .collect();
        let passenger: Vec<_> = self
            .ausbau
            .passagiere
            .iter()
            .filter(|(id, _)| self.flotten.get(id).is_some_and(|f| f.besitzer == sid))
            .collect();
        let growth: Vec<_> = self
            .ausbau
            .wachstumsrest
            .iter()
            .filter(|(p, _)| own.contains(p))
            .collect();
        let person_revision = super::kontext::fingerprint(&(people, schooling, passenger, growth,own.iter().map(|p|self.silo_sicht(*p as usize)).collect::<Vec<_>>(),self.salven_sicht(sid)));
        Ok(super::kontext::fingerprint(&(
            person_revision,
            own.iter().map(|p|(self.produktionsstufen(*p as usize),self.parallel_sicht(*p as usize,0),self.parallel_sicht(*p as usize,1),self.boden_sicht(*p as usize),self.produktionsserien_sicht(*p as usize))).collect::<Vec<_>>(),
            self.planer_regelkontext().wirksamer_regelhash,
            self.zeit,
            sp,
            planets,
            fleets,
            self.planer_ereignisse(sid),
            integrity,
            repairs,
            offers,
            self.reich_status(sid),
        )))
    }
    /// A whitelist of known self-contained paths, not a prediction of hidden battles.
    fn planer_unsicherheit(&self, sid: SpielerId) -> Option<&'static str> {
        let own = self.planer_eigen(sid);
        if self.ausbau.raketensystem.salven.values().any(|s|s.von==sid||s.gegner==sid){return Some("Raketenschlag hat einen noch veränderlichen Ausgang");}
        if own.iter().any(|p| {
            self.planeten[*p as usize].blockade.is_some()
                || self.kolonisation.besetzungen.contains_key(p)
        }) {
            return Some("Blockade oder Besetzung benötigt fremde Entscheidungen");
        }
        if self
            .flotten
            .values()
            .filter(|f| f.besitzer == sid)
            .any(|f| {
                f.zustand != Flottenzustand::Rueckflug
                    && !(matches!(f.mission, Mission::Transport | Mission::Stationieren)
                        && self.belegung.get(&f.ziel).is_some_and(|p| own.contains(p)))
            })
        {
            return Some("Eigene Außenmission hat noch einen unbekannten Ausgang");
        }
        if self.vertraege.iter().any(|v| {
            v.art == Vertragsart::Tribut
                && (v.a == sid || v.b == sid)
                && matches!(
                    v.status,
                    Vertragsstatus::Aktiv | Vertragsstatus::Gekuendigt { .. }
                )
        }) {
            return Some("Tributzahlungen hängen vom fremden Vertragspartner ab");
        }
        None
    }
    pub(crate) fn planer_kopie(&self, sid: SpielerId) -> Welt {
        let mut w = self.clone();
        let own = self.planer_eigen(sid);
        w.startwert = 0;
        w.logpuffer.clear();
        w.fachereignisse.clear();
        for sp in &mut w.spieler {
            sp.ki = false;
            if sp.id != sid {
                w.aufklaerung.inaktive_spieler.insert(sp.id);
            }
        }
        w.flotten.retain(|_, f| f.besitzer == sid);
        w.ereignisse = self.planer_ereignisse(sid).into_iter().collect();
        w.seq = w.ereignisse.len() as u64;
        w.kolonisation.besetzungen.clear();
        w.kolonisation.besetzungsmarken.clear();
        w.kolonisation.besetzungsangreifer.clear();
        w.kolonisation
            .integritaet
            .retain(|(p, _), _| own.contains(p));
        w.kolonisation
            .reparaturen
            .retain(|(p, _), _| own.contains(p));
        w.vertraege.clear();
        w.kommunikation.angebote.retain(|a| a.kaeufer == Some(sid));
        w.kommunikation.anfragen.clear();
        w.kommunikation.hilfe.clear();
        w
    }
    pub fn plan_zustand(&self, sid: SpielerId) -> Value {
        let sp = &self.spieler[sid as usize];
        json!({"zeit":self.zeit,"credits_milli":sp.credits,"toepfe_milli":sp.toepfe,
            "forschung":sp.forschung[..self.forschungszahl()].to_vec(),"forschung_aktiv":sp.forschung_aktiv,"forschung_schlange":sp.forschung_schlange,
            "stufe":sp.stufe,"stufen_zaehler":sp.stufen_zaehler,"schuld_seit":sp.schuld_seit,"reich":self.reich_status(sid),
            "planeten":sp.planeten.iter().map(|pid|{let p=&self.planeten[*pid as usize];json!({
                "planet":p.koord.to_string(),"bestand_milli":self.bestand_jetzt(*pid as usize),"rate_milli":p.rate,
                "personen":self.ausbau.personen.get(&p.id),"bevoelkerung_milli":p.bevoelkerung,"stabilitaet_milli":p.stabilitaet,"wohnraum_milli":p.wohnraum,
                "gebaeude":p.gebaeude.to_vec(),"einheiten":p.einheiten.to_vec(),"raketensilo":self.silo_sicht(p.id as usize),"raketen":p.raketen,
                "bau":p.bauschleife,"fertigung":p.fertigung,"energie":[p.energie_erzeugung,p.energie_verbrauch],
                "versorgung":[p.nahrung_deckung,p.konsum_deckung],"fp_rate":p.fp_rate})}).collect::<Vec<_>>()})
    }
    fn plan_fertig(&self, sid: SpielerId, a: &Aktion) -> bool {
        let planet = match a {
            Aktion::Bauen { planet, .. }
            | Aktion::Reparieren { planet, .. }
            | Aktion::Abreissen { planet, .. }
            | Aktion::Fertigen { planet, .. }
            | Aktion::RaketenBauen { planet, .. } => Some(*planet),
            _ => None,
        };
        let p = planet
            .and_then(|k| self.belegung.get(&k))
            .map(|p| &self.planeten[*p as usize]);
        match a {
            Aktion::Bauen { .. } => p.is_some_and(|p| p.bauschleife.is_empty()),
            Aktion::Reparieren { .. } => p.is_some_and(|p| {
                !self
                    .kolonisation
                    .reparaturen
                    .keys()
                    .any(|(pid, _)| *pid == p.id)
            }),
            Aktion::Fertigen { bauteil, .. } => {
                p.is_some_and(|p| p.fertigung[usize::from(bauteil.is_some())].is_empty())
            }
            Aktion::Forschen { .. } => {
                self.spieler[sid as usize].forschung_aktiv.is_none()
                    && self.spieler[sid as usize].forschung_schlange.is_empty()
            }
            Aktion::RaketenBauen { .. } => p.is_some_and(|p| {
                !self
                    .ereignisse
                    .iter()
                    .any(|e| matches!(e.art,EreignisArt::RaketenFertig{planet,..} if planet==p.id))
            }),
            _ => true,
        }
    }
    pub fn ablaufplan(&self, sid: SpielerId, q: &PlanAnfrage) -> Result<Value, String> {
        validieren(q)?;
        let revision = self.plan_revision(sid)?;
        let context = self.planer_regelkontext();
        if let Some(reason) = self.planer_unsicherheit(sid) {
            return Ok(
                json!({"status":"unknown","grund":reason,"revision":revision,"kontext":context,"schritte":[],"endzustand":self.plan_zustand(sid)}),
            );
        }
        let mut w = self.planer_kopie(sid);
        let mut rows: Vec<Value> = Vec::new();
        let mut samples = vec![w.plan_zustand(sid)];
        let end = (self.zeit + q.horizont_stunden as i64 * STUNDE).min(self.regeln.epochenende());
        let mut ix = 0;
        let mut active = None;
        let mut last_reason = String::new();
        let mut loops = 0;
        let mut status = "completed";
        loop {
            if !w.spieler_aktiv(sid) {
                status = "blocked";
                last_reason = "Reich ist inaktiv oder ausgeschieden".into();
                break;
            }
            if let Some(ref a) = active {
                if w.plan_fertig(sid, a) {
                    rows[ix]["fertig_im_fenster"] = json!(w.zeit);
                    ix += 1;
                    active = None;
                }
            }
            if ix < q.schritte.len() && active.is_none() {
                let a = schritt(&q.schritte[ix])?;
                // Rejected probes never change even the simulated world's statistics or log.
                let mut attempt = w.clone();
                let (ok, reason) = attempt.handeln(sid, q.rolle, &q.schritte[ix]);
                if ok {
                    w = attempt;
                    rows.push(json!({"index":ix,"aktion":q.schritte[ix],"begonnen":w.zeit,"fertig_im_fenster":null}));
                    active = Some(a);
                    last_reason.clear();
                    continue;
                }
                last_reason = reason;
            }
            if ix == q.schritte.len() && !q.schritte.is_empty() {
                break;
            }
            if w.zeit >= end || w.beendet() || loops >= 12000 {
                status = "horizon_reached";
                break;
            }
            if w.zeit + w.regeln.fenster() > end {
                status = "horizon_reached";
                break;
            }
            w.schritt();
            loops += 1;
            if samples
                .last()
                .and_then(|v| v["zeit"].as_i64())
                .is_none_or(|t| w.zeit / STUNDE != t / STUNDE)
            {
                samples.push(w.plan_zustand(sid));
            }
        }
        let final_state = w.plan_zustand(sid);
        if samples.last() != Some(&final_state) {
            samples.push(final_state.clone());
        }
        Ok(
            json!({"status":status,"grund":last_reason,"revision":revision,"kontext":context,"schritte":rows,
            "startzeit":self.zeit,"naechster_schritt":ix,"zeitreihe":samples,"endzustand":final_state,"fenster_berechnet":loops,
            "annahmen":["Geordnete Schritte: nächster Start nach Abschluss der betreffenden Warteschlange",
                "Exakte heutige Core-Ticks und Entscheidungsfenster; keine künftigen fremden Aktionen",
                "Eigene Rückflüge, interne Transporte und bereits gebuchte neutrale Lieferungen enthalten",
                "Fremde Hilfszusagen und unbekannte Außenmissionen sind keine garantierten Erträge",
                "Horizontende ist keine Behauptung dauerhafter Unmöglichkeit"]}),
        )
    }
    pub fn plan_uebernehmen(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        revision: &str,
        schritte: &[Value],
    ) -> Result<String, String> {
        if schritte.is_empty() || schritte.len() > 32 {
            return Err("1 bis 32 sofort ausführbare Schritte erforderlich".into());
        }
        if self.plan_revision(sid)? != revision {
            return Err("Plan veraltet oder bereits übernommen; neu berechnen".into());
        }
        let mut next = self.clone();
        for (i, v) in schritte.iter().enumerate() {
            schritt(v)?;
            let (ok, reason) = next.handeln(sid, rolle, v);
            if !ok {
                return Err(format!("Schritt {}: {reason}; nichts übernommen", i + 1));
            }
        }
        *self = next;
        Ok(format!("{} Planschritte atomar übernommen", schritte.len()))
    }
}
