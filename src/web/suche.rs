//! Globale Suche & Schnellbefehle (Strg+K, „/“ oder Lupe in der Seitenleiste):
//! Kunden, Aufträge, Dokumente, Buchungen, Termine, Leistungen und Befehle in einem Feld.

use super::navigation::NAV;
use super::views::ic;
use super::{el, esc, query, set_html, App};
use crate::engine::date::german;
use crate::engine::model::{BuchArt, Id};
use crate::engine::money::fmt_eur;
use std::fmt::Write as _;

struct Treffer {
    icon: &'static str,
    farbe: u8,
    titel: String,
    text: String,
    rechts: String,
    act: String,
    id: Option<Id>,
    v: String,
    /// kleiner = besser
    rang: u8,
}

/// Befehle: (Bezeichnung, Suchwörter, Symbol, Aktion, Wert)
const BEFEHLE: &[(&str, &str, &str, &str, &str)] = &[
    ("Neue Rechnung", "rechnung erstellen anlegen", "receipt", "dok-neu", "rechnung"),
    ("Neue Auftragsbestätigung", "bestätigung ab auftrag erstellen", "check", "dok-neu", "ab"),
    ("Neuer Kunde", "kunde anlegen kontakt", "users", "kunde-neu", ""),
    ("Neuer Termin", "termin kalender eintragen", "calendar", "termin-neu", ""),
    ("Neue Ausgabe", "ausgabe beleg kosten buchen", "out", "buch-neu", "ausgabe"),
    ("Neue Einnahme", "einnahme buchen", "in", "buch-neu", "einnahme"),
    ("Backup speichern", "sichern datensicherung export", "database", "backup", ""),
    ("Dunkles Design", "dunkel dark modus nacht", "moon", "theme", "dark"),
    ("Helles Design", "hell light modus tag", "moon", "theme", "light"),
];

fn norm(s: &str) -> String {
    s.to_lowercase()
}

/// Alle Suchwörter enthalten? Rang 0 = Titel beginnt mit der Suche.
fn passt(q: &str, titel: &str, heu: &str) -> Option<u8> {
    let t = norm(titel);
    let alles = format!("{t} {}", norm(heu));
    if !q.split_whitespace().all(|w| alles.contains(w)) {
        return None;
    }
    Some(if t.starts_with(q) { 0 } else if t.contains(q) { 1 } else { 2 })
}

impl App {
    fn treffer(&self, q: &str) -> Vec<(&'static str, Vec<Treffer>)> {
        let d = &self.data;
        let q = norm(q.trim());
        let mut gruppen: Vec<(&'static str, Vec<Treffer>)> = Vec::new();
        let leer = q.is_empty();
        // Befehle
        let mut bef = Vec::new();
        for (titel, worte, icon, act, v) in BEFEHLE {
            if let Some(rang) = if leer { Some(1) } else { passt(&q, titel, worte) } {
                bef.push(Treffer { icon, farbe: 0, titel: titel.to_string(), text: String::new(), rechts: "<kbd>↵</kbd>".into(), act: act.to_string(), id: None, v: v.to_string(), rang });
            }
        }
        if !leer {
            if let Some(rang) = passt(&q, "Bestätigungen", "gehe zu auftragsbestätigung ab") {
                bef.push(Treffer { icon: "check", farbe: 7, titel: "Gehe zu Bestätigungen".into(), text: String::new(), rechts: String::new(), act: "dok-tab".into(), id: None, v: "ab".into(), rang });
            }
            if let Some(rang) = passt(&q, "Mahnwesen", "gehe zu mahnung mahnen") {
                bef.push(Treffer { icon: "bell", farbe: 7, titel: "Gehe zu Mahnwesen".into(), text: String::new(), rechts: String::new(), act: "nav".into(), id: None, v: "mahnungen".into(), rang });
            }
            for (key, icon, label) in NAV {
                if let Some(rang) = passt(&q, label, "gehe zu öffnen bereich") {
                    bef.push(Treffer { icon, farbe: 7, titel: format!("Gehe zu {label}"), text: String::new(), rechts: String::new(), act: "nav".into(), id: None, v: key.to_string(), rang });
                }
            }
            if let Some(rang) = passt(&q, "Einstellungen", "gehe zu firma logo bank") {
                bef.push(Treffer { icon: "settings", farbe: 7, titel: "Gehe zu Einstellungen".into(), text: String::new(), rechts: String::new(), act: "nav".into(), id: None, v: "einstellungen".into(), rang });
            }
        }
        if leer {
            bef.truncate(6);
        }
        if !leer {
            // Kunden
            let mut v = Vec::new();
            for k in &d.kunden {
                let heu = format!("{} {} {} {} {} {}", k.nr, k.ansprechpartner, k.ort, k.plz, k.email, k.telefon);
                if let Some(rang) = passt(&q, &k.name(), &heu) {
                    v.push(Treffer { icon: "users", farbe: 1, titel: k.name(), text: [k.nr.as_str(), k.ort.as_str(), k.telefon.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · "), rechts: String::new(), act: "kunde-open".into(), id: Some(k.id), v: String::new(), rang });
                }
            }
            gruppen.push(("Kunden", v));
            // Aufträge
            let mut v = Vec::new();
            for a in &d.auftraege {
                let kunde = d.kunden_name(a.kunde_id);
                if let Some(rang) = passt(&q, &a.titel(), &format!("{} {} {}", a.nr, kunde, a.beschreibung)) {
                    v.push(Treffer { icon: "briefcase", farbe: 4, titel: a.titel(), text: format!("{} · {}", a.nr, kunde), rechts: format!("<span class=\"badge st-{}\">{}</span>", a.status.key(), a.status.label()), act: "auftrag-open".into(), id: Some(a.id), v: String::new(), rang });
                }
            }
            gruppen.push(("Aufträge", v));
            // Dokumente
            let mut v = Vec::new();
            for x in &d.dokumente {
                let kunde = d.kunden_name(x.kunde_id);
                let titel = format!("{} {}", x.titel(), x.nummer);
                let betrag = fmt_eur(x.summen().brutto);
                if let Some(rang) = passt(&q, &titel, &format!("{} {} {} {}", kunde, x.betreff, x.empfaenger, betrag)) {
                    v.push(Treffer { icon: if x.art == crate::engine::model::DokArt::Rechnung { "receipt" } else { "file" }, farbe: 0, titel, text: format!("{} · {}", kunde, german(&x.datum)), rechts: format!("<b>{}</b>", esc(&betrag)), act: "dok-open".into(), id: Some(x.id), v: String::new(), rang });
                }
            }
            gruppen.push(("Dokumente", v));
            // Buchungen
            let mut v = Vec::new();
            for b in &d.buchungen {
                let titel = [b.text.as_str(), b.partner.as_str(), b.kategorie.as_str()].iter().find(|s| !s.trim().is_empty()).map(|s| s.to_string()).unwrap_or_default();
                let betrag = fmt_eur(b.brutto);
                if let Some(rang) = passt(&q, &titel, &format!("{} {} {} {}", b.partner, b.kategorie, b.text, betrag)) {
                    let ein = b.art == BuchArt::Einnahme;
                    v.push(Treffer { icon: if ein { "in" } else { "out" }, farbe: if ein { 1 } else { 2 }, titel, text: format!("{} · {}", german(&b.datum), b.kategorie), rechts: format!("<b class=\"{}\">{}{}</b>", if ein { "plus" } else { "neg" }, if ein { "+" } else { "−" }, esc(&betrag)), act: "buch-open".into(), id: Some(b.id), v: String::new(), rang });
                }
            }
            gruppen.push(("Buchungen", v));
            // Termine
            let mut v = Vec::new();
            for t in &d.termine {
                if let Some(rang) = passt(&q, &t.titel, &format!("{} {} {}", t.ort, t.notiz, d.kunden_name(t.kunde_id))) {
                    v.push(Treffer { icon: "calendar", farbe: 5, titel: t.titel.clone(), text: format!("{}{}", german(&t.datum), if t.von.is_empty() { String::new() } else { format!(" · {}", t.von) }), rechts: String::new(), act: "termin-open".into(), id: Some(t.id), v: String::new(), rang });
                }
            }
            gruppen.push(("Termine", v));
            // Leistungen
            let mut v = Vec::new();
            for l in &d.katalog {
                if let Some(rang) = passt(&q, &l.text, &l.kategorie) {
                    v.push(Treffer { icon: "book", farbe: 8, titel: l.text.lines().next().unwrap_or("").to_string(), text: format!("{} / {}", fmt_eur(l.preis), l.einheit), rechts: String::new(), act: "nav".into(), id: None, v: "katalog".into(), rang });
                }
            }
            gruppen.push(("Leistungen", v));
        } else {
            // zuletzt angelegte Dokumente
            let mut docs: Vec<_> = d.dokumente.iter().collect();
            docs.sort_by_key(|x| std::cmp::Reverse(x.id));
            let v = docs
                .into_iter()
                .take(4)
                .map(|x| Treffer { icon: "file", farbe: 0, titel: format!("{} {}", x.titel(), x.nummer), text: d.kunden_name(x.kunde_id), rechts: format!("<b>{}</b>", esc(&fmt_eur(x.summen().brutto))), act: "dok-open".into(), id: Some(x.id), v: String::new(), rang: 0 })
                .collect();
            gruppen.push(("Zuletzt", v));
        }
        bef.sort_by_key(|t| t.rang);
        if leer {
            gruppen.insert(0, ("Schnellbefehle", bef));
        } else {
            gruppen.push(("Befehle", bef));
        }
        for (_, v) in gruppen.iter_mut() {
            v.sort_by_key(|t| t.rang);
            v.truncate(5);
        }
        gruppen.retain(|g| !g.1.is_empty());
        gruppen
    }

    fn suche_liste(&self, q: &str) -> String {
        let gruppen = self.treffer(q);
        if gruppen.is_empty() {
            return format!("<div class=\"sr-leer\">{}<b>Nichts gefunden</b><span>Versuche es mit einem Namen, einer Nummer oder einem Betrag.</span></div>", ic("search"));
        }
        let mut h = String::new();
        let mut i = 0;
        for (name, v) in gruppen {
            let _ = write!(h, "<div class=\"sr-h\">{name}</div>");
            for t in v {
                let _ = write!(
                    h,
                    "<button class=\"sr{}\" data-act=\"sr\" data-i=\"{i}\"{} data-v=\"{}|{}\"><span class=\"tile t{}\">{}</span><div><b>{}</b>{}</div>{}</button>",
                    if i == 0 { " sel" } else { "" },
                    t.id.map(|x| format!(" data-id=\"{x}\"")).unwrap_or_default(),
                    esc(&t.act),
                    esc(&t.v),
                    t.farbe,
                    ic(t.icon),
                    esc(&t.titel),
                    if t.text.is_empty() { String::new() } else { format!("<span>{}</span>", esc(&t.text)) },
                    t.rechts
                );
                i += 1;
            }
        }
        h
    }

    pub fn suche_oeffnen(&mut self) {
        let html = format!(
            "<div class=\"palette\"><div class=\"pal-in\">{}<input type=\"search\" id=\"suche-q\" data-search=\"suche\" placeholder=\"Suchen oder Befehl eingeben …\" autocomplete=\"off\" autofocus><kbd>Esc</kbd></div><div class=\"pal-list\" id=\"suche-list\">{}</div><div class=\"pal-foot\"><span><kbd>↑</kbd><kbd>↓</kbd> auswählen</span><span><kbd>↵</kbd> öffnen</span><span><kbd>Strg K</kbd> Suche</span></div></div>",
            ic("search"),
            self.suche_liste("")
        );
        self.modal(&html);
        if let Some(m) = el("modal-card") {
            let _ = m.class_list().add_1("pal-card");
        }
    }

    pub fn suche_tippen(&mut self, q: &str) {
        set_html("suche-list", &self.suche_liste(q));
    }

    /// Pfeiltasten/Enter in der Suche.
    pub fn suche_taste(&mut self, taste: &str) {
        let Ok(list) = super::dom().query_selector_all("#suche-list .sr") else { return };
        let n = list.length();
        if n == 0 {
            return;
        }
        let akt = query("#suche-list .sr.sel").and_then(|e| e.get_attribute("data-i")).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let neu = match taste {
            "ArrowDown" => (akt + 1) % n,
            "ArrowUp" => (akt + n - 1) % n,
            _ => {
                // direkt ausführen (ein simulierter Klick würde hier nicht ankommen)
                if let Some(e) = query("#suche-list .sr.sel") {
                    let id = e.get_attribute("data-id").and_then(|v| v.parse::<Id>().ok());
                    let v = e.get_attribute("data-v").unwrap_or_default();
                    self.suche_action("sr", id, &v);
                }
                return;
            }
        };
        for i in 0..n {
            if let Some(e) = list.item(i).and_then(|x| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(x).ok()) {
                let _ = if i == neu { e.class_list().add_1("sel") } else { e.class_list().remove_1("sel") };
                if i == neu {
                    let opt = web_sys::ScrollIntoViewOptions::new();
                    opt.set_block(web_sys::ScrollLogicalPosition::Nearest);
                    e.scroll_into_view_with_scroll_into_view_options(&opt);
                }
            }
        }
    }

    pub fn suche_action(&mut self, act: &str, id: Option<Id>, v: &str) -> bool {
        match act {
            "suche" => self.suche_oeffnen(),
            "sr" => {
                let (a, w) = v.split_once('|').unwrap_or((v, ""));
                let (a, w) = (a.to_string(), w.to_string());
                if let Some(m) = el("modal-card") {
                    let _ = m.class_list().remove_1("pal-card");
                }
                self.close_modal();
                self.action(&a, id, &w);
            }
            _ => return false,
        }
        true
    }
}
