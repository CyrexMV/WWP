//! Reaktionen auf Klicks (`data-act`), Eingaben (`data-f`) und Suchfelder (`data-search`).

use super::storage::{self, load_file, open_file, save_file, share_file};
use super::{apply_theme, checked_of, el, set_html, today, value_of, App, View};
use crate::engine::backup;
use crate::engine::date;
use crate::engine::model::*;
use crate::engine::money::{parse_amount, parse_decimal};
use crate::engine::pdf;
use crate::engine::report;
use wasm_bindgen_futures::spawn_local;

/// Dateiname aus Nummer und Kunde: „Rechnung_RE-2026-0001_Baeckerei-Schoen.pdf“
fn file_name(x: &Dokument, d: &Data) -> String {
    let kunde: String = d
        .kunden_name(x.kunde_id)
        .chars()
        .map(|c| match c {
            'ä' => "ae".to_string(),
            'ö' => "oe".to_string(),
            'ü' => "ue".to_string(),
            'Ä' => "Ae".to_string(),
            'Ö' => "Oe".to_string(),
            'Ü' => "Ue".to_string(),
            'ß' => "ss".to_string(),
            c if c.is_ascii_alphanumeric() => c.to_string(),
            _ => "-".to_string(),
        })
        .collect();
    let kunde = kunde.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    let kunde: String = kunde.chars().take(30).collect();
    if kunde.is_empty() {
        format!("{}_{}.pdf", x.titel().replace(' ', ""), x.nummer)
    } else {
        format!("{}_{}_{}.pdf", x.titel().replace(' ', ""), x.nummer, kunde)
    }
}

enum Out {
    Open,
    Save,
    Send,
}

impl App {
    fn pdf_of(&self, id: Id) -> Option<(Vec<u8>, String)> {
        let x = self.data.dokument(Some(id))?;
        let bytes = pdf::dokument_pdf(&self.data, x, self.logo.as_deref());
        Some((bytes, file_name(x, &self.data)))
    }

    fn output_doc(&mut self, id: Id, out: Out) {
        let Some((bytes, name)) = self.pdf_of(id) else { return };
        match out {
            Out::Open => open_file(&bytes, "application/pdf", &name),
            Out::Save => save_file(&bytes, "application/pdf", &name),
            Out::Send => {
                let Some(x) = self.data.dokument(Some(id)).cloned() else { return };
                let f = self.data.firma.clone();
                let k = self.data.kunde(x.kunde_id).cloned();
                let to = k.as_ref().map(|k| k.email.trim().to_string()).unwrap_or_default();
                let (subject, body) = match x.art {
                    DokArt::Angebot => (x.fill(&f.mail_betreff_angebot, &f, k.as_ref()), x.fill(&f.mail_text_angebot, &f, k.as_ref())),
                    DokArt::Rechnung => (x.fill(&f.mail_betreff_rechnung, &f, k.as_ref()), x.fill(&f.mail_text_rechnung, &f, k.as_ref())),
                };
                share_file(&bytes, "application/pdf", &name, &to, &subject, &body);
                if x.status == DokStatus::Entwurf {
                    self.mark_sent(id);
                }
            }
        }
    }

    /// Mahnschreiben (mit Rechnungskopie) ansehen oder per E-Mail senden.
    fn output_mahnung(&mut self, id: Id, idx: usize, send: bool) {
        let Some(x) = self.data.dokument(Some(id)).cloned() else { return };
        let Some(m) = x.mahnungen.get(idx).cloned() else { return };
        let Some(bytes) = pdf::mahnung_pdf(&self.data, &x, idx, self.logo.as_deref()) else { return };
        let name = file_name(&x, &self.data).replacen("Rechnung_", &format!("{}_", m.label().replace(". ", "").replace(' ', "")), 1);
        if !send {
            open_file(&bytes, "application/pdf", &name);
            return;
        }
        let f = self.data.firma.clone();
        let k = self.data.kunde(x.kunde_id).cloned();
        let to = k.as_ref().map(|k| k.email.trim().to_string()).unwrap_or_default();
        let subject = x.fill_mahnung(&f.mail_betreff_mahnung, &f, k.as_ref(), &m);
        let body = x.fill_mahnung(&f.mail_text_mahnung, &f, k.as_ref(), &m);
        share_file(&bytes, "application/pdf", &name, &to, &subject, &body);
        if let Some(mm) = self.data.dokument_mut(id).and_then(|x| x.mahnungen.get_mut(idx)) {
            mm.versendet = true;
        }
        self.changed();
        self.render();
    }

    fn mark_sent(&mut self, id: Id) {
        let heute = today();
        let Some(x) = self.data.dokument_mut(id) else { return };
        if x.status != DokStatus::Entwurf {
            return;
        }
        x.status = DokStatus::Versendet;
        x.versendet_am = heute.clone();
        let (art, auftrag) = (x.art, x.auftrag_id);
        // Auftrag im Ablauf mitziehen
        if let Some(aid) = auftrag {
            let st = self.data.auftrag(Some(aid)).map(|a| a.status);
            match (art, st) {
                (DokArt::Angebot, Some(AuftragStatus::Anfrage)) => self.data.auftrag_status(aid, AuftragStatus::Angebot, &heute),
                (DokArt::Rechnung, Some(s)) if s != AuftragStatus::Abgerechnet && s != AuftragStatus::Abgelehnt => {
                    self.data.auftrag_status(aid, AuftragStatus::Erledigt, &heute)
                }
                _ => {}
            }
        }
        self.toast("Als versendet markiert");
        self.changed();
        self.render();
    }

    fn new_doc(&mut self, art: DokArt, kunde: Option<Id>, auftrag: Option<Id>) {
        let id = self.data.neues_dokument(art, kunde, auftrag, &today());
        self.close_modal();
        self.changed();
        self.go(View::Dokument(id));
    }

    // -----------------------------------------------------------------------
    // Klicks
    // -----------------------------------------------------------------------

    pub fn action(&mut self, act: &str, id: Option<Id>, v: &str) {
        if self.settings_action(act, v) {
            return;
        }
        let heute = today();
        let idx = v.parse::<usize>().ok();
        match act {
            "nav" => {
                self.close_modal();
                let target = match v {
                    "kunden" => View::Kunden(None),
                    "auftraege" => View::Auftraege,
                    "angebote" => View::Dokumente(DokArt::Angebot),
                    "rechnungen" => View::Dokumente(DokArt::Rechnung),
                    "finanzen" => View::Finanzen,
                    "einstellungen" => View::Einstellungen,
                    _ => View::Start,
                };
                if let View::Dokumente(_) = target {
                    if self.view() != target {
                        self.ui.dok_filter.clear();
                        self.ui.dok_q.clear();
                    }
                }
                self.go_root(target);
            }
            "back" => {
                self.back();
            }
            "modal-close" => self.close_modal(),
            "confirm-yes" => {
                let f = self.confirm.take();
                self.close_modal();
                if let Some(f) = f {
                    f(self);
                }
            }
            "nav-fin" => self.go_root(View::Finanzen),
            "kpi-offen" => {
                self.ui.dok_filter = "offen".into();
                self.ui.dok_q.clear();
                self.go_root(View::Dokumente(DokArt::Rechnung));
            }
            "kpi-mahnen" => {
                self.ui.dok_filter = "zumahnen".into();
                self.ui.dok_q.clear();
                self.go_root(View::Dokumente(DokArt::Rechnung));
            }
            "kpi-angebote" => {
                self.ui.dok_filter = "versendet".into();
                self.ui.dok_q.clear();
                self.go_root(View::Dokumente(DokArt::Angebot));
            }
            "kpi-auftraege" => self.go_root(View::Auftraege),
            "demo" => {
                self.ask("Beispieldaten laden?", "Es werden Beispielkunden, -aufträge, -rechnungen und -buchungen angelegt. Du kannst sie später über „Alle Daten löschen“ entfernen.", "Laden", false, |a| {
                    a.data = backup::demo(&today());
                    a.set_logo_bytes(None);
                    a.changed();
                    a.go_root(View::Start);
                });
            }

            // --- Kunden --------------------------------------------------------
            "kunde-neu" => {
                let k = self.data.neuer_kunde(&heute);
                self.ui.kunden_q.clear();
                self.changed();
                self.go(View::Kunden(Some(k)));
                focus("[data-f='k.firma']");
            }
            "kunde-sel" => self.go(View::Kunden(id)),
            "kunde-open" => {
                self.ui.kunden_q.clear();
                self.go(View::Kunden(id));
            }
            "kunde-del" => {
                let Some(id) = id else { return };
                let name = self.data.kunden_name(Some(id));
                let n = self.data.dokumente_von(id).len();
                let msg = if n > 0 {
                    format!("{name} wird gelöscht. Die {n} Dokumente bleiben erhalten (mit der gespeicherten Anschrift).")
                } else {
                    format!("{name} wird gelöscht.")
                };
                self.ask("Kunden löschen?", &msg, "Löschen", true, move |a| {
                    a.data.kunde_loeschen(id);
                    a.changed();
                    a.stack.pop();
                    a.go(View::Kunden(None));
                });
            }
            "kunde-neu-dok" => {
                // Neuer Kunde direkt aus dem Dokument heraus
                let Some(doc) = id else { return };
                let k = self.data.neuer_kunde(&heute);
                if let Some(x) = self.data.dokument_mut(doc) {
                    x.kunde_id = Some(k);
                }
                self.changed();
                self.go(View::Kunden(Some(k)));
                self.toast("Kunde anlegen – danach mit „Zurück“ zum Dokument");
                focus("[data-f='k.firma']");
            }

            // --- Aufträge ------------------------------------------------------
            "auftrag-neu" => {
                let kunde = id.filter(|k| self.data.kunde(Some(*k)).is_some());
                let a = self.data.neuer_auftrag(kunde, &heute);
                self.changed();
                self.go(View::Auftrag(a));
                focus("[data-f='a.titel']");
            }
            "auftrag-open" => {
                if let Some(id) = id {
                    self.go(View::Auftrag(id));
                }
            }
            "auftrag-next" => {
                let Some(id) = id else { return };
                if let Some(n) = self.data.auftrag(Some(id)).and_then(|a| a.status.next()) {
                    self.data.auftrag_status(id, n, &heute);
                    self.changed();
                    self.toast(&format!("Status: {}", n.label()));
                    self.render();
                }
            }
            "auftrag-reject" => {
                if let Some(id) = id {
                    self.data.auftrag_status(id, AuftragStatus::Abgelehnt, &heute);
                    self.changed();
                    self.render();
                }
            }
            "auftrag-note" => {
                let Some(id) = id else { return };
                let t = value_of("note-text").trim().to_string();
                if t.is_empty() {
                    focus("#note-text");
                    return;
                }
                if let Some(a) = self.data.auftrag_mut(id) {
                    a.verlauf.push(Verlauf { datum: heute, text: t });
                }
                self.changed();
                self.render();
            }
            "auftrag-dok" => {
                let Some(id) = id else { return };
                let art = DokArt::from_key(v).unwrap_or(DokArt::Angebot);
                let kunde = self.data.auftrag(Some(id)).and_then(|a| a.kunde_id);
                self.new_doc(art, kunde, Some(id));
            }
            "auftrag-del" => {
                let Some(id) = id else { return };
                self.ask("Auftrag löschen?", "Der Auftrag samt Verlauf wird gelöscht. Angebote und Rechnungen bleiben erhalten.", "Löschen", true, move |a| {
                    a.data.auftrag_loeschen(id);
                    a.changed();
                    a.back();
                });
            }

            // --- Dokumente -------------------------------------------------------
            "dok-neu" => {
                let art = DokArt::from_key(v).unwrap_or(DokArt::Rechnung);
                if self.data.kunden.is_empty() {
                    self.new_doc(art, None, None);
                } else {
                    self.pick_kunde_modal(art);
                }
            }
            "dok-neu-kunde" => {
                let art = DokArt::from_key(v).unwrap_or(DokArt::Rechnung);
                self.new_doc(art, id, None);
            }
            "dok-neu-ohne" => {
                let art = DokArt::from_key(v).unwrap_or(DokArt::Rechnung);
                self.new_doc(art, None, None);
            }
            "dok-neu-neukunde" => {
                let art = DokArt::from_key(v).unwrap_or(DokArt::Rechnung);
                let k = self.data.neuer_kunde(&heute);
                let d = self.data.neues_dokument(art, Some(k), None, &heute);
                self.close_modal();
                self.changed();
                self.stack.push(View::Dokument(d));
                self.go(View::Kunden(Some(k)));
                self.toast("Kunde anlegen – danach mit „Zurück“ zum Dokument");
                focus("[data-f='k.firma']");
            }
            "dok-open" => {
                if let Some(id) = id {
                    self.go(View::Dokument(id));
                }
            }
            "dok-filter" => {
                self.ui.dok_filter = v.to_string();
                self.render();
            }
            "dok-pdf" => {
                if let Some(id) = id {
                    self.output_doc(id, Out::Open);
                }
            }
            "dok-save" => {
                if let Some(id) = id {
                    self.output_doc(id, Out::Save);
                }
            }
            "dok-send" => {
                let Some(id) = id else { return };
                let x = self.data.dokument(Some(id)).cloned();
                if let Some(x) = x {
                    if x.summen().brutto == 0 && x.positionen.iter().all(|p| p.text.trim().is_empty()) {
                        self.toast("Bitte zuerst Positionen eintragen");
                        return;
                    }
                }
                self.output_doc(id, Out::Send);
            }
            "mahn-new" => {
                let Some(id) = id else { return };
                let stufe = self.data.dokument(Some(id)).map(|x| x.naechste_stufe()).unwrap_or(0);
                self.mahn_modal(id, stufe);
            }
            "mahn-ok" => {
                let Some(id) = id else { return };
                let stufe: u8 = value_of("mahn-stufe").parse().unwrap_or(0);
                let datum = value_of("mahn-datum");
                let datum = if date::parse(&datum).is_some() { datum } else { heute.clone() };
                let frist = value_of("mahn-frist");
                let frist = if date::parse(&frist).is_some() {
                    frist
                } else {
                    date::add_days(&datum, self.data.firma.mahnstufe(stufe).0 as i64)
                };
                let gebuehr = parse_amount(&value_of("mahn-gebuehr")).unwrap_or(0);
                let zinsen = checked_of("mahn-zinsen");
                let Some(idx) = self.data.mahnung_erstellen(id, stufe, &datum, &frist, gebuehr, zinsen) else { return };
                self.close_modal();
                self.changed();
                self.render();
                self.output_mahnung(id, idx, v == "send");
            }
            "mahn-pdf" | "mahn-send" => {
                let (Some(id), Some(i)) = (id, idx) else { return };
                self.output_mahnung(id, i, act == "mahn-send");
            }
            "mahn-del" => {
                let (Some(id), Some(i)) = (id, idx) else { return };
                self.ask("Mahnung löschen?", "Die Mahnung und ihre Gebühren werden aus der Rechnung entfernt.", "Löschen", true, move |a| {
                    a.data.mahnung_loeschen(id, i);
                    a.changed();
                    a.render();
                });
            }
            "zahlung-del" => {
                let Some(id) = id else { return };
                let Ok(zid) = v.parse::<Id>() else { return };
                self.ask("Zahlung löschen?", "Die Zahlung und die zugehörige Einnahme werden gelöscht; der Betrag gilt wieder als offen.", "Löschen", true, move |a| {
                    a.data.zahlung_loeschen(id, zid);
                    a.changed();
                    a.render();
                });
            }
            "dok-mark-sent" => {
                if let Some(id) = id {
                    self.mark_sent(id);
                }
            }
            "dok-status" => {
                let (Some(id), Some(st)) = (id, DokStatus::from_key(v)) else { return };
                let auftrag = self.data.dokument(Some(id)).and_then(|x| x.auftrag_id);
                if let Some(x) = self.data.dokument_mut(id) {
                    x.status = st;
                }
                if let Some(aid) = auftrag {
                    match st {
                        DokStatus::Angenommen => self.data.auftrag_status(aid, AuftragStatus::Beauftragt, &heute),
                        DokStatus::Abgelehnt => self.data.auftrag_status(aid, AuftragStatus::Abgelehnt, &heute),
                        _ => {}
                    }
                }
                self.changed();
                self.render();
            }
            "dok-unlock" => {
                if let Some(id) = id {
                    self.ui.entsperrt.insert(id);
                    self.render();
                }
            }
            "dok-paid" => {
                if let Some(id) = id {
                    self.paid_modal(id);
                }
            }
            "dok-paid-ok" => {
                let Some(id) = id else { return };
                let datum = value_of("pay-date");
                let datum = if date::parse(&datum).is_some() { datum } else { heute };
                let za = value_of("pay-art");
                let rest = self.data.dokument(Some(id)).map(|x| x.rest()).unwrap_or(0);
                let betrag = parse_amount(&value_of("pay-betrag")).unwrap_or(rest);
                if betrag <= 0 {
                    self.toast("Bitte einen Betrag eingeben");
                    return;
                }
                self.data.zahlung_erfassen(id, &datum, betrag, &za);
                self.close_modal();
                self.changed();
                self.render();
                let rest = self.data.dokument(Some(id)).map(|x| x.rest()).unwrap_or(0);
                if rest > 0 {
                    self.toast(&format!("Teilzahlung gebucht – noch offen: {}", crate::engine::money::fmt_eur(rest)));
                } else {
                    self.toast("Bezahlt – Einnahme wurde gebucht");
                }
            }
            "dok-unpaid" => {
                let Some(id) = id else { return };
                self.ask("Zahlungen zurücknehmen?", "Alle Zahlungen dieser Rechnung und die zugehörigen Einnahmen werden gelöscht; die Rechnung gilt wieder als offen.", "Zurücknehmen", true, move |a| {
                    a.data.zahlung_zuruecknehmen(id);
                    a.changed();
                    a.render();
                });
            }
            "dok-to-invoice" => {
                let Some(id) = id else { return };
                if let Some(r) = self.data.rechnung_aus_angebot(id, &heute) {
                    self.changed();
                    self.go(View::Dokument(r));
                    self.toast("Rechnung aus Angebot erstellt");
                }
            }
            "dok-dup" => {
                let Some(id) = id else { return };
                if let Some(n) = self.data.duplizieren(id, &heute) {
                    self.changed();
                    self.go(View::Dokument(n));
                    self.toast("Kopie erstellt");
                }
            }
            "dok-storno" => {
                let Some(id) = id else { return };
                self.ask(
                    "Rechnung stornieren?",
                    "Es wird eine Stornorechnung mit negativen Beträgen erstellt und die Rechnung als storniert markiert. War sie bereits bezahlt, buche die Rückzahlung bitte als Ausgabe.",
                    "Stornieren",
                    true,
                    move |a| {
                        if let Some(n) = a.data.stornieren(id, &today()) {
                            a.changed();
                            a.go(View::Dokument(n));
                        }
                    },
                );
            }
            "dok-del" => {
                let Some(id) = id else { return };
                let Some(x) = self.data.dokument(Some(id)) else { return };
                let warn = if x.art == DokArt::Rechnung && x.status != DokStatus::Entwurf {
                    "Achtung: Versendete Rechnungen sollten aus steuerlichen Gründen nicht gelöscht, sondern storniert werden. Trotzdem löschen?"
                } else {
                    "Das Dokument wird endgültig gelöscht."
                };
                let title = format!("{} {} löschen?", x.titel(), x.nummer);
                self.ask(&title, warn, "Löschen", true, move |a| {
                    a.data.dokument_loeschen(id);
                    a.changed();
                    a.back();
                });
            }
            "pos-add" => {
                if let View::Dokument(id) = self.view() {
                    let ust = self.data.firma.ust_satz();
                    if let Some(x) = self.data.dokument_mut(id) {
                        x.positionen.push(Position { ust, ..Default::default() });
                    }
                    self.changed();
                    self.render();
                    let n = self.data.dokument(Some(id)).map(|x| x.positionen.len()).unwrap_or(1);
                    focus(&format!("[data-f='p.{}.text']", n - 1));
                }
            }
            "pos-del" | "pos-up" | "pos-down" | "pos-dup" => {
                let (View::Dokument(id), Some(i)) = (self.view(), idx) else { return };
                if let Some(x) = self.data.dokument_mut(id) {
                    let n = x.positionen.len();
                    if i >= n {
                        return;
                    }
                    match act {
                        "pos-del" => {
                            x.positionen.remove(i);
                        }
                        "pos-up" if i > 0 => x.positionen.swap(i, i - 1),
                        "pos-down" if i + 1 < n => x.positionen.swap(i, i + 1),
                        "pos-dup" => {
                            let p = x.positionen[i].clone();
                            x.positionen.insert(i + 1, p);
                        }
                        _ => {}
                    }
                }
                self.changed();
                self.render();
            }
            "pos-katalog" => self.katalog_modal(),
            "kat-pick" => {
                let (View::Dokument(did), Some(lid)) = (self.view(), id) else { return };
                let Some(l) = self.data.katalog.iter().find(|l| l.id == lid).cloned() else { return };
                if let Some(x) = self.data.dokument_mut(did) {
                    let p = Position { text: l.text, menge: 1.0, einheit: l.einheit, preis: l.preis, ust: l.ust };
                    // leere Startposition ersetzen
                    if x.positionen.len() == 1 && x.positionen[0].text.trim().is_empty() && x.positionen[0].preis == 0 {
                        x.positionen[0] = p;
                    } else {
                        x.positionen.push(p);
                    }
                }
                self.close_modal();
                self.changed();
                self.render();
                self.toast("Position eingefügt");
            }

            // --- Finanzen --------------------------------------------------------
            "buch-neu" => {
                let art = if v == "einnahme" { BuchArt::Einnahme } else { BuchArt::Ausgabe };
                let b = self.data.neue_buchung(art, &heute);
                self.changed();
                self.go(View::Buchung(b));
            }
            "buch-open" => {
                if let Some(id) = id {
                    self.go(View::Buchung(id));
                }
            }
            "buch-art" => {
                let View::Buchung(id) = self.view() else { return };
                let art = if v == "einnahme" { BuchArt::Einnahme } else { BuchArt::Ausgabe };
                if let Some(b) = self.data.buchung_mut(id) {
                    if b.art != art {
                        b.art = art;
                        if !kategorien(art).iter().any(|k| k.name == b.kategorie) {
                            b.kategorie = kategorien(art)[0].name.into();
                        }
                    }
                }
                self.changed();
                self.render();
            }
            "buch-filter" => {
                self.ui.buch_filter = v.to_string();
                if self.view() != View::Finanzen {
                    self.go_root(View::Finanzen);
                } else {
                    self.render();
                }
            }
            "buch-del" => {
                let Some(id) = id else { return };
                let paid = self.data.buchung(Some(id)).and_then(|b| b.dokument_id);
                let msg = if paid.is_some() {
                    "Die Buchung gehört zu einer bezahlten Rechnung. Besser: in der Rechnung „Zahlung zurücknehmen“. Trotzdem löschen?"
                } else {
                    "Die Buchung und ihr Beleg werden gelöscht."
                };
                self.ask("Buchung löschen?", msg, "Löschen", true, move |a| {
                    if let Some(f) = a.data.buchung_loeschen(id) {
                        a.drop_file(f);
                    }
                    a.changed();
                    a.back();
                });
            }
            "beleg-add" => {
                self.beleg_ziel = id;
                storage::pick_file("file-beleg");
            }
            "beleg-view" => {
                let Some(fid) = id.and_then(|b| self.data.buchung(Some(b))).and_then(|b| b.beleg) else { return };
                let Some(meta) = self.data.datei(Some(fid)).cloned() else { return };
                spawn_local(async move {
                    match load_file(fid).await {
                        Some(bytes) => open_file(&bytes, &meta.mime, &meta.name),
                        None => super::toast("Beleg nicht gefunden"),
                    }
                });
            }
            "beleg-del" => {
                let Some(bid) = id else { return };
                self.ask("Beleg entfernen?", "Die angehängte Datei wird gelöscht.", "Entfernen", true, move |a| {
                    let f = a.data.buchung_mut(bid).and_then(|b| b.beleg.take());
                    if let Some(f) = f {
                        a.drop_file(f);
                    }
                    a.changed();
                    a.render();
                });
            }
            "fin-csv" => {
                let csv = report::csv_buchungen(&self.data, Some(self.ui.jahr));
                save_file(csv.as_bytes(), "text/csv", &format!("Buchungen_{}.csv", self.ui.jahr));
            }
            "fin-csv-re" => {
                let csv = report::csv_rechnungen(&self.data, Some(self.ui.jahr));
                save_file(csv.as_bytes(), "text/csv", &format!("Rechnungsausgangsbuch_{}.csv", self.ui.jahr));
            }
            "fin-pdf" => {
                let a = report::auswertung(&self.data, self.ui.jahr);
                let bytes = pdf::auswertung_pdf(&self.data, &a, &heute, self.logo.as_deref());
                open_file(&bytes, "application/pdf", &format!("Finanzuebersicht_{}.pdf", self.ui.jahr));
            }

            // --- Einstellungen ---------------------------------------------------
            "logo-add" => storage::pick_file("file-logo"),
            "logo-del" => {
                self.ask("Logo entfernen?", "Auf den PDFs erscheint dann der Firmenname.", "Entfernen", true, |a| {
                    if let Some(l) = a.data.firma.logo.take() {
                        a.drop_file(l);
                    }
                    a.set_logo_bytes(None);
                    a.changed();
                    a.render();
                });
            }
            "preview" => {
                // Muster-Rechnung mit den aktuellen Einstellungen
                let mut d = self.data.clone();
                let k = d.neuer_kunde(&heute);
                if let Some(kd) = d.kunde_mut(k) {
                    kd.firma = "Beispielkunde GmbH".into();
                    kd.anrede = "Frau".into();
                    kd.ansprechpartner = "Erika Mustermann".into();
                    kd.strasse = "Musterstraße 1".into();
                    kd.plz = "12345".into();
                    kd.ort = "Musterstadt".into();
                }
                let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, &heute);
                let muster = format!("{}-MUSTER", d.firma.prefix_rechnung);
                if let Some(x) = d.dokument_mut(r) {
                    x.nummer = muster;
                }
                let ust = d.firma.ust_satz();
                if let Some(x) = d.dokument_mut(r) {
                    x.betreff = "Musterrechnung (Vorschau)".into();
                    x.positionen = vec![
                        Position { text: "Beratung und Planung\nVor-Ort-Termin inkl. Aufmaß".into(), menge: 2.0, einheit: "Std.".into(), preis: 7_500, ust },
                        Position { text: "Material".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 12_000, ust },
                    ];
                }
                if let Some(x) = d.dokument(Some(r)) {
                    let bytes = pdf::dokument_pdf(&d, x, self.logo.as_deref());
                    open_file(&bytes, "application/pdf", "Briefbogen-Vorschau.pdf");
                }
            }
            "kat-add" => {
                let id = self.data.new_id();
                let ust = self.data.firma.ust_satz();
                self.data.katalog.push(Leistung { id, ust, ..Default::default() });
                self.changed();
                self.render();
                let n = self.data.katalog.len();
                focus(&format!("[data-f='l.{}.text']", n - 1));
            }
            "kat-del" => {
                if let Some(i) = idx {
                    if i < self.data.katalog.len() {
                        self.data.katalog.remove(i);
                        self.changed();
                        self.render();
                    }
                }
            }
            "theme" => {
                apply_theme(v);
                self.render();
            }
            "backup" => self.create_backup(false),
            "backup-share" => self.create_backup(true),
            "restore" => storage::pick_file("file-restore"),
            "wipe" => {
                self.ask("Alle Daten löschen?", "Kunden, Aufträge, Angebote, Rechnungen, Buchungen, Belege und Einstellungen werden unwiderruflich gelöscht. Erstelle vorher ein Backup!", "Weiter", true, |a| {
                    a.ask("Wirklich alles löschen?", "Dieser Schritt kann nicht rückgängig gemacht werden.", "Endgültig löschen", true, |a| a.wipe());
                });
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Suchfelder
    // -----------------------------------------------------------------------

    pub fn search(&mut self, key: &str, v: &str) {
        match key {
            "kunden" => {
                self.ui.kunden_q = v.to_string();
                let sel = if let View::Kunden(s) = self.view() { s } else { None };
                set_html("list", &self.kunden_list_html(sel));
            }
            "dok" => {
                self.ui.dok_q = v.to_string();
                if let View::Dokumente(art) = self.view() {
                    set_html("list", &self.dok_list_html(art));
                }
            }
            "buch" => {
                self.ui.buch_q = v.to_string();
                set_html("list", &self.buch_list_html());
            }
            "buch-monat" => {
                self.ui.buch_monat = v.parse().unwrap_or(0);
                set_html("list", &self.buch_list_html());
            }
            "mahn-stufe" => {
                if let View::Dokument(id) = self.view() {
                    self.mahn_modal(id, v.parse().unwrap_or(0));
                }
            }
            "pick-kunde" => {
                let art = super::query("#pick-q").and_then(|e| e.get_attribute("data-v")).unwrap_or_default();
                set_html("pick-list", &self.kunden_pick_list(v, &art));
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Felder
    // -----------------------------------------------------------------------

    pub fn field(&mut self, key: &str, v: &str, checked: bool) {
        let (scope, rest) = key.split_once('.').unwrap_or((key, ""));
        let view = self.view();
        match (scope, view) {
            ("ui", _) => {
                match rest {
                    "jahr" => {
                        self.ui.jahr = v.parse().unwrap_or(self.ui.jahr);
                        self.ui.buch_monat = 0;
                    }
                    "abgelehnte" => self.ui.abgelehnte = checked,
                    _ => {}
                }
                self.render();
                return;
            }
            ("k", View::Kunden(Some(id))) => {
                let Some(k) = self.data.kunde_mut(id) else { return };
                let s = v.to_string();
                match rest {
                    "firma" => k.firma = s,
                    "anrede" => k.anrede = s,
                    "ansprechpartner" => k.ansprechpartner = s,
                    "strasse" => k.strasse = s,
                    "plz" => k.plz = s,
                    "ort" => k.ort = s,
                    "land" => k.land = s,
                    "email" => k.email = s,
                    "telefon" => k.telefon = s,
                    "ust_id" => k.ust_id = s,
                    "notizen" => k.notizen = s,
                    _ => return,
                }
                // Entwürfe dieses Kunden übernehmen die neue Anschrift
                let anschrift = k.anschrift();
                for x in self.data.dokumente.iter_mut().filter(|x| x.kunde_id == Some(id) && x.status == DokStatus::Entwurf) {
                    x.empfaenger = anschrift.clone();
                }
                if matches!(rest, "firma" | "ansprechpartner" | "plz" | "ort") {
                    set_html("list", &self.kunden_list_html(Some(id)));
                }
            }
            ("a", View::Auftrag(id)) => {
                let heute = today();
                if rest == "status" {
                    if let Some(st) = AuftragStatus::from_key(v) {
                        self.data.auftrag_status(id, st, &heute);
                        self.changed();
                        self.render();
                    }
                    return;
                }
                let Some(a) = self.data.auftrag_mut(id) else { return };
                let s = v.to_string();
                match rest {
                    "titel" => a.titel = s,
                    "beschreibung" => a.beschreibung = s,
                    "notizen" => a.notizen = s,
                    "eingang" => a.eingang = s,
                    "termin" => a.termin = s,
                    "wert" => a.wert = parse_amount(v).unwrap_or(0),
                    "kunde_id" => {
                        a.kunde_id = v.parse().ok();
                        self.changed();
                        self.render();
                        return;
                    }
                    _ => return,
                }
            }
            ("d", View::Dokument(id)) | ("p", View::Dokument(id)) => {
                if !self.field_doc(id, scope, rest, v, checked) {
                    return;
                }
            }
            ("b", View::Buchung(id)) => {
                let Some(b) = self.data.buchung_mut(id) else { return };
                let s = v.to_string();
                let mut rerender = false;
                match rest {
                    "brutto" => b.brutto = parse_amount(v).unwrap_or(0),
                    "ust" => b.ust = v.parse().unwrap_or(0),
                    "datum" => b.datum = s,
                    "kategorie" => {
                        b.kategorie = s;
                        if ist_neutral(&b.kategorie) {
                            b.ust = 0;
                        }
                        rerender = true;
                    }
                    "text" => b.text = s,
                    "partner" => b.partner = s,
                    "zahlart" => b.zahlart = s,
                    _ => return,
                }
                let html = self.buch_calc_html(self.data.buchung(Some(id)).unwrap());
                set_html("buch-calc", &html);
                if rerender {
                    self.changed();
                    self.render();
                    return;
                }
            }
            ("f", View::Einstellungen) => {
                let f = &mut self.data.firma;
                let s = v.to_string();
                let mut rerender = false;
                let mut nav = false;
                match rest {
                    "name" => {
                        f.name = s;
                        nav = true;
                    }
                    "zusatz" => f.zusatz = s,
                    "strasse" => f.strasse = s,
                    "plz_ort" => f.plz_ort = s,
                    "telefon" => f.telefon = s,
                    "email" => f.email = s,
                    "web" => f.web = s,
                    "bank" => f.bank = s,
                    "iban" => f.iban = s.to_uppercase(),
                    "bic" => f.bic = s.to_uppercase(),
                    "steuernummer" => f.steuernummer = s,
                    "ust_id" => f.ust_id = s,
                    "kleinunternehmer" => {
                        f.kleinunternehmer = checked;
                        rerender = true;
                    }
                    "ust_standard" => f.ust_standard = v.parse().unwrap_or(19),
                    "zahlungsziel" => f.zahlungsziel = v.trim().parse().unwrap_or(f.zahlungsziel),
                    "angebot_gueltig" => f.angebot_gueltig = v.trim().parse().unwrap_or(f.angebot_gueltig),
                    "prefix_angebot" => f.prefix_angebot = s.trim().to_string(),
                    "prefix_rechnung" => f.prefix_rechnung = s.trim().to_string(),
                    "einleitung_angebot" => f.einleitung_angebot = s,
                    "schluss_angebot" => f.schluss_angebot = s,
                    "einleitung_rechnung" => f.einleitung_rechnung = s,
                    "schluss_rechnung" => f.schluss_rechnung = s,
                    "mail_betreff_angebot" => f.mail_betreff_angebot = s,
                    "mail_text_angebot" => f.mail_text_angebot = s,
                    "mail_betreff_rechnung" => f.mail_betreff_rechnung = s,
                    "mail_text_rechnung" => f.mail_text_rechnung = s,
                    "farbe" => f.farbe = s,
                    "logo_breite_mm" => f.logo_breite_mm = v.parse().unwrap_or(45.0),
                    "girocode" => f.girocode = checked,
                    "verzugszinsen" => f.verzugszinsen = checked,
                    "basiszins" => f.basiszins = parse_decimal(v).unwrap_or(f.basiszins),
                    "mail_betreff_mahnung" => f.mail_betreff_mahnung = s,
                    "mail_text_mahnung" => f.mail_text_mahnung = s,
                    k if k.starts_with("mahn_") => {
                        // mahn_tage.N / mahn_gebuehr.N / mahn_text.N
                        let Some((feld, i)) = k.split_once('.') else { return };
                        let Ok(i) = i.parse::<usize>() else { return };
                        if i > 2 {
                            return;
                        }
                        let (t, _, x) = f.mahnstufe(i as u8);
                        // Listen auf drei Stufen auffüllen
                        while f.mahn_tage.len() < 3 {
                            let n = f.mahn_tage.len() as u8;
                            f.mahn_tage.push(f.mahnstufe(n).0);
                        }
                        while f.mahn_gebuehr.len() < 3 {
                            let n = f.mahn_gebuehr.len() as u8;
                            f.mahn_gebuehr.push(f.mahnstufe(n).1);
                        }
                        while f.mahn_text.len() < 3 {
                            let n = f.mahn_text.len() as u8;
                            f.mahn_text.push(f.mahnstufe(n).2);
                        }
                        match feld {
                            "mahn_tage" => f.mahn_tage[i] = v.trim().parse().unwrap_or(t),
                            "mahn_gebuehr" => f.mahn_gebuehr[i] = parse_amount(v).unwrap_or(0).max(0),
                            "mahn_text" => f.mahn_text[i] = if v.trim().is_empty() { x } else { s },
                            _ => return,
                        }
                    }
                    _ => return,
                }
                if rerender {
                    self.changed();
                    self.render();
                    return;
                }
                if nav {
                    set_html("nav", &self.nav_html());
                }
                self.settings_refresh();
            }
            ("l", View::Einstellungen) => {
                let mut it = rest.splitn(2, '.');
                let (Some(i), Some(fld)) = (it.next().and_then(|i| i.parse::<usize>().ok()), it.next()) else { return };
                let Some(l) = self.data.katalog.get_mut(i) else { return };
                match fld {
                    "text" => l.text = v.to_string(),
                    "einheit" => l.einheit = v.to_string(),
                    "preis" => l.preis = parse_amount(v).unwrap_or(0),
                    "ust" => l.ust = v.parse().unwrap_or(19),
                    _ => return,
                }
            }
            _ => return,
        }
        self.changed();
    }

    /// Feld im Dokument-Editor; `false`, wenn nichts geändert wurde.
    fn field_doc(&mut self, id: Id, scope: &str, rest: &str, v: &str, checked: bool) -> bool {
        let locked = self.data.dokument(Some(id)).map(|x| x.festgeschrieben()).unwrap_or(true) && !self.ui.entsperrt.contains(&id);
        if locked {
            return false;
        }
        let kunde_anschrift = |d: &Data, k: Option<Id>| d.kunde(k).map(|k| (k.anschrift(), briefanrede(k)));
        if scope == "d" && rest == "kunde_id" {
            let k: Option<Id> = v.parse().ok();
            let info = kunde_anschrift(&self.data, k);
            let alte_anrede = self
                .data
                .dokument(Some(id))
                .and_then(|x| self.data.kunde(x.kunde_id))
                .map(briefanrede)
                .unwrap_or_else(|| "Sehr geehrte Damen und Herren,".into());
            if let Some(x) = self.data.dokument_mut(id) {
                x.kunde_id = k;
                if let Some((anschrift, anrede)) = info {
                    x.empfaenger = anschrift;
                    // Briefanrede in der Einleitung mitziehen
                    if x.einleitung.starts_with(&alte_anrede) {
                        x.einleitung = x.einleitung.replacen(&alte_anrede, &anrede, 1);
                    }
                }
            }
            self.changed();
            self.render();
            return false;
        }
        let Some(x) = self.data.dokument_mut(id) else { return false };
        let mut rerender = false;
        if scope == "d" {
            let s = v.to_string();
            match rest {
                "empfaenger" => x.empfaenger = s,
                "datum" => {
                    // Fälligkeit mit dem Datum verschieben
                    if let Some(diff) = date::diff_days(&x.datum, &x.faellig) {
                        x.faellig = date::add_days(&s, diff);
                        rerender = true;
                    }
                    x.datum = s;
                }
                "leistung" => x.leistung = s,
                "faellig" => x.faellig = s,
                "betreff" => x.betreff = s,
                "einleitung" => x.einleitung = s,
                "schluss" => x.schluss = s,
                "kleinunternehmer" => {
                    x.kleinunternehmer = checked;
                    rerender = true;
                }
                _ => return false,
            }
        } else {
            // p.<i>.<feld>
            let mut it = rest.splitn(2, '.');
            let (Some(i), Some(fld)) = (it.next().and_then(|i| i.parse::<usize>().ok()), it.next()) else { return false };
            let Some(p) = x.positionen.get_mut(i) else { return false };
            match fld {
                "text" => p.text = v.to_string(),
                "menge" => p.menge = parse_decimal(v).unwrap_or(0.0),
                "einheit" => p.einheit = v.to_string(),
                "preis" => p.preis = parse_amount(v).unwrap_or(0),
                "ust" => p.ust = v.parse().unwrap_or(19),
                _ => return false,
            }
            let total = crate::engine::money::fmt_eur(p.netto());
            if let Some(e) = el(&format!("lt-{i}")) {
                e.set_text_content(Some(&total));
            }
        }
        let x = self.data.dokument(Some(id)).unwrap();
        set_html("doc-sum", &self.doc_sum_html(x));
        if rerender {
            self.changed();
            self.render();
            return false;
        }
        true
    }
}

/// Feld nach dem Zeichnen fokussieren.
fn focus(sel: &str) {
    let sel = sel.to_string();
    super::set_timeout(60, move || {
        if let Some(e) = super::query(&sel) {
            use wasm_bindgen::JsCast;
            if let Ok(h) = e.dyn_into::<web_sys::HtmlElement>() {
                let _ = h.focus();
            }
        }
    });
}
