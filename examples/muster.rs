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
            Position {
                text: "Wartung Gasbrennwertkessel\nReinigung, Abgasmessung, Prüfprotokoll".into(),
                menge: 1.0,
                einheit: "Pausch.".into(),
                preis: 18_900,
                ust: 19,
                ..Default::default()
            },
            Position {
                text: "Arbeitszeit Monteur".into(),
                menge: 2.5,
                einheit: "Std.".into(),
                preis: 6_500,
                ust: 19,
                ..Default::default()
            },
            Position {
                text: "Dichtungssatz".into(),
                menge: 2.0,
                einheit: "Stk.".into(),
                preis: 1_290,
                ust: 19,
                ..Default::default()
            },
            Position {
                text: "Anfahrt".into(),
                menge: 1.0,
                einheit: "Pausch.".into(),
                preis: 3_500,
                ust: 19,
                ..Default::default()
            },
        ];
        if art == DokArt::Angebot {
            let x = d.dokument_mut(r).unwrap();
            x.positionen.insert(0, Position { art: PosArt::Titel, text: "Heizung".into(), ..Default::default() });
            x.positionen.insert(3, Position { art: PosArt::Titel, text: "Material und Anfahrt".into(), ..Default::default() });
            x.positionen.insert(4, Position { art: PosArt::Text, text: "Material wird nach tatsächlichem Verbrauch abgerechnet.".into(), ..Default::default() });
            x.positionen.push(Position { art: PosArt::Optional, text: "Hocheffizienzpumpe tauschen".into(), menge: 1.0, einheit: "Stk.".into(), preis: 32_000, ust: 19, ..Default::default() });
            x.positionen[2].rabatt = 10.0;
            x.rabatt_prozent = 5.0;
        } else {
            let x = d.dokument_mut(r).unwrap();
            x.skonto_prozent = 2.0;
            x.skonto_tage = 7;
        }
        let doc = d.dokument(Some(r)).unwrap().clone();
        let bytes = pdf::dokument_pdf(&d, &doc, None);
        let name = format!("{}/{}.pdf", dir, doc.nummer);
        std::fs::write(&name, bytes).unwrap();
        println!("{name}");
        if art == DokArt::Angebot {
            // Auftragsbestätigung (ohne Unterschrift) und unterschriebener Lieferschein
            let ab = d
                .folgedokument(r, DokArt::Auftragsbestaetigung, heute)
                .unwrap();
            let ls = d.folgedokument(ab, DokArt::Lieferschein, heute).unwrap();
            let striche: Vec<Vec<[f32; 3]>> = (0..3)
                .map(|k| {
                    (0..60)
                        .map(|i| {
                            let t = i as f32 / 60.0;
                            [
                                0.05 + k as f32 * 0.3 + t * 0.28,
                                0.16 + (t * 12.0 + k as f32).sin() * 0.07 * (1.0 - t * 0.5),
                                0.3 + 0.6 * (t * 3.1).sin().abs(),
                            ]
                        })
                        .collect()
                })
                .collect();
            d.unterschreiben(
                ls,
                Unterschrift {
                    striche,
                    name: "Anna Schön".into(),
                    ort: "München".into(),
                    datum: heute.into(),
                },
                heute,
            );
            for id in [ab, ls] {
                let doc = d.dokument(Some(id)).unwrap().clone();
                let name = format!("{}/{}.pdf", dir, doc.nummer);
                std::fs::write(&name, pdf::dokument_pdf(&d, &doc, None)).unwrap();
                println!("{name}");
            }
        }
        if art == DokArt::Rechnung {
            // Mahnung mit Gebühr und Verzugszinsen nach einer Teilzahlung
            d.dokument_mut(r).unwrap().status = DokStatus::Versendet;
            d.mahnung_erstellen(r, 0, "2026-12-01", "2026-12-08", 0, false);
            d.zahlung_erfassen(r, "2026-12-03", 10_000, "Überweisung");
            let i = d
                .mahnung_erstellen(r, 1, "2026-12-20", "2026-12-30", 500, true)
                .unwrap();
            let doc = d.dokument(Some(r)).unwrap().clone();
            let name = format!("{}/Mahnung-{}.pdf", dir, doc.nummer);
            std::fs::write(&name, pdf::mahnung_pdf(&d, &doc, i, None).unwrap()).unwrap();
            println!("{name}");
        }
    }
}
