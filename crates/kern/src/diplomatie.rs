//! Nachrichten, Verträge mit Kaution, Allianzen, Tribute und das öffentliche Register.
//!
//! Verhandelt wird in freier Sprache, Verträge aber sind Objekte der Engine. Ein Bruch
//! ist jederzeit möglich, wird öffentlich und kostet die hinterlegte Kaution.

use crate::typen::*;
use crate::welt::*;

impl Welt {
    fn in_kraft(v: &Vertrag) -> bool {
        matches!(v.status, Vertragsstatus::Aktiv | Vertragsstatus::Gekuendigt { .. })
    }

    pub fn vertrag_zwischen(&self, a: SpielerId, b: SpielerId, art: Vertragsart) -> bool {
        self.allianzvertrag_zwischen(a,b,art) || self.vertraege
            .iter()
            .any(|v| v.art == art && Self::in_kraft(v) && ((v.a == a && v.b == b) || (v.a == b && v.b == a)))
    }

    /// Verteidigungsbündnis oder dieselbe Allianz.
    pub fn verbuendet(&self, a: SpielerId, b: SpielerId) -> bool {
        if a == b {
            return false;
        }
        let (aa, ab) = (self.spieler[a as usize].allianz, self.spieler[b as usize].allianz);
        (aa.is_some() && aa == ab) || self.vertrag_zwischen(a, b, Vertragsart::Verteidigungsbuendnis)
    }

    fn registrieren(&mut self, vertrag: u32, art: &str, a: SpielerId, b: SpielerId, vorgang: &str) {
        let eintrag = Registereintrag {
            zeit: self.zeit,
            vertrag,
            art: art.to_string(),
            a: self.spieler[a as usize].name.clone(),
            b: self.spieler[b as usize].name.clone(),
            vorgang: vorgang.to_string(),
        };
        self.register.push(eintrag);
    }

    pub fn nachricht(&mut self, sid: SpielerId, an: &[String], allianz: bool, text: &str) -> Result<String, String> {
        if allianz {
            if !an.is_empty() {return Err("Allianzchat erlaubt keine externen Empfänger".into());}
            return self.brief_senden(sid,"allianz","","Allianzchat",text,None);
        }
        // Validate the complete recipient set before delivering any copy.
        let mut ids=Vec::new();
        for name in an {let id=self.spieler_nach_name(name)?;if id!=sid&&!ids.contains(&id){ids.push(id);}}
        if ids.is_empty() {return Err("Kein Empfänger angegeben".into());}
        if ids.iter().any(|id|!self.spieler_aktiv(*id)) {return Err("Empfänger ist nicht aktiv".into());}
        if ids.len()+self.spieler[sid as usize].nachrichten_heute as usize>self.regeln.agenten.nachrichten_je_tag as usize {return Err("Nachrichten-Tageslimit erreicht".into());}
        for id in ids {let name=self.spieler[id as usize].name.clone();self.brief_senden(sid,"privat",&name,"Nachricht",text,None)?;}
        Ok("Nachrichten zugestellt".into())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn vertrag_anbieten(
        &mut self,
        sid: SpielerId,
        partner: &str,
        art: Vertragsart,
        kaution: i64,
        tribut_gut: Option<Gut>,
        tribut_menge: i64,
        tribut_tage: i64,
    ) -> Result<String, String> {
        let r = self.regeln.clone();
        let b = self.spieler_nach_name(partner)?;
        if b == sid {
            return Err("ein Vertrag braucht einen Partner".into());
        }
        if kaution < 0 {
            return Err("kaution darf nicht negativ sein".into());
        }
        let offen = self.vertraege.iter().any(|v| {
            v.art == art
                && (Self::in_kraft(v) || v.status == Vertragsstatus::Angeboten)
                && ((v.a == sid && v.b == b) || (v.a == b && v.b == sid))
        });
        if offen && art != Vertragsart::Tribut {
            return Err(format!("zwischen euch gibt es schon einen Vertrag {art} oder ein offenes Angebot"));
        }
        if art == Vertragsart::Verteidigungsbuendnis {
            for x in [sid, b] {
                let n = self.vertraege.iter().filter(|v| v.art == art && Self::in_kraft(v) && (v.a == x || v.b == x)).count();
                if n >= r.diplomatie.buendnisse_max {
                    return Err(format!(
                        "{} hat schon {} Verteidigungsbündnisse, mehr sind nicht erlaubt",
                        self.spieler[x as usize].name,
                        r.diplomatie.buendnisse_max
                    ));
                }
            }
        }
        if art == Vertragsart::Tribut && (tribut_menge <= 0 || !(1..=365).contains(&tribut_tage)) {
            return Err("ein Tribut braucht tribut_menge über 0 und tribut_tage zwischen 1 und 365".into());
        }
        if self.spieler[sid as usize].credits < kaution {
            return Err(format!("für die Kaution fehlen Credits ({} nötig)", ganz(kaution)));
        }
        self.spieler[sid as usize].credits -= kaution;
        let id = self.vertraege.len() as u32 + 1;
        self.vertraege.push(Vertrag {
            id,
            art,
            a: sid,
            b,
            angeboten: self.zeit,
            seit: 0,
            status: Vertragsstatus::Angeboten,
            kaution,
            tribut_gut,
            tribut_menge,
            tribut_bis: tribut_tage * TAG,
        });
        let von = self.spieler[sid as usize].name.clone();
        self.vorfall(b, "vertrag", format!("{von} bietet Vertrag {id} an: {art}"));
        self.wecke(b, Rolle::Diplomat, &format!("Vertragsangebot von {von}"), false);
        Ok(format!("Vertrag {id} ({art}) angeboten"))
    }

    fn vertrag_index(&self, id: u32) -> Result<usize, String> {
        self.vertraege.iter().position(|v| v.id == id).ok_or_else(|| format!("Vertrag {id} gibt es nicht"))
    }

    pub fn vertrag_annehmen(&mut self, sid: SpielerId, id: u32) -> Result<String, String> {
        let i = self.vertrag_index(id)?;
        let v = self.vertraege[i].clone();
        if v.b != sid || v.status != Vertragsstatus::Angeboten {
            return Err(format!("Vertrag {id} ist kein offenes Angebot an dich"));
        }
        if v.art == Vertragsart::Verteidigungsbuendnis {
            for spieler in [v.a, v.b] {
                let aktiv = self.vertraege.iter().filter(|t| t.art == v.art && Self::in_kraft(t) && (t.a == spieler || t.b == spieler)).count();
                if aktiv >= self.regeln.diplomatie.buendnisse_max {
                    return Err(format!("{} hat inzwischen das Bündnislimit erreicht", self.spieler[spieler as usize].name));
                }
            }
        }
        if self.spieler[sid as usize].credits < v.kaution {
            return Err(format!("für die Kaution fehlen Credits ({} nötig)", ganz(v.kaution)));
        }
        self.spieler[sid as usize].credits -= v.kaution;
        let jetzt = self.zeit;
        let vv = &mut self.vertraege[i];
        vv.status = Vertragsstatus::Aktiv;
        vv.seit = jetzt;
        vv.tribut_bis += jetzt;
        self.registrieren(id, v.art.name(), v.a, v.b, "geschlossen");
        let name = self.spieler[sid as usize].name.clone();
        self.vorfall(v.a, "vertrag", format!("{name} hat Vertrag {id} ({}) angenommen", v.art));
        self.wecke(v.a, Rolle::Diplomat, "Vertrag angenommen", false);
        Ok(format!("Vertrag {id} ({}) ist in Kraft", v.art))
    }

    pub fn vertrag_ablehnen(&mut self, sid: SpielerId, id: u32) -> Result<String, String> {
        let i = self.vertrag_index(id)?;
        let v = self.vertraege[i].clone();
        if (v.b != sid && v.a != sid) || v.status != Vertragsstatus::Angeboten {
            return Err(format!("Vertrag {id} ist kein offenes Angebot von dir oder an dich"));
        }
        self.vertraege[i].status = Vertragsstatus::Abgelehnt;
        self.spieler[v.a as usize].credits += v.kaution;
        if sid != v.a {
            let name = self.spieler[sid as usize].name.clone();
            self.vorfall(v.a, "vertrag", format!("{name} hat Vertrag {id} ({}) abgelehnt", v.art));
        }
        Ok(format!("Vertrag {id} abgelehnt"))
    }

    fn kautionen_zurueck(&mut self, i: usize) {
        let v = self.vertraege[i].clone();
        self.spieler[v.a as usize].credits += v.kaution;
        self.spieler[v.b as usize].credits += v.kaution;
        self.vertraege[i].kaution = 0;
    }

    fn brechen(&mut self, i: usize, brecher: SpielerId, grund: &str) {
        let v = self.vertraege[i].clone();
        let opfer = if v.a == brecher { v.b } else { v.a };
        self.vertraege[i].status = Vertragsstatus::Gebrochen;
        self.vertraege[i].kaution = 0;
        self.spieler[opfer as usize].credits += 2 * v.kaution;
        self.spieler[brecher as usize].statistik.vertragsbrueche += 1;
        let name = self.spieler[brecher as usize].name.clone();
        self.registrieren(v.id, v.art.name(), v.a, v.b, &format!("gebrochen durch {name}: {grund}"));
        self.vorfall(opfer, "vertrag", format!("{name} hat Vertrag {} ({}) gebrochen: {grund}", v.id, v.art));
        self.vorfall(brecher, "vertrag", format!("Vertrag {} ({}) gebrochen, die Kaution ist verfallen", v.id, v.art));
        self.wecke(opfer, Rolle::Diplomat, "Vertragsbruch", true);
        self.wecke(opfer, Rolle::Stratege, "Vertragsbruch", false);
    }

    pub fn vertrag_kuendigen(&mut self, sid: SpielerId, id: u32) -> Result<String, String> {
        let r = self.regeln.clone();
        let i = self.vertrag_index(id)?;
        let v = self.vertraege[i].clone();
        if (v.a != sid && v.b != sid) || v.status != Vertragsstatus::Aktiv {
            return Err(format!("Vertrag {id} ist kein laufender Vertrag von dir"));
        }
        let partner = if v.a == sid { v.b } else { v.a };
        let name = self.spieler[sid as usize].name.clone();
        match v.art {
            Vertragsart::Nichtangriffspakt | Vertragsart::Verteidigungsbuendnis => {
                let ende = self.zeit + r.diplomatie.kuendigungsfrist_stunden * STUNDE;
                self.vertraege[i].status = Vertragsstatus::Gekuendigt { ende };
                self.plane(ende, EreignisArt::VertragEnde { vertrag: id });
                self.registrieren(id, v.art.name(), v.a, v.b, &format!("gekündigt durch {name}, endet {}", zeittext(ende)));
                self.vorfall(partner, "vertrag", format!("{name} hat Vertrag {id} ({}) gekündigt, er endet {}", v.art, zeittext(ende)));
                self.wecke(partner, Rolle::Diplomat, "Vertrag gekündigt", false);
                Ok(format!("Vertrag {id} gekündigt, er gilt noch bis {}", zeittext(ende)))
            }
            Vertragsart::Tribut if sid == v.a => {
                // Der Zahler steigt vor Ablauf aus: das ist ein Bruch.
                self.brechen(i, sid, "Tribut eingestellt");
                Ok(format!("Tribut {id} eingestellt, das gilt als Vertragsbruch"))
            }
            _ => {
                self.vertraege[i].status = Vertragsstatus::Beendet;
                self.kautionen_zurueck(i);
                self.registrieren(id, v.art.name(), v.a, v.b, &format!("beendet durch {name}"));
                self.vorfall(partner, "vertrag", format!("{name} hat Vertrag {id} ({}) beendet", v.art));
                self.wecke(partner, Rolle::Diplomat, "Vertrag beendet", false);
                Ok(format!("Vertrag {id} beendet"))
            }
        }
    }

    pub fn vertrag_ende(&mut self, id: u32) {
        let Ok(i) = self.vertrag_index(id) else {
            return;
        };
        let v = self.vertraege[i].clone();
        if v.status == (Vertragsstatus::Gekuendigt { ende: self.zeit }) {
            self.vertraege[i].status = Vertragsstatus::Beendet;
            self.kautionen_zurueck(i);
            self.registrieren(id, v.art.name(), v.a, v.b, "beendet nach Kündigungsfrist");
        }
    }

    /// Ein Angriff auf einen Vertragspartner bricht Pakt und Bündnis und wird öffentlich.
    pub fn bruch_pruefen(&mut self, angreifer: SpielerId, opfer: SpielerId) {
        self.allianzbruch(angreifer,opfer);
        for i in 0..self.vertraege.len() {
            let v = &self.vertraege[i];
            let betrifft = (v.a == angreifer && v.b == opfer) || (v.a == opfer && v.b == angreifer);
            let schuetzt = matches!(v.art, Vertragsart::Nichtangriffspakt | Vertragsart::Verteidigungsbuendnis);
            if betrifft && schuetzt && Self::in_kraft(v) {
                self.brechen(i, angreifer, "Angriff auf den Partner");
            }
        }
        let aid = self.spieler[angreifer as usize].allianz;
        if aid.is_some() && aid == self.spieler[opfer as usize].allianz {
            self.registrieren(0, "allianz", angreifer, opfer, "Angriff auf ein Allianzmitglied, Ausschluss des Angreifers");
            self.spieler[angreifer as usize].statistik.vertragsbrueche += 1;
            self.allianz_entfernen(angreifer);
        }
    }

    pub fn tribute_zahlen(&mut self) {
        let jetzt = self.zeit;
        for i in 0..self.vertraege.len() {
            let v = self.vertraege[i].clone();
            if v.art != Vertragsart::Tribut || v.status != Vertragsstatus::Aktiv {
                continue;
            }
            if !self.spieler_aktiv(v.a) || !self.spieler_aktiv(v.b) {
                self.vertraege[i].status=Vertragsstatus::Beendet;
                self.kautionen_zurueck(i);
                continue;
            }
            if jetzt > v.tribut_bis {
                self.vertraege[i].status = Vertragsstatus::Beendet;
                self.kautionen_zurueck(i);
                self.registrieren(v.id, v.art.name(), v.a, v.b, "abgelaufen");
                continue;
            }
            let bezahlt = match v.tribut_gut {
                None => {
                    if self.spieler[v.a as usize].credits >= v.tribut_menge {
                        self.spieler[v.a as usize].credits -= v.tribut_menge;
                        self.spieler[v.b as usize].credits += v.tribut_menge;
                        true
                    } else {
                        false
                    }
                }
                Some(g) => {
                    let (von, an) = (self.spieler[v.a as usize].heimat as usize, self.spieler[v.b as usize].heimat as usize);
                    self.abrechnen(von);
                    if self.planeten[von].bestand[g.idx()] >= v.tribut_menge {
                        self.abrechnen(an);
                        self.planeten[von].bestand[g.idx()] -= v.tribut_menge;
                        self.planeten[an].bestand[g.idx()] += v.tribut_menge;
                        true
                    } else {
                        false
                    }
                }
            };
            if bezahlt {
                let wert = match v.tribut_gut {
                    None => v.tribut_menge,
                    Some(g) => {
                        let mut k = [0i64; GUETER];
                        k[g.idx()] = v.tribut_menge;
                        self.regeln.wert(&k)
                    }
                };
                self.spieler[v.a as usize].statistik.geschenkt += wert;
                self.spieler[v.b as usize].statistik.erhalten += wert;
            } else {
                self.brechen(i, v.a, "Tribut ausgefallen");
            }
        }
    }

    pub fn schenken(&mut self, sid: SpielerId, an: &str, credits: i64) -> Result<String, String> {
        let b = self.spieler_nach_name(an)?;
        if b == sid || credits <= 0 {
            return Err("schenken braucht einen anderen Spieler und einen Betrag über 0".into());
        }
        if self.spieler[sid as usize].credits < credits {
            return Err(format!("es fehlen Credits ({} vorhanden)", ganz(self.spieler[sid as usize].credits)));
        }
        self.spieler[sid as usize].credits -= credits;
        self.spieler[b as usize].credits += credits;
        self.spieler[sid as usize].statistik.geschenkt += credits;
        self.spieler[b as usize].statistik.erhalten += credits;
        let (von, name) = (self.spieler[sid as usize].name.clone(), self.spieler[b as usize].name.clone());
        self.vorfall(b, "geschenk", format!("{von} schenkt {} Credits", ganz(credits)));
        self.wecke(b, Rolle::Diplomat, "Geschenk erhalten", false);
        Ok(format!("{} Credits an {name} überwiesen", ganz(credits)))
    }

    // ---------------------------------------------------------------- Allianzen

    pub fn allianz_gruenden(&mut self, sid: SpielerId, name: &str) -> Result<String, String> {
        self.allianz_gruenden_mit_tag(sid, name, None)
    }

    /// Older alliances receive a stable, unique short code without changing their snapshot layout.
    pub fn allianz_tag(&self, id: u32) -> String {
        if let Some(tag) = self.kommunikation.allianz_tags.get(&id) { return tag.clone(); }
        let name = self.allianzen.iter().find(|a|a.id == id).map(|a|a.name.as_str()).unwrap_or("ALL");
        let suffix = id.to_string();
        let mut prefix: String = name.chars().filter(char::is_ascii_alphanumeric).take(6usize.saturating_sub(suffix.len()).min(4)).map(|c|c.to_ascii_uppercase()).collect();
        if prefix.is_empty() { prefix.push('A'); }
        format!("{prefix}{suffix}")
    }

    pub fn allianz_verzeichnis(&self, sid: SpielerId) -> Vec<serde_json::Value> {
        self.allianzen.iter().filter(|a|!a.mitglieder.is_empty()).map(|a| {
            let voelker: std::collections::BTreeSet<_> = a.mitglieder.iter().map(|m|self.spieler[*m as usize].volk.name()).collect();
            serde_json::json!({"id":a.id,"tag":self.allianz_tag(a.id),"name":a.name,"anzahl":a.mitglieder.len(),
                "voelker":voelker,"leitung":self.spieler[a.mitglieder[0] as usize].name,"beziehung":self.beziehung(sid,a.mitglieder[0])})
        }).collect()
    }

    pub fn allianz_gruenden_mit_tag(&mut self, sid: SpielerId, name: &str, tag: Option<&str>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 30 {
            return Err("der Allianzname braucht 1 bis 30 Zeichen".into());
        }
        if self.spieler[sid as usize].allianz.is_some() {
            return Err("du bist schon in einer Allianz".into());
        }
        if self.allianzen.iter().any(|a| a.name.eq_ignore_ascii_case(name) && !a.mitglieder.is_empty()) {
            return Err(format!("eine Allianz '{name}' gibt es schon"));
        }
        let tag = tag.map(|t|t.trim().to_ascii_uppercase()).filter(|t|!t.is_empty());
        if let Some(tag) = &tag {
            if !(2..=6).contains(&tag.len()) || !tag.bytes().all(|c|c.is_ascii_uppercase() || c.is_ascii_digit()) {
                return Err("Das Allianz-Kürzel braucht 2 bis 6 Buchstaben oder Ziffern (A–Z, 0–9)".into());
            }
            if self.allianzen.iter().any(|a|self.allianz_tag(a.id) == *tag) {
                return Err(format!("Das Allianz-Kürzel [{tag}] ist bereits vergeben"));
            }
        }
        let id = self.allianzen.len() as u32 + 1;
        let tag = tag.unwrap_or_else(|| {
            let suffix = id.to_string();
            let prefix: String = name.chars().filter(char::is_ascii_alphanumeric).take(6usize.saturating_sub(suffix.len()).min(4)).map(|c|c.to_ascii_uppercase()).collect();
            let mut candidate = format!("{}{suffix}",if prefix.is_empty(){"A"}else{&prefix});
            let mut alternative = id;
            while self.allianzen.iter().any(|a|self.allianz_tag(a.id) == candidate) {
                alternative += 1; candidate = format!("A{alternative}");
            }
            candidate
        });
        self.allianzen.push(Allianz { id, name: name.to_string(), mitglieder: vec![sid], eingeladen: Vec::new() });
        self.kommunikation.allianz_tags.insert(id, tag);
        self.spieler[sid as usize].allianz = Some(id);
        self.kommunikationsereignis(sid,"alliance.membership",serde_json::json!({"alliance":id,"joined":true}));
        self.registrieren(0, "allianz", sid, sid, &format!("Allianz '{name}' gegründet"));
        Ok(format!("Allianz '{name}' gegründet"))
    }

    pub fn allianz_einladen(&mut self, sid: SpielerId, spieler: &str) -> Result<String, String> {
        let max = self.regeln.diplomatie.allianz_max;
        let b = self.spieler_nach_name(spieler)?;
        let Some(aid) = self.spieler[sid as usize].allianz else {
            return Err("du bist in keiner Allianz".into());
        };
        if !self.allianz_leitung(sid,aid) {return Err("Nur die obersten vier Mitglieder dürfen einladen".into());}
        if self.spieler[b as usize].allianz.is_some() {
            return Err(format!("{} ist schon in einer Allianz", self.spieler[b as usize].name));
        }
        let al = self.allianzen.iter_mut().find(|a| a.id == aid).ok_or("Allianz nicht gefunden")?;
        if al.mitglieder.len() + al.eingeladen.len() >= max {
            return Err(format!("eine Allianz hat höchstens {max} Mitglieder"));
        }
        if !al.eingeladen.contains(&b) {
            al.eingeladen.push(b);
        }
        let aname = al.name.clone();
        let von = self.spieler[sid as usize].name.clone();
        self.vorfall(b, "allianz", format!("{von} lädt dich in die Allianz '{aname}' ein"));
        self.wecke(b, Rolle::Diplomat, "Einladung in eine Allianz", false);
        Ok(format!("{} in die Allianz '{aname}' eingeladen", self.spieler[b as usize].name))
    }

    pub fn allianz_beitreten(&mut self, sid: SpielerId, name: &str) -> Result<String, String> {
        let max = self.regeln.diplomatie.allianz_max;
        if self.spieler[sid as usize].allianz.is_some() {
            return Err("du bist schon in einer Allianz".into());
        }
        let n = name.trim();
        let al = self
            .allianzen
            .iter_mut()
            .find(|a| a.name.eq_ignore_ascii_case(n) && !a.mitglieder.is_empty())
            .ok_or_else(|| format!("eine Allianz '{n}' gibt es nicht"))?;
        if !al.eingeladen.contains(&sid) {
            return Err(format!("für die Allianz '{n}' liegt keine Einladung vor"));
        }
        if al.mitglieder.len() >= max {
            return Err(format!("die Allianz '{n}' ist voll"));
        }
        al.eingeladen.retain(|x| *x != sid);
        al.mitglieder.push(sid);
        let (id, aname, erster) = (al.id, al.name.clone(), al.mitglieder[0]);
        self.spieler[sid as usize].allianz = Some(id);
        self.kommunikationsereignis(sid,"alliance.membership",serde_json::json!({"alliance":id,"joined":true}));
        self.registrieren(0, "allianz", sid, erster, &format!("Beitritt zur Allianz '{aname}'"));
        Ok(format!("der Allianz '{aname}' beigetreten"))
    }

    fn allianz_entfernen(&mut self, sid: SpielerId) {
        if let Some(aid) = self.spieler[sid as usize].allianz.take() {
            if let Some(al) = self.allianzen.iter_mut().find(|a| a.id == aid) {
                al.mitglieder.retain(|x| *x != sid);
                if al.mitglieder.is_empty() {al.eingeladen.clear();}
            }
            self.kommunikationsereignis(sid,"alliance.membership",serde_json::json!({"alliance":aid,"joined":false}));
            self.kommunikation_tick();
        }
    }

    pub fn allianz_verlassen(&mut self, sid: SpielerId) -> Result<String, String> {
        let Some(aid) = self.spieler[sid as usize].allianz else {
            return Err("du bist in keiner Allianz".into());
        };
        let aname = self.allianzen.iter().find(|a| a.id == aid).map(|a| a.name.clone()).unwrap_or_default();
        self.allianz_entfernen(sid);
        self.registrieren(0, "allianz", sid, sid, &format!("Austritt aus der Allianz '{aname}'"));
        Ok(format!("Allianz '{aname}' verlassen"))
    }
}
