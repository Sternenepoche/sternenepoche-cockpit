//! Kernwerte (ConfigWelt, 21.10.2026; Karl: „ich brauche mehr Kontrolle und Übersicht“): Spielwerte, die bis dahin
//! fest im Kern standen – Klima- und Wachstumstabelle je Volk und Platz, Solarkurve und Strahlkraft der Sternklassen,
//! Produktionsausbau, Bodentruppen samt Kontermatrix, Belagerung und Volksmiliz, Trümmer der Mondzerstörung, Zielwahl
//! im Kampf, Soforthandel, die Industrie-Rezepte neuer Welten und die beiden Expeditionsschiffe.
//!
//! Regelblock `regeln/kernwerte.ron`; wirksam nur in Welten, die `Welt::technik_aktivieren` bei Spielbeginn aufrufen
//! (Server, lokale Online-Partie, App). Die Werte liegen in `ausbau.technik.kernwerte` (JSON-Hülle, nur geschrieben,
//! wenn vorhanden) und gespiegelt in `regeln.technik`. Laufende Welten lesen das Feld leer und rechnen mit
//! [`KernWerte::default`] – das sind genau die bisherigen Konstanten (`habitabilitaet::WACHSTUM_V1`, `SOLAR_V2`,
//! `boden_katalog::*`, `invasion::MILIZ_*`, `monde::TRUEMMER_JE_KM` …), die als Grundwerte erhalten bleiben.
//! Keine neuen Felder in Regel-Structs des Binärteils.
use crate::regeln::{EinheitRegel, Preis};
use crate::typen::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const REGELN_RON: &str = include_str!("../../../regeln/kernwerte.ron");

// ---------------------------------------------------------------- Klima und Wachstum

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct KlimaWerte {
    /// Wachstumsfaktor je Volk und Platz (Bahn 1–12) in Promille des Grundwerts `wirtschaft.wachstum_je_tag`
    /// (1000 = Grundwert, Klimaoptimum). Neue Welten (Ablauf ≥ 1).
    pub wachstum: BTreeMap<Volk, Vec<i64>>,
    /// Nötige Terraforming-Stufe je Volk und Platz; 0 = sofort bewohnbar.
    pub freigabe: BTreeMap<Volk, Vec<u8>>,
    /// Rest der Klimaabweichung bei voller Terraforming-Technik in Promille (100 = 10 % bleiben).
    pub restklima_promille: i64,
    /// Terraforming-Stufe, ab der nur noch der Rest bleibt.
    pub endspiel_tech: u8,
    /// Bewohnerverlust je Stunde auf unbewohnbaren Bahnen bei fehlender Technik, in Promille.
    pub klimaverlust_promille: i64,
    /// Wachstum auf Xeno-Planeten (natürlicher Xenokristall) in Promille des sonst üblichen.
    pub xeno_wachstum_promille: i64,
}

impl Default for KlimaWerte {
    fn default() -> Self {
        let h = &crate::habitabilitaet::WACHSTUM_V1;
        let f = &crate::habitabilitaet::FREIGABE_V1;
        KlimaWerte {
            wachstum: Volk::ALLE.iter().map(|v| (*v, h[v.idx()].to_vec())).collect(),
            freigabe: Volk::ALLE.iter().map(|v| (*v, f[v.idx()].to_vec())).collect(),
            restklima_promille: crate::habitabilitaet::RESTKLIMA,
            endspiel_tech: crate::habitabilitaet::ENDSPIEL_TECH,
            klimaverlust_promille: crate::habitabilitaet::KLIMAVERLUST,
            xeno_wachstum_promille: 700,
        }
    }
}

impl KlimaWerte {
    pub fn tabelle(&self, v: Volk) -> [i64; 12] {
        self.wachstum.get(&v).and_then(|t| <[i64; 12]>::try_from(t.as_slice()).ok()).unwrap_or(crate::habitabilitaet::WACHSTUM_V1[v.idx()])
    }
    pub fn freigabe(&self, v: Volk) -> [u8; 12] {
        self.freigabe.get(&v).and_then(|t| <[u8; 12]>::try_from(t.as_slice()).ok()).unwrap_or(crate::habitabilitaet::FREIGABE_V1[v.idx()])
    }
}

// ---------------------------------------------------------------- Sonnen

/// Sternklasse eines Systems – dieselbe Würfelung wie die 3D-Ansicht (`web-client/galaxy.js` `sternklasse`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sternklasse { G, K, M, F, A }

impl Sternklasse {
    pub const ALLE: [Sternklasse; 5] = [Sternklasse::G, Sternklasse::K, Sternklasse::M, Sternklasse::F, Sternklasse::A];
    /// Radius in der 3D-Ansicht (galaxy.js `KLASSEN[].r`); Bezug für „Strahlkraft nach Größe“ ist der Gelbe Zwerg.
    pub fn radius(self) -> f64 {
        match self { Sternklasse::G => 9.5, Sternklasse::K => 9.0, Sternklasse::M => 8.0, Sternklasse::F => 10.0, Sternklasse::A => 10.5 }
    }
    pub fn name(self) -> &'static str {
        match self {
            Sternklasse::G => "Gelber Zwerg", Sternklasse::K => "Oranger Zwerg", Sternklasse::M => "Roter Zwerg",
            Sternklasse::F => "Weißgelber Stern", Sternklasse::A => "Weißer Stern",
        }
    }
    /// Anteil der Systeme dieser Klasse (Würfelgrenzen der 3D-Ansicht: 40 / 25 / 15 / 12 / 8 Prozent).
    pub fn anteil(self) -> f64 {
        match self { Sternklasse::G => 0.40, Sternklasse::K => 0.25, Sternklasse::M => 0.15, Sternklasse::F => 0.12, Sternklasse::A => 0.08 }
    }
    /// Klasse des Systems `sektor:system` (FNV-1a über „s:n:stern“ wie `PlanetKugel.streuung`).
    pub fn von(sektor: u8, system: u8) -> Sternklasse {
        let mut h: u32 = 2_166_136_261;
        for b in format!("{sektor}:{system}:stern").bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(16_777_619);
        }
        let x = h % 1000;
        if x < 400 { Sternklasse::G } else if x < 650 { Sternklasse::K } else if x < 800 { Sternklasse::M } else if x < 920 { Sternklasse::F } else { Sternklasse::A }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SternWerte {
    /// Strahlkraft der Klasse (1.0 = wie bisher); multipliziert Solarkraftwerke und Solarsonden im System.
    pub strahlkraft: f64,
}

impl Default for SternWerte {
    fn default() -> Self { SternWerte { strahlkraft: 1.0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SonnenWerte {
    /// Sonnenfaktor je Platz (Bahn 1–12) in Promille (1000 = Grundertrag des Solarkraftwerks). Neue Welten (Ablauf ≥ 2).
    pub solar_promille: Vec<i64>,
    /// Strahlkraft je Sternklasse.
    pub klassen: BTreeMap<Sternklasse, SternWerte>,
    /// Strahlkraft zusätzlich nach Größe: × (Radius / Radius des Gelben Zwergs) ^ Exponent. 0 = Größe ohne Wirkung,
    /// 2 = proportional zur Sternoberfläche.
    pub groessen_exponent: f64,
    /// Solarleistung fremder Planeten und Mondsatelliten in einem System mit vollendeter Dysonsphäre (Dysonschatten).
    pub dyson_schatten: f64,
}

impl Default for SonnenWerte {
    fn default() -> Self {
        SonnenWerte {
            solar_promille: crate::habitabilitaet::SOLAR_V2.to_vec(),
            klassen: Sternklasse::ALLE.iter().map(|k| (*k, SternWerte::default())).collect(),
            groessen_exponent: 0.0,
            dyson_schatten: 0.5,
        }
    }
}

impl SonnenWerte {
    /// Wirksame Strahlkraft einer Sternklasse in Promille.
    /// Sonnenfaktor einer Bahn (0–11) in Promille.
    pub fn solar(&self, bahn: usize) -> i64 {
        self.solar_promille.get(bahn).copied().unwrap_or(crate::habitabilitaet::SOLAR_V2[bahn.min(11)])
    }
    pub fn stern_promille(&self, k: Sternklasse) -> i64 {
        let s = self.klassen.get(&k).map(|w| w.strahlkraft).unwrap_or(1.0);
        let g = if self.groessen_exponent == 0.0 { 1.0 } else { (k.radius() / Sternklasse::G.radius()).powf(self.groessen_exponent) };
        milli(s * g)
    }
}

// ---------------------------------------------------------------- Produktion

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ProduktionsWerte {
    /// Faktor auf Minen, Synthesizer, Farm, Gießerei, Elektronik-, Konsumgüterwerk und Xenoextraktor (neue Welten).
    pub produktionsausbau: f64,
    /// Untergrenze des Arbeiterfaktors durch Automatisierung (0.5 = höchstens −50 %).
    pub automatisierung_untergrenze: f64,
    /// Ausbeute des Xenoextraktors ohne natürliches Vorkommen (Synthese).
    pub xeno_synthese_ausbeute: f64,
    /// Einsatz je Xenokristall aus Synthese.
    pub xeno_synthese_rezept: BTreeMap<Gut, f64>,
    /// Stromfaktor des Xenoextraktors im Synthesebetrieb.
    pub xeno_synthese_energie: f64,
    /// Gewicht der Legierung in der Baumasse (Bauzeit = (Erz + Kristall + Gewicht × Legierung) × 3600 / Teiler).
    pub baumasse_legierung: i64,
}

impl Default for ProduktionsWerte {
    fn default() -> Self {
        ProduktionsWerte {
            produktionsausbau: 2.0,
            automatisierung_untergrenze: 0.5,
            xeno_synthese_ausbeute: 0.65,
            xeno_synthese_rezept: BTreeMap::from([(Gut::Deuterium, 12.0), (Gut::Kristall, 20.0), (Gut::Legierung, 8.0), (Gut::Elektronik, 4.0)]),
            xeno_synthese_energie: 4.0,
            baumasse_legierung: 2,
        }
    }
}

impl ProduktionsWerte {
    pub fn rezept(&self) -> [(Gut, f64); 4] {
        let n = |g: Gut| self.xeno_synthese_rezept.get(&g).copied().unwrap_or(0.0);
        [(Gut::Deuterium, n(Gut::Deuterium)), (Gut::Kristall, n(Gut::Kristall)), (Gut::Legierung, n(Gut::Legierung)), (Gut::Elektronik, n(Gut::Elektronik))]
    }
}

// ---------------------------------------------------------------- Bodentruppen, Belagerung, Volksmiliz

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BodenTyp {
    /// Truppenplätze im Truppentransporter je Stück.
    pub plaetze: i64,
    /// Belegte Fertigungsplätze der Kaserne je Stück (Kasernenstufe = Plätze).
    pub fertigungsplaetze: i64,
    /// Rekruten (Einwohner) je Stück.
    pub personal: i64,
    /// Grunddauer der Ausbildung in Sekunden.
    pub dauer_sekunden: i64,
    pub angriff: i64,
    pub struktur: i64,
    /// Nötige Kasernenstufe.
    pub kaserne: u8,
    /// Nötige Zivilisationsstufe.
    pub ab_stufe: u8,
    /// Kosten je Stück in ganzen Einheiten.
    pub kosten: Preis,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BodenWerte {
    pub typen: BTreeMap<String, BodenTyp>,
    /// Trefferfaktor in Promille je Angreifer und Ziel (fehlt ein Eintrag: 1000). Ziele: die zwölf Typen und „miliz“.
    pub konter: BTreeMap<String, BTreeMap<String, i64>>,
    /// Truppenplätze je Truppentransporter.
    pub transporter_plaetze: i64,
    /// Koordination: +Promille Schaden je Aufklärungsdrohne, höchstens für so viele Drohnen.
    pub koordination_je_drohne_promille: i64,
    pub koordination_max_drohnen: i64,
    /// Reparierte Struktur je Reparaturdrohne und Runde.
    pub reparatur_je_drohne: i64,
    /// Volksmiliz (Invasion v1): Waffenwert je Bewohner in Promille, Rüstwert (Struktur) je Bewohner.
    pub miliz_waffen_promille: i64,
    pub miliz_ruestung: i64,
    /// Kapitulation, sobald dieser Anteil des Aufgebots gefallen ist (Promille).
    pub miliz_kapitulation_promille: i64,
    /// Anteil jeder Gruppe, der zum Dienst eingezogen wird (Promille).
    pub miliz_dienst_promille: i64,
    /// Kosten des Aufgebots: Legierung je Bewohner.
    pub miliz_legierung_je_bewohner: i64,
    /// Belagerung bis zur frühesten Übernahme in Stunden.
    pub belagerung_stunden: i64,
    /// Bodenrunde und Bombardement-Takt in Minuten.
    pub runde_minuten: i64,
    /// Bombardement: Strukturpunkte je Gebäudestufe, Bombenlast je Bombenschacht, Untergrenze der Integrität (Promille).
    pub bombardement_struktur_je_stufe: i64,
    pub bombenlast_je_schacht: i64,
    pub integritaet_grenze_promille: i64,
    /// 0.4.8 (Karl 24.10.2026): Anteil der Bewohner je Volk, der in der Miliz kämpft (Promille; fehlt ein Volk oder ist die
    /// Liste leer: alle Bewohner wie bisher). Krath 1000, Veyari 900, Syntheten 800, Aurelianer 700.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub miliz_anteil_promille: BTreeMap<String, i64>,
    /// Miliz auch ohne laufende Belagerung ausrufen (Vorbereitung) und jederzeit entlassen; Standard nur in der Belagerung.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub miliz_jederzeit: bool,
    /// Reparatur in Promille der Baukosten der *aktuellen* Stufe bei voll heruntergebombtem Gebäude (anteilig zum Schaden);
    /// 0 = bisherige Formel (beschädigter Anteil der kumulierten Baukosten aller Stufen).
    #[serde(skip_serializing_if = "ist_null")]
    pub reparatur_stufe_promille: i64,
    /// Invasion v2 (Karl 24.10.2026): Zahl der Verteidigungslinien (die letzte ist der Rückhalt); 0 = Invasion v1
    /// (48-Stunden-Belagerung mit Bodenrunden alle 15 min).
    #[serde(skip_serializing_if = "ist_null")]
    pub wellen: i64,
    /// Abstand der Angriffswellen in Spielstunden (Standard 12 → vier Linien in 48 Stunden).
    #[serde(skip_serializing_if = "ist_12")]
    pub wellen_abstand_stunden: i64,
    /// Höchstens so viele Bodenrunden je Welle; danach hält die Linie bis zur nächsten Welle.
    #[serde(skip_serializing_if = "ist_6")]
    pub wellen_runden_max: i64,
    /// Nötige Forschung für die Miliz (neben Kaserne 1); leer = keine.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub miliz_forschung: BTreeMap<Forschung, u8>,
    /// Ausbildungszeit der Miliz in Spielsekunden je 1.000 Milizionäre (bei Tempo 1 = Echtzeit); 0 = sofort.
    #[serde(skip_serializing_if = "ist_null")]
    pub miliz_ausbildung_sekunden_je_1000: i64,
    /// Ausbildungstempo der Kaserne während einer Invasion in Promille (500 = halb so schnell).
    #[serde(skip_serializing_if = "ist_1000")]
    pub invasion_ausbildung_promille: i64,
}

impl Default for BodenWerte {
    fn default() -> Self {
        use crate::boden_katalog as b;
        let typen = (0..b::ANZAHL).map(|i| {
            let k = b::kosten(i);
            let kosten: Preis = [Gut::Legierung, Gut::Elektronik, Gut::Deuterium].into_iter()
                .filter(|g| k[g.idx()] > 0).map(|g| (g, k[g.idx()] / M)).collect();
            (b::NAMEN[i].to_string(), BodenTyp {
                plaetze: b::PLAETZE[i], fertigungsplaetze: b::FERTIGUNGSPLAETZE[i], personal: b::PERSONAL[i],
                dauer_sekunden: b::DAUER[i], angriff: b::ANGRIFF[i], struktur: b::STRUKTUR[i], kaserne: b::KASERNE[i],
                ab_stufe: b::EPOCHE[i], kosten,
            })
        }).collect();
        let mut konter: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
        for a in 0..b::ANZAHL {
            for d in 0..b::KAMPFZAHL {
                let f = b::faktor(a, d);
                if f != 1000 { konter.entry(b::NAMEN[a].into()).or_default().insert(crate::boden_kampf::name(d).into(), f); }
            }
        }
        BodenWerte {
            typen, konter, transporter_plaetze: 100, koordination_je_drohne_promille: 100, koordination_max_drohnen: 3,
            reparatur_je_drohne: 25,
            miliz_waffen_promille: crate::invasion::MILIZ_WAFFEN_PROMILLE, miliz_ruestung: crate::invasion::MILIZ_RUESTUNG,
            miliz_kapitulation_promille: crate::invasion::MILIZ_KAPITULATION_PROMILLE,
            miliz_dienst_promille: crate::invasion::MILIZ_DIENST_PROMILLE,
            miliz_legierung_je_bewohner: crate::invasion::MILIZ_LEGIERUNG_JE_BEWOHNER,
            belagerung_stunden: crate::invasion::BELAGERUNG / STUNDE, runde_minuten: crate::invasion::RUNDE / MINUTE,
            bombardement_struktur_je_stufe: crate::invasion::STRUKTUR_JE_STUFE,
            bombenlast_je_schacht: crate::invasion::BOMBENLAST_JE_SCHACHT,
            integritaet_grenze_promille: crate::invasion::INTEGRITAET_GRENZE,
            miliz_anteil_promille: BTreeMap::new(),
            miliz_jederzeit: false,
            reparatur_stufe_promille: 0,
            wellen: 0,
            wellen_abstand_stunden: 12,
            wellen_runden_max: 6,
            miliz_forschung: BTreeMap::new(),
            miliz_ausbildung_sekunden_je_1000: 0,
            invasion_ausbildung_promille: 1000,
        }
    }
}

impl BodenWerte {
    /// Werte eines Typs nach Index (Reihenfolge `boden_katalog::NAMEN`).
    pub fn typ(&self, i: usize) -> &BodenTyp {
        self.typen.get(crate::boden_katalog::NAMEN[i]).unwrap_or_else(|| &standard().boden.typen[crate::boden_katalog::NAMEN[i]])
    }
    pub fn plaetze(&self, i: usize) -> i64 { self.typ(i).plaetze }
    pub fn fertigungsplaetze(&self, i: usize) -> i64 { self.typ(i).fertigungsplaetze.max(1) }
    pub fn personal(&self, i: usize) -> i64 { self.typ(i).personal }
    pub fn dauer(&self, i: usize) -> i64 { self.typ(i).dauer_sekunden }
    pub fn angriff(&self, i: usize) -> i64 { self.typ(i).angriff }
    pub fn struktur(&self, i: usize) -> i64 { self.typ(i).struktur.max(1) }
    pub fn kaserne(&self, i: usize) -> u8 { self.typ(i).kaserne }
    pub fn ab_stufe(&self, i: usize) -> u8 { self.typ(i).ab_stufe }
    /// Kosten je Stück in Tausendsteln.
    pub fn kosten(&self, i: usize) -> [i64; GUETER] { crate::regeln::preis_array(&self.typ(i).kosten) }
    /// Trefferfaktor in Promille (Kampfindex: 0–11 Typen, 12 Miliz).
    pub fn faktor(&self, a: usize, d: usize) -> i64 {
        if a >= crate::boden_katalog::ANZAHL { return 1000; }
        self.konter.get(crate::boden_katalog::NAMEN[a]).and_then(|m| m.get(crate::boden_kampf::name(d))).copied().unwrap_or(1000)
    }
    pub fn spalte<T>(&self, f: impl Fn(&BodenTyp) -> T) -> Vec<T> {
        (0..crate::boden_katalog::ANZAHL).map(|i| f(self.typ(i))).collect()
    }
    pub fn runde(&self) -> i64 { self.runde_minuten.max(1) * MINUTE }
    pub fn belagerung(&self) -> i64 { self.belagerung_stunden.max(0) * STUNDE }
}

// ---------------------------------------------------------------- Flug

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FlugWerte {
    /// Flugzeit = max(flug.min_sekunden, flug.zeitfaktor × (Grundzeit + Zeitkonstante / s × √(10 × d / v))) Sekunden;
    /// d Entfernung, v Tempo des langsamsten Schiffs, s Geschwindigkeitsstufe 0,1–1,0.
    pub grundzeit: i64,
    pub zeitkonstante: i64,
    /// 0.4.8 (Karl 24.10.2026): „kein Rückflug“ und gestrandete Flotten (Start vom Orbit mit Deuterium aus der Ladung,
    /// sichtbar und angreifbar, Mitkämpfen am Planeten). Standard aus = laufende Welten unverändert.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub orbitflotten: bool,
}

impl Default for FlugWerte {
    fn default() -> Self { FlugWerte { grundzeit: 10, zeitkonstante: 350, orbitflotten: false } }
}

// ---------------------------------------------------------------- Mond, Kampf, Handel

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MondWerte {
    /// Grundanteil des Trümmerrings je Kilometer Monddurchmesser (Erz, Kristall).
    pub truemmer_je_km_erz: i64,
    pub truemmer_je_km_kristall: i64,
    /// Anteil der Besatzung stationierter Schiffe, der die Zerstörung überlebt (Promille).
    pub ueberlebende_promille: i64,
    /// Zerstörungschance in Prozent = (Grenze − √Durchmesser) × √Weltenkrümmer; ab Durchmesser Grenze² keine Chance.
    pub zerstoerung_wurzel_grenze: i64,
    /// Verlustchance der Weltenkrümmer in Prozent = √Durchmesser × 10 / Teiler (Teiler 20 → √d / 2).
    pub verlust_teiler: i64,
}

impl Default for MondWerte {
    fn default() -> Self {
        MondWerte {
            truemmer_je_km_erz: crate::monde::TRUEMMER_JE_KM[0], truemmer_je_km_kristall: crate::monde::TRUEMMER_JE_KM[1],
            ueberlebende_promille: crate::monde::UEBERLEBENDE_PROMILLE, zerstoerung_wurzel_grenze: 100, verlust_teiler: 20,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct KampfWerte {
    /// Schadensfaktor gegen Primärziele und Sekundärziele (Zielwahl der Flottendoktrin).
    pub primaer_faktor: f64,
    pub sekundaer_faktor: f64,
    /// Mehrfachfeuer (Karl 24.10.2026, neue Welten): Schnellfeuer springt nach einem Abschuss auf ein neues Ziel derselben
    /// Art, statt mit dem Abschuss zu enden (`kampf::feuern_doktrin`). Standard aus = bisherige Rechnung; ohne Wert wird
    /// das Feld nicht geschrieben, damit Hülle und Hash laufender Welten gleich bleiben.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub schnellfeuer_springt: bool,
}

impl Default for KampfWerte {
    fn default() -> Self { KampfWerte { primaer_faktor: 2.0, sekundaer_faktor: 1.5, schnellfeuer_springt: false } }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HandelsWerte {
    /// Kurse des Soforthandels in Credits je Einheit (Erz, Kristall, Deuterium, Nahrung, Credits).
    pub soforthandel_kurse: BTreeMap<String, f64>,
    /// Gebühr des Soforthandels (0.02 = 2 %).
    pub soforthandel_gebuehr: f64,
    /// Marktgebühr mit Handelsabkommen (Faktor).
    pub handelsabkommen_faktor: f64,
    /// Rollenbudget einer Kauforder bei Storno, Planetenverlust und Preisdifferenz zurückbuchen (Funktionsprüfung
    /// 25.10.2026, handel#7/#14: bisher floss nur das Credit-Guthaben zurück, der Topf blieb belastet). Neue Welten;
    /// Standard aus = bisherige Rechnung, ohne Wert nicht geschrieben (Hülle laufender Welten bleibt gleich).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub topf_rueckbuchung: bool,
}

impl Default for HandelsWerte {
    fn default() -> Self {
        HandelsWerte {
            soforthandel_kurse: [("erz", 1.0), ("kristall", 1.5), ("deuterium", 3.0), ("nahrung", 0.5), ("credits", 1.0)]
                .into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            soforthandel_gebuehr: 0.02,
            handelsabkommen_faktor: 0.5,
            topf_rueckbuchung: false,
        }
    }
}

impl HandelsWerte {
    /// Kurs in Tausendsteln Credits (None: nicht im Soforthandel).
    pub fn kurs_milli(&self, key: &str) -> Option<i64> { self.soforthandel_kurse.get(key).map(|k| milli(*k)).filter(|k| *k > 0) }
}

// ---------------------------------------------------------------- Industrie neuer Welten, Expeditionsschiffe

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct IndustrieWerte {
    /// Kaserne ab Zivilisationsstufe.
    pub kaserne_ab_stufe: u8,
    /// Kolonieschiff neuer Welten: Kosten, Freischaltung, Werftstufe, Forschung (ersetzt die Werte der Flottendatei).
    pub kolonieschiff_kosten: Preis,
    pub kolonieschiff_ab_stufe: u8,
    pub kolonieschiff_werft: u8,
    /// Nötige Gebäude (neue Welten: keine Orbitalwerft).
    pub kolonieschiff_braucht: BTreeMap<Gebaeude, u8>,
    pub kolonieschiff_forschung: BTreeMap<Forschung, u8>,
    /// Bauteil Habitatmodul (Orbitalwerft).
    pub habitatmodul_kosten: Preis,
}

impl Default for IndustrieWerte {
    fn default() -> Self {
        IndustrieWerte {
            kaserne_ab_stufe: 3,
            kolonieschiff_kosten: BTreeMap::from([(Gut::Legierung, 1000), (Gut::Elektronik, 500), (Gut::Habitatmodul, 1)]),
            kolonieschiff_ab_stufe: 3,
            kolonieschiff_werft: 3,
            kolonieschiff_braucht: BTreeMap::new(),
            // Karl 25.10.2026: „ab Impulsantrieb 2 reicht ohne Astrophysik 1 für kleines Koloschiff“
            kolonieschiff_forschung: BTreeMap::from([(Forschung::Impulsantrieb, 2)]),
            habitatmodul_kosten: BTreeMap::from([(Gut::Legierung, 400), (Gut::Elektronik, 250)]),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpeditionsWerte {
    pub pathfinder: EinheitRegel,
    pub bergungsschiff: EinheitRegel,
    /// Zusätzliche Personenplätze des Bergungsschiffs.
    pub bergungsschiff_personenplaetze: i64,
    /// Forschung für beide Schiffe.
    pub forschung: BTreeMap<Forschung, u8>,
}

impl Default for ExpeditionsWerte {
    fn default() -> Self {
        ExpeditionsWerte {
            pathfinder: crate::ausbau::schiffsregel(Einheit::Pathfinder).clone(),
            bergungsschiff: crate::ausbau::schiffsregel(Einheit::Bergungsschiff).clone(),
            bergungsschiff_personenplaetze: 200,
            forschung: BTreeMap::from([(Forschung::Impulsantrieb, 2), (Forschung::Astrophysik, 2)]),
        }
    }
}

impl PartialEq for ExpeditionsWerte {
    fn eq(&self, o: &Self) -> bool {
        serde_json::to_value(self).ok() == serde_json::to_value(o).ok()
    }
}

// ---------------------------------------------------------------- Kolonieplätze und Terraformer (0.4.7)

/// Kolonieplätze (Karl 24.10.2026: „2 Kolonieplätze frei ab Start“, ab der 3. Kolonie je eine Stufe Terraforming).
/// Erlaubt = min(wirtschaft.kolonien_max, 14, frei + Stufe der Forschung `forschung`). Standard = bisheriges Verhalten:
/// keine freien Plätze, je Astrophysikstufe eine Kolonie (Welten vor Generation 12 weiter mit der Halbstufen-Formel).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct KolonieWerte {
    /// Kolonieplätze ohne Forschung.
    pub frei: u8,
    /// Forschung, deren Stufe je eine weitere Kolonie erlaubt.
    pub forschung: Forschung,
}

impl Default for KolonieWerte {
    fn default() -> Self { KolonieWerte { frei: 0, forschung: Forschung::Astrophysik } }
}

impl KolonieWerte {
    pub fn ist_standard(&self) -> bool { *self == KolonieWerte::default() }
}

/// Terraformer-Schiffe (Karl 24.10.2026: „mit jeder Forschungsstufe braucht man weniger Terraformerschiffe … die Felder
/// können von der Grundgröße maximal 40 Felder vergrößert werden … Bahn 13 als Aurelier ca. 60–70 Terraformer mit
/// Terraforming 8, mit Stufe 9 nur noch 55–65, bei Stufe 10 nur noch ca. 50“). Rechnung in `crate::terraform`.
/// `aktiv` false = kein Terraformer (bisheriges Verhalten); nur neue Welten mit Monden und Technik v1.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TerraformWerte {
    pub aktiv: bool,
    /// Höchste Vergrößerung über die Grundgröße auf einem Platz ohne Klimalücke (Karl: 40 Felder).
    pub felder_max: u16,
    /// Abzug von `felder_max` auf dem ungünstigsten Platz des Volkes (40 − 10 = 30, Karls Beispiel Bahn 13).
    pub felder_abzug: u16,
    /// Terraformer für das volle Projekt (Klima und Felder) auf dem ungünstigsten Platz je Terraforming-Stufe 0, 1, 2 …;
    /// höhere Stufen nehmen den letzten Eintrag.
    pub schiffe: Vec<i64>,
    /// Anteil dieser Schiffe für das Klima in Promille (600 = 60 %); der Rest formt die Kruste (Felder).
    pub klima_anteil_promille: i64,
    /// Feldgrenze, für die der Feldanteil gilt (30 = Feldgrenze des ungünstigsten Platzes: dort ergeben Klima- und
    /// Feldschiffe zusammen genau `schiffe`).
    pub felder_bezug: u16,
}

impl Default for TerraformWerte {
    fn default() -> Self {
        TerraformWerte {
            aktiv: false,
            felder_max: 40,
            felder_abzug: 10,
            schiffe: vec![185, 163, 143, 125, 110, 96, 84, 74, 65, 57, 50],
            klima_anteil_promille: 600,
            felder_bezug: 30,
        }
    }
}

impl TerraformWerte {
    pub fn ist_standard(&self) -> bool { *self == TerraformWerte::default() }
    /// Schiffe für das volle Projekt auf dem ungünstigsten Platz bei Terraforming-Stufe `stufe`.
    pub fn schiffe_bei(&self, stufe: u8) -> i64 {
        self.schiffe.get(stufe as usize).or(self.schiffe.last()).copied().unwrap_or(1).max(1)
    }
}

// ---------------------------------------------------------------- Gesamt

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct KernWerte {
    pub version: u8,
    pub klima: KlimaWerte,
    pub sonne: SonnenWerte,
    pub produktion: ProduktionsWerte,
    pub flug: FlugWerte,
    pub boden: BodenWerte,
    pub mond: MondWerte,
    pub kampf: KampfWerte,
    pub handel: HandelsWerte,
    pub industrie: IndustrieWerte,
    pub expedition: ExpeditionsWerte,
    /// Kolonieplätze (0.4.7); Standard wird nicht geschrieben, damit Hülle und Hash laufender Welten gleich bleiben.
    #[serde(skip_serializing_if = "KolonieWerte::ist_standard")]
    pub kolonien: KolonieWerte,
    /// Terraformer-Schiffe (0.4.7); Standard wird nicht geschrieben.
    #[serde(skip_serializing_if = "TerraformWerte::ist_standard")]
    pub terraform: TerraformWerte,
    /// Eigene Werte einzelner Einheiten je Volk (Karl 21.10.2026: „schiffe von aurelianern haben andere werte wie die der
    /// krath“): Faktoren auf die Einheitenregel, zusätzlich zu den allgemeinen Volksfaktoren (`voelker.<volk>.waffen` …).
    /// Leer = keine Abweichung; dann wird das Feld nicht geschrieben und jede Rechnung bleibt bitgleich.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub volk_einheiten: BTreeMap<Volk, BTreeMap<Einheit, VolkEinheit>>,
}

/// Faktoren einer Einheit für ein Volk (1 = wie die Einheitenregel). Wirken auf Angriff, Schild und Struktur im Kampf
/// (auch Raketeneinschlag), Frachtraum, Tempo, Treibstoffverbrauch und Baukosten (damit auch Bauzeit und Unterhalt).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VolkEinheit {
    pub angriff: f64,
    pub schild: f64,
    pub struktur: f64,
    pub ladung: f64,
    pub tempo: f64,
    pub verbrauch: f64,
    pub kosten: f64,
}
impl Default for VolkEinheit {
    fn default() -> Self { VolkEinheit { angriff: 1.0, schild: 1.0, struktur: 1.0, ladung: 1.0, tempo: 1.0, verbrauch: 1.0, kosten: 1.0 } }
}

impl Default for KernWerte {
    fn default() -> Self {
        KernWerte {
            version: 1,
            klima: KlimaWerte::default(),
            sonne: SonnenWerte::default(),
            produktion: ProduktionsWerte::default(),
            flug: FlugWerte::default(),
            boden: BodenWerte::default(),
            mond: MondWerte::default(),
            kampf: KampfWerte::default(),
            handel: HandelsWerte::default(),
            industrie: IndustrieWerte::default(),
            expedition: ExpeditionsWerte::default(),
            kolonien: KolonieWerte::default(),
            terraform: TerraformWerte::default(),
            volk_einheiten: BTreeMap::new(),
        }
    }
}

impl KernWerte {
    /// Werte, die vom Standard (bisherige Konstanten) abweichen: (Pfad, Standard, wirksam).
    pub fn abweichungen(&self) -> Vec<(String, String, String)> {
        fn flach(v: &serde_json::Value, pfad: String, out: &mut BTreeMap<String, String>) {
            match v {
                serde_json::Value::Object(m) => for (k, x) in m { flach(x, if pfad.is_empty() { k.clone() } else { format!("{pfad}.{k}") }, out) },
                serde_json::Value::Array(a) => for (i, x) in a.iter().enumerate() { flach(x, format!("{pfad}.{}", i + 1), out) },
                x => { out.insert(pfad, x.to_string()); }
            }
        }
        let (mut a, mut b) = (BTreeMap::new(), BTreeMap::new());
        flach(&serde_json::to_value(standard()).unwrap_or_default(), String::new(), &mut a);
        flach(&serde_json::to_value(self).unwrap_or_default(), String::new(), &mut b);
        b.into_iter().filter(|(k, v)| a.get(k) != Some(v)).map(|(k, v)| { let s = a.get(&k).cloned().unwrap_or_else(|| "–".into()); (k, s, v) }).collect()
    }
}

/// Bisherige Konstanten (für laufende Welten und Welten ohne Technik v1).
// 0.4.8: Neue Felder mit Standardwert werden nicht geschrieben – Hüllen laufender Welten bleiben bytegleich.
fn ist_null(x: &i64) -> bool { *x == 0 }
fn ist_6(x: &i64) -> bool { *x == 6 }
fn ist_12(x: &i64) -> bool { *x == 12 }
fn ist_1000(x: &i64) -> bool { *x == 1000 }

pub fn standard() -> &'static KernWerte {
    static S: std::sync::OnceLock<KernWerte> = std::sync::OnceLock::new();
    S.get_or_init(KernWerte::default)
}

fn bereich<T: PartialOrd + std::fmt::Display>(name: &str, v: T, min: T, max: T) -> Result<(), String> {
    if v < min || v > max { Err(format!("Kernwerte: {name} = {v} liegt außerhalb {min} … {max}")) } else { Ok(()) }
}

fn zahl(name: &str, v: f64, min: f64, max: f64) -> Result<(), String> {
    if !v.is_finite() { return Err(format!("Kernwerte: {name} ist keine Zahl")); }
    bereich(name, v, min, max)
}

impl KernWerte {
    pub fn laden(text: &str) -> Result<KernWerte, String> {
        let k: KernWerte = ron::from_str(text).map_err(|e| format!("Kernwerte nicht lesbar: {e}"))?;
        k.pruefen()?;
        Ok(k)
    }

    /// Grenzen, außerhalb derer der Kern nicht sinnvoll rechnet (Überläufe, Teilung durch null, Endlosschleifen).
    pub fn pruefen(&self) -> Result<(), String> {
        if self.version != 1 { return Err("Kernwerte: unbekannte Version".into()); }
        let k = &self.klima;
        for (v, t) in &k.wachstum { if t.len() != 12 { return Err(format!("Kernwerte: klima.wachstum.{v} braucht 12 Plätze")); } }
        for (v, t) in &k.freigabe { if t.len() != 12 { return Err(format!("Kernwerte: klima.freigabe.{v} braucht 12 Plätze")); } }
        if self.sonne.solar_promille.len() != 12 { return Err("Kernwerte: sonne.solar_promille braucht 12 Plätze".into()); }
        for v in Volk::ALLE {
            for (i, w) in k.tabelle(v).iter().enumerate() { bereich(&format!("klima.wachstum.{v}[{}]", i + 1), *w, 0, 10_000)?; }
            for (i, w) in k.freigabe(v).iter().enumerate() { bereich(&format!("klima.freigabe.{v}[{}]", i + 1), *w, 0, 30)?; }
        }
        bereich("klima.restklima_promille", k.restklima_promille, 0, 1000)?;
        bereich("klima.endspiel_tech", k.endspiel_tech, 1, 30)?;
        bereich("klima.klimaverlust_promille", k.klimaverlust_promille, 0, 1000)?;
        bereich("klima.xeno_wachstum_promille", k.xeno_wachstum_promille, 0, 5000)?;
        let s = &self.sonne;
        for (i, w) in s.solar_promille.iter().enumerate() { bereich(&format!("sonne.solar_promille[{}]", i + 1), *w, 0, 10_000)?; }
        for (kl, w) in &s.klassen { zahl(&format!("sonne.klassen.{kl:?}.strahlkraft"), w.strahlkraft, 0.0, 10.0)?; }
        zahl("sonne.groessen_exponent", s.groessen_exponent, -4.0, 6.0)?;
        zahl("sonne.dyson_schatten", s.dyson_schatten, 0.0, 1.0)?;
        let p = &self.produktion;
        zahl("produktion.produktionsausbau", p.produktionsausbau, 0.0, 20.0)?;
        zahl("produktion.automatisierung_untergrenze", p.automatisierung_untergrenze, 0.0, 1.0)?;
        zahl("produktion.xeno_synthese_ausbeute", p.xeno_synthese_ausbeute, 0.0, 5.0)?;
        zahl("produktion.xeno_synthese_energie", p.xeno_synthese_energie, 0.0, 50.0)?;
        for (g, n) in &p.xeno_synthese_rezept {
            if !matches!(g, Gut::Deuterium | Gut::Kristall | Gut::Legierung | Gut::Elektronik) {
                return Err(format!("Kernwerte: Xenosynthese kennt nur Deuterium, Kristall, Legierung und Elektronik ({g})"));
            }
            zahl(&format!("produktion.xeno_synthese_rezept.{g}"), *n, 0.0, 1000.0)?;
        }
        bereich("produktion.baumasse_legierung", p.baumasse_legierung, 0, 100)?;
        bereich("flug.grundzeit", self.flug.grundzeit, 0, 100_000)?;
        bereich("flug.zeitkonstante", self.flug.zeitkonstante, 1, 100_000)?;
        let b = &self.boden;
        for n in crate::boden_katalog::NAMEN {
            let Some(t) = b.typen.get(n) else { return Err(format!("Kernwerte: Bodentruppe {n} fehlt")); };
            bereich(&format!("boden.typen.{n}.plaetze"), t.plaetze, 1, 1000)?;
            bereich(&format!("boden.typen.{n}.fertigungsplaetze"), t.fertigungsplaetze, 1, 100)?;
            bereich(&format!("boden.typen.{n}.personal"), t.personal, 0, 1000)?;
            bereich(&format!("boden.typen.{n}.dauer_sekunden"), t.dauer_sekunden, 1, 30 * TAG)?;
            bereich(&format!("boden.typen.{n}.angriff"), t.angriff, 0, 1_000_000)?;
            bereich(&format!("boden.typen.{n}.struktur"), t.struktur, 1, 10_000_000)?;
            bereich(&format!("boden.typen.{n}.kaserne"), t.kaserne, 1, 40)?;
            bereich(&format!("boden.typen.{n}.ab_stufe"), t.ab_stufe, 1, 5)?;
            for (g, m) in &t.kosten { bereich(&format!("boden.typen.{n}.kosten.{g}"), *m, 0, 10_000_000)?; }
        }
        for n in b.typen.keys() {
            if !crate::boden_katalog::NAMEN.contains(&n.as_str()) { return Err(format!("Kernwerte: unbekannte Bodentruppe {n}")); }
        }
        for (a, m) in &b.konter {
            if !crate::boden_katalog::NAMEN.contains(&a.as_str()) { return Err(format!("Kernwerte: Konter für unbekannte Bodentruppe {a}")); }
            for (d, f) in m {
                if !crate::boden_katalog::NAMEN.contains(&d.as_str()) && d != "miliz" { return Err(format!("Kernwerte: Konterziel {d} unbekannt")); }
                bereich(&format!("boden.konter.{a}.{d}"), *f, 0, 100_000)?;
            }
        }
        bereich("boden.transporter_plaetze", b.transporter_plaetze, 1, 100_000)?;
        bereich("boden.koordination_je_drohne_promille", b.koordination_je_drohne_promille, 0, 10_000)?;
        bereich("boden.koordination_max_drohnen", b.koordination_max_drohnen, 0, 1000)?;
        bereich("boden.reparatur_je_drohne", b.reparatur_je_drohne, 0, 1_000_000)?;
        bereich("boden.miliz_waffen_promille", b.miliz_waffen_promille, 0, 1_000_000)?;
        bereich("boden.miliz_ruestung", b.miliz_ruestung, 1, 1_000_000)?;
        bereich("boden.miliz_kapitulation_promille", b.miliz_kapitulation_promille, 1, 1000)?;
        bereich("boden.miliz_dienst_promille", b.miliz_dienst_promille, 0, 1000)?;
        bereich("boden.miliz_legierung_je_bewohner", b.miliz_legierung_je_bewohner, 0, 1000)?;
        bereich("boden.belagerung_stunden", b.belagerung_stunden, 0, 24 * 30)?;
        bereich("boden.runde_minuten", b.runde_minuten, 1, 24 * 60)?;
        bereich("boden.bombardement_struktur_je_stufe", b.bombardement_struktur_je_stufe, 1, 10_000_000)?;
        bereich("boden.bombenlast_je_schacht", b.bombenlast_je_schacht, 0, 10_000_000)?;
        bereich("boden.integritaet_grenze_promille", b.integritaet_grenze_promille, 0, 999)?;
        let m = &self.mond;
        bereich("mond.truemmer_je_km_erz", m.truemmer_je_km_erz, 0, 1_000_000)?;
        bereich("mond.truemmer_je_km_kristall", m.truemmer_je_km_kristall, 0, 1_000_000)?;
        bereich("mond.ueberlebende_promille", m.ueberlebende_promille, 0, 1000)?;
        bereich("mond.zerstoerung_wurzel_grenze", m.zerstoerung_wurzel_grenze, 1, 10_000)?;
        bereich("mond.verlust_teiler", m.verlust_teiler, 1, 100_000)?;
        zahl("kampf.primaer_faktor", self.kampf.primaer_faktor, 0.0, 20.0)?;
        zahl("kampf.sekundaer_faktor", self.kampf.sekundaer_faktor, 0.0, 20.0)?;
        let h = &self.handel;
        for (g, k) in &h.soforthandel_kurse {
            if !["erz", "kristall", "deuterium", "nahrung", "credits"].contains(&g.as_str()) {
                return Err(format!("Kernwerte: Soforthandel kennt {g} nicht (Erz, Kristall, Deuterium, Nahrung, Credits)"));
            }
            zahl(&format!("handel.soforthandel_kurse.{g}"), *k, 0.001, 1_000_000.0)?;
        }
        zahl("handel.soforthandel_gebuehr", h.soforthandel_gebuehr, 0.0, 0.9)?;
        zahl("handel.handelsabkommen_faktor", h.handelsabkommen_faktor, 0.0, 1.0)?;
        let i = &self.industrie;
        bereich("industrie.kaserne_ab_stufe", i.kaserne_ab_stufe, 1, 5)?;
        bereich("industrie.kolonieschiff_ab_stufe", i.kolonieschiff_ab_stufe, 1, 5)?;
        bereich("industrie.kolonieschiff_werft", i.kolonieschiff_werft, 0, 40)?;
        for (g, n) in i.kolonieschiff_kosten.iter().chain(&i.habitatmodul_kosten) { bereich(&format!("industrie.kosten.{g}"), *n, 0, 100_000_000)?; }
        for (f, n) in i.kolonieschiff_forschung.iter().chain(&self.expedition.forschung) { bereich(&format!("forschung.{f}"), *n, 0, crate::regeln::MAX_FORSCHUNG)?; }
        let e = &self.expedition;
        for (name, r) in [("pathfinder", &e.pathfinder), ("bergungsschiff", &e.bergungsschiff)] {
            if r.tempo <= 0 || r.antrieb.is_none() || r.struktur <= 0 { return Err(format!("Kernwerte: {name} braucht Antrieb, Tempo und Struktur")); }
        }
        bereich("expedition.bergungsschiff_personenplaetze", e.bergungsschiff_personenplaetze, 0, 1_000_000)?;
        bereich("kolonien.frei", self.kolonien.frei, 0, (crate::verwaltung::MAX_PLANETEN - 1) as u8)?;
        let t = &self.terraform;
        bereich("terraform.felder_max", t.felder_max, 0, 1000)?;
        bereich("terraform.felder_abzug", t.felder_abzug, 0, t.felder_max)?;
        if t.schiffe.is_empty() || t.schiffe.len() > 31 { return Err("Kernwerte: terraform.schiffe braucht 1 bis 31 Einträge (Stufe 0, 1, 2 …)".into()); }
        for (i, n) in t.schiffe.iter().enumerate() { bereich(&format!("terraform.schiffe[{i}]"), *n, 1, 100_000)?; }
        bereich("terraform.klima_anteil_promille", t.klima_anteil_promille, 0, 1000)?;
        bereich("terraform.felder_bezug", t.felder_bezug, 1, 1000)?;
        for (v, m) in &self.volk_einheiten {
            for (e, f) in m {
                for (n, x) in [("angriff", f.angriff), ("schild", f.schild), ("struktur", f.struktur), ("ladung", f.ladung), ("tempo", f.tempo), ("verbrauch", f.verbrauch), ("kosten", f.kosten)] {
                    zahl(&format!("volk_einheiten.{v}.{e}.{n}"), x, if n == "struktur" || n == "kosten" || n == "tempo" { 0.05 } else { 0.0 }, 20.0)?;
                }
            }
        }
        Ok(())
    }
}

impl crate::Regelwerk {
    /// Wirksame Kernwerte: aus `regeln/kernwerte.ron` (neue Welten mit Technik v1), sonst die bisherigen Konstanten.
    pub fn kw(&self) -> &KernWerte {
        self.technik.as_ref().and_then(|t| t.kernwerte.as_ref()).unwrap_or_else(|| standard())
    }
    /// Strahlkraft der Sonne des Systems in Promille (1000 ohne Kernwerte-Änderung).
    pub fn stern_promille(&self, k: Koord) -> i64 {
        self.kw().sonne.stern_promille(Sternklasse::von(k.sektor, k.system))
    }
    /// Kurs des Soforthandels (Credits je Einheit) für ein Gut, falls dort tauschbar.
    pub fn sofortkurs(&self, g: Gut) -> Option<f64> {
        self.kw().handel.soforthandel_kurse.get(g.name()).copied()
    }
    /// Eigene Faktoren der Einheit `e` für `volk` (None = wie die Einheitenregel; Aufrufer rechnen dann unverändert).
    pub fn volk_einheit(&self, volk: Volk, e: Einheit) -> Option<&VolkEinheit> {
        let m = &self.kw().volk_einheiten;
        if m.is_empty() { return None; }
        m.get(&volk).and_then(|x| x.get(&e))
    }
    /// Frachtraum eines Stücks in Tausendsteln, vor Volks- und Logistikfaktor (mit eigenem Faktor je Volk und Einheit).
    pub fn ladung_milli(&self, e: Einheit, volk: Volk) -> i64 {
        let b = self.einh(e).ladung * crate::typen::M;
        match self.volk_einheit(volk, e) { Some(v) if v.ladung != 1.0 => crate::typen::mal(b, v.ladung), _ => b }
    }
}
