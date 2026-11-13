//! Unterschriftenfeld: der Kunde unterschreibt auf dem Tablet (S-Pen mit Druckstufen, Finger oder Maus).
//!
//! Die Striche werden als Vektor-Punkte gespeichert (`[x, y, Druck]`, normiert auf die Feldbreite)
//! und im PDF als Vektor-Tinte gezeichnet. Sobald der Stift einmal aufgesetzt hat, werden
//! Finger-/Handballen-Berührungen ignoriert.

use super::views::{btn, ic};
use super::{dom, el, esc, on, today, value_of, window, with_app, App};
use crate::engine::model::{DokArt, Id, Unterschrift, UNTERSCHRIFT_VERHAELTNIS};
use crate::engine::money::fmt_eur;
use std::fmt::Write as _;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Event, HtmlCanvasElement, PointerEvent};

/// Zustand des offenen Unterschriftenfelds.
pub struct Pad {
    pub dok: Id,
    /// Unterschrift für ein Wartungsprotokoll statt für ein Dokument
    pub wartung: bool,
    pub striche: Vec<Vec<[f32; 3]>>,
    aktiv: Option<i32>,
    stift: bool,
}

const TINTE: &str = "#14205a";

fn canvas() -> Option<HtmlCanvasElement> {
    dom().get_element_by_id("sig-canvas")?.dyn_into::<HtmlCanvasElement>().ok()
}

fn ctx(c: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    c.get_context("2d").ok()??.dyn_into::<CanvasRenderingContext2d>().ok()
}

/// Strichbreite in Pixeln für eine Feldbreite `w` und einen Druck `p`.
fn breite(w: f64, p: f32) -> f64 {
    w * (0.0035 + 0.0085 * p.clamp(0.05, 1.0) as f64)
}

fn segment(cx: &CanvasRenderingContext2d, w: f64, a: [f32; 3], b: [f32; 3]) {
    cx.set_line_width(breite(w, (a[2] + b[2]) * 0.5));
    cx.begin_path();
    cx.move_to(a[0] as f64 * w, a[1] as f64 * w);
    cx.line_to(b[0] as f64 * w, b[1] as f64 * w);
    cx.stroke();
}

/// Canvas an die angezeigte Größe anpassen (scharf auf hochauflösenden Displays) und neu zeichnen.
fn resize_and_redraw(pad: &Pad) {
    let Some(c) = canvas() else { return };
    let dpr = window().device_pixel_ratio().max(1.0);
    let w = c.client_width() as f64;
    if w <= 0.0 {
        return;
    }
    c.set_width((w * dpr) as u32);
    c.set_height((w / UNTERSCHRIFT_VERHAELTNIS as f64 * dpr) as u32);
    let Some(cx) = ctx(&c) else { return };
    let _ = cx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    cx.set_line_cap("round");
    cx.set_line_join("round");
    cx.set_stroke_style_str(TINTE);
    cx.set_fill_style_str(TINTE);
    cx.clear_rect(0.0, 0.0, w, w);
    for s in &pad.striche {
        if s.len() == 1 {
            let p = s[0];
            cx.begin_path();
            let _ = cx.arc(p[0] as f64 * w, p[1] as f64 * w, breite(w, p[2]) * 0.5, 0.0, std::f64::consts::TAU);
            cx.fill();
        }
        for seg in s.windows(2) {
            segment(&cx, w, seg[0], seg[1]);
        }
    }
    if let Some(f) = el("sig-pad") {
        let _ = if pad.striche.is_empty() { f.class_list().remove_1("has-ink") } else { f.class_list().add_1("has-ink") };
    }
}

/// Position eines Zeigers relativ zum Feld (normiert auf die Breite) inkl. Druck.
fn punkt(e: &PointerEvent, c: &HtmlCanvasElement) -> [f32; 3] {
    let r = c.get_bounding_client_rect();
    let w = r.width().max(1.0);
    let x = ((e.client_x() as f64 - r.left()) / w).clamp(0.0, 1.0) as f32;
    let y = ((e.client_y() as f64 - r.top()) / w).clamp(0.0, 1.0 / UNTERSCHRIFT_VERHAELTNIS as f64) as f32;
    let druck = if e.pointer_type() == "pen" { e.pressure().max(0.08) } else { 0.5 };
    [x, y, druck]
}

fn auf_feld(e: &Event) -> bool {
    e.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .map(|t| t.id() == "sig-canvas")
        .unwrap_or(false)
}

/// Zeiger-Ereignisse einmalig auf Dokumentebene anmelden.
pub fn bind() {
    let d: web_sys::EventTarget = dom().into();
    on(&d, "pointerdown", |e: Event| {
        if !auf_feld(&e) {
            return;
        }
        let Some(pe) = e.dyn_ref::<PointerEvent>() else { return };
        let Some(c) = canvas() else { return };
        let p = punkt(pe, &c);
        let typ = pe.pointer_type();
        let id = pe.pointer_id();
        e.prevent_default();
        with_app(|a| {
            let Some(pad) = a.pad.as_mut() else { return };
            if typ == "pen" {
                pad.stift = true;
            } else if typ == "touch" && pad.stift {
                return; // Handballen ignorieren, sobald mit dem Stift geschrieben wird
            }
            if pad.aktiv.is_some() {
                return;
            }
            let _ = c.set_pointer_capture(id);
            pad.aktiv = Some(id);
            pad.striche.push(vec![p]);
            if let Some(cx) = ctx(&c) {
                let w = c.client_width() as f64;
                cx.begin_path();
                let _ = cx.arc(p[0] as f64 * w, p[1] as f64 * w, breite(w, p[2]) * 0.5, 0.0, std::f64::consts::TAU);
                cx.fill();
            }
            if let Some(f) = el("sig-pad") {
                let _ = f.class_list().add_1("has-ink");
            }
        });
    });
    on(&d, "pointermove", |e: Event| {
        if !auf_feld(&e) {
            return;
        }
        let Some(pe) = e.dyn_ref::<PointerEvent>() else { return };
        let Some(c) = canvas() else { return };
        let id = pe.pointer_id();
        let p = punkt(pe, &c);
        with_app(|a| {
            let Some(pad) = a.pad.as_mut() else { return };
            if pad.aktiv != Some(id) {
                return;
            }
            let Some(strich) = pad.striche.last_mut() else { return };
            let Some(&last) = strich.last() else { return };
            // sehr kleine Bewegungen zusammenfassen (weniger Daten, glattere Linie)
            if (p[0] - last[0]).abs() < 0.0015 && (p[1] - last[1]).abs() < 0.0015 {
                return;
            }
            strich.push(p);
            if let Some(cx) = ctx(&c) {
                segment(&cx, c.client_width() as f64, last, p);
            }
        });
    });
    for ev in ["pointerup", "pointercancel", "lostpointercapture"] {
        on(&d, ev, |e: Event| {
            let Some(pe) = e.dyn_ref::<PointerEvent>() else { return };
            let id = pe.pointer_id();
            with_app(|a| {
                if let Some(pad) = a.pad.as_mut() {
                    if pad.aktiv == Some(id) {
                        pad.aktiv = None;
                    }
                }
            });
        });
    }
    on(&window().into(), "resize", |_| {
        with_app(|a| {
            if let Some(pad) = &a.pad {
                resize_and_redraw(pad);
            }
        });
    });
}

/// Unterschrift als SVG (für die Anzeige in der App).
pub fn sig_svg(u: &Unterschrift) -> String {
    let w = 300.0f32;
    let h = w / UNTERSCHRIFT_VERHAELTNIS;
    let mut s = format!("<svg class=\"sig-svg\" viewBox=\"0 0 {w} {h}\" preserveAspectRatio=\"xMinYMid meet\">");
    for strich in &u.striche {
        if strich.len() == 1 {
            let p = strich[0];
            let _ = write!(s, "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"1.6\" fill=\"{TINTE}\"/>", p[0] * w, p[1] * w);
            continue;
        }
        for seg in strich.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let sw = w * (0.0035 + 0.0085 * ((a[2] + b[2]) * 0.5).clamp(0.05, 1.0));
            let _ = write!(
                s,
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke-width=\"{:.2}\"/>",
                a[0] * w,
                a[1] * w,
                b[0] * w,
                b[1] * w,
                sw
            );
        }
    }
    s.push_str("</svg>");
    s
}

impl App {
    /// Unterschriftenfeld für ein Dokument öffnen.
    pub fn sig_open(&mut self, dok: Id) {
        let Some(x) = self.data.dokument(Some(dok)) else { return };
        let k = self.data.kunde(x.kunde_id);
        let name = k
            .map(|k| if k.ansprechpartner.trim().is_empty() { k.name() } else { k.ansprechpartner.trim().to_string() })
            .unwrap_or_default();
        let ort = k.map(|k| k.ort.trim().to_string()).unwrap_or_default();
        let erklaerung = match x.art {
            DokArt::Angebot => format!(
                "Hiermit nehme ich das Angebot {} über {} an und erteile den Auftrag.",
                x.nummer,
                fmt_eur(x.summen().brutto)
            ),
            DokArt::Auftragsbestaetigung => format!("Hiermit bestätige ich den Auftrag gemäß Auftragsbestätigung {}.", x.nummer),
            DokArt::Lieferschein => format!("Hiermit bestätige ich den Erhalt der Leistungen laut Lieferschein {}.", x.nummer),
            DokArt::Rechnung => return,
        };
        let html = format!(
            "<h3>Unterschrift des Kunden</h3><p class=\"muted\">{}</p><div class=\"sig-pad\" id=\"sig-pad\"><canvas id=\"sig-canvas\"></canvas><span class=\"sig-hint\">{}Hier mit dem S-Pen oder dem Finger unterschreiben</span><span class=\"sig-x\">×</span><i class=\"sig-line\"></i></div><div class=\"form\"><label class=\"field\"><span>Name in Druckbuchstaben</span><input type=\"text\" id=\"sig-name\" value=\"{}\"></label><label class=\"field\"><span>Ort</span><input type=\"text\" id=\"sig-ort\" value=\"{}\"></label></div><div class=\"modal-actions spread\">{}<span class=\"flex1\"></span>{}{}</div>",
            esc(&erklaerung),
            ic("pen"),
            esc(&name),
            esc(&ort),
            btn("sig-clear", "undo", "Neu", "ghost", None, ""),
            btn("modal-close", "", "Abbrechen", "ghost", None, ""),
            btn("sig-ok", "check", "Unterschrift übernehmen", "primary", Some(dok), "")
        );
        self.pad_oeffnen(dok, false, &html);
    }

    /// Unterschriftenfeld für ein Wartungsprotokoll.
    pub fn sig_open_wartung(&mut self, wid: Id) {
        let Some(w) = self.data.wartung(Some(wid)) else { return };
        let k = self.data.kunde(w.kunde_id);
        let name = k.map(|k| if k.ansprechpartner.trim().is_empty() { k.name() } else { k.ansprechpartner.trim().to_string() }).unwrap_or_default();
        let ort = k.map(|k| k.ort.trim().to_string()).unwrap_or_default();
        let erklaerung = format!("Hiermit bestätige ich die Durchführung der Wartung laut Protokoll {} vom {}.", w.nummer, crate::engine::date::german(&w.datum));
        let html = format!(
            "<h3>Unterschrift des Kunden</h3><p class=\"muted\">{}</p><div class=\"sig-pad\" id=\"sig-pad\"><canvas id=\"sig-canvas\"></canvas><span class=\"sig-hint\">{}Hier mit dem S-Pen oder dem Finger unterschreiben</span><span class=\"sig-x\">×</span><i class=\"sig-line\"></i></div><div class=\"form\"><label class=\"field\"><span>Name in Druckbuchstaben</span><input type=\"text\" id=\"sig-name\" value=\"{}\"></label><label class=\"field\"><span>Ort</span><input type=\"text\" id=\"sig-ort\" value=\"{}\"></label></div><div class=\"modal-actions spread\">{}<span class=\"flex1\"></span>{}{}</div>",
            esc(&erklaerung),
            ic("pen"),
            esc(&name),
            esc(&ort),
            btn("sig-clear", "undo", "Neu", "ghost", None, ""),
            btn("modal-close", "", "Abbrechen", "ghost", None, ""),
            btn("sig-ok", "check", "Unterschrift übernehmen", "primary", Some(wid), "")
        );
        self.pad_oeffnen(wid, true, &html);
    }

    fn pad_oeffnen(&mut self, dok: Id, wartung: bool, html: &str) {
        self.pad = Some(Pad { dok, wartung, striche: Vec::new(), aktiv: None, stift: false });
        self.modal(html);
        if let Some(m) = el("modal-card") {
            let _ = m.class_list().add_1("wide");
        }
        super::set_timeout(30, || {
            with_app(|a| {
                if let Some(pad) = &a.pad {
                    resize_and_redraw(pad);
                }
            });
        });
    }

    pub fn sig_clear(&mut self) {
        if let Some(pad) = self.pad.as_mut() {
            pad.striche.clear();
            pad.aktiv = None;
        }
        if let Some(pad) = &self.pad {
            resize_and_redraw(pad);
        }
    }

    pub fn sig_ok(&mut self) {
        let Some(pad) = self.pad.as_ref() else { return };
        let rund = |v: f32| (v * 10_000.0).round() / 10_000.0;
        let striche: Vec<Vec<[f32; 3]>> = pad
            .striche
            .iter()
            .map(|s| s.iter().map(|p| [rund(p[0]), rund(p[1]), (p[2] * 100.0).round() / 100.0]).collect())
            .collect();
        let u = Unterschrift { striche, name: value_of("sig-name").trim().to_string(), ort: value_of("sig-ort").trim().to_string(), datum: today() };
        if !u.gueltig() {
            self.toast("Bitte zuerst im Feld unterschreiben");
            return;
        }
        let dok = pad.dok;
        let ok = if pad.wartung { self.data.wartung_unterschreiben(dok, u) } else { self.data.unterschreiben(dok, u, &today()) };
        if ok {
            self.close_modal();
            self.changed();
            self.render();
            self.toast("Unterschrift übernommen");
        }
    }
}
