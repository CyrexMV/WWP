//! Seitenleiste: alle Bereiche als eigene Menüpunkte, Reihenfolge und Sichtbarkeit anpassbar
//! (Einstellungen → Darstellung: ziehen oder Pfeile, Schalter zum Ausblenden).

use super::views::{btn, ic};
use super::{esc, today, App, View};
use crate::engine::model::{AuftragStatus, DokArt};
use std::fmt::Write as _;

/// (Schlüssel, Symbol, Bezeichnung) – Standardreihenfolge
pub const NAV: &[(&str, &str, &str)] = &[
    ("start", "home", "Übersicht"),
    ("kunden", "users", "Kunden"),
    ("auftraege", "briefcase", "Aufträge"),
    ("kalender", "calendar", "Kalender"),
    ("angebote", "file", "Angebote"),
    ("ab", "check", "Auftragsbestätigungen"),
    ("rechnungen", "receipt", "Rechnungen"),
    ("mahnungen", "bell", "Mahnwesen"),
    ("finanzen", "wallet", "Finanzen"),
    ("katalog", "book", "Leistungskatalog"),
];

/// Ziel-Ansicht eines Menüpunkts.
pub fn ziel(key: &str) -> View {
    match key {
        "kunden" => View::Kunden(None),
        "auftraege" => View::Auftraege,
        "kalender" => View::Kalender,
        "angebote" => View::Dokumente(DokArt::Angebot),
        "ab" => View::Dokumente(DokArt::Auftragsbestaetigung),
        "rechnungen" => View::Dokumente(DokArt::Rechnung),
        "mahnungen" => View::Mahnwesen,
        "finanzen" => View::Finanzen,
        "katalog" => View::Katalog,
        "einstellungen" => View::Einstellungen,
        _ => View::Start,
    }
}

fn art_key(art: DokArt) -> &'static str {
    match art {
        DokArt::Angebot => "angebote",
        DokArt::Auftragsbestaetigung => "ab",
        DokArt::Lieferschein => "ab",
        DokArt::Rechnung => "rechnungen",
    }
}

impl App {
    /// Alle Menüpunkte in der gespeicherten Reihenfolge (neue Punkte hinten angehängt).
    pub fn nav_reihenfolge(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = Vec::new();
        for k in &self.data.nav_reihenfolge {
            if let Some(n) = NAV.iter().find(|n| n.0 == k.as_str()) {
                if !v.contains(&n.0) {
                    v.push(n.0);
                }
            }
        }
        for n in NAV {
            if !v.contains(&n.0) {
                v.push(n.0);
            }
        }
        v
    }

    fn nav_sichtbar(&self, key: &str) -> bool {
        key == "start" || !self.data.nav_aus.iter().any(|k| k == key)
    }

    /// Welcher Menüpunkt gehört zur aktuellen Ansicht?
    fn nav_aktiv(&self) -> &'static str {
        if self.view() == View::Start {
            return "start";
        }
        match self.stack.iter().rev().find(|v| !matches!(v, View::Start)).copied().unwrap_or(View::Start) {
            View::Kunden(_) => "kunden",
            View::Auftraege | View::Auftrag(_) => "auftraege",
            View::Kalender => "kalender",
            View::Dokumente(art) => art_key(art),
            View::Dokument(id) => self.data.dokument(Some(id)).map(|d| art_key(d.art)).unwrap_or("rechnungen"),
            View::Mahnwesen => "mahnungen",
            View::Finanzen | View::Buchung(_) => "finanzen",
            View::Katalog => "katalog",
            View::Einstellungen => "einstellungen",
            View::Start => "start",
        }
    }

    /// Hinweiszahl am Menüpunkt (Anzahl, Warnfarbe).
    fn nav_zahl(&self, key: &str) -> (usize, bool) {
        let heute = today();
        let d = &self.data;
        match key {
            "auftraege" => (d.auftraege.iter().filter(|a| a.status == AuftragStatus::Anfrage).count(), false),
            "kalender" => (d.termine_am(&heute).len(), false),
            "rechnungen" => (d.dokumente.iter().filter(|x| x.ueberfaellig(&heute)).count(), true),
            "mahnungen" => (d.dokumente.iter().filter(|x| x.offen() && x.zu_mahnen(&heute)).count(), true),
            _ => (0, false),
        }
    }

    pub fn nav_html(&self) -> String {
        let cur = self.nav_aktiv();
        let item = |key: &str, icon: &str, label: &str, (count, alert): (usize, bool)| {
            let b = if count > 0 { format!("<em class=\"{}\">{}</em>", if alert { "alert" } else { "" }, count) } else { String::new() };
            // kurze Bezeichnung in der Leiste, voller Name als Hinweis
            let kurz = if key == "ab" { "Bestätigungen" } else { label };
            format!(
                "<button class=\"nav-item{}\" data-act=\"nav\" data-v=\"{key}\" title=\"{}\">{}<span>{}</span>{b}</button>",
                if cur == key { " active" } else { "" },
                esc(label),
                ic(icon),
                esc(kurz)
            )
        };
        let firm = if self.data.firma.name.trim().is_empty() { "Zento".to_string() } else { self.data.firma.name.trim().to_string() };
        let logo = match &self.logo_url {
            Some(u) => format!("<img class=\"brand-logo\" src=\"{}\" alt=\"\">", esc(u)),
            None => format!("<div class=\"brand-mark\">{}</div>", ic("logo")),
        };
        let mut h = format!(
            "<div class=\"brand\">{logo}<span>{}</span></div><button class=\"nav-search\" data-act=\"suche\" title=\"Suchen (Strg+K)\">{}<span>Suchen …</span><kbd>Strg K</kbd></button><div class=\"nav-list\">",
            esc(&firm),
            ic("search")
        );
        for key in self.nav_reihenfolge() {
            if !self.nav_sichtbar(key) {
                continue;
            }
            let n = NAV.iter().find(|n| n.0 == key).unwrap();
            let mut it = item(n.0, n.1, n.2, self.nav_zahl(key));
            if key == "auftraege" && self.data.laufende_uhr().is_some() {
                it = it.replacen("</span>", "</span><i class=\"live\" title=\"Stoppuhr läuft\"></i>", 1);
            }
            h += &it;
        }
        h += "</div><div class=\"nav-space\"></div>";
        let _ = write!(
            h,
            "<div class=\"nav-foot\">{}<button class=\"nav-edit\" data-act=\"nav-anpassen\" title=\"Seitenleiste anpassen\">{}</button></div><p class=\"save-state\" id=\"save-state\">Gespeichert</p>",
            item("einstellungen", "settings", "Einstellungen", (0, false)),
            ic("grip")
        );
        h
    }

    /// Einstellungen → Darstellung: Reihenfolge (ziehen oder Pfeile) und Sichtbarkeit.
    pub fn nav_einstellungen_html(&self) -> String {
        let order = self.nav_reihenfolge();
        let n = order.len();
        let mut h = String::from("<div class=\"nav-sort\">");
        for (i, key) in order.iter().enumerate() {
            let e = NAV.iter().find(|x| x.0 == *key).unwrap();
            let sichtbar = self.nav_sichtbar(key);
            let schalter = if *key == "start" {
                "<span class=\"muted small\">immer</span>".to_string()
            } else {
                format!("<input type=\"checkbox\" class=\"tgl\" data-f=\"nav.{key}\"{} title=\"Anzeigen\">", if sichtbar { " checked" } else { "" })
            };
            let _ = write!(
                h,
                "<div class=\"nav-row{}\" data-drag-row=\"nav\" data-i=\"{i}\"><span class=\"nav-grip\" data-drag=\"{i}\" data-drag-list=\"nav\" title=\"Zum Verschieben ziehen\">{}</span><span class=\"tile t{}\">{}</span><b>{}</b><span class=\"flex1\"></span><button class=\"icon-btn sm\" data-act=\"nav-hoch\" data-v=\"{key}\" title=\"Nach oben\"{}>{}</button><button class=\"icon-btn sm\" data-act=\"nav-runter\" data-v=\"{key}\" title=\"Nach unten\"{}>{}</button>{schalter}</div>",
                if sichtbar { "" } else { " aus" },
                ic("grip"),
                i % 9,
                ic(e.1),
                esc(e.2),
                if i == 0 { " disabled" } else { "" },
                ic("up"),
                if i + 1 == n { " disabled" } else { "" },
                ic("down")
            );
        }
        let _ = write!(h, "</div><div class=\"pad-top\">{}</div>", btn("nav-reset", "undo", "Standard wiederherstellen", "ghost sm", None, ""));
        h
    }

    fn nav_setzen(&mut self, order: Vec<&str>) {
        self.data.nav_reihenfolge = order.into_iter().map(String::from).collect();
        self.changed();
        self.render();
    }

    /// Menüpunkt `von` an Position `nach` verschieben (Ziehen in den Einstellungen).
    pub fn nav_verschieben(&mut self, von: usize, nach: usize) {
        let mut o = self.nav_reihenfolge();
        if von >= o.len() {
            return;
        }
        let k = o.remove(von);
        o.insert(nach.min(o.len()), k);
        self.nav_setzen(o);
    }

    /// Knöpfe der Navigation (Reihenfolge, Zurücksetzen, Anpassen).
    pub fn nav_action(&mut self, act: &str, v: &str) -> bool {
        match act {
            "nav-hoch" | "nav-runter" => {
                let mut o = self.nav_reihenfolge();
                if let Some(i) = o.iter().position(|k| *k == v) {
                    let j = if act == "nav-hoch" { i.checked_sub(1) } else { (i + 1 < o.len()).then_some(i + 1) };
                    if let Some(j) = j {
                        o.swap(i, j);
                        self.nav_setzen(o);
                    }
                }
            }
            "nav-reset" => {
                self.data.nav_reihenfolge.clear();
                self.data.nav_aus.clear();
                self.changed();
                self.render();
                self.toast("Standard-Reihenfolge wiederhergestellt");
            }
            "nav-anpassen" => {
                self.ui.settings_tab = "darstellung".into();
                self.go_root(View::Einstellungen);
            }
            _ => return false,
        }
        true
    }

    /// Schalter „anzeigen“ eines Menüpunkts.
    pub fn nav_sichtbarkeit(&mut self, key: &str, an: bool) {
        self.data.nav_aus.retain(|k| k != key);
        if !an && key != "start" {
            self.data.nav_aus.push(key.to_string());
        }
        self.changed();
        self.render();
    }
}
