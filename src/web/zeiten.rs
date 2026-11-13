//! Zeiterfassung im Auftrag: Stoppuhr (läuft auch weiter, wenn die App im Hintergrund ist –
//! gespeichert wird nur die Startzeit), nachgetragene Zeiten und Übernahme in die Rechnung.

use super::views::{btn, ic};
use super::{el, element_value, esc, query, set_timeout, today, App, View};
use crate::engine::date::german;
use crate::engine::model::Id;
use crate::engine::money::fmt_eur;
use crate::engine::zeit::{dauer, parse_dauer};
use std::fmt::Write as _;

fn jetzt_ms() -> f64 {
    js_sys::Date::now()
}

/// „1:02:03“
fn uhrzeit(ms: f64) -> String {
    let s = (ms / 1000.0).max(0.0) as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// Alle Stoppuhr-Anzeigen (`[data-uhr]`) jede Sekunde nachführen.
pub fn bind() {
    fn tick() {
        if let Ok(list) = super::dom().query_selector_all("[data-uhr]") {
            let jetzt = jetzt_ms();
            for i in 0..list.length() {
                if let Some(e) = list.item(i).and_then(|n| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(n).ok()) {
                    if let Some(start) = e.get_attribute("data-uhr").and_then(|v| v.parse::<f64>().ok()) {
                        e.set_text_content(Some(&uhrzeit(jetzt - start)));
                    }
                }
            }
        }
        set_timeout(500, tick);
    }
    tick();
}

impl App {
    pub fn zeiten_card(&self, aid: Id) -> String {
        let Some(a) = self.data.auftrag(Some(aid)) else { return String::new() };
        let f = &self.data.firma;
        let gesamt: u32 = a.zeiten.iter().map(|z| z.minuten).sum();
        let offen: u32 = a.zeiten.iter().filter(|z| !z.abgerechnet).map(|z| z.minuten).sum();
        let mut h = String::from("<div class=\"card zeit-card\"><div class=\"card-head\"><h2>Zeiterfassung</h2>");
        h += &btn("zeit-neu", "plus", "Nachtragen", "ghost sm", Some(aid), "");
        h += "</div>";
        // Stoppuhr
        match a.uhr_start {
            Some(start) => {
                let _ = write!(
                    h,
                    "<div class=\"uhr an\"><span class=\"uhr-dot\"></span><b data-uhr=\"{start}\">{}</b><span class=\"uhr-l\">läuft</span>{}</div>",
                    uhrzeit(jetzt_ms() - start),
                    btn("uhr-stopp", "x", "Stopp", "danger", Some(aid), "")
                );
            }
            None => {
                let andere = self.data.laufende_uhr().is_some();
                let _ = write!(
                    h,
                    "<div class=\"uhr\"><span class=\"uhr-dot\"></span><b>0:00:00</b><span class=\"uhr-l\">{}</span>{}</div>",
                    if andere { "Uhr läuft bei einem anderen Auftrag" } else { "bereit" },
                    if andere { String::new() } else { btn("uhr-start", "clock", "Start", "success", Some(aid), "") }
                );
            }
        }
        // Liste
        if !a.zeiten.is_empty() {
            h += "<div class=\"rows\">";
            for z in a.zeiten.iter().rev() {
                let _ = write!(
                    h,
                    "<div class=\"row static zeit-row\"><span class=\"tdate\">{}</span><input class=\"z-text\" data-f=\"z.{}\" value=\"{}\" placeholder=\"Tätigkeit …\"{}><b class=\"z-dauer\">{}</b>{}<button class=\"icon-btn sm danger-text\" data-act=\"zeit-del\" data-id=\"{aid}\" data-v=\"{}\" title=\"Löschen\">{}</button></div>",
                    esc(&german(&z.datum)),
                    z.id,
                    esc(&z.text),
                    if z.abgerechnet { " disabled" } else { "" },
                    dauer(z.minuten),
                    if z.abgerechnet { "<span class=\"badge st-bezahlt\">abgerechnet</span>" } else { "" },
                    z.id,
                    ic("trash")
                );
            }
            h += "</div>";
        }
        // Summe + Abrechnen
        let satz = if f.stundensatz > 0 {
            format!("{} / Std.", esc(&fmt_eur(f.stundensatz)))
        } else {
            "<a href=\"#\" data-act=\"zeit-satz\">Stundensatz festlegen</a>".to_string()
        };
        // wie in der Rechnung: Stunden je Eintrag auf 0,01 gerundet
        let betrag: i64 = a
            .zeiten
            .iter()
            .filter(|z| !z.abgerechnet)
            .map(|z| ((z.minuten as f64 / 60.0 * 100.0).round() * f.stundensatz as f64 / 100.0).round() as i64)
            .sum();
        let wert = if f.stundensatz > 0 && offen > 0 { format!(" · netto {}", esc(&fmt_eur(betrag))) } else { String::new() };
        let _ = write!(
            h,
            "<div class=\"zeit-foot\"><div><span>Gesamt <b>{}</b> · offen <b>{}</b>{wert}</span><span class=\"muted small\">{satz}</span></div>{}</div></div>",
            dauer(gesamt),
            dauer(offen),
            if offen > 0 { btn("zeit-rechnung", "receipt", "In Rechnung übernehmen", "primary", Some(aid), "") } else { String::new() }
        );
        h
    }

    fn zeit_modal(&mut self, aid: Id) {
        let html = format!(
            "<h3>Zeit nachtragen</h3><div class=\"form\"><label class=\"field\"><span>Datum</span><input type=\"date\" id=\"z-datum\" value=\"{}\"></label><label class=\"field\"><span>Dauer</span><input type=\"text\" id=\"z-dauer\" inputmode=\"decimal\" placeholder=\"1:30 oder 1,5\" autofocus></label><label class=\"field wide\"><span>Tätigkeit (optional)</span><input type=\"text\" id=\"z-text\" placeholder=\"z. B. Rohre verlegt\"></label></div><div class=\"modal-actions\">{}{}</div>",
            today(),
            btn("modal-close", "", "Abbrechen", "ghost", None, ""),
            btn("zeit-save", "check", "Eintragen", "primary", Some(aid), "").replacen("<button ", "<button data-enter ", 1)
        );
        self.modal(&html);
    }

    /// Knöpfe der Zeiterfassung.
    pub fn zeit_action(&mut self, act: &str, id: Option<Id>, v: &str) -> bool {
        let heute = today();
        match act {
            "uhr-start" => {
                let Some(aid) = id else { return true };
                if self.data.uhr_starten(aid, jetzt_ms()) {
                    self.changed();
                    self.render();
                    self.toast("Stoppuhr läuft");
                }
            }
            "uhr-stopp" => {
                let Some(aid) = id else { return true };
                if let Some(min) = self.data.uhr_stoppen(aid, jetzt_ms(), &heute) {
                    self.changed();
                    self.render();
                    self.toast(&format!("{} eingetragen", dauer(min)));
                    // Tätigkeit gleich eintragen lassen
                    if let Some(i) = query(".zeit-row .z-text").and_then(|e| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(e).ok()) {
                        let _ = i.focus();
                    }
                }
            }
            "zeit-neu" => {
                if let Some(aid) = id {
                    self.zeit_modal(aid);
                }
            }
            "zeit-save" => {
                let Some(aid) = id else { return true };
                let wert = |sel: &str| query(sel).map(|e| element_value(&e)).unwrap_or_default();
                let Some(min) = parse_dauer(&wert("#z-dauer")) else {
                    self.toast("Bitte eine Dauer eingeben, z. B. 1:30 oder 1,5");
                    if let Some(e) = el("z-dauer") {
                        let _ = e.focus();
                    }
                    return true;
                };
                let datum = wert("#z-datum");
                self.data.zeit_eintragen(aid, if datum.is_empty() { &heute } else { &datum }, min, &wert("#z-text"));
                self.changed();
                self.close_modal();
                self.render();
                self.toast(&format!("{} eingetragen", dauer(min)));
            }
            "zeit-del" => {
                let (Some(aid), Ok(zid)) = (id, v.parse::<Id>()) else { return true };
                self.ask("Zeit löschen?", "Der Eintrag wird entfernt. Bereits erstellte Rechnungen bleiben unverändert.", "Löschen", true, move |a| {
                    a.data.zeit_loeschen(aid, zid);
                    a.changed();
                    a.render();
                });
            }
            "zeit-rechnung" => {
                let Some(aid) = id else { return true };
                if self.data.firma.stundensatz == 0 {
                    self.toast("Tipp: Stundensatz in den Einstellungen festlegen – Preis bitte in der Rechnung eintragen");
                }
                if let Some(r) = self.data.zeiten_abrechnen(aid, &heute) {
                    self.changed();
                    self.go(View::Dokument(r));
                    self.toast("Zeiten in die Rechnung übernommen");
                }
            }
            "zeit-satz" => {
                self.ui.settings_tab = "bank".into();
                self.go_root(View::Einstellungen);
            }
            _ => return false,
        }
        true
    }

    /// Tätigkeit eines Eintrags direkt in der Liste ändern (`z.<id>`).
    pub fn zeit_feld(&mut self, zid: &str, v: &str) {
        let View::Auftrag(aid) = self.view() else { return };
        let Ok(zid) = zid.parse::<Id>() else { return };
        if let Some(z) = self.data.auftrag_mut(aid).and_then(|a| a.zeiten.iter_mut().find(|z| z.id == zid)) {
            z.text = v.to_string();
            self.changed();
        }
    }
}
