//! Sternenepoche: deterministischer Spielkern.
//!
//! Reine Bibliothek ohne Netz, Dateien oder Uhr. Aus Startwert und Aktionsprotokoll
//! lässt sich jede Epoche exakt nachspielen.

#![recursion_limit = "512"]

pub mod ablauf;
pub mod technik;
pub mod aktion;
pub mod ausscheiden;
pub mod ausbau;
pub mod habitabilitaet;
pub mod expedition;
pub mod aufklaerung;
pub mod berichte;
mod snapshot_layout;
pub mod diplomatie;
pub mod kommunikation;
pub mod erweiterung;
pub mod raketen;
pub mod flotte;
pub mod flottenausbau;
pub mod kampf;
pub mod zielwahl;
pub mod kolonisation;
pub mod planung;
pub mod planer;
pub mod markt;
pub mod regeln;
pub mod regeltext;
pub mod sicht;
pub mod spielhilfe;
pub mod sternkarte;
pub mod sim;
pub mod typen;
pub mod umgebung;
pub mod welt;
pub mod wertung;
pub mod warteschlangen;
pub mod wirtschaft;
pub mod verwaltung;
pub mod industrie;
pub mod boden;
pub mod serien;

pub use regeln::Regelwerk;
pub use typen::*;
pub use welt::Welt;

pub mod flugkoordination;

mod migration034;

pub mod parallel;
pub mod parallel_silo;

pub mod boden_katalog;
pub mod boden_kampf;
pub mod miliz;
pub mod invasion;
pub mod invasion2;
pub mod orbitflotten;
pub mod monde;
pub mod dyson;
pub mod kernwerte;
pub mod quellen;
pub mod schiffhandel;
pub mod terraform;
