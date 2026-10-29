//! Read-only result contracts. None of these types is part of a saved world.
use crate::typen::*;
use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;
pub type GueterMilli = [i64; GUETER];

/// Every delta is measured AFTER the original core rounding, not from a UI formula.
#[derive(Clone, Debug, Serialize)]
pub struct Rechenschritt {
    pub reihenfolge: usize,
    pub quelle: &'static str,
    pub operation: &'static str,
    pub parameter: f64,
    pub vorher: i64,
    pub nachher: i64,
    pub delta: i64,
    pub rundung: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ErklaerterWert {
    pub einheit: &'static str,
    pub grundwert: i64,
    pub schritte: Vec<Rechenschritt>,
    pub wert: i64,
}
impl ErklaerterWert {
    pub fn neu(grundwert: i64, einheit: &'static str) -> Self {
        Self {
            einheit,
            grundwert,
            schritte: Vec::new(),
            wert: grundwert,
        }
    }
    pub fn in_einheit(mut self, einheit: &'static str) -> Self {
        self.einheit = einheit;
        self
    }
    fn schritt(
        mut self,
        quelle: &'static str,
        operation: &'static str,
        parameter: f64,
        nachher: i64,
        rundung: &'static str,
    ) -> Self {
        self.schritte.push(Rechenschritt {
            reihenfolge: self.schritte.len(),
            quelle,
            operation,
            parameter,
            vorher: self.wert,
            nachher,
            delta: nachher - self.wert,
            rundung,
        });
        self.wert = nachher;
        self
    }
    pub fn promille(self, quelle: &'static str, faktor: i64) -> Self {
        let n = anteil(self.wert, faktor, 1000);
        self.schritt(
            quelle,
            "promille",
            faktor as f64,
            n,
            "Ganzzahldivision gegen null nach Multiplikation / 1000",
        )
    }
    pub fn faktor(self, quelle: &'static str, faktor: f64) -> Self {
        let n = mal(self.wert, faktor);
        self.schritt(
            quelle,
            "faktor",
            faktor,
            n,
            "Core mal: Faktor in Festkomma 1e6, Ganzzahldivision gegen null",
        )
    }
    pub fn plus(self, quelle: &'static str, zusatz: i64) -> Self {
        let n = self.wert + zusatz;
        self.schritt(quelle, "addition", zusatz as f64, n, "keine")
    }
    pub fn mindestens(self, quelle: &'static str, minimum: i64) -> Self {
        let n = self.wert.max(minimum);
        self.schritt(quelle, "minimum", minimum as f64, n, "keine")
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct KostenErgebnis {
    pub einheit: &'static str,
    pub positionen: Vec<KostenPosition>,
    pub gesamt: GueterMilli,
}
#[derive(Clone, Debug, Serialize)]
pub struct KostenPosition {
    pub kennung: String,
    pub stufe: Option<u8>,
    pub anzahl: i64,
    pub stueck: GueterMilli,
    pub gesamt: GueterMilli,
}

/// Personnel are counts, never a second resource price for people.
#[derive(Clone, Debug, Serialize)]
pub struct PersonenVertrag {
    pub modell: &'static str,
    pub bevoelkerung_einheit: &'static str,
    pub arbeitswerte_einheit: &'static str,
    pub personenplaetze_einheit: &'static str,
    pub crew_einheit: &'static str,
    pub crew_abbuchung: &'static str,
    pub herkunft: &'static str,
    pub qualifikation: &'static str,
    pub gemischte_bevoelkerung: bool,
    pub dauerhafte_qualifikationen: bool,
    pub personenbefoerderung: bool,
    pub expeditionen: bool,
}
impl Default for PersonenVertrag {
    fn default() -> Self {
        Self {
            modell: "bestand_v1",
            bevoelkerung_einheit: "tausendstel_person",
            arbeitswerte_einheit: "tausendstel_person",
            personenplaetze_einheit: "ganze_person",
            crew_einheit: "ganze_person",
            crew_abbuchung: "bei_bestellung_einmalig_von_planetenbevoelkerung",
            herkunft: "nicht_individuell_gespeichert_versorgung_nach_reichsvolk",
            qualifikation: "aus_akademie_und_arbeitsangebot_abgeleitet_keine_personenqualifikation",
            gemischte_bevoelkerung: false,
            dauerhafte_qualifikationen: false,
            personenbefoerderung: false,
            expeditionen: false,
        }
    }
}
