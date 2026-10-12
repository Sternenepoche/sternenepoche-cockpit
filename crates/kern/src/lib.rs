//! Sternenepoche: deterministischer Spielkern.
//!
//! Reine Bibliothek ohne Netz, Dateien oder Uhr. Aus Startwert und Aktionsprotokoll
//! lässt sich jede Epoche exakt nachspielen.

#![recursion_limit = "512"]

pub mod aktion;
pub mod ausscheiden;
pub mod aufklaerung;
mod snapshot_layout;
pub mod diplomatie;
pub mod kommunikation;
pub mod erweiterung;
pub mod flotte;
pub mod kampf;
pub mod kolonisation;
pub mod planung;
pub mod markt;
pub mod regeln;
pub mod regeltext;
pub mod sicht;
pub mod sternkarte;
pub mod sim;
pub mod typen;
pub mod umgebung;
pub mod welt;
pub mod wertung;
pub mod wirtschaft;

pub use regeln::Regelwerk;
pub use typen::*;
pub use welt::Welt;
