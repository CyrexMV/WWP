//! Komplett-Backup (alle Daten + Dateien wie Logo und Belege) als eine JSON-Datei,
//! außerdem Base64 und Beispieldaten zum Ausprobieren.

use super::model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn b64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut val = [255u8; 256];
    for (i, &c) in T.iter().enumerate() {
        val[c as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0;
    for &c in s.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = val[c as usize];
        if v == 255 {
            return None;
        }
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Backup {
    format: String,
    version: u32,
    erstellt: String,
    data: Data,
    files: BTreeMap<String, String>,
}

pub fn make_backup(data: &Data, files: &[(Id, Vec<u8>)], heute: &str) -> String {
    let b = Backup {
        format: "wwp-backup".into(),
        version: 1,
        erstellt: heute.into(),
        data: data.clone(),
        files: files.iter().map(|(id, bytes)| (id.to_string(), b64_encode(bytes))).collect(),
    };
    serde_json::to_string(&b).unwrap_or_default()
}

/// Inhalt eines Backups: Daten, Dateien (ID, Bytes) und Erstellungsdatum.
pub type Restored = (Data, Vec<(Id, Vec<u8>)>, String);

/// Backup lesen → (Daten, Dateien, Erstellungsdatum)
pub fn parse_backup(s: &str) -> Result<Restored, String> {
    let b: Backup = serde_json::from_str(s.trim_start_matches('\u{feff}')).map_err(|_| "Keine gültige Backup-Datei".to_string())?;
    if b.format != "wwp-backup" {
        return Err("Keine Zento-Backup-Datei".into());
    }
    let data = Data::from_json(&serde_json::to_string(&b.data).map_err(|e| e.to_string())?)?;
    let mut files = Vec::new();
    for (k, v) in b.files {
        if let (Ok(id), Some(bytes)) = (k.parse::<Id>(), b64_decode(&v)) {
            files.push((id, bytes));
        }
    }
    Ok((data, files, b.erstellt))
}

/// Beispieldaten zum Kennenlernen der App.
pub fn demo(heute: &str) -> Data {
    use super::date::add_days;
    let mut d = Data::new();
    let f = &mut d.firma;
    f.name = "Muster Haustechnik".into();
    f.zusatz = "Inh. Max Muster".into();
    f.strasse = "Werkstraße 5".into();
    f.plz_ort = "80331 München".into();
    f.telefon = "089 1234567".into();
    f.email = "info@muster-haustechnik.de".into();
    f.bank = "Musterbank".into();
    f.iban = "DE89 3704 0044 0532 0130 00".into();
    f.steuernummer = "143/123/45678".into();
    d.katalog = vec![
        Leistung { id: 0, text: "Arbeitszeit Monteur".into(), einheit: "Std.".into(), preis: 6_500, ust: 19, ..Default::default() },
        Leistung { id: 0, text: "Anfahrt (Pauschale)".into(), einheit: "Pausch.".into(), preis: 3_500, ust: 19, ..Default::default() },
        Leistung { id: 0, text: "Wartung Heizungsanlage".into(), einheit: "Pausch.".into(), preis: 18_900, ust: 19, ..Default::default() },
    ];
    for i in 0..d.katalog.len() {
        d.katalog[i].id = d.new_id();
    }
    let kunden = [
        ("Bäckerei Schön GmbH", "Frau", "Anna Schön", "Am Markt 3", "80333", "München"),
        ("", "Herr", "Peter Klein", "Lindenweg 12", "85521", "Ottobrunn"),
        ("Hausverwaltung Nord KG", "Frau", "Sabine Weber", "Nordallee 40", "80805", "München"),
    ];
    let mut kid = Vec::new();
    for (firma, anrede, ap, str_, plz, ort) in kunden {
        let id = d.neuer_kunde(&add_days(heute, -60));
        let k = d.kunde_mut(id).unwrap();
        k.firma = firma.into();
        k.anrede = anrede.into();
        k.ansprechpartner = ap.into();
        k.strasse = str_.into();
        k.plz = plz.into();
        k.ort = ort.into();
        k.email = format!("{}@example.com", ap.split_whitespace().last().unwrap_or("kunde").to_lowercase());
        kid.push(id);
    }
    let auftraege = [
        (0, "Wartung Heizung Filiale", AuftragStatus::Abgerechnet, 30_000),
        (1, "Bad-Armaturen tauschen", AuftragStatus::InArbeit, 85_000),
        (2, "Heizungscheck 12 Wohnungen", AuftragStatus::Angebot, 240_000),
        (1, "Wasserhahn tropft", AuftragStatus::Anfrage, 0),
    ];
    let mut aid = Vec::new();
    for (k, titel, st, wert) in auftraege {
        let id = d.neuer_auftrag(Some(kid[k]), &add_days(heute, -20));
        let a = d.auftrag_mut(id).unwrap();
        a.titel = titel.into();
        a.wert = wert;
        a.termin = add_days(heute, 10);
        d.auftrag_status(id, st, heute);
        aid.push(id);
    }
    // bezahlte Rechnung
    let r1 = d.neues_dokument(DokArt::Rechnung, Some(kid[0]), Some(aid[0]), &add_days(heute, -25));
    {
        let r = d.dokument_mut(r1).unwrap();
        r.positionen = vec![
            Position { text: "Wartung Heizungsanlage".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 18_900, ust: 19, ..Default::default() },
            Position { text: "Arbeitszeit Monteur".into(), menge: 1.5, einheit: "Std.".into(), preis: 6_500, ust: 19, ..Default::default() },
        ];
        r.status = DokStatus::Versendet;
    }
    d.als_bezahlt(r1, &add_days(heute, -12), "Überweisung");
    // ältere, bezahlte Rechnung
    let r0 = d.neues_dokument(DokArt::Rechnung, Some(kid[2]), None, &add_days(heute, -70));
    {
        let r = d.dokument_mut(r0).unwrap();
        r.betreff = "Heizungsnotdienst".into();
        r.positionen = vec![
            Position { text: "Notdienst-Einsatz Wochenende".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 45_000, ust: 19, ..Default::default() },
            Position { text: "Arbeitszeit Monteur".into(), menge: 4.0, einheit: "Std.".into(), preis: 6_500, ust: 19, ..Default::default() },
        ];
        r.status = DokStatus::Versendet;
    }
    d.als_bezahlt(r0, &add_days(heute, -58), "Überweisung");
    // offene, überfällige Rechnung
    let r2 = d.neues_dokument(DokArt::Rechnung, Some(kid[1]), Some(aid[1]), &add_days(heute, -20));
    {
        let r = d.dokument_mut(r2).unwrap();
        r.betreff = "Abschlag Bad-Armaturen".into();
        r.positionen = vec![Position { text: "Abschlagszahlung Material".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 40_000, ust: 19, ..Default::default() }];
        r.status = DokStatus::Versendet;
    }
    // Auftragsbestätigung (noch ohne Rechnung)
    let a1 = d.neues_dokument(DokArt::Auftragsbestaetigung, Some(kid[2]), Some(aid[2]), &add_days(heute, -5));
    {
        let a = d.dokument_mut(a1).unwrap();
        a.positionen = vec![
            Position { text: "Heizungscheck je Wohnung\nSichtprüfung, Entlüften, Druck prüfen".into(), menge: 12.0, einheit: "Stk.".into(), preis: 15_000, ust: 19, ..Default::default() },
            Position { text: "Anfahrt (Pauschale)".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 3_500, ust: 19, ..Default::default() },
        ];
        a.status = DokStatus::Versendet;
        a.versendet_am = add_days(heute, -5);
    }
    // Anlagen (eine bald fällig, eine überfällig)
    for (k, art, hersteller, modell, sn, baujahr, standort, tage) in [
        (0usize, super::anlagen::AnlagenArt::GasBrennwert, "Vaillant", "ecoTEC plus VC 206/5-5", "21183400100245", "2018", "Keller, Heizraum", -350i64),
        (1, super::anlagen::AnlagenArt::Waermepumpe, "Viessmann", "Vitocal 250-A", "7891234500017", "2022", "Garten, Außeneinheit links", -380),
    ] {
        let id = d.neue_anlage(Some(kid[k]));
        let x = d.anlage_mut(id).unwrap();
        x.art = art;
        x.hersteller = hersteller.into();
        x.modell = modell.into();
        x.seriennummer = sn.into();
        x.baujahr = baujahr.into();
        x.standort = standort.into();
        x.letzte_wartung = add_days(heute, tage);
    }
    // Ausgaben
    let ausgaben = [
        (-40, 23_800, 19, "Material / Wareneinkauf", "Großhandel Sanitär", "Armaturen, Dichtungen"),
        (-30, 8_900, 19, "Fahrzeugkosten", "Tankstelle", "Diesel"),
        (-15, 3_999, 19, "Telefon & Internet", "Mobilfunk AG", "Handyvertrag"),
        (-8, 1_190, 19, "Software & Abos", "Cloud GmbH", "Speicherplatz"),
    ];
    for (tage, brutto, ust, kat, partner, text) in ausgaben {
        let id = d.neue_buchung(BuchArt::Ausgabe, &add_days(heute, tage));
        let b = d.buchung_mut(id).unwrap();
        b.brutto = brutto;
        b.ust = ust;
        b.kategorie = kat.into();
        b.partner = partner.into();
        b.text = text.into();
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for s in [&b""[..], b"a", b"ab", b"abc", b"\x00\xff\x10hello world"] {
            assert_eq!(b64_decode(&b64_encode(s)).unwrap(), s);
        }
        assert_eq!(b64_encode(b"Man"), "TWFu");
    }

    #[test]
    fn backup_roundtrip() {
        let d = demo("2026-11-12");
        let s = make_backup(&d, &[(5, vec![1, 2, 3])], "2026-11-12");
        let (e, files, when) = parse_backup(&s).unwrap();
        assert_eq!(d, e);
        assert_eq!(files, vec![(5, vec![1, 2, 3])]);
        assert_eq!(when, "2026-11-12");
        assert!(parse_backup("{}").is_err());
    }

    #[test]
    fn demo_is_consistent() {
        let d = demo("2026-11-12");
        assert_eq!(d.kunden.len(), 3);
        assert!(d.dokumente.iter().any(|x| x.ueberfaellig("2026-11-12")));
        assert!(d.buchungen.iter().any(|b| b.dokument_id.is_some()));
    }
}
