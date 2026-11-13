//! Eingebettete Schriften (PDF/A verlangt eingebettete Schriftprogramme): Liberation Sans,
//! metrisch kompatibel zu Helvetica/Arial, Lizenz SIL OFL 1.1 (siehe assets/fonts/LICENSE-Liberation.txt).
//! Es ist nur die WinAnsi-Teilmenge enthalten (erzeugt mit `cargo run --example schriften`).

pub const REGULAR: &[u8] = include_bytes!("../../assets/fonts/LiberationSans-Regular.ttf");
pub const BOLD: &[u8] = include_bytes!("../../assets/fonts/LiberationSans-Bold.ttf");
