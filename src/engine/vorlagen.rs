//! Branchenvorlagen für den Leistungskatalog. Preise sind Richtwerte (netto, Stand 2026)
//! und sollten an die eigene Kalkulation angepasst werden.

use super::model::{Data, Leistung};
use super::money::Cents;

/// (Kategorie, Bezeichnung – zweite Zeile = Beschreibung, Einheit, VK netto, EK netto)
type Eintrag = (&'static str, &'static str, &'static str, Cents, Cents);

/// Heizungsbau (SHK): Arbeit, Wartung, Material, Heizkörper/Fußbodenheizung, Anlagen.
pub const HEIZUNG: &[Eintrag] = &[
    // Arbeit & Anfahrt
    ("Arbeit", "Arbeitsstunde Meister", "Std.", 8_500, 0),
    ("Arbeit", "Arbeitsstunde Monteur / Geselle", "Std.", 7_200, 0),
    ("Arbeit", "Arbeitsstunde Helfer / Auszubildender", "Std.", 4_800, 0),
    ("Arbeit", "Notdienstzuschlag\nAbend, Wochenende oder Feiertag", "Pausch.", 9_500, 0),
    ("Arbeit", "Anfahrtspauschale", "Pausch.", 4_500, 0),
    ("Arbeit", "Fahrtkosten außerhalb des Einsatzgebiets", "km", 95, 0),
    ("Arbeit", "Störungssuche / Fehlerdiagnose", "Std.", 8_500, 0),
    // Wartung & Service
    ("Wartung", "Wartung Gas-Brennwertkessel\nSichtprüfung, Reinigung von Brenner und Wärmetauscher, Abgasmessung, Funktions- und Sicherheitsprüfung", "Pausch.", 18_900, 0),
    ("Wartung", "Wartung Öl-Brennwertkessel\nReinigung, Düsen- und Filterwechsel, Abgasmessung, Funktions- und Sicherheitsprüfung", "Pausch.", 22_900, 0),
    ("Wartung", "Wartung Wärmepumpe\nSichtprüfung, Kontrolle Kältekreis und Kondensatablauf, Filter reinigen, Betriebsdaten auswerten", "Pausch.", 21_900, 0),
    ("Wartung", "Wartung Pelletheizung\nReinigung Brennraum und Wärmetauscher, Aschebehälter leeren, Fördersystem prüfen, Abgasmessung", "Pausch.", 27_900, 0),
    ("Wartung", "Heizungscheck nach DIN EN 15378\nBewertung von Wärmeerzeuger, Verteilung und Regelung mit Prüfbericht", "Pausch.", 14_900, 0),
    ("Wartung", "Abgas- und Emissionsmessung", "Pausch.", 5_900, 0),
    ("Wartung", "Heizungsanlage entlüften, Wasser nachfüllen, Druck einstellen", "Pausch.", 6_900, 0),
    ("Wartung", "Ausdehnungsgefäß prüfen und Vordruck einstellen", "Pausch.", 4_900, 0),
    ("Wartung", "Hydraulischer Abgleich (Verfahren B)\nRaumweise Heizlastberechnung, Voreinstellung der Ventile, Pumpe einstellen, Bestätigung für Förderung", "Pausch.", 69_000, 0),
    ("Wartung", "Heizungsanlage spülen (je Heizkreis)", "Stk.", 29_000, 0),
    // Material
    ("Material", "Thermostatkopf", "Stk.", 2_900, 1_400),
    ("Material", "Thermostatventil-Unterteil, voreinstellbar", "Stk.", 3_900, 1_800),
    ("Material", "Rücklaufverschraubung Heizkörper", "Stk.", 2_400, 1_000),
    ("Material", "Ausdehnungsgefäß 25 l", "Stk.", 8_900, 4_200),
    ("Material", "Sicherheitsventil 3 bar", "Stk.", 3_400, 1_400),
    ("Material", "Hocheffizienz-Umwälzpumpe", "Stk.", 34_900, 19_000),
    ("Material", "Schlamm- und Magnetitabscheider", "Stk.", 18_900, 9_500),
    ("Material", "Raumthermostat, programmierbar", "Stk.", 12_900, 6_000),
    ("Material", "Kugelhahn ½ Zoll", "Stk.", 1_900, 800),
    ("Material", "Kupferrohr 15 mm inkl. Fittings", "m", 1_850, 800),
    ("Material", "Kupferrohr 22 mm inkl. Fittings", "m", 2_400, 1_100),
    ("Material", "Mehrschichtverbundrohr 16 mm inkl. Fittings", "m", 1_450, 600),
    ("Material", "Rohrdämmung nach GEG", "m", 850, 300),
    ("Material", "Heizungsschutzmittel / Inhibitor (1 l)", "Stk.", 3_400, 1_500),
    ("Material", "Kleinmaterial, Dicht- und Befestigungsmittel", "Pausch.", 1_500, 500),
    // Heizkörper & Fußbodenheizung
    ("Heizkörper", "Heizkörper Typ 22, 600 × 1000 mm, inkl. Konsolen", "Stk.", 38_900, 19_000),
    ("Heizkörper", "Badheizkörper 1200 × 600 mm, inkl. Befestigung", "Stk.", 34_900, 17_000),
    ("Heizkörper", "Heizkörper austauschen\nAlten Heizkörper demontieren, neuen montieren, anschließen, befüllen und entlüften", "Stk.", 18_000, 0),
    ("Heizkörper", "Heizkörper ab- und wieder anbauen (z. B. für Malerarbeiten)", "Stk.", 9_500, 0),
    ("Heizkörper", "Fußbodenheizung verlegen, Tackersystem inkl. Material", "m²", 5_900, 2_500),
    ("Heizkörper", "Heizkreisverteiler Fußbodenheizung, 6 Kreise", "Stk.", 48_900, 24_000),
    // Anlagen & Austausch
    ("Anlage", "Gas-Brennwerttherme 20 kW, wandhängend", "Stk.", 389_000, 220_000),
    ("Anlage", "Montage Brennwerttherme\nAnschluss Heizung, Gas und Abgas, Inbetriebnahme und Einweisung", "Pausch.", 145_000, 0),
    ("Anlage", "Luft-Wasser-Wärmepumpe 8 kW, Monoblock", "Stk.", 1_190_000, 740_000),
    ("Anlage", "Montage Wärmepumpe\nAufstellung, Hydraulik, Elektroanschluss nach Vorgabe, Inbetriebnahme und Einweisung", "Pausch.", 390_000, 0),
    ("Anlage", "Warmwasserspeicher 300 l für Wärmepumpe", "Stk.", 189_000, 105_000),
    ("Anlage", "Pufferspeicher 200 l", "Stk.", 99_000, 52_000),
    ("Anlage", "Abgasleitung Kunststoff DN 80", "m", 7_900, 3_500),
    ("Anlage", "Demontage und Entsorgung Altkessel", "Pausch.", 59_000, 0),
    ("Anlage", "Heizlastberechnung nach DIN EN 12831 (Einfamilienhaus)", "Pausch.", 45_000, 0),
    ("Anlage", "Inbetriebnahme und Einweisung", "Pausch.", 19_000, 0),
    ("Anlage", "Fachunternehmererklärung / Unterlagen für den Förderantrag (BEG)", "Pausch.", 15_000, 0),
    ("Anlage", "Entsorgung Altmaterial", "Pausch.", 4_900, 0),
];

impl Data {
    /// Vorlage in den Katalog übernehmen; vorhandene Leistungen (gleiche Bezeichnung) werden übersprungen.
    /// Gibt die Zahl der neuen Einträge zurück.
    pub fn vorlage_laden(&mut self, vorlage: &[Eintrag]) -> usize {
        let ust = self.firma.ust_satz();
        let mut neu = 0;
        for (kategorie, text, einheit, preis, ek) in vorlage {
            let schon = self.katalog.iter().any(|l| l.text.trim().eq_ignore_ascii_case(text.trim()));
            if schon {
                continue;
            }
            let id = self.new_id();
            self.katalog.push(Leistung { id, text: text.to_string(), einheit: einheit.to_string(), preis: *preis, ust, ek: *ek, kategorie: kategorie.to_string() });
            neu += 1;
        }
        neu
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::EINHEITEN;

    #[test]
    fn heizung_laden() {
        let mut d = Data::new();
        let n = d.vorlage_laden(HEIZUNG);
        assert_eq!(n, HEIZUNG.len());
        assert!(n >= 40);
        // alle Einheiten sind bekannte Einheiten, EK unter VK
        for l in &d.katalog {
            assert!(EINHEITEN.contains(&l.einheit.as_str()), "{}", l.einheit);
            assert!(l.ek < l.preis, "{}", l.text);
            assert_eq!(l.ust, 19);
        }
        // zweites Laden fügt nichts doppelt hinzu
        assert_eq!(d.vorlage_laden(HEIZUNG), 0);
        // Kleinunternehmer: ohne USt.
        let mut k = Data::new();
        k.firma.kleinunternehmer = true;
        k.vorlage_laden(HEIZUNG);
        assert!(k.katalog.iter().all(|l| l.ust == 0));
    }
}
