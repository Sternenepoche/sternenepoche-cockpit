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
    FlotteBeladen {flug:serde_json::Value,#[serde(default)] personen:crate::ausbau::Personen,#[serde(default,deserialize_with="crate::boden_katalog::truppen")] truppen:[i64;crate::boden_katalog::ANZAHL]},
    Schiffangebot {flug:serde_json::Value,#[serde(default)] preis:f64,#[serde(default)] personen:crate::ausbau::Personen,#[serde(default,deserialize_with="crate::boden_katalog::truppen")] truppen:[i64;crate::boden_katalog::ANZAHL]},
    SchiffangebotAntwort {angebot:u64,annehmen:bool},
    Ausbilden {planet:Koord,volk:Volk,qualifikation:u8,anzahl:i64},
    PersonenSenden {flug:serde_json::Value,personen:crate::ausbau::Personen},
    ExpeditionStarten {flug:serde_json::Value},
    ExpeditionReparieren {flotte:FlottenId,einheit:Einheit},
    RettungStarten {expedition:FlottenId,flug:serde_json::Value},
    PlanUebernehmen { revision: String, schritte: Vec<serde_json::Value> },
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
    PlanetAufgeben { planet: Koord, bestaetigung: Koord },
    SystemAufklaeren { start: Koord, ziel: Koord, #[serde(default)] abbrechen: bool },
    AuftragPause { planet: Koord, art: String, pausiert: bool },
    PlanetBenennen { planet: Koord, #[serde(default)] name: String },
    RaketenStorno {planet:Koord,auftrag:u64},
    MilizAufrufen {planet:Koord,anzahl:i64},
    MilizAusrufen {planet:Koord},
    MilizEntlassen {planet:Koord},
    /// Invasion v2: Truppen und Miliz den Verteidigungslinien zuteilen (je Linie 13 Zahlen: 12 Bodentypen, dann Miliz).
    InvasionLinien {planet:Koord,linien:Vec<Vec<i64>>},
    /// 0.4.8: Start einer gestrandeten Flotte mit Deuterium aus ihrer Ladung (rueckgeschwindigkeit 0 = kein Rückflug).
    FlotteStarten {flotte:FlottenId,#[serde(default)] ziel:Option<Koord>,mission:Mission,#[serde(default="voll")] geschwindigkeit:f64,
        #[serde(default="voll")] rueckgeschwindigkeit:f64,#[serde(default)] haltedauer_stunden:i64,#[serde(default)] zielflotte:Option<FlottenId>},
    /// 0.4.8: Flottenübergabe einer gestrandeten Flotte an den Besitzer des Zielplaneten (Angebot).
    OrbitflotteAnbieten {flotte:FlottenId,ziel:Koord,#[serde(default)] preis:f64},
    /// 0.4.8: Angriff von einem eigenen Planeten auf eine gestrandete Flotte.
    OrbitflotteAngreifen {start:Koord,zielflotte:FlottenId,schiffe:BTreeMap<Einheit,i64>,#[serde(default="voll")] geschwindigkeit:f64,
        #[serde(default="voll")] rueckgeschwindigkeit:f64},
    BodenBauen {planet:Koord,art:String,anzahl:i64},
    BodenStorno {planet:Koord,auftrag:u64},
    FlottePlanen {flug:serde_json::Value,#[serde(default)] ankunft:Option<i64>,#[serde(default="voll")] rueckgeschwindigkeit:f64},
    FlugplanStorno {auftrag:u64},
    BodenSenden {flug:serde_json::Value,#[serde(deserialize_with="crate::boden_katalog::truppen")] truppen:[i64;crate::boden_katalog::ANZAHL]},
    Produktionsserie {planet:Koord,art:String,produkt:String,ziel:i64},
    ProduktionSetzen {planet:Koord,gebaeude:Gebaeude,prozent:u8},
    Soforthandel { planet: Koord, angebot: BTreeMap<String, f64>, ziel: String },
    SchleifeLeeren {
        planet: Koord,
    },
    WarteschlangeAendern { planet:Koord,art:String,index:usize,ziel:Option<usize>,loeschen:bool,kennung:String },
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
        #[serde(default)]
        siedler_volk: Option<Volk>,
        /// Kolonisieren (Karl 24.10.2026): Siedler als [Bewohner, Fachkräfte, Forscher] in ganzen Personen, Summe = Plätze
        /// des Kolonieschiffs. Ohne Angabe wählt der Kern wie bisher (erst Bewohner, dann Fachkräfte, dann Forscher).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        siedler_mischung: Option<[i64; 3]>,
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
        #[serde(default)] art: Option<String>,
        #[serde(default)] flotte: Option<FlottenId>,
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
    /// Direkte Einladung ablehnen (Funktionsprüfung 25.10.2026: Einladungen ließen sich nur annehmen).
    AllianzEinladungAblehnen {
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
    /// Karl 26.10.2026: Der Stratege bucht vorhandenes Topfguthaben sofort um (doktrin verteilt nur künftiges Einkommen).
    TopfUmbuchen {
        von: Topf,
        nach: Topf,
        menge: f64,
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
            Aktion::FlotteBeladen{..}=>"flotte_beladen",
            Aktion::Schiffangebot{..}=>"schiffangebot",
            Aktion::SchiffangebotAntwort{..}=>"schiffangebot_antwort",
            Aktion::Ausbilden{..}=>"ausbilden",
            Aktion::PersonenSenden{..}=>"personen_senden",
            Aktion::ExpeditionStarten{..}=>"expedition_starten",
            Aktion::ExpeditionReparieren{..}=>"expedition_reparieren",
            Aktion::RettungStarten{..}=>"rettung_starten",
            Aktion::PlanUebernehmen { .. } => "plan_uebernehmen",
            Aktion::Bauen { .. } => "bauen",
            Aktion::Reparieren { .. } => "reparieren",
            Aktion::Abreissen { .. } => "abreissen",
            Aktion::PlanetAufgeben { .. } => "planet_aufgeben",
            Aktion::SystemAufklaeren { .. } => "system_aufklaeren",
            Aktion::AuftragPause { .. } => "auftrag_pause",
            Aktion::PlanetBenennen { .. } => "planet_benennen",
            Aktion::RaketenStorno{..}=>"raketen_storno",
            Aktion::MilizAufrufen {..} => "miliz_aufrufen",
            Aktion::MilizAusrufen {..} => "miliz_ausrufen",
            Aktion::MilizEntlassen {..} => "miliz_entlassen",
            Aktion::InvasionLinien {..} => "invasion_linien",
            Aktion::FlotteStarten {..} => "flotte_starten",
            Aktion::OrbitflotteAngreifen {..} => "orbitflotte_angreifen",
            Aktion::OrbitflotteAnbieten {..} => "orbitflotte_anbieten",
            Aktion::BodenBauen {..} => "boden_bauen",
            Aktion::BodenStorno {..} => "boden_storno",
            Aktion::BodenSenden {..} => "boden_senden",
            Aktion::FlottePlanen{..}=>"flotte_planen",
            Aktion::FlugplanStorno{..}=>"flugplan_storno",
            Aktion::Produktionsserie {..} => "produktionsserie",
            Aktion::ProduktionSetzen {..} => "produktion_setzen",
            Aktion::Soforthandel { .. } => "soforthandel",
            Aktion::SchleifeLeeren { .. } => "schleife_leeren",
            Aktion::WarteschlangeAendern { .. } => "warteschlange_aendern",
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
            Aktion::AllianzEinladungAblehnen { .. } => "allianz_einladung_ablehnen",
            Aktion::AllianzVerlassen {} => "allianz_verlassen",
            Aktion::MarktOrder { .. } => "markt_order",
            Aktion::MarktStorno { .. } => "markt_storno",
            Aktion::Doktrin { .. } => "doktrin",
            Aktion::TopfUmbuchen { .. } => "topf_umbuchen",
            Aktion::Meldung { .. } => "meldung",
            Aktion::Schenken { .. } => "schenken",
        }
    }

    /// Rollen, die diese Aktion ausführen dürfen.
    pub fn zustaendig(&self) -> &'static [Rolle] {
        use Rolle::*;
        match self {
            Aktion::Ausbilden{..}=>&[Verwalter],
            Aktion::PersonenSenden{..}|Aktion::FlotteBeladen{..}|Aktion::ExpeditionStarten{..}|Aktion::ExpeditionReparieren{..}|Aktion::RettungStarten{..}=>&[Verwalter,Feldherr],
            Aktion::Schiffangebot{..}|Aktion::SchiffangebotAntwort{..}|Aktion::OrbitflotteAnbieten{..}=>&[Diplomat,Feldherr],
            Aktion::PlanUebernehmen { .. } => &[Stratege, Verwalter, Feldherr],
            Aktion::Doktrin { .. } | Aktion::TopfUmbuchen { .. } | Aktion::PlanetAufgeben { .. } => &[Stratege],
            Aktion::FlottePlanen{..}|Aktion::FlugplanStorno{..}=>&[Feldherr,Verwalter],
            Aktion::MilizAufrufen{..}|Aktion::MilizAusrufen{..}|Aktion::MilizEntlassen{..}|Aktion::InvasionLinien{..}|Aktion::FlotteStarten{..}|Aktion::OrbitflotteAngreifen{..}|Aktion::BodenBauen {..}|Aktion::BodenStorno {..}|Aktion::BodenSenden {..} => &[Feldherr],
            Aktion::Soforthandel { .. } | Aktion::ProduktionSetzen {..} => &[Verwalter],
            Aktion::Produktionsserie {..} => &[Verwalter,Feldherr],
            Aktion::AuftragPause { .. } => &[Verwalter, Feldherr],
            Aktion::PlanetBenennen { .. } => &[Stratege, Verwalter],
            Aktion::SystemAufklaeren { .. } => &[Feldherr],
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
            // Agentenzugang 26.10.2026: raketen_bauen durften beide, stornieren nur der Feldherr – der Verwalter blieb
            // auf eigenen Fehlbestellungen sitzen.
            | Aktion::RaketenStorno { .. }
            | Aktion::WarteschlangeAendern { .. }
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
                | Mission::Recyceln
                | Mission::Terraformen => &[Verwalter, Feldherr],
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
        ("topf_umbuchen", &[Rolle::Stratege][..]),
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
        ("ausbilden", &[Rolle::Verwalter][..]),
        ("personen_senden", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("flotte_beladen", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("schiffangebot", &[Rolle::Diplomat,Rolle::Feldherr][..]),
        ("schiffangebot_antwort", &[Rolle::Diplomat,Rolle::Feldherr][..]),
        ("expedition_starten", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("expedition_reparieren", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("rettung_starten", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("plan_uebernehmen", &[Rolle::Stratege,Rolle::Verwalter,Rolle::Feldherr][..]),
        ("bauen", &[Rolle::Verwalter][..]),
        ("reparieren", &[Rolle::Verwalter][..]),
        ("abreissen", &[Rolle::Verwalter][..]),
        ("planet_aufgeben", &[Rolle::Stratege][..]),
        ("system_aufklaeren", &[Rolle::Feldherr][..]),
        ("raketen_storno",&[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("miliz_aufrufen", &[Rolle::Feldherr][..]),
        ("miliz_ausrufen", &[Rolle::Feldherr][..]),
        ("miliz_entlassen", &[Rolle::Feldherr][..]),
        ("invasion_linien", &[Rolle::Feldherr][..]),
        ("flotte_starten", &[Rolle::Feldherr][..]),
        ("orbitflotte_angreifen", &[Rolle::Feldherr][..]),
        ("orbitflotte_anbieten", &[Rolle::Diplomat,Rolle::Feldherr][..]),
        ("boden_bauen", &[Rolle::Feldherr][..]),
        ("boden_storno", &[Rolle::Feldherr][..]),
        ("boden_senden", &[Rolle::Feldherr][..]),
        ("produktionsserie", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("flotte_planen", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("flugplan_storno", &[Rolle::Verwalter,Rolle::Feldherr][..]),
        ("produktion_setzen", &[Rolle::Verwalter][..]),
        ("auftrag_pause", &[Rolle::Verwalter, Rolle::Feldherr][..]),
        ("planet_benennen", &[Rolle::Stratege, Rolle::Verwalter][..]),
        ("soforthandel", &[Rolle::Verwalter][..]),
        ("schleife_leeren", &[Rolle::Verwalter][..]),
        ("warteschlange_aendern", &[Rolle::Verwalter,Rolle::Feldherr][..]),
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
        ("allianz_einladung_ablehnen", &[Rolle::Diplomat][..]),
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
        // Mondgeneration: eigene Aktionen und flotte_senden mit start-/zielart mond.
        let ergebnis = if let Some(e) = self.mond_aktion(sid, rolle, aktion) { e } else if let Some(e) = self.dyson_aktion(sid, rolle, aktion) { e } else { match serde_json::from_value::<Aktion>(aktion.clone()) {
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
        } };
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
            Aktion::Ausbilden{planet,volk,qualifikation,anzahl}=>{if rolle!=Rolle::Alle&&rolle!=Rolle::Verwalter{return Err("Ausbildung benötigt Verwalterrolle".into());}self.ausbilden(sid,planet,volk,qualifikation,anzahl)},
            Aktion::PersonenSenden{flug,personen}=>self.personen_senden(sid,rolle,&flug,personen),
            Aktion::FlotteBeladen{flug,personen,truppen}=>self.flotte_beladen(sid,rolle,&flug,personen,truppen),
            Aktion::Schiffangebot{flug,preis,personen,truppen}=>self.schiffangebot(sid,rolle,&flug,preis,personen,truppen),
            Aktion::SchiffangebotAntwort{angebot,annehmen}=>self.schiffangebot_antwort(sid,rolle,angebot,annehmen),
            Aktion::ExpeditionStarten{flug}=>self.expedition_starten(sid,rolle,&flug),
            Aktion::ExpeditionReparieren{flotte,einheit}=>self.expedition_reparieren(sid,flotte,einheit),
            Aktion::RettungStarten{expedition,flug}=>self.rettung_starten(sid,rolle,expedition,&flug),
            Aktion::PlanUebernehmen { revision, schritte } => self.plan_uebernehmen(sid,rolle,&revision,&schritte),
            Aktion::Bauen { planet, gebaeude } => self.bauen(sid, rolle, planet, gebaeude),
            Aktion::Reparieren { planet, gebaeude } => {
                self.reparieren(sid, rolle, planet, gebaeude)
            }
            Aktion::Abreissen { planet, gebaeude } => self.abreissen(sid, planet, gebaeude),
            Aktion::PlanetAufgeben { planet, bestaetigung } => self.planet_aufgeben(sid, planet, bestaetigung),
            Aktion::SystemAufklaeren { start, ziel, abbrechen } => self.system_aufklaeren(sid, rolle, start, ziel, abbrechen),
            Aktion::RaketenStorno{planet,auftrag}=>self.raketen_storno(sid,planet,auftrag),
            Aktion::MilizAufrufen {planet,anzahl} => self.miliz_aufrufen(sid,rolle,planet,anzahl),
            Aktion::MilizAusrufen {planet} => self.miliz_ausrufen(sid,rolle,planet),
            Aktion::MilizEntlassen {planet} => self.miliz_entlassen_befehl(sid,planet),
            Aktion::InvasionLinien {planet,linien} => self.linien_zuteilen(sid,planet,linien),
            Aktion::FlotteStarten {flotte,ziel,mission,geschwindigkeit,rueckgeschwindigkeit,haltedauer_stunden,zielflotte} => self.flotte_starten(sid,&serde_json::json!({
                "flotte":flotte,"ziel":ziel.map(|k|k.to_string()),"mission":mission,"geschwindigkeit":geschwindigkeit,"rueckgeschwindigkeit":rueckgeschwindigkeit,
                "haltedauer_stunden":haltedauer_stunden,"zielflotte":zielflotte})),
            Aktion::OrbitflotteAnbieten {flotte,ziel,preis} => self.orbitflotte_anbieten(sid,flotte,ziel,preis),
            Aktion::OrbitflotteAngreifen {start,zielflotte,schiffe,geschwindigkeit,rueckgeschwindigkeit} => self.orbitflotte_angreifen(sid,&serde_json::json!({
                "start":start.to_string(),"zielflotte":zielflotte,"schiffe":schiffe.iter().map(|(e,n)|(e.name().to_string(),serde_json::json!(n))).collect::<serde_json::Map<_,_>>(),
                "geschwindigkeit":geschwindigkeit,"rueckgeschwindigkeit":rueckgeschwindigkeit})),
            Aktion::BodenBauen {planet,art,anzahl} => self.boden_bauen_rolle(sid,rolle,planet,&art,anzahl),
            Aktion::BodenStorno {planet,auftrag} => {let pid=self.eigener_planet(sid,planet)?;let index=self.ausbau.boden.auftraege.get(&(pid as u32)).and_then(|q|q.iter().position(|a|a.id==auftrag)).ok_or("Kasernenauftrag existiert nicht mehr")?;self.boden_storno(sid,planet,index)},
            Aktion::FlottePlanen{flug,ankunft,rueckgeschwindigkeit}=>self.flotte_planen(sid,rolle,flug,ankunft,rueckgeschwindigkeit),
            Aktion::FlugplanStorno{auftrag}=>self.flugplan_storno(sid,auftrag),
            Aktion::BodenSenden {flug,truppen} => self.boden_senden(sid,rolle,&flug,truppen),
            Aktion::Produktionsserie {planet,art,produkt,ziel} => self.produktionsserie_setzen(sid,rolle,planet,&art,&produkt,ziel),
            Aktion::ProduktionSetzen {planet,gebaeude,prozent} => self.produktion_setzen(sid,planet,gebaeude,prozent),
            Aktion::AuftragPause { planet, art, pausiert } => self.auftrag_pause(sid, rolle, planet, &art, pausiert),
            Aktion::PlanetBenennen { planet, name } => self.planet_benennen(sid, planet, &name),
            Aktion::Soforthandel { planet, angebot, ziel } => self.soforthandel(sid, rolle, planet, &angebot, &ziel),
            Aktion::SchleifeLeeren { planet } => self.schleife_leeren(sid, planet),
            Aktion::WarteschlangeAendern{planet,art,index,ziel,loeschen,kennung}=>self.warteschlange_aendern(sid,rolle,&art,planet,index,ziel,loeschen,&kennung),
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
                siedler_volk,
                siedler_mischung,
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
                self.flotte_senden_mit_mischung(
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
                    siedler_volk,
                    siedler_mischung,
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
                zieltyp, art, flotte,
            } => if self.ausbau.version>0 {self.taktisch_starten(sid,start,ziel,anzahl,zieltyp,art.as_deref().unwrap_or("interplanetar"),flotte)} else {self.raketen_starten(sid,start,ziel,anzahl,zieltyp)},
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
            Aktion::AllianzEinladungAblehnen { allianz } => self.allianz_einladung_ablehnen(sid, &allianz),
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
            Aktion::TopfUmbuchen { von, nach, menge } => self.topf_umbuchen(sid, von, nach, menge_geprueft(menge, "menge")?),
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

/// Restrict unit identifiers to the active rule profile. The generic schema below
/// remains the complete protocol vocabulary for offline tooling.
pub fn antwortschema_fuer(r:&crate::Regelwerk,rolle:Rolle)->serde_json::Value {
    let mut schema=antwortschema(rolle);
    if r.dyson.is_some() {
        if let Some(a)=schema["properties"]["aktionen"]["items"]["anyOf"].as_array_mut(){a.extend(crate::dyson::dyson_aktionsschemas(rolle));}
    }
    if r.monde.is_some() {
        if let Some(a)=schema["properties"]["aktionen"]["items"]["anyOf"].as_array_mut(){a.extend(crate::monde::mond_aktionsschemas(rolle));}
        // flotte_senden (auch als Flugauftrag in Hüllen wie flotte_planen oder schiffangebot): start-/zielart planet oder
        // mond, als Pflichtfeld im strengen Stil. Bodentruppen und Personen fliegen nie zu Monden – dort nicht angeboten.
        fn mondart(v:&mut serde_json::Value,in_planethuelle:bool){
            let typ=v["properties"]["typ"]["enum"][0].as_str().unwrap_or("").to_string();
            let huelle=in_planethuelle||typ=="boden_senden"||typ=="personen_senden";
            if (typ=="flotte_senden"&&!huelle)||typ=="raketen_starten" {
                if let Some(p)=v["properties"].as_object_mut(){
                    let art=|wo:&str|serde_json::json!({"type":"string","enum":["planet","mond"],"description":format!("{wo}: planet (Standard) oder mond derselben Koordinate")});
                    p.insert("zielart".into(),art("Ziel"));
                    if typ=="flotte_senden" {p.insert("startart".into(),art("Start"));}
                }
                if let Some(req)=v["required"].as_array_mut(){
                    req.push(serde_json::json!("zielart"));
                    if typ=="flotte_senden" {req.push(serde_json::json!("startart"));}
                }
            }
            if let Some(o)=v.as_object_mut(){for (_,k) in o.iter_mut(){mondart(k,huelle);}}
            else if let Some(a)=v.as_array_mut(){for k in a{mondart(k,huelle);}}
        }
        mondart(&mut schema["properties"]["aktionen"],false);
        mondart(&mut schema["properties"]["abfragen"],false);
        mondart(&mut schema["$defs"],false);
    }
    // Terraformer (0.4.7): Mission terraformen und Werkzeug terraformbedarf nur mit dem Schiff im Regelwerk.
    if !r.einheiten.contains_key(&Einheit::Terraformer) {
        if let Some(a)=schema["properties"]["abfragen"]["items"]["anyOf"].as_array_mut(){a.retain(|v|v["properties"]["typ"]["enum"][0]!="terraformbedarf");}
    }
    fn filtern(v:&mut serde_json::Value,r:&crate::Regelwerk){
        let erlaubt=|s:&str|(s!="mond_zerstoeren"||r.monde.is_some()) && (s!="terraformen"||r.einheiten.contains_key(&Einheit::Terraformer)) && Einheit::aus_name(s).is_none_or(|e|r.einheiten.contains_key(&e))
            && Gebaeude::aus_name(s).is_none_or(|g|r.gebaeude.contains_key(&g))
            && Forschung::aus_name(s).is_none_or(|f|r.forschung.contains_key(&f));
        if let Some(o)=v.as_object_mut(){
            for key in ["enum","required"]{if let Some(a)=o.get_mut(key).and_then(serde_json::Value::as_array_mut){a.retain(|v|v.as_str().is_none_or(&erlaubt));}}
            if let Some(p)=o.get_mut("properties").and_then(serde_json::Value::as_object_mut){p.retain(|k,_|erlaubt(k));}
            for child in o.values_mut(){filtern(child,r);}
        }else if let Some(a)=v.as_array_mut(){for child in a{filtern(child,r);}}
    }
    filtern(&mut schema,r);schema
}

impl Welt {
    /// Antwortschema einer Rolle für genau diese Welt: Regelwerk (Einheiten, Monde, Dyson) und Weltschalter. Server,
    /// Harness, MCP und Browserteam nutzen nur diese Fassung, damit das Schema nie etwas anbietet, das der Kern in
    /// dieser Welt ablehnt (vorher z. B. Mission invasion unter Kolonisationsregeln v2: „bombardieren und
    /// kampfkolonisieren verwenden“).
    pub fn antwortschema(&self, rolle: Rolle) -> serde_json::Value {
        let mut schema = antwortschema_fuer(&self.regeln, rolle);
        let weg: &[&str] = if self.kolonisation.aktiv { &["invasion"] } else { &["bombardieren", "kampfkolonisieren"] };
        fn missionen_filtern(v: &mut serde_json::Value, weg: &[&str]) {
            if let Some(o) = v.as_object_mut() {
                if let Some(e) = o.get_mut("mission").and_then(|m| m.get_mut("enum")).and_then(serde_json::Value::as_array_mut) {
                    e.retain(|m| m.as_str().is_none_or(|m| !weg.contains(&m)));
                }
                for (_, k) in o.iter_mut() { missionen_filtern(k, weg); }
            } else if let Some(a) = v.as_array_mut() {
                for k in a { missionen_filtern(k, weg); }
            }
        }
        missionen_filtern(&mut schema, weg);
        // Mond- und Dysonaktionen nur, wenn sie in dieser Welt laufen (Regelwerk und Weltschalter).
        let monde = self.monde.aktiv;
        let dyson = self.ausbau.dyson.aktiv;
        let plaetze = if dyson { self.dyson_regeln().map(|r| r.anschluesse) } else { None };
        if let Some(a) = schema["properties"]["aktionen"]["items"]["anyOf"].as_array_mut() {
            a.retain(|v| {
                let typ = v["properties"]["typ"]["enum"][0].as_str().unwrap_or("");
                (monde || !crate::monde::MOND_AKTIONEN.contains(&typ)) && (dyson || !crate::dyson::DYSON_AKTIONEN.contains(&typ))
            });
            // Anschlussplätze der Dysonsphäre aus den Regeln dieser Welt (Schema sagt sonst 8, der Kern erlaubt z. B. 4).
            if let Some(n) = plaetze {
                for v in a.iter_mut() {
                    if let Some(s) = v["properties"].get_mut("slot") { s["maximum"] = serde_json::json!(n); }
                }
            }
        }
        schema
    }

    /// Aktionstypen, die das Antwortschema dieser Welt der Rolle anbietet (ohne Doppelungen, Schemareihenfolge).
    pub fn aktionstypen(&self, rolle: Rolle) -> Vec<String> {
        let schema = self.antwortschema(rolle);
        let mut out: Vec<String> = Vec::new();
        for v in schema["properties"]["aktionen"]["items"]["anyOf"].as_array().into_iter().flatten() {
            if let Some(t) = v["properties"]["typ"]["enum"][0].as_str() {
                if !out.iter().any(|x| x == t) { out.push(t.to_string()); }
            }
        }
        out
    }
}

/// Antwortschema ohne `$ref`: setzt die Definitionen aus `$defs` ein (für Prüfer, Kataloge und Werkzeuge, die keine
/// Verweise auflösen). Das Schema selbst trägt `$defs`, damit Flugaufträge nur einmal beschrieben sind.
pub fn schema_aufloesen(schema: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    fn ein(v: &Value, defs: &Value) -> Value {
        match v {
            Value::Object(o) => {
                if let Some(r) = o.get("$ref").and_then(Value::as_str).and_then(|r| r.strip_prefix("#/$defs/")) {
                    return ein(&defs[r], defs);
                }
                Value::Object(o.iter().filter(|(k, _)| k.as_str() != "$defs").map(|(k, x)| (k.clone(), ein(x, defs))).collect())
            }
            Value::Array(a) => Value::Array(a.iter().map(|x| ein(x, defs)).collect()),
            _ => v.clone(),
        }
    }
    ein(schema, &schema["$defs"])
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
                siedler_volk: None,
                siedler_mischung: None,
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
    // Flugaufträge in Hüllenaktionen (boden_senden, personen_senden, flotte_beladen, schiffangebot, expedition_starten,
    // rettung_starten, flotte_planen) sind vollständige flotte_senden-Aktionen mit typ. Vorher stand dort nur
    // {"type":"object"}; Modelle rieten die Felder, und jede Ablehnung kostete eine Korrekturrunde (Agentenzugang 26.10.2026).
    // Die Definition steht einmal unter $defs.flug: siebenfach eingebettet wuchs das Schema um 46 KB, und das Labor-Backend
    // überschritt damit sein Kontextbudget.
    let siedler_mischung = || json!({"type": ["array", "null"], "minItems": 3, "maxItems": 3, "items": {"type": "integer", "minimum": 0},
        "description": "nur kolonisieren: Siedler als [Bewohner, Fachkräfte, Forscher] in ganzen Personen, Summe = Siedlerplätze der Kolonieschiffe; null = der Kern wählt"});
    let tempo = || begrenzt("number", json!(0.1), Some(json!(1.0)), "Anteil der Höchstgeschwindigkeit, 0.1 bis 1.0");
    let flug_felder = |m: Vec<&'static str>| -> Vec<(&str, Value)> {
        vec![
            ("start", koord()),
            ("ziel", koord()),
            ("mission", namen(m)),
            ("siedler_volk", oder_null(Volk::ALLE.iter().map(|v| v.name()).collect())),
            ("siedler_mischung", siedler_mischung()),
            ("schiffe", schiffe()),
            ("geschwindigkeit", tempo()),
            ("ladung", alle_felder(fracht.clone(), "number")),
            ("haltedauer_stunden", ganzzahl()),
        ]
    };
    let flug_def = {
        let alle: Vec<&'static str> = Mission::ALLE.iter().filter(|m| **m != Mission::FlottenSpionage).map(|m| m.name()).collect();
        let mut o = objekt("flotte_senden", flug_felder(alle));
        o["description"] = json!("Flugauftrag einer Hüllenaktion: vollständige flotte_senden-Aktion mit typ. Die Mission muss zur Hülle passen (siehe deren Beschreibung) und für flotte_senden der Rolle erlaubt sein.");
        o
    };
    let flug = || json!({"$ref": "#/$defs/flug"});
    let huelle = |mut o: Value, zweck: &str| { o["description"] = json!(zweck); o };
    let voelker: Vec<&str> = Volk::ALLE.iter().map(|v| v.name()).collect();
    let personen = || json!({"type": "object", "additionalProperties": false, "required": ["gruppen"],
        "description": format!("Reisende in Tausendstel-Personen (1000 = eine Person, nur Vielfache von 1000): gruppen[volk][0 Bewohner, 1 Fachkräfte, 2 Forscher], Völker in der Reihenfolge {}; Bestand in lage.planeten[].personengruppen", voelker.join(", ")),
        "properties": {"gruppen": {"type": "array", "minItems": 4, "maxItems": 4, "items": {"type": "array", "minItems": 3, "maxItems": 3, "items": {"type": "integer", "minimum": 0, "multipleOf": 1000}}}}});
    let truppen = |max: Option<i64>| {
        let mut s = json!({"type": "array", "minItems": crate::boden_katalog::ANZAHL, "maxItems": crate::boden_katalog::ANZAHL,
            "items": {"type": "integer", "minimum": 0},
            "description": format!("Bodeneinheiten je Art in der Reihenfolge {}; Bestand in lage.planeten[].bodentruppen.bestand", crate::boden::NAMEN.join(", "))});
        if let Some(m) = max { s["items"]["maximum"] = json!(m); }
        s
    };
    let hat = |typ: &str| erlaubte_typen(rolle).contains(&typ);
    // Warteschlangen und Pausen: der Feldherr verwaltet nur Werft und Bauteile (warteschlangen.rs, verwaltung.rs).
    let schlangen: Vec<&str> = if rolle == Rolle::Feldherr { vec!["werft", "bauteile"] } else { vec!["bau", "forschung", "werft", "bauteile"] };
    // Der Verwalter fertigt nur zivile Schiffe (wirtschaft.rs `fertigen`); das Schema bietet ihm nur diese an.
    let fertigbar: Vec<&str> = Einheit::ALLE.iter().filter(|e| rolle != Rolle::Verwalter || crate::wirtschaft::zivile_einheit(**e)).map(|e| e.name()).collect();
    let boden_huelle = || huelle(objekt("boden_senden", vec![("flug", flug()), ("truppen", truppen(Some(100_000)))]),
        "Bodentruppen im Flug: Truppentransporter in flug.schiffe; flug.mission transport, stationieren, kampfkolonisieren, kolonisieren, halten (zu Verbündeten verstärken) oder bombardieren (Invasion v2: Bomber, Eskorte und Transporter in einer Flotte, Landung bei 30 % Integrität)");
    let personen_huelle = || huelle(objekt("personen_senden", vec![("flug", flug()), ("personen", personen())]),
        "Personen zwischen Planeten: flug.mission transport oder stationieren, nie Monde");
    let beladen_huelle = || huelle(objekt("flotte_beladen", vec![("flug", flug()), ("personen", personen()),
        ("truppen", truppen(if rolle == Rolle::Verwalter { Some(0) } else { None }))]),
        "Flug mit Gütern, Personen und Truppen: mit Personen nur transport oder stationieren, mit Truppen wie boden_senden (nur Feldherr)");

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
                    ("einheit", namen(fertigbar.clone())),
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
            "raketen_storno"=>vec![("planet",koord()),("auftrag",json!({"type":"integer","minimum":0}))],
            "miliz_aufrufen"=>vec![("planet",koord()),("anzahl",json!({"type":"integer","minimum":1,"maximum":100000}))],
            "miliz_ausrufen"=>vec![("planet",koord())],
            "miliz_entlassen"=>vec![("planet",koord())],
            "flotte_starten"=>vec![("flotte",json!({"type":"integer","minimum":0})),("ziel",json!({"type":["string","null"],"description":"Zielkoordinate; null = Ort der zielflotte"})),("mission",json!({"type":"string","enum":["stationieren","transport","halten","spionage","angriff"]})),
                ("geschwindigkeit",json!({"type":"number","minimum":0.1,"maximum":1.0})),("rueckgeschwindigkeit",json!({"type":"number","minimum":0.0,"maximum":1.0})),
                ("haltedauer_stunden",json!({"type":"integer","minimum":0})),("zielflotte",json!({"type":["integer","null"],"minimum":0}))],
            "orbitflotte_anbieten"=>vec![("flotte",json!({"type":"integer","minimum":0})),("ziel",koord()),("preis",json!({"type":"number","minimum":0,"maximum":1000000000,"default":0}))],
            "orbitflotte_angreifen"=>vec![("start",koord()),("zielflotte",json!({"type":"integer","minimum":0})),("schiffe",schiffe()),
                ("geschwindigkeit",json!({"type":"number","minimum":0.1,"maximum":1.0})),("rueckgeschwindigkeit",json!({"type":"number","minimum":0.0,"maximum":1.0}))],
            "invasion_linien"=>vec![("planet",koord()),("linien",json!({"type":"array","maxItems":11,"description":"Zuteilung linien[Linie][Truppentyp, Index 12 = Miliz]; höchstens Wellenzahl − 1 Linien (neue Welten: 3), der Rest bildet den Rückhalt. Jeder Aufruf ersetzt die ganze Zuteilung","items":{"type":"array","maxItems":13,"items":{"type":"integer","minimum":0}}}))],
            "boden_bauen"=>vec![("planet",koord()),("art",namen(crate::boden::NAMEN.to_vec())),("anzahl",json!({"type":"integer","minimum":1,"maximum":1000}))],
            "boden_storno"=>vec![("planet",koord()),("auftrag",json!({"type":"integer","minimum":0}))],
            "boden_senden"=>{ aktionen.push(boden_huelle()); continue; },
            "flotte_planen"=>{
                // Geplant werden flotte_senden und die Hüllen, die die Rolle selbst senden darf (flugkoordination.rs).
                let mut arten=vec![flug()];
                if hat("boden_senden") { arten.push(boden_huelle()); }
                if hat("personen_senden") { arten.push(personen_huelle()); }
                if hat("flotte_beladen") { arten.push(beladen_huelle()); }
                vec![("flug",json!({"anyOf":arten})),("ankunft",json!({"type":["integer","null"],"minimum":0,"description":"Ankunft in Spielsekunden; null = sofort starten"})),
                    ("rueckgeschwindigkeit",json!({"type":"number","minimum":0.0,"maximum":1.0,"description":"Anteil der Höchstgeschwindigkeit für den Rückflug; 0 = kein Rückflug (nur Welten mit Orbitflotten)"}))]
            },
            "flugplan_storno"=>vec![("auftrag",json!({"type":"integer","minimum":0}))],
            "produktionsserie"=>vec![("planet",koord()),("art",namen(if rolle==Rolle::Verwalter {vec!["einheit","bauteil"]} else {vec!["einheit","bauteil","rakete","boden"]})),("produkt",json!({"type":"string","maxLength":64})),("ziel",json!({"type":"integer","minimum":0,"maximum":10000}))],
            "produktion_setzen"=>vec![("planet",koord()),("gebaeude",namen(crate::industrie::PRODUKTION.iter().map(|g|g.name()).collect())),("prozent",json!({"type":"integer","minimum":0,"maximum":120,"multipleOf":10}))],
            "ausbilden"=>vec![("planet",koord()),("volk",namen(vec!["aurelianer","krath","veyari","syntheten"])),("qualifikation",json!({"type":"integer","minimum":1,"maximum":2})),("anzahl",json!({"type":"integer","minimum":1,"maximum":1000}))],
            "personen_senden"=>{ aktionen.push(personen_huelle()); continue; },
            "flotte_beladen"=>{ aktionen.push(beladen_huelle()); continue; },
            // Der Kern plant die Übergabe als „alle“ (schiffhandel.rs): Mission ist immer transport, unabhängig von der Rolle.
            "schiffangebot"=>vec![("flug",flug()),
                ("personen",personen()),("truppen",truppen(None)),
                ("preis",json!({"type":"number","minimum":0,"maximum":1000000000,"description":"Credits, die der Empfänger bei Annahme zahlt; 0 = Geschenk"}))],
            "schiffangebot_antwort"=>vec![("angebot",json!({"type":"integer","minimum":1})),("annehmen",json!({"type":"boolean"}))],
            "expedition_starten"=>vec![("flug",flug())],
            "expedition_reparieren"=>vec![("flotte",ganzzahl()),("einheit",namen(einheit.clone()))],
            "rettung_starten"=>vec![("expedition",ganzzahl()),("flug",flug())],
            "plan_uebernehmen" => vec![("revision",json!({"type":"string"})),("schritte",json!({"type":"array","minItems":1,"maxItems":32,"items":{"type":"object"}}))],
            "bauen" | "abreissen" | "reparieren" => {
                vec![("planet", koord()), ("gebaeude", namen(gebaeude.clone()))]
            }
            "schleife_leeren" => vec![("planet", koord())],
            "planet_aufgeben" => vec![("planet", koord()), ("bestaetigung", koord())],
            "system_aufklaeren" => vec![("start", koord()), ("ziel", koord()), ("abbrechen", json!({"type":"boolean"}))],
            "planet_benennen" => vec![("planet", koord()), ("name", json!({"type":"string","maxLength":crate::verwaltung::PLANETENNAME_MAX,"description":"leer = Standardname"}))],
            "auftrag_pause" => vec![("planet", koord()), ("art", namen(schlangen.clone())), ("pausiert", json!({"type":"boolean"}))],
            "soforthandel" => vec![("planet", koord()), ("angebot", json!({"type":"object","additionalProperties":{"type":"number","minimum":0}})), ("ziel", namen(vec!["erz","kristall","deuterium","nahrung","credits"]))],
            "warteschlange_aendern"=>vec![("planet",koord()),("art",namen(schlangen.clone())),("index",json!({"type":"integer","minimum":0,"maximum":10})),("ziel",json!({"type":["integer","null"],"minimum":0,"maximum":10})),("loeschen",json!({"type":"boolean"})),("kennung",json!({"type":"string","minLength":64,"maxLength":64}))],
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
            "flotte_senden" => flug_felder(missionen.clone()),
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
                ("art", namen(vec!["abfang", "interplanetar", "flottenrakete", "bunkerbrecher", "hyperrakete"])),
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
                ("art",json!({"type":["string","null"],"enum":[null,"interplanetar","flottenrakete","bunkerbrecher","hyperrakete"]})),
                ("flotte",json!({"type":["integer","null"],"minimum":0})),
                ("start", koord()),
                ("ziel", koord()),
                (
                    "anzahl",
                    begrenzt(
                        "integer",
                        json!(1),
                        None,
                        "mindestens 1, höchstens der fertige Bestand der gewählten Raketenart",
                    ),
                ),
                (
                    "zieltyp",
                    namen(
                        Einheit::ALLE
                            .iter()
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
            "allianz_beitreten" | "allianz_einladung_ablehnen" => vec![("allianz", text())],
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
            "topf_umbuchen" => vec![
                ("von", namen(Topf::ALLE.iter().map(|t| t.name()).collect())),
                ("nach", namen(Topf::ALLE.iter().map(|t| t.name()).collect())),
                ("menge", json!({"type": "number", "exclusiveMinimum": 0, "description": "Werteinheiten wie in lage.toepfe.*.guthaben; höchstens das Guthaben von von"})),
            ],
            "meldung" => vec![("text", text())],
            "schenken" => vec![("an", text()), ("credits", zahl())],
            _ => Vec::new(),
        };
        aktionen.push(objekt(typ, felder));
    }
    for v in aktionen.iter_mut() {
        let zweck = match v["properties"]["typ"]["enum"][0].as_str() {
            Some("schiffangebot") => "Schiffe samt Ladung an einen anderen Spieler übergeben: flug.mission transport zu seinem bewohnten Planeten (zielart mond: sein Mond); preis 0 = Geschenk",
            Some("expedition_starten") => "Expedition: flug.mission saven zu Position 0 eines Systems, mit Pathfindern (bis 20 Bordsonden je Pathfinder)",
            Some("rettung_starten") => "Rettung: flug.mission saven mit Bergungsschiffen zum Ziel der vermissten Expedition innerhalb der Rettungsfrist",
            Some("flotte_planen") => "Flug so starten, dass er zur Ankunftszeit eintrifft (höchstens sieben Spieltage im Voraus)",
            _ => continue,
        };
        v["description"] = json!(zweck);
    }
    let scenario_party=|defender:bool| {
        let units=Einheit::ALLE.iter().filter(|e|defender||e.ist_schiff()).map(|e|
            (e.name().to_string(),json!({"type":"integer","minimum":0,"maximum":crate::spielhilfe::SIMULATOR_MAX_EINHEITEN}))).collect::<serde_json::Map<_,_>>();
        let tech=["waffentechnik","schildtechnik","panzerung"].iter().map(|k|
            (k.to_string(),json!({"type":"integer","minimum":0,"maximum":crate::regeln::MAX_FORSCHUNG}))).collect::<serde_json::Map<_,_>>();
        json!({"type":"object","properties":{"volk":namen(Volk::ALLE.iter().map(|v|v.name()).collect()),
            "technik":{"type":"object","properties":tech,"required":["waffentechnik","schildtechnik","panzerung"],"additionalProperties":false},
            "einheiten":{"type":"object","properties":units,"additionalProperties":false}},
            "required":["volk","technik","einheiten"],"additionalProperties":false})
    };
    let kostenart=|art:&str,felder:Vec<(&str,Value)>| {
        let mut v=objekt(art,felder);
        let props=v["properties"].as_object_mut().unwrap();
        let typ=props.remove("typ").unwrap();props.insert("art".into(),typ);
        v["required"][0]=json!("art");v
    };
    let von=||json!({"type":"integer","minimum":0,"maximum":254});
    let bis=||json!({"type":"integer","minimum":1,"maximum":255});
    let anzahl=||json!({"type":"integer","minimum":1,"maximum":10000});
    let kostenpositionen=json!({"type":"array","minItems":1,"maxItems":32,"items":{"anyOf":[
        kostenart("gebaeude",vec![("gebaeude",namen(gebaeude.clone())),("von",von()),("bis",bis())]),
        kostenart("forschung",vec![("forschung",namen(forschung.clone())),("von",von()),("bis",bis())]),
        kostenart("einheit",vec![("einheit",namen(einheit.clone())),("anzahl",anzahl())]),
        kostenart("bauteil",vec![("bauteil",namen(vec!["antriebskern","habitatmodul"])),("anzahl",anzahl())])
    ]}});
    let abfragen = vec![
        objekt("flugkoordination",vec![("flug",flug()),("ankunft",json!({"type":["integer","null"],"minimum":0})),("rueckgeschwindigkeit",json!({"type":"number","minimum":0.1,"maximum":1.0}))]),
        objekt("missionsplan",vec![("aktion",json!({"type":"object"}))]),
        objekt("raketenplan",vec![("aktion",json!({"type":"object"}))]),
        objekt("marktplan",vec![("aktion",json!({"type":"object"}))]),
        objekt("ablaufplan",vec![("plan",json!({"type":"object","properties":{"schritte":{"type":"array","maxItems":32,"items":{"type":"object"}},"horizont_stunden":{"type":"integer","minimum":1,"maximum":168},"rolle":{"type":"string","enum":["alle","verwalter","feldherr","stratege"]}},"required":["schritte","horizont_stunden"],"additionalProperties":false}))]),
        objekt("regelkontext",vec![]),
        objekt("wirtschaftsdiagnose",vec![("planet",koord())]),
        objekt("habitabilitaet",vec![("planet",koord())]),
        objekt("koloniebedarf",vec![("volk",oder_null(Volk::ALLE.iter().map(|v|v.name()).collect())),("einheit",oder_null(vec!["kolonieschiff","grosses_kolonieschiff"]))]),
        objekt("terraformbedarf",vec![("koord",koord()),("volk",oder_null(Volk::ALLE.iter().map(|v|v.name()).collect()))]),
        objekt("kostenplan",vec![("planet",koord()),("positionen",kostenpositionen)]),
        objekt(
            "kosten",
            vec![
                ("gebaeude", oder_null(gebaeude.clone())),
                ("forschung", oder_null(forschung.clone())),
                ("einheit", oder_null(einheit.clone())),
                ("rakete", oder_null(vec!["abfang", "interplanetar", "flottenrakete", "bunkerbrecher", "hyperrakete"])),
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
                ("flotte", json!({"type": ["integer", "null"], "description": "eigene gestrandete Flotte als Startort (dann gelten ihre Schiffe; start/schiffe werden ignoriert); sonst null"})),
            ],
        ),
        objekt(
            "kolonieplan",
            vec![("siedler_volk",oder_null(Volk::ALLE.iter().map(|v|v.name()).collect())),
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
            vec![("ziel", koord()), ("schiffe", schiffe()),
                ("zielart", json!({"type": "string", "enum": ["planet", "mond"], "description": "mond = gegen den letzten Mondspionagebericht"}))],
        ),
        objekt("kampfszenario",vec![("angreifer",scenario_party(false)),("verteidiger",scenario_party(true))]),
        objekt("briefarchiv", vec![("vor_id",ganzzahl()),("limit",ganzzahl()),("kanal",namen(vec!["alle","privat","allianz","diplomatie"]))]),
        objekt("regel", vec![("stichwort", text())]),
        // Vorschauen, die die Oberfläche nutzt; seit dem Agentenzugang (26.10.2026) auch im Schema, damit MCP-Clients sie
        // mit gültigen Feldern senden.
        objekt("planet_aufgeben", vec![("planet", koord())]),
        objekt("soforthandel", vec![("planet", koord()), ("angebot", json!({"type":"object","additionalProperties":{"type":"number","minimum":0},"description":"Gut → Menge, die abgegeben wird"})),
            ("ziel", namen(vec!["erz","kristall","deuterium","nahrung","credits"]))]),
        objekt("wiederaufbau", vec![("ziel", koord())]),
        objekt("raketensimulator", vec![
            ("art", namen(vec!["interplanetar", "flottenrakete", "bunkerbrecher", "hyperrakete"])),
            ("anzahl", json!({"type":"integer","minimum":1,"maximum":100000})),
            ("zieltyp", namen(Einheit::ALLE.iter().map(|e| e.name()).collect())),
            ("ziel_anzahl", json!({"type":"integer","minimum":0,"maximum":10000000})),
            ("angreifer", json!({"type":"object","additionalProperties":false,"required":["waffentechnik","volk"],"properties":{
                "waffentechnik":{"type":"integer","minimum":0,"maximum":30},"volk":namen(Volk::ALLE.iter().map(|v|v.name()).collect())}})),
            ("verteidiger", json!({"type":"object","additionalProperties":false,"required":["panzerung","volk","silo_intakt","abfangraketen"],"properties":{
                "panzerung":{"type":"integer","minimum":0,"maximum":30},"volk":namen(Volk::ALLE.iter().map(|v|v.name()).collect()),
                "silo_intakt":{"type":"boolean"},"abfangraketen":{"type":"integer","minimum":0,"maximum":1000000}}})),
        ]),
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
        "$defs": {"flug": flug_def},
    })
}
