//! Auswertungen für Übersicht und Steuer: Jahres-/Monats-/Quartalssummen, Kategorien, CSV-Export.

use super::date;
use super::model::{BuchArt, Buchung, Data, DokArt, DokStatus};
use super::money::{self, Cents};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Summe {
    pub netto: Cents,
    pub steuer: Cents,
    pub brutto: Cents,
}

impl Summe {
    pub fn add(&mut self, b: &Buchung) {
        self.netto += b.netto();
        self.steuer += b.steuer();
        self.brutto += b.brutto;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Monat {
    pub ein: Summe,
    pub aus: Summe,
}

#[derive(Clone, Debug, Default)]
pub struct Auswertung {
    pub jahr: i32,
    /// Betriebliche Einnahmen/Ausgaben (ohne Privatvorgänge und Steuerzahlungen)
    pub ein: Summe,
    pub aus: Summe,
    /// Neutrale Vorgänge (Privatentnahme, Zahlungen ans Finanzamt …)
    pub neutral_ein: Cents,
    pub neutral_aus: Cents,
    pub monate: [Monat; 12],
    pub kat_ein: Vec<(String, Summe, bool)>,
    pub kat_aus: Vec<(String, Summe, bool)>,
    pub anzahl: usize,
}

impl Auswertung {
    /// Gewinn (vereinfacht, netto): Einnahmen − Ausgaben.
    pub fn gewinn(&self) -> Cents {
        self.ein.netto - self.aus.netto
    }

    /// Umsatzsteuer − Vorsteuer
    pub fn zahllast(&self) -> Cents {
        self.ein.steuer - self.aus.steuer
    }

    /// Quartal 1–4: (Umsatzsteuer, Vorsteuer)
    pub fn quartal(&self, q: usize) -> (Cents, Cents) {
        let mut u = 0;
        let mut v = 0;
        for m in &self.monate[(q - 1) * 3..q * 3] {
            u += m.ein.steuer;
            v += m.aus.steuer;
        }
        (u, v)
    }
}

pub fn auswertung(d: &Data, jahr: i32) -> Auswertung {
    let mut a = Auswertung { jahr, ..Default::default() };
    let mut ke: BTreeMap<String, (Summe, bool)> = BTreeMap::new();
    let mut ka: BTreeMap<String, (Summe, bool)> = BTreeMap::new();
    for b in d.buchungen.iter().filter(|b| date::year(&b.datum) == Some(jahr)) {
        a.anzahl += 1;
        let m = date::month(&b.datum).unwrap_or(1).clamp(1, 12) as usize - 1;
        let neutral = b.neutral();
        let kat = if b.kategorie.trim().is_empty() { "Ohne Kategorie".to_string() } else { b.kategorie.clone() };
        match b.art {
            BuchArt::Einnahme => {
                ke.entry(kat).or_insert((Summe::default(), neutral)).0.add(b);
                if neutral {
                    a.neutral_ein += b.brutto;
                } else {
                    a.ein.add(b);
                    a.monate[m].ein.add(b);
                }
            }
            BuchArt::Ausgabe => {
                ka.entry(kat).or_insert((Summe::default(), neutral)).0.add(b);
                if neutral {
                    a.neutral_aus += b.brutto;
                } else {
                    a.aus.add(b);
                    a.monate[m].aus.add(b);
                }
            }
        }
    }
    let sort = |m: BTreeMap<String, (Summe, bool)>| {
        let mut v: Vec<(String, Summe, bool)> = m.into_iter().map(|(k, (s, n))| (k, s, n)).collect();
        v.sort_by(|x, y| x.2.cmp(&y.2).then(y.1.brutto.cmp(&x.1.brutto)));
        v
    };
    a.kat_ein = sort(ke);
    a.kat_aus = sort(ka);
    a
}

/// Alle Jahre mit Buchungen oder Rechnungen, plus das aktuelle – absteigend.
pub fn jahre(d: &Data, heute: &str) -> Vec<i32> {
    let mut v: Vec<i32> = d
        .buchungen
        .iter()
        .filter_map(|b| date::year(&b.datum))
        .chain(d.dokumente.iter().filter_map(|x| date::year(&x.datum)))
        .chain(date::year(heute))
        .collect();
    v.sort_unstable();
    v.dedup();
    v.reverse();
    v
}

fn csv_field(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\"").replace(['\r', '\n'], " "))
    } else {
        s.to_string()
    }
}

fn csv_line(fields: &[String]) -> String {
    let mut s = fields.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(";");
    s.push_str("\r\n");
    s
}

/// Buchungsliste als CSV (Excel/Steuerberater: Semikolon, Dezimalkomma, UTF-8 mit BOM).
pub fn csv_buchungen(d: &Data, jahr: Option<i32>) -> String {
    let mut out = String::from("\u{feff}");
    out += &csv_line(
        &["Datum", "Art", "Kategorie", "Beschreibung", "Kunde/Lieferant", "Zahlart", "Netto", "USt-Satz", "USt", "Brutto", "Beleg", "Rechnung", "Gewinnneutral"]
            .map(String::from),
    );
    let mut list: Vec<&Buchung> = d.buchungen.iter().filter(|b| jahr.is_none() || date::year(&b.datum) == jahr).collect();
    list.sort_by(|a, b| a.datum.cmp(&b.datum).then(a.id.cmp(&b.id)));
    for b in list {
        let sign = if b.art == BuchArt::Ausgabe { -1 } else { 1 };
        out += &csv_line(&[
            date::german(&b.datum),
            b.art.label().into(),
            b.kategorie.clone(),
            b.text.clone(),
            b.partner.clone(),
            b.zahlart.clone(),
            money::fmt_num(sign * b.netto()),
            format!("{} %", b.ust),
            money::fmt_num(sign * b.steuer()),
            money::fmt_num(sign * b.brutto),
            d.datei(b.beleg).map(|f| f.name.clone()).unwrap_or_default(),
            d.dokument(b.dokument_id).map(|x| x.nummer.clone()).unwrap_or_default(),
            if b.neutral() { "ja".into() } else { String::new() },
        ]);
    }
    out
}

/// Rechnungsausgangsbuch als CSV.
pub fn csv_rechnungen(d: &Data, jahr: Option<i32>) -> String {
    let mut out = String::from("\u{feff}");
    out += &csv_line(
        &["Nummer", "Datum", "Kunde", "Betreff", "Netto", "USt", "Brutto", "Status", "Fällig", "Bezahlt am", "Gezahlt", "Offen", "Mahnstufe", "Mahnkosten"]
            .map(String::from),
    );
    let mut list: Vec<_> = d
        .dokumente
        .iter()
        .filter(|x| x.art == DokArt::Rechnung && x.status != DokStatus::Entwurf)
        .filter(|x| jahr.is_none() || date::year(&x.datum) == jahr)
        .collect();
    list.sort_by(|a, b| a.nummer.cmp(&b.nummer));
    for r in list {
        let s = r.summen();
        out += &csv_line(&[
            r.nummer.clone(),
            date::german(&r.datum),
            d.kunden_name(r.kunde_id),
            r.betreff.clone(),
            money::fmt_num(s.netto),
            money::fmt_num(s.ust),
            money::fmt_num(s.brutto),
            r.status.label().into(),
            date::german(&r.faellig),
            date::german(&r.bezahlt_am),
            money::fmt_num(r.gezahlt()),
            money::fmt_num(if r.offen() { r.rest() } else { 0 }),
            r.letzte_mahnung().map(|m| m.label().to_string()).unwrap_or_default(),
            money::fmt_num(r.mahnkosten()),
        ]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::*;

    #[test]
    fn yearly_report() {
        let mut d = Data::new();
        let e = d.neue_buchung(BuchArt::Einnahme, "2026-02-10");
        d.buchung_mut(e).unwrap().brutto = 11_900;
        let a = d.neue_buchung(BuchArt::Ausgabe, "2026-05-03");
        d.buchung_mut(a).unwrap().brutto = 2_380;
        let p = d.neue_buchung(BuchArt::Ausgabe, "2026-05-04");
        {
            let b = d.buchung_mut(p).unwrap();
            b.brutto = 50_000;
            b.ust = 0;
            b.kategorie = "Privatentnahme".into();
        }
        let alt = d.neue_buchung(BuchArt::Einnahme, "2025-12-31");
        d.buchung_mut(alt).unwrap().brutto = 100;

        let r = auswertung(&d, 2026);
        assert_eq!(r.anzahl, 3);
        assert_eq!(r.ein.netto, 10_000);
        assert_eq!(r.aus.netto, 2_000);
        assert_eq!(r.gewinn(), 8_000);
        assert_eq!(r.zahllast(), 1_900 - 380);
        assert_eq!(r.quartal(1), (1_900, 0));
        assert_eq!(r.quartal(2), (0, 380));
        assert_eq!(r.neutral_aus, 50_000);
        assert_eq!(jahre(&d, "2026-11-12"), vec![2026, 2025]);

        let csv = csv_buchungen(&d, Some(2026));
        assert!(csv.contains("10.02.2026;Einnahme;Umsatzerlöse;;;Überweisung;100,00;19 %;19,00;119,00"));
        assert!(csv.contains("-500,00"));
        assert_eq!(csv.lines().count(), 4);
    }
}
