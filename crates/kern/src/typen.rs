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
    // Technik v1 (neue Welten ab 18.10.2026): nur in Regelwerken nach `Welt::technik_aktivieren`, siehe `technik.rs`.
    Plasmatechnik = "plasmatechnik",
]);

/// Historical enum discriminants stay append-only; array positions group all ships first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Einheit {
    #[serde(rename = "spionagesonde")] Spionagesonde,
    #[serde(rename = "kleiner_transporter")] KleinerTransporter,
    #[serde(rename = "grosser_transporter")] GrosserTransporter,
    #[serde(rename = "leichter_jaeger")] LeichterJaeger,
    #[serde(rename = "bergbauschiff")] Bergbauschiff,
    #[serde(rename = "kreuzer")] Kreuzer,
    #[serde(rename = "recycler")] Recycler,
    #[serde(rename = "kolonieschiff")] Kolonieschiff,
    #[serde(rename = "truppentransporter")] Truppentransporter,
    #[serde(rename = "schlachtschiff")] Schlachtschiff,
    #[serde(rename = "bomber")] Bomber,
    #[serde(rename = "zerstoerer")] Zerstoerer,
    #[serde(rename = "raketenwerfer")] Raketenwerfer,
    #[serde(rename = "lasergeschuetz")] Lasergeschuetz,
    #[serde(rename = "ionengeschuetz")] Ionengeschuetz,
    #[serde(rename = "gausskanone")] Gausskanone,
    #[serde(rename = "plasmawerfer")] Plasmawerfer,
    #[serde(rename = "planetenschild")] Planetenschild,
    #[serde(rename = "abfangjaeger")] Abfangjaeger,
    #[serde(rename = "schwerer_jaeger")] SchwererJaeger,
    #[serde(rename = "sturmjaeger")] Sturmjaeger,
    #[serde(rename = "schnellfrachter")] Schnellfrachter,
    #[serde(rename = "schwerlastfrachter")] Schwerlastfrachter,
    #[serde(rename = "raketenkreuzer")] Raketenkreuzer,
    #[serde(rename = "schildkreuzer")] Schildkreuzer,
    #[serde(rename = "flottenkreuzer")] Flottenkreuzer,
    #[serde(rename = "sturmschlachtschiff")] Sturmschlachtschiff,
    #[serde(rename = "traegerschlachtschiff")] Traegerschlachtschiff,
    #[serde(rename = "belagerungsschlachtschiff")] Belagerungsschlachtschiff,
    #[serde(rename = "strategischer_bomber")] StrategischerBomber,
    #[serde(rename = "abfangzerstoerer")] Abfangzerstoerer,
    #[serde(rename = "schwerer_zerstoerer")] SchwererZerstoerer,
    #[serde(rename = "grosses_kolonieschiff")] GrossesKolonieschiff,
    #[serde(rename = "planetenschild_ii")] PlanetenschildII,
    #[serde(rename = "planetenschild_iii")] PlanetenschildIII,
    #[serde(rename = "handelsfrachter")] Handelsfrachter,
    #[serde(rename = "pathfinder")] Pathfinder,
    #[serde(rename = "bergungsschiff")] Bergungsschiff,
    // Mondgeneration (STERNE13): stationäre Solarsonde (Anlage), Megaschiff und Schlachtkreuzer-Klasse.
    #[serde(rename = "solarsonde")] Solarsonde,
    #[serde(rename = "weltentruemmer")] Weltentruemmer,
    #[serde(rename = "schlachtkreuzer")] Schlachtkreuzer,
    #[serde(rename = "abfang_schlachtkreuzer")] AbfangSchlachtkreuzer,
    #[serde(rename = "schwerer_schlachtkreuzer")] SchwererSchlachtkreuzer,
    // Terraforming-Generation (STERNE14, Karl 24.10.2026): Terraformer, nur in Welten mit diesem Schiff im Regelwerk.
    #[serde(rename = "terraformer")] Terraformer,
}
pub const EINHEITEN: usize = 44;
pub const SCHIFFE: usize = 35;
impl Einheit {
    pub const ALLE: [Self; EINHEITEN] = [Self::Spionagesonde, Self::KleinerTransporter, Self::GrosserTransporter, Self::LeichterJaeger, Self::Bergbauschiff, Self::Kreuzer, Self::Recycler, Self::Kolonieschiff, Self::Truppentransporter, Self::Schlachtschiff, Self::Bomber, Self::Zerstoerer, Self::Abfangjaeger, Self::SchwererJaeger, Self::Sturmjaeger, Self::Schnellfrachter, Self::Schwerlastfrachter, Self::Raketenkreuzer, Self::Schildkreuzer, Self::Flottenkreuzer, Self::Sturmschlachtschiff, Self::Traegerschlachtschiff, Self::Belagerungsschlachtschiff, Self::StrategischerBomber, Self::Abfangzerstoerer, Self::SchwererZerstoerer, Self::GrossesKolonieschiff, Self::Handelsfrachter, Self::Pathfinder, Self::Bergungsschiff, Self::Weltentruemmer, Self::Schlachtkreuzer, Self::AbfangSchlachtkreuzer, Self::SchwererSchlachtkreuzer, Self::Terraformer, Self::Raketenwerfer, Self::Lasergeschuetz, Self::Ionengeschuetz, Self::Gausskanone, Self::Plasmawerfer, Self::Planetenschild, Self::PlanetenschildII, Self::PlanetenschildIII, Self::Solarsonde];
    pub fn idx(self) -> usize { let n=self as usize; if n==35 {27} else if n==36 || n==37 {n-8} else if n==38 {43} else if n==43 {34} else if n>=39 {n-9} else if (12..18).contains(&n) {n+23} else if (18..33).contains(&n) {n-6} else if n>=33 {n+8} else {n} }
    pub fn name(self) -> &'static str { match self {Self::Pathfinder=>"pathfinder",Self::Bergungsschiff=>"bergungsschiff",Self::Spionagesonde=>"spionagesonde",Self::KleinerTransporter=>"kleiner_transporter",Self::GrosserTransporter=>"grosser_transporter",Self::LeichterJaeger=>"leichter_jaeger",Self::Bergbauschiff=>"bergbauschiff",Self::Kreuzer=>"kreuzer",Self::Recycler=>"recycler",Self::Kolonieschiff=>"kolonieschiff",Self::Truppentransporter=>"truppentransporter",Self::Schlachtschiff=>"schlachtschiff",Self::Bomber=>"bomber",Self::Zerstoerer=>"zerstoerer",Self::Raketenwerfer=>"raketenwerfer",Self::Lasergeschuetz=>"lasergeschuetz",Self::Ionengeschuetz=>"ionengeschuetz",Self::Gausskanone=>"gausskanone",Self::Plasmawerfer=>"plasmawerfer",Self::Planetenschild=>"planetenschild",Self::Abfangjaeger=>"abfangjaeger",Self::SchwererJaeger=>"schwerer_jaeger",Self::Sturmjaeger=>"sturmjaeger",Self::Schnellfrachter=>"schnellfrachter",Self::Schwerlastfrachter=>"schwerlastfrachter",Self::Raketenkreuzer=>"raketenkreuzer",Self::Schildkreuzer=>"schildkreuzer",Self::Flottenkreuzer=>"flottenkreuzer",Self::Sturmschlachtschiff=>"sturmschlachtschiff",Self::Traegerschlachtschiff=>"traegerschlachtschiff",Self::Belagerungsschlachtschiff=>"belagerungsschlachtschiff",Self::StrategischerBomber=>"strategischer_bomber",Self::Abfangzerstoerer=>"abfangzerstoerer",Self::SchwererZerstoerer=>"schwerer_zerstoerer",Self::GrossesKolonieschiff=>"grosses_kolonieschiff",Self::PlanetenschildII=>"planetenschild_ii",Self::Handelsfrachter=>"handelsfrachter",Self::PlanetenschildIII=>"planetenschild_iii",Self::Solarsonde=>"solarsonde",Self::Weltentruemmer=>"weltentruemmer",Self::Schlachtkreuzer=>"schlachtkreuzer",Self::AbfangSchlachtkreuzer=>"abfang_schlachtkreuzer",Self::SchwererSchlachtkreuzer=>"schwerer_schlachtkreuzer",Self::Terraformer=>"terraformer"} }
    pub fn aus_name(s:&str)->Option<Self>{ match s {"pathfinder"=>Some(Self::Pathfinder),"bergungsschiff"=>Some(Self::Bergungsschiff),"spionagesonde"=>Some(Self::Spionagesonde),"kleiner_transporter"=>Some(Self::KleinerTransporter),"grosser_transporter"=>Some(Self::GrosserTransporter),"leichter_jaeger"=>Some(Self::LeichterJaeger),"bergbauschiff"=>Some(Self::Bergbauschiff),"kreuzer"=>Some(Self::Kreuzer),"recycler"=>Some(Self::Recycler),"kolonieschiff"=>Some(Self::Kolonieschiff),"truppentransporter"=>Some(Self::Truppentransporter),"schlachtschiff"=>Some(Self::Schlachtschiff),"bomber"=>Some(Self::Bomber),"zerstoerer"=>Some(Self::Zerstoerer),"raketenwerfer"=>Some(Self::Raketenwerfer),"lasergeschuetz"=>Some(Self::Lasergeschuetz),"ionengeschuetz"=>Some(Self::Ionengeschuetz),"gausskanone"=>Some(Self::Gausskanone),"plasmawerfer"=>Some(Self::Plasmawerfer),"planetenschild"=>Some(Self::Planetenschild),"abfangjaeger"=>Some(Self::Abfangjaeger),"schwerer_jaeger"=>Some(Self::SchwererJaeger),"sturmjaeger"=>Some(Self::Sturmjaeger),"schnellfrachter"=>Some(Self::Schnellfrachter),"schwerlastfrachter"=>Some(Self::Schwerlastfrachter),"raketenkreuzer"=>Some(Self::Raketenkreuzer),"schildkreuzer"=>Some(Self::Schildkreuzer),"flottenkreuzer"=>Some(Self::Flottenkreuzer),"sturmschlachtschiff"=>Some(Self::Sturmschlachtschiff),"traegerschlachtschiff"=>Some(Self::Traegerschlachtschiff),"belagerungsschlachtschiff"=>Some(Self::Belagerungsschlachtschiff),"strategischer_bomber"=>Some(Self::StrategischerBomber),"abfangzerstoerer"=>Some(Self::Abfangzerstoerer),"schwerer_zerstoerer"=>Some(Self::SchwererZerstoerer),"grosses_kolonieschiff"=>Some(Self::GrossesKolonieschiff),"planetenschild_ii"=>Some(Self::PlanetenschildII),"handelsfrachter"=>Some(Self::Handelsfrachter),"planetenschild_iii"=>Some(Self::PlanetenschildIII),"solarsonde"=>Some(Self::Solarsonde),"weltentruemmer"=>Some(Self::Weltentruemmer),"schlachtkreuzer"=>Some(Self::Schlachtkreuzer),"abfang_schlachtkreuzer"=>Some(Self::AbfangSchlachtkreuzer),"schwerer_schlachtkreuzer"=>Some(Self::SchwererSchlachtkreuzer),"terraformer"=>Some(Self::Terraformer),_=>None} }
    pub fn ist_schiff(self)->bool {self.idx()<SCHIFFE}
    /// Inhalte der Mondgeneration: nur nach `monde_aktivieren` im Regelwerk.
    pub fn ist_mondinhalt(self)->bool {(38..43).contains(&(self as u8))}
}
impl fmt::Display for Einheit {fn fmt(&self,f:&mut fmt::Formatter)->fmt::Result{f.write_str(self.name())}}

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
    MondZerstoeren = "mond_zerstoeren",
    // Terraforming-Generation (STERNE14): Terraformer formen Klima und Kruste eines Platzes (crate::terraform).
    Terraformen = "terraformen",
]);

impl Mission {
    pub fn feindlich(self) -> bool {
        matches!(self, Mission::Angriff | Mission::Blockade | Mission::Invasion | Mission::Bombardieren | Mission::Kampfkolonisieren | Mission::MondZerstoeren)
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
/// Ganze Zahl mit deutschem Tausenderpunkt („1.234“) für Spielertexte.
pub fn tausender(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut aus = String::new();
    for (i, c) in s.chars().enumerate() { if i > 0 && (s.len() - i) % 3 == 0 { aus.push('.'); } aus.push(c); }
    if n < 0 { format!("-{aus}") } else { aus }
}

/// „1 Runde“ / „n Runden“ für Spielertexte (Nachprüfung 25.10.2026: „nach 1 Runden“).
pub fn runden_text(n: impl Into<i64>) -> String { let n = n.into(); if n == 1 { "1 Runde".into() } else { format!("{n} Runden") } }

pub fn zeittext(t: SimZeit) -> String {
    let tag = t / TAG + 1;
    let rest = t % TAG;
    format!("Tag {}, {:02}:{:02}", tag, rest / STUNDE, (rest % STUNDE) / MINUTE)
}

/// Menge aus Tausendsteln als gerundete ganze Zahl für Texte.
pub fn ganz(m: i64) -> i64 {
    m / M
}
