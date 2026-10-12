//! Aktionen der Agenten: festes Schema, Rollenrechte, genaue Ablehnungsgründe.

use crate::flotte::Flugauftrag;
use crate::typen::*;
use crate::welt::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn eins() -> i64 {
    1
}
fn voll() -> f64 {
    1.0
}

fn menge_geprueft(wert: f64, feld: &str) -> Result<i64, String> {
    let skaliert = wert * M as f64;
    if !wert.is_finite() || wert < 0.0 || !skaliert.is_finite() || skaliert >= i64::MAX as f64 {
        return Err(format!(
            "{feld} muss eine nichtnegative, darstellbare Menge sein"
        ));
    }
    Ok(skaliert.round() as i64)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "typ", rename_all = "snake_case")]
pub enum Aktion {
    Bauen {
        planet: Koord,
        gebaeude: Gebaeude,
    },
    Reparieren {
        planet: Koord,
        gebaeude: Gebaeude,
    },
    Abreissen {
        planet: Koord,
        gebaeude: Gebaeude,
    },
    SchleifeLeeren {
        planet: Koord,
    },
    Forschen {
        forschung: Forschung,
        #[serde(default)]
        planet: Option<Koord>,
    },
    Fertigen {
        planet: Koord,
        #[serde(default)]
        einheit: Option<Einheit>,
        #[serde(default)]
        bauteil: Option<Gut>,
        #[serde(default = "eins")]
        anzahl: i64,
    },
    Steuersatz {
        prozent: u8,
    },
    Prioritaeten {
        planet: Koord,
        reihenfolge: Vec<Gebaeude>,
    },
    Stufenaufstieg {},
    FlotteSenden {
        start: Koord,
        ziel: Koord,
        mission: Mission,
        schiffe: BTreeMap<Einheit, i64>,
        #[serde(default = "voll")]
        geschwindigkeit: f64,
        #[serde(default)]
        ladung: BTreeMap<Gut, f64>,
        #[serde(default)]
        haltedauer_stunden: i64,
    },
    FlotteZurueckrufen {
        flotte: u32,
    },
    FlotteAusspaehen {
        start: Koord,
        flotte: u32,
        sonden: i64,
        #[serde(default = "voll")]
        geschwindigkeit: f64,
    },
    VerbandOeffnen {
        flotte: u32,
    },
    VerbandBeitreten {
        flotte: u32,
        fuehrung: u32,
    },
    RaketenBauen {
        planet: Koord,
        art: String,
        anzahl: i64,
    },
    RaketenStarten {
        start: Koord,
        ziel: Koord,
        anzahl: i64,
        zieltyp: Einheit,
    },
    BriefSenden { kanal: String, an: String, betreff: String, text: String, #[serde(default)] antwort_auf: Option<u64> },
    BriefLesen { brief: u64 },
    AllianzAnfrage { allianz:String, art:String, text:String },
    AllianzPaktKuendigen { anfrage:u64 },
    DiplomatieAnfrage { partner: String, art: String, text: String },
    DiplomatieEntscheiden { anfrage: u64, annehmen: bool },
    AllianzRolle { spieler: String, rang: u8 },
    AllianzAusschliessen { spieler: String },
    AllianzHilfe { planet: Koord, text: String },
    AllianzHilfeStatus { hilfe: u64, erledigt: bool },
    InternAnbieten { planet: Koord, gut: Gut, menge: f64, preis: f64 },
    InternKaufen { angebot: u64, planet: Koord },
    InternStorno { angebot: u64 },
    Nachricht {
        #[serde(default)]
        an: Vec<String>,
        #[serde(default)]
        allianz: bool,
        text: String,
    },
    VertragAnbieten {
        partner: String,
        art: Vertragsart,
        #[serde(default)]
        kaution: f64,
        #[serde(default)]
        tribut_gut: Option<Gut>,
        #[serde(default)]
        tribut_menge: f64,
        #[serde(default)]
        tribut_tage: i64,
    },
    VertragAnnehmen {
        vertrag: u32,
    },
    VertragAblehnen {
        vertrag: u32,
    },
    VertragKuendigen {
        vertrag: u32,
    },
    AllianzGruenden {
        name: String,
        #[serde(default)]
        tag: Option<String>,
    },
    AllianzEinladen {
        spieler: String,
    },
    AllianzBeitreten {
        allianz: String,
    },
    AllianzVerlassen {},
    MarktOrder {
        planet: Koord,
        gut: Gut,
        seite: Marktseite,
        menge: f64,
        preis: f64,
    },
    MarktStorno {
        order: u32,
    },
    Doktrin {
        #[serde(default)]
        anteile: BTreeMap<Topf, u8>,
        text: String,
    },
    Meldung {
        text: String,
    },
    Schenken {
        an: String,
        credits: f64,
    },
    FlotteVersorgen {
        start: Koord,
        ziel: Koord,
        versorgungsflotte: FlottenId,
        schiffe: BTreeMap<Einheit, i64>,
        #[serde(default = "voll")]
        geschwindigkeit: f64,
        #[serde(default)]
        ladung: BTreeMap<Gut, f64>,
    },
}

impl Aktion {
    pub fn typ(&self) -> &'static str {
        match self {
            Aktion::Bauen { .. } => "bauen",
            Aktion::Reparieren { .. } => "reparieren",
            Aktion::Abreissen { .. } => "abreissen",
            Aktion::SchleifeLeeren { .. } => "schleife_leeren",
            Aktion::Forschen { .. } => "forschen",
            Aktion::Fertigen { .. } => "fertigen",
            Aktion::Steuersatz { .. } => "steuersatz",
            Aktion::Prioritaeten { .. } => "prioritaeten",
            Aktion::Stufenaufstieg {} => "stufenaufstieg",
            Aktion::FlotteSenden { .. } => "flotte_senden",
            Aktion::FlotteVersorgen { .. } => "flotte_versorgen",
            Aktion::FlotteZurueckrufen { .. } => "flotte_zurueckrufen",
            Aktion::FlotteAusspaehen { .. } => "flotte_ausspaehen",
            Aktion::VerbandOeffnen { .. } => "verband_oeffnen",
            Aktion::VerbandBeitreten { .. } => "verband_beitreten",
            Aktion::RaketenBauen { .. } => "raketen_bauen",
            Aktion::RaketenStarten { .. } => "raketen_starten",
            Aktion::BriefSenden { .. } => "brief_senden",
            Aktion::BriefLesen { .. } => "brief_lesen",
            Aktion::AllianzAnfrage { .. } => "allianz_anfrage",
            Aktion::AllianzPaktKuendigen { .. } => "allianz_pakt_kuendigen",
            Aktion::DiplomatieAnfrage { .. } => "diplomatie_anfrage",
            Aktion::DiplomatieEntscheiden { .. } => "diplomatie_entscheiden",
            Aktion::AllianzRolle { .. } => "allianz_rolle",
            Aktion::AllianzAusschliessen { .. } => "allianz_ausschliessen",
            Aktion::AllianzHilfe { .. } => "allianz_hilfe",
            Aktion::AllianzHilfeStatus { .. } => "allianz_hilfe_status",
            Aktion::InternAnbieten { .. } => "intern_anbieten",
            Aktion::InternKaufen { .. } => "intern_kaufen",
            Aktion::InternStorno { .. } => "intern_storno",
            Aktion::Nachricht { .. } => "nachricht",
            Aktion::VertragAnbieten { .. } => "vertrag_anbieten",
            Aktion::VertragAnnehmen { .. } => "vertrag_annehmen",
            Aktion::VertragAblehnen { .. } => "vertrag_ablehnen",
            Aktion::VertragKuendigen { .. } => "vertrag_kuendigen",
            Aktion::AllianzGruenden { .. } => "allianz_gruenden",
            Aktion::AllianzEinladen { .. } => "allianz_einladen",
            Aktion::AllianzBeitreten { .. } => "allianz_beitreten",
            Aktion::AllianzVerlassen {} => "allianz_verlassen",
            Aktion::MarktOrder { .. } => "markt_order",
            Aktion::MarktStorno { .. } => "markt_storno",
            Aktion::Doktrin { .. } => "doktrin",
            Aktion::Meldung { .. } => "meldung",
            Aktion::Schenken { .. } => "schenken",
        }
    }

    /// Rollen, die diese Aktion ausführen dürfen.
    pub fn zustaendig(&self) -> &'static [Rolle] {
        use Rolle::*;
        match self {
            Aktion::Doktrin { .. } => &[Stratege],
            Aktion::Stufenaufstieg {} => &[Stratege, Verwalter],
            Aktion::Meldung { .. } => &[Stratege, Verwalter, Feldherr, Diplomat],
            Aktion::Bauen { .. }
            | Aktion::Reparieren { .. }
            | Aktion::Abreissen { .. }
            | Aktion::SchleifeLeeren { .. }
            | Aktion::Forschen { .. }
            | Aktion::Steuersatz { .. }
            | Aktion::Prioritaeten { .. }
            | Aktion::MarktOrder { .. }
            | Aktion::MarktStorno { .. } => &[Verwalter],
            Aktion::Fertigen { .. }
            | Aktion::FlotteZurueckrufen { .. }
            | Aktion::RaketenBauen { .. }
            | Aktion::FlotteVersorgen { .. } => &[Verwalter, Feldherr],
            Aktion::RaketenStarten { .. }
            | Aktion::FlotteAusspaehen { .. }
            | Aktion::VerbandOeffnen { .. }
            | Aktion::VerbandBeitreten { .. } => &[Feldherr],
            Aktion::FlotteSenden { mission, .. } => match mission {
                Mission::Transport
                | Mission::Stationieren
                | Mission::Kolonisieren
                | Mission::Abbau
                | Mission::Recyceln => &[Verwalter, Feldherr],
                Mission::Saven => &[Verwalter, Feldherr],
                _ => &[Feldherr],
            },
            Aktion::InternAnbieten {..}|Aktion::InternKaufen {..}|Aktion::InternStorno {..} => &[Diplomat,Verwalter],
            Aktion::AllianzHilfe {..}|Aktion::AllianzHilfeStatus {..} => &[Diplomat,Feldherr],
            _ => &[Diplomat],
        }
    }
}

/// Aktionstypen, die eine Rolle ausführen darf, für Prompt und Antwortschema.
pub fn erlaubte_typen(rolle: Rolle) -> Vec<&'static str> {
    let alle = [
        ("doktrin", &[Rolle::Stratege][..]),
        ("stufenaufstieg", &[Rolle::Stratege, Rolle::Verwalter][..]),
        (
            "meldung",
            &[
                Rolle::Stratege,
                Rolle::Verwalter,
                Rolle::Feldherr,
                Rolle::Diplomat,
            ][..],
        ),
        ("bauen", &[Rolle::Verwalter][..]),
        ("reparieren", &[Rolle::Verwalter][..]),
        ("abreissen", &[Rolle::Verwalter][..]),
        ("schleife_leeren", &[Rolle::Verwalter][..]),
        ("forschen", &[Rolle::Verwalter][..]),
        ("steuersatz", &[Rolle::Verwalter][..]),
        ("prioritaeten", &[Rolle::Verwalter][..]),
        ("markt_order", &[Rolle::Verwalter][..]),
        ("markt_storno", &[Rolle::Verwalter][..]),
        ("fertigen", &[Rolle::Verwalter, Rolle::Feldherr][..]),
        ("flotte_senden", &[Rolle::Verwalter, Rolle::Feldherr][..]),
        ("flotte_versorgen", &[Rolle::Verwalter, Rolle::Feldherr][..]),
        (
            "flotte_zurueckrufen",
            &[Rolle::Verwalter, Rolle::Feldherr][..],
        ),
        ("verband_oeffnen", &[Rolle::Feldherr][..]),
        ("flotte_ausspaehen", &[Rolle::Feldherr][..]),
        ("verband_beitreten", &[Rolle::Feldherr][..]),
        ("raketen_bauen", &[Rolle::Verwalter, Rolle::Feldherr][..]),
        ("raketen_starten", &[Rolle::Feldherr][..]),
        ("brief_senden", &[Rolle::Diplomat][..]),
        ("brief_lesen", &[Rolle::Diplomat][..]),
        ("allianz_anfrage", &[Rolle::Diplomat][..]),
        ("allianz_pakt_kuendigen", &[Rolle::Diplomat][..]),
        ("diplomatie_anfrage", &[Rolle::Diplomat][..]),
        ("diplomatie_entscheiden", &[Rolle::Diplomat][..]),
        ("allianz_rolle", &[Rolle::Diplomat][..]),
        ("allianz_ausschliessen", &[Rolle::Diplomat][..]),
        ("allianz_hilfe", &[Rolle::Diplomat, Rolle::Feldherr][..]),
        ("allianz_hilfe_status", &[Rolle::Diplomat, Rolle::Feldherr][..]),
        ("intern_anbieten", &[Rolle::Diplomat, Rolle::Verwalter][..]),
        ("intern_kaufen", &[Rolle::Diplomat, Rolle::Verwalter][..]),
        ("intern_storno", &[Rolle::Diplomat, Rolle::Verwalter][..]),
        ("nachricht", &[Rolle::Diplomat][..]),
        ("vertrag_anbieten", &[Rolle::Diplomat][..]),
        ("vertrag_annehmen", &[Rolle::Diplomat][..]),
        ("vertrag_ablehnen", &[Rolle::Diplomat][..]),
        ("vertrag_kuendigen", &[Rolle::Diplomat][..]),
        ("allianz_gruenden", &[Rolle::Diplomat][..]),
        ("allianz_einladen", &[Rolle::Diplomat][..]),
        ("allianz_beitreten", &[Rolle::Diplomat][..]),
        ("allianz_verlassen", &[Rolle::Diplomat][..]),
        ("schenken", &[Rolle::Diplomat][..]),
    ];
    alle.iter()
        .filter(|(_, r)| rolle == Rolle::Alle || r.contains(&rolle))
        .map(|(t, _)| *t)
        .collect()
}

impl Welt {
    fn protokolliere(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        aktion: String,
        ok: bool,
        text: &str,
    ) {
        let eintrag = Logeintrag {
            zeit: self.zeit,
            spieler: sid,
            rolle,
            aktion,
            ok,
            text: text.to_string(),
        };
        let mut h = Sha256::new();
        h.update(self.log_hash.as_bytes());
        h.update(
            format!(
                "{}|{}|{}|{}|{}",
                eintrag.zeit, sid, rolle, eintrag.aktion, ok
            )
            .as_bytes(),
        );
        self.log_hash = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        self.log_anzahl += 1;
        self.logpuffer.push(eintrag);
    }

    /// Wendet eine Aktion als JSON an. Liefert, ob sie angenommen wurde, und den Text dazu.
    pub fn handeln(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        aktion: &serde_json::Value,
    ) -> (bool, String) {
        if sid as usize >= self.spieler.len() {
            return (false, "Spieler existiert nicht".into());
        }
        if self.ist_besiegt(sid) { return (false, "Reich besiegt; bis zur nächsten Epoche nur noch zuschauen".into()); }
        if !self.spieler_aktiv(sid) { return (false, "Startplatz noch nicht belegt".into()); }
        if self.beendet() {
            return (
                false,
                "Die Epoche ist beendet; keine weiteren Spielaktionen".into(),
            );
        }
        let roh = aktion.to_string();
        let ergebnis = match serde_json::from_value::<Aktion>(aktion.clone()) {
            Err(e) => Err(format!("Aktion nicht lesbar: {e}")),
            Ok(a) => {
                let hidden_fleet = if self.aufklaerungsregeln() {
                    match &a {
                        Aktion::FlotteZurueckrufen{flotte}|Aktion::VerbandOeffnen{flotte}|
                        Aktion::FlotteVersorgen{versorgungsflotte:flotte,..}=>!self.flotten.get(flotte).is_some_and(|f|f.besitzer==sid),
                        Aktion::VerbandBeitreten{flotte,fuehrung}=>!self.flotten.get(flotte).is_some_and(|f|f.besitzer==sid) || !self.flotten.get(fuehrung).is_some_and(|f|f.verband==Some(f.id) && (f.besitzer==sid || self.verbuendet(sid,f.besitzer))),
                        _=>false,
                    }
                } else {false};
                if hidden_fleet {Err("Keine berechtigte Flotte unter dieser Kennung".into())}
                else if rolle != Rolle::Alle && !a.zustaendig().contains(&rolle) {
                    let wer: Vec<&str> = a.zustaendig().iter().map(|r| r.name()).collect();
                    Err(format!(
                        "{} gehört nicht zur Rolle {rolle}, zuständig: {}",
                        a.typ(),
                        wer.join(", ")
                    ))
                } else {
                    self.anwenden(sid, rolle, a)
                }
            }
        };
        let st = &mut self.spieler[sid as usize].statistik;
        st.aktionen += 1;
        let (ok, text) = match ergebnis {
            Ok(t) => (true, t),
            Err(t) => {
                st.abgelehnt += 1;
                (false, t)
            }
        };
        self.protokolliere(sid, rolle, roh, ok, &text);
        (ok, text)
    }

    /// Schließt den Aufruf einer Rolle ab: Notizbuch und Wecker setzen, Auslöser löschen.
    pub fn aufruf_ende(
        &mut self,
        sid: SpielerId,
        rolle: Rolle,
        notiz: Option<&str>,
        wecker_sekunden: Option<i64>,
        hinweise: &[String],
    ) {
        if rolle == Rolle::Alle {
            return;
        }
        // Endgültig abgelehnte Aktionen stehen mit ihrem Grund im nächsten Lagebild.
        for h in hinweise.iter().take(10) {
            let kurz: String = h.chars().take(300).collect();
            self.vorfall(sid, "abgelehnt", format!("{rolle}: {kurz}"));
        }
        let jetzt = self.zeit;
        let max = self.regeln.agenten.notiz_zeichen;
        // Ein früherer Wecker wird auf den frühesten erlaubten Aufruf verschoben (Regel agenten.frueh_anteil).
        let frueh = self
            .regeln
            .agenten
            .frueheste_sekunden(rolle)
            .max(15 * MINUTE);
        let sp = &mut self.spieler[sid as usize];
        let w = &mut sp.weck[rolle.idx()];
        w.letzter = jetzt;
        w.ausloeser.clear();
        w.dringend = false;
        w.wecker = wecker_sekunden.map(|s| jetzt + s.clamp(frueh, 30 * TAG));
        if let Some(n) = notiz {
            sp.notizen[rolle.idx()] = n.chars().take(max).collect();
        }
        let roh = serde_json::json!({"typ": "aufruf_ende", "notiz": notiz, "wecker_sekunden": wecker_sekunden, "hinweise": hinweise}).to_string();
        self.protokolliere(sid, rolle, roh, true, "");
    }

    fn anwenden(&mut self, sid: SpielerId, rolle: Rolle, a: Aktion) -> Result<String, String> {
        match a {
            Aktion::Bauen { planet, gebaeude } => self.bauen(sid, rolle, planet, gebaeude),
            Aktion::Reparieren { planet, gebaeude } => {
                self.reparieren(sid, rolle, planet, gebaeude)
            }
            Aktion::Abreissen { planet, gebaeude } => self.abreissen(sid, planet, gebaeude),
            Aktion::SchleifeLeeren { planet } => self.schleife_leeren(sid, planet),
            Aktion::Forschen { forschung, planet } => {
                if self.spieler[sid as usize].forschung[forschung.idx()]
                    >= crate::regeln::MAX_FORSCHUNG
                {
                    return Err(format!("{forschung} hat die höchste Stufe erreicht"));
                }
                self.forschen(sid, rolle, forschung, planet)
            }
            Aktion::Fertigen {
                planet,
                einheit,
                bauteil,
                anzahl,
            } => {
                let produkt = match (einheit, bauteil) {
                    (Some(e), None) => Produkt::Einheit(e),
                    (None, Some(g)) => Produkt::Bauteil(g),
                    _ => return Err("fertigen braucht genau eines von einheit oder bauteil".into()),
                };
                self.fertigen(sid, rolle, planet, produkt, anzahl)
            }
            Aktion::Steuersatz { prozent } => {
                if prozent > 50 {
                    return Err("der Steuersatz liegt zwischen 0 und 50 Prozent".into());
                }
                self.spieler[sid as usize].steuersatz = prozent;
                Ok(format!("Steuersatz auf {prozent} Prozent gesetzt"))
            }
            Aktion::Prioritaeten {
                planet,
                reihenfolge,
            } => {
                let pid = self.eigener_planet(sid, planet)?;
                let mut v: Vec<Gebaeude> = Vec::new();
                for g in reihenfolge {
                    if !v.contains(&g) {
                        v.push(g);
                    }
                }
                self.planeten[pid].prioritaeten = v;
                self.raten_neu(pid);
                Ok(format!("Arbeitsprioritäten auf {planet} gesetzt"))
            }
            Aktion::Stufenaufstieg {} => self.stufenaufstieg(sid),
            Aktion::FlotteSenden {
                start,
                ziel,
                mission,
                schiffe,
                geschwindigkeit,
                ladung,
                haltedauer_stunden,
            } => {
                let mut s = [0i64; SCHIFFE];
                for (e, n) in &schiffe {
                    if !e.ist_schiff() {
                        return Err(format!("{e} ist kein Schiff"));
                    }
                    s[e.idx()] = *n;
                }
                let mut l = [0i64; GUETER];
                for (g, m) in &ladung {
                    l[g.idx()] = menge_geprueft(*m, "ladung")?;
                }
                if !geschwindigkeit.is_finite() || !(0.1..=1.0).contains(&geschwindigkeit) {
                    return Err("geschwindigkeit muss zwischen 0.1 und 1.0 liegen".into());
                }
                self.flotte_senden(
                    sid,
                    rolle,
                    Flugauftrag {
                        start,
                        ziel,
                        mission,
                        schiffe: s,
                        sigma_pm: milli(geschwindigkeit),
                        ladung: l,
                        haltedauer: haltedauer_stunden.clamp(0, 10_000) * STUNDE,
                    },
                )
            }
            Aktion::FlotteZurueckrufen { flotte } => self.flotte_zurueckrufen(sid, flotte),
            Aktion::FlotteAusspaehen { start, flotte, sonden, geschwindigkeit } => {
                if !geschwindigkeit.is_finite() || !(0.1..=1.0).contains(&geschwindigkeit) {
                    return Err("geschwindigkeit muss zwischen 0.1 und 1.0 liegen".into());
                }
                self.flotte_ausspaehen(sid, rolle, start, flotte, sonden, milli(geschwindigkeit))
            }
            Aktion::FlotteVersorgen {
                start,
                ziel,
                versorgungsflotte,
                schiffe,
                geschwindigkeit,
                ladung,
            } => {
                let mut s = [0i64; SCHIFFE];
                for (e, n) in schiffe {
                    if !e.ist_schiff() {
                        return Err(format!("{e} ist kein Schiff"));
                    }
                    s[e.idx()] = n;
                }
                let mut l = [0i64; GUETER];
                for (g, m) in ladung {
                    l[g.idx()] = menge_geprueft(m, "ladung")?;
                }
                if !geschwindigkeit.is_finite() || !(0.1..=1.0).contains(&geschwindigkeit) {
                    return Err("geschwindigkeit muss zwischen 0.1 und 1.0 liegen".into());
                }
                self.flotte_versorgen(
                    sid,
                    rolle,
                    Flugauftrag {
                        start,
                        ziel,
                        mission: Mission::Transport,
                        schiffe: s,
                        sigma_pm: milli(geschwindigkeit),
                        ladung: l,
                        haltedauer: 0,
                    },
                    versorgungsflotte,
                )
            }
            Aktion::VerbandOeffnen { flotte } => self.verband_oeffnen(sid, flotte),
            Aktion::VerbandBeitreten { flotte, fuehrung } => {
                self.verband_beitreten(sid, flotte, fuehrung)
            }
            Aktion::RaketenBauen {
                planet,
                art,
                anzahl,
            } => self.raketen_bauen(sid, rolle, planet, &art, anzahl),
            Aktion::RaketenStarten {
                start,
                ziel,
                anzahl,
                zieltyp,
            } => self.raketen_starten(sid, start, ziel, anzahl, zieltyp),
            Aktion::BriefSenden {kanal,an,betreff,text,antwort_auf} => self.brief_senden(sid,&kanal,&an,&betreff,&text,antwort_auf),
            Aktion::AllianzAnfrage {allianz,art,text} => self.allianz_anfrage(sid,&allianz,&art,&text),
            Aktion::AllianzPaktKuendigen {anfrage} => self.allianz_pakt_kuendigen(sid,anfrage),
            Aktion::BriefLesen {brief} => self.brief_lesen(sid,brief),
            Aktion::DiplomatieAnfrage {partner,art,text} => self.diplomatie_anfrage(sid,&partner,&art,&text),
            Aktion::DiplomatieEntscheiden {anfrage,annehmen} => self.diplomatie_entscheiden(sid,anfrage,annehmen),
            Aktion::AllianzRolle {spieler,rang} => self.allianz_rolle(sid,&spieler,rang),
            Aktion::AllianzAusschliessen {spieler} => self.allianz_ausschliessen(sid,&spieler),
            Aktion::AllianzHilfe {planet,text} => self.allianz_hilfe(sid,planet,&text),
            Aktion::AllianzHilfeStatus {hilfe,erledigt} => self.allianz_hilfe_status(sid,hilfe,erledigt),
            Aktion::InternAnbieten {planet,gut,menge,preis} => self.intern_anbieten(sid,planet,gut,menge_geprueft(menge,"menge")?,menge_geprueft(preis,"preis")?),
            Aktion::InternKaufen {angebot,planet} => {
                let topf=Self::topf_fuer(rolle,Topf::Wirtschaft);
                let o=self.kommunikation.angebote.iter().find(|o|o.id==angebot&&Some(o.allianz)==self.spieler[sid as usize].allianz).ok_or("Angebot nicht zugänglich")?;
                let wert=(o.menge as i128*o.preis as i128/M as i128) as i64;
                let kosten=wert+mal(wert,self.gebuehr(sid,Some(o.von)));
                if topf.is_some_and(|t|self.spieler[sid as usize].toepfe[t.idx()]<kosten){return Err("Rollenbudget reicht für internen Kauf nicht".into());}
                let result=self.intern_kaufen(sid,angebot,planet)?;
                if let Some(t)=topf{self.spieler[sid as usize].toepfe[t.idx()]-=kosten;}Ok(result)
            },
            Aktion::InternStorno {angebot} => self.intern_storno(sid,angebot),
            Aktion::Nachricht { an, allianz, text } => self.nachricht(sid, &an, allianz, &text),
            Aktion::VertragAnbieten {
                partner,
                art,
                kaution,
                tribut_gut,
                tribut_menge,
                tribut_tage,
            } => self.vertrag_anbieten(
                sid,
                &partner,
                art,
                menge_geprueft(kaution, "kaution")?,
                tribut_gut,
                menge_geprueft(tribut_menge, "tribut_menge")?,
                tribut_tage,
            ),
            Aktion::VertragAnnehmen { vertrag } => self.vertrag_annehmen(sid, vertrag),
            Aktion::VertragAblehnen { vertrag } => self.vertrag_ablehnen(sid, vertrag),
            Aktion::VertragKuendigen { vertrag } => self.vertrag_kuendigen(sid, vertrag),
            Aktion::AllianzGruenden { name, tag } => self.allianz_gruenden_mit_tag(sid, &name, tag.as_deref()),
            Aktion::AllianzEinladen { spieler } => self.allianz_einladen(sid, &spieler),
            Aktion::AllianzBeitreten { allianz } => self.allianz_beitreten(sid, &allianz),
            Aktion::AllianzVerlassen {} => self.allianz_verlassen(sid),
            Aktion::MarktOrder {
                planet,
                gut,
                seite,
                menge,
                preis,
            } => self.markt_order(
                sid,
                rolle,
                planet,
                gut,
                seite,
                menge_geprueft(menge, "menge")?,
                menge_geprueft(preis, "preis")?,
            ),
            Aktion::MarktStorno { order } => self.markt_storno(sid, order),
            Aktion::Doktrin { anteile, text } => {
                let max = self.regeln.agenten.doktrin_zeichen;
                if !anteile.is_empty() {
                    let summe: u32 = anteile.values().map(|v| *v as u32).sum();
                    if summe != 100 {
                        return Err(format!(
                            "die Anteile der Töpfe müssen zusammen 100 ergeben, sind aber {summe}"
                        ));
                    }
                    let sp = &mut self.spieler[sid as usize];
                    for t in Topf::ALLE {
                        sp.anteile[t.idx()] = anteile.get(&t).copied().unwrap_or(0);
                    }
                }
                self.spieler[sid as usize].doktrin = text.chars().take(max).collect();
                Ok("Doktrin gesetzt".into())
            }
            Aktion::Meldung { text } => {
                let zeit = self.zeit;
                let kurz: String = text.chars().take(400).collect();
                self.spieler[sid as usize].meldungen.push(Meldung {
                    zeit,
                    von: rolle,
                    text: kurz,
                });
                if rolle != Rolle::Stratege {
                    self.wecke(sid, Rolle::Stratege, &format!("Meldung vom {rolle}"), false);
                }
                Ok("Meldung hinterlegt".into())
            }
            Aktion::Schenken { an, credits } => {
                self.schenken(sid, &an, menge_geprueft(credits, "credits")?)
            }
        }
    }
}

/// JSON-Schema der Antwort einer Rolle. Es steht neben der Aufzählung der Aktionen, damit
/// beide nicht auseinanderlaufen, und ist so gebaut, dass es sich beim Dekodieren erzwingen
/// lässt: alle Felder Pflicht, keine Zusatzfelder, fehlende Werte als null.
pub fn antwortschema_legacy(rolle: Rolle) -> serde_json::Value {
    let mut schema=antwortschema(rolle);
    fn legacy(v:&mut serde_json::Value) {
        if let Some(o)=v.as_object_mut() {
            if let Some(list)=o.get_mut("enum").and_then(serde_json::Value::as_array_mut) {
                list.retain(|v|!matches!(v.as_str(),Some("geheimdienst"|"ueberwachungstechnik"|"abschirmtechnik"|"saven"|"system_erkunden"|"flotten_spionage")));
            }
            if let Some(cargo)=o.get_mut("ladung") {
                if let Some(props)=cargo["properties"].as_object_mut(){props.remove("antriebskern");props.remove("habitatmodul");}
                if let Some(required)=cargo["required"].as_array_mut(){required.retain(|v|v!="antriebskern" && v!="habitatmodul");}
            }
            for child in o.values_mut(){legacy(child);}
        } else if let Some(a)=v.as_array_mut(){for child in a{legacy(child);}}
    }
    schema["properties"]["aktionen"]["items"]["anyOf"].as_array_mut().unwrap().retain(|v|v["properties"]["typ"]["enum"][0]!="flotte_ausspaehen");
    legacy(&mut schema);schema
}

pub fn antwortschema(rolle: Rolle) -> serde_json::Value {
    use serde_json::{json, Map, Value};
    let namen = |v: Vec<&str>| json!({"type": "string", "enum": v});
    let oder_null = |v: Vec<&str>| {
        let mut werte: Vec<Value> = v.into_iter().map(|x| json!(x)).collect();
        werte.push(Value::Null);
        json!({"type": ["string", "null"], "enum": werte})
    };
    let koord = || json!({"type": "string", "description": "Koordinate Sektor:System:Position, etwa 1:27:6"});
    let text = || json!({"type": "string"});
    let ganzzahl = || json!({"type": "integer"});
    let zahl = || json!({"type": "number"});
    // Grenzen, die der Kern prüft, stehen auch im Schema. Im echten Lauf schickte der Feldherr fertigen mit
    // anzahl 0; DeepInfra setzt minimum/maximum unter strengem Schema durch, Alibaba nimmt sie an, ohne sie
    // durchzusetzen (geprüft am 4. Okt. 2026); die Beschreibung hilft dann wenigstens dem Modell.
    let begrenzt = |typ: &str, min: Value, max: Option<Value>, text: &str| {
        let mut s = json!({"type": typ, "minimum": min, "description": text});
        if let Some(m) = max {
            s["maximum"] = m;
        }
        s
    };
    let objekt = |typ: &str, felder: Vec<(&str, Value)>| -> Value {
        let mut props = Map::new();
        props.insert("typ".into(), json!({"type": "string", "enum": [typ]}));
        let mut pflicht = vec![json!("typ")];
        for (n, sch) in felder {
            props.insert(n.to_string(), sch);
            pflicht.push(json!(n));
        }
        json!({"type": "object", "properties": props, "required": pflicht, "additionalProperties": false})
    };
    let alle_felder = |namen: Vec<&str>, typ: &str| -> Value {
        let props: Map<String, Value> = namen
            .iter()
            .map(|n| (n.to_string(), json!({"type": typ})))
            .collect();
        json!({"type": "object", "properties": props, "required": namen, "additionalProperties": false})
    };
    let gebaeude: Vec<&str> = Gebaeude::ALLE.iter().map(|g| g.name()).collect();
    let forschung: Vec<&str> = Forschung::ALLE.iter().map(|g| g.name()).collect();
    let einheit: Vec<&str> = Einheit::ALLE.iter().map(|g| g.name()).collect();
    let schiffsnamen: Vec<&str> = Einheit::ALLE
        .iter()
        .filter(|e| e.ist_schiff())
        .map(|g| g.name())
        .collect();
    let fracht: Vec<&str> = Gut::ALLE.iter().map(|g| g.name()).collect();
    let gut: Vec<&str> = Gut::ALLE.iter().map(|g| g.name()).collect();
    let schiffe = || alle_felder(schiffsnamen.clone(), "integer");
    let missionen: Vec<&str> = Mission::ALLE
        .iter()
        .filter(|m| {
            if **m == Mission::FlottenSpionage { return false; }
            let a = Aktion::FlotteSenden {
                start: Koord::neu(1, 1, 1),
                ziel: Koord::neu(1, 1, 1),
                mission: **m,
                schiffe: BTreeMap::new(),
                geschwindigkeit: 1.0,
                ladung: BTreeMap::new(),
                haltedauer_stunden: 0,
            };
            rolle == Rolle::Alle || a.zustaendig().contains(&rolle)
        })
        .map(|m| m.name())
        .collect();

    let mut aktionen: Vec<Value> = Vec::new();
    for typ in erlaubte_typen(rolle) {
        if typ == "fertigen" {
            // Genau eines von einheit und bauteil, als zwei Varianten: mit zwei Feldern, die beide null sein durften,
            // schickten Modelle leere Platzhalter, und jede Ablehnung kostete eine Korrekturrunde.
            let stueck = || {
                begrenzt(
                    "integer",
                    json!(1),
                    Some(json!(10_000)),
                    "Stückzahl 1 bis 10000",
                )
            };
            aktionen.push(objekt(
                typ,
                vec![
                    ("planet", koord()),
                    ("einheit", namen(einheit.clone())),
                    ("anzahl", stueck()),
                ],
            ));
            aktionen.push(objekt(
                typ,
                vec![
                    ("planet", koord()),
                    ("bauteil", namen(vec!["antriebskern", "habitatmodul"])),
                    ("anzahl", stueck()),
                ],
            ));
            continue;
        }
        let felder: Vec<(&str, Value)> = match typ {
            "bauen" | "abreissen" | "reparieren" => {
                vec![("planet", koord()), ("gebaeude", namen(gebaeude.clone()))]
            }
            "schleife_leeren" => vec![("planet", koord())],
            "forschen" => vec![
                ("forschung", namen(forschung.clone())),
                ("planet", json!({"type": ["string", "null"]})),
            ],
            "steuersatz" => vec![(
                "prozent",
                begrenzt("integer", json!(0), Some(json!(50)), "Prozent, 0 bis 50"),
            )],
            "prioritaeten" => vec![
                ("planet", koord()),
                (
                    "reihenfolge",
                    json!({"type": "array", "items": namen(gebaeude.clone())}),
                ),
            ],
            "flotte_senden" => vec![
                ("start", koord()),
                ("ziel", koord()),
                ("mission", namen(missionen.clone())),
                ("schiffe", schiffe()),
                (
                    "geschwindigkeit",
                    begrenzt(
                        "number",
                        json!(0.1),
                        Some(json!(1.0)),
                        "Anteil der Höchstgeschwindigkeit, 0.1 bis 1.0",
                    ),
                ),
                ("ladung", alle_felder(fracht.clone(), "number")),
                ("haltedauer_stunden", ganzzahl()),
            ],
            "flotte_zurueckrufen" | "verband_oeffnen" => vec![("flotte", ganzzahl())],
            "flotte_ausspaehen" => vec![("start", koord()), ("flotte", ganzzahl()),
                ("sonden",begrenzt("integer",json!(1),Some(json!(1000)),"Physische Spionagesonden")),
                ("geschwindigkeit",begrenzt("number",json!(0.1),Some(json!(1.0)),"Anteil der Höchstgeschwindigkeit"))],
            "flotte_versorgen" => vec![
                ("start", koord()),
                ("ziel", koord()),
                ("versorgungsflotte", ganzzahl()),
                ("schiffe", schiffe()),
                (
                    "geschwindigkeit",
                    begrenzt(
                        "number",
                        json!(0.1),
                        Some(json!(1.0)),
                        "Anteil der Höchstgeschwindigkeit",
                    ),
                ),
                ("ladung", alle_felder(fracht.clone(), "number")),
            ],
            "verband_beitreten" => vec![("flotte", ganzzahl()), ("fuehrung", ganzzahl())],
            "raketen_bauen" => vec![
                ("planet", koord()),
                ("art", namen(vec!["abfang", "interplanetar"])),
                (
                    "anzahl",
                    begrenzt(
                        "integer",
                        json!(1),
                        Some(json!(1_000_000)),
                        "Stückzahl, mindestens 1; das Silo begrenzt weiter",
                    ),
                ),
            ],
            "raketen_starten" => vec![
                ("start", koord()),
                ("ziel", koord()),
                (
                    "anzahl",
                    begrenzt(
                        "integer",
                        json!(1),
                        None,
                        "mindestens 1, höchstens die fertigen Interplanetarraketen",
                    ),
                ),
                (
                    "zieltyp",
                    namen(
                        Einheit::ALLE
                            .iter()
                            .filter(|e| !e.ist_schiff())
                            .map(|e| e.name())
                            .collect(),
                    ),
                ),
            ],
            "brief_senden" => vec![("kanal",namen(vec!["privat","allianz","diplomatie"])),("an",text()),("betreff",text()),("text",text()),("antwort_auf",json!({"type":["integer","null"]}))],
            "allianz_anfrage" => vec![("allianz",text()),("art",namen(vec!["nichtangriffspakt","verteidigungsbuendnis","handelsabkommen","krieg","frieden"])),("text",text())],
            "allianz_pakt_kuendigen" => vec![("anfrage",ganzzahl())],
            "brief_lesen" => vec![("brief",ganzzahl())],
            "diplomatie_anfrage" => vec![("partner",text()),("art",namen(vec!["nichtangriffspakt","verteidigungsbuendnis","handelsabkommen","allianz","krieg","frieden"])),("text",text())],
            "diplomatie_entscheiden" => vec![("anfrage",ganzzahl()),("annehmen",json!({"type":"boolean"}))],
            "allianz_rolle" => vec![("spieler",text()),("rang",ganzzahl())],
            "allianz_ausschliessen" => vec![("spieler",text())],
            "allianz_hilfe" => vec![("planet",koord()),("text",text())],
            "allianz_hilfe_status" => vec![("hilfe",ganzzahl()),("erledigt",json!({"type":"boolean"}))],
            "intern_anbieten" => vec![("planet",koord()),("gut",namen(gut.clone())),("menge",zahl()),("preis",zahl())],
            "intern_kaufen" => vec![("angebot",ganzzahl()),("planet",koord())],
            "intern_storno" => vec![("angebot",ganzzahl())],
            "nachricht" => vec![
                ("an", json!({"type": "array", "items": text()})),
                ("allianz", json!({"type": "boolean"})),
                ("text", text()),
            ],
            "vertrag_anbieten" => vec![
                ("partner", text()),
                (
                    "art",
                    namen(Vertragsart::ALLE.iter().map(|v| v.name()).collect()),
                ),
                ("kaution", zahl()),
                ("tribut_gut", oder_null(gut.clone())),
                ("tribut_menge", zahl()),
                ("tribut_tage", ganzzahl()),
            ],
            "vertrag_annehmen" | "vertrag_ablehnen" | "vertrag_kuendigen" => {
                vec![("vertrag", ganzzahl())]
            }
            "allianz_gruenden" => vec![("name", text()),("tag",json!({"type":["string","null"],"description":"Allianz-Kürzel: 2–6 Buchstaben/Ziffern oder null für automatisch"}))],
            "allianz_einladen" => vec![("spieler", text())],
            "allianz_beitreten" => vec![("allianz", text())],
            "markt_order" => vec![
                ("planet", koord()),
                ("gut", namen(gut.clone())),
                ("seite", namen(vec!["kauf", "verkauf"])),
                ("menge", zahl()),
                ("preis", zahl()),
            ],
            "markt_storno" => vec![("order", ganzzahl())],
            "doktrin" => vec![
                (
                    "anteile",
                    alle_felder(Topf::ALLE.iter().map(|t| t.name()).collect(), "integer"),
                ),
                ("text", text()),
            ],
            "meldung" => vec![("text", text())],
            "schenken" => vec![("an", text()), ("credits", zahl())],
            _ => Vec::new(),
        };
        aktionen.push(objekt(typ, felder));
    }
    let abfragen = vec![
        objekt(
            "kosten",
            vec![
                ("gebaeude", oder_null(gebaeude.clone())),
                ("forschung", oder_null(forschung.clone())),
                ("einheit", oder_null(einheit.clone())),
                ("rakete", oder_null(vec!["abfang", "interplanetar"])),
                ("anzahl", json!({"type": ["integer", "null"]})),
                ("stufe", json!({"type": ["integer", "null"]})),
                ("planet", json!({"type": ["string", "null"]})),
            ],
        ),
        objekt(
            "flugzeit",
            vec![
                ("start", koord()),
                ("ziel", koord()),
                ("schiffe", schiffe()),
                ("geschwindigkeit", zahl()),
            ],
        ),
        objekt(
            "kolonieplan",
            vec![
                ("start", koord()),
                ("ziel", koord()),
                ("schiffe", schiffe()),
                ("ladung", alle_felder(fracht.clone(), "number")),
                ("geschwindigkeit", zahl()),
                (
                    "aufbau",
                    json!({"type":"array","items":namen(gebaeude.clone()),"maxItems":32}),
                ),
            ],
        ),
        objekt(
            "kampfsimulator",
            vec![("ziel", koord()), ("schiffe", schiffe())],
        ),
        objekt("briefarchiv", vec![("vor_id",ganzzahl()),("limit",ganzzahl()),("kanal",namen(vec!["alle","privat","allianz","diplomatie"]))]),
        objekt("regel", vec![("stichwort", text())]),
        objekt(
            "galaxie",
            vec![
                ("sektor", ganzzahl()),
                ("von", ganzzahl()),
                ("bis", ganzzahl()),
            ],
        ),
    ];
    json!({
        "type": "object",
        "properties": {
            "begruendung": {"type": "string", "description": "kurze Begründung, höchstens 150 Wörter"},
            "abfragen": {"type": "array", "items": {"anyOf": abfragen}},
            "aktionen": {"type": "array", "items": {"anyOf": aktionen}},
            "prognose": {"type": "string", "description": "was du bis zum nächsten Aufruf erwartest, ein Satz"},
            "notiz": {"type": "string", "description": "dein neues Notizbuch, ersetzt das alte"},
            "wecker_stunden": {"type": ["number", "null"], "description": "Wecker in Spielstunden oder null"},
        },
        "required": ["begruendung", "abfragen", "aktionen", "prognose", "notiz", "wecker_stunden"],
        "additionalProperties": false,
    })
}
