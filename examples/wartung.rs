//! Beispiel-Wartungsprotokoll als PDF: `cargo run --example wartung -- <Ordner>`
use wwp::engine::anlagen::{AnlagenArt, Pruefung};
use wwp::engine::backup::demo;
use wwp::engine::pdf;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut d = demo("2026-11-13");
    let k = d.kunden[0].id;
    let a = d.neue_anlage(Some(k));
    {
        let x = d.anlage_mut(a).unwrap();
        x.art = AnlagenArt::GasBrennwert;
        x.hersteller = "Vaillant".into();
        x.modell = "ecoTEC plus VC 206/5-5".into();
        x.seriennummer = "21183400100245".into();
        x.baujahr = "2018".into();
        x.leistung = "20 kW".into();
        x.standort = "Keller, Heizraum".into();
    }
    let w = d.wartung_starten(a, "Max Monteur", "2026-11-13").unwrap();
    {
        let x = d.wartung_mut(w).unwrap();
        for p in x.pruefpunkte.iter_mut() {
            p.status = Pruefung::Ok;
        }
        x.pruefpunkte[6].status = Pruefung::Mangel;
        x.pruefpunkte[6].notiz = "Siphon stark verschmutzt, gereinigt – Dichtung bei nächster Wartung tauschen".into();
        for (m, v) in x.messwerte.iter_mut().zip(["46", "18", "9,1", "4,6", "12", "1,2", "20", "1,6", "1,0"]) {
            m.wert = v.into();
        }
        x.arbeiten = "Wartung nach Herstellervorgabe durchgeführt, Brenner und Wärmetauscher gereinigt, Abgasmessung ohne Beanstandung.".into();
        x.maengel = "Kondensatsiphon-Dichtung porös – Austausch bei nächster Wartung empfohlen.".into();
    }
    d.wartung_abschliessen(w);
    let bytes = pdf::wartung_pdf(&d, d.wartung(Some(w)).unwrap(), None);
    let p = format!("{dir}/Wartungsprotokoll.pdf");
    std::fs::write(&p, bytes).unwrap();
    println!("{p}");
}
