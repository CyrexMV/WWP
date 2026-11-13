//! HTML-Erzeugung für alle Ansichten.

use super::{esc, set_html, today, App, View};
use crate::engine::date::{self, german, MONTHS, MONTHS_SHORT};
use crate::engine::model::*;
use crate::engine::money::{fmt_eur, fmt_num, fmt_qty, Cents};
use crate::engine::pdf::kurz;
use crate::engine::report;
use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// Bausteine
// ---------------------------------------------------------------------------

pub fn ic(name: &str) -> String {
    format!("<svg class=\"ic\" aria-hidden=\"true\"><use href=\"#i-{name}\"/></svg>")
}

fn dis(d: bool) -> &'static str {
    if d {
        " disabled"
    } else {
        ""
    }
}

pub(super) fn text(label: &str, key: &str, value: &str, extra: &str) -> String {
    format!(
        "<label class=\"field\"><span>{}</span><input type=\"text\" data-f=\"{}\" value=\"{}\" {}></label>",
        esc(label),
        key,
        esc(value),
        extra
    )
}

pub(super) fn area(label: &str, key: &str, value: &str, rows: u32, extra: &str) -> String {
    format!(
        "<label class=\"field wide\"><span>{}</span><textarea data-f=\"{}\" rows=\"{}\" {}>{}</textarea></label>",
        esc(label),
        key,
        rows,
        extra,
        esc(value)
    )
}

fn date_f(label: &str, key: &str, value: &str, extra: &str) -> String {
    format!(
        "<label class=\"field\"><span>{}</span><input type=\"date\" data-f=\"{}\" value=\"{}\" {}></label>",
        esc(label),
        key,
        esc(value),
        extra
    )
}

pub(super) fn money_f(label: &str, key: &str, c: Cents, extra: &str) -> String {
    format!(
        "<label class=\"field\"><span>{}</span><div class=\"euro\"><input type=\"text\" inputmode=\"decimal\" data-f=\"{}\" value=\"{}\" {}><i>€</i></div></label>",
        esc(label),
        key,
        if c == 0 { String::new() } else { fmt_num(c) },
        extra
    )
}

pub(super) fn select_f(label: &str, key: &str, opts: &[(String, String)], sel: &str, extra: &str) -> String {
    let mut o = String::new();
    for (v, l) in opts {
        let _ = write!(o, "<option value=\"{}\"{}>{}</option>", esc(v), if v == sel { " selected" } else { "" }, esc(l));
    }
    format!("<label class=\"field\"><span>{}</span><select data-f=\"{}\" {}>{}</select></label>", esc(label), key, extra, o)
}

pub(super) fn ust_opts() -> Vec<(String, String)> {
    [19u8, 7, 0].iter().map(|r| (r.to_string(), format!("{r} %"))).collect()
}

pub fn badge(key: &str, label: &str) -> String {
    format!("<span class=\"badge st-{key}\">{}</span>", esc(label))
}

pub(super) fn btn(act: &str, icon: &str, label: &str, cls: &str, id: Option<Id>, v: &str) -> String {
    let id = id.map(|i| format!(" data-id=\"{i}\"")).unwrap_or_default();
    let v = if v.is_empty() { String::new() } else { format!(" data-v=\"{}\"", esc(v)) };
    let icon = if icon.is_empty() { String::new() } else { ic(icon) };
    format!("<button class=\"btn {cls}\" data-act=\"{act}\"{id}{v}>{icon}<span>{}</span></button>", esc(label))
}

pub(super) fn empty(icon: &str, title: &str, text: &str, action: &str) -> String {
    format!("<div class=\"empty\">{}<h3>{}</h3><p>{}</p>{}</div>", ic(icon), esc(title), esc(text), action)
}

pub(super) fn head(title: &str, sub: &str, actions: &str, back: bool) -> String {
    let b = if back { format!("<button class=\"icon-btn\" data-act=\"back\" title=\"Zurück\">{}</button>", ic("back")) } else { String::new() };
    let sub = if sub.is_empty() { String::new() } else { format!("<p class=\"sub\">{sub}</p>") };
    format!("<header class=\"page-head\">{b}<div class=\"ph-title\"><h1>{}</h1>{sub}</div><div class=\"ph-actions\">{actions}</div></header>", esc(title))
}

fn kpi(label: &str, value: &str, note: &str, cls: &str, act: &str) -> String {
    let a = if act.is_empty() { String::new() } else { format!(" data-act=\"{act}\"") };
    format!(
        "<div class=\"kpi {cls}\"{a}><span class=\"kpi-l\">{}</span><b class=\"kpi-v\">{}</b><span class=\"kpi-n\">{}</span></div>",
        esc(label),
        esc(value),
        note
    )
}

/// Balkendiagramm Einnahmen/Ausgaben je Monat.
fn chart(a: &report::Auswertung) -> String {
    let max = a.monate.iter().map(|m| m.ein.netto.max(m.aus.netto)).max().unwrap_or(0).max(1) as f32;
    let (w, h, top, bottom) = (720.0f32, 220.0f32, 14.0f32, 26.0f32);
    let gw = w / 12.0;
    let bw = (gw - 14.0) / 2.0;
    let mut s = format!("<svg class=\"chart\" viewBox=\"0 0 {w} {h}\" preserveAspectRatio=\"none\" role=\"img\" aria-label=\"Einnahmen und Ausgaben je Monat\">");
    for k in 1..=3 {
        let y = top + (h - top - bottom) * (1.0 - k as f32 / 3.0);
        let _ = write!(s, "<line class=\"grid\" x1=\"0\" x2=\"{w}\" y1=\"{y}\" y2=\"{y}\"/>");
    }
    for (i, m) in a.monate.iter().enumerate() {
        let x = i as f32 * gw + 7.0;
        for (j, (v, cls)) in [(m.ein.netto, "bar-in"), (m.aus.netto, "bar-out")].into_iter().enumerate() {
            let bh = (v.max(0) as f32 / max) * (h - top - bottom);
            let _ = write!(
                s,
                "<rect class=\"{cls}\" x=\"{}\" y=\"{}\" width=\"{bw}\" height=\"{}\" rx=\"3\"><title>{} {}: {}</title></rect>",
                x + j as f32 * bw,
                h - bottom - bh,
                bh.max(0.0),
                MONTHS[i],
                if j == 0 { "Einnahmen" } else { "Ausgaben" },
                fmt_eur(v)
            );
        }
        let _ = write!(s, "<text x=\"{}\" y=\"{}\" class=\"axis\">{}</text>", x + bw, h - 8.0, MONTHS_SHORT[i]);
    }
    let _ = write!(s, "<text x=\"4\" y=\"{}\" class=\"axis left\">{}</text>", top + 4.0, kurz(max as Cents));
    s.push_str("</svg>");
    s
}

// ---------------------------------------------------------------------------
// Rahmen
// ---------------------------------------------------------------------------

impl App {
    pub fn render(&mut self) {
        set_html("nav", &self.nav_html());
        let html = match self.view() {
            View::Start => self.start_html(),
            View::Kunden(sel) => self.kunden_html(sel),
            View::Auftraege => self.board_html(),
            View::Auftrag(id) => self.auftrag_html(id),
            View::Dokumente(art) => self.dokumente_html(art),
            View::Dokument(id) => self.dokument_html(id),
            View::Finanzen => self.finanzen_html(),
            View::Buchung(id) => self.buchung_html(id),
            View::Einstellungen => self.settings_html(),
        };
        set_html("main", &html);
    }

    pub fn nav_html(&self) -> String {
        let heute = today();
        let overdue = self.data.dokumente.iter().filter(|d| d.ueberfaellig(&heute)).count();
        let anfragen = self.data.auftraege.iter().filter(|a| a.status == AuftragStatus::Anfrage).count();
        let cur = match self.stack.iter().rev().find(|v| !matches!(v, View::Start)).copied().unwrap_or(View::Start) {
            _ if self.view() == View::Start => "start",
            View::Kunden(_) => "kunden",
            View::Auftraege | View::Auftrag(_) => "auftraege",
            View::Dokumente(DokArt::Angebot) => "angebote",
            View::Dokumente(DokArt::Rechnung) => "rechnungen",
            View::Dokument(id) => match self.data.dokument(Some(id)).map(|d| d.art) {
                Some(DokArt::Angebot) => "angebote",
                _ => "rechnungen",
            },
            View::Finanzen | View::Buchung(_) => "finanzen",
            View::Einstellungen => "einstellungen",
            View::Start => "start",
        };
        let item = |key: &str, icon: &str, label: &str, count: usize, alert: bool| {
            let b = if count > 0 { format!("<em class=\"{}\">{}</em>", if alert { "alert" } else { "" }, count) } else { String::new() };
            format!(
                "<button class=\"nav-item{}\" data-act=\"nav\" data-v=\"{key}\">{}<span>{}</span>{b}</button>",
                if cur == key { " active" } else { "" },
                ic(icon),
                esc(label)
            )
        };
        let firm = if self.data.firma.name.trim().is_empty() { "WWP Büro".to_string() } else { self.data.firma.name.trim().to_string() };
        let logo = match &self.logo_url {
            Some(u) => format!("<img class=\"brand-logo\" src=\"{}\" alt=\"\">", esc(u)),
            None => format!("<div class=\"brand-mark\">{}</div>", ic("logo")),
        };
        format!(
            "<div class=\"brand\">{logo}<span>{}</span></div>{}{}{}{}{}{}<div class=\"nav-space\"></div>{}<p class=\"save-state\" id=\"save-state\">Gespeichert</p>",
            esc(&firm),
            item("start", "home", "Übersicht", 0, false),
            item("kunden", "users", "Kunden", 0, false),
            item("auftraege", "briefcase", "Aufträge", anfragen, false),
            item("angebote", "file", "Angebote", 0, false),
            item("rechnungen", "receipt", "Rechnungen", overdue, true),
            item("finanzen", "wallet", "Finanzen", 0, false),
            item("einstellungen", "settings", "Einstellungen", 0, false),
        )
    }

    // -----------------------------------------------------------------------
    // Übersicht
    // -----------------------------------------------------------------------

    fn start_html(&self) -> String {
        let heute = today();
        let d = &self.data;
        let jahr = date::year(&heute).unwrap_or(2026);
        let a = report::auswertung(d, jahr);
        let offen: Vec<&Dokument> = d.dokumente.iter().filter(|x| x.offen()).collect();
        let offen_sum: Cents = offen.iter().map(|x| x.rest()).sum();
        let ueber: Vec<&&Dokument> = offen.iter().filter(|x| x.ueberfaellig(&heute)).collect();
        let ueber_sum: Cents = ueber.iter().map(|x| x.rest()).sum();
        let zu_mahnen = offen.iter().filter(|x| x.zu_mahnen(&heute)).count();
        let angebote: Vec<&Dokument> = d.dokumente.iter().filter(|x| x.art == DokArt::Angebot && x.status == DokStatus::Versendet).collect();
        let angebote_sum: Cents = angebote.iter().map(|x| x.summen().netto).sum();
        let aktiv = d.auftraege.iter().filter(|x| matches!(x.status, AuftragStatus::Beauftragt | AuftragStatus::InArbeit)).count();
        let anfragen = d.auftraege.iter().filter(|x| x.status == AuftragStatus::Anfrage).count();

        let mut h = String::new();
        let actions = [
            btn("auftrag-neu", "plus", "Anfrage", "ghost", None, ""),
            btn("dok-neu", "plus", "Angebot", "ghost", None, "angebot"),
            btn("dok-neu", "plus", "Rechnung", "primary", None, "rechnung"),
        ]
        .concat();
        let wochentag = ["Sonntag", "Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag"]
            [(date::to_days(&heute).unwrap_or(0) + 4).rem_euclid(7) as usize];
        h += &head("Übersicht", &format!("{wochentag}, {}", german(&heute)), &actions, false);

        if self.loaded && d.firma.name.trim().is_empty() {
            h += &format!(
                "<div class=\"banner\">{}<div><b>Willkommen!</b> Trage zuerst deine Firmendaten, Bankverbindung und dein Logo ein – sie erscheinen auf allen Angeboten und Rechnungen.</div>{}{}</div>",
                ic("info"),
                btn("nav", "settings", "Einrichten", "primary", None, "einstellungen"),
                if d.kunden.is_empty() && d.dokumente.is_empty() { btn("demo", "", "Beispieldaten", "ghost", None, "") } else { String::new() }
            );
        }

        h += "<section class=\"kpis\">";
        h += &kpi(
            "Offene Rechnungen",
            &fmt_eur(offen_sum),
            &if ueber.is_empty() {
                format!("{} offen", offen.len())
            } else {
                format!(
                    "{} offen · <b class=\"neg\">{} überfällig ({})</b>{}",
                    offen.len(),
                    ueber.len(),
                    esc(&fmt_eur(ueber_sum)),
                    if zu_mahnen > 0 { format!("<br>{zu_mahnen} zu mahnen") } else { String::new() }
                )
            },
            if ueber.is_empty() { "" } else { "warn" },
            if zu_mahnen > 0 { "kpi-mahnen" } else { "kpi-offen" },
        );
        h += &kpi(&format!("Einnahmen {jahr}"), &fmt_eur(a.ein.netto), "netto, bezahlt", "", "nav-fin");
        h += &kpi(&format!("Ausgaben {jahr}"), &fmt_eur(a.aus.netto), "netto", "", "nav-fin");
        h += &kpi(&format!("Gewinn {jahr}"), &fmt_eur(a.gewinn()), "Einnahmen − Ausgaben", if a.gewinn() < 0 { "warn" } else { "good" }, "nav-fin");
        h += &kpi("Offene Angebote", &fmt_eur(angebote_sum), &format!("{} versendet, netto", angebote.len()), "", "kpi-angebote");
        h += &kpi("Aufträge", &format!("{aktiv} aktiv"), &format!("{anfragen} neue Anfrage(n)"), "", "kpi-auftraege");
        h += "</section>";

        h += "<section class=\"grid2\">";
        // Diagramm
        h += &format!(
            "<div class=\"card\"><div class=\"card-head\"><h2>Einnahmen &amp; Ausgaben {jahr}</h2><div class=\"legend\"><i class=\"in\"></i>Einnahmen <i class=\"out\"></i>Ausgaben</div></div>{}</div>",
            chart(&a)
        );
        // offene Rechnungen
        let mut list = offen.clone();
        list.sort_by(|x, y| x.faellig.cmp(&y.faellig));
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Offene Rechnungen</h2></div>";
        if list.is_empty() {
            h += "<p class=\"muted pad\">Keine offenen Rechnungen. 🎉</p>";
        } else {
            h += "<div class=\"rows\">";
            for r in list.iter().take(8) {
                let tage = date::diff_days(&heute, &r.faellig).unwrap_or(0);
                let due = if tage < 0 {
                    format!("<span class=\"neg\">{} Tage überfällig</span>", -tage)
                } else if tage == 0 {
                    "heute fällig".to_string()
                } else {
                    format!("fällig in {tage} Tagen")
                };
                let stufe = r.letzte_mahnung().map(|m| format!(" · {}", m.label())).unwrap_or_default();
                let mahnen = if r.zu_mahnen(&heute) { badge("ueberfaellig", "mahnen") } else { String::new() };
                h += &format!(
                    "<button class=\"row\" data-act=\"dok-open\" data-id=\"{}\"><div><b>{}</b><span>{} · {}{}</span></div><div class=\"row-r\">{}<strong>{}</strong></div></button>",
                    r.id,
                    esc(&d.kunden_name(r.kunde_id)),
                    esc(&r.nummer),
                    due,
                    stufe,
                    mahnen,
                    esc(&fmt_eur(r.rest()))
                );
            }
            h += "</div>";
        }
        h += "</div>";
        // aktuelle Aufträge
        let mut auf: Vec<&Auftrag> = d.auftraege.iter().filter(|x| x.status.offen()).collect();
        auf.sort_by(|x, y| {
            let tx = if x.termin.is_empty() { "9999" } else { &x.termin };
            let ty = if y.termin.is_empty() { "9999" } else { &y.termin };
            tx.cmp(ty)
        });
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Aktuelle Aufträge &amp; Anfragen</h2></div>";
        if auf.is_empty() {
            h += &format!("<p class=\"muted pad\">Noch keine Aufträge.</p><div class=\"pad\">{}</div>", btn("auftrag-neu", "plus", "Anfrage erfassen", "ghost", None, ""));
        } else {
            h += "<div class=\"rows\">";
            for x in auf.iter().take(8) {
                let termin = if x.termin.is_empty() {
                    String::new()
                } else if x.termin < heute {
                    format!(" · <span class=\"neg\">Termin {}</span>", german(&x.termin))
                } else {
                    format!(" · Termin {}", german(&x.termin))
                };
                h += &format!(
                    "<button class=\"row\" data-act=\"auftrag-open\" data-id=\"{}\"><div><b>{}</b><span>{}{}</span></div>{}</button>",
                    x.id,
                    esc(&x.titel()),
                    esc(&d.kunden_name(x.kunde_id)),
                    termin,
                    badge(x.status.key(), x.status.label())
                );
            }
            h += "</div>";
        }
        h += "</div>";
        // letzte Buchungen
        let mut bu: Vec<&Buchung> = d.buchungen.iter().collect();
        bu.sort_by(|x, y| y.datum.cmp(&x.datum).then(y.id.cmp(&x.id)));
        h += &format!(
            "<div class=\"card\"><div class=\"card-head\"><h2>Letzte Buchungen</h2><div>{}{}</div></div>",
            btn("buch-neu", "plus", "Einnahme", "ghost sm", None, "einnahme"),
            btn("buch-neu", "plus", "Ausgabe", "ghost sm", None, "ausgabe")
        );
        if bu.is_empty() {
            h += "<p class=\"muted pad\">Noch keine Einnahmen oder Ausgaben erfasst.</p>";
        } else {
            h += "<div class=\"rows\">";
            for b in bu.iter().take(6) {
                h += &buch_row(b, d);
            }
            h += "</div>";
        }
        h += "</div></section>";
        h
    }

    // -----------------------------------------------------------------------
    // Kunden
    // -----------------------------------------------------------------------

    pub fn kunden_list_html(&self, sel: Option<Id>) -> String {
        let q = self.ui.kunden_q.trim();
        let mut list: Vec<&Kunde> = self.data.kunden.iter().filter(|k| q.is_empty() || k.matches(q)).collect();
        list.sort_by_key(|k| k.name().to_lowercase());
        if list.is_empty() {
            return if q.is_empty() {
                "<p class=\"muted pad\">Noch keine Kunden.</p>".into()
            } else {
                "<p class=\"muted pad\">Nichts gefunden.</p>".into()
            };
        }
        let mut h = String::new();
        for k in list {
            let ort = [k.plz.trim(), k.ort.trim()].join(" ").trim().to_string();
            let _ = write!(
                h,
                "<button class=\"li{}\" data-act=\"kunde-sel\" data-id=\"{}\"><div class=\"avatar\">{}</div><div><b>{}</b><span>{}{}</span></div></button>",
                if Some(k.id) == sel { " active" } else { "" },
                k.id,
                esc(&initials(&k.name())),
                esc(&k.name()),
                esc(&k.nr),
                if ort.is_empty() { String::new() } else { format!(" · {}", esc(&ort)) }
            );
        }
        h
    }

    fn kunden_html(&self, sel: Option<Id>) -> String {
        let mut h = String::new();
        h += &head("Kunden", &format!("{} Kunden", self.data.kunden.len()), &btn("kunde-neu", "plus", "Neuer Kunde", "primary", None, ""), false);
        let detail = match self.data.kunde(sel) {
            Some(k) => self.kunde_detail(k),
            None => empty("users", "Kein Kunde ausgewählt", "Wähle links einen Kunden aus oder lege einen neuen an.", &btn("kunde-neu", "plus", "Neuer Kunde", "primary", None, "")),
        };
        let _ = write!(
            h,
            "<div class=\"split{}\"><aside class=\"list-pane\"><div class=\"search\">{}<input type=\"search\" placeholder=\"Kunden suchen …\" data-search=\"kunden\" value=\"{}\"></div><div id=\"list\" class=\"list\">{}</div></aside><section class=\"detail-pane\">{}</section></div>",
            if sel.is_some() { " has-sel" } else { "" },
            ic("search"),
            esc(&self.ui.kunden_q),
            self.kunden_list_html(sel),
            detail
        );
        h
    }

    fn kunde_detail(&self, k: &Kunde) -> String {
        let d = &self.data;
        let mut h = String::new();
        let mut contact = String::new();
        if !k.email.trim().is_empty() {
            let _ = write!(contact, "<a class=\"btn ghost sm\" href=\"mailto:{}\">{}<span>E-Mail</span></a>", esc(k.email.trim()), ic("mail"));
        }
        if !k.telefon.trim().is_empty() {
            let tel: String = k.telefon.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
            let _ = write!(contact, "<a class=\"btn ghost sm\" href=\"tel:{}\">{}<span>Anrufen</span></a>", esc(&tel), ic("phone"));
        }
        let _ = write!(
            h,
            "<div class=\"detail-head\"><button class=\"icon-btn only-narrow\" data-act=\"kunde-sel\" title=\"Zurück zur Liste\">{}</button><div class=\"avatar lg\">{}</div><div class=\"dh-title\"><h2>{}</h2><span class=\"muted\">{} · Kunde seit {}</span></div><div class=\"dh-actions\">{}</div></div>",
            ic("back"),
            esc(&initials(&k.name())),
            esc(&k.name()),
            esc(&k.nr),
            german(&k.erstellt),
            contact
        );
        let _ = write!(
            h,
            "<div class=\"quick\">{}{}{}{}</div>",
            btn("auftrag-neu", "plus", "Anfrage", "ghost", Some(k.id), ""),
            btn("dok-neu-kunde", "plus", "Angebot", "ghost", Some(k.id), "angebot"),
            btn("dok-neu-kunde", "plus", "Rechnung", "ghost", Some(k.id), "rechnung"),
            btn("kunde-del", "trash", "Löschen", "ghost danger-text", Some(k.id), "")
        );
        let anreden: Vec<(String, String)> = ["", "Frau", "Herr"].iter().map(|s| (s.to_string(), if s.is_empty() { "—".into() } else { s.to_string() })).collect();
        h += "<div class=\"card\"><div class=\"form\">";
        h += &text("Firma", "k.firma", &k.firma, "placeholder=\"leer bei Privatkunden\"");
        h += &select_f("Anrede", "k.anrede", &anreden, &k.anrede, "");
        h += &text("Ansprechpartner", "k.ansprechpartner", &k.ansprechpartner, "autocomplete=\"off\"");
        h += &text("Straße & Hausnr.", "k.strasse", &k.strasse, "");
        h += &text("PLZ", "k.plz", &k.plz, "inputmode=\"numeric\"");
        h += &text("Ort", "k.ort", &k.ort, "");
        h += &text("Land", "k.land", &k.land, "placeholder=\"Deutschland\"");
        h += &text("E-Mail", "k.email", &k.email, "inputmode=\"email\" autocomplete=\"off\"");
        h += &text("Telefon", "k.telefon", &k.telefon, "inputmode=\"tel\"");
        h += &text("USt-IdNr.", "k.ust_id", &k.ust_id, "");
        h += &area("Notizen", "k.notizen", &k.notizen, 3, "");
        h += "</div></div>";

        let docs = d.dokumente_von(k.id);
        let offen: Cents = docs.iter().filter(|x| x.offen()).map(|x| x.summen().brutto).sum();
        let _ = write!(
            h,
            "<div class=\"kpis small\">{}{}{}</div>",
            kpi("Umsatz (bezahlt)", &fmt_eur(d.umsatz_kunde(k.id)), "brutto", "", ""),
            kpi("Offen", &fmt_eur(offen), "brutto", if offen > 0 { "warn" } else { "" }, ""),
            kpi("Aufträge", &d.auftraege_von(k.id).len().to_string(), "gesamt", "", "")
        );

        let auf = d.auftraege_von(k.id);
        if !auf.is_empty() {
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Aufträge &amp; Anfragen</h2></div><div class=\"rows\">";
            for a in auf {
                let _ = write!(
                    h,
                    "<button class=\"row\" data-act=\"auftrag-open\" data-id=\"{}\"><div><b>{}</b><span>{} · {}</span></div>{}</button>",
                    a.id,
                    esc(&a.titel()),
                    esc(&a.nr),
                    german(&a.eingang),
                    badge(a.status.key(), a.status.label())
                );
            }
            h += "</div></div>";
        }
        if !docs.is_empty() {
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Angebote &amp; Rechnungen</h2></div><div class=\"rows\">";
            let heute = today();
            for x in docs {
                h += &doc_row(x, d, &heute, false);
            }
            h += "</div></div>";
        }
        h
    }

    // -----------------------------------------------------------------------
    // Aufträge
    // -----------------------------------------------------------------------

    fn board_html(&self) -> String {
        let heute = today();
        let d = &self.data;
        let mut h = String::new();
        let toggle = format!(
            "<label class=\"switch\"><input type=\"checkbox\" data-f=\"ui.abgelehnte\"{}><span>Abgelehnte</span></label>",
            if self.ui.abgelehnte { " checked" } else { "" }
        );
        h += &head(
            "Aufträge",
            "Von der Anfrage bis zur Abrechnung",
            &format!("{toggle}{}", btn("auftrag-neu", "plus", "Neue Anfrage", "primary", None, "")),
            false,
        );
        if d.auftraege.is_empty() {
            h += &empty(
                "briefcase",
                "Noch keine Aufträge",
                "Erfasse eine Anfrage – daraus werden Angebot, Auftrag und Rechnung.",
                &btn("auftrag-neu", "plus", "Erste Anfrage erfassen", "primary", None, ""),
            );
            return h;
        }
        h += "<div class=\"board\">";
        for st in AuftragStatus::ALLE {
            if st == AuftragStatus::Abgelehnt && !self.ui.abgelehnte {
                continue;
            }
            let mut list: Vec<&Auftrag> = d.auftraege.iter().filter(|a| a.status == st).collect();
            list.sort_by(|a, b| {
                let ta = if a.termin.is_empty() { "9999" } else { &a.termin };
                let tb = if b.termin.is_empty() { "9999" } else { &b.termin };
                ta.cmp(tb).then(b.id.cmp(&a.id))
            });
            let sum: Cents = list.iter().map(|a| a.wert).sum();
            let _ = write!(
                h,
                "<div class=\"col st-col-{}\"><div class=\"col-head\"><b>{}</b><em>{}</em>{}</div><div class=\"col-body\">",
                st.key(),
                st.label(),
                list.len(),
                if sum > 0 { format!("<span>{}</span>", kurz(sum)) } else { String::new() }
            );
            if st == AuftragStatus::Anfrage {
                let _ = write!(h, "<button class=\"add-card\" data-act=\"auftrag-neu\">{}<span>Anfrage</span></button>", ic("plus"));
            }
            for a in list {
                let termin = if a.termin.is_empty() {
                    String::new()
                } else {
                    let late = a.termin < heute && a.status.offen() && a.status != AuftragStatus::Erledigt;
                    format!("<span class=\"{}\">{} {}</span>", if late { "neg" } else { "" }, ic("calendar"), german(&a.termin))
                };
                let next = match a.status.next() {
                    Some(n) => format!(
                        "<button class=\"next\" data-act=\"auftrag-next\" data-id=\"{}\" title=\"Weiter zu: {}\">{}{}</button>",
                        a.id,
                        n.label(),
                        esc(n.label()),
                        ic("arrow")
                    ),
                    None => String::new(),
                };
                let _ = write!(
                    h,
                    "<div class=\"ocard\" data-act=\"auftrag-open\" data-id=\"{}\"><b>{}</b><span class=\"muted\">{}</span><div class=\"ometa\">{}{}</div>{}</div>",
                    a.id,
                    esc(&a.titel()),
                    esc(&d.kunden_name(a.kunde_id)),
                    termin,
                    if a.wert != 0 { format!("<span>{}</span>", fmt_eur(a.wert)) } else { String::new() },
                    next
                );
            }
            h += "</div></div>";
        }
        h += "</div>";
        h
    }

    fn kunden_opts(&self, with_none: &str) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = vec![(String::new(), with_none.to_string())];
        let mut ks: Vec<&Kunde> = self.data.kunden.iter().collect();
        ks.sort_by_key(|k| k.name().to_lowercase());
        v.extend(ks.iter().map(|k| (k.id.to_string(), format!("{} ({})", k.name(), k.nr))));
        v
    }

    fn auftrag_html(&self, id: Id) -> String {
        let d = &self.data;
        let Some(a) = d.auftrag(Some(id)) else { return empty("briefcase", "Auftrag nicht gefunden", "", "") };
        let mut h = String::new();
        h += &head(&a.titel(), &format!("{} · eingegangen am {}", esc(&a.nr), german(&a.eingang)), &badge(a.status.key(), a.status.label()), true);
        h += "<div class=\"editor\"><div class=\"ed-main\">";
        h += "<div class=\"card\"><div class=\"form\">";
        h += &text("Titel / Anliegen", "a.titel", &a.titel, "placeholder=\"z. B. Heizung warten\"");
        h += &select_f("Kunde", "a.kunde_id", &self.kunden_opts("— kein Kunde —"), &a.kunde_id.map(|i| i.to_string()).unwrap_or_default(), "");
        let st: Vec<(String, String)> = AuftragStatus::ALLE.iter().map(|s| (s.key().to_string(), s.label().to_string())).collect();
        h += &select_f("Status", "a.status", &st, a.status.key(), "");
        h += &date_f("Eingang", "a.eingang", &a.eingang, "");
        h += &date_f("Termin / Frist", "a.termin", &a.termin, "");
        h += &money_f("Geschätzter Wert (netto)", "a.wert", a.wert, "placeholder=\"0,00\"");
        h += &area("Beschreibung der Anfrage", "a.beschreibung", &a.beschreibung, 5, "placeholder=\"Was wünscht der Kunde? Umfang, Ort, Besonderheiten …\"");
        h += &area("Interne Notizen", "a.notizen", &a.notizen, 3, "");
        h += "</div></div>";
        // Verlauf
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Verlauf</h2></div><ul class=\"timeline\">";
        for v in a.verlauf.iter().rev() {
            let _ = write!(h, "<li><span>{}</span>{}</li>", german(&v.datum), esc(&v.text));
        }
        let _ = write!(
            h,
            "</ul><div class=\"note-add\"><input type=\"text\" id=\"note-text\" placeholder=\"Notiz hinzufügen, z. B. „Kunde angerufen“\">{}</div></div>",
            btn("auftrag-note", "plus", "Eintragen", "ghost", Some(a.id), "")
        );
        h += "</div><aside class=\"ed-side\">";
        // Aktionen
        let next = a
            .status
            .next()
            .map(|n| btn("auftrag-next", "arrow", &format!("Weiter: {}", n.label()), "primary block", Some(a.id), ""))
            .unwrap_or_default();
        let _ = write!(
            h,
            "<div class=\"card actions\">{}{}{}{}{}</div>",
            next,
            btn("auftrag-dok", "file", "Angebot erstellen", "ghost block", Some(a.id), "angebot"),
            btn("auftrag-dok", "receipt", "Rechnung erstellen", "ghost block", Some(a.id), "rechnung"),
            if a.status != AuftragStatus::Abgelehnt { btn("auftrag-reject", "x", "Als abgelehnt markieren", "ghost block", Some(a.id), "") } else { String::new() },
            btn("auftrag-del", "trash", "Löschen", "ghost block danger-text", Some(a.id), "")
        );
        if let Some(k) = d.kunde(a.kunde_id) {
            let _ = write!(
                h,
                "<div class=\"card\"><div class=\"card-head\"><h2>Kunde</h2></div><button class=\"row\" data-act=\"kunde-open\" data-id=\"{}\"><div><b>{}</b><span>{}</span></div>{}</button></div>",
                k.id,
                esc(&k.name()),
                esc(&[k.telefon.as_str(), k.email.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ")),
                ic("chevron")
            );
        }
        let docs: Vec<&Dokument> = d.dokumente.iter().filter(|x| x.auftrag_id == Some(a.id)).collect();
        if !docs.is_empty() {
            let heute = today();
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Dokumente</h2></div><div class=\"rows\">";
            for x in docs {
                h += &doc_row(x, d, &heute, true);
            }
            h += "</div></div>";
        }
        h += "</aside></div>";
        h
    }

    // -----------------------------------------------------------------------
    // Angebote / Rechnungen
    // -----------------------------------------------------------------------

    fn doc_filter_ok(&self, x: &Dokument, heute: &str) -> bool {
        match self.ui.dok_filter.as_str() {
            "" | "alle" => true,
            "offen" => x.offen(),
            "zumahnen" => x.zu_mahnen(heute),
            "ueberfaellig" => x.ueberfaellig(heute),
            k => x.status.key() == k,
        }
    }

    pub fn dok_list_html(&self, art: DokArt) -> String {
        let heute = today();
        let q = self.ui.dok_q.trim().to_lowercase();
        let d = &self.data;
        let mut list: Vec<&Dokument> = d
            .dokumente
            .iter()
            .filter(|x| x.art == art && self.doc_filter_ok(x, &heute))
            .filter(|x| {
                q.is_empty()
                    || x.nummer.to_lowercase().contains(&q)
                    || x.betreff.to_lowercase().contains(&q)
                    || d.kunden_name(x.kunde_id).to_lowercase().contains(&q)
                    || x.empfaenger.to_lowercase().contains(&q)
            })
            .collect();
        list.sort_by(|a, b| b.datum.cmp(&a.datum).then(b.nummer.cmp(&a.nummer)));
        if list.is_empty() {
            return "<p class=\"muted pad\">Keine Einträge.</p>".into();
        }
        let mut h = String::from(
            "<table class=\"tbl\"><thead><tr><th>Nummer</th><th>Datum</th><th>Kunde</th><th class=\"hide-sm\">Betreff</th><th class=\"r\">Betrag</th><th>Status</th></tr></thead><tbody>",
        );
        let mut sum = 0;
        for x in &list {
            let s = x.summen();
            sum += s.brutto;
            let _ = write!(
                h,
                "<tr data-act=\"dok-open\" data-id=\"{}\"><td><b>{}</b></td><td>{}</td><td>{}</td><td class=\"hide-sm muted\">{}</td><td class=\"r\">{}</td><td>{}</td></tr>",
                x.id,
                esc(&x.nummer),
                german(&x.datum),
                esc(&d.kunden_name(x.kunde_id)),
                esc(&x.betreff),
                fmt_eur(s.brutto),
                badge(x.status_key(&heute), x.status_text(&heute))
            );
        }
        let _ = write!(
            h,
            "</tbody><tfoot><tr><td colspan=\"3\">{} {}</td><td class=\"hide-sm\"></td><td class=\"r\"><b>{}</b></td><td></td></tr></tfoot></table>",
            list.len(),
            if list.len() == 1 { art.label() } else { art.plural() },
            fmt_eur(sum)
        );
        h
    }

    fn dokumente_html(&self, art: DokArt) -> String {
        let mut h = String::new();
        let n = self.data.dokumente.iter().filter(|x| x.art == art).count();
        h += &head(art.plural(), &format!("{n} insgesamt"), &btn("dok-neu", "plus", &format!("Neue{} {}", if art == DokArt::Angebot { "s" } else { "" }, art.label()), "primary", None, art.key()), false);
        if n == 0 {
            h += &empty(
                if art == DokArt::Angebot { "file" } else { "receipt" },
                &format!("Noch keine {}", art.plural()),
                "Einfach ausfüllen – das PDF mit deinem Logo entsteht automatisch und kann direkt versendet werden.",
                &btn("dok-neu", "plus", &format!("{} erstellen", art.label()), "primary", None, art.key()),
            );
            return h;
        }
        let filters: Vec<(&str, &str)> = match art {
            DokArt::Angebot => vec![("alle", "Alle"), ("entwurf", "Entwurf"), ("versendet", "Versendet"), ("angenommen", "Angenommen"), ("abgelehnt", "Abgelehnt")],
            DokArt::Rechnung => vec![("alle", "Alle"), ("entwurf", "Entwurf"), ("offen", "Offen"), ("ueberfaellig", "Überfällig"), ("zumahnen", "Zu mahnen"), ("bezahlt", "Bezahlt"), ("storniert", "Storniert")],
        };
        let cur = if self.ui.dok_filter.is_empty() { "alle" } else { self.ui.dok_filter.as_str() };
        h += "<div class=\"toolbar\"><div class=\"chips\">";
        for (k, l) in filters {
            let _ = write!(h, "<button class=\"chip{}\" data-act=\"dok-filter\" data-v=\"{k}\">{l}</button>", if k == cur { " active" } else { "" });
        }
        let _ = write!(
            h,
            "</div><div class=\"search\">{}<input type=\"search\" placeholder=\"Nummer, Kunde, Betreff …\" data-search=\"dok\" value=\"{}\"></div></div>",
            ic("search"),
            esc(&self.ui.dok_q)
        );
        let _ = write!(h, "<div class=\"card flush\" id=\"list\">{}</div>", self.dok_list_html(art));
        h
    }

    /// Summenblock des Dokument-Editors (wird beim Tippen aktualisiert).
    pub fn doc_sum_html(&self, x: &Dokument) -> String {
        let s = x.summen();
        let mut h = String::from("<dl class=\"sum\">");
        if !s.steuern.is_empty() {
            let _ = write!(h, "<dt>Summe netto</dt><dd>{}</dd>", fmt_eur(s.netto));
            for (r, b, t) in &s.steuern {
                let label = if s.steuern.len() > 1 { format!("{r} % USt. auf {}", fmt_num(*b)) } else { format!("{r} % USt.") };
                let _ = write!(h, "<dt>{}</dt><dd>{}</dd>", label, fmt_eur(*t));
            }
        }
        let _ = write!(h, "<dt class=\"total\">Gesamt</dt><dd class=\"total\">{}</dd>", fmt_eur(s.brutto));
        if x.art == DokArt::Rechnung && (x.mahnkosten() > 0 || x.gezahlt() > 0) {
            if x.mahnkosten() > 0 {
                let _ = write!(h, "<dt>Mahngebühren/Zinsen</dt><dd>{}</dd>", fmt_eur(x.mahnkosten()));
            }
            if x.gezahlt() > 0 {
                let _ = write!(h, "<dt>Bereits bezahlt</dt><dd class=\"plus\">− {}</dd>", fmt_eur(x.gezahlt()));
            }
            let _ = write!(h, "<dt class=\"rest\">Noch offen</dt><dd class=\"rest\">{}</dd>", fmt_eur(x.rest()));
        }
        h += "</dl>";
        if x.kleinunternehmer {
            h += "<p class=\"muted small\">Ohne Umsatzsteuer (§ 19 UStG)</p>";
        }
        h
    }

    fn dokument_html(&self, id: Id) -> String {
        let heute = today();
        let d = &self.data;
        let Some(x) = d.dokument(Some(id)) else { return empty("file", "Dokument nicht gefunden", "", "") };
        let locked = x.festgeschrieben() && !self.ui.entsperrt.contains(&id);
        let ds = dis(locked);
        let mut h = String::new();
        h += &head(
            &format!("{} {}", x.titel(), x.nummer),
            &format!("{} · {}", esc(&d.kunden_name(x.kunde_id)), german(&x.datum)),
            &badge(x.status_key(&heute), x.status_text(&heute)),
            true,
        );
        if locked {
            let _ = write!(
                h,
                "<div class=\"banner\">{}<div>Dieses Dokument ist <b>{}</b> und damit festgeschrieben. Korrekturen bei Rechnungen bitte per Storno und neuer Rechnung.</div>{}</div>",
                ic("lock"),
                esc(&x.status.label().to_lowercase()),
                btn("dok-unlock", "unlock", "Trotzdem bearbeiten", "ghost", Some(id), "")
            );
        }
        h += "<div class=\"editor\"><div class=\"ed-main\">";
        // Empfänger
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Empfänger</h2></div><div class=\"form\">";
        h += &select_f("Kunde", "d.kunde_id", &self.kunden_opts("— ohne Kundenkonto —"), &x.kunde_id.map(|i| i.to_string()).unwrap_or_default(), ds);
        let _ = write!(h, "<div class=\"field-btn\">{}</div>", if locked { String::new() } else { btn("kunde-neu-dok", "plus", "Neuer Kunde", "ghost sm", Some(id), "") });
        h += &area("Anschrift (wie im Adressfeld)", "d.empfaenger", &x.empfaenger, 4, ds);
        h += "</div></div>";
        // Daten
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Angaben</h2></div><div class=\"form\">";
        h += &date_f(if x.art == DokArt::Rechnung { "Rechnungsdatum" } else { "Angebotsdatum" }, "d.datum", &x.datum, ds);
        h += &text(
            if x.art == DokArt::Rechnung { "Leistungsdatum / -zeitraum" } else { "Ausführung (optional)" },
            "d.leistung",
            &x.leistung,
            &format!("placeholder=\"z. B. {} oder Oktober 2026\"{}", german(&heute), ds),
        );
        h += &date_f(if x.art == DokArt::Rechnung { "Zahlbar bis" } else { "Gültig bis" }, "d.faellig", &x.faellig, ds);
        h += &text("Betreff", "d.betreff", &x.betreff, &format!("placeholder=\"z. B. Wartung Heizungsanlage\"{ds}"));
        h += &area("Einleitung", "d.einleitung", &x.einleitung, 3, ds);
        h += "</div></div>";
        // Positionen
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Positionen</h2></div><div class=\"positions\">";
        let n = x.positionen.len();
        for (i, p) in x.positionen.iter().enumerate() {
            let ust = if x.kleinunternehmer {
                String::new()
            } else {
                select_f("USt.", &format!("p.{i}.ust"), &ust_opts(), &p.ust.to_string(), ds)
            };
            let tools = if locked {
                String::new()
            } else {
                format!(
                    "<div class=\"pos-tools\"><button class=\"icon-btn sm\" data-act=\"pos-up\" data-v=\"{i}\" title=\"Nach oben\"{}>{}</button><button class=\"icon-btn sm\" data-act=\"pos-down\" data-v=\"{i}\" title=\"Nach unten\"{}>{}</button><button class=\"icon-btn sm\" data-act=\"pos-dup\" data-v=\"{i}\" title=\"Duplizieren\">{}</button><button class=\"icon-btn sm danger-text\" data-act=\"pos-del\" data-v=\"{i}\" title=\"Entfernen\">{}</button></div>",
                    dis(i == 0),
                    ic("up"),
                    dis(i + 1 == n),
                    ic("down"),
                    ic("copy"),
                    ic("trash")
                )
            };
            let _ = write!(
                h,
                "<div class=\"pos\"><div class=\"pos-num\">{}</div><div class=\"pos-body\"><textarea data-f=\"p.{i}.text\" rows=\"2\" placeholder=\"Leistung / Artikel (mehrzeilig: erste Zeile wird fett)\"{ds}>{}</textarea><div class=\"pos-grid\">{}{}{}{}<div class=\"pos-total\"><span>Gesamt</span><b id=\"lt-{i}\">{}</b></div></div></div>{}</div>",
                i + 1,
                esc(&p.text),
                text("Menge", &format!("p.{i}.menge"), &fmt_qty(p.menge), &format!("inputmode=\"decimal\"{ds}")),
                text("Einheit", &format!("p.{i}.einheit"), &p.einheit, &format!("list=\"einheiten\"{ds}")),
                money_f("Einzelpreis (netto)", &format!("p.{i}.preis"), p.preis, &format!("placeholder=\"0,00\"{ds}")),
                ust,
                fmt_eur(p.netto()),
                tools
            );
        }
        h += "<datalist id=\"einheiten\">";
        for e in EINHEITEN {
            let _ = write!(h, "<option value=\"{e}\">");
        }
        h += "</datalist></div>";
        if !locked {
            let _ = write!(
                h,
                "<div class=\"pos-add\">{}{}</div>",
                btn("pos-add", "plus", "Position", "ghost", None, ""),
                btn("pos-katalog", "book", "Aus Leistungskatalog", "ghost", None, "")
            );
        }
        h += "</div>";
        // Schluss
        h += "<div class=\"card\"><div class=\"form\">";
        h += &area("Schlusstext", "d.schluss", &x.schluss, 3, ds);
        h += "<p class=\"hint wide\">Platzhalter: {betrag} {faellig} {gueltig} {nummer} {datum} {zahlungsziel} {kunde}</p>";
        h += "</div></div>";
        h += "</div><aside class=\"ed-side\">";
        // Summen + Aktionen
        let _ = write!(h, "<div class=\"card sticky\"><div id=\"doc-sum\">{}</div>", self.doc_sum_html(x));
        h += "<div class=\"actions\">";
        h += &btn("dok-pdf", "eye", "PDF ansehen", "ghost block", Some(id), "");
        h += &btn("dok-send", "send", if x.status == DokStatus::Entwurf { "Per E-Mail senden" } else { "Erneut senden" }, "primary block", Some(id), "");
        h += &btn("dok-save", "download", "PDF speichern", "ghost block", Some(id), "");
        match x.art {
            DokArt::Rechnung => {
                if x.status == DokStatus::Entwurf {
                    h += &btn("dok-mark-sent", "check", "Als versendet markieren", "ghost block", Some(id), "");
                }
                if x.status == DokStatus::Versendet && x.storno_von.is_none() {
                    h += &btn("dok-paid", "check", "Zahlung erfassen", "success block", Some(id), "");
                    if x.ueberfaellig(&heute) || !x.mahnungen.is_empty() {
                        let label = format!("{} erstellen", MAHNSTUFEN[x.naechste_stufe() as usize]);
                        h += &btn("mahn-new", "bell", &label, if x.zu_mahnen(&heute) { "warn block" } else { "ghost block" }, Some(id), "");
                    }
                }
                if x.status == DokStatus::Bezahlt {
                    h += &format!("<p class=\"paid\">{} Bezahlt am {}</p>", ic("check"), german(&x.bezahlt_am));
                    h += &btn("dok-unpaid", "undo", "Zahlungen zurücknehmen", "ghost block", Some(id), "");
                }
                if matches!(x.status, DokStatus::Versendet | DokStatus::Bezahlt) && x.storno_von.is_none() {
                    h += &btn("dok-storno", "x", "Stornieren", "ghost block", Some(id), "");
                }
            }
            DokArt::Angebot => {
                if x.status == DokStatus::Entwurf {
                    h += &btn("dok-mark-sent", "check", "Als versendet markieren", "ghost block", Some(id), "");
                }
                h += &btn("dok-to-invoice", "receipt", "In Rechnung umwandeln", "ghost block", Some(id), "");
                if x.status == DokStatus::Versendet {
                    h += &btn("dok-status", "check", "Angenommen", "ghost block", Some(id), "angenommen");
                    h += &btn("dok-status", "x", "Abgelehnt", "ghost block", Some(id), "abgelehnt");
                }
            }
        }
        h += &btn("dok-dup", "copy", "Duplizieren", "ghost block", Some(id), "");
        h += &btn("dok-del", "trash", "Löschen", "ghost block danger-text", Some(id), "");
        h += "</div>";
        let _ = write!(
            h,
            "<label class=\"switch\"><input type=\"checkbox\" data-f=\"d.kleinunternehmer\"{}{ds}><span>Ohne USt. (§ 19 UStG)</span></label>",
            if x.kleinunternehmer { " checked" } else { "" }
        );
        h += "</div>";
        // Zahlungen
        if !x.zahlungen.is_empty() {
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Zahlungen</h2></div><div class=\"rows\">";
            for z in &x.zahlungen {
                let _ = write!(
                    h,
                    "<div class=\"row static\"><div><b class=\"plus\">{}</b><span>{}{}</span></div><button class=\"icon-btn sm danger-text\" data-act=\"zahlung-del\" data-id=\"{id}\" data-v=\"{}\" title=\"Zahlung löschen\">{}</button></div>",
                    fmt_eur(z.betrag),
                    german(&z.datum),
                    if z.zahlart.is_empty() { String::new() } else { format!(" · {}", esc(&z.zahlart)) },
                    z.id,
                    ic("trash")
                );
            }
            h += "</div></div>";
        }
        // Mahnungen
        if !x.mahnungen.is_empty() {
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Mahnungen</h2></div><div class=\"rows\">";
            for (i, m) in x.mahnungen.iter().enumerate() {
                let kosten = if m.gebuehr + m.zinsen > 0 { format!(" · +{}", fmt_eur(m.gebuehr + m.zinsen)) } else { String::new() };
                let _ = write!(
                    h,
                    "<div class=\"row static\"><div><b>{}</b><span>{} · Frist {}{}{}</span></div><div class=\"row-r\"><button class=\"icon-btn sm\" data-act=\"mahn-pdf\" data-id=\"{id}\" data-v=\"{i}\" title=\"Ansehen\">{}</button><button class=\"icon-btn sm\" data-act=\"mahn-send\" data-id=\"{id}\" data-v=\"{i}\" title=\"Senden\">{}</button><button class=\"icon-btn sm danger-text\" data-act=\"mahn-del\" data-id=\"{id}\" data-v=\"{i}\" title=\"Löschen\">{}</button></div></div>",
                    m.label(),
                    german(&m.datum),
                    german(&m.frist),
                    kosten,
                    if m.versendet { " · versendet" } else { "" },
                    ic("eye"),
                    ic("send"),
                    ic("trash")
                );
            }
            h += "</div></div>";
        }
        // Verknüpfungen
        let mut links = String::new();
        if let Some(k) = d.kunde(x.kunde_id) {
            let _ = write!(links, "<button class=\"row\" data-act=\"kunde-open\" data-id=\"{}\"><div><b>{}</b><span>Kunde {}</span></div>{}</button>", k.id, esc(&k.name()), esc(&k.nr), ic("chevron"));
        }
        if let Some(a) = d.auftrag(x.auftrag_id) {
            let _ = write!(links, "<button class=\"row\" data-act=\"auftrag-open\" data-id=\"{}\"><div><b>{}</b><span>Auftrag {}</span></div>{}</button>", a.id, esc(&a.titel()), esc(&a.nr), ic("chevron"));
        }
        for o in [x.aus_angebot, x.storno_von] {
            if let Some(o) = d.dokument(o) {
                let _ = write!(links, "<button class=\"row\" data-act=\"dok-open\" data-id=\"{}\"><div><b>{} {}</b><span>{}</span></div>{}</button>", o.id, o.titel(), esc(&o.nummer), german(&o.datum), ic("chevron"));
            }
        }
        for o in d.dokumente.iter().filter(|o| o.aus_angebot == Some(id) || o.storno_von == Some(id)) {
            let _ = write!(links, "<button class=\"row\" data-act=\"dok-open\" data-id=\"{}\"><div><b>{} {}</b><span>{}</span></div>{}</button>", o.id, o.titel(), esc(&o.nummer), german(&o.datum), ic("chevron"));
        }
        if !links.is_empty() {
            let _ = write!(h, "<div class=\"card\"><div class=\"card-head\"><h2>Verknüpft</h2></div><div class=\"rows\">{links}</div></div>");
        }
        h += "</aside></div>";
        h
    }

    // -----------------------------------------------------------------------
    // Finanzen
    // -----------------------------------------------------------------------

    pub fn buch_list_html(&self) -> String {
        let d = &self.data;
        let q = self.ui.buch_q.trim().to_lowercase();
        let mut list: Vec<&Buchung> = d
            .buchungen
            .iter()
            .filter(|b| date::year(&b.datum) == Some(self.ui.jahr))
            .filter(|b| self.ui.buch_monat == 0 || date::month(&b.datum) == Some(self.ui.buch_monat))
            .filter(|b| match self.ui.buch_filter.as_str() {
                "einnahme" => b.art == BuchArt::Einnahme,
                "ausgabe" => b.art == BuchArt::Ausgabe,
                "ohnebeleg" => b.art == BuchArt::Ausgabe && b.beleg.is_none(),
                _ => true,
            })
            .filter(|b| q.is_empty() || b.matches(&q))
            .collect();
        list.sort_by(|a, b| b.datum.cmp(&a.datum).then(b.id.cmp(&a.id)));
        if list.is_empty() {
            return "<p class=\"muted pad\">Keine Buchungen in diesem Zeitraum.</p>".into();
        }
        let mut ein = 0;
        let mut aus = 0;
        let mut h = String::from("<div class=\"rows\">");
        for b in &list {
            match b.art {
                BuchArt::Einnahme => ein += b.brutto,
                BuchArt::Ausgabe => aus += b.brutto,
            }
            h += &buch_row(b, d);
        }
        let _ = write!(
            h,
            "</div><div class=\"list-foot\"><span>{} Buchungen</span><span class=\"plus\">+ {}</span><span class=\"neg\">− {}</span></div>",
            list.len(),
            fmt_eur(ein),
            fmt_eur(aus)
        );
        h
    }

    fn finanzen_html(&self) -> String {
        let d = &self.data;
        let heute = today();
        let a = report::auswertung(d, self.ui.jahr);
        let ku = d.firma.kleinunternehmer;
        let mut h = String::new();
        let jahre: Vec<(String, String)> = report::jahre(d, &heute).iter().map(|y| (y.to_string(), y.to_string())).collect();
        let jahr_sel = select_f("", "ui.jahr", &jahre, &self.ui.jahr.to_string(), "class=\"compact\"");
        h += &head(
            "Finanzen",
            "Einnahmen, Ausgaben und Steuer im Überblick",
            &format!(
                "{jahr_sel}{}{}",
                btn("buch-neu", "plus", "Einnahme", "ghost", None, "einnahme"),
                btn("buch-neu", "plus", "Ausgabe", "primary", None, "ausgabe")
            ),
            false,
        );
        h += "<section class=\"kpis\">";
        h += &kpi("Einnahmen", &fmt_eur(a.ein.netto), &format!("netto · brutto {}", esc(&fmt_eur(a.ein.brutto))), "", "");
        h += &kpi("Ausgaben", &fmt_eur(a.aus.netto), &format!("netto · brutto {}", esc(&fmt_eur(a.aus.brutto))), "", "");
        h += &kpi("Gewinn", &fmt_eur(a.gewinn()), "vereinfacht, netto", if a.gewinn() < 0 { "warn" } else { "good" }, "");
        if !ku {
            h += &kpi("USt.-Zahllast", &fmt_eur(a.zahllast()), &format!("USt. {} − VSt. {}", esc(&fmt_eur(a.ein.steuer)), esc(&fmt_eur(a.aus.steuer))), "", "");
        }
        h += "</section>";

        h += "<section class=\"grid2\">";
        let _ = write!(
            h,
            "<div class=\"card\"><div class=\"card-head\"><h2>Monatsverlauf {}</h2><div class=\"legend\"><i class=\"in\"></i>Einnahmen <i class=\"out\"></i>Ausgaben</div></div>{}</div>",
            a.jahr,
            chart(&a)
        );
        // Steuer / Export
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Für die Steuer</h2></div>";
        if !ku {
            h += "<table class=\"tbl compact\"><thead><tr><th>Quartal</th><th class=\"r\">Umsatzsteuer</th><th class=\"r\">Vorsteuer</th><th class=\"r\">Zahllast</th></tr></thead><tbody>";
            for q in 1..=4 {
                let (u, v) = a.quartal(q);
                let _ = write!(h, "<tr><td>Q{q}</td><td class=\"r\">{}</td><td class=\"r\">{}</td><td class=\"r\"><b>{}</b></td></tr>", fmt_eur(u), fmt_eur(v), fmt_eur(u - v));
            }
            h += "</tbody></table>";
        } else {
            h += "<p class=\"muted pad\">Kleinunternehmer (§ 19 UStG): keine Umsatzsteuer-Voranmeldung nötig.</p>";
        }
        let ohne_beleg = d.buchungen.iter().filter(|b| b.art == BuchArt::Ausgabe && b.beleg.is_none() && date::year(&b.datum) == Some(self.ui.jahr)).count();
        if ohne_beleg > 0 {
            let _ = write!(h, "<p class=\"pad warn-text\">{} {} Ausgabe(n) ohne Beleg – <a href=\"#\" data-act=\"buch-filter\" data-v=\"ohnebeleg\">anzeigen</a></p>", ic("alert"), ohne_beleg);
        }
        let _ = write!(
            h,
            "<div class=\"export\">{}{}{}</div></div>",
            btn("fin-pdf", "file", &format!("Jahresübersicht {} (PDF)", a.jahr), "ghost", None, ""),
            btn("fin-csv", "download", "Buchungen (CSV)", "ghost", None, ""),
            btn("fin-csv-re", "download", "Rechnungsausgangsbuch (CSV)", "ghost", None, "")
        );
        // Kategorien
        if !a.kat_aus.is_empty() || !a.kat_ein.is_empty() {
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Nach Kategorie</h2></div><div class=\"cats\">";
            let max = a.kat_aus.iter().chain(a.kat_ein.iter()).map(|k| k.1.netto.abs()).max().unwrap_or(1).max(1);
            for (list, cls) in [(&a.kat_ein, "in"), (&a.kat_aus, "out")] {
                for (k, s, neutral) in list.iter() {
                    let w = (s.netto.abs() as f64 / max as f64 * 100.0).max(1.0);
                    let _ = write!(
                        h,
                        "<div class=\"cat\"><span>{}{}</span><div class=\"catbar\"><i class=\"{cls}\" style=\"width:{:.1}%\"></i></div><b>{}</b></div>",
                        esc(k),
                        if *neutral { " <em>(neutral)</em>" } else { "" },
                        w,
                        fmt_eur(s.netto)
                    );
                }
            }
            h += "</div></div>";
        }
        h += "</section>";

        // Buchungsliste
        let cur = if self.ui.buch_filter.is_empty() { "alle" } else { self.ui.buch_filter.as_str() };
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Buchungen</h2></div><div class=\"toolbar pad\"><div class=\"chips\">";
        for (k, l) in [("alle", "Alle"), ("einnahme", "Einnahmen"), ("ausgabe", "Ausgaben"), ("ohnebeleg", "Ohne Beleg")] {
            let _ = write!(h, "<button class=\"chip{}\" data-act=\"buch-filter\" data-v=\"{k}\">{l}</button>", if k == cur { " active" } else { "" });
        }
        let mut monate = String::from("<option value=\"0\">Ganzes Jahr</option>");
        for (i, m) in MONTHS.iter().enumerate() {
            let _ = write!(monate, "<option value=\"{}\"{}>{m}</option>", i + 1, if self.ui.buch_monat == i as u32 + 1 { " selected" } else { "" });
        }
        let _ = write!(
            h,
            "</div><select class=\"compact\" data-search=\"buch-monat\">{monate}</select><div class=\"search\">{}<input type=\"search\" placeholder=\"Suchen …\" data-search=\"buch\" value=\"{}\"></div></div><div id=\"list\">{}</div></div>",
            ic("search"),
            esc(&self.ui.buch_q),
            self.buch_list_html()
        );
        h
    }

    pub fn buch_calc_html(&self, b: &Buchung) -> String {
        format!(
            "<span>Netto <b>{}</b></span><span>USt. {} % <b>{}</b></span>",
            fmt_eur(b.netto()),
            b.ust,
            fmt_eur(b.steuer())
        )
    }

    fn buchung_html(&self, id: Id) -> String {
        let d = &self.data;
        let Some(b) = d.buchung(Some(id)) else { return empty("wallet", "Buchung nicht gefunden", "", "") };
        let mut h = String::new();
        h += &head(
            if b.art == BuchArt::Einnahme { "Einnahme" } else { "Ausgabe" },
            &german(&b.datum),
            &btn("buch-del", "trash", "Löschen", "ghost danger-text", Some(id), ""),
            true,
        );
        h += "<div class=\"editor\"><div class=\"ed-main\"><div class=\"card\">";
        let _ = write!(
            h,
            "<div class=\"seg\"><button class=\"{}\" data-act=\"buch-art\" data-v=\"einnahme\">{}Einnahme</button><button class=\"{}\" data-act=\"buch-art\" data-v=\"ausgabe\">{}Ausgabe</button></div>",
            if b.art == BuchArt::Einnahme { "active in" } else { "" },
            ic("in"),
            if b.art == BuchArt::Ausgabe { "active out" } else { "" },
            ic("out")
        );
        h += "<div class=\"form\">";
        h += &money_f("Betrag (brutto)", "b.brutto", b.brutto, "placeholder=\"0,00\" autofocus");
        h += &select_f("USt.-Satz", "b.ust", &ust_opts(), &b.ust.to_string(), "");
        let _ = write!(h, "<div class=\"calc wide\" id=\"buch-calc\">{}</div>", self.buch_calc_html(b));
        h += &date_f("Datum (Zahlung)", "b.datum", &b.datum, "");
        let kats: Vec<(String, String)> = {
            let mut v: Vec<(String, String)> = kategorien(b.art).iter().map(|k| (k.name.to_string(), k.name.to_string())).collect();
            if !v.iter().any(|x| x.0 == b.kategorie) && !b.kategorie.is_empty() {
                v.push((b.kategorie.clone(), b.kategorie.clone()));
            }
            v
        };
        h += &select_f("Kategorie", "b.kategorie", &kats, &b.kategorie, "");
        h += &text("Beschreibung", "b.text", &b.text, "placeholder=\"z. B. Material Baustelle Müller\"");
        h += &text(if b.art == BuchArt::Einnahme { "Kunde / Zahler" } else { "Lieferant / Empfänger" }, "b.partner", &b.partner, "list=\"partner\"");
        let za: Vec<(String, String)> = ZAHLARTEN.iter().map(|z| (z.to_string(), z.to_string())).collect();
        h += &select_f("Zahlart", "b.zahlart", &za, &b.zahlart, "");
        h += "<datalist id=\"partner\">";
        let mut names: Vec<String> = d.kunden.iter().map(|k| k.name()).chain(d.buchungen.iter().map(|x| x.partner.clone())).filter(|s| !s.trim().is_empty()).collect();
        names.sort();
        names.dedup();
        for n in names {
            let _ = write!(h, "<option value=\"{}\">", esc(&n));
        }
        h += "</datalist></div>";
        if ist_neutral(&b.kategorie) {
            h += "<p class=\"hint pad\">Diese Kategorie ist gewinnneutral und zählt nicht zu Gewinn oder Umsatzsteuer.</p>";
        }
        h += "</div></div><aside class=\"ed-side\">";
        // Beleg
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Beleg</h2></div>";
        match d.datei(b.beleg) {
            Some(f) => {
                let _ = write!(
                    h,
                    "<div class=\"file\">{}<div><b>{}</b><span>{} KB</span></div></div><div class=\"actions\">{}{}{}</div>",
                    ic(if f.mime == "application/pdf" { "file" } else { "image" }),
                    esc(&f.name),
                    f.groesse / 1024,
                    btn("beleg-view", "eye", "Ansehen", "ghost block", Some(id), ""),
                    btn("beleg-add", "paperclip", "Ersetzen", "ghost block", Some(id), ""),
                    btn("beleg-del", "trash", "Entfernen", "ghost block danger-text", Some(id), "")
                );
            }
            None => {
                let _ = write!(
                    h,
                    "<p class=\"muted pad\">Foto oder PDF der Quittung/Rechnung – wichtig für das Finanzamt.</p><div class=\"actions\">{}</div>",
                    btn("beleg-add", "paperclip", "Beleg anhängen", "primary block", Some(id), "")
                );
            }
        }
        h += "</div>";
        if let Some(x) = d.dokument(b.dokument_id) {
            let _ = write!(
                h,
                "<div class=\"card\"><div class=\"card-head\"><h2>Zugehörige Rechnung</h2></div><button class=\"row\" data-act=\"dok-open\" data-id=\"{}\"><div><b>{}</b><span>{}</span></div>{}</button></div>",
                x.id,
                esc(&x.nummer),
                esc(&d.kunden_name(x.kunde_id)),
                ic("chevron")
            );
        }
        h += "</aside></div>";
        h
    }

    // -----------------------------------------------------------------------
    // Dialoge
    // -----------------------------------------------------------------------

    pub fn kunden_pick_list(&self, q: &str, art: &str) -> String {
        let q = q.trim().to_lowercase();
        let mut ks: Vec<&Kunde> = self.data.kunden.iter().filter(|k| q.is_empty() || k.matches(&q)).collect();
        ks.sort_by_key(|k| k.name().to_lowercase());
        let mut h = String::new();
        for k in ks.iter().take(60) {
            let _ = write!(
                h,
                "<button class=\"li\" data-act=\"dok-neu-kunde\" data-id=\"{}\" data-v=\"{}\"><div class=\"avatar\">{}</div><div><b>{}</b><span>{} · {}</span></div></button>",
                k.id,
                art,
                esc(&initials(&k.name())),
                esc(&k.name()),
                esc(&k.nr),
                esc(&k.ort)
            );
        }
        if ks.is_empty() {
            h += "<p class=\"muted pad\">Kein Kunde gefunden.</p>";
        }
        h
    }

    pub fn pick_kunde_modal(&mut self, art: DokArt) {
        let html = format!(
            "<h3>{} für welchen Kunden?</h3><div class=\"search\">{}<input type=\"search\" placeholder=\"Kunden suchen …\" data-search=\"pick-kunde\" data-v=\"{}\" id=\"pick-q\" autofocus></div><div class=\"list pick\" id=\"pick-list\">{}</div><div class=\"modal-actions\">{}{}{}</div>",
            art.label(),
            ic("search"),
            art.key(),
            self.kunden_pick_list("", art.key()),
            btn("modal-close", "", "Abbrechen", "ghost", None, ""),
            btn("dok-neu-ohne", "", "Ohne Kunde", "ghost", None, art.key()),
            btn("dok-neu-neukunde", "plus", "Neuer Kunde", "primary", None, art.key())
        );
        self.modal(&html);
    }

    pub fn katalog_modal(&mut self) {
        let mut h = String::from("<h3>Aus Leistungskatalog einfügen</h3><div class=\"list pick\">");
        if self.data.katalog.is_empty() {
            h += "<p class=\"muted pad\">Der Katalog ist noch leer. Lege Leistungen unter Einstellungen → Leistungskatalog an.</p>";
        }
        for l in &self.data.katalog {
            let _ = write!(
                h,
                "<button class=\"li\" data-act=\"kat-pick\" data-id=\"{}\"><div><b>{}</b><span>{} € je {} · {} % USt.</span></div>{}</button>",
                l.id,
                esc(if l.text.is_empty() { "Ohne Bezeichnung" } else { &l.text }),
                fmt_num(l.preis),
                esc(&l.einheit),
                l.ust,
                ic("plus")
            );
        }
        let _ = write!(h, "</div><div class=\"modal-actions\">{}{}</div>", btn("nav", "settings", "Katalog bearbeiten", "ghost", None, "einstellungen"), btn("modal-close", "", "Fertig", "primary", None, ""));
        self.modal(&h);
    }

    /// Dialog „Mahnung erstellen“ (Stufe, Datum, Frist, Gebühr, Verzugszinsen).
    pub fn mahn_modal(&mut self, id: Id, stufe: u8) {
        let heute = today();
        let Some(x) = self.data.dokument(Some(id)) else { return };
        let f = &self.data.firma;
        let (tage, gebuehr, _) = f.mahnstufe(stufe);
        let (satz, ztage, zinsen) = self.data.verzugszinsen(id, &heute);
        let mut opts = String::new();
        for (i, l) in MAHNSTUFEN.iter().enumerate() {
            let _ = write!(opts, "<option value=\"{i}\"{}>{l}</option>", if i as u8 == stufe { " selected" } else { "" });
        }
        let html = format!(
            "<h3>Mahnung erstellen</h3><p class=\"muted\">Rechnung {} · fällig seit {} · offen <b>{}</b></p><div class=\"form\"><label class=\"field\"><span>Stufe</span><select id=\"mahn-stufe\" data-search=\"mahn-stufe\" data-id=\"{id}\">{opts}</select></label><label class=\"field\"><span>Datum</span><input type=\"date\" id=\"mahn-datum\" value=\"{heute}\"></label><label class=\"field\"><span>Neue Frist bis</span><input type=\"date\" id=\"mahn-frist\" value=\"{}\"></label><label class=\"field\"><span>Mahngebühr</span><div class=\"euro\"><input type=\"text\" inputmode=\"decimal\" id=\"mahn-gebuehr\" value=\"{}\"><i>€</i></div></label><label class=\"switch wide\"><input type=\"checkbox\" id=\"mahn-zinsen\"{}><span>Verzugszinsen: {} ({} % p. a., {} Tage)</span></label></div><div class=\"modal-actions\">{}{}<button class=\"btn primary\" data-act=\"mahn-ok\" data-id=\"{id}\" data-v=\"send\" data-enter>{}<span>Erstellen &amp; senden</span></button></div>",
            esc(&x.nummer),
            german(&x.faellig),
            fmt_eur(x.rest()),
            date::add_days(&heute, tage as i64),
            fmt_num(gebuehr),
            if f.verzugszinsen { " checked" } else { "" },
            fmt_eur(zinsen),
            format!("{satz:.2}").replace('.', ","),
            ztage,
            btn("modal-close", "", "Abbrechen", "ghost", None, ""),
            btn("mahn-ok", "eye", "Erstellen & ansehen", "ghost", Some(id), "open"),
            ic("send")
        );
        self.modal(&html);
    }

    pub fn paid_modal(&mut self, id: Id) {
        let Some(x) = self.data.dokument(Some(id)) else { return };
        let mut za = String::new();
        for z in ZAHLARTEN {
            let _ = write!(za, "<option>{z}</option>");
        }
        let html = format!(
            "<h3>Zahlung erfassen</h3><p class=\"muted\">Rechnung {} · offen <b>{}</b>. Die Einnahme wird automatisch in den Finanzen gebucht; bei einem kleineren Betrag bleibt der Rest offen (Teilzahlung).</p><div class=\"form\"><label class=\"field\"><span>Betrag</span><div class=\"euro\"><input type=\"text\" inputmode=\"decimal\" id=\"pay-betrag\" value=\"{}\"><i>€</i></div></label><label class=\"field\"><span>Zahlungseingang am</span><input type=\"date\" id=\"pay-date\" value=\"{}\"></label><label class=\"field\"><span>Zahlart</span><select id=\"pay-art\">{za}</select></label></div><div class=\"modal-actions\">{}<button class=\"btn success\" data-act=\"dok-paid-ok\" data-id=\"{id}\" data-enter>{}<span>Zahlung buchen</span></button></div>",
            esc(&x.nummer),
            fmt_eur(x.rest()),
            fmt_num(x.rest()),
            today(),
            btn("modal-close", "", "Abbrechen", "ghost", None, ""),
            ic("check")
        );
        self.modal(&html);
    }
}

/// Hinweis unter dem IBAN-Feld.
pub fn iban_hint(iban: &str) -> String {
    if iban.trim().is_empty() {
        "Ohne IBAN erscheint kein GiroCode auf den Rechnungen.".into()
    } else if crate::engine::girocode::iban_valid(iban) {
        "✓ IBAN gültig – GiroCode wird erzeugt.".into()
    } else {
        "⚠ IBAN ungültig (Prüfziffer) – bitte prüfen.".into()
    }
}

fn initials(name: &str) -> String {
    let mut s: String = name.split_whitespace().filter_map(|w| w.chars().next()).filter(|c| c.is_alphanumeric()).take(2).collect();
    if s.is_empty() {
        s = "?".into();
    }
    s.to_uppercase()
}

fn doc_row(x: &Dokument, d: &Data, heute: &str, show_kind: bool) -> String {
    let kind = if show_kind { format!("{} ", x.titel()) } else { String::new() };
    let info = if !x.betreff.trim().is_empty() { x.betreff.trim().to_string() } else { d.kunden_name(x.kunde_id) };
    format!(
        "<button class=\"row\" data-act=\"dok-open\" data-id=\"{}\"><div><b>{}{}</b><span>{} · {}</span></div><div class=\"row-r\"><strong>{}</strong>{}</div></button>",
        x.id,
        esc(&kind),
        esc(&x.nummer),
        german(&x.datum),
        esc(&info),
        fmt_eur(x.summen().brutto),
        badge(x.status_key(heute), x.status_text(heute))
    )
}

fn buch_row(b: &Buchung, d: &Data) -> String {
    let sign = if b.art == BuchArt::Einnahme { "+" } else { "−" };
    let titel = if !b.text.trim().is_empty() {
        b.text.trim().to_string()
    } else if !b.partner.trim().is_empty() {
        b.partner.trim().to_string()
    } else {
        b.kategorie.clone()
    };
    let beleg = if b.beleg.is_some() {
        ic("paperclip")
    } else if b.art == BuchArt::Ausgabe {
        "<span class=\"nobeleg\" title=\"Kein Beleg\">!</span>".to_string()
    } else {
        String::new()
    };
    let partner = if !b.partner.trim().is_empty() && b.partner.trim() != titel { format!(" · {}", esc(b.partner.trim())) } else { String::new() };
    let rechnung = d.dokument(b.dokument_id).map(|x| format!(" · {}", esc(&x.nummer))).unwrap_or_default();
    format!(
        "<button class=\"row\" data-act=\"buch-open\" data-id=\"{}\"><div class=\"bdate\">{}</div><div><b>{}</b><span>{}{}</span></div><div class=\"row-r\">{}<strong class=\"{}\">{} {}</strong></div></button>",
        b.id,
        date::german_short(&b.datum),
        esc(&titel),
        esc(&b.kategorie),
        partner + &rechnung,
        beleg,
        if b.art == BuchArt::Einnahme { "plus" } else { "neg" },
        sign,
        fmt_eur(b.brutto)
    )
}
