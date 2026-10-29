use super::modell::{PersonenVertrag, SCHEMA_VERSION};
use crate::{typen::*, Welt};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize)]
pub struct RegelKontext {
    pub schema_version: u32,
    pub spielstand_generation: u8,
    pub ausbau_version: u8,
    pub regelversion: String,
    pub protokoll_regelhash: String,
    pub wirksamer_regelhash: String,
    pub kolonisation_version: u8,
    pub aufklaerung: bool,
    pub flottendoktrin: bool,
    pub personen: PersonenVertrag,
    pub gueter_reihenfolge: Vec<&'static str>,
    pub mengen_einheit: &'static str,
    pub raten_einheit: &'static str,
    pub zeit_einheit: &'static str,
    pub energie_einheit: &'static str,
    pub deckung_einheit: &'static str,
}
pub(crate) fn fingerprint(v: &impl Serialize) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(v).expect("finite core projection"))
    )
}
impl Welt {
    pub fn planer_regelkontext(&self) -> RegelKontext {
        // Flottendoktrin is serde(skip) in legacy Regelwerk. Include it explicitly.
        // Never hash the hidden world or its random streams into public metadata.
        let version = if self.kolonisation.aktiv {
            self.kolonisation.version
        } else {
            0
        };
        let basis = (
            &*self.regeln,
            &self.regeln.flottendoktrin,
            version,
            self.aufklaerungsregeln(),
            &self.ausscheiden.regeln,
            self.ausbau.version,
            (self.ausbau.version>0).then(||crate::raketen::regeln(&self.regeln)),
            (self.ausbau.version>0).then_some((crate::habitabilitaet::WACHSTUM,crate::habitabilitaet::FREIGABE,crate::habitabilitaet::SOLAR,crate::habitabilitaet::ENDSPIEL_TECH,crate::habitabilitaet::RESTKLIMA,crate::habitabilitaet::KLIMAVERLUST,crate::habitabilitaet::OPTIMUM)),
        );
        // Ablauf-Version 1 ergänzt ihre Tabellen; Version 0 behält den bisherigen Fingerabdruck bytegleich.
        let wirksamer_regelhash = if self.ausbau.ablauf == 0 {
            fingerprint(&basis)
        } else {
            fingerprint(&(basis, self.ausbau.ablauf, crate::habitabilitaet::WACHSTUM_V1, crate::habitabilitaet::FREIGABE_V1, crate::habitabilitaet::SOLAR_V1,
                crate::ablauf::RESERVE_PROZENT, crate::ablauf::RESERVE_PERSONEN))
        };
        // Kernwerte (neue Welten ab 21.10.2026) gehen in den Fingerabdruck ein; ohne sie bleibt er bytegleich.
        let wirksamer_regelhash = match self.regeln.technik.as_ref().and_then(|t| t.kernwerte.as_ref()) {
            Some(kw) if self.ausbau.ablauf > 0 => fingerprint(&(&wirksamer_regelhash, kw)),
            _ => wirksamer_regelhash,
        };
        let mut personen=PersonenVertrag::default();
        if self.ausbau.version>0{
            personen.modell="gruppen_v1";personen.herkunft="dauerhaft_nach_herkunft";personen.qualifikation="dauerhafte_disjunkte_qualifikation";
            personen.gemischte_bevoelkerung=true;personen.dauerhafte_qualifikationen=true;personen.personenbefoerderung=true;personen.expeditionen=true;
        }
        RegelKontext {
            spielstand_generation:if self.ausbau.version>0{12}else{11},ausbau_version:self.ausbau.version,
            schema_version: SCHEMA_VERSION,
            regelversion: self.regeln.version.clone(),
            protokoll_regelhash: self.regeln.hash.clone(),
            wirksamer_regelhash,
            kolonisation_version: version,
            aufklaerung: self.aufklaerungsregeln(),
            flottendoktrin: !self.regeln.flottendoktrin.is_empty(),
            personen,
            gueter_reihenfolge: Gut::ALLE.iter().map(|g| g.name()).collect(),
            mengen_einheit: "tausendstel_gut",
            raten_einheit: "tausendstel_gut_je_spielstunde",
            zeit_einheit: "spielsekunden",
            energie_einheit: "tausendstel_energie",
            deckung_einheit: "promille",
        }
    }
}
