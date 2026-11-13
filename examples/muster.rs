//! Erzeugt eine Musterrechnung und ein Musterangebot: `cargo run --example muster -- <Ordner>`
use wwp::engine::model::*;
use wwp::engine::pdf;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let heute = "2026-11-12";
    let mut d = Data::new();
    let f = &mut d.firma;
    f.name = "Muster Haustechnik".into();
    f.zusatz = "Inh. Max Muster".into();
    f.strasse = "Werkstraße 5".into();
    f.plz_ort = "80331 München".into();
    f.telefon = "089 1234567".into();
    f.email = "info@muster-haustechnik.de".into();
    f.web = "www.muster-haustechnik.de".into();
    f.bank = "Stadtsparkasse München".into();
    f.iban = "DE89 3704 0044 0532 0130 00".into();
    f.bic = "SSKMDEMMXXX".into();
    f.steuernummer = "143/123/45678".into();
    f.ust_id = "DE123456789".into();
    let k = d.neuer_kunde(heute);
    let kd = d.kunde_mut(k).unwrap();
    kd.firma = "Bäckerei Schön GmbH".into();
    kd.anrede = "Frau".into();
    kd.ansprechpartner = "Anna Schön".into();
    kd.strasse = "Am Markt 3".into();
    kd.plz = "80333".into();
    kd.ort = "München".into();
    for art in [DokArt::Rechnung, DokArt::Angebot] {
        let r = d.neues_dokument(art, Some(k), None, heute);
        let doc = d.dokument_mut(r).unwrap();
        doc.betreff = "Wartung der Heizungsanlage".into();
        doc.positionen = vec![
            Position { text: "Wartung Gasbrennwertkessel\nReinigung, Abgasmessung, Prüfprotokoll".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 18_900, ust: 19 },
            Position { text: "Arbeitszeit Monteur".into(), menge: 2.5, einheit: "Std.".into(), preis: 6_500, ust: 19 },
            Position { text: "Dichtungssatz".into(), menge: 2.0, einheit: "Stk.".into(), preis: 1_290, ust: 19 },
            Position { text: "Anfahrt".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 3_500, ust: 19 },
        ];
        let doc = d.dokument(Some(r)).unwrap().clone();
        let bytes = pdf::dokument_pdf(&d, &doc, None);
        let name = format!("{}/{}.pdf", dir, doc.nummer);
        std::fs::write(&name, bytes).unwrap();
        println!("{name}");
    }
}
