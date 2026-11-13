//! Übersicht: „Heute zu tun“ – automatisch ermittelte Aufgaben mit direktem Knopf.

use super::views::{btn, ic};
use super::{esc, App};
use crate::engine::aufgaben::{Typ, BACKUP_TAGE};
use crate::engine::date::german;
use crate::engine::model::MAHNSTUFEN;
use crate::engine::money::fmt_eur;
use crate::engine::zeit::dauer;
use std::fmt::Write as _;

fn tage(n: i64) -> String {
    if n == 1 { "1 Tag".into() } else { format!("{n} Tagen") }
}

impl App {
    pub fn heute_card(&self, heute: &str) -> String {
        let d = &self.data;
        let liste = d.aufgaben(heute);
        let mut rows = String::new();
        let max = 7;
        for x in liste.iter().take(max) {
            let dok = d.dokument(x.id);
            let kunde = |id| d.kunden_name(id);
            // (Symbol, Farbe, Titel, Text, Knopf)
            let (icon, farbe, titel, text, knopf): (&str, &str, String, String, String) = match x.typ {
                Typ::Uhr => {
                    let a = d.auftrag(x.id);
                    let start = a.and_then(|a| a.uhr_start).unwrap_or(0.0);
                    (
                        "clock",
                        "live",
                        format!("Stoppuhr läuft · <b data-uhr=\"{start}\">…</b>"),
                        esc(&a.map(|a| a.titel()).unwrap_or_default()),
                        btn("uhr-stopp", "x", "Stopp", "danger sm", x.id, ""),
                    )
                }
                Typ::Mahnen => {
                    let dk = dok.unwrap();
                    (
                        "bell",
                        "rot",
                        format!("{} {} mahnen", esc(&dk.nummer), esc(&kunde(dk.kunde_id))),
                        format!("{} überfällig · {} offen", tage(x.zahl), esc(&fmt_eur(dk.rest()))),
                        btn("mahn-new", "bell", MAHNSTUFEN[dk.naechste_stufe() as usize], "warn sm", x.id, ""),
                    )
                }
                Typ::Ueberfaellig => {
                    let dk = dok.unwrap();
                    (
                        "alert",
                        "rot",
                        format!("{} ist überfällig", esc(&dk.nummer)),
                        format!("{} · seit {} · {}", esc(&kunde(dk.kunde_id)), tage(x.zahl), esc(&fmt_eur(dk.rest()))),
                        btn("dok-open", "", "Öffnen", "ghost sm", x.id, ""),
                    )
                }
                Typ::TerminHeute | Typ::TerminMorgen => {
                    let t = d.termine.iter().find(|t| Some(t.id) == x.id);
                    let zeit = t.map(|t| if t.von.is_empty() { "ganztägig".to_string() } else { t.von.clone() }).unwrap_or_default();
                    (
                        "calendar",
                        "blau",
                        format!("{} {} · {}", if x.typ == Typ::TerminHeute { "Heute" } else { "Morgen" }, esc(&zeit), esc(&t.map(|t| t.titel.clone()).unwrap_or_default())),
                        esc(&t.map(|t| if t.ort.is_empty() { kunde(t.kunde_id) } else { t.ort.clone() }).unwrap_or_default()),
                        btn("termin-open", "", "Öffnen", "ghost sm", x.id, ""),
                    )
                }
                Typ::Frist => {
                    let a = d.auftrag(x.id);
                    let wann = match x.zahl {
                        n if n < 0 => format!("Frist seit {} verpasst", tage(-n)),
                        0 => "Frist ist heute".into(),
                        1 => "Frist ist morgen".into(),
                        n => format!("Frist in {n} Tagen"),
                    };
                    (
                        "briefcase",
                        if x.dringend { "rot" } else { "gelb" },
                        format!("{} – {}", esc(&a.map(|a| a.titel()).unwrap_or_default()), wann),
                        format!("{} · {}", esc(&kunde(a.and_then(|a| a.kunde_id))), german(&a.map(|a| a.termin.clone()).unwrap_or_default())),
                        btn("auftrag-open", "", "Öffnen", "ghost sm", x.id, ""),
                    )
                }
                Typ::Nachfassen => {
                    let dk = dok.unwrap();
                    (
                        "send",
                        "gelb",
                        format!("Angebot {} nachfassen", esc(&dk.nummer)),
                        format!("{} · seit {} ohne Antwort · {}", esc(&kunde(dk.kunde_id)), tage(x.zahl), esc(&fmt_eur(dk.summen().brutto))),
                        btn("dok-open", "", "Öffnen", "ghost sm", x.id, ""),
                    )
                }
                Typ::Abgelaufen => {
                    let dk = dok.unwrap();
                    (
                        "clock",
                        "grau",
                        format!("Angebot {} ist abgelaufen", esc(&dk.nummer)),
                        format!("{} · gültig bis {}", esc(&kunde(dk.kunde_id)), german(&dk.faellig)),
                        btn("dok-open", "", "Öffnen", "ghost sm", x.id, ""),
                    )
                }
                Typ::Zeiten => {
                    let a = d.auftrag(x.id);
                    (
                        "clock",
                        "lila",
                        format!("{} Arbeitszeit abrechnen", dauer(x.zahl as u32)),
                        esc(&a.map(|a| a.titel()).unwrap_or_default()),
                        btn("zeit-rechnung", "receipt", "Abrechnen", "primary sm", x.id, ""),
                    )
                }
                Typ::Entwurf => {
                    let dk = dok.unwrap();
                    (
                        "file",
                        "grau",
                        format!("{} {} ist noch nicht versendet", esc(&dk.titel()), esc(&dk.nummer)),
                        format!("{} · {}", esc(&kunde(dk.kunde_id)), esc(&fmt_eur(dk.summen().brutto))),
                        btn("dok-open", "", "Öffnen", "ghost sm", x.id, ""),
                    )
                }
                Typ::Anfragen => (
                    "users",
                    "blau",
                    if x.zahl == 1 { "1 neue Anfrage beantworten".into() } else { format!("{} neue Anfragen beantworten", x.zahl) },
                    "Angebot erstellen oder Termin vereinbaren".into(),
                    btn("nav", "", "Ansehen", "ghost sm", None, "auftraege"),
                ),
                Typ::Belege => (
                    "paperclip",
                    "gelb",
                    if x.zahl == 1 { "1 Ausgabe ohne Beleg".into() } else { format!("{} Ausgaben ohne Beleg", x.zahl) },
                    "Für die Steuer am besten gleich abfotografieren".into(),
                    btn("heute-belege", "", "Zeigen", "ghost sm", None, ""),
                ),
                Typ::Backup => (
                    "database",
                    "lila",
                    if x.zahl < 0 { "Noch kein Backup gespeichert".into() } else { format!("Letztes Backup vor {} Tagen", x.zahl) },
                    format!("Alle {BACKUP_TAGE} Tage sichern – die Daten liegen nur auf diesem Gerät"),
                    btn("backup", "download", "Sichern", "ghost sm", None, ""),
                ),
            };
            let _ = write!(
                rows,
                "<div class=\"todo\"><span class=\"todo-ic {farbe}\">{}</span><div><b>{titel}</b><span>{text}</span></div>{knopf}</div>",
                ic(icon)
            );
        }
        let mehr = liste.len().saturating_sub(max);
        let mut h = format!(
            "<div class=\"card heute\"><div class=\"card-head\"><h2>Heute zu tun</h2>{}</div>",
            if liste.is_empty() { String::new() } else { format!("<span class=\"count\">{}</span>", liste.len()) }
        );
        if liste.is_empty() {
            let _ = write!(h, "<div class=\"todo-leer\">{}<b>Alles erledigt</b><span>Keine offenen Aufgaben – genieß den Tag.</span></div>", ic("check"));
        } else {
            let _ = write!(h, "<div class=\"todos\">{rows}</div>");
            if mehr > 0 {
                let _ = write!(h, "<p class=\"hint pad\">+ {mehr} weitere</p>");
            }
        }
        h += "</div>";
        h
    }
}
