//! Datenmodell: Firma, Kunden, Aufträge (inkl. Anfragen), Angebote/Rechnungen, Buchungen.
//!
//! Alles wird als JSON gespeichert. `#[serde(default)]` sorgt dafür, dass ältere Datenstände
//! mit fehlenden Feldern weiterhin geladen werden.

use super::date;
use super::money::{self, Cents};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Id = u64;

// ---------------------------------------------------------------------------
// Firma / Einstellungen
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Firma {
    pub name: String,
    /// z. B. „Inhaber Max Mustermann“
    pub zusatz: String,
    pub strasse: String,
    pub plz_ort: String,
    pub telefon: String,
    pub email: String,
    pub web: String,
    pub steuernummer: String,
    pub ust_id: String,
    pub bank: String,
    pub iban: String,
    pub bic: String,
    /// Kleinunternehmerregelung (§ 19 UStG): keine Umsatzsteuer auf Rechnungen.
    pub kleinunternehmer: bool,
    pub ust_standard: u8,
    pub zahlungsziel: u32,
    pub angebot_gueltig: u32,
    pub prefix_angebot: String,
    pub prefix_rechnung: String,
    pub einleitung_angebot: String,
    pub schluss_angebot: String,
    pub einleitung_rechnung: String,
    pub schluss_rechnung: String,
    pub mail_betreff_angebot: String,
    pub mail_text_angebot: String,
    pub mail_betreff_rechnung: String,
    pub mail_text_rechnung: String,
    /// Akzentfarbe der PDFs als „#rrggbb“.
    pub farbe: String,
    /// Logo (Datei-ID im Dateispeicher).
    pub logo: Option<Id>,
    pub logo_breite_mm: f32,
}

impl Default for Firma {
    fn default() -> Self {
        Firma {
            name: String::new(),
            zusatz: String::new(),
            strasse: String::new(),
            plz_ort: String::new(),
            telefon: String::new(),
            email: String::new(),
            web: String::new(),
            steuernummer: String::new(),
            ust_id: String::new(),
            bank: String::new(),
            iban: String::new(),
            bic: String::new(),
            kleinunternehmer: false,
            ust_standard: 19,
            zahlungsziel: 14,
            angebot_gueltig: 30,
            prefix_angebot: "AN".into(),
            prefix_rechnung: "RE".into(),
            einleitung_angebot: "vielen Dank für Ihre Anfrage. Gerne unterbreiten wir Ihnen folgendes Angebot:".into(),
            schluss_angebot: "Dieses Angebot ist gültig bis zum {gueltig}. Wir freuen uns auf Ihren Auftrag!".into(),
            einleitung_rechnung: "vielen Dank für Ihren Auftrag. Wir berechnen Ihnen folgende Leistungen:".into(),
            schluss_rechnung: "Bitte überweisen Sie den Betrag von {betrag} bis zum {faellig} unter Angabe der Rechnungsnummer {nummer}.".into(),
            mail_betreff_angebot: "Angebot {nummer} – {firma}".into(),
            mail_text_angebot: "Guten Tag {ansprechpartner},\n\nanbei erhalten Sie unser Angebot {nummer} über {betrag}.\n\nBei Fragen sind wir gerne für Sie da.\n\nMit freundlichen Grüßen\n{firma}".into(),
            mail_betreff_rechnung: "Rechnung {nummer} – {firma}".into(),
            mail_text_rechnung: "Guten Tag {ansprechpartner},\n\nanbei erhalten Sie unsere Rechnung {nummer} über {betrag}, zahlbar bis zum {faellig}.\n\nVielen Dank für Ihren Auftrag!\n\nMit freundlichen Grüßen\n{firma}".into(),
            farbe: "#2f5bd3".into(),
            logo: None,
            logo_breite_mm: 45.0,
        }
    }
}

impl Firma {
    /// Absenderzeile über dem Adressfeld.
    pub fn absenderzeile(&self) -> String {
        [self.name.as_str(), self.strasse.as_str(), self.plz_ort.as_str()]
            .iter()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ")
    }

    pub fn ust_satz(&self) -> u8 {
        if self.kleinunternehmer {
            0
        } else {
            self.ust_standard
        }
    }
}

// ---------------------------------------------------------------------------
// Kunden
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Kunde {
    pub id: Id,
    pub nr: String,
    pub firma: String,
    pub anrede: String,
    pub ansprechpartner: String,
    pub strasse: String,
    pub plz: String,
    pub ort: String,
    pub land: String,
    pub email: String,
    pub telefon: String,
    pub ust_id: String,
    pub notizen: String,
    pub erstellt: String,
}

impl Kunde {
    /// Anzeigename: Firma, sonst Ansprechpartner.
    pub fn name(&self) -> String {
        if !self.firma.trim().is_empty() {
            self.firma.trim().to_string()
        } else if !self.ansprechpartner.trim().is_empty() {
            self.ansprechpartner.trim().to_string()
        } else {
            "Unbenannter Kunde".into()
        }
    }

    /// Anschrift für das Adressfeld (mehrzeilig).
    pub fn anschrift(&self) -> String {
        let mut l = Vec::new();
        if !self.firma.trim().is_empty() {
            l.push(self.firma.trim().to_string());
        }
        let person = format!("{} {}", self.anrede.trim(), self.ansprechpartner.trim()).trim().to_string();
        if !person.is_empty() {
            l.push(person);
        }
        if !self.strasse.trim().is_empty() {
            l.push(self.strasse.trim().to_string());
        }
        let ort = format!("{} {}", self.plz.trim(), self.ort.trim()).trim().to_string();
        if !ort.is_empty() {
            l.push(ort);
        }
        if !self.land.trim().is_empty() && !self.land.trim().eq_ignore_ascii_case("deutschland") {
            l.push(self.land.trim().to_uppercase());
        }
        l.join("\n")
    }

    pub fn matches(&self, q: &str) -> bool {
        let q = q.to_lowercase();
        [&self.nr, &self.firma, &self.ansprechpartner, &self.ort, &self.email, &self.telefon, &self.plz]
            .iter()
            .any(|f| f.to_lowercase().contains(&q))
    }
}

// ---------------------------------------------------------------------------
// Aufträge (Anfrage → … → abgerechnet)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
pub enum AuftragStatus {
    #[default]
    Anfrage,
    Angebot,
    Beauftragt,
    InArbeit,
    Erledigt,
    Abgerechnet,
    Abgelehnt,
}

impl AuftragStatus {
    pub const ALLE: [AuftragStatus; 7] = [
        AuftragStatus::Anfrage,
        AuftragStatus::Angebot,
        AuftragStatus::Beauftragt,
        AuftragStatus::InArbeit,
        AuftragStatus::Erledigt,
        AuftragStatus::Abgerechnet,
        AuftragStatus::Abgelehnt,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AuftragStatus::Anfrage => "Anfrage",
            AuftragStatus::Angebot => "Angebot gesendet",
            AuftragStatus::Beauftragt => "Beauftragt",
            AuftragStatus::InArbeit => "In Arbeit",
            AuftragStatus::Erledigt => "Erledigt",
            AuftragStatus::Abgerechnet => "Abgerechnet",
            AuftragStatus::Abgelehnt => "Abgelehnt",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            AuftragStatus::Anfrage => "anfrage",
            AuftragStatus::Angebot => "angebot",
            AuftragStatus::Beauftragt => "beauftragt",
            AuftragStatus::InArbeit => "inarbeit",
            AuftragStatus::Erledigt => "erledigt",
            AuftragStatus::Abgerechnet => "abgerechnet",
            AuftragStatus::Abgelehnt => "abgelehnt",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALLE.iter().copied().find(|s| s.key() == k)
    }

    /// Nächster Schritt im Ablauf.
    pub fn next(self) -> Option<Self> {
        match self {
            AuftragStatus::Anfrage => Some(AuftragStatus::Angebot),
            AuftragStatus::Angebot => Some(AuftragStatus::Beauftragt),
            AuftragStatus::Beauftragt => Some(AuftragStatus::InArbeit),
            AuftragStatus::InArbeit => Some(AuftragStatus::Erledigt),
            AuftragStatus::Erledigt => Some(AuftragStatus::Abgerechnet),
            _ => None,
        }
    }

    pub fn offen(self) -> bool {
        !matches!(self, AuftragStatus::Abgerechnet | AuftragStatus::Abgelehnt)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Verlauf {
    pub datum: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Auftrag {
    pub id: Id,
    pub nr: String,
    pub kunde_id: Option<Id>,
    pub titel: String,
    pub beschreibung: String,
    pub status: AuftragStatus,
    /// Eingang der Anfrage
    pub eingang: String,
    /// Wunschtermin / Frist
    pub termin: String,
    /// geschätzter Auftragswert (netto)
    pub wert: Cents,
    pub notizen: String,
    pub verlauf: Vec<Verlauf>,
}

impl Auftrag {
    pub fn titel(&self) -> String {
        if self.titel.trim().is_empty() {
            "Ohne Titel".into()
        } else {
            self.titel.trim().to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// Angebote und Rechnungen
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum DokArt {
    #[default]
    Angebot,
    Rechnung,
}

impl DokArt {
    pub fn label(self) -> &'static str {
        match self {
            DokArt::Angebot => "Angebot",
            DokArt::Rechnung => "Rechnung",
        }
    }
    pub fn plural(self) -> &'static str {
        match self {
            DokArt::Angebot => "Angebote",
            DokArt::Rechnung => "Rechnungen",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            DokArt::Angebot => "angebot",
            DokArt::Rechnung => "rechnung",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        match k {
            "angebot" => Some(DokArt::Angebot),
            "rechnung" => Some(DokArt::Rechnung),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum DokStatus {
    #[default]
    Entwurf,
    Versendet,
    Angenommen,
    Abgelehnt,
    Bezahlt,
    Storniert,
}

impl DokStatus {
    pub fn label(self) -> &'static str {
        match self {
            DokStatus::Entwurf => "Entwurf",
            DokStatus::Versendet => "Versendet",
            DokStatus::Angenommen => "Angenommen",
            DokStatus::Abgelehnt => "Abgelehnt",
            DokStatus::Bezahlt => "Bezahlt",
            DokStatus::Storniert => "Storniert",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            DokStatus::Entwurf => "entwurf",
            DokStatus::Versendet => "versendet",
            DokStatus::Angenommen => "angenommen",
            DokStatus::Abgelehnt => "abgelehnt",
            DokStatus::Bezahlt => "bezahlt",
            DokStatus::Storniert => "storniert",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        [
            DokStatus::Entwurf,
            DokStatus::Versendet,
            DokStatus::Angenommen,
            DokStatus::Abgelehnt,
            DokStatus::Bezahlt,
            DokStatus::Storniert,
        ]
        .into_iter()
        .find(|s| s.key() == k)
    }
    /// Mögliche Status je Dokumentart.
    pub fn fuer(art: DokArt) -> &'static [DokStatus] {
        match art {
            DokArt::Angebot => &[DokStatus::Entwurf, DokStatus::Versendet, DokStatus::Angenommen, DokStatus::Abgelehnt],
            DokArt::Rechnung => &[DokStatus::Entwurf, DokStatus::Versendet, DokStatus::Bezahlt, DokStatus::Storniert],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Position {
    pub text: String,
    pub menge: f64,
    pub einheit: String,
    /// Einzelpreis netto in Cent
    pub preis: Cents,
    pub ust: u8,
}

impl Default for Position {
    fn default() -> Self {
        Position { text: String::new(), menge: 1.0, einheit: "Stk.".into(), preis: 0, ust: 19 }
    }
}

impl Position {
    pub fn netto(&self) -> Cents {
        money::times(self.preis, self.menge)
    }
}

/// Summen eines Dokuments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summen {
    pub netto: Cents,
    /// (Steuersatz, Nettobasis, Steuer) je Satz, aufsteigend
    pub steuern: Vec<(u8, Cents, Cents)>,
    pub ust: Cents,
    pub brutto: Cents,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Dokument {
    pub id: Id,
    pub art: DokArt,
    pub nummer: String,
    pub kunde_id: Option<Id>,
    pub auftrag_id: Option<Id>,
    /// Empfängeranschrift (beim Anlegen vom Kunden übernommen, danach frei änderbar)
    pub empfaenger: String,
    pub datum: String,
    /// Leistungsdatum / -zeitraum (Pflichtangabe auf Rechnungen)
    pub leistung: String,
    /// Rechnung: fällig am · Angebot: gültig bis
    pub faellig: String,
    pub betreff: String,
    pub einleitung: String,
    pub positionen: Vec<Position>,
    pub schluss: String,
    pub status: DokStatus,
    pub bezahlt_am: String,
    pub buchung_id: Option<Id>,
    /// Bei Stornorechnungen: die stornierte Rechnung
    pub storno_von: Option<Id>,
    /// Aus welchem Angebot die Rechnung entstand
    pub aus_angebot: Option<Id>,
    /// Kleinunternehmer-Hinweis statt Umsatzsteuer
    pub kleinunternehmer: bool,
    pub versendet_am: String,
}

impl Dokument {
    pub fn summen(&self) -> Summen {
        let mut by: BTreeMap<u8, Cents> = BTreeMap::new();
        let mut netto = 0;
        for p in &self.positionen {
            let n = p.netto();
            netto += n;
            let rate = if self.kleinunternehmer { 0 } else { p.ust };
            *by.entry(rate).or_default() += n;
        }
        let steuern: Vec<(u8, Cents, Cents)> = by
            .into_iter()
            .filter(|(r, _)| *r > 0)
            .map(|(r, b)| (r, b, money::tax_of_net(b, r)))
            .collect();
        let ust: Cents = steuern.iter().map(|s| s.2).sum();
        Summen { netto, steuern, ust, brutto: netto + ust }
    }

    /// Unbezahlte, versendete Rechnung mit überschrittener Frist?
    pub fn ueberfaellig(&self, heute: &str) -> bool {
        self.art == DokArt::Rechnung
            && self.status == DokStatus::Versendet
            && !self.faellig.is_empty()
            && self.faellig.as_str() < heute
    }

    pub fn offen(&self) -> bool {
        self.art == DokArt::Rechnung && self.status == DokStatus::Versendet
    }

    /// Versendete Dokumente sollen nicht mehr still verändert werden (GoBD).
    pub fn festgeschrieben(&self) -> bool {
        self.status != DokStatus::Entwurf
    }

    pub fn titel(&self) -> String {
        if self.storno_von.is_some() {
            "Stornorechnung".into()
        } else {
            self.art.label().into()
        }
    }

    /// Anzeigestatus inkl. „Überfällig“.
    pub fn status_text(&self, heute: &str) -> &'static str {
        if self.ueberfaellig(heute) {
            "Überfällig"
        } else {
            self.status.label()
        }
    }

    pub fn status_key(&self, heute: &str) -> &'static str {
        if self.ueberfaellig(heute) {
            "ueberfaellig"
        } else {
            self.status.key()
        }
    }

    /// Platzhalter in Texten ersetzen: {nummer}, {betrag}, {faellig}, {gueltig}, {datum}, {firma},
    /// {kunde}, {ansprechpartner}, {zahlungsziel}.
    pub fn fill(&self, text: &str, firma: &Firma, kunde: Option<&Kunde>) -> String {
        let s = self.summen();
        let ziel = date::diff_days(&self.datum, &self.faellig).unwrap_or(firma.zahlungsziel as i64);
        let ap = kunde
            .map(|k| format!("{} {}", k.anrede.trim(), k.ansprechpartner.trim()).trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        text.replace("{nummer}", &self.nummer)
            .replace("{betrag}", &money::fmt_eur(s.brutto))
            .replace("{faellig}", &date::german(&self.faellig))
            .replace("{gueltig}", &date::german(&self.faellig))
            .replace("{datum}", &date::german(&self.datum))
            .replace("{firma}", firma.name.trim())
            .replace("{kunde}", &kunde.map(|k| k.name()).unwrap_or_default())
            .replace("{ansprechpartner}", &ap)
            .replace("{zahlungsziel}", &ziel.to_string())
            .replace("Guten Tag ,", "Guten Tag,")
    }
}

// ---------------------------------------------------------------------------
// Einnahmen und Ausgaben
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum BuchArt {
    #[default]
    Einnahme,
    Ausgabe,
}

impl BuchArt {
    pub fn label(self) -> &'static str {
        match self {
            BuchArt::Einnahme => "Einnahme",
            BuchArt::Ausgabe => "Ausgabe",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            BuchArt::Einnahme => "einnahme",
            BuchArt::Ausgabe => "ausgabe",
        }
    }
}

/// Kategorien angelehnt an die Anlage EÜR. `neutral`: zählt nicht zum Gewinn
/// (Privatentnahmen, Steuerzahlungen ans Finanzamt).
pub struct Kategorie {
    pub name: &'static str,
    pub neutral: bool,
}

pub const KAT_EINNAHMEN: &[Kategorie] = &[
    Kategorie { name: "Umsatzerlöse", neutral: false },
    Kategorie { name: "Anzahlungen", neutral: false },
    Kategorie { name: "Sonstige Einnahmen", neutral: false },
    Kategorie { name: "USt-Erstattung Finanzamt", neutral: true },
    Kategorie { name: "Privateinlage", neutral: true },
];

pub const KAT_AUSGABEN: &[Kategorie] = &[
    Kategorie { name: "Material / Wareneinkauf", neutral: false },
    Kategorie { name: "Fremdleistungen", neutral: false },
    Kategorie { name: "Werkzeug & Geräte (GWG)", neutral: false },
    Kategorie { name: "Miete & Nebenkosten", neutral: false },
    Kategorie { name: "Fahrzeugkosten", neutral: false },
    Kategorie { name: "Reisekosten", neutral: false },
    Kategorie { name: "Telefon & Internet", neutral: false },
    Kategorie { name: "Software & Abos", neutral: false },
    Kategorie { name: "Bürobedarf & Porto", neutral: false },
    Kategorie { name: "Werbung & Marketing", neutral: false },
    Kategorie { name: "Versicherungen & Beiträge", neutral: false },
    Kategorie { name: "Fortbildung", neutral: false },
    Kategorie { name: "Bewirtung", neutral: false },
    Kategorie { name: "Bankgebühren", neutral: false },
    Kategorie { name: "Sonstige Ausgaben", neutral: false },
    Kategorie { name: "USt-Zahlung Finanzamt", neutral: true },
    Kategorie { name: "Einkommensteuer", neutral: true },
    Kategorie { name: "Privatentnahme", neutral: true },
];

pub fn kategorien(art: BuchArt) -> &'static [Kategorie] {
    match art {
        BuchArt::Einnahme => KAT_EINNAHMEN,
        BuchArt::Ausgabe => KAT_AUSGABEN,
    }
}

pub fn ist_neutral(kat: &str) -> bool {
    KAT_EINNAHMEN.iter().chain(KAT_AUSGABEN).any(|k| k.neutral && k.name == kat)
}

pub const ZAHLARTEN: &[&str] = &["Überweisung", "Bar", "EC-/Kreditkarte", "PayPal", "Lastschrift", "Sonstige"];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Buchung {
    pub id: Id,
    pub art: BuchArt,
    pub datum: String,
    /// Bruttobetrag in Cent
    pub brutto: Cents,
    pub ust: u8,
    pub kategorie: String,
    pub text: String,
    /// Kunde bzw. Lieferant
    pub partner: String,
    pub zahlart: String,
    /// Beleg (Datei-ID)
    pub beleg: Option<Id>,
    /// Zugehörige Rechnung
    pub dokument_id: Option<Id>,
}

impl Buchung {
    pub fn netto(&self) -> Cents {
        money::net_of_gross(self.brutto, self.ust)
    }
    pub fn steuer(&self) -> Cents {
        self.brutto - self.netto()
    }
    pub fn neutral(&self) -> bool {
        ist_neutral(&self.kategorie)
    }
    pub fn matches(&self, q: &str) -> bool {
        let q = q.to_lowercase();
        [&self.text, &self.partner, &self.kategorie, &self.zahlart].iter().any(|f| f.to_lowercase().contains(&q))
            || money::fmt_num(self.brutto).contains(&q)
    }
}

// ---------------------------------------------------------------------------
// Leistungskatalog, Dateien
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Leistung {
    pub id: Id,
    pub text: String,
    pub einheit: String,
    pub preis: Cents,
    pub ust: u8,
}

impl Default for Leistung {
    fn default() -> Self {
        Leistung { id: 0, text: String::new(), einheit: "Std.".into(), preis: 0, ust: 19 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Datei {
    pub id: Id,
    pub name: String,
    pub mime: String,
    pub groesse: u64,
}

// ---------------------------------------------------------------------------
// Gesamtbestand
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Data {
    pub version: u32,
    pub firma: Firma,
    pub kunden: Vec<Kunde>,
    pub auftraege: Vec<Auftrag>,
    pub dokumente: Vec<Dokument>,
    pub buchungen: Vec<Buchung>,
    pub katalog: Vec<Leistung>,
    pub dateien: Vec<Datei>,
    /// Nummernkreise: „RE-2026“ → zuletzt vergebene Nummer
    pub zaehler: BTreeMap<String, u32>,
    pub next_id: Id,
}

pub const EINHEITEN: &[&str] = &["Stk.", "Std.", "Pausch.", "m", "m²", "m³", "kg", "Tag(e)", "km"];

impl Data {
    pub fn new() -> Data {
        Data { version: 1, next_id: 1, ..Default::default() }
    }

    pub fn from_json(s: &str) -> Result<Data, String> {
        let mut d: Data = serde_json::from_str(s).map_err(|e| format!("Daten konnten nicht gelesen werden: {e}"))?;
        // IDs absichern, falls ein Bestand von Hand bearbeitet wurde
        let max = d
            .kunden
            .iter()
            .map(|x| x.id)
            .chain(d.auftraege.iter().map(|x| x.id))
            .chain(d.dokumente.iter().map(|x| x.id))
            .chain(d.buchungen.iter().map(|x| x.id))
            .chain(d.katalog.iter().map(|x| x.id))
            .chain(d.dateien.iter().map(|x| x.id))
            .max()
            .unwrap_or(0);
        d.next_id = d.next_id.max(max + 1);
        if d.version == 0 {
            d.version = 1;
        }
        Ok(d)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn new_id(&mut self) -> Id {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        id
    }

    /// Nächste Nummer eines Nummernkreises, z. B. „RE-2026-0007“.
    fn next_number(&mut self, prefix: &str, year: i32, digits: usize) -> String {
        let p = prefix.trim();
        let key = if p.is_empty() { year.to_string() } else { format!("{p}-{year}") };
        let n = self.zaehler.entry(key.clone()).or_insert(0);
        *n += 1;
        format!("{key}-{:0w$}", n, w = digits)
    }

    pub fn kunde(&self, id: Option<Id>) -> Option<&Kunde> {
        let id = id?;
        self.kunden.iter().find(|k| k.id == id)
    }
    pub fn kunde_mut(&mut self, id: Id) -> Option<&mut Kunde> {
        self.kunden.iter_mut().find(|k| k.id == id)
    }
    pub fn auftrag(&self, id: Option<Id>) -> Option<&Auftrag> {
        let id = id?;
        self.auftraege.iter().find(|k| k.id == id)
    }
    pub fn auftrag_mut(&mut self, id: Id) -> Option<&mut Auftrag> {
        self.auftraege.iter_mut().find(|k| k.id == id)
    }
    pub fn dokument(&self, id: Option<Id>) -> Option<&Dokument> {
        let id = id?;
        self.dokumente.iter().find(|k| k.id == id)
    }
    pub fn dokument_mut(&mut self, id: Id) -> Option<&mut Dokument> {
        self.dokumente.iter_mut().find(|k| k.id == id)
    }
    pub fn buchung(&self, id: Option<Id>) -> Option<&Buchung> {
        let id = id?;
        self.buchungen.iter().find(|k| k.id == id)
    }
    pub fn buchung_mut(&mut self, id: Id) -> Option<&mut Buchung> {
        self.buchungen.iter_mut().find(|k| k.id == id)
    }
    pub fn datei(&self, id: Option<Id>) -> Option<&Datei> {
        let id = id?;
        self.dateien.iter().find(|k| k.id == id)
    }

    pub fn kunden_name(&self, id: Option<Id>) -> String {
        self.kunde(id).map(|k| k.name()).unwrap_or_else(|| "—".into())
    }

    // --- Anlegen ----------------------------------------------------------

    pub fn neuer_kunde(&mut self, heute: &str) -> Id {
        let id = self.new_id();
        let n = self.zaehler.entry("K".into()).or_insert(1000);
        *n += 1;
        let nr = format!("K-{}", n);
        self.kunden.push(Kunde { id, nr, erstellt: heute.into(), land: String::new(), ..Default::default() });
        id
    }

    pub fn neuer_auftrag(&mut self, kunde: Option<Id>, heute: &str) -> Id {
        let id = self.new_id();
        let y = date::year(heute).unwrap_or(2026);
        let nr = self.next_number("A", y, 3);
        self.auftraege.push(Auftrag {
            id,
            nr,
            kunde_id: kunde,
            eingang: heute.into(),
            verlauf: vec![Verlauf { datum: heute.into(), text: "Anfrage aufgenommen".into() }],
            ..Default::default()
        });
        id
    }

    pub fn neues_dokument(&mut self, art: DokArt, kunde: Option<Id>, auftrag: Option<Id>, heute: &str) -> Id {
        let id = self.new_id();
        let y = date::year(heute).unwrap_or(2026);
        let f = self.firma.clone();
        let nummer = match art {
            DokArt::Angebot => self.next_number(&f.prefix_angebot, y, 4),
            DokArt::Rechnung => self.next_number(&f.prefix_rechnung, y, 4),
        };
        let (einleitung, schluss, tage) = match art {
            DokArt::Angebot => (f.einleitung_angebot.clone(), f.schluss_angebot.clone(), f.angebot_gueltig),
            DokArt::Rechnung => (f.einleitung_rechnung.clone(), f.schluss_rechnung.clone(), f.zahlungsziel),
        };
        let kunde_rec = self.kunde(kunde).cloned();
        let auftrag_rec = self.auftrag(auftrag).cloned();
        let empfaenger = kunde_rec.as_ref().map(|k| k.anschrift()).unwrap_or_default();
        let betreff = auftrag_rec.as_ref().map(|a| a.titel.clone()).unwrap_or_default();
        let anrede = kunde_rec.as_ref().map(briefanrede).unwrap_or_else(|| "Sehr geehrte Damen und Herren,".into());
        let mut positionen = Vec::new();
        if let Some(a) = &auftrag_rec {
            if !a.titel.trim().is_empty() && a.wert != 0 {
                positionen.push(Position {
                    text: a.titel.clone(),
                    menge: 1.0,
                    einheit: "Pausch.".into(),
                    preis: a.wert,
                    ust: f.ust_satz(),
                });
            }
        }
        if positionen.is_empty() {
            positionen.push(Position { ust: f.ust_satz(), ..Default::default() });
        }
        self.dokumente.push(Dokument {
            id,
            art,
            nummer,
            kunde_id: kunde,
            auftrag_id: auftrag,
            empfaenger,
            datum: heute.into(),
            leistung: if art == DokArt::Rechnung { date::german(heute) } else { String::new() },
            faellig: date::add_days(heute, tage as i64),
            betreff,
            einleitung: format!("{anrede}\n\n{einleitung}"),
            positionen,
            schluss,
            kleinunternehmer: f.kleinunternehmer,
            ..Default::default()
        });
        id
    }

    /// Angebot → neue Rechnung mit denselben Positionen.
    pub fn rechnung_aus_angebot(&mut self, angebot: Id, heute: &str) -> Option<Id> {
        let a = self.dokument(Some(angebot))?.clone();
        let id = self.neues_dokument(DokArt::Rechnung, a.kunde_id, a.auftrag_id, heute);
        let d = self.dokument_mut(id)?;
        d.positionen = a.positionen.clone();
        d.empfaenger = a.empfaenger.clone();
        d.betreff = a.betreff.clone();
        d.aus_angebot = Some(angebot);
        d.kleinunternehmer = a.kleinunternehmer;
        if let Some(an) = self.dokument_mut(angebot) {
            if an.status == DokStatus::Entwurf || an.status == DokStatus::Versendet {
                an.status = DokStatus::Angenommen;
            }
        }
        Some(id)
    }

    /// Kopie eines Dokuments (neue Nummer, Status Entwurf, heutiges Datum).
    pub fn duplizieren(&mut self, src: Id, heute: &str) -> Option<Id> {
        let s = self.dokument(Some(src))?.clone();
        let id = self.neues_dokument(s.art, s.kunde_id, s.auftrag_id, heute);
        let d = self.dokument_mut(id)?;
        d.positionen = s.positionen;
        d.empfaenger = s.empfaenger;
        d.betreff = s.betreff;
        d.einleitung = s.einleitung;
        d.schluss = s.schluss;
        Some(id)
    }

    /// Stornorechnung: gleiche Positionen mit negativer Menge, Original wird „Storniert“.
    pub fn stornieren(&mut self, src: Id, heute: &str) -> Option<Id> {
        let s = self.dokument(Some(src))?.clone();
        if s.art != DokArt::Rechnung {
            return None;
        }
        let id = self.neues_dokument(DokArt::Rechnung, s.kunde_id, s.auftrag_id, heute);
        let d = self.dokument_mut(id)?;
        d.positionen = s.positionen.iter().map(|p| Position { menge: -p.menge, ..p.clone() }).collect();
        d.empfaenger = s.empfaenger;
        d.betreff = format!("Storno zu Rechnung {}", s.nummer);
        d.einleitung = format!("hiermit stornieren wir die Rechnung {} vom {}.", s.nummer, date::german(&s.datum));
        d.schluss = String::new();
        d.storno_von = Some(src);
        d.kleinunternehmer = s.kleinunternehmer;
        d.faellig = heute.into();
        if let Some(o) = self.dokument_mut(src) {
            o.status = DokStatus::Storniert;
        }
        Some(id)
    }

    /// Rechnung als bezahlt markieren und die Einnahme buchen.
    pub fn als_bezahlt(&mut self, dok: Id, datum: &str, zahlart: &str) -> Option<Id> {
        let d = self.dokument(Some(dok))?.clone();
        let s = d.summen();
        // gemischte Steuersätze: Einnahme nach dem überwiegenden Satz, sonst getrennt buchen
        let rate = s.steuern.iter().max_by_key(|t| t.1).map(|t| t.0).unwrap_or(0);
        let partner = self.kunden_name(d.kunde_id);
        let mut ids = Vec::new();
        if s.steuern.len() > 1 {
            for (r, basis, steuer) in &s.steuern {
                let bid = self.new_id();
                ids.push(bid);
                self.buchungen.push(Buchung {
                    id: bid,
                    art: BuchArt::Einnahme,
                    datum: datum.into(),
                    brutto: basis + steuer,
                    ust: *r,
                    kategorie: "Umsatzerlöse".into(),
                    text: format!("Rechnung {} ({} %)", d.nummer, r),
                    partner: partner.clone(),
                    zahlart: zahlart.into(),
                    beleg: None,
                    dokument_id: Some(dok),
                });
            }
        } else {
            let bid = self.new_id();
            ids.push(bid);
            self.buchungen.push(Buchung {
                id: bid,
                art: BuchArt::Einnahme,
                datum: datum.into(),
                brutto: s.brutto,
                ust: rate,
                kategorie: "Umsatzerlöse".into(),
                text: format!("Rechnung {}", d.nummer),
                partner,
                zahlart: zahlart.into(),
                beleg: None,
                dokument_id: Some(dok),
            });
        }
        let first = ids.first().copied();
        if let Some(x) = self.dokument_mut(dok) {
            x.status = DokStatus::Bezahlt;
            x.bezahlt_am = datum.into();
            x.buchung_id = first;
        }
        if let Some(aid) = d.auftrag_id {
            self.auftrag_status(aid, AuftragStatus::Abgerechnet, datum);
        }
        first
    }

    /// Zahlung zurücknehmen: zugehörige Buchungen löschen, Status wieder „Versendet“.
    pub fn zahlung_zuruecknehmen(&mut self, dok: Id) {
        self.buchungen.retain(|b| b.dokument_id != Some(dok));
        if let Some(x) = self.dokument_mut(dok) {
            x.status = DokStatus::Versendet;
            x.bezahlt_am.clear();
            x.buchung_id = None;
        }
    }

    pub fn auftrag_status(&mut self, id: Id, st: AuftragStatus, heute: &str) {
        if let Some(a) = self.auftrag_mut(id) {
            if a.status != st {
                a.status = st;
                a.verlauf.push(Verlauf { datum: heute.into(), text: format!("Status: {}", st.label()) });
            }
        }
    }

    pub fn neue_buchung(&mut self, art: BuchArt, heute: &str) -> Id {
        let id = self.new_id();
        let ust = self.firma.ust_satz();
        self.buchungen.push(Buchung {
            id,
            art,
            datum: heute.into(),
            ust,
            kategorie: kategorien(art)[0].name.into(),
            zahlart: ZAHLARTEN[0].into(),
            ..Default::default()
        });
        id
    }

    // --- Löschen ----------------------------------------------------------

    /// Kunde löschen; Verweise in Aufträgen/Dokumenten bleiben als Text (Anschrift) erhalten.
    pub fn kunde_loeschen(&mut self, id: Id) {
        self.kunden.retain(|k| k.id != id);
        for a in &mut self.auftraege {
            if a.kunde_id == Some(id) {
                a.kunde_id = None;
            }
        }
        for d in &mut self.dokumente {
            if d.kunde_id == Some(id) {
                d.kunde_id = None;
            }
        }
    }

    pub fn auftrag_loeschen(&mut self, id: Id) {
        self.auftraege.retain(|k| k.id != id);
        for d in &mut self.dokumente {
            if d.auftrag_id == Some(id) {
                d.auftrag_id = None;
            }
        }
    }

    pub fn dokument_loeschen(&mut self, id: Id) {
        self.dokumente.retain(|k| k.id != id);
        for b in &mut self.buchungen {
            if b.dokument_id == Some(id) {
                b.dokument_id = None;
            }
        }
    }

    /// Buchung löschen; liefert die ID eines nicht mehr benötigten Belegs.
    pub fn buchung_loeschen(&mut self, id: Id) -> Option<Id> {
        let beleg = self.buchung(Some(id)).and_then(|b| b.beleg);
        self.buchungen.retain(|k| k.id != id);
        for d in &mut self.dokumente {
            if d.buchung_id == Some(id) {
                d.buchung_id = None;
            }
        }
        beleg
    }

    // --- Abfragen ---------------------------------------------------------

    pub fn dokumente_von(&self, kunde: Id) -> Vec<&Dokument> {
        let mut v: Vec<&Dokument> = self.dokumente.iter().filter(|d| d.kunde_id == Some(kunde)).collect();
        v.sort_by(|a, b| b.datum.cmp(&a.datum).then(b.id.cmp(&a.id)));
        v
    }

    pub fn auftraege_von(&self, kunde: Id) -> Vec<&Auftrag> {
        let mut v: Vec<&Auftrag> = self.auftraege.iter().filter(|d| d.kunde_id == Some(kunde)).collect();
        v.sort_by(|a, b| b.eingang.cmp(&a.eingang));
        v
    }

    /// Bezahlter Umsatz (brutto) eines Kunden.
    pub fn umsatz_kunde(&self, kunde: Id) -> Cents {
        self.dokumente
            .iter()
            .filter(|d| d.kunde_id == Some(kunde) && d.art == DokArt::Rechnung && d.status == DokStatus::Bezahlt)
            .map(|d| d.summen().brutto)
            .sum()
    }

    /// Ist die Datei noch irgendwo in Gebrauch?
    pub fn datei_benutzt(&self, id: Id) -> bool {
        self.firma.logo == Some(id) || self.buchungen.iter().any(|b| b.beleg == Some(id))
    }
}

/// „Sehr geehrte Frau Muster,“ / „Sehr geehrter Herr Muster,“ / „Sehr geehrte Damen und Herren,“
pub fn briefanrede(k: &Kunde) -> String {
    let name = k.ansprechpartner.trim();
    let nachname = name.split_whitespace().last().unwrap_or("");
    match k.anrede.trim() {
        "Frau" if !nachname.is_empty() => format!("Sehr geehrte Frau {nachname},"),
        "Herr" if !nachname.is_empty() => format!("Sehr geehrter Herr {nachname},"),
        _ if !name.is_empty() && k.anrede.trim().is_empty() && k.firma.trim().is_empty() => format!("Guten Tag {name},"),
        _ => "Sehr geehrte Damen und Herren,".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> (Data, Id) {
        let mut d = Data::new();
        d.firma.name = "Muster GmbH".into();
        let k = d.neuer_kunde("2026-11-12");
        let kd = d.kunde_mut(k).unwrap();
        kd.firma = "Kunde AG".into();
        kd.anrede = "Frau".into();
        kd.ansprechpartner = "Erika Mustermann".into();
        kd.strasse = "Hauptstr. 1".into();
        kd.plz = "12345".into();
        kd.ort = "Berlin".into();
        (d, k)
    }

    #[test]
    fn numbering() {
        let (mut d, k) = demo();
        let a = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-11-12");
        let b = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-11-13");
        let c = d.neues_dokument(DokArt::Angebot, Some(k), None, "2026-11-13");
        assert_eq!(d.dokument(Some(a)).unwrap().nummer, "RE-2026-0001");
        assert_eq!(d.dokument(Some(b)).unwrap().nummer, "RE-2026-0002");
        assert_eq!(d.dokument(Some(c)).unwrap().nummer, "AN-2026-0001");
        assert_eq!(d.kunde(Some(k)).unwrap().nr, "K-1001");
        let doc = d.dokument(Some(a)).unwrap();
        assert_eq!(doc.faellig, "2026-11-26");
        assert!(doc.empfaenger.contains("Kunde AG\nFrau Erika Mustermann\nHauptstr. 1\n12345 Berlin"));
        assert!(doc.einleitung.starts_with("Sehr geehrte Frau Mustermann,"));
    }

    #[test]
    fn sums_mixed_rates() {
        let mut doc = Dokument {
            positionen: vec![
                Position { text: "A".into(), menge: 2.0, einheit: "Std.".into(), preis: 5000, ust: 19 },
                Position { text: "B".into(), menge: 1.5, einheit: "kg".into(), preis: 1000, ust: 7 },
            ],
            ..Default::default()
        };
        let s = doc.summen();
        assert_eq!(s.netto, 11_500);
        assert_eq!(s.steuern, vec![(7, 1500, 105), (19, 10_000, 1900)]);
        assert_eq!(s.brutto, 13_505);
        doc.kleinunternehmer = true;
        assert_eq!(doc.summen().brutto, 11_500);
    }

    #[test]
    fn paying_creates_booking() {
        let (mut d, k) = demo();
        let a = d.neuer_auftrag(Some(k), "2026-11-01");
        let r = d.neues_dokument(DokArt::Rechnung, Some(k), Some(a), "2026-11-12");
        d.dokument_mut(r).unwrap().positionen[0].preis = 10_000;
        d.dokument_mut(r).unwrap().status = DokStatus::Versendet;
        assert!(d.dokument(Some(r)).unwrap().ueberfaellig("2026-12-01"));
        let b = d.als_bezahlt(r, "2026-11-20", "Überweisung").unwrap();
        let bu = d.buchung(Some(b)).unwrap();
        assert_eq!(bu.brutto, 11_900);
        assert_eq!(bu.netto(), 10_000);
        assert_eq!(d.auftrag(Some(a)).unwrap().status, AuftragStatus::Abgerechnet);
        assert_eq!(d.umsatz_kunde(k), 11_900);
        d.zahlung_zuruecknehmen(r);
        assert!(d.buchungen.is_empty());
        assert_eq!(d.dokument(Some(r)).unwrap().status, DokStatus::Versendet);
    }

    #[test]
    fn offer_to_invoice_and_storno() {
        let (mut d, k) = demo();
        let an = d.neues_dokument(DokArt::Angebot, Some(k), None, "2026-11-12");
        d.dokument_mut(an).unwrap().positionen[0].preis = 2500;
        let re = d.rechnung_aus_angebot(an, "2026-11-15").unwrap();
        assert_eq!(d.dokument(Some(re)).unwrap().positionen[0].preis, 2500);
        assert_eq!(d.dokument(Some(an)).unwrap().status, DokStatus::Angenommen);
        let st = d.stornieren(re, "2026-11-16").unwrap();
        assert_eq!(d.dokument(Some(st)).unwrap().summen().brutto, -2975);
        assert_eq!(d.dokument(Some(re)).unwrap().status, DokStatus::Storniert);
    }

    #[test]
    fn json_roundtrip_and_defaults() {
        let (d, _) = demo();
        let s = d.to_json();
        let e = Data::from_json(&s).unwrap();
        assert_eq!(d, e);
        // alter/teilweiser Bestand
        let f = Data::from_json(r#"{"kunden":[{"id":7,"firma":"X"}]}"#).unwrap();
        assert_eq!(f.next_id, 8);
        assert_eq!(f.firma.zahlungsziel, 14);
    }

    #[test]
    fn placeholders() {
        let (mut d, k) = demo();
        let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-11-12");
        d.dokument_mut(r).unwrap().positionen[0].preis = 10_000;
        let doc = d.dokument(Some(r)).unwrap();
        let t = doc.fill(&d.firma.schluss_rechnung, &d.firma, d.kunde(Some(k)));
        assert!(t.contains("119,00 €"));
        assert!(t.contains("26.11.2026"));
        assert!(t.contains("RE-2026-0001"));
    }
}
