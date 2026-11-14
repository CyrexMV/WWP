//! Termine (Besichtigung, Ausführung, Wartung …) mit Kalender-Export (iCalendar/.ics).

use super::date;
use super::model::{Data, Id};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Termin {
    pub id: Id,
    pub titel: String,
    pub auftrag_id: Option<Id>,
    pub kunde_id: Option<Id>,
    /// „JJJJ-MM-TT“
    pub datum: String,
    /// „HH:MM“ (leer = ganztägig)
    pub von: String,
    pub bis: String,
    /// letzter Tag bei mehrtägigen Terminen (leer = eintägig)
    pub bis_datum: String,
    pub ort: String,
    pub notiz: String,
    /// zugewiesener Monteur: Benutzer-ID aus der Cloud (leer ohne Cloud)
    pub monteur: String,
    /// Name des Monteurs (Anzeige, auch offline und ohne Cloud)
    pub monteur_name: String,
}

impl Termin {
    /// Ist jemand zugewiesen?
    pub fn zugewiesen(&self) -> bool {
        !self.monteur.trim().is_empty() || !self.monteur_name.trim().is_empty()
    }
    /// Gehört der Termin zu diesem Monteur (Cloud-ID oder – ohne Cloud – Name)?
    pub fn gehoert(&self, user_id: &str, name: &str) -> bool {
        if !user_id.is_empty() && self.monteur == user_id {
            return true;
        }
        self.monteur.is_empty() && !name.trim().is_empty() && self.monteur_name.trim().eq_ignore_ascii_case(name.trim())
    }
    pub fn ganztag(&self) -> bool {
        self.von.trim().is_empty()
    }
    /// Endet der Termin an einem späteren Tag?
    pub fn letzter_tag(&self) -> String {
        if self.bis_datum.as_str() > self.datum.as_str() {
            self.bis_datum.clone()
        } else {
            self.datum.clone()
        }
    }
    /// Findet der Termin an `tag` statt?
    pub fn an(&self, tag: &str) -> bool {
        self.datum.as_str() <= tag && tag <= self.letzter_tag().as_str()
    }
    pub fn zeit(&self) -> String {
        match (self.von.trim(), self.bis.trim()) {
            ("", _) => "ganztägig".into(),
            (v, "") => format!("{v} Uhr"),
            (v, b) => format!("{v}–{b} Uhr"),
        }
    }
}

/// Montag der Woche, in der `tag` liegt.
pub fn wochenanfang(tag: &str) -> String {
    let z = date::to_days(tag).unwrap_or(0);
    // 1970-01-01 war ein Donnerstag → (z + 3) mod 7 = 0 für Montag
    from(z - (z + 3).rem_euclid(7))
}

fn from(z: i64) -> String {
    date::from_days(z)
}

/// Wochentag 0 = Montag … 6 = Sonntag
pub fn wochentag(tag: &str) -> usize {
    (date::to_days(tag).unwrap_or(0) + 3).rem_euclid(7) as usize
}

pub const WOCHENTAGE: [&str; 7] = ["Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag", "Sonntag"];
pub const WT_KURZ: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];

/// Kalenderwoche nach ISO 8601.
pub fn kalenderwoche(tag: &str) -> u32 {
    let z = date::to_days(tag).unwrap_or(0);
    let donnerstag = z - wochentag(tag) as i64 + 3;
    let jahr = date::year(&from(donnerstag)).unwrap_or(1970);
    let erster = date::days_from_civil(jahr, 1, 1);
    ((donnerstag - erster) / 7 + 1) as u32
}

fn ics_text(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n").replace('\r', "")
}

/// Lange Zeilen nach RFC 5545 umbrechen (75 Oktette).
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut len = 0;
    for ch in line.chars() {
        let l = ch.len_utf8();
        if len + l > 74 {
            out.push_str("\r\n ");
            len = 1;
        }
        out.push(ch);
        len += l;
    }
    out.push_str("\r\n");
    out
}

/// Termine als iCalendar-Datei (lokale Zeit).
pub fn ics(data: &Data, termine: &[&Termin], stempel: &str) -> String {
    let mut s = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Zento//DE\r\nCALSCALE:GREGORIAN\r\n");
    for t in termine {
        s += "BEGIN:VEVENT\r\n";
        s += &fold(&format!("UID:wwp-termin-{}@wwp-buero", t.id));
        s += &fold(&format!("DTSTAMP:{}T000000Z", stempel.replace('-', "")));
        let d = t.datum.replace('-', "");
        if t.ganztag() {
            let ende = date::add_days(&t.letzter_tag(), 1).replace('-', "");
            s += &fold(&format!("DTSTART;VALUE=DATE:{d}"));
            s += &fold(&format!("DTEND;VALUE=DATE:{ende}"));
        } else {
            let von = t.von.replace(':', "");
            let bis = if t.bis.trim().is_empty() {
                // ohne Ende: eine Stunde
                let h: u32 = t.von.get(0..2).and_then(|x| x.parse().ok()).unwrap_or(8);
                format!("{:02}{}", (h + 1).min(23), t.von.get(3..5).unwrap_or("00"))
            } else {
                t.bis.replace(':', "")
            };
            let end_d = t.letzter_tag().replace('-', "");
            s += &fold(&format!("DTSTART:{d}T{von}00"));
            s += &fold(&format!("DTEND:{end_d}T{bis}00"));
        }
        s += &fold(&format!("SUMMARY:{}", ics_text(&t.titel)));
        if !t.ort.trim().is_empty() {
            s += &fold(&format!("LOCATION:{}", ics_text(t.ort.trim())));
        }
        let mut beschr = Vec::new();
        if let Some(k) = data.kunde(t.kunde_id) {
            let tel = if k.telefon.trim().is_empty() { String::new() } else { format!(" (Tel. {})", k.telefon.trim()) };
            beschr.push(format!("Kunde: {}{}", k.name(), tel));
        }
        if let Some(a) = data.auftrag(t.auftrag_id) {
            beschr.push(format!("Auftrag {}: {}", a.nr, a.titel()));
        }
        if !t.notiz.trim().is_empty() {
            beschr.push(t.notiz.trim().to_string());
        }
        if !beschr.is_empty() {
            s += &fold(&format!("DESCRIPTION:{}", ics_text(&beschr.join("\n"))));
        }
        s += "END:VEVENT\r\n";
    }
    s += "END:VCALENDAR\r\n";
    s
}

impl Data {
    pub fn termin(&self, id: Option<Id>) -> Option<&Termin> {
        let id = id?;
        self.termine.iter().find(|t| t.id == id)
    }
    pub fn termin_mut(&mut self, id: Id) -> Option<&mut Termin> {
        self.termine.iter_mut().find(|t| t.id == id)
    }

    /// Neuer Termin; Kunde, Titel und Ort werden vom Auftrag übernommen.
    pub fn neuer_termin(&mut self, auftrag: Option<Id>, kunde: Option<Id>, datum: &str) -> Id {
        let id = self.new_id();
        let a = self.auftrag(auftrag).cloned();
        let kunde = kunde.or(a.as_ref().and_then(|a| a.kunde_id));
        let k = self.kunde(kunde).cloned();
        let ort = k
            .as_ref()
            .map(|k| [k.strasse.trim(), &format!("{} {}", k.plz.trim(), k.ort.trim())].iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let titel = match (&a, &k) {
            (Some(a), _) if !a.titel.trim().is_empty() => a.titel.trim().to_string(),
            (_, Some(k)) => format!("Termin {}", k.name()),
            _ => "Termin".into(),
        };
        self.termine.push(Termin { id, titel, auftrag_id: auftrag, kunde_id: kunde, datum: datum.into(), von: "09:00".into(), bis: "10:00".into(), ort, ..Default::default() });
        id
    }

    /// Termine an einem Tag, nach Uhrzeit sortiert (ganztägige zuerst).
    pub fn termine_am(&self, tag: &str) -> Vec<&Termin> {
        let mut v: Vec<&Termin> = self.termine.iter().filter(|t| t.an(tag)).collect();
        v.sort_by(|a, b| (!a.ganztag(), a.von.as_str()).cmp(&(!b.ganztag(), b.von.as_str())));
        v
    }

    /// Kommende Termine ab `tag` (inkl. heute), chronologisch.
    pub fn naechste_termine(&self, tag: &str, n: usize) -> Vec<&Termin> {
        let mut v: Vec<&Termin> = self.termine.iter().filter(|t| t.letzter_tag().as_str() >= tag).collect();
        v.sort_by(|a, b| (a.datum.as_str(), a.von.as_str()).cmp(&(b.datum.as_str(), b.von.as_str())));
        v.truncate(n);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::*;

    #[test]
    fn wochen_und_tage() {
        assert_eq!(wochentag("2026-11-13"), 4); // Freitag
        assert_eq!(wochenanfang("2026-11-13"), "2026-11-09");
        assert_eq!(wochenanfang("2026-11-09"), "2026-11-09");
        assert_eq!(kalenderwoche("2026-11-13"), 46);
        assert_eq!(kalenderwoche("2027-01-01"), 53);
        assert_eq!(kalenderwoche("2026-01-01"), 1);
    }

    #[test]
    fn termine_und_ics() {
        let mut d = Data::new();
        let k = d.neuer_kunde("2026-11-01");
        {
            let kd = d.kunde_mut(k).unwrap();
            kd.firma = "Bäckerei Schön".into();
            kd.strasse = "Am Markt 3".into();
            kd.plz = "80333".into();
            kd.ort = "München".into();
        }
        let a = d.neuer_auftrag(Some(k), "2026-11-01");
        d.auftrag_mut(a).unwrap().titel = "Heizung warten".into();
        let t = d.neuer_termin(Some(a), None, "2026-11-16");
        let tt = d.termin(Some(t)).unwrap();
        assert_eq!(tt.kunde_id, Some(k));
        assert_eq!(tt.titel, "Heizung warten");
        assert_eq!(tt.ort, "Am Markt 3, 80333 München");
        let g = d.neuer_termin(None, Some(k), "2026-11-16");
        {
            let x = d.termin_mut(g).unwrap();
            x.von.clear();
            x.bis_datum = "2026-11-18".into();
        }
        assert_eq!(d.termine_am("2026-11-17").len(), 1);
        assert_eq!(d.termine_am("2026-11-16")[0].id, g, "ganztägig zuerst");
        let alle: Vec<&Termin> = d.termine.iter().collect();
        let s = ics(&d, &alle, "2026-11-13");
        assert!(s.contains("DTSTART:20261116T090000"));
        assert!(s.contains("DTSTART;VALUE=DATE:20261116\r\nDTEND;VALUE=DATE:20261119"));
        assert!(s.contains("LOCATION:Am Markt 3\\, 80333 München"));
        assert!(s.lines().all(|l| l.len() <= 76));
        assert_eq!(d.naechste_termine("2026-11-17", 5).len(), 1);
    }

    #[test]
    fn zuweisung() {
        let mut t = Termin { monteur: "u-1".into(), monteur_name: "Tom".into(), ..Default::default() };
        assert!(t.zugewiesen());
        assert!(t.gehoert("u-1", ""));
        assert!(!t.gehoert("u-2", "Tom"), "mit Cloud-ID zählt nur die ID");
        t.monteur.clear();
        assert!(t.gehoert("", "tom"));
        assert!(!t.gehoert("", ""));
        t.monteur_name.clear();
        assert!(!t.zugewiesen());
    }
}
