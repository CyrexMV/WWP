//! Listen per Ziehen umsortieren (Positionen im Editor, Menüpunkte in den Einstellungen):
//! Griff (`[data-drag]`, Liste in `data-drag-list`) gedrückt halten und verschieben.
//! Eine Linie zeigt die Einfügestelle, am Rand scrollt die Seite mit.

use super::{dom, el, on, with_app, App, View};
use wasm_bindgen::JsCast;
use web_sys::{Element, Event, PointerEvent};

pub struct Drag {
    liste: String,
    von: usize,
    pointer: i32,
    ziel: Option<(usize, bool)>,
}

fn zeile(liste: &str, i: usize) -> Option<Element> {
    dom().query_selector(&format!("[data-drag-row=\"{liste}\"][data-i=\"{i}\"]")).ok().flatten()
}

fn markierung_entfernen(liste: &str, ziel: Option<(usize, bool)>) {
    if let Some((i, _)) = ziel {
        if let Some(e) = zeile(liste, i) {
            let _ = e.class_list().remove_2("drop-before", "drop-after");
        }
    }
}

pub fn bind() {
    let d: web_sys::EventTarget = dom().into();
    on(&d, "pointerdown", |e: Event| {
        let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else { return };
        let Some(griff) = t.closest("[data-drag]").ok().flatten() else { return };
        let Some(von) = griff.get_attribute("data-drag").and_then(|v| v.parse::<usize>().ok()) else { return };
        let liste = griff.get_attribute("data-drag-list").unwrap_or_else(|| "pos".into());
        let Some(pe) = e.dyn_ref::<PointerEvent>() else { return };
        let pointer = pe.pointer_id();
        e.prevent_default();
        let _ = griff.set_pointer_capture(pointer);
        with_app(|a| {
            let passt = match liste.as_str() {
                "nav" => a.view() == View::Einstellungen,
                _ => matches!(a.view(), View::Dokument(_)),
            };
            if !passt {
                return;
            }
            let row = zeile(&liste, von);
            a.drag = Some(Drag { liste, von, pointer, ziel: None });
            if let Some(p) = row {
                let _ = p.class_list().add_1("dragging");
            }
        });
    });
    on(&d, "pointermove", |e: Event| {
        let Some(pe) = e.dyn_ref::<PointerEvent>() else { return };
        let (x, y) = (pe.client_x() as f32, pe.client_y() as f32);
        let id = pe.pointer_id();
        with_app(|a| {
            let Some(drag) = a.drag.as_mut() else { return };
            if drag.pointer != id {
                return;
            }
            // am Rand mitscrollen
            if let Some(m) = el("main") {
                let r = m.get_bounding_client_rect();
                if (y as f64) < r.top() + 70.0 {
                    m.set_scroll_top(m.scroll_top() - 14);
                } else if (y as f64) > r.bottom() - 70.0 {
                    m.set_scroll_top(m.scroll_top() + 14);
                }
            }
            let ziel = dom()
                .element_from_point(x, y)
                .and_then(|e| e.closest(&format!("[data-drag-row=\"{}\"]", drag.liste)).ok().flatten())
                .and_then(|p| {
                    let i = p.get_attribute("data-i")?.parse::<usize>().ok()?;
                    let r = p.get_bounding_client_rect();
                    Some((i, (y as f64) < r.top() + r.height() / 2.0, p))
                });
            if let Some((i, vorher, p)) = ziel {
                if drag.ziel != Some((i, vorher)) {
                    markierung_entfernen(&drag.liste, drag.ziel);
                    let _ = p.class_list().add_1(if vorher { "drop-before" } else { "drop-after" });
                    drag.ziel = Some((i, vorher));
                }
            }
        });
    });
    for ev in ["pointerup", "pointercancel"] {
        on(&d, ev, move |e: Event| {
            let Some(pe) = e.dyn_ref::<PointerEvent>() else { return };
            let id = pe.pointer_id();
            let abbrechen = e.type_() == "pointercancel";
            with_app(|a| {
                let Some(drag) = a.drag.take() else { return };
                if drag.pointer != id {
                    a.drag = Some(drag);
                    return;
                }
                markierung_entfernen(&drag.liste, drag.ziel);
                if let Some(p) = zeile(&drag.liste, drag.von) {
                    let _ = p.class_list().remove_1("dragging");
                }
                if abbrechen {
                    return;
                }
                a.drop(drag);
            });
        });
    }
}

impl App {
    fn drop(&mut self, drag: Drag) {
        let Some((i, vorher)) = drag.ziel else { return };
        let mut nach = if vorher { i } else { i + 1 };
        if drag.von < nach {
            nach -= 1;
        }
        if nach == drag.von {
            return;
        }
        if drag.liste == "nav" {
            self.nav_verschieben(drag.von, nach);
            return;
        }
        let View::Dokument(id) = self.view() else { return };
        if let Some(x) = self.data.dokument_mut(id) {
            if drag.von >= x.positionen.len() {
                return;
            }
            let p = x.positionen.remove(drag.von);
            let nach = nach.min(x.positionen.len());
            x.positionen.insert(nach, p);
        }
        self.changed();
        self.render();
    }
}
