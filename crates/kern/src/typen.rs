//! Grundtypen: Zeit, Kennungen, Aufzählungen und Festkommarechnung.
//!
//! Der Weltzustand kennt keine Gleitkommazahlen. Mengen sind Ganzzahlen in
//! Tausendsteln, Faktoren aus dem Regelwerk werden über `mal` und `pot` als
//! Festkomma mit sechs Nachkommastellen angewendet. So rechnet jede Maschine gleich.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Spielsekunden seit Beginn der Epoche.
pub type SimZeit = i64;
pub type SpielerId = u16;
pub type PlanetId = u32;
pub type FlottenId = u32;

pub const MINUTE: i64 = 60;
pub const STUNDE: i64 = 3_600;
pub const TAG: i64 = 86_400;
/// Tausendstel: 1 Einheit eines Guts sind `M` im Zustand.
pub const M: i64 = 1_000;
/// Festkommabasis für Faktoren.
pub const FX: i128 = 1_000_000;

pub fn fx(f: f64) -> i128 {
    (f * 1_000_000.0).round() as i128
}

/// Ganzzahl mal Faktor aus dem Regelwerk.
pub fn mal(a: i64, f: f64) -> i64 {
    ((a as i128 * fx(f)) / FX) as i64
}

/// `basis` hoch `n` als Festkomma (mal `FX`).
pub fn pot(basis: f64, n: u32) -> i128 {
    let b = fx(basis);
    let mut x = FX;
    for _ in 0..n {
        x = x * b / FX;
    }
    x
}

/// `a * zaehler / nenner` ohne Überlauf, bei Nenner 0 gleich 0.
pub fn anteil(a: i64, zaehler: i64, nenner: i64) -> i64 {
    if nenner == 0 {
        0
    } else {
        ((a as i128 * zaehler as i128) / nenner as i128) as i64
    }
}

/// Ganzzahlige Wurzel.
pub fn wurzel(x: i128) -> i128 {
    if x <= 0 {
        return 0;
    }
    let mut r = (x as f64).sqrt() as i128;
    while r * r > x {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= x {
        r += 1;
    }
    r
}

/// Menge im Zustand (Tausendstel) aus einer Zahl im Regelwerk.
pub fn milli(f: f64) -> i64 {
    (f * 1000.0).round() as i64
}

macro_rules! aufzaehlung {
    ($(#[$doc:meta])* $name:ident, $anz:ident, [$($v:ident = $s:tt),+ $(,)?]) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[repr(u8)]
        pub enum $name { $(#[serde(rename = $s)] $v),+ }
        pub const $anz: usize = [$($name::$v),+].len();
        impl $name {
            pub const ALLE: [$name; $anz] = [$($name::$v),+];
            #[inline]
            pub fn idx(self) -> usize { self as usize }
            pub fn name(self) -> &'static str { match self { $($name::$v => $s),+ } }
            pub fn aus_name(s: &str) -> Option<$name> { match s { $($s => Some($name::$v),)+ _ => None } }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str(self.name()) }
        }
    };
}

aufzaehlung!(
    /// Lagerbare Güter. Credits und Forschungspunkte liegen außerhalb.
    Gut, GUETER, [
    Erz = "erz", Kristall = "kristall", Deuterium = "deuterium", Nahrung = "nahrung",
    Legierung = "legierung", Elektronik = "elektronik", Konsumgut = "konsumgut",
    Xenokristall = "xenokristall", Antriebskern = "antriebskern", Habitatmodul = "habitatmodul",
]);

aufzaehlung!(Gebaeude, GEBAEUDE, [
    Erzmine = "erzmine", Kristallmine = "kristallmine", Deuteriumsynthesizer = "deuteriumsynthesizer",
    Farm = "farm", Solarkraftwerk = "solarkraftwerk", Fusionskraftwerk = "fusionskraftwerk",
    Giesserei = "giesserei", Elektronikwerk = "elektronikwerk", Konsumgueterwerk = "konsumgueterwerk",
    Xenoextraktor = "xenoextraktor", Lager = "lager", Bunker = "bunker", Wohnblock = "wohnblock",
    Akademie = "akademie", Markt = "markt", Verwaltungszentrum = "verwaltungszentrum",
    Labor = "labor", Bauhof = "bauhof", Nanofabrik = "nanofabrik", Raumhafen = "raumhafen",
    Werft = "werft", Orbitalwerft = "orbitalwerft", Sensorphalanx = "sensorphalanx", Kaserne = "kaserne",
    Raketensilo = "raketensilo", Orbitalring = "orbitalring", Forschungsarchiv = "forschungsarchiv", Versorgungsnetz = "versorgungsnetz",
    Geheimdienst = "geheimdienst",
]);

aufzaehlung!(Forschung, FORSCHUNGEN, [
    Energietechnik = "energietechnik", Werkstoffkunde = "werkstoffkunde",
    Automatisierung = "automatisierung", Agrarwissenschaft = "agrarwissenschaft",
    Soziologie = "soziologie", Verbrennungsantrieb = "verbrennungsantrieb",
    Impulsantrieb = "impulsantrieb", Hyperraumantrieb = "hyperraumantrieb",
    Astrophysik = "astrophysik", Logistik = "logistik", Computertechnik = "computertechnik",
    Waffentechnik = "waffentechnik", Schildtechnik = "schildtechnik", Panzerung = "panzerung",
    Spionagetechnik = "spionagetechnik", Xenomaterialkunde = "xenomaterialkunde",
    Terraforming = "terraforming", Ueberwachungstechnik = "ueberwachungstechnik", Abschirmtechnik = "abschirmtechnik",
]);

aufzaehlung!(
    /// Schiffe und Verteidigungsanlagen in einer Aufzählung. Die ersten `SCHIFFE` sind Schiffe.
    Einheit, EINHEITEN, [
    Spionagesonde = "spionagesonde", KleinerTransporter = "kleiner_transporter",
    GrosserTransporter = "grosser_transporter", LeichterJaeger = "leichter_jaeger",
    Bergbauschiff = "bergbauschiff", Kreuzer = "kreuzer", Recycler = "recycler",
    Kolonieschiff = "kolonieschiff", Truppentransporter = "truppentransporter",
    Schlachtschiff = "schlachtschiff", Bomber = "bomber", Zerstoerer = "zerstoerer",
    Raketenwerfer = "raketenwerfer", Lasergeschuetz = "lasergeschuetz",
    Ionengeschuetz = "ionengeschuetz", Gausskanone = "gausskanone",
    Plasmawerfer = "plasmawerfer", Planetenschild = "planetenschild",
]);

/// Anzahl der Schiffstypen am Anfang von `Einheit`.
pub const SCHIFFE: usize = 12;

impl Einheit {
    pub fn ist_schiff(self) -> bool {
        self.idx() < SCHIFFE
    }
}

aufzaehlung!(Volk, VOELKER, [
    Aurelianer = "aurelianer", Krath = "krath", Veyari = "veyari", Syntheten = "syntheten",
]);

aufzaehlung!(Zone, ZONEN, [Glut = "glut", Leben = "leben", Frost = "frost"]);

aufzaehlung!(
    /// Die vier Regierungsrollen. `Alle` nutzen Skriptbots, sie umgehen Töpfe und Rollenrechte.
    Rolle, ROLLEN_MIT_ALLE, [
    Stratege = "stratege", Verwalter = "verwalter", Feldherr = "feldherr",
    Diplomat = "diplomat", Alle = "alle",
]);
pub const ROLLEN: usize = 4;

aufzaehlung!(Topf, TOEPFE, [
    Wirtschaft = "wirtschaft", Militaer = "militaer", Forschung = "forschung", Reserve = "reserve",
]);

aufzaehlung!(Mission, MISSIONEN, [
    Angriff = "angriff", Transport = "transport", Stationieren = "stationieren",
    Halten = "halten", Spionage = "spionage", Kolonisieren = "kolonisieren",
    Recyceln = "recyceln", Abbau = "abbau", Blockade = "blockade", Invasion = "invasion",
    Bombardieren = "bombardieren", Kampfkolonisieren = "kampfkolonisieren",
    Saven = "saven", FlottenSpionage = "flotten_spionage", SystemErkunden = "system_erkunden",
]);

impl Mission {
    pub fn feindlich(self) -> bool {
        matches!(self, Mission::Angriff | Mission::Blockade | Mission::Invasion | Mission::Bombardieren | Mission::Kampfkolonisieren)
    }
}

aufzaehlung!(Vertragsart, VERTRAGSARTEN, [
    Nichtangriffspakt = "nichtangriffspakt", Handelsabkommen = "handelsabkommen",
    Verteidigungsbuendnis = "verteidigungsbuendnis", Tribut = "tribut",
]);

aufzaehlung!(Marktseite, MARKTSEITEN, [Kauf = "kauf", Verkauf = "verkauf"]);

/// Koordinate als Sektor, System, Position. Position 0 ist der Asteroidengürtel des Systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Koord {
    pub sektor: u8,
    pub system: u8,
    pub position: u8,
}

impl Koord {
    pub fn neu(sektor: u8, system: u8, position: u8) -> Koord {
        Koord { sektor, system, position }
    }
}

impl fmt::Display for Koord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}:{}", self.sektor, self.system, self.position)
    }
}

impl FromStr for Koord {
    type Err = String;
    fn from_str(s: &str) -> Result<Koord, String> {
        let teile: Vec<&str> = s.trim().split(':').collect();
        if teile.len() != 3 {
            return Err(format!("Koordinate '{s}' hat nicht die Form Sektor:System:Position"));
        }
        let z = |t: &str| t.trim().parse::<u8>().map_err(|_| format!("Koordinate '{s}' ist keine Zahl"));
        Ok(Koord { sektor: z(teile[0])?, system: z(teile[1])?, position: z(teile[2])? })
    }
}

impl Serialize for Koord {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Koord {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Koord, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Zeit als "Tag 12, 08:15" für Texte.
pub fn zeittext(t: SimZeit) -> String {
    let tag = t / TAG + 1;
    let rest = t % TAG;
    format!("Tag {}, {:02}:{:02}", tag, rest / STUNDE, (rest % STUNDE) / MINUTE)
}

/// Menge aus Tausendsteln als gerundete ganze Zahl für Texte.
pub fn ganz(m: i64) -> i64 {
    m / M
}
