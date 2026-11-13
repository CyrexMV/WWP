//! Kalender: Wochen- und Monatsansicht der Termine (plus Fristen der Aufträge), Termin-Dialog,
//! Übernahme in die Kalender-App (Android) bzw. .ics-Datei und Route in Google Maps.

use super::views::{btn, head, ic};
use super::{esc, today, value_of, App};
use crate::engine::date::{self, german, german_short, MONTHS};
use crate::engine::model::{AuftragStatus, Id};
use crate::engine::termine::{ics, kalenderwoche, wochenanfang, Termin, WT_KURZ};
use std::fmt::Write as _;

impl App {
    fn kal_tag(&self) -> String {
        if date::parse(&self.ui.kal_tag).is_some() {
            self.ui.kal_tag.clone()
        } else {
            today()
        }
    }

    /// Einträge eines Tages: Termine und Auftragsfristen.
    fn tag_eintraege(&self, tag: &str, kompakt: bool) -> String {
        let mut h = String::new();
        for a in self.data.auftraege.iter().filter(|a| a.termin == tag && a.status.offen() && a.status != AuftragStatus::Erledigt) {
            let _ = write!(
                h,
                "<button class=\"ev frist\" data-act=\"auftrag-open\" data-id=\"{}\" title=\"Frist: {}\">{}<span>{}</span></button>",
                a.id,
                esc(&a.titel()),
                ic("alert"),
                esc(&a.titel())
            );
        }
        for t in self.data.termine_am(tag) {
            let kunde = self.data.kunde(t.kunde_id).map(|k| k.name()).unwrap_or_default();
            if kompakt {
                let zeit = if t.ganztag() { String::new() } else { format!("<em>{}</em>", esc(t.von.trim())) };
                let _ = write!(h, "<button class=\"ev\" data-act=\"termin-open\" data-id=\"{}\">{zeit}<span>{}</span></button>", t.id, esc(&t.titel));
            } else {
                let _ = write!(
                    h,
                    "<button class=\"ev big{}\" data-act=\"termin-open\" data-id=\"{}\"><em>{}</em><b>{}</b>{}{}</button>",
                    if t.ganztag() { " allday" } else { "" },
                    t.id,
                    esc(&t.zeit()),
                    esc(&t.titel),
                    if kunde.is_empty() { String::new() } else { format!("<span>{}</span>", esc(&kunde)) },
                    if t.ort.trim().is_empty() { String::new() } else { format!("<span class=\"ort\">{}{}</span>", ic("map"), esc(t.ort.trim())) }
                );
            }
        }
        h
    }

    pub fn kalender_html(&self) -> String {
        let heute = today();
        let tag = self.kal_tag();
        let monat_ansicht = self.ui.kal_ansicht == "monat";
        let start = wochenanfang(&tag);
        let (y, m, _) = date::parse(&tag).unwrap_or((2026, 1, 1));
        let titel = if monat_ansicht {
            format!("{} {y}", MONTHS[(m - 1) as usize])
        } else {
            let ende = date::add_days(&start, 6);
            format!("KW {} · {} – {}", kalenderwoche(&start), german_short(&start), german(&ende))
        };
        let actions = format!(
            "<div class=\"segmented\"><button class=\"{}\" data-act=\"kal-ansicht\" data-v=\"woche\">Woche</button><button class=\"{}\" data-act=\"kal-ansicht\" data-v=\"monat\">Monat</button></div><div class=\"kal-nav\"><button class=\"icon-btn\" data-act=\"kal-nav\" data-v=\"-1\" title=\"Zurück\">{}</button><button class=\"btn ghost sm\" data-act=\"kal-nav\" data-v=\"0\">Heute</button><button class=\"icon-btn\" data-act=\"kal-nav\" data-v=\"1\" title=\"Weiter\">{}</button></div>{}{}",
            if monat_ansicht { "" } else { "active" },
            if monat_ansicht { "active" } else { "" },
            ic("back"),
            ic("chevron"),
            btn("kal-ics", "download", ".ics", "ghost sm", None, ""),
            btn("termin-neu", "plus", "Termin", "primary", None, &tag)
        );
        let mut h = head("Kalender", &titel, &actions, false);
        if monat_ansicht {
            let erster = date::iso(y, m, 1);
            let raster_start = wochenanfang(&erster);
            h += "<div class=\"month\">";
            for wt in WT_KURZ {
                let _ = write!(h, "<div class=\"mh\">{wt}</div>");
            }
            for i in 0..42 {
                let d = date::add_days(&raster_start, i);
                let anderer = date::month(&d) != Some(m);
                if i >= 35 && anderer {
                    break;
                }
                let tagnr = date::parse(&d).map(|p| p.2).unwrap_or(1);
                let _ = write!(
                    h,
                    "<div class=\"mc{}{}\"><button class=\"mday\" data-act=\"kal-tag\" data-v=\"{d}\">{tagnr}</button><div class=\"evs\">{}</div></div>",
                    if anderer { " other" } else { "" },
                    if d == heute { " today" } else { "" },
                    self.tag_eintraege(&d, true)
                );
            }
            h += "</div>";
        } else {
            h += "<div class=\"week\">";
            for i in 0..7 {
                let d = date::add_days(&start, i);
                let _ = write!(
                    h,
                    "<div class=\"wd{}\"><div class=\"wh\"><span>{}</span><b>{}</b></div><div class=\"evs\">{}</div><button class=\"wadd\" data-act=\"termin-neu\" data-v=\"{d}\" title=\"Termin am {}\">{}</button></div>",
                    if d == heute { " today" } else { "" },
                    WT_KURZ[i as usize],
                    german_short(&d),
                    self.tag_eintraege(&d, false),
                    german(&d),
                    ic("plus")
                );
            }
            h += "</div>";
        }
        // kommende Termine
        let kommend = self.data.naechste_termine(&heute, 8);
        if !kommend.is_empty() {
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Nächste Termine</h2></div><div class=\"rows\">";
            for t in kommend {
                h += &termin_row(self, t);
            }
            h += "</div></div>";
        }
        h
    }

    /// Termin-Dialog (neu oder bearbeiten).
    pub fn termin_modal(&mut self, t: Termin) {
        let neu = t.id == 0;
        let mut auftraege = String::from("<option value=\"\">— kein Auftrag —</option>");
        let mut liste: Vec<_> = self.data.auftraege.iter().filter(|a| a.status.offen() || Some(a.id) == t.auftrag_id).collect();
        liste.sort_by_key(|a| a.titel().to_lowercase());
        for a in liste {
            let _ = write!(
                auftraege,
                "<option value=\"{}\"{}>{} ({})</option>",
                a.id,
                if Some(a.id) == t.auftrag_id { " selected" } else { "" },
                esc(&a.titel()),
                esc(&self.data.kunden_name(a.kunde_id))
            );
        }
        let mut kunden = String::from("<option value=\"\">— kein Kunde —</option>");
        let mut ks: Vec<_> = self.data.kunden.iter().collect();
        ks.sort_by_key(|k| k.name().to_lowercase());
        for k in ks {
            let _ = write!(kunden, "<option value=\"{}\"{}>{}</option>", k.id, if Some(k.id) == t.kunde_id { " selected" } else { "" }, esc(&k.name()));
        }
        let extra = if neu {
            String::new()
        } else {
            format!(
                "{}{}{}",
                btn("termin-del", "trash", "Löschen", "ghost danger-text", Some(t.id), ""),
                btn("termin-export", "calendar", "In Kalender-App", "ghost", Some(t.id), ""),
                if t.ort.trim().is_empty() { String::new() } else { btn("termin-route", "map", "Route", "ghost", Some(t.id), "") }
            )
        };
        let html = format!(
            "<h3>{}</h3><div class=\"form\"><label class=\"field wide\"><span>Titel</span><input type=\"text\" id=\"tm-titel\" value=\"{}\" placeholder=\"z. B. Besichtigung, Montage, Wartung\" autofocus></label><label class=\"field\"><span>Datum</span><input type=\"date\" id=\"tm-datum\" value=\"{}\"></label><label class=\"field\"><span>Bis (mehrtägig)</span><input type=\"date\" id=\"tm-bis-datum\" value=\"{}\"></label><label class=\"field\"><span>Beginn</span><input type=\"time\" id=\"tm-von\" value=\"{}\"></label><label class=\"field\"><span>Ende</span><input type=\"time\" id=\"tm-bis\" value=\"{}\"></label><label class=\"field wide\"><span>Ort / Adresse</span><input type=\"text\" id=\"tm-ort\" value=\"{}\"></label><label class=\"field\"><span>Auftrag</span><select id=\"tm-auftrag\">{auftraege}</select></label><label class=\"field\"><span>Kunde</span><select id=\"tm-kunde\">{kunden}</select></label><label class=\"field wide\"><span>Notiz</span><textarea id=\"tm-notiz\" rows=\"2\">{}</textarea></label><p class=\"hint wide\">Ohne Beginn gilt der Termin als ganztägig.</p></div><div class=\"modal-actions spread\">{extra}<span class=\"flex1\"></span>{}<button class=\"btn primary\" data-act=\"termin-save\" data-id=\"{}\" data-enter>{}<span>Speichern</span></button></div>",
            if neu { "Neuer Termin" } else { "Termin bearbeiten" },
            esc(&t.titel),
            esc(&t.datum),
            esc(&t.bis_datum),
            esc(&t.von),
            esc(&t.bis),
            esc(&t.ort),
            esc(&t.notiz),
            btn("modal-close", "", "Abbrechen", "ghost", None, ""),
            t.id,
            ic("check")
        );
        self.modal(&html);
        if let Some(m) = super::el("modal-card") {
            let _ = m.class_list().add_1("wide");
        }
    }

    /// Werte aus dem Termin-Dialog übernehmen (neu anlegen oder ändern).
    pub fn termin_save(&mut self, id: Option<Id>) {
        let titel = value_of("tm-titel").trim().to_string();
        let datum = value_of("tm-datum");
        if date::parse(&datum).is_none() {
            self.toast("Bitte ein Datum wählen");
            return;
        }
        let auftrag = value_of("tm-auftrag").parse::<Id>().ok();
        let kunde = value_of("tm-kunde").parse::<Id>().ok();
        let tid = match id.filter(|i| *i != 0 && self.data.termin(Some(*i)).is_some()) {
            Some(i) => i,
            None => self.data.neuer_termin(auftrag, kunde, &datum),
        };
        let bis_datum = value_of("tm-bis-datum");
        if let Some(t) = self.data.termin_mut(tid) {
            t.titel = if titel.is_empty() { "Termin".into() } else { titel };
            t.datum = datum.clone();
            t.bis_datum = if bis_datum > datum { bis_datum } else { String::new() };
            t.von = value_of("tm-von");
            t.bis = value_of("tm-bis");
            t.ort = value_of("tm-ort").trim().to_string();
            t.auftrag_id = auftrag;
            t.kunde_id = kunde.or(t.kunde_id.filter(|_| auftrag.is_some()));
            t.notiz = value_of("tm-notiz");
        }
        self.ui.kal_tag = datum;
        self.close_modal();
        self.changed();
        self.render();
        self.toast("Termin gespeichert");
    }

    /// Vorschlag für einen neuen Termin.
    pub fn termin_entwurf(&self, datum: &str, auftrag: Option<Id>, kunde: Option<Id>) -> Termin {
        let mut d = self.data.clone();
        let id = d.neuer_termin(auftrag, kunde, datum);
        let mut t = d.termin(Some(id)).cloned().unwrap_or_default();
        t.id = 0;
        t
    }

    pub fn termin_export(&self, id: Id) {
        let Some(t) = self.data.termin(Some(id)) else { return };
        let mut beschr = Vec::new();
        if let Some(k) = self.data.kunde(t.kunde_id) {
            beschr.push(format!("Kunde: {}{}", k.name(), if k.telefon.trim().is_empty() { String::new() } else { format!(" (Tel. {})", k.telefon.trim()) }));
        }
        if !t.notiz.trim().is_empty() {
            beschr.push(t.notiz.trim().to_string());
        }
        let datei = ics(&self.data, &[t], &today());
        super::storage::kalender_eintragen(t, &beschr.join("\n"), &datei);
    }

    pub fn kal_ics_alle(&self) {
        let liste: Vec<&Termin> = self.data.termine.iter().collect();
        if liste.is_empty() {
            self.toast("Noch keine Termine");
            return;
        }
        let datei = ics(&self.data, &liste, &today());
        super::storage::save_file(datei.as_bytes(), "text/calendar", "Zento-Termine.ics");
    }

    pub fn kal_nav(&mut self, v: &str) {
        let tag = self.kal_tag();
        let schritt: i64 = v.parse().unwrap_or(0);
        self.ui.kal_tag = if schritt == 0 {
            today()
        } else if self.ui.kal_ansicht == "monat" {
            let (y, m, _) = date::parse(&tag).unwrap_or((2026, 1, 1));
            let gesamt = y as i64 * 12 + (m as i64 - 1) + schritt;
            date::iso((gesamt / 12) as i32, (gesamt % 12 + 1) as u32, 1)
        } else {
            date::add_days(&tag, 7 * schritt)
        };
        self.render();
    }
}

/// Termin als Listenzeile (Übersicht, Auftrag, Kunde).
pub fn termin_row(a: &App, t: &Termin) -> String {
    let heute = today();
    let tag = if t.datum == heute {
        "Heute".to_string()
    } else if t.datum == date::add_days(&heute, 1) {
        "Morgen".to_string()
    } else {
        let wt = crate::engine::termine::wochentag(&t.datum);
        format!("{} {}", WT_KURZ[wt], german_short(&t.datum))
    };
    let kunde = a.data.kunde(t.kunde_id).map(|k| k.name()).unwrap_or_default();
    let unter = [t.zeit(), kunde, t.ort.trim().to_string()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
    format!(
        "<button class=\"row\" data-act=\"termin-open\" data-id=\"{}\"><div class=\"tdate{}\">{}</div><div><b>{}</b><span>{}</span></div><div class=\"row-r\">{}</div></button>",
        t.id,
        if t.datum == heute { " heute" } else { "" },
        esc(&tag),
        esc(&t.titel),
        esc(&unter),
        ic("chevron")
    )
}
