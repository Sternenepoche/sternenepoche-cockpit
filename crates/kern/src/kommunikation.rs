//! Private mail, alliance chat and leadership mail share engine-enforced audiences.
//! Extension data never changes the legacy bincode layout. IDs are stable within an epoch.
use crate::{typen::*, welt::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap,BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Kommunikation {
    /// Optional JSON extensions; the legacy bincode world stays unchanged.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub allianz_tags: BTreeMap<u32, String>,
    /// Owner observations taken by a physical system probe, keyed by viewer:sector:system.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub systemkontakte: BTreeMap<String, BTreeMap<u8, Option<SpielerId>>>,
    pub briefe: BTreeMap<u64, Brief>,
    pub anfragen: Vec<Anfrage>,
    pub angebote: Vec<InternesAngebot>,
    pub hilfe: Vec<Hilferuf>,
    pub ereignisse: Vec<Value>,
    /// Number of older events a host already archived and removed from memory
    /// (`Welt::verlauf_auslagern`). Event ids stay continuous: id = ausgelagert + index + 1.
    /// Zero for kernels that keep the full history; then the saved layout is unchanged.
    #[serde(default, skip_serializing_if = "ist_null")]
    pub ereignisse_ausgelagert: u64,
    /// Derived lazily after loading; never changes saved worlds or state hashes.
    #[serde(skip)]
    offene_index: std::cell::RefCell<Option<BTreeSet<u64>>>,
    /// Derived lazily: (sender, recipient) pairs of private letters. Never saved or hashed.
    #[serde(skip)]
    kontakt_index: std::cell::RefCell<Option<BTreeSet<(SpielerId, SpielerId)>>>,
}
fn ist_null(n: &u64) -> bool {
    *n == 0
}
impl Kommunikation {
    /// All events of this epoch, including those already moved to a host archive.
    pub fn ereignisse_gesamt(&self) -> u64 {
        self.ereignisse_ausgelagert + self.ereignisse.len() as u64
    }
    /// Events with an id above `nr`, for archives that append continuously.
    /// Fails instead of silently skipping when such events are no longer in memory.
    pub fn ereignisse_nach(&self, nr: u64) -> Result<&[Value], String> {
        let ab = nr.checked_sub(self.ereignisse_ausgelagert).ok_or_else(|| {
            format!(
                "Kommunikationsereignisse {}..{} fehlen im Speicher und im Archiv",
                nr + 1,
                self.ereignisse_ausgelagert
            )
        })?;
        Ok(&self.ereignisse[(ab as usize).min(self.ereignisse.len())..])
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Brief {
    pub id: u64,
    pub kanal: String,
    pub betreff: String,
    pub allianzen: Vec<u32>,
    pub antwort_auf: Option<u64>,
    pub gelesen: Vec<SpielerId>,
    pub automatisch: bool,
    pub beantwortet: Vec<SpielerId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Anfrage {
    pub id: u64,
    pub von: SpielerId,
    pub an: SpielerId,
    pub art: String,
    pub text: String,
    pub status: String,
    pub zeit: SimZeit,
    pub vertrag: Option<u32>,
    pub allianz: Option<u32>,
    #[serde(default)]
    pub zielallianz: Option<u32>,
    #[serde(default)]
    pub kuendigung_ende: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InternesAngebot {
    pub id: u64,
    pub allianz: u32,
    pub von: SpielerId,
    pub planet: PlanetId,
    pub gut: Gut,
    pub menge: i64,
    pub preis: i64,
    pub status: String,
    pub kaeufer: Option<SpielerId>,
    pub ziel: Option<PlanetId>,
    pub ankunft: Option<SimZeit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hilferuf {
    pub id: u64,
    pub allianz: u32,
    pub von: SpielerId,
    pub planet: Koord,
    pub text: String,
    pub zeit: SimZeit,
    pub status: String,
    pub helfer: Vec<SpielerId>,
}
impl Welt {
    pub fn kommunikationsereignis(&mut self, sid: SpielerId, art: &str, payload: Value) {
        let e = json!({"id":self.kommunikation.ereignisse_gesamt()+1,"time":self.zeit,
            "event_type":art,"player_id":sid,"payload":payload});
        self.kommunikation.ereignisse.push(e.clone());
        self.fachereignisse.push(e);
    }
    pub fn allianz_id(&self, sid: SpielerId) -> Result<u32, String> {
        self.spieler
            .get(sid as usize)
            .and_then(|s| s.allianz)
            .ok_or("Du bist in keiner Allianz".into())
    }
    pub fn allianz_rang(&self, sid: SpielerId, aid: u32) -> Option<usize> {
        if self.spieler.get(sid as usize)?.allianz != Some(aid) {
            return None;
        }
        self.allianzen
            .iter()
            .find(|a| a.id == aid)?
            .mitglieder
            .iter()
            .position(|m| *m == sid)
    }
    pub fn allianz_leitung(&self, sid: SpielerId, aid: u32) -> bool {
        self.allianz_rang(sid, aid).is_some_and(|r| r < 4)
    }
    fn nachricht_pruefen(&self, sid: SpielerId, text: &str) -> Result<(), String> {
        if !self.spieler_aktiv(sid) {
            return Err("Kein aktiver Absender".into());
        }
        if text.trim().is_empty() || text.chars().count() > self.regeln.agenten.nachricht_zeichen {
            return Err(format!(
                "Text braucht 1 bis {} Zeichen",
                self.regeln.agenten.nachricht_zeichen
            ));
        }
        if self.spieler[sid as usize].nachrichten_heute >= self.regeln.agenten.nachrichten_je_tag {
            return Err("Nachrichten-Tageslimit erreicht".into());
        }
        Ok(())
    }
    pub fn brief_sichtbar(&self, sid: SpielerId, id: u64) -> bool {
        let Some(n) = id
            .checked_sub(1)
            .and_then(|i| self.nachrichten.get(i as usize))
        else {
            return false;
        };
        if n.von != sid && !n.an.contains(&sid) {
            return false;
        }
        let Some(b) = self.kommunikation.briefe.get(&id) else {
            return !n.allianz;
        };
        match b.kanal.as_str() {
            "privat" => true,
            "allianz" => b
                .allianzen
                .iter()
                .any(|a| self.allianz_rang(sid, *a).is_some()),
            "diplomatie" => b.allianzen.iter().any(|a| self.allianz_leitung(sid, *a)),
            _ => false,
        }
    }
    fn brief_speichern(
        &mut self,
        sid: SpielerId,
        empfaenger: Vec<SpielerId>,
        kanal: &str,
        betreff: &str,
        text: &str,
        allianzen: Vec<u32>,
        antwort_auf: Option<u64>,
        automatisch: bool,
    ) -> u64 {
        let id = self.nachrichten.len() as u64 + 1;
        let mut beantwortete = Vec::new();
        // A natural-language reply also satisfies the first-answer obligation.
        if kanal == "privat" {
            let ids=antwort_auf.map(|id|vec![id]).unwrap_or_else(||self.kommunikation.briefe.keys().copied().collect());
            for bid in ids {
                let Some(b)=self.kommunikation.briefe.get_mut(&bid) else {continue;};
                let n = &self.nachrichten[bid as usize - 1];
                if b.kanal == "privat"
                    && n.an.contains(&sid)
                    && empfaenger.contains(&n.von)
                    && !b.beantwortet.contains(&sid)
                    && (antwort_auf.is_none() || antwort_auf == Some(bid))
                {
                    b.beantwortet.push(sid);
                    beantwortete.push(bid);
                    if n.an.iter().all(|sid|b.beantwortet.contains(sid)) {if let Some(index)=self.kommunikation.offene_index.get_mut(){index.remove(&bid);}}
                }
            }
        }
        self.nachrichten.push(Nachricht {
            zeit: self.zeit,
            von: sid,
            an: empfaenger.clone(),
            allianz: kanal != "privat",
            text: text.trim().into(),
        });
        self.kommunikation.briefe.insert(
            id,
            Brief {
                id,
                kanal: kanal.into(),
                betreff: betreff.trim().into(),
                allianzen: allianzen.clone(),
                antwort_auf,
                gelesen: vec![sid],
                automatisch,
                beantwortet: Vec::new(),
            },
        );
        if kanal=="privat"&&!automatisch&&antwort_auf.is_none(){if let Some(index)=self.kommunikation.offene_index.get_mut(){index.insert(id);}}
        if kanal=="privat"{if let Some(index)=self.kommunikation.kontakt_index.get_mut(){for e in &empfaenger{index.insert((sid,*e));}}}
        self.kommunikationsereignis(sid,"communication.sent",json!({"message_id":id,"channel":kanal,"recipients":empfaenger,"alliances":allianzen,"reply_to":antwort_auf,"automatic":automatisch,"subject":betreff,"text":text.trim()}));
        for bid in beantwortete {
            self.kommunikationsereignis(
                sid,
                "communication.answered",
                json!({"message_id":bid,"reply_id":id}),
            );
        }
        for e in empfaenger {
            self.wecke(e, Rolle::Diplomat, "Neue Post", false);
        }
        id
    }
    pub fn brief_senden(
        &mut self,
        sid: SpielerId,
        kanal: &str,
        an: &str,
        betreff: &str,
        text: &str,
        antwort_auf: Option<u64>,
    ) -> Result<String, String> {
        self.nachricht_pruefen(sid, text)?;
        if betreff.chars().count() > 120 {
            return Err("Betreff höchstens 120 Zeichen".into());
        }
        let (empfaenger, allianzen) = match kanal {
            "privat" => {
                let e = self.spieler_nach_name(an.trim())?;
                if e == sid || !self.spieler_aktiv(e) {
                    return Err("Anderen aktiven Spieler angeben".into());
                }
                (vec![e], vec![])
            }
            "allianz" => {
                if !an.trim().is_empty() {
                    return Err("Allianzchat erlaubt keine externen Empfänger".into());
                }
                let aid = self.allianz_id(sid)?;
                let a = self
                    .allianzen
                    .iter()
                    .find(|a| a.id == aid)
                    .ok_or("Allianz fehlt")?;
                (
                    a.mitglieder.iter().copied().filter(|m| *m != sid).collect(),
                    vec![aid],
                )
            }
            "diplomatie" => {
                let aid = self.allianz_id(sid)?;
                if !self.allianz_leitung(sid, aid) {
                    return Err("Nur die obersten vier Mitglieder dürfen Allianzpost senden".into());
                }
                let other = self
                    .allianzen
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case(an.trim()) && !a.mitglieder.is_empty())
                    .ok_or("Zielallianz fehlt")?;
                if other.id == aid {
                    return Err("Andere Allianz angeben".into());
                }
                let ids = vec![aid, other.id];
                let recipients = self
                    .allianzen
                    .iter()
                    .filter(|a| ids.contains(&a.id))
                    .flat_map(|a| a.mitglieder.iter().take(4).copied())
                    .filter(|m| *m != sid)
                    .collect();
                (recipients, ids)
            }
            _ => return Err("Kanal muss privat, allianz oder diplomatie sein".into()),
        };
        if let Some(id) = antwort_auf {
            if !self.brief_sichtbar(sid, id) {
                return Err("Ursprungsbrief nicht zugänglich".into());
            }
            let b = self
                .kommunikation
                .briefe
                .get(&id)
                .ok_or("Alter Brief ohne Antwortverknüpfung")?;
            let n = &self.nachrichten[id as usize - 1];
            if b.kanal != kanal
                || (kanal == "privat" && !empfaenger.contains(&n.von))
                || (kanal != "privat" && b.allianzen.iter().any(|a| !allianzen.contains(a)))
            {
                return Err("Antwort muss im ursprünglichen Kommunikationsraum bleiben".into());
            }
        }
        self.spieler[sid as usize].nachrichten_heute += 1;
        let id = self.brief_speichern(
            sid,
            empfaenger,
            kanal,
            betreff,
            text,
            allianzen,
            antwort_auf,
            false,
        );
        Ok(format!("Nachricht {id} zugestellt"))
    }
    pub fn brief_lesen(&mut self, sid: SpielerId, id: u64) -> Result<String, String> {
        if !self.brief_sichtbar(sid, id) {
            return Err("Brief nicht zugänglich".into());
        }
        if let Some(b) = self.kommunikation.briefe.get_mut(&id) {
            if !b.gelesen.contains(&sid) {
                b.gelesen.push(sid);
                self.kommunikationsereignis(sid, "communication.read", json!({"message_id":id}));
            }
        }
        Ok("Als gelesen markiert".into())
    }
    /// Only root letters require an answer. Replies and automatic receipts never ping-pong.
    fn offene_brief_ids(&self)->Vec<u64> {
        let mut index=self.kommunikation.offene_index.borrow_mut();
        if index.is_none(){*index=Some(self.kommunikation.briefe.iter().filter(|(id,b)|b.kanal=="privat"&&!b.automatisch&&b.antwort_auf.is_none()&&self.nachrichten[**id as usize-1].an.iter().any(|sid|!b.beantwortet.contains(sid))).map(|(id,_)|*id).collect());}
        index.as_ref().unwrap().iter().copied().collect()
    }
    /// Has `von` ever written a private letter to `an` in this epoch? O(log n) after the first call.
    pub fn privat_kontakt(&self, von: SpielerId, an: SpielerId) -> bool {
        let mut index=self.kommunikation.kontakt_index.borrow_mut();
        if index.is_none(){*index=Some(self.kommunikation.briefe.iter().filter(|(_,b)|b.kanal=="privat").flat_map(|(id,_)|{let n=&self.nachrichten[*id as usize-1];n.an.iter().map(move|e|(n.von,*e))}).collect());}
        index.as_ref().unwrap().contains(&(von,an))
    }
    /// Moves communication events older than `vor_zeit` out of memory, but only those with an
    /// id up to `archiviert_bis` (already written to the host's archive). Returns the count.
    /// Only hosts with a durable archive call this; simulations keep the full history.
    pub fn verlauf_auslagern(&mut self, vor_zeit: SimZeit, archiviert_bis: u64) -> usize {
        let k=&mut self.kommunikation;
        let erlaubt=archiviert_bis.saturating_sub(k.ereignisse_ausgelagert).min(k.ereignisse.len() as u64) as usize;
        let n=k.ereignisse[..erlaubt].iter().take_while(|e|e["time"].as_i64().unwrap_or(SimZeit::MAX)<vor_zeit).count();
        if n>0 {k.ereignisse.drain(..n);k.ereignisse_ausgelagert+=n as u64;}
        n
    }
    /// Host retention for long-running worlds: battle reports and daily values older than
    /// `vor_zeit` leave memory. The view shows only recent reports; analysis runs (`lauf`)
    /// never call this and keep complete tables. Returns (reports, daily rows) removed.
    pub fn berichte_auslagern(&mut self, vor_zeit: SimZeit) -> (usize, usize) {
        let k=self.kampfberichte.iter().take_while(|b|b.zeit<vor_zeit).count();
        self.kampfberichte.drain(..k);
        let tag=vor_zeit.div_euclid(TAG);
        let t=self.tageswerte.iter().take_while(|z|z.tag<tag).count();
        self.tageswerte.drain(..t);
        (k,t)
    }
    pub fn offene_antworten(&self, sid: SpielerId) -> Vec<u64> {
        self.offene_brief_ids().into_iter().filter(|id|!self.kommunikation.briefe[id].beantwortet.contains(&sid)&&self.nachrichten[*id as usize-1].an.contains(&sid)).collect()
    }
    pub fn kommunikation_tick(&mut self) {
        // Scan the correspondence once, instead of the complete history for every
        // player at every decision window. Preserve the original player/id order.
        let ids=self.offene_brief_ids();
        let mut pending = ids.iter().map(|id|(id,&self.kommunikation.briefe[id]))
            .filter(|(id,b)|b.kanal=="privat"&&!b.automatisch&&b.antwort_auf.is_none()&&self.zeit-self.nachrichten[**id as usize-1].zeit>=STUNDE)
            .flat_map(|(id,b)|self.nachrichten[*id as usize-1].an.iter().filter(|sid|self.spieler[**sid as usize].ki&&self.spieler_aktiv(**sid)&&!b.beantwortet.contains(sid)).map(move |sid|(*sid,*id)))
            .collect::<Vec<_>>();
        pending.sort_unstable();
        for (sid, id) in pending {
            self.empfang_bei_zeitmangel(sid, id);
        }
        let expired = self
            .kommunikation
            .angebote
            .iter()
            .filter(|o| {
                o.status == "offen"
                    && (self.spieler[o.von as usize].allianz != Some(o.allianz)
                        || !self.spieler_aktiv(o.von)
                        || self.planeten[o.planet as usize].besitzer != o.von)
            })
            .map(|o| o.id)
            .collect::<Vec<_>>();
        for id in expired {
            self.intern_storno_ungeprueft(id);
        }
    }

    pub fn empfang_bei_zeitmangel(&mut self, sid: SpielerId, id: u64) {
        let Some(b)=self.kommunikation.briefe.get(&id) else {return;};
        if b.kanal!="privat"||b.automatisch||b.antwort_auf.is_some()||b.beantwortet.contains(&sid)||!self.nachrichten[id as usize-1].an.contains(&sid) {
            return;
        }
        let n = self.nachrichten[id as usize - 1].clone();
        if self.zeit - n.zeit < STUNDE {
            return;
        }
        let text=format!("Ich bin {}. Deine Nachricht ist angekommen. Meine Regierung hat derzeit keine freie Kapazität für ein Gespräch. Automatische Empfangsantwort; Anfragen werden getrennt entschieden.",self.spieler[sid as usize].name);
        self.brief_speichern(
            sid,
            vec![n.von],
            "privat",
            "Empfang bestätigt",
            &text,
            vec![],
            Some(id),
            true,
        );
    }
    pub fn diplomatie_anfrage(
        &mut self,
        sid: SpielerId,
        partner: &str,
        art: &str,
        text: &str,
    ) -> Result<String, String> {
        self.nachricht_pruefen(sid, text)?;
        let an = self.spieler_nach_name(partner)?;
        if an == sid || !self.spieler_aktiv(an) {
            return Err("Anderen aktiven Partner angeben".into());
        }
        if self
            .kommunikation
            .anfragen
            .iter()
            .any(|a| a.status == "offen" && a.von == sid && a.an == an && a.art == art)
        {
            return Err("Gleiche Anfrage ist bereits offen".into());
        }
        let mut vertrag = None;
        let mut allianz = None;
        match art {
            "nichtangriffspakt" | "verteidigungsbuendnis" | "handelsabkommen" => {
                let a = match art {
                    "nichtangriffspakt" => Vertragsart::Nichtangriffspakt,
                    "verteidigungsbuendnis" => Vertragsart::Verteidigungsbuendnis,
                    _ => Vertragsart::Handelsabkommen,
                };
                self.vertrag_anbieten(sid, partner, a, 0, None, 0, 0)?;
                vertrag = self.vertraege.last().map(|v| v.id);
            }
            "allianz" => {
                self.allianz_einladen(sid, partner)?;
                allianz = self.spieler[sid as usize].allianz;
            }
            "krieg" => {
                self.bruch_pruefen(sid, an);
            }
            "frieden" => {}
            _ => return Err("Unbekannte Anfrageart".into()),
        }
        let id = self.kommunikation.anfragen.len() as u64 + 1;
        self.kommunikation.anfragen.push(Anfrage {
            id,
            von: sid,
            an,
            art: art.into(),
            text: text.into(),
            status: if art == "krieg" { "erklaert" } else { "offen" }.into(),
            zeit: self.zeit,
            vertrag,
            allianz,
            zielallianz: None,
            kuendigung_ende: None,
        });
        self.spieler[sid as usize].nachrichten_heute += 1;
        let mid = self.brief_speichern(sid, vec![an], "privat", art, text, vec![], None, false);
        self.kommunikationsereignis(sid,"diplomacy.request",json!({"request_id":id,"message_id":mid,"recipient":an,"kind":art,"contract":vertrag,"alliance":allianz}));
        if art == "krieg" {
            self.register.push(Registereintrag {
                zeit: self.zeit,
                vertrag: 0,
                art: "krieg".into(),
                a: self.spieler[sid as usize].name.clone(),
                b: self.spieler[an as usize].name.clone(),
                vorgang: "Krieg erklärt".into(),
            });
        }
        Ok(format!("Anfrage {id}: {art}"))
    }
    pub fn diplomatie_entscheiden(
        &mut self,
        sid: SpielerId,
        id: u64,
        annehmen: bool,
    ) -> Result<String, String> {
        let i = id.checked_sub(1).ok_or("Anfrage fehlt")? as usize;
        let a = self
            .kommunikation
            .anfragen
            .get(i)
            .ok_or("Anfrage fehlt")?
            .clone();
        let permitted = if let Some(aid) = a.zielallianz {
            self.allianz_leitung(sid, aid)
        } else {
            a.an == sid
        };
        if !permitted || a.status != "offen" {
            return Err("Keine offene Anfrage an dich".into());
        }
        if let Some(target) = a.zielallianz {
            let source = a.allianz.ok_or("Quellallianz fehlt")?;
            if !self
                .allianzen
                .iter()
                .any(|al| al.id == source && !al.mitglieder.is_empty())
            {
                return Err("Quellallianz besteht nicht mehr".into());
            }
            if a.art == "frieden" && annehmen {
                let mut ended = Vec::new();
                for r in &mut self.kommunikation.anfragen {
                    if r.art == "krieg"
                        && r.status == "erklaert"
                        && ((r.allianz == Some(source) && r.zielallianz == Some(target))
                            || (r.allianz == Some(target) && r.zielallianz == Some(source)))
                    {
                        r.status = "beendet".into();
                        ended.push(r.id);
                    }
                }
                for id in ended {
                    self.kommunikationsereignis(
                        sid,
                        "alliance.war.ended",
                        json!({"request_id":id,"peace_request":a.id}),
                    );
                }
            }
            self.kommunikation.anfragen[i].status =
                if annehmen { "angenommen" } else { "abgelehnt" }.into();
            self.kommunikationsereignis(sid,"alliance.diplomacy.decision",json!({"request_id":id,"alliance":source,"target_alliance":target,"accepted":annehmen,"kind":a.art}));
            return Ok("Allianzanfrage entschieden".into());
        }
        if let Some(v) = a.vertrag {
            if annehmen {
                self.vertrag_annehmen(sid, v)?;
            } else {
                self.vertrag_ablehnen(sid, v)?;
            }
        } else if a.art == "allianz" {
            let aid = a.allianz.ok_or("Einladung ohne Allianz")?;
            if annehmen {
                let name = self
                    .allianzen
                    .iter()
                    .find(|al| al.id == aid && !al.mitglieder.is_empty())
                    .ok_or("Allianz besteht nicht mehr")?
                    .name
                    .clone();
                self.allianz_beitreten(sid, &name)?;
            } else if let Some(al) = self.allianzen.iter_mut().find(|al| al.id == aid) {
                al.eingeladen.retain(|s| *s != sid);
            }
        } else if a.art == "frieden" && annehmen {
            let mut ended = Vec::new();
            for w in &mut self.kommunikation.anfragen {
                if w.zielallianz.is_none()
                    && w.art == "krieg"
                    && w.status == "erklaert"
                    && ((w.von == sid && w.an == a.von) || (w.an == sid && w.von == a.von))
                {
                    w.status = "beendet".into();
                    ended.push(w.id);
                }
            }
            for id in ended {
                self.kommunikationsereignis(
                    sid,
                    "diplomacy.war.ended",
                    json!({"request_id":id,"peace_request":a.id}),
                );
            }
            self.register.push(Registereintrag {
                zeit: self.zeit,
                vertrag: 0,
                art: "frieden".into(),
                a: self.spieler[sid as usize].name.clone(),
                b: self.spieler[a.von as usize].name.clone(),
                vorgang: "Frieden vereinbart".into(),
            });
        }
        let status = if annehmen { "angenommen" } else { "abgelehnt" };
        self.kommunikation.anfragen[i].status = status.into();
        self.kommunikationsereignis(
            sid,
            "diplomacy.decision",
            json!({"request_id":id,"recipient":a.von,"accepted":annehmen,"kind":a.art}),
        );
        self.brief_speichern(
            sid,
            vec![a.von],
            "privat",
            "Entscheidung",
            &format!("{}: {status} (Anfrage {id})", a.art),
            vec![],
            None,
            true,
        );
        Ok(format!("Anfrage {id} {status}"))
    }
    pub fn allianz_rolle(
        &mut self,
        sid: SpielerId,
        spieler: &str,
        rang: u8,
    ) -> Result<String, String> {
        let aid = self.allianz_id(sid)?;
        if self.allianz_rang(sid, aid) != Some(0) {
            return Err("Nur die Allianzleitung darf Ränge vergeben".into());
        }
        let ziel = self.spieler_nach_name(spieler)?;
        let old = self
            .allianz_rang(ziel, aid)
            .ok_or("Spieler ist kein Mitglied")?;
        let a = self.allianzen.iter_mut().find(|a| a.id == aid).unwrap();
        if rang == 0 || rang as usize > a.mitglieder.len() {
            return Err("Rang außerhalb der Mitgliederliste".into());
        }
        a.mitglieder.swap(old, rang as usize - 1);
        self.kommunikationsereignis(
            sid,
            "alliance.role",
            json!({"alliance":aid,"member":ziel,"rank":rang,"previous_rank":old+1}),
        );
        Ok("Rang vergeben; Leitungsrechte sofort neu geprüft".into())
    }
    pub fn allianz_ausschliessen(
        &mut self,
        sid: SpielerId,
        spieler: &str,
    ) -> Result<String, String> {
        let aid = self.allianz_id(sid)?;
        if self.allianz_rang(sid, aid) != Some(0) {
            return Err("Nur die Allianzleitung darf ausschließen".into());
        }
        let ziel = self.spieler_nach_name(spieler)?;
        if ziel == sid || self.allianz_rang(ziel, aid).is_none() {
            return Err("Anderes Mitglied angeben".into());
        }
        self.allianz_verlassen(ziel)?;
        self.kommunikationsereignis(
            sid,
            "alliance.expelled",
            json!({"alliance":aid,"member":ziel}),
        );
        Ok("Mitglied ausgeschlossen".into())
    }
    pub fn allianz_hilfe(
        &mut self,
        sid: SpielerId,
        planet: Koord,
        text: &str,
    ) -> Result<String, String> {
        self.nachricht_pruefen(sid, text)?;
        let aid = self.allianz_id(sid)?;
        self.eigener_planet(sid, planet)?;
        let id = self.kommunikation.hilfe.len() as u64 + 1;
        self.brief_senden(
            sid,
            "allianz",
            "",
            "Hilferuf",
            &format!("{planet}: {text}"),
            None,
        )?;
        self.kommunikation.hilfe.push(Hilferuf {
            id,
            allianz: aid,
            von: sid,
            planet,
            text: text.into(),
            zeit: self.zeit,
            status: "offen".into(),
            helfer: vec![],
        });
        self.kommunikationsereignis(
            sid,
            "alliance.help.requested",
            json!({"help_id":id,"alliance":aid,"planet":planet,"text":text}),
        );
        let members = self
            .allianzen
            .iter()
            .find(|a| a.id == aid)
            .unwrap()
            .mitglieder
            .clone();
        for m in members {
            if m != sid {
                self.wecke(m, Rolle::Feldherr, "Allianzmitglied bittet um Hilfe", true);
            }
        }
        Ok(format!("Hilferuf {id} veröffentlicht"))
    }
    pub fn allianz_hilfe_status(
        &mut self,
        sid: SpielerId,
        id: u64,
        erledigt: bool,
    ) -> Result<String, String> {
        let aid = self.allianz_id(sid)?;
        let h = self
            .kommunikation
            .hilfe
            .iter_mut()
            .find(|h| h.id == id && h.allianz == aid)
            .ok_or("Hilferuf nicht zugänglich")?;
        if h.status != "offen" {
            return Err("Hilferuf bereits geschlossen".into());
        }
        if erledigt {
            if h.von != sid {
                return Err("Nur der Anfragende kann den Hilferuf schließen".into());
            }
            h.status = "erledigt".into();
        } else if !h.helfer.contains(&sid) {
            h.helfer.push(sid);
        }
        self.kommunikationsereignis(
            sid,
            "alliance.help.response",
            json!({"help_id":id,"alliance":aid,"closed":erledigt}),
        );
        Ok("Hilferuf aktualisiert; zugesagte Hilfe benötigt einen Flottenauftrag".into())
    }
    pub fn intern_anbieten(
        &mut self,
        sid: SpielerId,
        planet: Koord,
        gut: Gut,
        menge: i64,
        preis: i64,
    ) -> Result<String, String> {
        let aid = self.allianz_id(sid)?;
        let pid = self.eigener_planet(sid, planet)?;
        if menge < M
            || preis < 0
            || menge as i128 * preis as i128 / M as i128 > i64::MAX as i128 / 4
        {
            return Err("Ungültige Menge oder Preis".into());
        }
        let markt = self.planeten[pid].gebaeude[Gebaeude::Markt.idx()] as usize;
        if markt == 0 {
            return Err("Ein Markt auf dem Angebotsplaneten ist erforderlich".into());
        }
        if self
            .kommunikation
            .angebote
            .iter()
            .filter(|o| o.von == sid && o.planet == pid as PlanetId && o.status == "offen")
            .count()
            + self
                .orders
                .iter()
                .filter(|o| o.spieler == sid && o.planet == pid as PlanetId)
                .count()
            >= markt * self.regeln.markt.orders_je_marktstufe
        {
            return Err("Orderlimit erreicht".into());
        }
        self.abrechnen(pid);
        if self.planeten[pid].bestand[gut.idx()] < menge {
            return Err("Nicht genug Ware".into());
        }
        self.planeten[pid].bestand[gut.idx()] -= menge;
        let id = self.kommunikation.angebote.len() as u64 + 1;
        self.kommunikation.angebote.push(InternesAngebot {
            id,
            allianz: aid,
            von: sid,
            planet: pid as PlanetId,
            gut,
            menge,
            preis,
            status: "offen".into(),
            kaeufer: None,
            ziel: None,
            ankunft: None,
        });
        self.kommunikationsereignis(sid,"alliance.trade.offered",json!({"offer_id":id,"alliance":aid,"good":gut,"quantity_milli":menge,"price_milli":preis}));
        Ok(format!("Internes Angebot {id}; Ware reserviert"))
    }
    pub fn intern_kaufen(
        &mut self,
        sid: SpielerId,
        id: u64,
        planet: Koord,
    ) -> Result<String, String> {
        let aid = self.allianz_id(sid)?;
        let pid = self.eigener_planet(sid, planet)?;
        let o = self
            .kommunikation
            .angebote
            .iter()
            .find(|o| o.id == id && o.allianz == aid)
            .ok_or("Angebot nicht zugänglich")?
            .clone();
        if o.status != "offen"
            || o.von == sid
            || self.spieler[o.von as usize].allianz != Some(aid)
            || !self.spieler_aktiv(o.von)
            || self.planeten[o.planet as usize].besitzer != o.von
        {
            return Err("Angebot nicht mehr verfügbar".into());
        }
        let wert = (o.menge as i128 * o.preis as i128 / M as i128) as i64;
        let kosten = wert + mal(wert, self.gebuehr(sid, Some(o.von)));
        let erloes = wert - mal(wert, self.gebuehr(o.von, Some(sid)));
        if self.spieler[sid as usize].credits < kosten {
            return Err("Nicht genug Credits einschließlich Marktgebühr".into());
        }
        let neu = self.spieler[o.von as usize]
            .credits
            .checked_add(erloes)
            .ok_or("Creditgrenze erreicht")?;
        self.spieler[sid as usize].credits -= kosten;
        self.spieler[o.von as usize].credits = neu;
        let dauer = self.flugdauer(
            self.entfernung(self.planeten[o.planet as usize].koord, planet),
            self.regeln.markt.liefertempo,
            1000,
        );
        self.plane(
            self.zeit + dauer,
            EreignisArt::InterneLieferung { angebot: id },
        );
        let item = &mut self.kommunikation.angebote[id as usize - 1];
        item.status = "unterwegs".into();
        item.kaeufer = Some(sid);
        item.ziel = Some(pid as PlanetId);
        item.ankunft = Some(self.zeit + dauer);
        self.handel.push(Handel {
            zeit: self.zeit,
            gut: o.gut,
            menge: o.menge,
            preis: o.preis,
            kaeufer: sid,
            verkaeufer: o.von,
        });
        self.kommunikationsereignis(sid,"alliance.trade.bought",json!({"offer_id":id,"alliance":aid,"seller":o.von,"buyer":sid,"cost_milli":kosten,"arrival":self.zeit+dauer}));
        Ok(format!("Gekauft; Lieferung in {} Spielsekunden", dauer))
    }
    pub fn intern_liefern(&mut self, id: u64) {
        let Some(o) = self
            .kommunikation
            .angebote
            .iter()
            .find(|o| o.id == id && o.status == "unterwegs")
            .cloned()
        else {
            return;
        };
        let buyer = o.kaeufer.unwrap();
        let pid = o.ziel.unwrap() as usize;
        if !self.spieler_aktiv(buyer) || self.spieler[buyer as usize].planeten.is_empty() {
            self.kommunikation.angebote[id as usize - 1].status = "verloren".into();
            self.kommunikationsereignis(
                buyer,
                "alliance.trade.lost",
                json!({"offer_id":id,"alliance":o.allianz,"reason":"buyer_eliminated"}),
            );
            return;
        }
        if self.planeten[pid].besitzer != buyer || self.planeten[pid].blockade.is_some() {
            let target = if self.planeten[pid].besitzer == buyer {
                pid as PlanetId
            } else {
                self.spieler[buyer as usize].planeten[0]
            };
            let delay = if target as usize == pid {
                STUNDE
            } else {
                self.flugdauer(
                    self.entfernung(
                        self.planeten[pid].koord,
                        self.planeten[target as usize].koord,
                    ),
                    self.regeln.markt.liefertempo,
                    1000,
                )
            };
            let item = &mut self.kommunikation.angebote[id as usize - 1];
            item.ziel = Some(target);
            item.ankunft = Some(self.zeit + delay);
            self.plane(
                self.zeit + delay,
                EreignisArt::InterneLieferung { angebot: id },
            );
            self.kommunikationsereignis(buyer,"alliance.trade.delayed",json!({"offer_id":id,"alliance":o.allianz,"target":target,"arrival":self.zeit+delay}));
            return;
        }
        self.abrechnen(pid);
        self.planeten[pid].bestand[o.gut.idx()] += o.menge;
        self.kommunikation.angebote[id as usize - 1].status = "geliefert".into();
        self.kommunikationsereignis(buyer,"alliance.trade.delivered",json!({"offer_id":id,"alliance":o.allianz,"seller":o.von,"buyer":buyer,"planet":pid,"quantity_milli":o.menge,"good":o.gut}));
        self.vorfall(
            buyer,
            "markt",
            format!(
                "Allianzlieferung {id}: {} {} auf {}",
                ganz(o.menge),
                o.gut,
                self.planeten[pid].koord
            ),
        );
    }
    fn intern_storno_ungeprueft(&mut self, id: u64) {
        let o = self.kommunikation.angebote[id as usize - 1].clone();
        if o.status != "offen" {
            return;
        }
        // Never refund reserved goods into an enemy's captured world.
        let pid = if self.planeten[o.planet as usize].besitzer == o.von {
            Some(o.planet)
        } else {
            self.spieler[o.von as usize].planeten.first().copied()
        };
        if let Some(pid) = pid {
            self.abrechnen(pid as usize);
            self.planeten[pid as usize].bestand[o.gut.idx()] =
                self.planeten[pid as usize].bestand[o.gut.idx()].saturating_add(o.menge);
        }
        self.kommunikation.angebote[id as usize - 1].status = if pid.is_some() {
            "storniert"
        } else {
            "verloren"
        }
        .into();
        self.kommunikationsereignis(o.von,"alliance.trade.cancelled",json!({"offer_id":id,"alliance":o.allianz,"refund_planet":pid,"quantity_milli":o.menge}));
    }
    pub fn intern_storno(&mut self, sid: SpielerId, id: u64) -> Result<String, String> {
        if !self
            .kommunikation
            .angebote
            .iter()
            .any(|o| o.id == id && o.von == sid && o.status == "offen")
        {
            return Err("Kein eigenes offenes Angebot".into());
        }
        self.intern_storno_ungeprueft(id);
        Ok("Angebot storniert".into())
    }
    pub fn briefkasten(&self, sid: SpielerId) -> Vec<Value> {
        self.briefkasten_seite(sid, u64::MAX, 100, "alle")
    }
    pub fn briefkasten_seite(
        &self,
        sid: SpielerId,
        before: u64,
        limit: usize,
        channel: &str,
    ) -> Vec<Value> {
        self.nachrichten.iter().enumerate().rev().filter(|(i,_)|{
            let id=*i as u64+1;id<before&&self.brief_sichtbar(sid,id)&&(channel=="alle"||self.kommunikation.briefe.get(&id).map(|b|b.kanal.as_str()).unwrap_or("privat")==channel)
        }).take(limit.min(100)).map(|(i,n)|{
            let id=i as u64+1;let b=self.kommunikation.briefe.get(&id);
            json!({"id":id,"von_beziehung":self.beziehung(sid,n.von),"an_beziehungen":n.an.iter().map(|id|self.beziehung(sid,*id)).collect::<Vec<_>>(),"zeit":zeittext(n.zeit),"sekunden":n.zeit,"von":self.spieler[n.von as usize].name,"an":n.an.iter().map(|s|&self.spieler[*s as usize].name).collect::<Vec<_>>(),"text":n.text,"allianz":n.allianz,
                "kanal":b.map(|b|b.kanal.as_str()).unwrap_or("privat"),"betreff":b.map(|b|b.betreff.as_str()).unwrap_or("Nachricht"),"antwort_auf":b.and_then(|b|b.antwort_auf),"gelesen":b.is_some_and(|b|b.gelesen.contains(&sid)),"automatisch":b.is_some_and(|b|b.automatisch),"gesendet":n.von==sid,
                "zielallianz":b.and_then(|b|b.allianzen.iter().find(|a|Some(**a)!=self.spieler[sid as usize].allianz)).and_then(|id|self.allianzen.iter().find(|a|a.id==*id)).map(|a|&a.name)})
        }).collect()
    }
    pub fn kommunikation_sicht(&self, sid: SpielerId) -> Value {
        let aid = self.spieler[sid as usize].allianz;
        let anfragen = self
            .kommunikation
            .anfragen
            .iter()
            .filter(|a| a.zielallianz.is_none() && (a.von == sid || a.an == sid))
            .rev()
            .map(|a| {
                let mut v = json!(a);
                v["von"] = json!(self.spieler[a.von as usize].name);
                v["an"] = json!(self.spieler[a.an as usize].name);
                if let Some(id) = a.vertrag {
                    if let Some(t) = self.vertraege.iter().find(|t| t.id == id) {
                        if t.status != Vertragsstatus::Angeboten {
                            v["status"] = json!(match t.status {
                                Vertragsstatus::Aktiv => "angenommen",
                                Vertragsstatus::Abgelehnt => "abgelehnt",
                                _ => "beendet",
                            });
                        }
                    }
                }
                v
            })
            .collect::<Vec<_>>();
        let allianz=aid.and_then(|id|self.allianzen.iter().find(|a|a.id==id)).map(|a|{
            let rollen=["Leitung","Stellvertretung","Diplomatie","Quartiermeister"];
            let members=a.mitglieder.iter().enumerate().map(|(r,m)|json!({"name":self.spieler[*m as usize].name,"rang":r+1,"rolle":rollen.get(r).unwrap_or(&"Mitglied"),"planeten":self.spieler[*m as usize].planeten.iter().map(|p|self.planeten[*p as usize].koord).collect::<Vec<_>>()})).collect::<Vec<_>>();
            let angebote=self.kommunikation.angebote.iter().filter(|o|o.allianz==a.id).rev().take(100).map(|o|json!({"id":o.id,"von":self.spieler[o.von as usize].name,"planet":self.planeten[o.planet as usize].koord,"gut":o.gut,"menge":ganz(o.menge),"preis":o.preis as f64/M as f64,"status":o.status,"kaeufer":o.kaeufer.map(|s|&self.spieler[s as usize].name),"ankunft":o.ankunft})).collect::<Vec<_>>();
            let hilfe=self.kommunikation.hilfe.iter().filter(|h|h.allianz==a.id).rev().take(100).map(|h|json!({"id":h.id,"von":self.spieler[h.von as usize].name,"planet":h.planet,"text":h.text,"status":h.status,"helfer":h.helfer.iter().map(|s|&self.spieler[*s as usize].name).collect::<Vec<_>>()})).collect::<Vec<_>>();
            json!({"id":a.id,"name":a.name,"tag":self.allianz_tag(a.id),"leitung":self.allianz_leitung(sid,a.id),"gruender":self.allianz_rang(sid,a.id)==Some(0),"mitglieder":members,"angebote":angebote,"hilfe":hilfe})
        });
        json!({"version":1,"kontakte":self.spieler.iter().filter(|s|!self.aufklaerung.inaktive_spieler.contains(&s.id)).map(|s|self.beziehung(sid,s.id)).collect::<Vec<_>>(),"allianzanfragen":self.kommunikation.anfragen.iter().filter(|a|a.zielallianz.is_some()&&(a.allianz.is_some_and(|id|self.allianz_leitung(sid,id))||a.zielallianz.is_some_and(|id|self.allianz_leitung(sid,id)))).map(|a|{let mut v=json!(a);v["empfangsberechtigt"]=json!(a.zielallianz.is_some_and(|id|self.allianz_leitung(sid,id)));v}).collect::<Vec<_>>(),"briefkasten":self.briefkasten(sid).into_iter().take(100).collect::<Vec<_>>(),"anfragen":anfragen,"allianzbereich":allianz,"antworten_offen":self.offene_antworten(sid),
            "allianzen":self.allianz_verzeichnis(sid)})
    }
}

impl Welt {
    pub fn allianzvertrag_zwischen(&self, a: SpielerId, b: SpielerId, art: Vertragsart) -> bool {
        let (Some(x), Some(y)) = (
            self.spieler[a as usize].allianz,
            self.spieler[b as usize].allianz,
        ) else {
            return false;
        };
        self.kommunikation.anfragen.iter().any(|r| {
            r.art == art.name()
                && (r.status == "angenommen"
                    || (r.status == "gekuendigt"
                        && r.kuendigung_ende.is_some_and(|t| self.zeit < t)))
                && ((r.allianz == Some(x) && r.zielallianz == Some(y))
                    || (r.allianz == Some(y) && r.zielallianz == Some(x)))
        })
    }
    pub fn beziehung(&self, sid: SpielerId, ziel: SpielerId) -> Value {
        let sp = &self.spieler[ziel as usize];
        let (a, b) = (self.spieler[sid as usize].allianz, sp.allianz);
        let war = self.kommunikation.anfragen.iter().any(|r| {
            r.art == "krieg"
                && r.status == "erklaert"
                && if r.zielallianz.is_some() {
                    a.is_some()
                        && b.is_some()
                        && ((r.allianz == a && r.zielallianz == b)
                            || (r.allianz == b && r.zielallianz == a))
                } else {
                    (r.von == sid && r.an == ziel) || (r.an == sid && r.von == ziel)
                }
        });
        let (status, label, color) = if self.ist_besiegt(ziel) {
            ("ausgeschieden", "Ausgeschieden", Some("#9aa0a8"))
        } else if war {
            ("feindlich", "Feindlich", Some("#ef6b73"))
        } else if self.verbuendet(sid, ziel) {
            ("verbuendet", "Verbündet", Some("#63d89a"))
        } else if self.vertrag_zwischen(sid, ziel, Vertragsart::Nichtangriffspakt) {
            ("nap", "NAP", Some("#f1cf65"))
        } else if self.vertrag_zwischen(sid, ziel, Vertragsart::Handelsabkommen) {
            ("handelsabkommen", "Handelsabkommen", Some("#c296f5"))
        } else {
            ("neutral", "Neutral", None)
        };
        // A foreign alliance is red on the atlas without inventing a war declaration.
        let (karte_status, karte_farbe) = if sid == ziel { ("eigen", "#4cf58a") }
            else if status == "feindlich" { ("feindlich", "#ff6470") }
            else if status == "verbuendet" { ("verbuendet", "#48bd9b") }
            else if status == "nap" { ("nap", "#f3d365") }
            else if status == "neutral" && b.is_some() && b != a { ("fremde_allianz", "#ff6470") }
            else { ("neutral", "#ffffff") };
        json!({"spieler":sp.name,"allianz":sp.allianz.and_then(|id|self.allianzen.iter().find(|a|a.id==id)).map(|a|&a.name),
            "allianz_tag":sp.allianz.map(|id|self.allianz_tag(id)),"allianz_id":sp.allianz,
            "status":status,"bezeichnung":label,"farbe":color,"karte_status":karte_status,"karte_farbe":karte_farbe})
    }
    pub fn allianz_anfrage(
        &mut self,
        sid: SpielerId,
        ziel: &str,
        art: &str,
        text: &str,
    ) -> Result<String, String> {
        self.nachricht_pruefen(sid, text)?;
        let aid = self.allianz_id(sid)?;
        if !self.allianz_leitung(sid, aid) {
            return Err("Nur die obersten vier Mitglieder dürfen verhandeln".into());
        }
        let other = self
            .allianzen
            .iter()
            .find(|a| {
                a.name.eq_ignore_ascii_case(ziel.trim()) && !a.mitglieder.is_empty() && a.id != aid
            })
            .ok_or("Andere aktive Allianz angeben")?
            .clone();
        if ![
            "nichtangriffspakt",
            "verteidigungsbuendnis",
            "handelsabkommen",
            "krieg",
            "frieden",
        ]
        .contains(&art)
        {
            return Err("Unbekannte Vertragsart".into());
        }
        if self.kommunikation.anfragen.iter().any(|r| {
            r.art == art
                && (["offen", "angenommen", "erklaert"].contains(&r.status.as_str())
                    || (r.status == "gekuendigt"
                        && r.kuendigung_ende.is_some_and(|t| self.zeit < t)))
                && ((r.allianz == Some(aid) && r.zielallianz == Some(other.id))
                    || (r.allianz == Some(other.id) && r.zielallianz == Some(aid)))
        }) {
            return Err("Vereinbarung oder Angebot besteht bereits".into());
        }
        self.brief_senden(sid, "diplomatie", ziel, art, text, None)?;
        if art == "krieg" {
            self.allianzbruch(sid, other.mitglieder[0]);
        }
        let id = self.kommunikation.anfragen.len() as u64 + 1;
        self.kommunikation.anfragen.push(Anfrage {
            id,
            von: sid,
            an: other.mitglieder[0],
            art: art.into(),
            text: text.into(),
            status: if art == "krieg" { "erklaert" } else { "offen" }.into(),
            zeit: self.zeit,
            vertrag: None,
            allianz: Some(aid),
            zielallianz: Some(other.id),
            kuendigung_ende: None,
        });
        self.kommunikationsereignis(
            sid,
            "alliance.diplomacy.request",
            json!({"request_id":id,"alliance":aid,"target_alliance":other.id,"kind":art}),
        );
        Ok(format!("Allianzanfrage {id} übermittelt"))
    }
    pub fn allianz_pakt_kuendigen(&mut self, sid: SpielerId, id: u64) -> Result<String, String> {
        let r = self
            .kommunikation
            .anfragen
            .iter()
            .find(|r| r.id == id && r.zielallianz.is_some())
            .ok_or("Allianzpakt fehlt")?
            .clone();
        if !r.allianz.is_some_and(|a| self.allianz_leitung(sid, a))
            && !r.zielallianz.is_some_and(|a| self.allianz_leitung(sid, a))
        {
            return Err("Kein Leitungsrecht".into());
        }
        if r.status != "angenommen"
            || ![
                "nichtangriffspakt",
                "verteidigungsbuendnis",
                "handelsabkommen",
            ]
            .contains(&r.art.as_str())
        {
            return Err("Kein aktiver Pakt".into());
        }
        let end = self.zeit
            + if r.art == "handelsabkommen" {
                0
            } else {
                self.regeln.diplomatie.kuendigungsfrist_stunden * STUNDE
            };
        let a = &mut self.kommunikation.anfragen[id as usize - 1];
        a.status = "gekuendigt".into();
        a.kuendigung_ende = Some(end);
        self.kommunikationsereignis(
            sid,
            "alliance.diplomacy.cancelled",
            json!({"request_id":id,"alliance":r.allianz,"end":end}),
        );
        Ok(format!("Allianzpakt endet {}", zeittext(end)))
    }
    pub fn allianzbruch(&mut self, sid: SpielerId, ziel: SpielerId) {
        let (a, b) = (
            self.spieler[sid as usize].allianz,
            self.spieler[ziel as usize].allianz,
        );
        if a.is_none() || b.is_none() || a == b {
            return;
        }
        let ids = self
            .kommunikation
            .anfragen
            .iter()
            .filter(|r| {
                r.zielallianz.is_some()
                    && ["nichtangriffspakt", "verteidigungsbuendnis"].contains(&r.art.as_str())
                    && (r.status == "angenommen"
                        || (r.status == "gekuendigt"
                            && r.kuendigung_ende.is_some_and(|t| self.zeit < t)))
                    && ((r.allianz == a && r.zielallianz == b)
                        || (r.allianz == b && r.zielallianz == a))
            })
            .map(|r| r.id)
            .collect::<Vec<_>>();
        for id in ids {
            self.kommunikation.anfragen[id as usize - 1].status = "gebrochen".into();
            self.spieler[sid as usize].statistik.vertragsbrueche += 1;
            self.kommunikationsereignis(
                sid,
                "alliance.diplomacy.broken",
                json!({"request_id":id,"alliance":a,"target_alliance":b}),
            );
            self.register.push(Registereintrag {
                zeit: self.zeit,
                vertrag: 0,
                art: "allianzvertrag".into(),
                a: self.spieler[sid as usize].name.clone(),
                b: self.spieler[ziel as usize].name.clone(),
                vorgang: "Allianzschutzvertrag durch Angriff/Kriegserklärung gebrochen".into(),
            });
        }
    }
}

#[cfg(test)]
mod tick_tests {
    use super::*;
    #[test]
    fn batched_receipts_preserve_history_order_and_privacy() {
        let rules=crate::Regelwerk::laden(include_str!("../../../regeln/online-v1.ron")).unwrap();
        let mut w=Welt::neu(rules,712,8).unwrap();
        for (i,s) in w.spieler.iter_mut().enumerate(){s.ki=i%2==0;}
        w.brief_speichern(1,vec![6,2,4,3],"privat","Frage","Können wir handeln?",vec![],None,false);
        w.brief_speichern(3,vec![2,6],"privat","Antwort","Wir prüfen das.",vec![],Some(1),false);
        w.zeit=STUNDE;
        let mut reference=w.clone();
        let pending=(0..reference.spieler.len() as SpielerId).filter(|sid|reference.spieler[*sid as usize].ki&&reference.spieler_aktiv(*sid)).flat_map(|sid|reference.offene_antworten(sid).into_iter().map(move|id|(sid,id))).collect::<Vec<_>>();
        for(sid,id)in pending{reference.empfang_bei_zeitmangel(sid,id);}
        w.kommunikation_tick();assert_eq!(w.hash(),reference.hash());
        let hash=w.hash();w.kommunikation_tick();assert_eq!(hash,w.hash(),"No receipt ping-pong");
        assert!(!w.brief_sichtbar(0,1));
    }
}
