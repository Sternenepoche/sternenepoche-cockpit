//! Weltzustand und Erzeugung der Galaxie.
//!
//! Schlichte Tabellen mit typisierten Kennungen, feste Arrays mit Aufzählungen als Index,
//! keine HashMap: jede Iteration läuft in fester Reihenfolge.

use crate::regeln::{preis_array, Regelwerk};
use crate::typen::*;
use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use std::sync::Arc;

pub const KANAL_GALAXIE: u64 = 1;
pub const KANAL_KAMPF: u64 = 2;
pub const KANAL_SPIONAGE: u64 = 3;
pub const KANAL_REIHENFOLGE: u64 = 4;
pub const KANAL_SIMULATOR: u64 = 5;

/// Eigener Zufallsstrom je Zweck und laufender Nummer, abgeleitet aus dem Startwert der Epoche.
pub fn strom(startwert: u64, kanal: u64, nr: u64) -> ChaCha8Rng {
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&startwert.to_le_bytes());
    seed[8..16].copy_from_slice(&kanal.to_le_bytes());
    seed[16..24].copy_from_slice(&nr.to_le_bytes());
    ChaCha8Rng::from_seed(seed)
}

/// Gleichverteilte Zahl in `0..n`.
pub fn zufall(r: &mut ChaCha8Rng, n: u64) -> u64 {
    if n == 0 {
        0
    } else {
        r.next_u64() % n
    }
}

pub fn mischen<T>(r: &mut ChaCha8Rng, v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = zufall(r, (i + 1) as u64) as usize;
        v.swap(i, j);
    }
}

// Index in `Planet::faktor`.
pub const F_ERZ: usize = 0;
pub const F_KRISTALL: usize = 1;
pub const F_DEUTERIUM: usize = 2;
pub const F_NAHRUNG: usize = 3;
pub const F_SOLAR: usize = 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct System {
    pub sektor: u8,
    pub nummer: u8,
    /// Reichtum an Erz und Kristall in Promille.
    pub reich_erz: i64,
    pub reich_kristall: i64,
    pub nebel: bool,
    pub guertel: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Platz {
    pub felder: u16,
    pub zone: Zone,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bauauftrag {
    pub gebaeude: Gebaeude,
    pub stufe: u8,
    /// `None`: wartet auf Ressourcen oder auf den Auftrag davor.
    pub fertig: Option<SimZeit>,
    /// Work duration captured when payment starts the job. Zero in older saves.
    // Kept out of historical bincode layouts; V7 stores this UI timing extension.
    #[serde(skip)]
    pub dauer: SimZeit,
    pub topf: Option<Topf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Produkt {
    Einheit(Einheit),
    Bauteil(Gut),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fertigung {
    pub produkt: Produkt,
    pub rest: i64,
    /// Sekunden je Stück.
    pub dauer: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Malus {
    pub betrag: i64,
    pub start: SimZeit,
    pub ende: SimZeit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Planet {
    pub id: PlanetId,
    pub besitzer: SpielerId,
    pub koord: Koord,
    pub zone: Zone,
    pub felder: u16,
    pub heimat: bool,
    /// Ertragsfaktoren in Promille: Erz, Kristall, Deuterium, Nahrung, Solar.
    pub faktor: [i64; 5],
    /// Bestand je Gut in Tausendsteln.
    pub bestand: [i64; GUETER],
    /// Rate je Gut in Tausendsteln pro Spielstunde, gültig seit `stand`.
    pub rate: [i64; GUETER],
    pub stand: SimZeit,
    #[serde(with = "crate::snapshot_layout::buildings")]
    pub gebaeude: [u8; GEBAEUDE],
    /// Schiffe und Verteidigung auf dem Planeten.
    #[serde(with="crate::snapshot_layout::units")]
    pub einheiten: [i64; EINHEITEN],
    /// Silo ammunition: interceptor, interplanetary. Work in progress is in the event queue.
    pub raketen: [i64; 2],
    /// Einwohner in Tausendsteln.
    pub bevoelkerung: i64,
    /// 0 bis 100.000.
    pub stabilitaet: i64,
    pub grundwohnraum: i64,
    pub bauschleife: Vec<Bauauftrag>,
    /// 0: Werft, 1: Orbitalwerft.
    pub fertigung: [Vec<Fertigung>; 2],
    pub fertigung_naechste: [SimZeit; 2],
    pub prioritaeten: Vec<Gebaeude>,
    pub mali: Vec<Malus>,
    pub belagerungsmalus: i64,
    /// Feindliche Flotte, die den Orbit hält.
    pub blockade: Option<FlottenId>,
    pub leer_gemeldet: bool,
    pub gegruendet: SimZeit,
    // Zwischenwerte der letzten Ratenrechnung, für Lagebild und Regeln.
    pub energie_erzeugung: i64,
    pub energie_verbrauch: i64,
    pub arbeit_bedarf: i64,
    pub arbeit_verfuegbar: i64,
    pub fk_bedarf: i64,
    pub fk_verfuegbar: i64,
    pub nahrung_deckung: i64,
    pub konsum_deckung: i64,
    pub fp_rate: i64,
    pub stab_ziel: i64,
    pub wohnraum: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Weckzustand {
    pub letzter: SimZeit,
    pub wecker: Option<SimZeit>,
    pub ausloeser: Vec<String>,
    pub dringend: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vorfall {
    pub zeit: SimZeit,
    pub art: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meldung {
    pub zeit: SimZeit,
    pub von: Rolle,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forschungsauftrag {
    pub forschung: Forschung,
    pub stufe: u8,
    pub fp_rest: i64,
    /// UI timing extension, excluded from historical base snapshot layouts.
    #[serde(skip)]
    pub begonnen: Option<SimZeit>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Statistik {
    pub aktionen: u32,
    pub abgelehnt: u32,
    pub angriffe: u32,
    pub verteidigungen: u32,
    pub vertragsbrueche: u32,
    /// Werteinheiten in Tausendsteln.
    pub beute: i64,
    pub verluste: i64,
    pub lagerverlust: i64,
    pub geschenkt: i64,
    pub erhalten: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Punkte {
    pub wirtschaft: i64,
    pub forschung: i64,
    pub militaer: i64,
    pub zivilisation: i64,
}

impl Punkte {
    pub fn gesamt(&self) -> i64 {
        self.wirtschaft + self.forschung + self.militaer + self.zivilisation
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spionagebericht {
    pub zeit: SimZeit,
    pub ziel: Koord,
    pub besitzer: SpielerId,
    pub bestand: Vec<i64>,
    pub schiffe: Option<Vec<i64>>,
    pub verteidigung: Option<Vec<i64>>,
    pub gebaeude: Option<Vec<u8>>,
    pub forschung: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Erkundung {
    pub zeit: SimZeit,
    pub felder: u16,
    pub zone: Zone,
    pub reich_erz: i64,
    pub reich_kristall: i64,
    pub nebel: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spieler {
    pub id: SpielerId,
    pub name: String,
    pub volk: Volk,
    /// Wird über Rollen geweckt. Skriptbots haben hier `false`.
    pub ki: bool,
    pub stufe: u8,
    pub heimat: PlanetId,
    pub planeten: Vec<PlanetId>,
    /// Credits in Tausendsteln.
    pub credits: i64,
    pub steuersatz: u8,
    #[serde(with = "crate::snapshot_layout::research")]
    pub forschung: [u8; FORSCHUNGEN],
    pub forschung_aktiv: Option<Forschungsauftrag>,
    pub forschung_schlange: Vec<(Forschung, PlanetId, Option<Topf>)>,
    /// Sekunden, seit denen alle Bedingungen der nächsten Stufe erfüllt sind.
    pub stufen_zaehler: i64,
    pub schutz_bis: SimZeit,
    pub schuld_seit: Option<SimZeit>,
    /// Guthaben der Töpfe in Werteinheiten (Tausendstel).
    pub toepfe: [i64; TOEPFE],
    pub anteile: [u8; TOEPFE],
    pub doktrin: String,
    pub notizen: [String; ROLLEN],
    pub meldungen: Vec<Meldung>,
    pub weck: [Weckzustand; ROLLEN],
    pub vorfaelle: Vec<Vorfall>,
    pub berichte: Vec<Spionagebericht>,
    pub erkundet: BTreeMap<Koord, Erkundung>,
    pub nachrichten_heute: u16,
    pub punkte: Punkte,
    pub rang: u16,
    pub statistik: Statistik,
    pub allianz: Option<u32>,
    /// Spieler, die dieser Spieler schon angegriffen hat.
    pub angegriffen: Vec<SpielerId>,
    pub xeno_verbaut: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flottenzustand {
    Hinflug,
    ImOrbit,
    Rueckflug,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flotte {
    pub id: FlottenId,
    pub besitzer: SpielerId,
    pub start: PlanetId,
    pub start_koord: Koord,
    pub ziel: Koord,
    pub mission: Mission,
    #[serde(with="crate::snapshot_layout::ships")]
    pub schiffe: [i64; SCHIFFE],
    pub ladung: [i64; GUETER],
    /// Siedler an Bord (Tausendstel).
    pub siedler: i64,
    pub abflug: SimZeit,
    pub ankunft: SimZeit,
    pub flugdauer: i64,
    pub rueckkehr: SimZeit,
    pub orbit_ende: SimZeit,
    pub haltedauer: i64,
    pub zustand: Flottenzustand,
    /// Dem Ziel schon als Warnung gemeldet.
    pub gemeldet: bool,
    /// Gemeinsamer Angriff; die eröffnende Flotte trägt ihre eigene Kennung.
    #[serde(default)]
    pub verband: Option<FlottenId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EreignisArt {
    Tick,
    Tag,
    BauFertig { planet: PlanetId },
    FertigungFertig { planet: PlanetId, schleife: u8 },
    FlotteAnkunft { flotte: FlottenId },
    FlotteRueckkehr { flotte: FlottenId },
    OrbitEnde { flotte: FlottenId },
    Marktlieferung { planet: PlanetId, gut: Gut, menge: i64 },
    VertragEnde { vertrag: u32 },
    RaketenFertig { planet: PlanetId, besitzer: SpielerId, art: u8, anzahl: i64 },
    RaketenAnkunft { von: SpielerId, ziel: Koord, anzahl: i64, zieltyp: Einheit },
    // Append only: historical bincode enum discriminants remain unchanged.
    MarktlieferungGebunden { planet: PlanetId, empfaenger: SpielerId, gut: Gut, menge: i64 },
    SensorKontakt { flotte: FlottenId },
    InterneLieferung { angebot: u64 },
    ExpeditionSchritt { flotte:FlottenId },
    TaktischeRakete { salve:u64 },
}

/// Ereignis in der Prioritätswarteschlange. Der Schlüssel aus Zeit, Priorität und
/// Laufnummer ist eindeutig, auch bei gleichem Zeitstempel.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ereignis {
    pub zeit: SimZeit,
    pub prio: u8,
    pub seq: u64,
    pub art: EreignisArt,
}

impl PartialEq for Ereignis {
    fn eq(&self, o: &Self) -> bool {
        self.seq == o.seq
    }
}
impl Eq for Ereignis {}
impl PartialOrd for Ereignis {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Ereignis {
    // Umgekehrt, damit der BinaryHeap das früheste Ereignis zuerst liefert.
    fn cmp(&self, o: &Self) -> Ordering {
        (o.zeit, o.prio, o.seq).cmp(&(self.zeit, self.prio, self.seq))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vertragsstatus {
    Angeboten,
    Aktiv,
    Gekuendigt { ende: SimZeit },
    Beendet,
    Gebrochen,
    Abgelehnt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vertrag {
    pub id: u32,
    pub art: Vertragsart,
    /// Anbieter. Beim Tribut zahlt `a` an `b`.
    pub a: SpielerId,
    pub b: SpielerId,
    pub angeboten: SimZeit,
    pub seit: SimZeit,
    pub status: Vertragsstatus,
    /// Kaution je Seite in Credits (Tausendstel), verfällt bei Bruch an den Geschädigten.
    pub kaution: i64,
    pub tribut_gut: Option<Gut>,
    pub tribut_menge: i64,
    pub tribut_bis: SimZeit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registereintrag {
    pub zeit: SimZeit,
    pub vertrag: u32,
    pub art: String,
    pub a: String,
    pub b: String,
    pub vorgang: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Allianz {
    pub id: u32,
    pub name: String,
    pub mitglieder: Vec<SpielerId>,
    pub eingeladen: Vec<SpielerId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Nachricht {
    pub zeit: SimZeit,
    pub von: SpielerId,
    pub an: Vec<SpielerId>,
    pub allianz: bool,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: u32,
    pub spieler: SpielerId,
    pub planet: PlanetId,
    pub gut: Gut,
    pub seite: Marktseite,
    /// Offene Menge in Tausendsteln.
    pub menge: i64,
    /// Credits je Einheit in Tausendsteln.
    pub preis: i64,
    pub zeit: SimZeit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Handel {
    pub zeit: SimZeit,
    pub gut: Gut,
    pub menge: i64,
    pub preis: i64,
    pub kaeufer: SpielerId,
    pub verkaeufer: SpielerId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sieger {
    Angreifer,
    Verteidiger,
    Unentschieden,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kampfteilnehmer {
    pub flotte: FlottenId,
    pub spieler: SpielerId,
    #[serde(with="crate::snapshot_layout::units")]
    pub vorher: [i64; EINHEITEN],
    #[serde(with="crate::snapshot_layout::units")]
    pub nachher: [i64; EINHEITEN],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kampfbericht {
    pub nr: u64,
    pub zeit: SimZeit,
    pub ort: Koord,
    pub mission: Mission,
    pub angreifer: SpielerId,
    #[serde(default)]
    pub angreifer_gruppen: Vec<Kampfteilnehmer>,
    pub verteidiger: Vec<SpielerId>,
    pub runden: u8,
    pub sieger: Sieger,
    #[serde(with="crate::snapshot_layout::units")]
    pub ang_vorher: [i64; EINHEITEN],
    #[serde(with="crate::snapshot_layout::units")]
    pub ang_nachher: [i64; EINHEITEN],
    #[serde(with="crate::snapshot_layout::units")]
    pub vert_vorher: [i64; EINHEITEN],
    #[serde(with="crate::snapshot_layout::units")]
    pub vert_nachher: [i64; EINHEITEN],
    pub beute: [i64; GUETER],
    pub truemmer: [i64; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Logeintrag {
    pub zeit: SimZeit,
    pub spieler: SpielerId,
    pub rolle: Rolle,
    /// Aktion als JSON-Text.
    pub aktion: String,
    pub ok: bool,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tageswerte {
    pub tag: i64,
    pub spieler: SpielerId,
    pub stufe: u8,
    pub punkte: Punkte,
    pub bevoelkerung: i64,
    pub stabilitaet: i64,
    pub kolonien: u8,
    pub flottenwert: i64,
    pub produktion: i64,
    pub credits: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Welt {
    #[serde(skip)]
    pub ausbau: crate::ausbau::Ausbau,
    #[serde(skip)]
    pub kommunikation: crate::kommunikation::Kommunikation,
    #[serde(skip)]
    pub ausscheiden: crate::ausscheiden::AusscheidenZustand,
    #[serde(skip)]
    pub aufklaerung: crate::aufklaerung::Aufklaerungszustand,
    #[serde(skip)]
    pub scans: BTreeMap<(SpielerId, Koord), crate::sternkarte::Scan>,
    /// Extension lives in the versioned snapshot envelope, keeping old bincode layouts readable.
    #[serde(skip)]
    pub kolonisation: crate::kolonisation::Kolonisationszustand,
    #[serde(skip)]
    pub fachereignisse: Vec<serde_json::Value>,
    /// Mondgeneration; lebt in der STERNE13-Hülle, nie im historischen bincode-Layout.
    #[serde(skip)]
    pub monde: crate::monde::Mondzustand,
    /// Gemerkte Werksbilanzen für `werk_ruht` (nur Laufzeit, nie im Spielstand; siehe `industrie::WerkMerk`).
    #[serde(skip)]
    pub werk_merk: crate::industrie::WerkMerk,
    pub regeln: Arc<Regelwerk>,
    pub startwert: u64,
    pub zeit: SimZeit,
    pub systeme: Vec<System>,
    pub plaetze: Vec<Platz>,
    pub planeten: Vec<Planet>,
    pub belegung: BTreeMap<Koord, PlanetId>,
    pub spieler: Vec<Spieler>,
    pub flotten: BTreeMap<FlottenId, Flotte>,
    pub naechste_flotte: FlottenId,
    pub ereignisse: BinaryHeap<Ereignis>,
    pub seq: u64,
    pub kampf_nr: u64,
    pub spionage_nr: u64,
    /// Trümmerfelder: Erz und Kristall je Koordinate.
    pub truemmer: BTreeMap<Koord, [i64; 2]>,
    pub vertraege: Vec<Vertrag>,
    pub register: Vec<Registereintrag>,
    pub allianzen: Vec<Allianz>,
    pub nachrichten: Vec<Nachricht>,
    pub orders: Vec<Order>,
    pub naechste_order: u32,
    pub handel: Vec<Handel>,
    pub kampfberichte: Vec<Kampfbericht>,
    pub tageswerte: Vec<Tageswerte>,
    /// In diesem Fenster fällige Rollen, in ausgeloster Reihenfolge.
    pub faellig: Vec<Faellig>,
    /// Pro Fenster neu ausgeloste Reihenfolge aller Spieler.
    pub reihenfolge: Vec<SpielerId>,
    /// Hashkette über alle angewandten Aktionen: belegt das Aktionsprotokoll.
    pub log_hash: String,
    pub log_anzahl: u64,
    /// Noch nicht abgeholte Protokolleinträge. Der Aufrufer schreibt sie auf Platte.
    #[serde(skip)]
    pub logpuffer: Vec<Logeintrag>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Faellig {
    pub spieler: SpielerId,
    pub name: String,
    pub rolle: Rolle,
    pub gruende: Vec<String>,
}

const SILBEN: [&str; 24] = [
    "ka", "ro", "ve", "li", "an", "tor", "mi", "zu", "el", "dra", "son", "ir", "ta", "qua", "be", "nor", "xa",
    "ul", "fen", "gho", "ys", "mar", "ten", "ovi",
];

fn name_erzeugen(r: &mut ChaCha8Rng) -> String {
    let n = 2 + zufall(r, 2) as usize;
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(SILBEN[zufall(r, SILBEN.len() as u64) as usize]);
    }
    let mut c = s.chars();
    let erster = c.next().unwrap().to_uppercase().to_string();
    erster + c.as_str()
}

impl Welt {
    /// Erzeugt eine neue Welt mit `anzahl` Spielern aus dem Startwert.
    pub fn neu(regeln: Regelwerk, startwert: u64, anzahl: usize) -> Result<Welt, String> {
        let regeln = Arc::new(regeln);
        let online = regeln.gebaeude.contains_key(&Gebaeude::Geheimdienst);
        let w = &regeln.welt;
        let mut rng = strom(startwert, KANAL_GALAXIE, 0);
        let je_sektor_max = w.systeme_je_sektor as usize / w.start_abstand.max(1) as usize;
        if anzahl == 0 || anzahl > je_sektor_max * w.sektoren as usize {
            return Err(format!("Spielerzahl {anzahl} passt nicht in die Galaxie"));
        }

        let ist_nebel = |n: u8| n % w.nebel_abstand == w.nebel_versatz;
        let mut systeme = Vec::new();
        let mut plaetze = Vec::new();
        let spanne = (milli(w.reichtum_max) - milli(w.reichtum_min) + 1) as u64;
        for s in 1..=w.sektoren {
            for n in 1..=w.systeme_je_sektor {
                systeme.push(System {
                    sektor: s,
                    nummer: n,
                    reich_erz: milli(w.reichtum_min) + zufall(&mut rng, spanne) as i64,
                    reich_kristall: milli(w.reichtum_min) + zufall(&mut rng, spanne) as i64,
                    nebel: if online {zufall(&mut rng,w.nebel_abstand as u64)==0} else {ist_nebel(n)},
                    guertel: if online {zufall(&mut rng,w.guertel_abstand as u64)==0} else {n % w.guertel_abstand == w.guertel_versatz},
                });
                for pos in 1..=w.plaetze_je_system {
                    let zone_position=if online {1+zufall(&mut rng,w.plaetze_je_system as u64) as u8} else {pos};
                    let (zone, zr) = regeln
                        .zonen
                        .iter()
                        .find(|(_, z)| zone_position >= z.von && zone_position <= z.bis)
                        .ok_or_else(|| format!("Position {pos} liegt in keiner Zone"))?;
                    let breite = (zr.felder_max - zr.felder_min + 1) as u64;
                    plaetze.push(Platz { felder: zr.felder_min + zufall(&mut rng, breite) as u16, zone: *zone });
                }
            }
        }

        // Spieler auf Sektoren verteilen: kleine Runden bleiben in Sektor 1, damit sie sich begegnen.
        let mut je_sektor = vec![0usize; w.sektoren as usize];
        if anzahl <= je_sektor_max / 2 {
            je_sektor[0] = anzahl;
        } else {
            for i in 0..anzahl {
                je_sektor[i % w.sektoren as usize] += 1;
            }
        }
        let mut startorte: Vec<Koord> = Vec::new();
        for (si, k) in je_sektor.iter().enumerate() {
            let sector_nebel=|n:u8|systeme[si*w.systeme_je_sektor as usize+n as usize-1].nebel;
            let gewaehlt = Self::startsysteme(&mut rng, w.systeme_je_sektor, &sector_nebel, *k, w.start_abstand)?;
            for n in gewaehlt {
                startorte.push(Koord::neu(si as u8 + 1, n, w.heimat_position));
            }
        }
        mischen(&mut rng, &mut startorte);

        // Völker per Los: möglichst gleich große Gruppen.
        let mut voelker: Vec<Volk> = (0..anzahl).map(|i| Volk::ALLE[i % VOELKER]).collect();
        mischen(&mut rng, &mut voelker);

        let mut welt = Welt {
            ausbau: Default::default(),
            kommunikation: Default::default(),
            ausscheiden: Default::default(),
            aufklaerung: crate::aufklaerung::Aufklaerungszustand::neu(regeln.gebaeude.contains_key(&Gebaeude::Geheimdienst)),
            kolonisation: Default::default(),
            scans: Default::default(),
            fachereignisse: Vec::new(),
            monde: Default::default(),
            werk_merk: Default::default(),
            regeln: regeln.clone(),
            startwert,
            zeit: 0,
            systeme,
            plaetze,
            planeten: Vec::new(),
            belegung: BTreeMap::new(),
            spieler: Vec::new(),
            flotten: BTreeMap::new(),
            naechste_flotte: 1,
            ereignisse: BinaryHeap::new(),
            seq: 0,
            kampf_nr: 0,
            spionage_nr: 0,
            truemmer: BTreeMap::new(),
            vertraege: Vec::new(),
            register: Vec::new(),
            allianzen: Vec::new(),
            nachrichten: Vec::new(),
            orders: Vec::new(),
            naechste_order: 1,
            handel: Vec::new(),
            kampfberichte: Vec::new(),
            tageswerte: Vec::new(),
            faellig: Vec::new(),
            reihenfolge: Vec::new(),
            log_hash: String::new(),
            log_anzahl: 0,
            logpuffer: Vec::new(),
        };

        let mut namen: Vec<String> = Vec::new();
        for i in 0..anzahl {
            let mut name = name_erzeugen(&mut rng);
            while namen.contains(&name) {
                name = name_erzeugen(&mut rng);
            }
            namen.push(name.clone());
            let pid = welt.planeten.len() as PlanetId;
            let mut gebaeude = [0u8; GEBAEUDE];
            for (g, st) in &w.start_gebaeude {
                gebaeude[g.idx()] = *st;
            }
            welt.planeten.push(Planet {
                id: pid,
                besitzer: i as SpielerId,
                koord: startorte[i],
                zone: Zone::Leben,
                felder: w.heimat_felder,
                heimat: true,
                faktor: [1000; 5],
                bestand: preis_array(&w.start_bestand),
                rate: [0; GUETER],
                stand: 0,
                gebaeude,
                einheiten: [0; EINHEITEN],
                raketen: [0; 2],
                bevoelkerung: w.start_bevoelkerung * M,
                stabilitaet: milli(w.start_stabilitaet),
                grundwohnraum: w.grundwohnraum * M,
                bauschleife: Vec::new(),
                fertigung: [Vec::new(), Vec::new()],
                fertigung_naechste: [-1, -1],
                prioritaeten: Vec::new(),
                mali: Vec::new(),
                belagerungsmalus: 0,
                blockade: None,
                leer_gemeldet: false,
                gegruendet: 0,
                energie_erzeugung: 0,
                energie_verbrauch: 0,
                arbeit_bedarf: 0,
                arbeit_verfuegbar: 0,
                fk_bedarf: 0,
                fk_verfuegbar: 0,
                nahrung_deckung: 1000,
                konsum_deckung: 0,
                fp_rate: 0,
                stab_ziel: milli(w.start_stabilitaet),
                wohnraum: 0,
            });
            welt.belegung.insert(startorte[i], pid);

            let mut anteile = [0u8; TOEPFE];
            for t in Topf::ALLE {
                anteile[t.idx()] = regeln.agenten.start_anteile[&t];
            }
            let startwert_gueter = regeln.wert(&preis_array(&w.start_bestand));
            let mut toepfe = [0i64; TOEPFE];
            for t in Topf::ALLE {
                toepfe[t.idx()] = startwert_gueter * anteile[t.idx()] as i64 / 100;
            }
            // Beim ersten Fenster sind alle Rollen fällig.
            let weck = |rolle: Rolle| Weckzustand {
                letzter: -regeln.agenten.takt_stunden[&rolle] * STUNDE,
                ..Default::default()
            };
            welt.spieler.push(Spieler {
                id: i as SpielerId,
                name,
                volk: voelker[i],
                ki: true,
                stufe: 1,
                heimat: pid,
                planeten: vec![pid],
                credits: w.start_credits * M,
                steuersatz: w.start_steuersatz,
                forschung: [0; FORSCHUNGEN],
                forschung_aktiv: None,
                forschung_schlange: Vec::new(),
                stufen_zaehler: 0,
                schutz_bis: regeln.diplomatie.anfaengerschutz_tage * TAG,
                schuld_seit: None,
                toepfe,
                anteile,
                doktrin: String::new(),
                notizen: Default::default(),
                meldungen: Vec::new(),
                weck: [weck(Rolle::Stratege), weck(Rolle::Verwalter), weck(Rolle::Feldherr), weck(Rolle::Diplomat)],
                vorfaelle: Vec::new(),
                berichte: Vec::new(),
                erkundet: BTreeMap::new(),
                nachrichten_heute: 0,
                punkte: Punkte::default(),
                rang: 0,
                statistik: Statistik::default(),
                allianz: None,
                angegriffen: Vec::new(),
                xeno_verbaut: 0,
            });
        }

        for pid in 0..welt.planeten.len() {
            welt.raten_neu(pid);
        }
        welt.punkte_neu();
        if welt.aufklaerungsregeln() { welt.kolonisationsregeln_v2_aktivieren(); }
        welt.plane(STUNDE, EreignisArt::Tick);
        welt.plane(TAG, EreignisArt::Tag);
        welt.fenster_vorbereiten();
        Ok(welt)
    }

    /// Wählt `k` Startsysteme eines Sektors mit Mindestabstand, nie in einem Nebel.
    fn startsysteme(
        rng: &mut ChaCha8Rng,
        n_sys: u8,
        ist_nebel: &dyn Fn(u8) -> bool,
        k: usize,
        abstand: u8,
    ) -> Result<Vec<u8>, String> {
        if k == 0 {
            return Ok(Vec::new());
        }
        let passt = |gewaehlt: &Vec<u8>, c: u8| gewaehlt.iter().all(|g| (*g as i16 - c as i16).abs() >= abstand as i16);
        for _ in 0..500 {
            let mut kandidaten: Vec<u8> = (1..=n_sys).filter(|s| !ist_nebel(*s)).collect();
            mischen(rng, &mut kandidaten);
            let mut gewaehlt: Vec<u8> = Vec::new();
            for c in kandidaten {
                if passt(&gewaehlt, c) {
                    gewaehlt.push(c);
                    if gewaehlt.len() == k {
                        gewaehlt.sort();
                        return Ok(gewaehlt);
                    }
                }
            }
        }
        // Rückfall: der Reihe nach, dichteste Packung.
        let mut gewaehlt: Vec<u8> = Vec::new();
        for c in 1..=n_sys {
            if !ist_nebel(c) && passt(&gewaehlt, c) {
                gewaehlt.push(c);
                if gewaehlt.len() == k {
                    return Ok(gewaehlt);
                }
            }
        }
        Err(format!("{k} Startsysteme mit Abstand {abstand} passen nicht in einen Sektor"))
    }

    pub fn plane(&mut self, zeit: SimZeit, art: EreignisArt) {
        let prio = match art {
            EreignisArt::ExpeditionSchritt{..} => 8,
            EreignisArt::SensorKontakt { .. } => 0,
            EreignisArt::InterneLieferung { .. } => 4,
            EreignisArt::BauFertig { .. } => 1,
            EreignisArt::FertigungFertig { .. } | EreignisArt::RaketenFertig { .. } => 2,
            EreignisArt::FlotteRueckkehr { .. } => 3,
            EreignisArt::OrbitEnde { .. } => 4,
            EreignisArt::FlotteAnkunft { .. } | EreignisArt::RaketenAnkunft { .. } | EreignisArt::TaktischeRakete {..} => 5,
            EreignisArt::Marktlieferung { .. } | EreignisArt::MarktlieferungGebunden { .. } => 6,
            EreignisArt::VertragEnde { .. } => 7,
            EreignisArt::Tick => 8,
            EreignisArt::Tag => 9,
        };
        self.seq += 1;
        self.ereignisse.push(Ereignis { zeit, prio, seq: self.seq, art });
    }

    pub fn system(&self, k: Koord) -> Option<&System> {
        let w = &self.regeln.welt;
        if k.sektor == 0 || k.sektor > w.sektoren || k.system == 0 || k.system > w.systeme_je_sektor {
            return None;
        }
        self.systeme.get((k.sektor as usize - 1) * w.systeme_je_sektor as usize + k.system as usize - 1)
    }

    pub fn platz(&self, k: Koord) -> Option<&Platz> {
        let w = &self.regeln.welt;
        self.system(k)?;
        if k.position == 0 || k.position > w.plaetze_je_system {
            return None;
        }
        let si = (k.sektor as usize - 1) * w.systeme_je_sektor as usize + k.system as usize - 1;
        self.plaetze.get(si * w.plaetze_je_system as usize + k.position as usize - 1)
    }

    pub fn spieler_nach_name(&self, name: &str) -> Result<SpielerId, String> {
        let n = name.trim();
        self.spieler
            .iter()
            .find(|s| self.spieler_aktiv(s.id) && s.name.eq_ignore_ascii_case(n))
            .map(|s| s.id)
            .ok_or_else(|| format!("Spieler '{n}' gibt es nicht"))
    }

    pub fn eigener_planet(&self, sid: SpielerId, k: Koord) -> Result<usize, String> {
        if self.ist_besiegt(sid) {return Err("Reich besiegt; keine weiteren Spielaktionen".into());}
        if self.aufklaerungsregeln() {
            return self.spieler[sid as usize].planeten.iter().copied().find(|pid|self.planeten[*pid as usize].koord==k)
                .map(|pid|pid as usize).ok_or_else(||format!("{k} ist kein eigener Planet"));
        }
        match self.belegung.get(&k) {
            Some(pid) if self.planeten[*pid as usize].besitzer == sid => Ok(*pid as usize),
            Some(_) => Err(format!("{k} gehört dir nicht")),
            None => Err(format!("auf {k} liegt kein Planet")),
        }
    }

    pub fn vorfall(&mut self, sid: SpielerId, art: &str, text: String) {
        self.fachereignis("world.notice",sid,serde_json::json!({"kind":art,"text":text}));
        let zeit = self.zeit;
        self.spieler[sid as usize].vorfaelle.push(Vorfall { zeit, art: art.to_string(), text });
    }

    /// Merkt einen Auslöser für eine Rolle vor. Dringende Auslöser umgehen den Mindestabstand.
    pub fn wecke(&mut self, sid: SpielerId, rolle: Rolle, grund: &str, dringend: bool) {
        if rolle == Rolle::Alle {
            return;
        }
        let w = &mut self.spieler[sid as usize].weck[rolle.idx()];
        if !w.ausloeser.iter().any(|a| a == grund) {
            w.ausloeser.push(grund.to_string());
        }
        w.dringend |= dringend;
    }
}
