//! Sternenepoche-Harness: ein eigener, lokal gebauter Agent, der ausschließlich über die
//! Harness-API des Servers spielt. Bausteine (Cargo-Features) wählt der Harness Builder.
pub mod gedaechtnis;
pub mod kaefig;
pub mod konfig;
pub mod modell;
pub mod prompt;
pub mod routinen;
pub mod schleife;
pub mod selbsttest;
pub mod server;
pub mod wachposten;
pub mod wissen;
#[cfg(feature = "lokale-api")]
pub mod lokale_api;
#[cfg(feature = "wasm")]
pub mod plugin;
