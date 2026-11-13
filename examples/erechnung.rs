//! Schreibt Beispiel-E-Rechnungen (ZUGFeRD-XML, XRechnung, Kleinunternehmer, Storno, Rabatt/Skonto)
//! zum Prüfen mit den offiziellen Validierungsregeln: `cargo run --example erechnung -- <Ordner>`
use wwp::engine::erechnung::{cii_xml, Profil};
use wwp::engine::model::*;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let heute = "2026-11-12";
    let mut d = Data::new();
    let f = &mut d.firma;
    f.name = "Muster Haustechnik".into();
    f.zusatz = "Max Muster".into();
    f.strasse = "Werkstraße 5".into();
    f.plz_ort = "80331 München".into();
    f.telefon = "089 1234567".into();
    f.email = "info@muster-haustechnik.de".into();
    f.iban = "DE89 3704 0044 0532 0130 00".into();
    f.bic = "COBADEFFXXX".into();
    f.steuernummer = "143/123/45678".into();
    f.ust_id = "DE123456789".into();
    let k = d.neuer_kunde(heute);
    {
        let kd = d.kunde_mut(k).unwrap();
        kd.firma = "Bäckerei Schön GmbH".into();
        kd.ansprechpartner = "Anna Schön".into();
        kd.strasse = "Am Markt 3".into();
        kd.plz = "80333".into();
        kd.ort = "München".into();
        kd.email = "rechnung@schoen.de".into();
        kd.leitweg_id = "991-12345-67".into();
    }
    let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, heute);
    {
        let x = d.dokument_mut(r).unwrap();
        x.betreff = "Wartung der Heizungsanlage".into();
        x.positionen = vec![
            Position { art: PosArt::Titel, text: "Heizung".into(), ..Default::default() },
            Position { text: "Wartung Gasbrennwertkessel\nReinigung, Abgasmessung".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 18_900, ust: 19, ..Default::default() },
            Position { text: "Arbeitszeit Monteur".into(), menge: 2.5, einheit: "Std.".into(), preis: 6_500, ust: 19, rabatt: 10.0, ..Default::default() },
            Position { art: PosArt::Text, text: "Hinweis ohne Preis".into(), ..Default::default() },
            Position { text: "Fachbuch".into(), menge: 1.0, einheit: "Stk.".into(), preis: 3_000, ust: 7, ..Default::default() },
            Position { art: PosArt::Optional, text: "Optionale Pumpe".into(), menge: 1.0, einheit: "Stk.".into(), preis: 32_000, ust: 19, ..Default::default() },
        ];
        x.rabatt_prozent = 5.0;
        x.skonto_prozent = 2.0;
        x.skonto_tage = 7;
    }
    let doc = d.dokument(Some(r)).unwrap().clone();
    std::fs::write(format!("{dir}/zugferd.xml"), cii_xml(&d, &doc, Profil::Zugferd)).unwrap();
    std::fs::write(format!("{dir}/zugferd.pdf"), wwp::engine::pdf::zugferd_pdf(&d, &doc, None)).unwrap();
    std::fs::write(format!("{dir}/xrechnung.xml"), cii_xml(&d, &doc, Profil::XRechnung)).unwrap();
    // ohne USt-IdNr. (nur Steuernummer)
    let mut ohne = d.clone();
    ohne.firma.ust_id.clear();
    std::fs::write(format!("{dir}/ohne-ust-id.xml"), cii_xml(&ohne, ohne.dokument(Some(r)).unwrap(), Profil::XRechnung)).unwrap();
    // Kleinunternehmer
    let mut ku = d.clone();
    ku.dokument_mut(r).unwrap().kleinunternehmer = true;
    std::fs::write(format!("{dir}/kleinunternehmer.xml"), cii_xml(&ku, ku.dokument(Some(r)).unwrap(), Profil::XRechnung)).unwrap();
    // Storno
    d.dokument_mut(r).unwrap().status = DokStatus::Versendet;
    let st = d.stornieren(r, "2026-11-20").unwrap();
    std::fs::write(format!("{dir}/storno.xml"), cii_xml(&d, d.dokument(Some(st)).unwrap(), Profil::XRechnung)).unwrap();
    println!("geschrieben nach {dir}");
}
