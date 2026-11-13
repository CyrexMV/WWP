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
    pub prefix_ab: String,
    pub prefix_lieferschein: String,
    pub einleitung_angebot: String,
    pub schluss_angebot: String,
    pub einleitung_rechnung: String,
    pub schluss_rechnung: String,
    pub mail_betreff_angebot: String,
    pub mail_text_angebot: String,
    pub mail_betreff_rechnung: String,
    pub mail_text_rechnung: String,
    pub einleitung_ab: String,
    pub schluss_ab: String,
    pub einleitung_lieferschein: String,
    pub schluss_lieferschein: String,
    /// E-Mail für Auftragsbestätigungen und Lieferscheine
    pub mail_betreff_dok: String,
    pub mail_text_dok: String,
    /// Akzentfarbe der PDFs als „#rrggbb“.
    pub farbe: String,
    /// Logo (Datei-ID im Dateispeicher).
    pub logo: Option<Id>,
    pub logo_breite_mm: f32,
    /// GiroCode (QR-Code zum Bezahlen) auf Rechnungen und Mahnungen
    pub girocode: bool,
    /// Mahnwesen je Stufe (0 = Zahlungserinnerung, 1 = 1. Mahnung, 2 = 2. Mahnung)
    pub mahn_tage: Vec<u32>,
    pub mahn_gebuehr: Vec<Cents>,
    pub mahn_text: Vec<String>,
    pub mail_betreff_mahnung: String,
    pub mail_text_mahnung: String,
    /// Verzugszinsen berechnen (Basiszinssatz + 5 bzw. 9 Prozentpunkte, § 288 BGB)
    pub verzugszinsen: bool,
    pub basiszins: f64,
}

/// Bezeichnungen der Mahnstufen.
pub const MAHNSTUFEN: [&str; 3] = ["Zahlungserinnerung", "1. Mahnung", "2. Mahnung"];

const MAHN_TAGE: [u32; 3] = [7, 10, 7];
const MAHN_GEBUEHR: [Cents; 3] = [0, 500, 1000];
const MAHN_TEXT: [&str; 3] = [
    "sicher ist es Ihrer Aufmerksamkeit entgangen: Unsere Rechnung {nummer} vom {datum} über {betrag} war am {faellig} fällig. Bis heute konnten wir keinen Zahlungseingang feststellen.\n\nBitte überweisen Sie den offenen Betrag von {gesamt} bis zum {frist}. Sollten Sie die Zahlung bereits veranlasst haben, betrachten Sie dieses Schreiben bitte als gegenstandslos.",
    "leider konnten wir zu unserer Rechnung {nummer} vom {datum} trotz Zahlungserinnerung noch keinen vollständigen Zahlungseingang feststellen.\n\nWir bitten Sie, den offenen Betrag von {gesamt} bis spätestens {frist} zu überweisen.",
    "trotz unserer bisherigen Erinnerungen ist die Rechnung {nummer} vom {datum} noch immer nicht beglichen. Wir fordern Sie hiermit letztmalig auf, den offenen Betrag von {gesamt} bis zum {frist} zu überweisen.\n\nNach Ablauf dieser Frist werden wir ohne weitere Ankündigung gerichtliche Schritte (Mahnverfahren) einleiten.",
];

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
            prefix_ab: "AB".into(),
            prefix_lieferschein: "LS".into(),
            einleitung_ab: "vielen Dank für Ihren Auftrag, den wir hiermit gerne bestätigen. Wir führen folgende Leistungen für Sie aus:".into(),
            schluss_ab: "Bitte prüfen Sie die Angaben. Wir freuen uns auf die Zusammenarbeit!".into(),
            einleitung_lieferschein: "folgende Leistungen bzw. Lieferungen wurden erbracht:".into(),
            schluss_lieferschein: "Die Leistungen wurden vollständig und ordnungsgemäß erbracht.".into(),
            mail_betreff_dok: "{titel} {nummer} – {firma}".into(),
            mail_text_dok: "Guten Tag {ansprechpartner},\n\nanbei erhalten Sie das Dokument „{titel} {nummer}“.\n\nMit freundlichen Grüßen\n{firma}".into(),
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
            girocode: true,
            mahn_tage: MAHN_TAGE.to_vec(),
            mahn_gebuehr: MAHN_GEBUEHR.to_vec(),
            mahn_text: MAHN_TEXT.iter().map(|s| s.to_string()).collect(),
            mail_betreff_mahnung: "{stufe} zu Rechnung {nummer} – {firma}".into(),
            mail_text_mahnung: "Guten Tag {ansprechpartner},\n\nanbei erhalten Sie unsere {stufe} zur Rechnung {nummer} vom {datum}. Offen sind {gesamt}, bitte überweisen Sie den Betrag bis zum {frist}. Die Rechnung liegt als Kopie bei.\n\nSollte sich Ihre Zahlung mit diesem Schreiben überschnitten haben, betrachten Sie es bitte als gegenstandslos.\n\nMit freundlichen Grüßen\n{firma}".into(),
            verzugszinsen: false,
            basiszins: 1.27,
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

    /// Frist (Tage), Gebühr und Text einer Mahnstufe (mit Standardwerten als Rückfall).
    pub fn mahnstufe(&self, stufe: u8) -> (u32, Cents, String) {
        let i = (stufe as usize).min(2);
        (
            self.mahn_tage.get(i).copied().unwrap_or(MAHN_TAGE[i]),
            self.mahn_gebuehr.get(i).copied().unwrap_or(MAHN_GEBUEHR[i]),
            self.mahn_text.get(i).cloned().unwrap_or_else(|| MAHN_TEXT[i].to_string()),
        )
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
    Auftragsbestaetigung,
    Lieferschein,
}

impl DokArt {
    pub const ALLE: [DokArt; 4] = [DokArt::Angebot, DokArt::Auftragsbestaetigung, DokArt::Lieferschein, DokArt::Rechnung];

    pub fn label(self) -> &'static str {
        match self {
            DokArt::Angebot => "Angebot",
            DokArt::Rechnung => "Rechnung",
            DokArt::Auftragsbestaetigung => "Auftragsbestätigung",
            DokArt::Lieferschein => "Lieferschein",
        }
    }
    pub fn plural(self) -> &'static str {
        match self {
            DokArt::Angebot => "Angebote",
            DokArt::Rechnung => "Rechnungen",
            DokArt::Auftragsbestaetigung => "Auftragsbestätigungen",
            DokArt::Lieferschein => "Lieferscheine",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            DokArt::Angebot => "angebot",
            DokArt::Rechnung => "rechnung",
            DokArt::Auftragsbestaetigung => "ab",
            DokArt::Lieferschein => "lieferschein",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALLE.into_iter().find(|a| a.key() == k)
    }
    /// „ein Angebot“, „eine Rechnung“ … für Texte
    pub fn neu_label(self) -> &'static str {
        match self {
            DokArt::Angebot => "Neues Angebot",
            DokArt::Rechnung => "Neue Rechnung",
            DokArt::Auftragsbestaetigung => "Neue Auftragsbestätigung",
            DokArt::Lieferschein => "Neuer Lieferschein",
        }
    }
    /// Zeigt das Dokument Preise und Summen?
    pub fn mit_preisen(self) -> bool {
        self != DokArt::Lieferschein
    }
    /// Kann der Kunde das Dokument unterschreiben?
    pub fn unterschreibbar(self) -> bool {
        self != DokArt::Rechnung
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
    Unterschrieben,
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
            DokStatus::Unterschrieben => "Unterschrieben",
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
            DokStatus::Unterschrieben => "unterschrieben",
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
            DokStatus::Unterschrieben,
        ]
        .into_iter()
        .find(|s| s.key() == k)
    }
    /// Mögliche Status je Dokumentart.
    pub fn fuer(art: DokArt) -> &'static [DokStatus] {
        match art {
            DokArt::Angebot => &[DokStatus::Entwurf, DokStatus::Versendet, DokStatus::Angenommen, DokStatus::Abgelehnt],
            DokArt::Rechnung => &[DokStatus::Entwurf, DokStatus::Versendet, DokStatus::Bezahlt, DokStatus::Storniert],
            DokArt::Auftragsbestaetigung | DokArt::Lieferschein => &[DokStatus::Entwurf, DokStatus::Versendet, DokStatus::Unterschrieben],
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

/// Handschriftliche Unterschrift (z. B. mit dem S-Pen auf dem Tablet).
///
/// Striche als Punktfolgen `[x, y, Druck]`; x und y sind auf die Breite des Unterschriftenfelds
/// normiert (0…1, Feld im Seitenverhältnis 3:1, also y bis ≈ 0,33).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Unterschrift {
    pub striche: Vec<Vec<[f32; 3]>>,
    pub name: String,
    pub ort: String,
    pub datum: String,
}

impl Unterschrift {
    /// Enthält die Unterschrift genug Tinte?
    pub fn gueltig(&self) -> bool {
        self.striche.iter().map(|s| s.len()).sum::<usize>() >= 4
    }
}

/// Seitenverhältnis des Unterschriftenfelds (Breite : Höhe).
pub const UNTERSCHRIFT_VERHAELTNIS: f32 = 3.0;

/// Zahlungserinnerung bzw. Mahnung zu einer Rechnung (Stand zum Zeitpunkt der Erstellung).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Mahnung {
    /// 0 = Zahlungserinnerung, 1 = 1. Mahnung, 2 = 2. Mahnung
    pub stufe: u8,
    pub datum: String,
    /// neue Zahlungsfrist
    pub frist: String,
    /// offene Forderung vor dieser Mahnung (inkl. früherer Gebühren, abzüglich Zahlungen)
    pub offen_vorher: Cents,
    pub gebuehr: Cents,
    pub zinsen: Cents,
    pub zins_satz: f64,
    pub zins_tage: i64,
    pub versendet: bool,
}

impl Mahnung {
    pub fn label(&self) -> &'static str {
        MAHNSTUFEN[(self.stufe as usize).min(2)]
    }
    /// Zu zahlender Gesamtbetrag laut dieser Mahnung.
    pub fn gesamt(&self) -> Cents {
        self.offen_vorher + self.gebuehr + self.zinsen
    }
}

/// Zahlungseingang zu einer Rechnung (auch Teilzahlung).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Zahlung {
    pub id: Id,
    pub datum: String,
    pub betrag: Cents,
    pub zahlart: String,
    /// dazu angelegte Einnahmen
    pub buchungen: Vec<Id>,
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
    pub mahnungen: Vec<Mahnung>,
    pub zahlungen: Vec<Zahlung>,
    /// Unterschrift des Kunden (Angebot angenommen, Auftrag bestätigt, Lieferung erhalten)
    pub unterschrift: Option<Unterschrift>,
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

    /// Bruttobetrag je Steuersatz (inkl. 0 %), für die Aufteilung von Zahlungen.
    pub fn brutto_je_satz(&self) -> Vec<(u8, Cents)> {
        let s = self.summen();
        let mut v: Vec<(u8, Cents)> = s.steuern.iter().map(|(r, b, t)| (*r, b + t)).collect();
        let null = s.netto - s.steuern.iter().map(|t| t.1).sum::<Cents>();
        if null != 0 || v.is_empty() {
            v.insert(0, (0, null));
        }
        v
    }

    /// Mahngebühren und Verzugszinsen aller Mahnungen.
    pub fn mahnkosten(&self) -> Cents {
        self.mahnungen.iter().map(|m| m.gebuehr + m.zinsen).sum()
    }

    /// Gesamtforderung: Rechnungsbetrag + Mahnkosten.
    pub fn forderung(&self) -> Cents {
        self.summen().brutto + self.mahnkosten()
    }

    /// Summe aller Zahlungseingänge.
    pub fn gezahlt(&self) -> Cents {
        self.zahlungen.iter().map(|z| z.betrag).sum()
    }

    /// Noch offener Betrag (inkl. Mahnkosten).
    pub fn rest(&self) -> Cents {
        (self.forderung() - self.gezahlt()).max(0)
    }

    pub fn teilbezahlt(&self) -> bool {
        self.offen() && self.gezahlt() > 0
    }

    /// Letzte Mahnung.
    pub fn letzte_mahnung(&self) -> Option<&Mahnung> {
        self.mahnungen.last()
    }

    /// Nächste sinnvolle Mahnstufe (0–2).
    pub fn naechste_stufe(&self) -> u8 {
        self.letzte_mahnung().map(|m| (m.stufe + 1).min(2)).unwrap_or(0)
    }

    /// Überfällig und (noch nicht gemahnt oder Frist der letzten Mahnung verstrichen)?
    pub fn zu_mahnen(&self, heute: &str) -> bool {
        self.ueberfaellig(heute) && self.storno_von.is_none() && self.letzte_mahnung().map(|m| m.frist.as_str() < heute).unwrap_or(true)
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

    /// Anzeigestatus inkl. „Überfällig“, Mahnstufe und Teilzahlung.
    pub fn status_text(&self, heute: &str) -> &'static str {
        if self.offen() {
            if let Some(m) = self.letzte_mahnung() {
                return m.label();
            }
        }
        if self.ueberfaellig(heute) {
            "Überfällig"
        } else if self.teilbezahlt() {
            "Teilbezahlt"
        } else {
            self.status.label()
        }
    }

    pub fn status_key(&self, heute: &str) -> &'static str {
        if self.offen() && !self.mahnungen.is_empty() {
            "gemahnt"
        } else if self.ueberfaellig(heute) {
            "ueberfaellig"
        } else if self.teilbezahlt() {
            "teilbezahlt"
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
        text.replace("{titel}", &self.titel())
            .replace("{nummer}", &self.nummer)
            .replace("{betrag}", &money::fmt_eur(s.brutto))
            .replace("{faellig}", &date::german(&self.faellig))
            .replace("{gueltig}", &date::german(&self.faellig))
            .replace("{datum}", &date::german(&self.datum))
            .replace("{firma}", firma.name.trim())
            .replace("{kunde}", &kunde.map(|k| k.name()).unwrap_or_default())
            .replace("{ansprechpartner}", &ap)
            .replace("{zahlungsziel}", &ziel.to_string())
            .replace("{offen}", &money::fmt_eur(self.rest()))
            .replace("Guten Tag ,", "Guten Tag,")
    }

    /// Platzhalter einer Mahnung: zusätzlich {stufe}, {frist}, {gesamt}, {gebuehr}.
    pub fn fill_mahnung(&self, text: &str, firma: &Firma, kunde: Option<&Kunde>, m: &Mahnung) -> String {
        let t = text
            .replace("{stufe}", m.label())
            .replace("{frist}", &date::german(&m.frist))
            .replace("{gesamt}", &money::fmt_eur(m.gesamt()))
            .replace("{gebuehr}", &money::fmt_eur(m.gebuehr));
        self.fill(&t, firma, kunde)
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
        Data { version: 2, next_id: 1, ..Default::default() }
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
        // Version 2: Zahlungen werden je Rechnung geführt (Teilzahlungen, Mahnwesen)
        if d.version < 2 {
            let ids: Vec<Id> = d
                .dokumente
                .iter()
                .filter(|x| x.art == DokArt::Rechnung && x.status == DokStatus::Bezahlt && x.zahlungen.is_empty())
                .map(|x| x.id)
                .collect();
            for id in ids {
                let buchungen: Vec<&Buchung> = d.buchungen.iter().filter(|b| b.dokument_id == Some(id)).collect();
                let betrag: Cents = buchungen.iter().map(|b| b.brutto).sum();
                let bids: Vec<Id> = buchungen.iter().map(|b| b.id).collect();
                let zahlart = buchungen.first().map(|b| b.zahlart.clone()).unwrap_or_default();
                let zid = d.new_id();
                if let Some(x) = d.dokument_mut(id) {
                    let betrag = if betrag > 0 { betrag } else { x.summen().brutto };
                    let datum = x.bezahlt_am.clone();
                    x.zahlungen.push(Zahlung { id: zid, datum, betrag, zahlart, buchungen: bids });
                }
            }
            d.version = 2;
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
        let prefix = match art {
            DokArt::Angebot => &f.prefix_angebot,
            DokArt::Rechnung => &f.prefix_rechnung,
            DokArt::Auftragsbestaetigung => &f.prefix_ab,
            DokArt::Lieferschein => &f.prefix_lieferschein,
        };
        let nummer = self.next_number(prefix, y, 4);
        let (einleitung, schluss, tage) = match art {
            DokArt::Angebot => (f.einleitung_angebot.clone(), f.schluss_angebot.clone(), Some(f.angebot_gueltig)),
            DokArt::Rechnung => (f.einleitung_rechnung.clone(), f.schluss_rechnung.clone(), Some(f.zahlungsziel)),
            DokArt::Auftragsbestaetigung => (f.einleitung_ab.clone(), f.schluss_ab.clone(), None),
            DokArt::Lieferschein => (f.einleitung_lieferschein.clone(), f.schluss_lieferschein.clone(), None),
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
            leistung: if matches!(art, DokArt::Rechnung | DokArt::Lieferschein) { date::german(heute) } else { String::new() },
            faellig: tage.map(|t| date::add_days(heute, t as i64)).unwrap_or_default(),
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

    /// Folgedokument aus einem bestehenden (z. B. Angebot → Auftragsbestätigung, Lieferschein → Rechnung):
    /// gleiche Positionen, Anschrift und Betreff, neue Nummer.
    pub fn folgedokument(&mut self, src: Id, art: DokArt, heute: &str) -> Option<Id> {
        let a = self.dokument(Some(src))?.clone();
        if art == DokArt::Rechnung && a.art == DokArt::Angebot {
            return self.rechnung_aus_angebot(src, heute);
        }
        let id = self.neues_dokument(art, a.kunde_id, a.auftrag_id, heute);
        let d = self.dokument_mut(id)?;
        d.positionen = a.positionen.clone();
        d.empfaenger = a.empfaenger.clone();
        d.betreff = a.betreff.clone();
        d.kleinunternehmer = a.kleinunternehmer;
        d.aus_angebot = if a.art == DokArt::Angebot { Some(src) } else { a.aus_angebot };
        if a.art == DokArt::Lieferschein && art == DokArt::Rechnung && !a.leistung.trim().is_empty() {
            d.leistung = a.leistung.clone();
        }
        if a.art == DokArt::Angebot && art == DokArt::Auftragsbestaetigung {
            if let Some(an) = self.dokument_mut(src) {
                if an.status == DokStatus::Entwurf || an.status == DokStatus::Versendet {
                    an.status = DokStatus::Angenommen;
                }
            }
            if let Some(aid) = a.auftrag_id {
                if self.auftrag(Some(aid)).map(|x| x.status <= AuftragStatus::Angebot).unwrap_or(false) {
                    self.auftrag_status(aid, AuftragStatus::Beauftragt, heute);
                }
            }
        }
        Some(id)
    }

    /// Unterschrift des Kunden übernehmen: Angebot → angenommen, Auftragsbestätigung/Lieferschein →
    /// unterschrieben; der Auftrag läuft mit (beauftragt bzw. erledigt).
    pub fn unterschreiben(&mut self, dok: Id, u: Unterschrift, heute: &str) -> bool {
        if !u.gueltig() {
            return false;
        }
        let Some(x) = self.dokument_mut(dok) else { return false };
        if !x.art.unterschreibbar() {
            return false;
        }
        let art = x.art;
        x.unterschrift = Some(u);
        x.status = match art {
            DokArt::Angebot => DokStatus::Angenommen,
            _ => DokStatus::Unterschrieben,
        };
        if x.versendet_am.is_empty() {
            x.versendet_am = heute.into();
        }
        if let Some(aid) = x.auftrag_id {
            let st = self.auftrag(Some(aid)).map(|a| a.status);
            match (art, st) {
                (DokArt::Angebot | DokArt::Auftragsbestaetigung, Some(s)) if s <= AuftragStatus::Angebot => {
                    self.auftrag_status(aid, AuftragStatus::Beauftragt, heute)
                }
                (DokArt::Lieferschein, Some(s)) if s < AuftragStatus::Erledigt => self.auftrag_status(aid, AuftragStatus::Erledigt, heute),
                _ => {}
            }
        }
        true
    }

    /// Unterschrift entfernen (Status zurück auf „Versendet“).
    pub fn unterschrift_entfernen(&mut self, dok: Id) {
        if let Some(x) = self.dokument_mut(dok) {
            x.unterschrift = None;
            if matches!(x.status, DokStatus::Angenommen | DokStatus::Unterschrieben) {
                x.status = DokStatus::Versendet;
            }
        }
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

    /// Rechnung vollständig als bezahlt markieren (Restbetrag) und die Einnahme buchen.
    pub fn als_bezahlt(&mut self, dok: Id, datum: &str, zahlart: &str) -> Option<Id> {
        let rest = self.dokument(Some(dok))?.rest();
        self.zahlung_erfassen(dok, datum, rest, zahlart)
    }

    /// Zahlungseingang (auch Teilzahlung) erfassen und als Einnahme buchen.
    ///
    /// Die Zahlung deckt zuerst den Rechnungsbetrag – aufgeteilt nach Steuersätzen –, ein Rest
    /// gilt für Mahngebühren/Verzugszinsen (nicht umsatzsteuerbar, „Sonstige Einnahmen“).
    /// Liefert die ID der ersten Buchung.
    pub fn zahlung_erfassen(&mut self, dok: Id, datum: &str, betrag: Cents, zahlart: &str) -> Option<Id> {
        let d = self.dokument(Some(dok))?.clone();
        if betrag <= 0 {
            return None;
        }
        let brutto = d.summen().brutto;
        let vorher_auf_rechnung = d.gezahlt().clamp(0, brutto.max(0));
        let rechnungsteil = betrag.min((brutto - vorher_auf_rechnung).max(0));
        let kostenteil = betrag - rechnungsteil;
        let partner = self.kunden_name(d.kunde_id);
        let mut ids = Vec::new();
        let saetze = d.brutto_je_satz();
        if rechnungsteil > 0 {
            let mut verteilt = 0;
            let n = saetze.len();
            for (i, (rate, anteil)) in saetze.iter().enumerate() {
                let teil = if i + 1 == n {
                    rechnungsteil - verteilt
                } else {
                    (rechnungsteil as f64 * *anteil as f64 / brutto.max(1) as f64).round() as Cents
                };
                verteilt += teil;
                if teil == 0 {
                    continue;
                }
                let mut text = format!("Rechnung {}", d.nummer);
                if n > 1 {
                    text += &format!(" ({} %)", rate);
                }
                if rechnungsteil < brutto - vorher_auf_rechnung || vorher_auf_rechnung > 0 {
                    text += " – Teilzahlung";
                }
                let bid = self.new_id();
                ids.push(bid);
                self.buchungen.push(Buchung {
                    id: bid,
                    art: BuchArt::Einnahme,
                    datum: datum.into(),
                    brutto: teil,
                    ust: *rate,
                    kategorie: "Umsatzerlöse".into(),
                    text,
                    partner: partner.clone(),
                    zahlart: zahlart.into(),
                    beleg: None,
                    dokument_id: Some(dok),
                });
            }
        }
        if kostenteil > 0 {
            let bid = self.new_id();
            ids.push(bid);
            self.buchungen.push(Buchung {
                id: bid,
                art: BuchArt::Einnahme,
                datum: datum.into(),
                brutto: kostenteil,
                ust: 0,
                kategorie: "Sonstige Einnahmen".into(),
                text: format!("Mahngebühren/Zinsen Rechnung {}", d.nummer),
                partner,
                zahlart: zahlart.into(),
                beleg: None,
                dokument_id: Some(dok),
            });
        }
        let zid = self.new_id();
        let first = ids.first().copied();
        let mut fertig = false;
        if let Some(x) = self.dokument_mut(dok) {
            x.zahlungen.push(Zahlung { id: zid, datum: datum.into(), betrag, zahlart: zahlart.into(), buchungen: ids });
            if x.rest() == 0 {
                x.status = DokStatus::Bezahlt;
                x.bezahlt_am = datum.into();
                x.buchung_id = first;
                fertig = true;
            }
        }
        if fertig {
            if let Some(aid) = d.auftrag_id {
                self.auftrag_status(aid, AuftragStatus::Abgerechnet, datum);
            }
        }
        first
    }

    /// Einzelne Zahlung löschen (samt Buchungen); eine bezahlte Rechnung wird wieder offen.
    pub fn zahlung_loeschen(&mut self, dok: Id, zahlung: Id) {
        let Some(x) = self.dokument_mut(dok) else { return };
        let Some(pos) = x.zahlungen.iter().position(|z| z.id == zahlung) else { return };
        let z = x.zahlungen.remove(pos);
        if x.status == DokStatus::Bezahlt && x.rest() > 0 {
            x.status = DokStatus::Versendet;
            x.bezahlt_am.clear();
            x.buchung_id = None;
        }
        self.buchungen.retain(|b| !z.buchungen.contains(&b.id));
    }

    /// Alle Zahlungen zurücknehmen: zugehörige Buchungen löschen, Status wieder „Versendet“.
    pub fn zahlung_zuruecknehmen(&mut self, dok: Id) {
        self.buchungen.retain(|b| b.dokument_id != Some(dok));
        if let Some(x) = self.dokument_mut(dok) {
            x.status = DokStatus::Versendet;
            x.bezahlt_am.clear();
            x.buchung_id = None;
            x.zahlungen.clear();
        }
    }

    /// Verzugszinsen: Satz (% p. a.) und Betrag für den offenen Rechnungsbetrag
    /// vom Beginn (Fälligkeit bzw. letzte Mahnung) bis `datum`.
    pub fn verzugszinsen(&self, dok: Id, datum: &str) -> (f64, i64, Cents) {
        let Some(d) = self.dokument(Some(dok)) else { return (0.0, 0, 0) };
        let geschaeftskunde = self.kunde(d.kunde_id).map(|k| !k.firma.trim().is_empty()).unwrap_or(false);
        let satz = (self.firma.basiszins + if geschaeftskunde { 9.0 } else { 5.0 }).max(0.0);
        let beginn = d.letzte_mahnung().map(|m| m.datum.clone()).unwrap_or_else(|| d.faellig.clone());
        let tage = date::diff_days(&beginn, datum).unwrap_or(0).max(0);
        let brutto = d.summen().brutto;
        let offen = (brutto - d.gezahlt().clamp(0, brutto.max(0))).max(0);
        let zinsen = (offen as f64 * satz / 100.0 * tage as f64 / 365.0).round() as Cents;
        (satz, tage, zinsen)
    }

    /// Mahnung anlegen; liefert ihren Index in der Rechnung.
    pub fn mahnung_erstellen(&mut self, dok: Id, stufe: u8, datum: &str, frist: &str, gebuehr: Cents, mit_zinsen: bool) -> Option<usize> {
        let (satz, tage, zinsen) = if mit_zinsen { self.verzugszinsen(dok, datum) } else { (0.0, 0, 0) };
        let x = self.dokument_mut(dok)?;
        if x.art != DokArt::Rechnung {
            return None;
        }
        let m = Mahnung {
            stufe: stufe.min(2),
            datum: datum.into(),
            frist: frist.into(),
            offen_vorher: x.rest(),
            gebuehr: gebuehr.max(0),
            zinsen,
            zins_satz: if mit_zinsen { satz } else { 0.0 },
            zins_tage: if mit_zinsen { tage } else { 0 },
            versendet: false,
        };
        x.mahnungen.push(m);
        Some(x.mahnungen.len() - 1)
    }

    pub fn mahnung_loeschen(&mut self, dok: Id, idx: usize) {
        if let Some(x) = self.dokument_mut(dok) {
            if idx < x.mahnungen.len() {
                x.mahnungen.remove(idx);
            }
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
    fn partial_payments_and_fees() {
        let (mut d, k) = demo();
        let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-10-01");
        {
            let x = d.dokument_mut(r).unwrap();
            x.positionen = vec![
                Position { text: "A".into(), menge: 1.0, einheit: "Stk.".into(), preis: 10_000, ust: 19 },
                Position { text: "B".into(), menge: 1.0, einheit: "Stk.".into(), preis: 10_000, ust: 7 },
            ];
            x.status = DokStatus::Versendet;
        }
        // brutto 119 + 107 = 226,00 €
        assert_eq!(d.dokument(Some(r)).unwrap().rest(), 22_600);
        // Mahnung mit 5 € Gebühr
        let i = d.mahnung_erstellen(r, 1, "2026-11-01", "2026-11-11", 500, false).unwrap();
        let m = d.dokument(Some(r)).unwrap().mahnungen[i].clone();
        assert_eq!(m.offen_vorher, 22_600);
        assert_eq!(m.gesamt(), 23_100);
        assert_eq!(d.dokument(Some(r)).unwrap().status_text("2026-11-02"), "1. Mahnung");
        // Teilzahlung 100 € → aufgeteilt nach Steuersätzen
        d.zahlung_erfassen(r, "2026-11-05", 10_000, "Überweisung");
        let x = d.dokument(Some(r)).unwrap();
        assert_eq!(x.rest(), 13_100);
        assert_eq!(x.status, DokStatus::Versendet);
        assert!(x.teilbezahlt());
        let teile: Vec<(u8, Cents)> = d.buchungen.iter().map(|b| (b.ust, b.brutto)).collect();
        assert_eq!(teile, vec![(7, 4_735), (19, 5_265)]);
        // Rest inkl. Mahngebühr
        d.als_bezahlt(r, "2026-11-10", "Bar");
        let x = d.dokument(Some(r)).unwrap();
        assert_eq!(x.status, DokStatus::Bezahlt);
        assert_eq!(x.rest(), 0);
        let summe: Cents = d.buchungen.iter().map(|b| b.brutto).sum();
        assert_eq!(summe, 23_100);
        let gebuehr = d.buchungen.iter().find(|b| b.kategorie == "Sonstige Einnahmen").unwrap();
        assert_eq!((gebuehr.brutto, gebuehr.ust), (500, 0));
        // einzelne Zahlung löschen → wieder offen
        let zid = d.dokument(Some(r)).unwrap().zahlungen[1].id;
        d.zahlung_loeschen(r, zid);
        let x = d.dokument(Some(r)).unwrap();
        assert_eq!(x.status, DokStatus::Versendet);
        assert_eq!(x.rest(), 13_100);
        assert_eq!(d.buchungen.len(), 2);
    }

    #[test]
    fn interest_and_dunning_levels() {
        let (mut d, k) = demo();
        d.firma.basiszins = 1.0;
        let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-01-01");
        {
            let x = d.dokument_mut(r).unwrap();
            x.positionen[0].preis = 100_000;
            x.status = DokStatus::Versendet;
            x.faellig = "2026-01-15".into();
        }
        assert!(d.dokument(Some(r)).unwrap().zu_mahnen("2026-01-20"));
        assert_eq!(d.dokument(Some(r)).unwrap().naechste_stufe(), 0);
        // Geschäftskunde: 1 % + 9 % = 10 % p. a. auf 1.190 € für 73 Tage = 23,80 €
        let (satz, tage, zinsen) = d.verzugszinsen(r, "2026-03-29");
        assert_eq!((satz, tage, zinsen), (10.0, 73, 2_380));
        d.mahnung_erstellen(r, 0, "2026-01-20", "2026-01-27", 0, false);
        let x = d.dokument(Some(r)).unwrap();
        assert!(!x.zu_mahnen("2026-01-25"));
        assert!(x.zu_mahnen("2026-01-28"));
        assert_eq!(x.naechste_stufe(), 1);
        assert_eq!(x.status_key("2026-01-28"), "gemahnt");
    }

    #[test]
    fn migrates_old_paid_invoices() {
        let (mut d, k) = demo();
        let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-11-12");
        d.dokument_mut(r).unwrap().positionen[0].preis = 10_000;
        d.als_bezahlt(r, "2026-11-20", "Bar");
        // alten Stand (ohne Zahlungsliste) nachbilden
        d.version = 1;
        d.dokument_mut(r).unwrap().zahlungen.clear();
        let e = Data::from_json(&d.to_json()).unwrap();
        let x = e.dokument(Some(r)).unwrap();
        assert_eq!(x.zahlungen.len(), 1);
        assert_eq!(x.rest(), 0);
        assert_eq!(e.version, 2);
    }

    #[test]
    fn order_confirmation_delivery_note_and_signature() {
        let (mut d, k) = demo();
        let a = d.neuer_auftrag(Some(k), "2026-11-01");
        d.auftrag_status(a, AuftragStatus::Angebot, "2026-11-02");
        let an = d.neues_dokument(DokArt::Angebot, Some(k), Some(a), "2026-11-02");
        d.dokument_mut(an).unwrap().positionen[0].preis = 10_000;
        // Angebot vor Ort unterschreiben
        let sig = Unterschrift { striche: vec![vec![[0.1, 0.1, 0.5], [0.2, 0.15, 0.6], [0.3, 0.1, 0.4], [0.4, 0.2, 0.5]]], name: "Erika Mustermann".into(), ort: "Berlin".into(), datum: "2026-11-03".into() };
        assert!(!d.unterschreiben(an, Unterschrift::default(), "2026-11-03"));
        assert!(d.unterschreiben(an, sig.clone(), "2026-11-03"));
        assert_eq!(d.dokument(Some(an)).unwrap().status, DokStatus::Angenommen);
        assert_eq!(d.auftrag(Some(a)).unwrap().status, AuftragStatus::Beauftragt);
        // Auftragsbestätigung und Lieferschein aus dem Angebot
        let ab = d.folgedokument(an, DokArt::Auftragsbestaetigung, "2026-11-04").unwrap();
        let x = d.dokument(Some(ab)).unwrap();
        assert_eq!(x.nummer, "AB-2026-0001");
        assert_eq!(x.positionen[0].preis, 10_000);
        assert_eq!(x.aus_angebot, Some(an));
        assert!(x.faellig.is_empty());
        let ls = d.folgedokument(ab, DokArt::Lieferschein, "2026-11-10").unwrap();
        assert_eq!(d.dokument(Some(ls)).unwrap().nummer, "LS-2026-0001");
        assert!(d.unterschreiben(ls, sig, "2026-11-10"));
        assert_eq!(d.dokument(Some(ls)).unwrap().status, DokStatus::Unterschrieben);
        assert_eq!(d.auftrag(Some(a)).unwrap().status, AuftragStatus::Erledigt);
        // Lieferschein → Rechnung übernimmt Preise und Leistungsdatum
        let re = d.folgedokument(ls, DokArt::Rechnung, "2026-11-12").unwrap();
        let r = d.dokument(Some(re)).unwrap();
        assert_eq!(r.summen().netto, 10_000);
        assert_eq!(r.leistung, "10.11.2026");
        // Rechnungen sind nicht unterschreibbar
        assert!(!d.unterschreiben(re, Unterschrift { striche: vec![vec![[0.0, 0.0, 0.5]; 4]], ..Default::default() }, "2026-11-12"));
        d.unterschrift_entfernen(ls);
        assert_eq!(d.dokument(Some(ls)).unwrap().status, DokStatus::Versendet);
        assert_eq!(DokArt::from_key("ab"), Some(DokArt::Auftragsbestaetigung));
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
