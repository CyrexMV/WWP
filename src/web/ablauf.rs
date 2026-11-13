//! Ablauf eines Vorgangs: Angebot → Auftragsbestätigung → Lieferschein → Rechnung.
//! Zeigt, was es schon gibt, was übersprungen wurde und was als Nächstes kommt –
//! im Dokument-Editor und im Auftrag. Folgedokumente übernehmen Kunde und Positionen.

use super::views::{badge, btn, ic};
use super::{esc, today, App};
use crate::engine::date::german;
use crate::engine::model::{DokArt, DokStatus, Id};
use std::fmt::Write as _;

/// (Kurzname, Erklärung, optional?)
fn schritt_info(art: DokArt) -> (&'static str, &'static str, bool) {
    match art {
        DokArt::Angebot => ("Angebot", "Preisvorschlag an den Kunden", false),
        DokArt::Auftragsbestaetigung => ("Auftrags\u{ad}bestätigung", "Zusage schriftlich bestätigen", true),
        DokArt::Lieferschein => ("Lieferschein", "Kunde quittiert die Arbeit – ohne Preise", true),
        DokArt::Rechnung => ("Rechnung", "Geld anfordern – Zahlung wird in Finanzen gebucht", false),
    }
}

impl App {
    /// Ablauf-Karte. `aktuell`: gerade geöffnetes Dokument; `auftrag`: Auftrag des Vorgangs.
    pub fn ablauf_html(&self, aktuell: Option<Id>, auftrag: Option<Id>) -> String {
        let d = &self.data;
        let heute = today();
        let schritte = d.vorgang(aktuell, auftrag);
        let letzter = schritte.iter().rposition(|s| s.is_some());
        // Quelle für neue Folgedokumente: das späteste vorhandene Dokument
        let quelle = letzter.and_then(|i| schritte[i]);
        let mut h = String::from("<div class=\"card ablauf\"><div class=\"card-head\"><h2>Ablauf</h2></div><ol class=\"steps\">");
        for (i, art) in DokArt::ALLE.iter().enumerate() {
            let (name, erkl, optional) = schritt_info(*art);
            let nr = i + 1;
            match schritte[i].and_then(|id| d.dokument(Some(id))) {
                Some(x) => {
                    let hier = Some(x.id) == aktuell;
                    let fertig = matches!(x.status, DokStatus::Angenommen | DokStatus::Unterschrieben | DokStatus::Bezahlt);
                    let inhalt = format!(
                        "<span class=\"step-dot\">{}</span><div><b>{}</b><span>{} · {}</span></div>{}",
                        if fertig { ic("check") } else { nr.to_string() },
                        esc(name),
                        esc(&x.nummer),
                        if hier { "du bist hier".to_string() } else { german(&x.datum) },
                        badge(x.status_key(&heute), x.status_text(&heute))
                    );
                    if hier {
                        let _ = write!(h, "<li class=\"step da hier{}\">{inhalt}</li>", if fertig { " fertig" } else { "" });
                    } else {
                        let _ = write!(
                            h,
                            "<li class=\"step da{}\"><button class=\"step-link\" data-act=\"dok-open\" data-id=\"{}\" title=\"Öffnen\">{inhalt}</button></li>",
                            if fertig { " fertig" } else { "" },
                            x.id
                        );
                    }
                }
                None if letzter.is_some_and(|l| i < l) => {
                    let _ = write!(
                        h,
                        "<li class=\"step weg\"><span class=\"step-dot\">{nr}</span><div><b>{}</b><span>übersprungen</span></div></li>",
                        esc(name)
                    );
                }
                None => {
                    let knopf = match (quelle, auftrag) {
                        (Some(src), _) => btn("dok-folge", "plus", "Erstellen", if optional { "ghost sm" } else { "primary sm" }, Some(src), art.key()),
                        (None, Some(a)) => btn("auftrag-dok", "plus", "Erstellen", if optional { "ghost sm" } else { "primary sm" }, Some(a), art.key()),
                        (None, None) => String::new(),
                    };
                    let _ = write!(
                        h,
                        "<li class=\"step offen\"><span class=\"step-dot\">{nr}</span><div><b>{}</b><span>{}{}</span></div>{knopf}</li>",
                        esc(name),
                        if optional { "<i>optional</i> " } else { "" },
                        esc(erkl)
                    );
                }
            }
        }
        h += "</ol>";
        let tipp = match letzter.map(|i| (DokArt::ALLE[i], schritte[i].and_then(|id| d.dokument(Some(id))))) {
            None => "Starte mit einem Angebot – oder schreib direkt die Rechnung.".to_string(),
            Some((DokArt::Angebot, Some(x))) => match x.status {
                DokStatus::Angenommen => "Der Kunde hat zugesagt. Nach der Arbeit die Rechnung erstellen – Bestätigung und Lieferschein nur, wenn du sie brauchst.".into(),
                DokStatus::Abgelehnt => "Das Angebot wurde abgelehnt.".into(),
                DokStatus::Entwurf => "Angebot fertig ausfüllen und per E-Mail senden – oder vor Ort auf dem Tablet unterschreiben lassen.".into(),
                _ => "Warte auf die Zusage des Kunden. Sagt er zu: auf „Angenommen“ tippen oder unterschreiben lassen.".into(),
            },
            Some((DokArt::Auftragsbestaetigung, _)) => "Auftrag ist bestätigt. Nach der Arbeit: Lieferschein quittieren lassen oder direkt die Rechnung erstellen.".into(),
            Some((DokArt::Lieferschein, _)) => "Arbeit ist erledigt – jetzt die Rechnung erstellen.".into(),
            Some((DokArt::Rechnung, Some(x))) if x.status == DokStatus::Bezahlt => format!("{} Vorgang abgeschlossen – die Rechnung ist bezahlt.", ic("check")),
            Some((DokArt::Rechnung, Some(x))) if x.status == DokStatus::Entwurf => "Rechnung prüfen und per E-Mail senden.".into(),
            Some((DokArt::Rechnung, _)) => "Rechnung ist raus. Sobald das Geld da ist: „Zahlung erfassen“.".into(),
            Some(_) => String::new(),
        };
        let _ = write!(
            h,
            "<p class=\"step-tipp\">{tipp}</p>{}</div>",
            if quelle.is_some() && letzter.is_some_and(|l| l < 3) { "<p class=\"hint step-hint\">Beim Erstellen werden Kunde, Anschrift und Positionen übernommen.</p>" } else { "" }
        );
        h
    }

    /// Kurze Erklärung oben in den Dokumentlisten (weg­klickbar).
    pub fn ablauf_tipp_html(&self) -> String {
        if self.data.tipps_aus.iter().any(|t| t == "ablauf") {
            return String::new();
        }
        let mut h = String::from("<div class=\"card ablauf-tipp\"><div class=\"card-head\"><h2>So funktioniert der Ablauf</h2>");
        let _ = write!(h, "<button class=\"icon-btn sm\" data-act=\"tipp-aus\" data-v=\"ablauf\" title=\"Ausblenden\">{}</button></div><ol class=\"flow\">", ic("x"));
        for (i, art) in DokArt::ALLE.iter().enumerate() {
            let (name, erkl, optional) = schritt_info(*art);
            let _ = write!(
                h,
                "<li class=\"{}\"><span class=\"step-dot\">{}</span><b>{}</b><span>{}{}</span></li>",
                if optional { "opt" } else { "" },
                i + 1,
                esc(if *art == DokArt::Auftragsbestaetigung { "Bestätigung" } else { name }),
                if optional { "optional · " } else { "" },
                esc(erkl)
            );
        }
        h += "</ol><p class=\"hint\">Du fängst immer mit <b>einem</b> Dokument an (meist dem Angebot). Alles Weitere erstellst du im Dokument unter <b>Ablauf → Erstellen</b> – Kunde und Positionen werden übernommen. Die optionalen Schritte kannst du einfach auslassen.</p></div>";
        h
    }
}
