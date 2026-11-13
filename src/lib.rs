//! Zento – Kunden- und Auftragsverwaltung, Angebote & Rechnungen als PDF,
//! Einnahmen-/Ausgaben-Übersicht. Komplett in Rust (WebAssembly + Android-APK).
//!
//! * `engine`: Datenmodell, Rechnen mit Geld und Datum, PDF-Erzeugung, Auswertungen (reines Rust, getestet).
//! * `web`: Oberfläche im Browser bzw. in der Android-WebView – nur für WebAssembly.

pub mod engine;

#[cfg(target_arch = "wasm32")]
mod web;
