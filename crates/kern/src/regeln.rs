//! Das Regelwerk als Daten. Geladen aus `regeln/regelwerk.ron`.

use crate::typen::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Kosten in ganzen Einheiten je Gut.
pub type Preis = BTreeMap<Gut, i64>;

fn eins() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeltRegel {
    pub sektoren: u8,
    pub systeme_je_sektor: u8,
    pub plaetze_je_system: u8,
    pub nebel_abstand: u8,
    pub nebel_versatz: u8,
    pub guertel_abstand: u8,
    pub guertel_versatz: u8,
    pub reichtum_min: f64,
    pub reichtum_max: f64,
    pub epoche_tage: i64,
    pub fenster_sekunden: i64,
    pub heimat_position: u8,
    pub heimat_felder: u16,
    pub start_abstand: u8,
    pub start_bevoelkerung: i64,
    pub start_credits: i64,
    pub start_stabilitaet: f64,
    pub start_steuersatz: u8,
    pub grundwohnraum: i64,
    pub start_bestand: Preis,
    pub start_gebaeude: BTreeMap<Gebaeude, u8>,
    pub max_gebaeudestufe: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZoneRegel {
    pub von: u8,
    pub bis: u8,
    pub felder_min: u16,
    pub felder_max: u16,
    pub solar: f64,
    pub deuterium: f64,
    pub nahrung: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolkRegel {
    #[serde(default = "eins")]
    pub ladung: f64,
    #[serde(default = "eins")]
    pub marktgebuehr: f64,
    #[serde(default = "eins")]
    pub waffen: f64,
    #[serde(default = "eins")]
    pub panzerung: f64,
    #[serde(default = "eins")]
    pub steuer: f64,
    #[serde(default = "eins")]
    pub werftzeit: f64,
    #[serde(default = "eins")]
    pub forschung: f64,
    #[serde(default = "eins")]
    pub wachstum: f64,
    #[serde(default = "eins")]
    pub nahrung: f64,
    #[serde(default = "eins")]
    pub energie: f64,
    #[serde(default = "eins")]
    pub kolonieschiff: f64,
    #[serde(default)]
    pub pluenderquote: Option<f64>,
    #[serde(default)]
    pub ohne_nahrung: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WirtschaftRegel {
    pub wachstum_je_tag: f64,
    pub arbeitsquote: f64,
    pub fachkraefte_basis: i64,
    pub fachkraefte_je_akademie: f64,
    pub nahrung_je_1000: f64,
    pub konsum_je_1000: f64,
    pub energie_je_1000_syntheten: f64,
    pub hunger_min: f64,
    pub steuer_je_einwohner_stunde: f64,
    pub unterhalt_schiffe_je_tag: f64,
    pub verwaltung_je_kolonie_tag: i64,
    pub desertion_nach_tagen: i64,
    pub desertion_anteil: f64,
    pub ertragswachstum: f64,
    pub lager_roh: i64,
    pub lager_verarbeitet: i64,
    pub lager_selten: i64,
    pub lager_faktor: f64,
    pub bunker_je_stufe: i64,
    pub bunker_verarbeitet_anteil: f64,
    pub bauzeit_teiler: i64,
    pub min_bauzeit_sekunden: i64,
    pub warteschlange: usize,
    pub forschung_warteschlange: usize,
    pub rezepte: BTreeMap<Gut, BTreeMap<Gut, f64>>,
    pub fusion_deuterium: f64,
    pub energietechnik_bonus: f64,
    pub werkstoffkunde_bonus: f64,
    pub automatisierung_bonus: f64,
    pub agrar_bonus: f64,
    pub xeno_bonus: f64,
    pub terraforming_felder: u16,
    pub siedler: i64,
    pub habitat_wohnraum: i64,
    pub kolonien_max: u8,
    pub bauteile: BTreeMap<Gut, Preis>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StabilitaetRegel {
    pub ziel_basis: f64,
    pub annaeherung_je_stunde: f64,
    pub konsum_bonus: f64,
    pub wohnraum_bonus: f64,
    pub wohnraum_frei_voll: f64,
    pub soziologie_je_stufe: f64,
    pub steuer_frei_bis: u8,
    pub steuer_malus_je_punkt: f64,
    pub kolonie_ueber_grenze: f64,
    pub pluenderung_malus: f64,
    pub pluenderung_tage: i64,
    pub pluenderung_malus_max: f64,
    pub belagerung_je_tag: f64,
    pub belagerung_erholung_je_tag: f64,
    pub wachstum_voll_ab: f64,
    pub wachstum_null_unter: f64,
    pub unruhen_unter: f64,
    pub uebernahme_unter: f64,
    pub produktivitaet_basis: f64,
    pub produktivitaet_spanne: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GebaeudeRegel {
    pub kosten: Preis,
    pub faktor: f64,
    #[serde(default)]
    pub arbeiter: f64,
    #[serde(default)]
    pub fachkraefte: f64,
    #[serde(default)]
    pub energie: f64,
    pub ab_stufe: u8,
    #[serde(default)]
    pub ertrag: f64,
    #[serde(default)]
    pub braucht: BTreeMap<Gebaeude, u8>,
    pub wirkung: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForschungRegel {
    pub kosten: Preis,
    pub faktor: f64,
    pub fp: f64,
    pub ab_stufe: u8,
    pub labor: u8,
    pub wirkung: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EinheitRegel {
    pub kosten: Preis,
    pub struktur: i64,
    pub schild: i64,
    pub angriff: i64,
    #[serde(default)]
    pub ladung: i64,
    #[serde(default)]
    pub tempo: i64,
    #[serde(default)]
    pub antrieb: Option<Forschung>,
    #[serde(default)]
    pub verbrauch: i64,
    #[serde(default)]
    pub besatzung: i64,
    pub ab_stufe: u8,
    #[serde(default)]
    pub werft: u8,
    #[serde(default)]
    pub braucht: BTreeMap<Gebaeude, u8>,
    #[serde(default)]
    pub schnellfeuer: BTreeMap<Einheit, u32>,
    #[serde(default)]
    pub max_anzahl: i64,
    pub wirkung: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StufenRegel {
    pub name: String,
    pub einwohner: i64,
    #[serde(default)]
    pub gebaeude: BTreeMap<Gebaeude, u8>,
    #[serde(default)]
    pub forschung: BTreeMap<Forschung, u8>,
    #[serde(default)]
    pub versorgung_plus: bool,
    #[serde(default)]
    pub stabilitaet: f64,
    #[serde(default)]
    pub konsum_deckung: f64,
    #[serde(default)]
    pub kolonien: u8,
    pub kosten: Preis,
    pub schaltet_frei: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlugRegel {
    pub zeitfaktor: i64,
    pub min_sekunden: i64,
    pub system_basis: i64,
    pub je_position: i64,
    pub sektor_basis: i64,
    pub je_system: i64,
    pub je_sektor: i64,
    pub treibstoff_teiler: i64,
    pub antrieb_bonus: BTreeMap<Forschung, f64>,
    pub logistik_bonus: f64,
    pub raumhafen_ersparnis: f64,
    pub raumhafen_ersparnis_max: f64,
    pub halten_max_stunden: i64,
    pub abbau_max_stunden: i64,
    pub abbau_erz_je_stunde: i64,
    pub abbau_kristall_je_stunde: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KampfRegel {
    pub runden: u8,
    pub verpuffen_anteil: f64,
    pub explosion_unter: f64,
    pub truemmer_anteil: f64,
    pub verteidigung_wiederaufbau: f64,
    pub pluenderquote: f64,
    pub tech_je_stufe: f64,
    pub warnzeit_basis_minuten: i64,
    pub warnzeit_je_phalanx_minuten: i64,
    pub truppen_je_transporter: i64,
    pub garnison_je_kaserne: i64,
    pub garnison_je_100_einwohner: i64,
    pub eroberung_stufenverlust: f64,
    pub eroberung_bevoelkerung: f64,
    pub simulator_laeufe: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiplomatieRegel {
    pub kuendigungsfrist_stunden: i64,
    pub allianz_max: usize,
    pub buendnisse_max: usize,
    pub anfaengerschutz_tage: i64,
    pub anfaengerschutz_bis_stufe: u8,
    pub schwachenschutz_anteil: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarktRegel {
    pub gebuehr: f64,
    pub orders_je_marktstufe: usize,
    pub liefertempo: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WertungRegel {
    pub einheit: i64,
    pub gewichte: BTreeMap<Gut, f64>,
    pub einwohner_je_punkt: i64,
    pub stufenbonus: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentenRegel {
    pub takt_stunden: BTreeMap<Rolle, i64>,
    pub mindestabstand_stunden: i64,
    /// Ein Wecker oder ein nicht dringendes Ereignis ruft eine Rolle frühestens nach diesem Anteil ihres Takts
    /// wieder auf. Ohne die Grenze stellen Modelle jede Stunde einen Wecker, und ein Lauf kostet ein Vielfaches.
    #[serde(default = "halber_takt")]
    pub frueh_anteil: f64,
    pub doktrin_zeichen: usize,
    pub notiz_zeichen: usize,
    pub nachricht_zeichen: usize,
    pub nachrichten_je_tag: u16,
    pub aktionen_je_aufruf: usize,
    pub abfragen_je_aufruf: usize,
    pub start_anteile: BTreeMap<Topf, u8>,
    pub chronik_tage: i64,
}

fn halber_takt() -> f64 {
    0.5
}

impl AgentenRegel {
    /// Frühester erneuter Aufruf einer Rolle durch Wecker oder nicht dringendes Ereignis, in Sekunden.
    pub fn frueheste_sekunden(&self, rolle: Rolle) -> i64 {
        let takt = self.takt_stunden.get(&rolle).copied().unwrap_or(0) * STUNDE;
        (self.mindestabstand_stunden * STUNDE).max(mal(takt, self.frueh_anteil))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Regelwerk {
    pub version: String,
    pub welt: WeltRegel,
    pub zonen: BTreeMap<Zone, ZoneRegel>,
    pub voelker: BTreeMap<Volk, VolkRegel>,
    pub wirtschaft: WirtschaftRegel,
    pub stabilitaet: StabilitaetRegel,
    pub gebaeude: BTreeMap<Gebaeude, GebaeudeRegel>,
    pub forschung: BTreeMap<Forschung, ForschungRegel>,
    pub einheiten: BTreeMap<Einheit, EinheitRegel>,
    pub stufen: Vec<StufenRegel>,
    pub stufen_haltezeit_stunden: i64,
    pub flug: FlugRegel,
    pub kampf: KampfRegel,
    pub diplomatie: DiplomatieRegel,
    pub markt: MarktRegel,
    pub wertung: WertungRegel,
    pub agenten: AgentenRegel,
    pub zusatz: ZusatzRegeln,
    /// SHA-256 der Regeldatei, steht in jedem Protokoll.
    #[serde(default)]
    pub hash: String,
    /// Vorberechnete Tabellen, beim Laden gefüllt.
    #[serde(default)]
    pub cache: RegelCache,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZusatzRegeln {
    pub silo_plaetze_je_stufe: i64,
    pub raketen_kosten: [Preis; 2],
    pub raketen_bauzeit_sekunden: i64,
    pub raketen_flug_min_sekunden: i64,
    pub raketen_flug_je_system_sekunden: i64,
    pub raketen_reichweite_je_silo: u8,
    pub raketen_schaden: i64,
    pub orbitalring_felder: u16,
    pub orbitalring_wohnraum: i64,
    pub archiv_fp_bonus: f64,
    pub versorgungsnetz_energie_bonus: f64,
}

/// Höchste Forschungsstufe.
pub const MAX_FORSCHUNG: u8 = 30;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegelCache {
    /// Ertragswachstum hoch n als Festkomma.
    pub pot_ertrag: Vec<i64>,
    /// Wert aller Stufen 1 bis n je Gebäude und Forschung.
    pub kum_gebaeude: Vec<Vec<i64>>,
    pub kum_forschung: Vec<Vec<i64>>,
}

pub fn preis_array(p: &Preis) -> [i64; GUETER] {
    let mut a = [0i64; GUETER];
    for (g, m) in p {
        a[g.idx()] = *m * M;
    }
    a
}

impl Regelwerk {
    /// Liest das Regelwerk aus RON, prüft es auf Vollständigkeit und merkt sich den Hash.
    pub fn laden(text: &str) -> Result<Regelwerk, String> {
        let mut r: Regelwerk = ron::from_str(text).map_err(|e| format!("Regelwerk nicht lesbar: {e}"))?;
        let mut h = Sha256::new();
        h.update(text.replace("\r\n", "\n").as_bytes());
        r.hash = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        r.pruefe()?;
        r.cache_fuellen();
        Ok(r)
    }

    fn cache_fuellen(&mut self) {
        let max = self.welt.max_gebaeudestufe;
        let pot_ertrag = (0..=max as u32).map(|n| pot(self.wirtschaft.ertragswachstum, n) as i64).collect();
        let kum = |kosten: &dyn Fn(u8) -> [i64; GUETER], bis: u8| -> Vec<i64> {
            let mut v = vec![0i64];
            for s in 1..=bis {
                let letzter = *v.last().unwrap();
                v.push(letzter.saturating_add(self.wert(&kosten(s))));
            }
            v
        };
        let kum_gebaeude = Gebaeude::ALLE.iter().filter(|g| self.gebaeude.contains_key(g)).map(|g| kum(&|s| self.kosten_gebaeude(*g, s), max)).collect();
        let kum_forschung = Forschung::ALLE.iter().filter(|f| self.forschung.contains_key(f)).map(|f| kum(&|s| self.kosten_forschung(*f, s), MAX_FORSCHUNG)).collect();
        self.cache = RegelCache { pot_ertrag, kum_gebaeude, kum_forschung };
    }

    fn pruefe(&self) -> Result<(), String> {
        let z=&self.zusatz;
        if !(1..=1000).contains(&z.silo_plaetze_je_stufe) || !(1..=86_400).contains(&z.raketen_bauzeit_sekunden)
            || z.raketen_flug_min_sekunden<=self.welt.fenster_sekunden || z.raketen_flug_min_sekunden<1200
            || !(0..=86_400).contains(&z.raketen_flug_je_system_sekunden) || z.raketen_reichweite_je_silo==0
            || !(1..=1_000_000_000).contains(&z.raketen_schaden) || z.orbitalring_felder>1000
            || !(0..=1_000_000_000).contains(&z.orbitalring_wohnraum)
            || !z.archiv_fp_bonus.is_finite() || !(0.0..=100.0).contains(&z.archiv_fp_bonus)
            || !z.versorgungsnetz_energie_bonus.is_finite() || !(0.0..=100.0).contains(&z.versorgungsnetz_energie_bonus)
            || z.raketen_kosten.iter().any(|p|p.is_empty() || p.values().any(|n|!(0..=100_000).contains(n))) {
            return Err("Regelwerk: ungültige Zusatzregeln für Raketen oder Großprojekte".into());
        }
        for g in Gebaeude::ALLE {
            if self.version == "0.1.0" && g == Gebaeude::Geheimdienst { continue; }
            if !self.gebaeude.contains_key(&g) {
                return Err(format!("Regelwerk: Gebäude {g} fehlt"));
            }
        }
        for f in Forschung::ALLE {
            if self.version == "0.1.0" && matches!(f, Forschung::Ueberwachungstechnik | Forschung::Abschirmtechnik) { continue; }
            if !self.forschung.contains_key(&f) {
                return Err(format!("Regelwerk: Forschung {f} fehlt"));
            }
        }
        for e in Einheit::ALLE {
            let Some(er) = self.einheiten.get(&e) else {
                return Err(format!("Regelwerk: Einheit {e} fehlt"));
            };
            if e.ist_schiff() && (er.antrieb.is_none() || er.tempo <= 0) {
                return Err(format!("Regelwerk: Schiff {e} braucht Antrieb und Tempo"));
            }
        }
        for v in Volk::ALLE {
            if !self.voelker.contains_key(&v) {
                return Err(format!("Regelwerk: Volk {v} fehlt"));
            }
        }
        for z in Zone::ALLE {
            if !self.zonen.contains_key(&z) {
                return Err(format!("Regelwerk: Zone {z} fehlt"));
            }
        }
        for g in Gut::ALLE {
            if !self.wertung.gewichte.contains_key(&g) {
                return Err(format!("Regelwerk: Gewicht für {g} fehlt"));
            }
        }
        if self.stufen.len() != 4 {
            return Err("Regelwerk: genau vier Aufstiege (II bis V) erwartet".into());
        }
        if self.wertung.stufenbonus.len() != 5 {
            return Err("Regelwerk: fünf Stufenboni erwartet".into());
        }
        let summe: u32 = Topf::ALLE.iter().map(|t| *self.agenten.start_anteile.get(t).unwrap_or(&0) as u32).sum();
        if summe != 100 {
            return Err("Regelwerk: Startanteile der Töpfe müssen 100 ergeben".into());
        }
        // Fairnessregel: die kürzeste Flugzeit liegt über der Fensterbreite.
        if self.flug.min_sekunden <= self.welt.fenster_sekunden {
            return Err("Regelwerk: kürzeste Flugzeit muss über der Fensterbreite liegen".into());
        }
        Ok(())
    }

    pub fn geb(&self, g: Gebaeude) -> &GebaeudeRegel {
        self.gebaeude.get(&g).unwrap_or_else(|| &self.gebaeude[&Gebaeude::Sensorphalanx])
    }
    pub fn forsch(&self, f: Forschung) -> &ForschungRegel {
        self.forschung.get(&f).unwrap_or_else(|| &self.forschung[&Forschung::Spionagetechnik])
    }
    pub fn einh(&self, e: Einheit) -> &EinheitRegel {
        &self.einheiten[&e]
    }
    pub fn volk(&self, v: Volk) -> &VolkRegel {
        &self.voelker[&v]
    }

    /// Grundwert mal Stufe mal Ertragswachstum hoch Stufe, in Tausendsteln.
    pub fn stufenwert(&self, grundwert: f64, n: u8) -> i64 {
        if n == 0 || grundwert == 0.0 {
            return 0;
        }
        let p = self.cache.pot_ertrag[(n as usize).min(self.cache.pot_ertrag.len() - 1)] as i128;
        ((fx(grundwert) * n as i128 * p) / FX / 1000) as i64
    }

    fn hoch(basis: &Preis, faktor: f64, stufe: u8) -> [i64; GUETER] {
        let p = pot(faktor, stufe.saturating_sub(1) as u32);
        let mut a = [0i64; GUETER];
        for (g, m) in basis {
            let v = (*m as i128 * M as i128 * p) / FX;
            a[g.idx()] = v.min(i64::MAX as i128 / 4) as i64;
        }
        a
    }

    /// Kosten, um ein Gebäude auf `stufe` zu bringen.
    pub fn kosten_gebaeude(&self, g: Gebaeude, stufe: u8) -> [i64; GUETER] {
        let r = self.geb(g);
        Self::hoch(&r.kosten, r.faktor, stufe)
    }

    pub fn kosten_forschung(&self, f: Forschung, stufe: u8) -> [i64; GUETER] {
        let r = self.forsch(f);
        Self::hoch(&r.kosten, r.faktor, stufe)
    }

    /// Forschungspunkte für `stufe`, in Tausendsteln.
    pub fn fp_forschung(&self, f: Forschung, stufe: u8) -> i64 {
        let r = self.forsch(f);
        ((fx(r.fp) * pot(r.faktor, stufe.saturating_sub(1) as u32)) / FX / 1000) as i64
    }

    pub fn kosten_einheit(&self, e: Einheit, volk: Volk) -> [i64; GUETER] {
        let mut a = preis_array(&self.einh(e).kosten);
        if e == Einheit::Kolonieschiff {
            let f = self.volk(volk).kolonieschiff;
            for g in [Gut::Erz, Gut::Kristall, Gut::Deuterium, Gut::Legierung, Gut::Elektronik] {
                a[g.idx()] = mal(a[g.idx()], f);
            }
        }
        a
    }

    pub fn kosten_bauteil(&self, g: Gut) -> Option<[i64; GUETER]> {
        self.wirtschaft.bauteile.get(&g).map(preis_array)
    }

    /// Wert einer Gütermenge in Werteinheiten (Tausendstel).
    pub fn wert(&self, k: &[i64; GUETER]) -> i64 {
        let mut s: i128 = 0;
        for g in Gut::ALLE {
            s += k[g.idx()] as i128 * fx(self.wertung.gewichte[&g]) / FX;
        }
        s.min(i64::MAX as i128 / 4) as i64
    }

    /// Summe der Kosten aller Stufen 1 bis `stufe` als Wert.
    pub fn wert_gebaeude_bis(&self, g: Gebaeude, stufe: u8) -> i64 {
        if stufe == 0 { return 0; }
        let v = &self.cache.kum_gebaeude[g.idx()];
        v[(stufe as usize).min(v.len() - 1)]
    }

    pub fn wert_forschung_bis(&self, f: Forschung, stufe: u8) -> i64 {
        if stufe == 0 { return 0; }
        let v = &self.cache.kum_forschung[f.idx()];
        v[(stufe as usize).min(v.len() - 1)]
    }

    /// Lagergrenze je Gut für eine Lagerstufe, in Tausendsteln.
    pub fn lagergrenze(&self, stufe: u8) -> [i64; GUETER] {
        let p = pot(self.wirtschaft.lager_faktor, stufe as u32);
        let f = |basis: i64| ((basis as i128 * M as i128 * p) / FX) as i64;
        let mut a = [0i64; GUETER];
        for g in Gut::ALLE {
            a[g.idx()] = match g {
                Gut::Erz | Gut::Kristall | Gut::Deuterium | Gut::Nahrung => f(self.wirtschaft.lager_roh),
                Gut::Legierung | Gut::Elektronik | Gut::Konsumgut => f(self.wirtschaft.lager_verarbeitet),
                _ => f(self.wirtschaft.lager_selten),
            };
        }
        a
    }

    /// Vor Plünderung geschützte Menge je Gut.
    pub fn bunkerschutz(&self, stufe: u8) -> [i64; GUETER] {
        let voll = self.wirtschaft.bunker_je_stufe * M * stufe as i64;
        let mut a = [0i64; GUETER];
        for g in Gut::ALLE {
            a[g.idx()] = match g {
                Gut::Erz | Gut::Kristall | Gut::Deuterium | Gut::Nahrung => voll,
                _ => mal(voll, self.wirtschaft.bunker_verarbeitet_anteil),
            };
        }
        a
    }

    pub fn fenster(&self) -> i64 {
        self.welt.fenster_sekunden
    }

    pub fn epochenende(&self) -> SimZeit {
        self.welt.epoche_tage * TAG
    }
}
