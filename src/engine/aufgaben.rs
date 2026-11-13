//! „Heute zu tun“: Was in der App gerade ansteht – automatisch aus den Daten ermittelt.

use super::date;
use super::model::{AuftragStatus, BuchArt, Data, DokArt, DokStatus, Id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typ {
    /// Stoppuhr läuft (Auftrag)
    Uhr,
    /// Rechnung mahnen (Dokument)
    Mahnen,
    /// Rechnung überfällig, Mahnfrist noch nicht erreicht (Dokument)
    Ueberfaellig,
    /// Bestätigung versendet/unterschrieben, aber noch keine Rechnung (Dokument)
    Abrechnen,
    /// Termin heute (Termin)
    TerminHeute,
    /// Termin morgen (Termin)
    TerminMorgen,
    /// Frist eines Auftrags in den nächsten Tagen oder verpasst (Auftrag)
    Frist,
    /// Angebot ohne Antwort seit einer Woche (Dokument)
    Nachfassen,
    /// Angebot abgelaufen (Dokument)
    Abgelaufen,
    /// Erledigter Auftrag mit nicht abgerechneten Zeiten (Auftrag)
    Zeiten,
    /// Dokument im Entwurf (Dokument)
    Entwurf,
    /// neue Anfragen (Anzahl)
    Anfragen,
    /// Ausgaben ohne Beleg (Anzahl)
    Belege,
    /// Backup fällig (Tage seit dem letzten oder `None` = noch nie)
    Backup,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aufgabe {
    pub typ: Typ,
    pub id: Option<Id>,
    /// Zahl zum Typ: Tage überfällig/ohne Antwort, Anzahl, Minuten …
    pub zahl: i64,
    /// dringend = rot markiert
    pub dringend: bool,
}

/// Tage, nach denen ein versendetes Angebot nachgefasst werden sollte.
pub const NACHFASSEN_TAGE: i64 = 7;
/// Tage, nach denen an ein Backup erinnert wird.
pub const BACKUP_TAGE: i64 = 14;

impl Data {
    /// Anstehende Aufgaben, wichtigste zuerst.
    pub fn aufgaben(&self, heute: &str) -> Vec<Aufgabe> {
        let mut v = Vec::new();
        let a = |typ, id, zahl, dringend| Aufgabe { typ, id, zahl, dringend };
        if let Some((aid, _)) = self.laufende_uhr() {
            v.push(a(Typ::Uhr, Some(aid), 0, false));
        }
        for d in &self.dokumente {
            let tage = |datum: &str| date::diff_days(datum, heute).unwrap_or(0);
            match d.art {
                DokArt::Rechnung if d.offen() => {
                    if d.zu_mahnen(heute) {
                        v.push(a(Typ::Mahnen, Some(d.id), tage(&d.faellig), true));
                    } else if d.ueberfaellig(heute) {
                        v.push(a(Typ::Ueberfaellig, Some(d.id), tage(&d.faellig), true));
                    }
                }
                DokArt::Angebot if d.status == DokStatus::Versendet => {
                    if !d.faellig.is_empty() && d.faellig.as_str() < heute {
                        v.push(a(Typ::Abgelaufen, Some(d.id), tage(&d.faellig), false));
                    } else {
                        let seit = tage(if d.versendet_am.is_empty() { &d.datum } else { &d.versendet_am });
                        if seit >= NACHFASSEN_TAGE {
                            v.push(a(Typ::Nachfassen, Some(d.id), seit, false));
                        }
                    }
                }
                DokArt::Auftragsbestaetigung
                    if matches!(d.status, DokStatus::Versendet | DokStatus::Unterschrieben) && self.vorgang(Some(d.id), None)[3].is_none() =>
                {
                    v.push(a(Typ::Abrechnen, Some(d.id), tage(if d.versendet_am.is_empty() { &d.datum } else { &d.versendet_am }), false));
                }
                _ => {}
            }
            if d.status == DokStatus::Entwurf && d.storno_von.is_none() && d.summen().brutto != 0 && d.art != DokArt::Lieferschein {
                v.push(a(Typ::Entwurf, Some(d.id), tage(&d.datum), false));
            }
        }
        let morgen = date::add_days(heute, 1);
        for t in self.termine_am(heute) {
            v.push(a(Typ::TerminHeute, Some(t.id), 0, false));
        }
        for t in self.termine_am(&morgen) {
            v.push(a(Typ::TerminMorgen, Some(t.id), 1, false));
        }
        let mut anfragen = 0;
        for x in &self.auftraege {
            if x.status == AuftragStatus::Anfrage {
                anfragen += 1;
            }
            if x.status.offen() && !x.termin.is_empty() {
                let rest = date::diff_days(heute, &x.termin).unwrap_or(99);
                if rest <= 3 {
                    v.push(a(Typ::Frist, Some(x.id), rest, rest < 0));
                }
            }
            if x.status >= AuftragStatus::Erledigt && x.zeiten.iter().any(|z| !z.abgerechnet) {
                v.push(a(Typ::Zeiten, Some(x.id), x.zeiten.iter().filter(|z| !z.abgerechnet).map(|z| z.minuten as i64).sum(), false));
            }
        }
        if anfragen > 0 {
            v.push(a(Typ::Anfragen, None, anfragen, false));
        }
        let jahr = date::year(heute);
        let belege = self.buchungen.iter().filter(|b| b.art == BuchArt::Ausgabe && b.beleg.is_none() && !b.neutral() && date::year(&b.datum) == jahr).count();
        if belege > 0 {
            v.push(a(Typ::Belege, None, belege as i64, false));
        }
        let hat_daten = !self.dokumente.is_empty() || !self.buchungen.is_empty() || !self.kunden.is_empty();
        if hat_daten {
            if self.letztes_backup.is_empty() {
                v.push(a(Typ::Backup, None, -1, false));
            } else if let Some(t) = date::diff_days(&self.letztes_backup, heute).filter(|t| *t >= BACKUP_TAGE) {
                v.push(a(Typ::Backup, None, t, false));
            }
        }
        // dringende zuerst, sonst in der Reihenfolge der Typen
        v.sort_by_key(|x| (x.typ != Typ::Uhr, !x.dringend, x.typ as u8, -x.zahl));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::Data;

    #[test]
    fn aufgaben_ermitteln() {
        let mut d = Data::new();
        let k = d.neuer_kunde("2026-10-01");
        assert!(d.aufgaben("2026-11-13").iter().any(|x| x.typ == Typ::Backup && x.zahl == -1));
        d.letztes_backup = "2026-11-10".into();
        assert!(d.aufgaben("2026-11-13").is_empty());
        // Angebot seit 9 Tagen ohne Antwort
        let an = d.neues_dokument(DokArt::Angebot, Some(k), None, "2026-11-04");
        let x = d.dokument_mut(an).unwrap();
        x.positionen[0].preis = 10_000;
        x.status = DokStatus::Versendet;
        x.versendet_am = "2026-11-04".into();
        // überfällige Rechnung
        let re = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-10-01");
        let x = d.dokument_mut(re).unwrap();
        x.positionen[0].preis = 5_000;
        x.status = DokStatus::Versendet;
        x.faellig = "2026-11-10".into();
        // Auftrag mit Frist morgen
        let au = d.neuer_auftrag(Some(k), "2026-11-01");
        d.auftrag_mut(au).unwrap().termin = "2026-11-14".into();
        let v = d.aufgaben("2026-11-13");
        let typen: Vec<Typ> = v.iter().map(|x| x.typ).collect();
        assert!(matches!(v[0].typ, Typ::Mahnen | Typ::Ueberfaellig));
        assert!(v[0].dringend);
        assert_eq!(v[0].zahl, 3);
        assert!(typen.contains(&Typ::Nachfassen));
        assert!(typen.contains(&Typ::Frist));
        assert!(typen.contains(&Typ::Anfragen));
        // Bestätigung ohne Rechnung → abrechnen; mit Rechnung verschwindet der Punkt
        let ab = d.neues_dokument(DokArt::Auftragsbestaetigung, Some(k), None, "2026-11-12");
        d.dokument_mut(ab).unwrap().status = DokStatus::Unterschrieben;
        assert!(d.aufgaben("2026-11-13").iter().any(|x| x.typ == Typ::Abrechnen && x.id == Some(ab)));
        d.folgedokument(ab, DokArt::Rechnung, "2026-11-13");
        assert!(!d.aufgaben("2026-11-13").iter().any(|x| x.typ == Typ::Abrechnen));
        // laufende Stoppuhr steht immer oben
        d.uhr_starten(au, 0.0);
        assert_eq!(d.aufgaben("2026-11-13")[0].typ, Typ::Uhr);
    }
}
