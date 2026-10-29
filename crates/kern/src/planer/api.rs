use super::{
    kontext::fingerprint,
    kosten::{self, KostenAnfrage},
};
use crate::{typen::*, Welt};
use serde_json::{json, Value};

impl Welt {
    /// Projects one owned planet only; does not clone/advance hidden fleets or enemies.
    pub fn wirtschaftsdiagnose(&self, sid: SpielerId, koord: Koord) -> Result<Value, String> {
        let sp = self.spieler.get(sid as usize).ok_or("Spieler fehlt")?;
        let pid = self.eigener_planet(sid, koord)?;
        let kontext = self.planer_regelkontext();
        let mut p = self.planeten[pid].clone();
        p.bestand = self.bestand_jetzt(pid);
        let hp: [i64; GEBAEUDE] = std::array::from_fn(|i| self.integritaet(pid, Gebaeude::ALLE[i]));
        let aktiv = self.spieler_aktiv(sid);
        let revision = fingerprint(&(
            &kontext.wirksamer_regelhash,
            self.zeit,
            &p,
            &sp.forschung,
            sp.volk,
            hp,
            self.produktionsstufen(pid),
            aktiv,
        ));
        let ergebnis =
            aktiv.then(|| self.industrie_berechnen(&p));
        Ok(
            json!({"kontext":kontext,"planet":koord.to_string(),"spielzeit":self.zeit,"revision":revision,
            "status":if aktiv {"berechnet"}else{"inaktiv"},"bestand_milli":p.bestand,"bevoelkerung_milli":p.bevoelkerung,
            "gespeicherte_rate_milli":p.rate,"berechnung":ergebnis,"habitabilitaet":self.klima_sicht(sid,koord),
            "annahmen":["Momentaufnahme nach Bestandsabrechnung; keine Zukunftssimulation",
                "Neuberechnung wie raten_neu; gespeicherte Raten gelten bis zur nächsten tatsächlichen Neuberechnung",
                "Revision gilt nur für diesen eigenen Ausschnitt; Weltkennung wird vom Transport ergänzt"]}),
        )
    }
    pub fn kostenplan(
        &self,
        sid: SpielerId,
        koord: Koord,
        anfragen: &[KostenAnfrage],
    ) -> Result<Value, String> {
        let sp = self.spieler.get(sid as usize).ok_or("Spieler fehlt")?;
        let pid = self.eigener_planet(sid, koord)?;
        let mut ergebnis = kosten::berechnen(&self.regeln, sp.volk, anfragen)?;
        if self.ausbau.industrie_version>0 {
            for pos in &mut ergebnis.positionen {
                let work=if pos.stufe.is_some(){None}else if Einheit::aus_name(&pos.kennung).is_some(){Some(Gebaeude::Werft)}else{Some(if self.planeten[pid].gebaeude[Gebaeude::Orbitalwerft.idx()]>0{Gebaeude::Orbitalwerft}else{Gebaeude::Werft})};
                if let Some(g)=work{pos.stueck=self.produktionskosten(pid,g,pos.stueck);pos.gesamt=pos.stueck.map(|n|n*pos.anzahl);}
            }
            ergebnis.gesamt=std::array::from_fn(|i|ergebnis.positionen.iter().map(|p|p.gesamt[i]).sum());
        }
        let bestand = self.bestand_jetzt(pid);
        let fehlmenge: [i64; GUETER] =
            std::array::from_fn(|i| (ergebnis.gesamt[i] - bestand[i]).max(0));
        let kontext = self.planer_regelkontext();
        let revision = fingerprint(&(
            &kontext.wirksamer_regelhash,
            self.zeit,
            sid,
            koord,
            bestand,
            sp.volk,
            self.produktionsstufen(pid),
        ));
        let mut crew = 0i64;
        for a in anfragen {
            if let KostenAnfrage::Einheit { einheit, anzahl } = a {
                crew = crew
                    .checked_add(
                        self.regeln
                            .einh(*einheit)
                            .besatzung
                            .checked_mul(*anzahl)
                            .ok_or("Crewbedarf zu groß")?,
                    )
                    .ok_or("Crewbedarf zu groß")?;
            }
        }
        Ok(
            json!({"kontext":kontext,"planet":koord.to_string(),"spielzeit":self.zeit,"revision":revision,
            "eingabehash":fingerprint(&anfragen),"kosten":ergebnis,"bestand_milli":bestand,"fehlmenge_milli":fehlmenge,"crewbedarf_personen":crew,
            "annahmen":["Nur Preise; keine Bauzulassung, Budgetfreigabe oder Zeitfolge zugesagt",
                "Bestellte und schon bezahlte Aufträge sind bereits im Bestand abgezogen; keine zweite Reservierung",
                "Alle angegebenen Positionen sind neue, noch unbezahlte Vorhaben"]}),
        )
    }
}
