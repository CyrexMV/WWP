//! Einstellungen: Kategorien links, Inhalt in Gruppen rechts, Live-Vorschau des Briefbogens
//! und eine Übersicht, wie vollständig die Einrichtung ist.

use super::views::{area, btn, head, ic, iban_hint, money_f, text};
use super::{esc, set_html, theme, App};
use crate::engine::girocode::iban_valid;
use crate::engine::model::{EINHEITEN, MAHNSTUFEN};
use std::fmt::Write as _;

/// (Schlüssel, Symbol, Titel, Beschreibung)
pub const SECTIONS: &[(&str, &str, &str, &str)] = &[
    ("firma", "building", "Firma & Kontakt", "Name, Anschrift, Erreichbarkeit"),
    ("bank", "landmark", "Bank & Steuern", "IBAN, Steuernummer, GiroCode"),
    ("brief", "palette", "Briefbogen", "Logo, Farbe, Vorschau"),
    ("nummern", "hash", "Nummern & Fristen", "Präfixe, Zahlungsziel"),
    ("mahnung", "bell", "Mahnwesen", "Stufen, Gebühren, Zinsen"),
    ("texte", "type", "Texte & E-Mails", "Vorlagen für Angebote und Rechnungen"),
    ("katalog", "book", "Leistungskatalog", "Häufige Positionen"),
    ("darstellung", "moon", "Darstellung", "Hell, dunkel oder automatisch"),
    ("daten", "database", "Daten & Backup", "Sichern und wiederherstellen"),
];

/// Akzentfarben zur Auswahl.
const FARBEN: &[(&str, &str)] = &[
    ("#2f5bd3", "Blau"),
    ("#0369a1", "Petrol"),
    ("#0f766e", "Türkis"),
    ("#15803d", "Grün"),
    ("#7c3aed", "Violett"),
    ("#be123c", "Rot"),
    ("#c2410c", "Orange"),
    ("#1f2937", "Anthrazit"),
];

const LOGO_GROESSEN: &[(u32, &str)] = &[(30, "Klein"), (45, "Mittel"), (60, "Groß"), (80, "Sehr groß")];

fn group(title: &str, desc: &str, body: &str) -> String {
    let d = if desc.is_empty() { String::new() } else { format!("<p>{desc}</p>") };
    format!("<section class=\"group\"><div class=\"group-head\"><h3>{title}</h3>{d}</div><div class=\"group-body\">{body}</div></section>")
}

fn group_with(title: &str, desc: &str, action: &str, body: &str) -> String {
    let d = if desc.is_empty() { String::new() } else { format!("<p>{desc}</p>") };
    format!(
        "<section class=\"group\"><div class=\"group-head row-head\"><div><h3>{title}</h3>{d}</div>{action}</div><div class=\"group-body\">{body}</div></section>"
    )
}

/// Zeile mit Beschreibung und Kippschalter.
fn toggle(key: &str, checked: bool, title: &str, desc: &str) -> String {
    format!(
        "<label class=\"toggle-row\"><div><b>{title}</b><span>{desc}</span></div><input type=\"checkbox\" class=\"tgl\" data-f=\"{key}\"{}></label>",
        if checked { " checked" } else { "" }
    )
}

/// Segmentierte Auswahl (Knöpfe).
fn seg(act: &str, opts: &[(String, String)], cur: &str) -> String {
    let mut h = String::from("<div class=\"segmented\">");
    for (v, l) in opts {
        let _ = write!(h, "<button class=\"{}\" data-act=\"{act}\" data-v=\"{}\">{}</button>", if v == cur { "active" } else { "" }, esc(v), esc(l));
    }
    h += "</div>";
    h
}

/// Zahlenfeld mit Einheit dahinter („Tage“, „%“).
fn suffix(label: &str, key: &str, value: &str, unit: &str) -> String {
    format!(
        "<label class=\"field\"><span>{}</span><div class=\"euro unit\"><input type=\"text\" inputmode=\"decimal\" data-f=\"{key}\" value=\"{}\"><i>{unit}</i></div></label>",
        esc(label),
        esc(value)
    )
}

fn placeholders(list: &[&str]) -> String {
    let mut h = String::from("<div class=\"ph-chips\"><span>Platzhalter</span>");
    for p in list {
        let _ = write!(h, "<code>{{{p}}}</code>");
    }
    h += "</div>";
    h
}

impl App {
    fn settings_tab(&self) -> &'static str {
        SECTIONS.iter().find(|s| s.0 == self.ui.settings_tab).map(|s| s.0).unwrap_or("firma")
    }

    /// Einrichtungs-Checkliste: (erledigt, Bezeichnung, Bereich)
    fn checkliste(&self) -> Vec<(bool, &'static str, &'static str)> {
        let f = &self.data.firma;
        vec![
            (!f.name.trim().is_empty(), "Firmenname", "firma"),
            (!f.strasse.trim().is_empty() && !f.plz_ort.trim().is_empty(), "Anschrift", "firma"),
            (!f.telefon.trim().is_empty() || !f.email.trim().is_empty(), "Kontakt", "firma"),
            (iban_valid(&f.iban), "IBAN", "bank"),
            (!f.steuernummer.trim().is_empty() || !f.ust_id.trim().is_empty(), "Steuernummer", "bank"),
            (self.logo.is_some(), "Logo", "brief"),
            (!self.data.katalog.is_empty(), "Leistungskatalog", "katalog"),
        ]
    }

    /// Profilkarte mit Fortschritt der Einrichtung.
    pub fn settings_profile_html(&self) -> String {
        let f = &self.data.firma;
        let list = self.checkliste();
        let done = list.iter().filter(|c| c.0).count();
        let pct = done * 100 / list.len();
        let avatar = match &self.logo_url {
            Some(u) => format!("<div class=\"prof-logo\"><img src=\"{}\" alt=\"\"></div>", esc(u)),
            None => format!("<div class=\"prof-logo empty\">{}</div>", ic("building")),
        };
        let name = if f.name.trim().is_empty() { "Deine Firma".to_string() } else { f.name.trim().to_string() };
        let ort = [f.zusatz.trim(), f.plz_ort.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
        let mut missing = String::new();
        for (ok, l, sec) in &list {
            if !ok {
                let _ = write!(missing, "<button class=\"miss\" data-act=\"set-tab\" data-v=\"{sec}\">{}{l}</button>", ic("plus"));
            }
        }
        let status = if done == list.len() {
            format!("<p class=\"prof-ok\">{} Alles eingerichtet</p>", ic("check"))
        } else {
            format!("<div class=\"prof-miss\">{missing}</div>")
        };
        format!(
            "<div class=\"profile\">{avatar}<div class=\"prof-text\"><b>{}</b><span>{}</span></div></div><div class=\"progress\"><div class=\"progress-top\"><span>Einrichtung</span><b>{done}/{}</b></div><div class=\"bar\"><i style=\"width:{pct}%\"></i></div></div>{status}",
            esc(&name),
            if ort.is_empty() { "Noch keine Anschrift".to_string() } else { esc(&ort) },
            list.len()
        )
    }

    /// Verkleinerter Briefbogen als Live-Vorschau.
    pub fn brief_preview_html(&self) -> String {
        let f = &self.data.firma;
        let farbe = if f.farbe.len() == 7 && f.farbe.starts_with('#') { f.farbe.clone() } else { "#2f5bd3".into() };
        let kopf = match &self.logo_url {
            Some(u) => format!(
                "<img class=\"bp-logo\" src=\"{}\" style=\"width:{:.0}%\" alt=\"\">",
                esc(u),
                (f.logo_breite_mm.clamp(15.0, 90.0) / 210.0 * 100.0)
            ),
            None => format!(
                "<div class=\"bp-name\">{}</div>",
                esc(if f.name.trim().is_empty() { "Deine Firma" } else { f.name.trim() })
            ),
        };
        let absender = f.absenderzeile();
        let qr = if f.girocode && iban_valid(&f.iban) { "<div class=\"bp-qr\"></div>" } else { "" };
        let ku = if f.kleinunternehmer { "<div class=\"bp-small\">Gemäß § 19 UStG keine Umsatzsteuer</div>" } else { "" };
        let fuss = [
            f.name.trim(),
            f.strasse.trim(),
            f.plz_ort.trim(),
            f.iban.trim(),
            f.steuernummer.trim(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| esc(s))
        .collect::<Vec<_>>()
        .join(" · ");
        format!(
            "<div class=\"bp-page\" style=\"--bp:{farbe}\"><div class=\"bp-head\">{kopf}</div><div class=\"bp-sender\">{}</div><div class=\"bp-row\"><div class=\"bp-addr\">Beispielkunde GmbH<br>Erika Mustermann<br>Musterstraße 1<br>12345 Musterstadt</div><div class=\"bp-info\"><span>Rechnungsnr.</span><b>{}-2026-0001</b><span>Datum</span><b>13.11.2026</b><span>Zahlbar bis</span><b>27.11.2026</b></div></div><div class=\"bp-title\">Rechnung {}-2026-0001</div><div class=\"bp-text\"><i></i><i class=\"s\"></i></div><div class=\"bp-table\"><div class=\"bp-th\"></div><div class=\"bp-tr\"></div><div class=\"bp-tr\"></div><div class=\"bp-tr\"></div></div><div class=\"bp-sum\">{qr}<div class=\"bp-total\"><span>Gesamtbetrag</span><b>297,50 €</b></div></div>{ku}<div class=\"bp-text\"><i></i><i class=\"s\"></i></div><div class=\"bp-foot\">{}</div></div>",
            esc(&absender),
            esc(&f.prefix_rechnung),
            esc(&f.prefix_rechnung),
            fuss
        )
    }

    fn nr_beispiel(&self) -> String {
        let f = &self.data.firma;
        let bsp = |p: &str| if p.trim().is_empty() { "2026-0001".to_string() } else { format!("{}-2026-0001", p.trim()) };
        format!(
            "<span class=\"pill\">Angebot <b>{}</b></span><span class=\"pill\">Rechnung <b>{}</b></span>",
            esc(&bsp(&f.prefix_angebot)),
            esc(&bsp(&f.prefix_rechnung))
        )
    }

    /// Nach einer Eingabe: Profil, Vorschau und Beispiele aktualisieren (ohne Neuzeichnen).
    pub fn settings_refresh(&self) {
        set_html("set-profile", &self.settings_profile_html());
        set_html("brief-prev", &self.brief_preview_html());
        set_html("nr-bsp", &self.nr_beispiel());
        set_html("iban-hint", &iban_pill(&self.data.firma.iban));
    }

    pub fn settings_html(&self) -> String {
        let tab = self.settings_tab();
        let mut h = head(
            "Einstellungen",
            "Dein Auftritt, deine Abläufe, deine Daten",
            &btn("preview", "eye", "Briefbogen als PDF", "ghost", None, ""),
            false,
        );
        h += "<div class=\"set-layout\"><aside class=\"set-nav\">";
        let _ = write!(h, "<div class=\"card set-profile\" id=\"set-profile\">{}</div><nav class=\"set-list\">", self.settings_profile_html());
        let offene: Vec<&str> = self.checkliste().iter().filter(|c| !c.0).map(|c| c.2).collect();
        for (i, (key, icon, title, desc)) in SECTIONS.iter().enumerate() {
            let dot = if offene.contains(key) { "<em class=\"dot\" title=\"Unvollständig\"></em>" } else { "" };
            let _ = write!(
                h,
                "<button class=\"set-item{}\" data-act=\"set-tab\" data-v=\"{key}\"><span class=\"tile t{}\">{}</span><div><b>{title}</b><span>{desc}</span></div>{dot}</button>",
                if *key == tab { " active" } else { "" },
                i % 9,
                ic(icon)
            );
        }
        h += "</nav></aside><div class=\"set-main\">";
        let (i, sec) = SECTIONS.iter().enumerate().find(|(_, s)| s.0 == tab).unwrap_or((0, &SECTIONS[0]));
        let _ = write!(
            h,
            "<div class=\"set-title\"><span class=\"tile big t{}\">{}</span><div><h2>{}</h2><p>{}</p></div></div>",
            i % 9,
            ic(sec.1),
            sec.2,
            sec.3
        );
        h += &match tab {
            "bank" => self.sec_bank(),
            "brief" => self.sec_brief(),
            "nummern" => self.sec_nummern(),
            "mahnung" => self.sec_mahnung(),
            "texte" => self.sec_texte(),
            "katalog" => self.sec_katalog(),
            "darstellung" => self.sec_darstellung(),
            "daten" => self.sec_daten(),
            _ => self.sec_firma(),
        };
        h += "</div></div>";
        h
    }

    fn sec_firma(&self) -> String {
        let f = &self.data.firma;
        let mut h = String::new();
        h += &group(
            "Unternehmen",
            "Erscheint im Briefkopf, in der Fußzeile und als Absender deiner E-Mails.",
            &format!(
                "<div class=\"form\">{}{}</div>",
                text("Firmenname", "f.name", &f.name, "placeholder=\"Muster GmbH\" autocomplete=\"organization\""),
                text("Zusatz", "f.zusatz", &f.zusatz, "placeholder=\"z. B. Inh. Max Muster\"")
            ),
        );
        h += &group(
            "Anschrift",
            "",
            &format!(
                "<div class=\"form\">{}{}</div>",
                text("Straße & Hausnr.", "f.strasse", &f.strasse, "autocomplete=\"street-address\""),
                text("PLZ & Ort", "f.plz_ort", &f.plz_ort, "placeholder=\"80331 München\"")
            ),
        );
        h += &group(
            "Kontakt",
            "",
            &format!(
                "<div class=\"form\">{}{}{}</div>",
                text("Telefon", "f.telefon", &f.telefon, "inputmode=\"tel\""),
                text("E-Mail", "f.email", &f.email, "inputmode=\"email\""),
                text("Webseite", "f.web", &f.web, "placeholder=\"www.beispiel.de\"")
            ),
        );
        h
    }

    fn sec_bank(&self) -> String {
        let f = &self.data.firma;
        let mut h = String::new();
        h += &group(
            "Bankverbindung",
            "Für die Fußzeile, den Zahlungshinweis und den GiroCode.",
            &format!(
                "<div class=\"form\">{}{}{}<div class=\"wide\" id=\"iban-hint\">{}</div></div>",
                text("IBAN", "f.iban", &f.iban, "autocapitalize=\"characters\" placeholder=\"DE00 0000 0000 0000 0000 00\""),
                text("BIC", "f.bic", &f.bic, "autocapitalize=\"characters\""),
                text("Bank", "f.bank", &f.bank, ""),
                iban_pill(&f.iban)
            ),
        );
        let mut steuer = format!(
            "<div class=\"form\">{}{}</div>{}",
            text("Steuernummer", "f.steuernummer", &f.steuernummer, "placeholder=\"143/123/45678\""),
            text("USt-IdNr.", "f.ust_id", &f.ust_id, "placeholder=\"DE123456789\""),
            toggle(
                "f.kleinunternehmer",
                f.kleinunternehmer,
                "Kleinunternehmer (§ 19 UStG)",
                "Rechnungen ohne Umsatzsteuer, mit Hinweis auf die Regelung."
            )
        );
        if !f.kleinunternehmer {
            let _ = write!(
                steuer,
                "<div class=\"opt-row\"><div><b>Standard-Steuersatz</b><span>Für neue Positionen und Buchungen</span></div>{}</div>",
                seg("ust-std", &[("19".into(), "19 %".into()), ("7".into(), "7 %".into()), ("0".into(), "0 %".into())], &f.ust_standard.to_string())
            );
        }
        h += &group("Steuern", "", &steuer);
        h += &group(
            "Bezahlen per QR-Code",
            "",
            &format!(
                "<div class=\"qr-row\"><div class=\"qr-art\">{}</div>{}</div>",
                ic("qr"),
                toggle(
                    "f.girocode",
                    f.girocode,
                    "GiroCode auf Rechnungen und Mahnungen",
                    "Kunden scannen den Code mit ihrer Banking-App – IBAN, Betrag und Verwendungszweck sind sofort ausgefüllt."
                )
            ),
        );
        h
    }

    fn sec_brief(&self) -> String {
        let f = &self.data.firma;
        let mut ctl = String::new();
        // Logo
        let logo = match &self.logo_url {
            Some(u) => format!("<div class=\"logo-prev\"><img src=\"{}\" alt=\"Logo\"></div>", esc(u)),
            None => format!("<button class=\"logo-prev empty-logo\" data-act=\"logo-add\">{}<span>Logo hochladen</span></button>", ic("image")),
        };
        let groesse: Vec<(String, String)> = LOGO_GROESSEN.iter().map(|(w, l)| (w.to_string(), l.to_string())).collect();
        let cur = LOGO_GROESSEN
            .iter()
            .min_by_key(|(w, _)| (*w as f32 - f.logo_breite_mm).abs() as i32)
            .map(|(w, _)| w.to_string())
            .unwrap_or_default();
        ctl += &group(
            "Logo",
            "PNG oder JPG, am besten mit transparentem Hintergrund.",
            &format!(
                "<div class=\"logo-row\">{logo}<div class=\"actions inline\">{}{}</div></div><div class=\"opt-row\"><div><b>Größe im Briefkopf</b><span>{:.0} mm breit</span></div>{}</div>",
                btn("logo-add", "upload", if self.logo.is_some() { "Ersetzen" } else { "Hochladen" }, "primary sm", None, ""),
                if self.logo.is_some() { btn("logo-del", "trash", "Entfernen", "ghost sm danger-text", None, "") } else { String::new() },
                f.logo_breite_mm,
                seg("logo-breite", &groesse, &cur)
            ),
        );
        // Farbe
        let mut sw = String::from("<div class=\"swatches\">");
        for (c, n) in FARBEN {
            let _ = write!(
                sw,
                "<button class=\"swatch{}\" style=\"--sw:{c}\" data-act=\"farbe\" data-v=\"{c}\" title=\"{n}\">{}</button>",
                if f.farbe.eq_ignore_ascii_case(c) { " active" } else { "" },
                ic("check")
            );
        }
        let _ = write!(
            sw,
            "<label class=\"swatch custom\" title=\"Eigene Farbe\"><input type=\"color\" data-f=\"f.farbe\" value=\"{}\">{}</label></div>",
            esc(&f.farbe),
            ic("plus")
        );
        ctl += &group("Akzentfarbe", "Für Firmenname, Tabellenkopf und Summen im PDF.", &sw);
        format!(
            "<div class=\"brief-grid\"><div>{ctl}</div><div class=\"brief-side\"><div class=\"brief-label\">Live-Vorschau</div><div class=\"brief-prev\" id=\"brief-prev\">{}</div>{}</div></div>",
            self.brief_preview_html(),
            btn("preview", "eye", "Als PDF ansehen", "ghost block", None, "")
        )
    }

    fn sec_nummern(&self) -> String {
        let f = &self.data.firma;
        let mut h = String::new();
        h += &group(
            "Nummernkreise",
            "Nummern laufen je Jahr fortlaufend und lückenlos – so wie das Finanzamt es erwartet.",
            &format!(
                "<div class=\"form\">{}{}</div><div class=\"nr-bsp\" id=\"nr-bsp\">{}</div>",
                text("Präfix Angebote", "f.prefix_angebot", &f.prefix_angebot, "autocapitalize=\"characters\""),
                text("Präfix Rechnungen", "f.prefix_rechnung", &f.prefix_rechnung, "autocapitalize=\"characters\""),
                self.nr_beispiel()
            ),
        );
        h += &group(
            "Fristen",
            "Werden bei neuen Dokumenten vorgeschlagen und lassen sich dort einzeln ändern.",
            &format!(
                "<div class=\"form\">{}{}</div>",
                suffix("Zahlungsziel Rechnungen", "f.zahlungsziel", &f.zahlungsziel.to_string(), "Tage"),
                suffix("Gültigkeit Angebote", "f.angebot_gueltig", &f.angebot_gueltig.to_string(), "Tage")
            ),
        );
        h
    }

    fn sec_mahnung(&self) -> String {
        let f = &self.data.firma;
        let mut stufen = String::from("<div class=\"stages\">");
        for (i, name) in MAHNSTUFEN.iter().enumerate() {
            let (tage, gebuehr, text_) = f.mahnstufe(i as u8);
            let _ = write!(
                stufen,
                "<div class=\"stage s{i}\"><div class=\"stage-head\"><span>{}</span><b>{name}</b></div><div class=\"form\">{}{}{}</div></div>",
                i + 1,
                suffix("Neue Frist", &format!("f.mahn_tage.{i}"), &tage.to_string(), "Tage"),
                money_f("Gebühr", &format!("f.mahn_gebuehr.{i}"), gebuehr, "placeholder=\"0,00\""),
                area("Text", &format!("f.mahn_text.{i}"), &text_, 5, "")
            );
        }
        stufen += "</div>";
        stufen += &placeholders(&["nummer", "datum", "betrag", "faellig", "gesamt", "gebuehr", "frist"]);
        let mut h = group("Mahnstufen", "Zahlungserinnerung → 1. Mahnung → 2. Mahnung. Die Briefanrede wird automatisch vorangestellt.", &stufen);
        h += &group(
            "Verzugszinsen",
            "",
            &format!(
                "{}<div class=\"form\">{}<p class=\"hint field-note\">Aktueller Wert: Deutsche Bundesbank, „Basiszinssatz“. Er ändert sich zum 1. Januar und 1. Juli.</p></div>",
                toggle(
                    "f.verzugszinsen",
                    f.verzugszinsen,
                    "Standardmäßig berechnen",
                    "Basiszinssatz + 5 Prozentpunkte (Privatkunden) bzw. + 9 Prozentpunkte (Geschäftskunden)."
                ),
                suffix("Basiszinssatz", "f.basiszins", &format!("{}", f.basiszins).replace('.', ","), "%")
            ),
        );
        h += &group(
            "E-Mail zur Mahnung",
            "Die Rechnung wird automatisch als Kopie angehängt.",
            &format!(
                "<div class=\"form\">{}{}</div>{}",
                text("Betreff", "f.mail_betreff_mahnung", &f.mail_betreff_mahnung, ""),
                area("Text", "f.mail_text_mahnung", &f.mail_text_mahnung, 6, ""),
                placeholders(&["stufe", "nummer", "datum", "gesamt", "frist", "firma", "ansprechpartner"])
            ),
        );
        h
    }

    fn sec_texte(&self) -> String {
        let f = &self.data.firma;
        let mut h = String::new();
        h += &group(
            "Angebote",
            "Die Briefanrede („Sehr geehrte Frau …“) wird automatisch vor die Einleitung gesetzt.",
            &format!(
                "<div class=\"form\">{}{}{}{}</div>{}",
                area("Einleitung", "f.einleitung_angebot", &f.einleitung_angebot, 2, ""),
                area("Schluss", "f.schluss_angebot", &f.schluss_angebot, 2, ""),
                text("E-Mail-Betreff", "f.mail_betreff_angebot", &f.mail_betreff_angebot, ""),
                area("E-Mail-Text", "f.mail_text_angebot", &f.mail_text_angebot, 6, ""),
                placeholders(&["nummer", "betrag", "gueltig", "datum", "firma", "kunde", "ansprechpartner"])
            ),
        );
        h += &group(
            "Rechnungen",
            "",
            &format!(
                "<div class=\"form\">{}{}{}{}</div>{}",
                area("Einleitung", "f.einleitung_rechnung", &f.einleitung_rechnung, 2, ""),
                area("Schluss", "f.schluss_rechnung", &f.schluss_rechnung, 2, ""),
                text("E-Mail-Betreff", "f.mail_betreff_rechnung", &f.mail_betreff_rechnung, ""),
                area("E-Mail-Text", "f.mail_text_rechnung", &f.mail_text_rechnung, 6, ""),
                placeholders(&["nummer", "betrag", "faellig", "zahlungsziel", "datum", "firma", "kunde", "ansprechpartner"])
            ),
        );
        h
    }

    fn sec_katalog(&self) -> String {
        let add = btn("kat-add", "plus", "Leistung", "primary sm", None, "");
        if self.data.katalog.is_empty() {
            return group_with(
                "Leistungen &amp; Artikel",
                "",
                &add,
                &format!(
                    "<div class=\"empty small-empty\">{}<p>Häufige Leistungen hier speichern und in Angeboten und Rechnungen mit einem Tipp einfügen.</p></div>",
                    ic("book")
                ),
            );
        }
        let mut body = String::from("<div class=\"katalog\">");
        for (i, l) in self.data.katalog.iter().enumerate() {
            let ust: Vec<(String, String)> = [19u8, 7, 0].iter().map(|r| (r.to_string(), format!("{r} %"))).collect();
            let mut opts = String::new();
            for (v, lb) in &ust {
                let _ = write!(opts, "<option value=\"{v}\"{}>{lb}</option>", if *v == l.ust.to_string() { " selected" } else { "" });
            }
            let _ = write!(
                body,
                "<div class=\"kat-row\">{}{}{}<label class=\"field\"><span>USt.</span><select data-f=\"l.{i}.ust\">{opts}</select></label><button class=\"icon-btn sm danger-text\" data-act=\"kat-del\" data-v=\"{i}\" title=\"Entfernen\">{}</button></div>",
                text("Bezeichnung", &format!("l.{i}.text"), &l.text, "placeholder=\"z. B. Arbeitsstunde\""),
                text("Einheit", &format!("l.{i}.einheit"), &l.einheit, "list=\"einheiten\""),
                money_f("Preis netto", &format!("l.{i}.preis"), l.preis, "placeholder=\"0,00\""),
                ic("trash")
            );
        }
        body += "<datalist id=\"einheiten\">";
        for e in EINHEITEN {
            let _ = write!(body, "<option value=\"{e}\">");
        }
        body += "</datalist></div>";
        group_with("Leistungen &amp; Artikel", &format!("{} Einträge", self.data.katalog.len()), &add, &body)
    }

    fn sec_darstellung(&self) -> String {
        let t = theme();
        let mut h = String::from("<div class=\"themes\">");
        for (k, l, d) in [("auto", "Automatisch", "Folgt dem Tablet"), ("light", "Hell", "Klar und freundlich"), ("dark", "Dunkel", "Schont die Augen")] {
            let _ = write!(
                h,
                "<button class=\"theme-card{}\" data-act=\"theme\" data-v=\"{k}\"><div class=\"tp tp-{k}\"><i></i><i></i><i></i></div><b>{l}</b><span>{d}</span></button>",
                if t == k { " active" } else { "" }
            );
        }
        h += "</div>";
        group("Farbschema", "", &h)
    }

    fn sec_daten(&self) -> String {
        let d = &self.data;
        let mut tiles = String::from("<div class=\"stat-tiles\">");
        for (n, l) in [
            (d.kunden.len(), "Kunden"),
            (d.auftraege.len(), "Aufträge"),
            (d.dokumente.len(), "Dokumente"),
            (d.buchungen.len(), "Buchungen"),
            (d.dateien.len(), "Dateien"),
        ] {
            let _ = write!(tiles, "<div><b>{n}</b><span>{l}</span></div>");
        }
        tiles += "</div>";
        let mut h = group("Bestand", "Alle Daten liegen nur auf diesem Gerät – ohne Konto und ohne Cloud.", &tiles);
        h += &group(
            "Backup",
            "Ein Backup enthält alles: Daten, Logo und Belege. Am besten regelmäßig in Google Drive teilen.",
            &format!(
                "<div class=\"action-tiles\">{}{}{}</div>{}",
                action_tile("backup", "download", "Speichern", "In Download/WWP"),
                action_tile("backup-share", "share", "Teilen", "Drive, Mail, …"),
                action_tile("restore", "upload", "Wiederherstellen", "Aus einer Datei"),
                if d.kunden.is_empty() && d.dokumente.is_empty() {
                    format!("<div class=\"pad-top\">{}</div>", btn("demo", "", "Beispieldaten laden", "ghost", None, ""))
                } else {
                    String::new()
                }
            ),
        );
        h += &format!(
            "<section class=\"group danger-zone\"><div class=\"group-head row-head\"><div><h3>Alle Daten löschen</h3><p>Kunden, Dokumente, Buchungen, Belege und Einstellungen werden unwiderruflich entfernt.</p></div>{}</div></section>",
            btn("wipe", "trash", "Löschen", "danger sm", None, "")
        );
        let _ = write!(h, "<p class=\"version\">WWP Büro · Rust + WebAssembly · Version {}</p>", esc(&super::app_version()));
        h
    }

    /// Vorgaben für Einstellungen per Knopf (Farbe, Logo-Größe, Steuersatz, Bereich).
    pub fn settings_action(&mut self, act: &str, v: &str) -> bool {
        match act {
            "set-tab" => {
                self.ui.settings_tab = v.to_string();
                if self.view() != super::View::Einstellungen {
                    self.go_root(super::View::Einstellungen);
                } else {
                    self.render();
                }
                if let Some(m) = super::el("main") {
                    m.set_scroll_top(0);
                }
            }
            "farbe" => {
                self.data.firma.farbe = v.to_string();
                self.changed();
                self.render();
            }
            "logo-breite" => {
                self.data.firma.logo_breite_mm = v.parse().unwrap_or(45.0);
                self.changed();
                self.render();
            }
            "ust-std" => {
                self.data.firma.ust_standard = v.parse().unwrap_or(19);
                self.changed();
                self.render();
            }
            _ => return false,
        }
        true
    }
}

fn action_tile(act: &str, icon: &str, title: &str, desc: &str) -> String {
    format!("<button class=\"action-tile\" data-act=\"{act}\">{}<b>{title}</b><span>{desc}</span></button>", ic(icon))
}

/// IBAN-Status als farbige Plakette.
pub fn iban_pill(iban: &str) -> String {
    let cls = if iban.trim().is_empty() {
        "info"
    } else if iban_valid(iban) {
        "ok"
    } else {
        "warn"
    };
    format!("<span class=\"status-pill {cls}\">{}</span>", esc(&iban_hint(iban)))
}
