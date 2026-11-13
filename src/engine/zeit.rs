//! Zeiterfassung pro Auftrag: Stoppuhr oder nachgetragene Zeiten,
//! die mit einem Tipp als Positionen in eine Rechnung übernommen werden.

use super::date;
use super::model::{Data, DokArt, DokStatus, Id, PosArt, Position};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Zeit {
    pub id: Id,
    pub datum: String,
    pub minuten: u32,
    /// Tätigkeit, z. B. „Rohre verlegt“
    pub text: String,
    /// schon in eine Rechnung übernommen
    pub abgerechnet: bool,
}

/// „1:05 h“
pub fn dauer(min: u32) -> String {
    format!("{}:{:02} h", min / 60, min % 60)
}

/// Eingabe wie „1:30“, „1,5“, „90 min“ oder „2 h“ in Minuten.
pub fn parse_dauer(s: &str) -> Option<u32> {
    let t = s.trim().to_lowercase().replace("std", "").replace('h', "");
    let t = t.trim();
    if let Some(m) = t.strip_suffix("min") {
        return m.trim().parse::<u32>().ok().filter(|m| *m > 0);
    }
    if let Some((h, m)) = t.split_once(':') {
        let h: u32 = h.trim().parse().ok()?;
        let m: u32 = m.trim().parse().ok()?;
        return Some(h * 60 + m).filter(|x| *x > 0 && m < 60);
    }
    let h: f64 = t.replace(',', ".").parse().ok()?;
    let m = (h * 60.0).round();
    (m > 0.0 && m < 100_000.0).then_some(m as u32)
}

impl Data {
    pub fn uhr_starten(&mut self, auftrag: Id, jetzt_ms: f64) -> bool {
        match self.auftraege.iter_mut().find(|a| a.id == auftrag) {
            Some(a) if a.uhr_start.is_none() => {
                a.uhr_start = Some(jetzt_ms);
                true
            }
            _ => false,
        }
    }

    /// Stoppuhr anhalten; die Zeit wird (mindestens 1 Minute) eingetragen.
    pub fn uhr_stoppen(&mut self, auftrag: Id, jetzt_ms: f64, heute: &str) -> Option<u32> {
        let start = self.auftraege.iter_mut().find(|a| a.id == auftrag)?.uhr_start.take()?;
        let min = (((jetzt_ms - start) / 60_000.0).round().max(1.0)) as u32;
        self.zeit_eintragen(auftrag, heute, min, "");
        Some(min)
    }

    pub fn zeit_eintragen(&mut self, auftrag: Id, datum: &str, minuten: u32, text: &str) -> Option<Id> {
        let id = self.new_id();
        let a = self.auftraege.iter_mut().find(|a| a.id == auftrag)?;
        a.zeiten.push(Zeit { id, datum: datum.into(), minuten, text: text.trim().into(), abgerechnet: false });
        a.zeiten.sort_by(|x, y| x.datum.cmp(&y.datum).then(x.id.cmp(&y.id)));
        Some(id)
    }

    pub fn zeit_loeschen(&mut self, auftrag: Id, zeit: Id) {
        if let Some(a) = self.auftraege.iter_mut().find(|a| a.id == auftrag) {
            a.zeiten.retain(|z| z.id != zeit);
        }
    }

    /// Läuft gerade irgendwo eine Stoppuhr?
    pub fn laufende_uhr(&self) -> Option<(Id, f64)> {
        self.auftraege.iter().find_map(|a| a.uhr_start.map(|s| (a.id, s)))
    }

    /// Offene Zeiten als Positionen in die Rechnung des Auftrags übernehmen (Entwurf weiterverwenden,
    /// sonst neue Rechnung). Gibt die Rechnung zurück.
    pub fn zeiten_abrechnen(&mut self, auftrag: Id, heute: &str) -> Option<Id> {
        let a = self.auftrag(Some(auftrag))?.clone();
        let offen: Vec<_> = a.zeiten.iter().filter(|z| !z.abgerechnet).cloned().collect();
        if offen.is_empty() {
            return None;
        }
        let entwurf = self
            .dokumente
            .iter()
            .filter(|d| d.art == DokArt::Rechnung && d.auftrag_id == Some(auftrag) && d.status == DokStatus::Entwurf && d.storno_von.is_none())
            .map(|d| d.id)
            .max();
        let rid = match entwurf {
            Some(r) => r,
            None => self.neues_dokument(DokArt::Rechnung, a.kunde_id, Some(auftrag), heute),
        };
        let satz = self.firma.stundensatz;
        let ust = if self.firma.kleinunternehmer { 0 } else { self.firma.ust_standard };
        let d = self.dokument_mut(rid)?;
        // leere Startposition ersetzen
        d.positionen.retain(|p| !(p.art == PosArt::Normal && p.text.trim().is_empty() && p.preis == 0));
        for z in &offen {
            let text = if z.text.is_empty() { format!("Arbeitszeit am {}", date::german(&z.datum)) } else { format!("{} ({})", z.text, date::german(&z.datum)) };
            d.positionen.push(Position {
                art: PosArt::Normal,
                text,
                menge: (z.minuten as f64 / 60.0 * 100.0).round() / 100.0,
                einheit: "Std.".into(),
                preis: satz,
                ust,
                rabatt: 0.0,
            });
        }
        if d.betreff.trim().is_empty() {
            d.betreff = a.titel();
        }
        let von = offen.iter().map(|z| z.datum.as_str()).min().unwrap_or(heute).to_string();
        let bis = offen.iter().map(|z| z.datum.as_str()).max().unwrap_or(heute).to_string();
        d.leistung = if von == bis { date::german(&von) } else { format!("{} – {}", date::german(&von), date::german(&bis)) };
        if let Some(x) = self.auftraege.iter_mut().find(|x| x.id == auftrag) {
            for z in x.zeiten.iter_mut() {
                z.abgerechnet = true;
            }
        }
        Some(rid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::Data;

    #[test]
    fn dauer_lesen() {
        assert_eq!(parse_dauer("1:30"), Some(90));
        assert_eq!(parse_dauer("1,5"), Some(90));
        assert_eq!(parse_dauer("2 h"), Some(120));
        assert_eq!(parse_dauer("45 min"), Some(45));
        assert_eq!(parse_dauer("0"), None);
        assert_eq!(parse_dauer("abc"), None);
        assert_eq!(dauer(125), "2:05 h");
    }

    #[test]
    fn stoppuhr_und_abrechnung() {
        let mut d = Data::new();
        let k = d.neuer_kunde("2026-11-10");
        d.firma.stundensatz = 6_000;
        let a = d.neuer_auftrag(Some(k), "2026-11-10");
        assert!(d.uhr_starten(a, 1_000_000.0));
        assert!(!d.uhr_starten(a, 1_000_000.0));
        assert_eq!(d.laufende_uhr(), Some((a, 1_000_000.0)));
        // 95 Minuten
        assert_eq!(d.uhr_stoppen(a, 1_000_000.0 + 95.0 * 60_000.0, "2026-11-10"), Some(95));
        assert!(d.laufende_uhr().is_none());
        d.zeit_eintragen(a, "2026-11-11", 30, "Material besorgt");
        let r = d.zeiten_abrechnen(a, "2026-11-12").unwrap();
        let x = d.dokument(Some(r)).unwrap();
        assert_eq!(x.positionen.len(), 2);
        assert_eq!(x.positionen[0].menge, 1.58);
        assert_eq!(x.positionen[1].text, "Material besorgt (11.11.2026)");
        assert_eq!(x.leistung, "10.11.2026 – 11.11.2026");
        assert_eq!(x.summen().netto, 9_480 + 3_000);
        // alles abgerechnet → nichts mehr zu tun; neue Zeit landet im selben Entwurf
        assert!(d.zeiten_abrechnen(a, "2026-11-12").is_none());
        d.zeit_eintragen(a, "2026-11-12", 60, "");
        assert_eq!(d.zeiten_abrechnen(a, "2026-11-12"), Some(r));
        assert_eq!(d.dokument(Some(r)).unwrap().positionen.len(), 3);
    }
}
