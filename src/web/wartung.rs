//! Anlagen beim Kunden, digitales Wartungsprotokoll und Monteur-Modus
//! (schlanke Oberfläche ohne Preise und Finanzen, Verlassen optional mit PIN).

use super::storage::{self, load_file};
use super::views::{badge, btn, ic};
use super::{el, element_value, esc, query, today, window, App, View};
use crate::engine::anlagen::{AnlagenArt, Pruefung};
use crate::engine::date::{self, german};
use crate::engine::model::Id;
use std::fmt::Write as _;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

const LS_MONTEUR: &str = "zento-monteur";
const LS_NAME: &str = "zento-monteur-name";

fn ls_get(k: &str) -> Option<String> {
    window().local_storage().ok().flatten().and_then(|s| s.get_item(k).ok().flatten())
}
fn ls_set(k: &str, v: &str) {
    if let Ok(Some(s)) = window().local_storage() {
        let _ = s.set_item(k, v);
    }
}

/// Monteur-Modus ist auf diesem Gerät eingeschaltet?
pub fn monteur_aktiv() -> bool {
    ls_get(LS_MONTEUR).as_deref() == Some("1")
}

/// Name des Monteurs auf diesem Gerät (für neue Protokolle).
pub fn monteur_name() -> String {
    ls_get(LS_NAME).unwrap_or_default()
}

/// Ansichten, die im Monteur-Modus erlaubt sind.
pub fn erlaubt_fuer_monteur(v: View) -> bool {
    matches!(v, View::MeinTag | View::Kunden(_) | View::Wartungen | View::Anlage(_) | View::Wartung(_) | View::Kalender)
}

/// Fotos (`img[data-foto]`) nach dem Aufbau aus dem Speicher nachladen.
pub fn fotos_laden() {
    let Ok(list) = super::dom().query_selector_all("img[data-foto]") else { return };
    for i in 0..list.length() {
        let Some(img) = list.item(i).and_then(|n| n.dyn_into::<web_sys::HtmlImageElement>().ok()) else { continue };
        let Some(id) = img.get_attribute("data-foto").and_then(|v| v.parse::<Id>().ok()) else { continue };
        spawn_local(async move {
            if let Some(bytes) = load_file(id).await {
                let mime = if crate::engine::image::is_png(&bytes) { "image/png" } else { "image/jpeg" };
                if let Some(url) = storage::blob_url(&bytes, mime) {
                    img.set_src(&url);
                }
            }
        });
    }
}

fn faellig_text(n: &str, heute: &str) -> (String, &'static str) {
    let rest = date::diff_days(heute, n).unwrap_or(0);
    match rest {
        r if r < 0 => (format!("seit {} Tagen fällig", -r), "ueberfaellig"),
        0 => ("heute fällig".into(), "ueberfaellig"),
        r if r <= 30 => (format!("fällig in {r} Tagen"), "inarbeit"),
        _ => (format!("nächste {}", german(n)), "versendet"),
    }
}

impl App {
    pub fn monteur(&self) -> bool {
        monteur_aktiv()
    }

    // -----------------------------------------------------------------------
    // Anlagen am Kunden
    // -----------------------------------------------------------------------

    pub fn anlagen_card(&self, kunde: Id) -> String {
        let heute = today();
        let list = self.data.anlagen_von(kunde);
        let mut h = format!(
            "<div class=\"card\"><div class=\"card-head\"><h2>Anlagen</h2>{}</div>",
            btn("anlage-neu", "plus", "Anlage", "ghost sm", Some(kunde), "")
        );
        if list.is_empty() {
            h += "<p class=\"muted pad\">Noch keine Anlage erfasst – z. B. Gas-Brennwerttherme im Keller. Dann lässt sich die Wartung mit Protokoll durchführen und die nächste wird automatisch vorgemerkt.</p>";
        } else {
            h += "<div class=\"rows\">";
            for a in list {
                let status = match a.naechste_wartung() {
                    Some(n) => {
                        let (t, k) = faellig_text(&n, &heute);
                        badge(k, &t)
                    }
                    None if a.letzte_wartung.is_empty() => badge("entwurf", "noch keine Wartung"),
                    None => String::new(),
                };
                let _ = write!(
                    h,
                    "<div class=\"row static\"><span class=\"tile t4\">{}</span><button class=\"row-main\" data-act=\"anlage-open\" data-id=\"{}\"><b>{}</b><span>{}</span></button>{status}{}</div>",
                    ic("flame"),
                    a.id,
                    esc(&a.titel()),
                    esc(&[a.standort.trim(), a.seriennummer.trim()].iter().filter(|s| !s.is_empty()).map(|s| s.to_string()).collect::<Vec<_>>().join(" · ")),
                    btn("wa-start", "wrench", "Wartung", "primary sm", Some(a.id), "")
                );
            }
            h += "</div>";
        }
        h += "</div>";
        h
    }

    pub fn anlage_html(&self, id: Id) -> String {
        let d = &self.data;
        let Some(a) = d.anlage(Some(id)) else { return super::views::empty("flame", "Anlage nicht gefunden", "", "") };
        let heute = today();
        let kunde = d.kunde(a.kunde_id);
        let sub = match a.naechste_wartung() {
            Some(n) => format!("{} · {}", esc(&d.kunden_name(a.kunde_id)), faellig_text(&n, &heute).0),
            None => esc(&d.kunden_name(a.kunde_id)),
        };
        let mut h = super::views::head(&a.titel(), &sub, &btn("wa-start", "wrench", "Wartung durchführen", "primary", Some(id), ""), true);
        h += "<div class=\"editor\"><div class=\"ed-main\"><div class=\"card\"><div class=\"form\">";
        let mut opts = String::new();
        for art in AnlagenArt::ALLE {
            let _ = write!(opts, "<option value=\"{}\"{}>{}</option>", art.key(), if art == a.art { " selected" } else { "" }, art.label());
        }
        let _ = write!(h, "<label class=\"field\"><span>Art der Anlage</span><select data-f=\"an.art\">{opts}</select></label>");
        h += &super::views::text("Hersteller", "an.hersteller", &a.hersteller, "placeholder=\"z. B. Vaillant\"");
        h += &super::views::text("Modell", "an.modell", &a.modell, "placeholder=\"z. B. ecoTEC plus\"");
        h += &super::views::text("Seriennummer", "an.seriennummer", &a.seriennummer, "autocapitalize=\"characters\"");
        h += &super::views::text("Baujahr", "an.baujahr", &a.baujahr, "inputmode=\"numeric\" placeholder=\"z. B. 2018\"");
        h += &super::views::text("Leistung", "an.leistung", &a.leistung, "placeholder=\"z. B. 20 kW\"");
        h += &super::views::text("Standort", "an.standort", &a.standort, "placeholder=\"z. B. Keller, Heizraum\"");
        let _ = write!(
            h,
            "<label class=\"field\"><span>Letzte Wartung</span><input type=\"date\" data-f=\"an.letzte_wartung\" value=\"{}\"></label>",
            esc(&a.letzte_wartung)
        );
        let mut iv = String::new();
        for (m, l) in [(6u32, "alle 6 Monate"), (12, "jährlich"), (24, "alle 2 Jahre"), (0, "keine Erinnerung")] {
            let _ = write!(iv, "<option value=\"{m}\"{}>{l}</option>", if m == a.intervall { " selected" } else { "" });
        }
        let _ = write!(h, "<label class=\"field\"><span>Wartungsintervall</span><select data-f=\"an.intervall\">{iv}</select></label>");
        h += &super::views::area("Notizen", "an.notizen", &a.notizen, 3, "placeholder=\"z. B. Schlüssel beim Nachbarn, Hund im Garten …\"");
        h += "</div></div>";
        // Historie
        let ws = d.wartungen_von(id);
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Wartungen</h2></div>";
        if ws.is_empty() {
            h += "<p class=\"muted pad\">Noch kein Protokoll – „Wartung durchführen“ startet die Checkliste.</p>";
        } else {
            h += "<div class=\"rows\">";
            for w in ws {
                h += &self.wartung_row(w.id);
            }
            h += "</div>";
        }
        h += "</div></div><aside class=\"ed-side\">";
        // Typenschild
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Typenschild</h2></div>";
        match a.foto {
            Some(fid) => {
                let _ = write!(
                    h,
                    "<div class=\"foto\"><img data-foto=\"{fid}\" alt=\"Typenschild\"></div><div class=\"actions\"><div class=\"btn-pair\">{}{}</div></div>",
                    btn("foto-add", "camera", "Neu", "ghost", Some(id), ""),
                    btn("foto-del", "trash", "Entfernen", "ghost danger-text", Some(id), "")
                );
            }
            None => {
                let _ = write!(
                    h,
                    "<button class=\"foto leer\" data-act=\"foto-add\" data-id=\"{id}\">{}<b>Foto aufnehmen</b><span>Typenschild fotografieren – Seriennummer immer griffbereit</span></button>",
                    ic("camera")
                );
            }
        }
        h += "</div>";
        // Kunde
        if let Some(k) = kunde {
            let _ = write!(
                h,
                "<div class=\"card\"><div class=\"card-head\"><h2>Kunde</h2></div><button class=\"row\" data-act=\"kunde-open\" data-id=\"{}\"><div><b>{}</b><span>{}</span></div>{}</button><div class=\"actions\">{}</div></div>",
                k.id,
                esc(&k.name()),
                esc(&[k.strasse.trim(), k.ort.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(", ")),
                ic("chevron"),
                btn("kunde-route", "map", "Navigation starten", "ghost block", Some(k.id), "")
            );
        }
        if !self.monteur() {
            let _ = write!(h, "<div class=\"card actions\">{}</div>", btn("anlage-del", "trash", "Anlage löschen", "ghost block danger-text", Some(id), ""));
        }
        h += "</aside></div>";
        h
    }

    fn wartung_row(&self, wid: Id) -> String {
        let d = &self.data;
        let Some(w) = d.wartung(Some(wid)) else { return String::new() };
        let a = d.anlage(w.anlage_id);
        let maengel = w.anzahl_maengel();
        let status = if !w.abgeschlossen {
            badge("inarbeit", "in Arbeit")
        } else if maengel > 0 {
            badge("ueberfaellig", &if maengel == 1 { "1 Mangel".to_string() } else { format!("{maengel} Mängel") })
        } else {
            badge("bezahlt", "ohne Mängel")
        };
        format!(
            "<button class=\"row\" data-act=\"wa-open\" data-id=\"{wid}\"><div><b>{} · {}</b><span>{} · {}</span></div><div class=\"row-r\">{status}{}</div></button>",
            esc(&w.nummer),
            german(&w.datum),
            esc(&d.kunden_name(w.kunde_id)),
            esc(&a.map(|a| a.titel()).unwrap_or_else(|| "Anlage gelöscht".into())),
            ic("chevron")
        )
    }

    // -----------------------------------------------------------------------
    // Wartungsprotokoll
    // -----------------------------------------------------------------------

    pub fn wartung_html(&self, id: Id) -> String {
        let d = &self.data;
        let Some(w) = d.wartung(Some(id)) else { return super::views::empty("wrench", "Protokoll nicht gefunden", "", "") };
        let a = d.anlage(w.anlage_id);
        let ds = if w.abgeschlossen { " disabled" } else { "" };
        let status = if w.abgeschlossen { badge("bezahlt", "Abgeschlossen") } else { badge("inarbeit", "In Arbeit") };
        let mut h = super::views::head(
            &format!("Wartungsprotokoll {}", w.nummer),
            &format!("{} · {}", esc(&d.kunden_name(w.kunde_id)), esc(&a.map(|a| a.titel()).unwrap_or_default())),
            &status,
            true,
        );
        if w.abgeschlossen {
            let _ = write!(
                h,
                "<div class=\"banner\">{}<div>Das Protokoll ist <b>abgeschlossen</b>. Für Korrekturen wieder öffnen.</div>{}</div>",
                ic("lock"),
                btn("wa-oeffnen", "unlock", "Wieder öffnen", "ghost", Some(id), "")
            );
        }
        h += "<div class=\"editor\"><div class=\"ed-main\">";
        // Prüfpunkte
        let geprueft = w.pruefpunkte.iter().filter(|p| p.status != Pruefung::Offen).count();
        let _ = write!(
            h,
            "<div class=\"card\"><div class=\"card-head\"><h2>Prüfpunkte</h2><div><span class=\"muted small pp-zaehler\">{geprueft} von {} geprüft</span>{}</div></div><div class=\"pp-list\">",
            w.pruefpunkte.len(),
            if w.abgeschlossen { String::new() } else { btn("wp-alle", "check", "Rest OK", "ghost sm", Some(id), "") }
        );
        for (i, p) in w.pruefpunkte.iter().enumerate() {
            let cls = match p.status {
                Pruefung::Ok => "ok",
                Pruefung::Mangel => "mangel",
                Pruefung::Entfaellt => "na",
                Pruefung::Offen => "",
            };
            let knopf = |s: &str, label: &str, aktiv: bool| {
                format!("<button class=\"{s}{}\" data-act=\"wp\" data-id=\"{id}\" data-v=\"{i}:{s}\"{ds}>{label}</button>", if aktiv { " active" } else { "" })
            };
            let _ = write!(
                h,
                "<div class=\"pp {cls}\"><b>{}</b><div class=\"pp-seg\">{}{}{}</div>{}</div>",
                esc(&p.text),
                knopf("ok", "OK", p.status == Pruefung::Ok),
                knopf("mangel", "Mangel", p.status == Pruefung::Mangel),
                knopf("na", "–", p.status == Pruefung::Entfaellt),
                if p.status == Pruefung::Mangel || !p.notiz.is_empty() {
                    format!("<input class=\"pp-notiz\" data-f=\"wp.{i}\" value=\"{}\" placeholder=\"Was ist aufgefallen? Was wurde gemacht?\"{ds}>", esc(&p.notiz))
                } else {
                    String::new()
                }
            );
        }
        h += "</div></div>";
        // Messwerte
        if !w.messwerte.is_empty() {
            h += "<div class=\"card\"><div class=\"card-head\"><h2>Messwerte</h2></div><div class=\"form\">";
            for (i, m) in w.messwerte.iter().enumerate() {
                let _ = write!(
                    h,
                    "<label class=\"field\"><span>{}</span><div class=\"euro unit\"><input type=\"text\" inputmode=\"decimal\" data-f=\"wm.{i}\" value=\"{}\"{ds}><i>{}</i></div></label>",
                    esc(&m.name),
                    esc(&m.wert),
                    esc(&m.einheit)
                );
            }
            h += "</div></div>";
        }
        // Arbeiten, Mängel
        h += "<div class=\"card\"><div class=\"form\">";
        h += &super::views::area("Durchgeführte Arbeiten / Bemerkungen", "wa.arbeiten", &w.arbeiten, 3, &format!("placeholder=\"z. B. Wartung nach Herstellervorgabe, Brenner gereinigt …\"{ds}"));
        h += &super::views::area("Festgestellte Mängel / Empfehlungen", "wa.maengel", &w.maengel, 3, &format!("placeholder=\"z. B. Ausdehnungsgefäß tauschen\"{ds}"));
        h += "</div></div></div><aside class=\"ed-side\">";
        // Seite: Angaben + Aktionen
        let _ = write!(
            h,
            "<div class=\"card\"><div class=\"form\"><label class=\"field\"><span>Datum</span><input type=\"date\" data-f=\"wa.datum\" value=\"{}\"{ds}></label><label class=\"field\"><span>Monteur</span><input type=\"text\" data-f=\"wa.monteur\" value=\"{}\" placeholder=\"Name\"{ds}></label></div></div>",
            esc(&w.datum),
            esc(&w.monteur)
        );
        h += "<div class=\"card\"><div class=\"actions\">";
        if !w.abgeschlossen {
            if w.unterschrift.is_none() {
                h += &btn("sig-wa", "pen", "Kunde unterschreiben lassen", "success block", Some(id), "");
            }
            h += &btn("wa-fertig", "check", "Protokoll abschließen", "primary block", Some(id), "");
        } else {
            h += &btn("wa-send", "send", "An Kunden senden", "primary block", Some(id), "");
        }
        let _ = write!(
            h,
            "<div class=\"btn-pair\">{}{}</div>",
            btn("wa-pdf", "eye", "Ansehen", "ghost", Some(id), ""),
            btn("wa-save", "download", "Speichern", "ghost", Some(id), "")
        );
        if !self.monteur() {
            match w.rechnung_id.filter(|r| d.dokument(Some(*r)).is_some()) {
                Some(r) => h += &btn("dok-open", "receipt", "Rechnung öffnen", "ghost block", Some(r), ""),
                None if w.abgeschlossen => h += &btn("wa-rechnung", "receipt", "Rechnung erstellen", "ghost block", Some(id), ""),
                None => {}
            }
            let _ = write!(
                h,
                "<details class=\"more\"><summary>{}Weitere Aktionen</summary><div class=\"actions\">{}</div></details>",
                ic("chevron"),
                btn("wa-del", "trash", "Protokoll löschen", "ghost block danger-text", Some(id), "")
            );
        }
        h += "</div></div>";
        if let Some(u) = &w.unterschrift {
            let unter = [u.name.trim().to_string(), u.ort.trim().to_string(), german(&u.datum)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
            let _ = write!(
                h,
                "<div class=\"card\"><div class=\"card-head\"><h2>Unterschrift Kunde</h2>{}</div><div class=\"sig-show\">{}</div><p class=\"sig-meta\">{} {}</p></div>",
                if w.abgeschlossen { String::new() } else { format!("<button class=\"icon-btn sm danger-text\" data-act=\"sig-wa-del\" data-id=\"{id}\" title=\"Unterschrift entfernen\">{}</button>", ic("trash")) },
                super::signature::sig_svg(u),
                ic("check"),
                esc(&unter)
            );
        }
        if let Some(a) = a {
            let _ = write!(
                h,
                "<div class=\"card\"><div class=\"card-head\"><h2>Anlage</h2></div><button class=\"row\" data-act=\"anlage-open\" data-id=\"{}\"><div><b>{}</b><span>{}</span></div>{}</button></div>",
                a.id,
                esc(&a.titel()),
                esc(&[a.seriennummer.trim(), a.standort.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ")),
                ic("chevron")
            );
        }
        h += "</aside></div>";
        h
    }

    // -----------------------------------------------------------------------
    // Listen: Wartungen und „Mein Tag“
    // -----------------------------------------------------------------------

    fn faellige(&self, heute: &str, tage: i64) -> Vec<(String, Id)> {
        let mut v: Vec<(String, Id)> = self
            .data
            .anlagen
            .iter()
            .filter_map(|a| a.naechste_wartung().filter(|n| date::diff_days(heute, n).unwrap_or(999) <= tage).map(|n| (n, a.id)))
            .collect();
        v.sort();
        v
    }

    fn faellig_rows(&self, liste: &[(String, Id)], heute: &str) -> String {
        let d = &self.data;
        let mut h = String::from("<div class=\"rows\">");
        for (n, aid) in liste {
            let Some(a) = d.anlage(Some(*aid)) else { continue };
            let (t, k) = faellig_text(n, heute);
            let _ = write!(
                h,
                "<div class=\"row static\"><span class=\"tile t4\">{}</span><button class=\"row-main\" data-act=\"anlage-open\" data-id=\"{aid}\"><b>{}</b><span>{} · {}</span></button>{}{}</div>",
                ic("flame"),
                esc(&d.kunden_name(a.kunde_id)),
                esc(&a.titel()),
                esc(a.standort.trim()),
                badge(k, &t),
                btn("wa-start", "wrench", "Starten", "primary sm", Some(*aid), "")
            );
        }
        h += "</div>";
        h
    }

    pub fn wartungen_html(&self) -> String {
        let d = &self.data;
        let heute = today();
        let mut h = super::views::head("Wartungen", &format!("{} Anlagen · {} Protokolle", d.anlagen.len(), d.wartungen.len()), "", false);
        if d.anlagen.is_empty() {
            h += &super::views::empty(
                "wrench",
                "Noch keine Anlagen",
                "Anlagen legst du beim Kunden an: Kunde öffnen → + Anlage. Danach startest du hier die Wartung mit Checkliste, Messwerten und Unterschrift.",
                &btn("nav", "users", "Zu den Kunden", "primary", None, "kunden"),
            );
            return h;
        }
        let faellig = self.faellige(&heute, 30);
        let _ = write!(h, "<div class=\"card\"><div class=\"card-head\"><h2>Fällig in den nächsten 30 Tagen</h2><span class=\"count-pill\">{}</span></div>", faellig.len());
        if faellig.is_empty() {
            h += "<p class=\"muted pad\">Keine Wartung fällig.</p>";
        } else {
            h += &self.faellig_rows(&faellig, &heute);
        }
        h += "</div><section class=\"grid2\">";
        // alle Anlagen
        let mut alle: Vec<_> = d.anlagen.iter().collect();
        alle.sort_by_key(|a| (a.naechste_wartung().unwrap_or_else(|| "9999".into()), d.kunden_name(a.kunde_id)));
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Alle Anlagen</h2></div><div class=\"rows\">";
        for a in alle {
            let status = match a.naechste_wartung() {
                Some(n) => {
                    let (t, k) = faellig_text(&n, &heute);
                    badge(k, &t)
                }
                None => badge("entwurf", "keine Wartung erfasst"),
            };
            let _ = write!(
                h,
                "<button class=\"row\" data-act=\"anlage-open\" data-id=\"{}\"><div><b>{}</b><span>{}</span></div><div class=\"row-r\">{status}</div></button>",
                a.id,
                esc(&d.kunden_name(a.kunde_id)),
                esc(&a.titel())
            );
        }
        h += "</div></div>";
        // letzte Protokolle
        let mut ws: Vec<_> = d.wartungen.iter().collect();
        ws.sort_by(|a, b| b.datum.cmp(&a.datum).then(b.id.cmp(&a.id)));
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Letzte Protokolle</h2></div>";
        if ws.is_empty() {
            h += "<p class=\"muted pad\">Noch keine Protokolle.</p>";
        } else {
            h += "<div class=\"rows\">";
            for w in ws.iter().take(15) {
                h += &self.wartung_row(w.id);
            }
            h += "</div>";
        }
        h += "</div></section>";
        h
    }

    pub fn mein_tag_html(&self) -> String {
        let d = &self.data;
        let heute = today();
        let wt = ["Sonntag", "Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag"][(date::to_days(&heute).unwrap_or(0) + 4).rem_euclid(7) as usize];
        let name = monteur_name();
        let gruss = if name.trim().is_empty() { "Mein Tag".to_string() } else { format!("Hallo {}", name.trim()) };
        let mut h = super::views::head(&gruss, &format!("{wt}, {}", german(&heute)), &btn("suche", "search", "Kunde suchen", "ghost", None, ""), false);
        if name.trim().is_empty() {
            let _ = write!(
                h,
                "<div class=\"banner\">{}<div><b>Wie heißt du?</b> Dein Name kommt auf die Wartungsprotokolle.</div><input class=\"name-in\" type=\"text\" data-f=\"mt.name\" placeholder=\"Vor- und Nachname\"></div>",
                ic("users")
            );
        }
        // Termine heute
        let termine = d.termine_am(&heute);
        h += "<section class=\"grid2\"><div class=\"card\"><div class=\"card-head\"><h2>Termine heute</h2></div>";
        if termine.is_empty() {
            h += "<p class=\"muted pad\">Heute keine Termine eingetragen.</p>";
        } else {
            h += "<div class=\"rows\">";
            for t in termine {
                h += &super::kalender::termin_row(self, t);
            }
            h += "</div>";
        }
        h += "</div>";
        // Fällige Wartungen
        let faellig = self.faellige(&heute, 30);
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Fällige Wartungen</h2></div>";
        if faellig.is_empty() {
            h += "<p class=\"muted pad\">Keine Wartung fällig.</p>";
        } else {
            h += &self.faellig_rows(&faellig.into_iter().take(8).collect::<Vec<_>>(), &heute);
        }
        h += "</div>";
        // Protokolle heute
        let heutige: Vec<_> = d.wartungen.iter().filter(|w| w.datum == heute).collect();
        h += "<div class=\"card\"><div class=\"card-head\"><h2>Protokolle heute</h2></div>";
        if heutige.is_empty() {
            h += "<p class=\"muted pad\">Noch kein Protokoll heute. Kunde öffnen → Anlage → „Wartung“.</p>";
        } else {
            h += "<div class=\"rows\">";
            for w in heutige {
                h += &self.wartung_row(w.id);
            }
            h += "</div>";
        }
        h += "</div></section>";
        h
    }

    // -----------------------------------------------------------------------
    // Aktionen und Felder
    // -----------------------------------------------------------------------

    fn monteur_setzen(&mut self, an: bool) {
        ls_set(LS_MONTEUR, if an { "1" } else { "0" });
        self.close_modal();
        if an {
            self.go_root(View::MeinTag);
            self.toast("Monteur-Modus: ohne Preise und Finanzen");
        } else {
            self.go_root(View::Start);
            self.toast("Monteur-Modus beendet");
        }
    }

    pub fn wartung_action(&mut self, act: &str, id: Option<Id>, v: &str) -> bool {
        let heute = today();
        match act {
            "anlage-neu" => {
                let kunde = id.or(match self.view() {
                    View::Kunden(k) => k,
                    _ => None,
                });
                let a = self.data.neue_anlage(kunde);
                self.changed();
                self.go(View::Anlage(a));
            }
            "anlage-open" => {
                if let Some(a) = id {
                    self.go(View::Anlage(a));
                }
            }
            "anlage-del" => {
                let Some(a) = id else { return true };
                self.ask("Anlage löschen?", "Die Anlage wird entfernt. Vorhandene Wartungsprotokolle bleiben erhalten.", "Löschen", true, move |app| {
                    if let Some(f) = app.data.anlage(Some(a)).and_then(|x| x.foto) {
                        app.drop_file(f);
                    }
                    app.data.anlage_loeschen(a);
                    app.changed();
                    app.back();
                });
            }
            "kunde-route" => {
                if let Some(k) = self.data.kunde(id) {
                    let ziel = format!("{}, {} {}", k.strasse.trim(), k.plz.trim(), k.ort.trim());
                    let url = format!("https://www.google.com/maps/dir/?api=1&destination={}", String::from(js_sys::encode_uri_component(ziel.trim_matches([',', ' ']))));
                    storage::open_url(&url);
                }
            }
            "foto-add" => {
                if id.is_some() {
                    storage::pick_file("file-foto");
                }
            }
            "foto-del" => {
                let Some(a) = id else { return true };
                if let Some(f) = self.data.anlage_mut(a).and_then(|x| x.foto.take()) {
                    self.drop_file(f);
                }
                self.changed();
                self.render();
            }
            "wa-start" => {
                let Some(a) = id else { return true };
                let name = monteur_name();
                if let Some(w) = self.data.wartung_starten(a, &name, &heute) {
                    self.changed();
                    self.go(View::Wartung(w));
                }
            }
            "wa-open" => {
                if let Some(w) = id {
                    self.go(View::Wartung(w));
                }
            }
            "wp" => {
                let Some(w) = id else { return true };
                let (i, s) = v.split_once(':').unwrap_or(("", ""));
                let Ok(i) = i.parse::<usize>() else { return true };
                if let Some(p) = self.data.wartung_mut(w).filter(|x| !x.abgeschlossen).and_then(|x| x.pruefpunkte.get_mut(i)) {
                    let neu = match s {
                        "ok" => Pruefung::Ok,
                        "mangel" => Pruefung::Mangel,
                        _ => Pruefung::Entfaellt,
                    };
                    p.status = if p.status == neu { Pruefung::Offen } else { neu };
                    let mangel = p.status == Pruefung::Mangel;
                    self.changed();
                    self.render();
                    if mangel {
                        if let Some(e) = query(&format!("[data-f='wp.{i}']")).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                            let _ = e.focus();
                        }
                    }
                }
            }
            "wp-alle" => {
                let Some(w) = id else { return true };
                if let Some(x) = self.data.wartung_mut(w) {
                    for p in x.pruefpunkte.iter_mut().filter(|p| p.status == Pruefung::Offen) {
                        p.status = Pruefung::Ok;
                    }
                }
                self.changed();
                self.render();
            }
            "wa-fertig" => {
                let Some(w) = id else { return true };
                let Some(x) = self.data.wartung(Some(w)) else { return true };
                let offen = x.offene_punkte();
                let mut hinweise = Vec::new();
                if offen > 0 {
                    hinweise.push(format!("{offen} Prüfpunkt(e) sind noch offen und werden als „entfällt“ vermerkt."));
                }
                if x.unterschrift.is_none() {
                    hinweise.push("Der Kunde hat noch nicht unterschrieben.".to_string());
                }
                let fertig = move |a: &mut App| {
                    a.data.wartung_abschliessen(w);
                    a.changed();
                    a.render();
                    a.toast("Protokoll abgeschlossen – nächste Wartung vorgemerkt");
                };
                if hinweise.is_empty() {
                    fertig(self);
                } else {
                    self.ask("Protokoll abschließen?", &hinweise.join(" "), "Trotzdem abschließen", false, fertig);
                }
            }
            "wa-oeffnen" => {
                if let Some(x) = id.and_then(|w| self.data.wartung_mut(w)) {
                    x.abgeschlossen = false;
                }
                self.changed();
                self.render();
            }
            "wa-pdf" | "wa-save" | "wa-send" => {
                let Some(w) = id.and_then(|w| self.data.wartung(Some(w))).cloned() else { return true };
                let bytes = crate::engine::pdf::wartung_pdf(&self.data, &w, self.logo.as_deref());
                let kunde = self.data.kunde(w.kunde_id).cloned();
                let name = format!("Wartungsprotokoll_{}.pdf", w.nummer);
                match act {
                    "wa-pdf" => storage::open_file(&bytes, "application/pdf", &name),
                    "wa-save" => storage::save_file(&bytes, "application/pdf", &name),
                    _ => {
                        let an = kunde.as_ref().map(|k| k.email.trim().to_string()).unwrap_or_default();
                        let anrede = kunde.as_ref().map(crate::engine::model::briefanrede).unwrap_or_else(|| "Sehr geehrte Damen und Herren,".into());
                        let firma = self.data.firma.name.trim().to_string();
                        let text = format!(
                            "{anrede}\n\nanbei erhalten Sie das Protokoll zur Wartung Ihrer Anlage vom {}.\n\nMit freundlichen Grüßen\n{}",
                            german(&w.datum),
                            if w.monteur.trim().is_empty() { firma.clone() } else { format!("{}\n{firma}", w.monteur.trim()) }
                        );
                        storage::share_file(&bytes, "application/pdf", &name, &an, &format!("Wartungsprotokoll {} – {firma}", w.nummer), &text);
                    }
                }
            }
            "wa-rechnung" => {
                let Some(w) = id else { return true };
                if let Some(r) = self.data.wartung_rechnung(w, &heute) {
                    self.changed();
                    self.go(View::Dokument(r));
                    self.toast("Rechnung zur Wartung erstellt");
                }
            }
            "wa-del" => {
                let Some(w) = id else { return true };
                self.ask("Protokoll löschen?", "Das Wartungsprotokoll wird endgültig gelöscht.", "Löschen", true, move |a| {
                    a.data.wartungen.retain(|x| x.id != w);
                    a.changed();
                    a.back();
                });
            }
            "sig-wa" => {
                if let Some(w) = id {
                    self.sig_open_wartung(w);
                }
            }
            "sig-wa-del" => {
                if let Some(x) = id.and_then(|w| self.data.wartung_mut(w)) {
                    x.unterschrift = None;
                }
                self.changed();
                self.render();
            }
            "monteur-an" => self.monteur_setzen(true),
            "monteur-aus" => {
                if self.data.firma.monteur_pin.trim().is_empty() {
                    self.monteur_setzen(false);
                } else {
                    let html = format!(
                        "<h3>Monteur-Modus beenden</h3><p class=\"muted\">Bitte die PIN eingeben.</p><div class=\"form\"><label class=\"field wide\"><span>PIN</span><input type=\"password\" id=\"pin\" inputmode=\"numeric\" autocomplete=\"off\" autofocus></label></div><div class=\"modal-actions\">{}{}</div>",
                        btn("modal-close", "", "Abbrechen", "ghost", None, ""),
                        btn("monteur-pin", "unlock", "Beenden", "primary", None, "").replacen("<button ", "<button data-enter ", 1)
                    );
                    self.modal(&html);
                }
            }
            "monteur-pin" => {
                let pin = query("#pin").map(|e| element_value(&e)).unwrap_or_default();
                if pin.trim() == self.data.firma.monteur_pin.trim() {
                    self.monteur_setzen(false);
                } else {
                    self.toast("PIN falsch");
                    if let Some(e) = el("pin") {
                        let _ = e.focus();
                    }
                }
            }
            _ => return false,
        }
        true
    }

    /// Felder: `an.*` Anlage, `wa.*` Protokoll, `wp.<i>` Notiz zum Prüfpunkt, `wm.<i>` Messwert, `mt.name`.
    pub fn wartung_feld(&mut self, scope: &str, rest: &str, v: &str) -> bool {
        let view = self.view();
        match (scope, view) {
            ("mt", _) => {
                if rest == "name" {
                    ls_set(LS_NAME, v.trim());
                }
                return true;
            }
            ("an", View::Anlage(id)) => {
                let Some(a) = self.data.anlage_mut(id) else { return true };
                let s = v.to_string();
                let mut neu = false;
                match rest {
                    "art" => {
                        a.art = AnlagenArt::from_key(v).unwrap_or_default();
                        neu = true;
                    }
                    "hersteller" => a.hersteller = s,
                    "modell" => a.modell = s,
                    "seriennummer" => a.seriennummer = s.to_uppercase(),
                    "baujahr" => a.baujahr = s,
                    "leistung" => a.leistung = s,
                    "standort" => a.standort = s,
                    "letzte_wartung" => {
                        a.letzte_wartung = s;
                        neu = true;
                    }
                    "intervall" => {
                        a.intervall = v.parse().unwrap_or(12);
                        neu = true;
                    }
                    "notizen" => a.notizen = s,
                    _ => return true,
                }
                let titel = a.titel();
                self.changed();
                if neu {
                    self.render();
                } else if let Some(h1) = query(".page-head h1") {
                    h1.set_text_content(Some(&titel));
                }
            }
            ("wa" | "wp" | "wm", View::Wartung(id)) => {
                let Some(w) = self.data.wartung_mut(id) else { return true };
                if w.abgeschlossen {
                    return true;
                }
                match scope {
                    "wa" => match rest {
                        "datum" => w.datum = v.to_string(),
                        "monteur" => {
                            w.monteur = v.to_string();
                            ls_set(LS_NAME, v.trim());
                        }
                        "arbeiten" => w.arbeiten = v.to_string(),
                        "maengel" => w.maengel = v.to_string(),
                        _ => return true,
                    },
                    "wp" => {
                        if let Some(p) = rest.parse::<usize>().ok().and_then(|i| w.pruefpunkte.get_mut(i)) {
                            p.notiz = v.to_string();
                        }
                    }
                    _ => {
                        if let Some(m) = rest.parse::<usize>().ok().and_then(|i| w.messwerte.get_mut(i)) {
                            m.wert = v.trim().to_string();
                        }
                    }
                }
                self.changed();
            }
            _ => return false,
        }
        true
    }
}
