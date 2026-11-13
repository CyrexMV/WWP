//! Anlagen beim Kunden (Heizung, Wärmepumpe …) und digitale Wartungsprotokolle
//! mit Checkliste, Messwerten, Mängeln und Unterschrift des Kunden.

use super::date;
use super::model::{Data, DokArt, Id, PosArt, Position, Unterschrift};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AnlagenArt {
    #[default]
    GasBrennwert,
    Oel,
    Waermepumpe,
    Pellet,
    Sonstige,
}

impl AnlagenArt {
    pub const ALLE: [AnlagenArt; 5] = [AnlagenArt::GasBrennwert, AnlagenArt::Oel, AnlagenArt::Waermepumpe, AnlagenArt::Pellet, AnlagenArt::Sonstige];
    pub fn label(self) -> &'static str {
        match self {
            AnlagenArt::GasBrennwert => "Gas-Brennwert",
            AnlagenArt::Oel => "Öl-Heizung",
            AnlagenArt::Waermepumpe => "Wärmepumpe",
            AnlagenArt::Pellet => "Pelletheizung",
            AnlagenArt::Sonstige => "Sonstige Anlage",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            AnlagenArt::GasBrennwert => "gas",
            AnlagenArt::Oel => "oel",
            AnlagenArt::Waermepumpe => "wp",
            AnlagenArt::Pellet => "pellet",
            AnlagenArt::Sonstige => "sonstige",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALLE.into_iter().find(|a| a.key() == k)
    }
    /// Suchwort für die passende Wartung im Leistungskatalog
    fn katalog_wort(self) -> &'static str {
        match self {
            AnlagenArt::GasBrennwert => "gas",
            AnlagenArt::Oel => "öl",
            AnlagenArt::Waermepumpe => "wärmepumpe",
            AnlagenArt::Pellet => "pellet",
            AnlagenArt::Sonstige => "heizung",
        }
    }

    /// Prüfpunkte der Checkliste
    pub fn pruefpunkte(self) -> Vec<&'static str> {
        let mut v: Vec<&str> = match self {
            AnlagenArt::GasBrennwert => vec![
                "Brenner gereinigt und geprüft",
                "Wärmetauscher gereinigt",
                "Zündung und Flammenüberwachung geprüft",
                "Gasanschluss- und Fließdruck geprüft",
                "Dichtheit Gasleitung am Gerät geprüft",
                "Abgasweg und Verbrennungsluftzufuhr geprüft",
                "Kondensatsiphon gereinigt",
            ],
            AnlagenArt::Oel => vec![
                "Brenner gereinigt, Düse gewechselt",
                "Ölfilter gewechselt",
                "Kessel / Wärmetauscher gereinigt",
                "Zündung und Flammenüberwachung geprüft",
                "Abgasweg und Verbrennungsluftzufuhr geprüft",
                "Öltank und Leitungen Sichtprüfung",
            ],
            AnlagenArt::Waermepumpe => vec![
                "Außeneinheit / Verdampfer gereinigt",
                "Kondensatablauf geprüft",
                "Kältekreis Sichtprüfung (Öl- und Leckspuren)",
                "Schmutzfänger und Filter gereinigt",
                "Betriebsdaten und Fehlerspeicher ausgelesen",
                "Warmwasserspeicher / Opferanode geprüft",
            ],
            AnlagenArt::Pellet => vec![
                "Brennraum und Brennerschale gereinigt",
                "Wärmetauscher gereinigt",
                "Asche entleert",
                "Fördersystem / Saugturbine geprüft",
                "Pelletlager Sichtprüfung",
                "Abgasweg geprüft",
            ],
            AnlagenArt::Sonstige => vec![],
        };
        v.extend([
            "Sichtprüfung auf Dichtheit und Beschädigungen",
            "Anlagendruck geprüft, ggf. Wasser nachgefüllt",
            "Ausdehnungsgefäß: Vordruck geprüft",
            "Sicherheitsventil auf Funktion geprüft",
            "Umwälzpumpe(n) geprüft",
            "Regelung und Heizkurve geprüft",
        ]);
        v
    }

    /// Messwerte (Bezeichnung, Einheit)
    pub fn messwerte(self) -> Vec<(&'static str, &'static str)> {
        match self {
            AnlagenArt::GasBrennwert | AnlagenArt::Oel => {
                let mut v = vec![
                    ("Abgastemperatur", "°C"),
                    ("Verbrennungslufttemperatur", "°C"),
                    ("CO2", "%"),
                    ("O2", "%"),
                    ("CO", "ppm"),
                    ("Abgasverlust", "%"),
                ];
                if self == AnlagenArt::GasBrennwert {
                    v.push(("Gasanschlussdruck", "mbar"));
                }
                v.extend([("Anlagendruck", "bar"), ("Vordruck Ausdehnungsgefäß", "bar")]);
                v
            }
            AnlagenArt::Waermepumpe => vec![
                ("Vorlauftemperatur", "°C"),
                ("Rücklauftemperatur", "°C"),
                ("Warmwassertemperatur", "°C"),
                ("Betriebsstunden Verdichter", "h"),
                ("Anlagendruck", "bar"),
                ("Vordruck Ausdehnungsgefäß", "bar"),
            ],
            AnlagenArt::Pellet => vec![
                ("Abgastemperatur", "°C"),
                ("CO", "mg/m³"),
                ("Staub", "mg/m³"),
                ("Anlagendruck", "bar"),
                ("Vordruck Ausdehnungsgefäß", "bar"),
            ],
            AnlagenArt::Sonstige => vec![("Anlagendruck", "bar"), ("Vordruck Ausdehnungsgefäß", "bar")],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Anlage {
    pub id: Id,
    pub kunde_id: Option<Id>,
    pub art: AnlagenArt,
    pub hersteller: String,
    pub modell: String,
    pub seriennummer: String,
    pub baujahr: String,
    pub leistung: String,
    /// z. B. „Keller, Heizraum“
    pub standort: String,
    pub letzte_wartung: String,
    /// Wartungsintervall in Monaten (0 = keine Erinnerung)
    pub intervall: u32,
    pub notizen: String,
    /// Foto des Typenschilds (Datei-ID)
    pub foto: Option<Id>,
}

impl Default for Anlage {
    fn default() -> Self {
        Anlage {
            id: 0,
            kunde_id: None,
            art: AnlagenArt::GasBrennwert,
            hersteller: String::new(),
            modell: String::new(),
            seriennummer: String::new(),
            baujahr: String::new(),
            leistung: String::new(),
            standort: String::new(),
            letzte_wartung: String::new(),
            intervall: 12,
            notizen: String::new(),
            foto: None,
        }
    }
}

impl Anlage {
    /// „Gas-Brennwert · Vaillant ecoTEC plus“
    pub fn titel(&self) -> String {
        let hm = [self.hersteller.trim(), self.modell.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
        if hm.is_empty() {
            self.art.label().to_string()
        } else {
            format!("{} · {hm}", self.art.label())
        }
    }

    /// Datum der nächsten fälligen Wartung (ohne letzte Wartung: keine Angabe).
    pub fn naechste_wartung(&self) -> Option<String> {
        if self.intervall == 0 || self.letzte_wartung.is_empty() {
            return None;
        }
        let (y, m, d) = date::parse(&self.letzte_wartung)?;
        let total = (y * 12 + m as i32 - 1) + self.intervall as i32;
        let (ny, nm) = (total / 12, (total % 12) as u32 + 1);
        Some(date::iso(ny, nm, d.min(28)))
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pruefung {
    #[default]
    Offen,
    Ok,
    Mangel,
    /// nicht zutreffend
    Entfaellt,
}

impl Pruefung {
    pub fn label(self) -> &'static str {
        match self {
            Pruefung::Offen => "–",
            Pruefung::Ok => "OK",
            Pruefung::Mangel => "Mangel",
            Pruefung::Entfaellt => "entfällt",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Pruefpunkt {
    pub text: String,
    pub status: Pruefung,
    pub notiz: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Messwert {
    pub name: String,
    pub einheit: String,
    pub wert: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Wartung {
    pub id: Id,
    pub nummer: String,
    pub anlage_id: Option<Id>,
    pub kunde_id: Option<Id>,
    pub datum: String,
    pub monteur: String,
    pub pruefpunkte: Vec<Pruefpunkt>,
    pub messwerte: Vec<Messwert>,
    /// durchgeführte Arbeiten / Bemerkungen
    pub arbeiten: String,
    /// festgestellte Mängel und Empfehlungen
    pub maengel: String,
    pub unterschrift: Option<Unterschrift>,
    pub abgeschlossen: bool,
    /// daraus erstellte Rechnung
    pub rechnung_id: Option<Id>,
}

impl Wartung {
    pub fn anzahl_maengel(&self) -> usize {
        self.pruefpunkte.iter().filter(|p| p.status == Pruefung::Mangel).count()
    }
    pub fn offene_punkte(&self) -> usize {
        self.pruefpunkte.iter().filter(|p| p.status == Pruefung::Offen).count()
    }
}

impl Data {
    pub fn anlage(&self, id: Option<Id>) -> Option<&Anlage> {
        let id = id?;
        self.anlagen.iter().find(|a| a.id == id)
    }
    pub fn anlage_mut(&mut self, id: Id) -> Option<&mut Anlage> {
        self.anlagen.iter_mut().find(|a| a.id == id)
    }
    pub fn anlagen_von(&self, kunde: Id) -> Vec<&Anlage> {
        self.anlagen.iter().filter(|a| a.kunde_id == Some(kunde)).collect()
    }
    pub fn neue_anlage(&mut self, kunde: Option<Id>) -> Id {
        let id = self.new_id();
        self.anlagen.push(Anlage { id, kunde_id: kunde, ..Default::default() });
        id
    }
    pub fn anlage_loeschen(&mut self, id: Id) {
        self.anlagen.retain(|a| a.id != id);
        for w in self.wartungen.iter_mut().filter(|w| w.anlage_id == Some(id)) {
            w.anlage_id = None;
        }
    }

    pub fn wartung(&self, id: Option<Id>) -> Option<&Wartung> {
        let id = id?;
        self.wartungen.iter().find(|w| w.id == id)
    }
    pub fn wartung_mut(&mut self, id: Id) -> Option<&mut Wartung> {
        self.wartungen.iter_mut().find(|w| w.id == id)
    }
    pub fn wartungen_von(&self, anlage: Id) -> Vec<&Wartung> {
        let mut v: Vec<&Wartung> = self.wartungen.iter().filter(|w| w.anlage_id == Some(anlage)).collect();
        v.sort_by(|a, b| b.datum.cmp(&a.datum).then(b.id.cmp(&a.id)));
        v
    }

    /// Neues Protokoll mit Checkliste und Messwerten passend zur Anlagenart.
    pub fn wartung_starten(&mut self, anlage: Id, monteur: &str, heute: &str) -> Option<Id> {
        let a = self.anlage(Some(anlage))?.clone();
        let id = self.new_id();
        let nummer = self.next_number("WA", date::year(heute).unwrap_or(2026), 4);
        self.wartungen.push(Wartung {
            id,
            nummer,
            anlage_id: Some(anlage),
            kunde_id: a.kunde_id,
            datum: heute.into(),
            monteur: monteur.trim().into(),
            pruefpunkte: a.art.pruefpunkte().into_iter().map(|t| Pruefpunkt { text: t.into(), ..Default::default() }).collect(),
            messwerte: a.art.messwerte().into_iter().map(|(n, e)| Messwert { name: n.into(), einheit: e.into(), wert: String::new() }).collect(),
            ..Default::default()
        });
        Some(id)
    }

    /// Protokoll abschließen: Datum der letzten Wartung an der Anlage setzen.
    pub fn wartung_abschliessen(&mut self, id: Id) -> bool {
        let Some(w) = self.wartung_mut(id) else { return false };
        w.abgeschlossen = true;
        for p in w.pruefpunkte.iter_mut().filter(|p| p.status == Pruefung::Offen) {
            p.status = Pruefung::Entfaellt;
        }
        let (aid, datum) = (w.anlage_id, w.datum.clone());
        if let Some(a) = aid.and_then(|aid| self.anlage_mut(aid)) {
            if a.letzte_wartung.as_str() < datum.as_str() {
                a.letzte_wartung = datum;
            }
        }
        true
    }

    pub fn wartung_unterschreiben(&mut self, id: Id, u: Unterschrift) -> bool {
        if !u.gueltig() {
            return false;
        }
        match self.wartung_mut(id) {
            Some(w) => {
                w.unterschrift = Some(u);
                true
            }
            None => false,
        }
    }

    /// Rechnung zur Wartung: Position aus dem Leistungskatalog („Wartung …“ passend zur Anlagenart), sonst ohne Preis.
    pub fn wartung_rechnung(&mut self, id: Id, heute: &str) -> Option<Id> {
        let w = self.wartung(Some(id))?.clone();
        if let Some(r) = w.rechnung_id.filter(|r| self.dokument(Some(*r)).is_some()) {
            return Some(r);
        }
        let a = self.anlage(w.anlage_id).cloned().unwrap_or_default();
        let wort = a.art.katalog_wort();
        let treffer = self
            .katalog
            .iter()
            .find(|l| {
                let t = l.text.to_lowercase();
                t.starts_with("wartung") && t.lines().next().unwrap_or("").contains(wort)
            })
            .cloned();
        let rid = self.neues_dokument(DokArt::Rechnung, w.kunde_id, None, heute);
        let ust = self.firma.ust_satz();
        let d = self.dokument_mut(rid)?;
        let titel = match &treffer {
            Some(l) => l.text.lines().next().unwrap_or("").to_string(),
            None => format!("Wartung {}", a.art.label()),
        };
        let geraet = [a.hersteller.trim(), a.modell.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
        let mut text = titel;
        text += &format!("\nlaut Wartungsprotokoll {} vom {}", w.nummer, date::german(&w.datum));
        if !geraet.is_empty() {
            text += &format!(" ({geraet})");
        }
        d.positionen = vec![Position {
            art: PosArt::Normal,
            text,
            menge: 1.0,
            einheit: treffer.as_ref().map(|l| l.einheit.clone()).unwrap_or_else(|| "Pausch.".into()),
            preis: treffer.as_ref().map(|l| l.preis).unwrap_or(0),
            ust: treffer.as_ref().map(|l| l.ust).unwrap_or(ust),
            rabatt: 0.0,
        }];
        d.leistung = date::german(&w.datum);
        d.betreff = format!("Wartung {}", a.art.label());
        if let Some(x) = self.wartung_mut(id) {
            x.rechnung_id = Some(rid);
        }
        Some(rid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::vorlagen::HEIZUNG;

    #[test]
    fn anlage_und_wartung() {
        let mut d = Data::new();
        d.vorlage_laden(HEIZUNG);
        let k = d.neuer_kunde("2026-01-01");
        let a = d.neue_anlage(Some(k));
        {
            let x = d.anlage_mut(a).unwrap();
            x.hersteller = "Vaillant".into();
            x.modell = "ecoTEC plus".into();
            x.letzte_wartung = "2025-11-20".into();
        }
        assert_eq!(d.anlage(Some(a)).unwrap().naechste_wartung().as_deref(), Some("2026-11-20"));
        assert_eq!(d.anlage(Some(a)).unwrap().titel(), "Gas-Brennwert · Vaillant ecoTEC plus");
        let w = d.wartung_starten(a, "Max", "2026-11-13").unwrap();
        let x = d.wartung(Some(w)).unwrap();
        assert_eq!(x.nummer, "WA-2026-0001");
        assert!(x.pruefpunkte.len() >= 10);
        assert!(x.messwerte.iter().any(|m| m.name == "Gasanschlussdruck"));
        d.wartung_mut(w).unwrap().pruefpunkte[0].status = Pruefung::Mangel;
        assert!(d.wartung_abschliessen(w));
        let x = d.wartung(Some(w)).unwrap();
        assert_eq!(x.anzahl_maengel(), 1);
        assert_eq!(x.offene_punkte(), 0);
        assert_eq!(d.anlage(Some(a)).unwrap().letzte_wartung, "2026-11-13");
        assert_eq!(d.anlage(Some(a)).unwrap().naechste_wartung().as_deref(), Some("2027-11-13"));
        // Rechnung mit Preis aus dem Katalog, nur einmal
        let r = d.wartung_rechnung(w, "2026-11-13").unwrap();
        let re = d.dokument(Some(r)).unwrap();
        assert_eq!(re.positionen[0].preis, 18_900);
        assert!(re.positionen[0].text.contains("WA-2026-0001"));
        assert_eq!(d.wartung_rechnung(w, "2026-11-13"), Some(r));
        // Wärmepumpe: eigene Checkliste
        let wp = d.neue_anlage(Some(k));
        d.anlage_mut(wp).unwrap().art = AnlagenArt::Waermepumpe;
        let w2 = d.wartung_starten(wp, "", "2026-11-13").unwrap();
        assert!(d.wartung(Some(w2)).unwrap().pruefpunkte.iter().any(|p| p.text.contains("Verdampfer")));
        d.anlage_loeschen(wp);
        assert!(d.wartung(Some(w2)).unwrap().anlage_id.is_none());
    }
}
