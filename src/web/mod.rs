//! Browser-/WebView-Anbindung: Zustand, Navigation, Ereignisse.
//!
//! Die Oberfläche wird in Rust als HTML erzeugt (`views`) und per Ereignis-Delegation bedient:
//! * Klicks auf Elemente mit `data-act` → [`App::action`] (`actions.rs`)
//! * Eingaben in Felder mit `data-f` → [`App::field`] (`actions.rs`), live gespeichert
//! * Suchfelder mit `data-search` → nur die Liste wird neu gezeichnet (Fokus bleibt erhalten)

mod actions;
mod dnd;
mod kalender;
mod settings;
mod signature;
mod storage;
mod views;

use crate::engine::{DokArt, Id};
use std::cell::RefCell;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, Event, EventTarget, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

pub use crate::engine::Data;

// ---------------------------------------------------------------------------
// Globale App
// ---------------------------------------------------------------------------

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Führt `f` mit der App aus (`None`, falls sie gerade beschäftigt ist).
pub fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| match cell.try_borrow_mut() {
        Ok(mut b) => b.as_mut().map(f),
        Err(_) => None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Start,
    Kunden(Option<Id>),
    Auftraege,
    Auftrag(Id),
    Dokumente(DokArt),
    Dokument(Id),
    Finanzen,
    Buchung(Id),
    Kalender,
    Einstellungen,
}

/// Filter und Suchbegriffe der Listen.
#[derive(Default)]
pub struct Ui {
    pub kunden_q: String,
    pub dok_q: String,
    pub dok_filter: String,
    pub buch_q: String,
    pub buch_filter: String,
    pub buch_monat: u32,
    pub jahr: i32,
    pub abgelehnte: bool,
    /// Festgeschriebene Dokumente, die trotzdem bearbeitet werden dürfen
    pub entsperrt: HashSet<Id>,
    /// geöffneter Bereich der Einstellungen
    pub settings_tab: String,
    /// Rechnungen, die trotz unvollständiger E-Rechnung gesendet werden dürfen
    pub e_ok: HashSet<Id>,
    /// Kalender: angezeigter Tag und Ansicht („woche“/„monat“)
    pub kal_tag: String,
    pub kal_ansicht: String,
}

/// Aktion, die nach „Ja“ in einer Sicherheitsabfrage ausgeführt wird.
pub type Confirm = Box<dyn FnOnce(&mut App)>;

pub struct App {
    pub data: Data,
    pub stack: Vec<View>,
    pub db: Option<web_sys::IdbDatabase>,
    pub logo: Option<Vec<u8>>,
    pub logo_url: Option<String>,
    pub ui: Ui,
    pub confirm: Option<Confirm>,
    /// Buchung, an die der nächste ausgewählte Beleg angehängt wird
    pub beleg_ziel: Option<Id>,
    save_gen: u32,
    pub loaded: bool,
    /// offenes Unterschriftenfeld
    pub pad: Option<signature::Pad>,
    /// laufendes Verschieben einer Position
    pub drag: Option<dnd::Drag>,
}

impl App {
    fn new() -> App {
        App {
            data: Data::new(),
            stack: vec![View::Start],
            db: None,
            logo: None,
            logo_url: None,
            ui: Ui { jahr: crate::engine::date::year(&today()).unwrap_or(2026), ..Default::default() },
            confirm: None,
            beleg_ziel: None,
            save_gen: 0,
            loaded: false,
            pad: None,
            drag: None,
        }
    }

    pub fn view(&self) -> View {
        *self.stack.last().unwrap_or(&View::Start)
    }

    /// Zu einer Ansicht wechseln (Verlauf für „Zurück“).
    pub fn go(&mut self, v: View) {
        if self.view() != v {
            // Liste/Detail derselben Art ersetzen sich
            if matches!((self.view(), v), (View::Kunden(_), View::Kunden(_))) {
                self.stack.pop();
            }
            self.stack.push(v);
            if self.stack.len() > 30 {
                self.stack.remove(1);
            }
        }
        self.render();
        scroll_top();
    }

    /// Hauptbereich über die Navigation öffnen (Verlauf zurücksetzen).
    pub fn go_root(&mut self, v: View) {
        self.stack = vec![View::Start];
        if v != View::Start {
            self.stack.push(v);
        }
        self.render();
        scroll_top();
    }

    /// Zurück: Dialog schließen → vorige Ansicht. `false`, wenn nichts mehr zurückgeht.
    pub fn back(&mut self) -> bool {
        if is_modal_open() {
            self.close_modal();
            return true;
        }
        if self.stack.len() > 1 {
            self.stack.pop();
            // gelöschte Objekte überspringen
            while self.stack.len() > 1 && !self.view_exists(self.view()) {
                self.stack.pop();
            }
            self.render();
            return true;
        }
        false
    }

    fn view_exists(&self, v: View) -> bool {
        match v {
            View::Kunden(Some(id)) => self.data.kunde(Some(id)).is_some(),
            View::Auftrag(id) => self.data.auftrag(Some(id)).is_some(),
            View::Dokument(id) => self.data.dokument(Some(id)).is_some(),
            View::Buchung(id) => self.data.buchung(Some(id)).is_some(),
            _ => true,
        }
    }

    /// Daten geändert: verzögert speichern.
    pub fn changed(&mut self) {
        self.save_gen = self.save_gen.wrapping_add(1);
        let g = self.save_gen;
        set_save_state("Speichert …");
        set_timeout(500, move || {
            with_app(|a| {
                if a.save_gen == g {
                    a.save_now();
                }
            });
        });
    }

    pub fn toast(&self, msg: &str) {
        toast(msg);
    }

    // --- Dialoge ------------------------------------------------------------

    pub fn modal(&mut self, html: &str) {
        set_html("modal-card", html);
        if let Some(m) = el("modal") {
            let _ = m.class_list().remove_1("hidden");
        }
        // erstes Feld fokussieren
        if let Some(f) = query("#modal-card [autofocus]") {
            if let Ok(h) = f.dyn_into::<HtmlElement>() {
                let _ = h.focus();
            }
        }
    }

    pub fn close_modal(&mut self) {
        self.confirm = None;
        self.pad = None;
        if let Some(m) = el("modal-card") {
            let _ = m.class_list().remove_1("wide");
        }
        if let Some(m) = el("modal") {
            let _ = m.class_list().add_1("hidden");
        }
        set_html("modal-card", "");
    }

    /// Sicherheitsabfrage; `f` läuft bei „Ja“.
    pub fn ask(&mut self, title: &str, text: &str, yes: &str, danger: bool, f: impl FnOnce(&mut App) + 'static) {
        let html = format!(
            "<h3>{}</h3><p class=\"muted\">{}</p><div class=\"modal-actions\"><button class=\"btn ghost\" data-act=\"modal-close\">Abbrechen</button><button class=\"btn {}\" data-act=\"confirm-yes\">{}</button></div>",
            esc(title),
            esc(text),
            if danger { "danger" } else { "primary" },
            esc(yes)
        );
        self.modal(&html);
        self.confirm = Some(Box::new(f));
    }
}

// ---------------------------------------------------------------------------
// DOM-Helfer
// ---------------------------------------------------------------------------

pub fn window() -> web_sys::Window {
    web_sys::window().expect("kein window")
}

pub fn dom() -> web_sys::Document {
    window().document().expect("kein document")
}

pub fn el(id: &str) -> Option<HtmlElement> {
    dom().get_element_by_id(id).and_then(|e| e.dyn_into::<HtmlElement>().ok())
}

pub fn query(sel: &str) -> Option<Element> {
    dom().query_selector(sel).ok().flatten()
}

pub fn set_html(id: &str, html: &str) {
    if let Some(e) = el(id) {
        e.set_inner_html(html);
    }
}

pub fn set_timeout(ms: i32, f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
}

pub fn on(target: &EventTarget, ev: &str, f: impl FnMut(Event) + 'static) {
    let cb = Closure::<dyn FnMut(Event)>::new(f);
    let _ = target.add_event_listener_with_callback(ev, cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Wert eines Eingabefelds (input/select/textarea) per ID.
pub fn value_of(id: &str) -> String {
    let Some(e) = dom().get_element_by_id(id) else { return String::new() };
    element_value(&e)
}

/// Ist die Checkbox mit dieser ID angehakt?
pub fn checked_of(id: &str) -> bool {
    dom()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.checked())
        .unwrap_or(false)
}

pub fn element_value(e: &Element) -> String {
    if let Some(i) = e.dyn_ref::<HtmlInputElement>() {
        i.value()
    } else if let Some(s) = e.dyn_ref::<HtmlSelectElement>() {
        s.value()
    } else if let Some(t) = e.dyn_ref::<HtmlTextAreaElement>() {
        t.value()
    } else {
        String::new()
    }
}

/// HTML-Sonderzeichen maskieren (für Text und Attribute).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

/// Heutiges Datum (lokal) als „JJJJ-MM-TT“.
pub fn today() -> String {
    let d = js_sys::Date::new_0();
    crate::engine::date::iso(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
}

pub fn toast(msg: &str) {
    if let Some(t) = el("toast") {
        t.set_text_content(Some(msg));
        let _ = t.class_list().add_1("show");
        thread_local!(static GEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) });
        let g = GEN.with(|g| {
            g.set(g.get() + 1);
            g.get()
        });
        set_timeout(2800, move || {
            if GEN.with(|x| x.get()) == g {
                if let Some(t) = el("toast") {
                    let _ = t.class_list().remove_1("show");
                }
            }
        });
    }
}

pub fn set_save_state(s: &str) {
    if let Some(e) = el("save-state") {
        e.set_text_content(Some(s));
    }
}

fn is_modal_open() -> bool {
    el("modal").map(|m| !m.class_list().contains("hidden")).unwrap_or(false)
}

fn scroll_top() {
    if let Some(m) = el("main") {
        m.set_scroll_top(0);
    }
}

/// Versionsangabe: APK-Version (Android) bzw. Build-Stempel.
pub fn app_version() -> String {
    let build = option_env!("WWP_BUILD").unwrap_or("dev");
    let apk = storage::android_bridge()
        .and_then(|b| js_sys::Reflect::get(&b, &"version".into()).ok())
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call0(&JsValue::NULL).ok())
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty());
    match apk {
        Some(v) => format!("{v} ({build})"),
        None => build.to_string(),
    }
}

/// Farbschema: "auto" | "light" | "dark"
pub fn apply_theme(t: &str) {
    if let Some(root) = dom().document_element() {
        let _ = root.set_attribute("data-theme", t);
    }
    if let Ok(Some(s)) = window().local_storage() {
        let _ = s.set_item("wwp-theme", t);
    }
}

pub fn theme() -> String {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item("wwp-theme").ok().flatten())
        .unwrap_or_else(|| "auto".into())
}

// ---------------------------------------------------------------------------
// Start und Ereignisse
// ---------------------------------------------------------------------------

fn closest_with(target: &EventTarget, attr: &str) -> Option<Element> {
    let e: &Element = target.dyn_ref::<Element>()?;
    e.closest(&format!("[{attr}]")).ok().flatten()
}

/// Feldtypen, die über `change` statt `input` gemeldet werden.
fn is_change_field(e: &Element) -> bool {
    if e.dyn_ref::<HtmlSelectElement>().is_some() {
        return true;
    }
    if let Some(i) = e.dyn_ref::<HtmlInputElement>() {
        return matches!(i.type_().as_str(), "checkbox" | "radio" | "date" | "color");
    }
    false
}

fn field_event(e: &Element) {
    let Some(key) = e.get_attribute("data-f") else { return };
    let checked = e.dyn_ref::<HtmlInputElement>().map(|i| i.checked()).unwrap_or(false);
    let v = element_value(e);
    with_app(|a| a.field(&key, &v, checked));
}

#[wasm_bindgen(start)]
pub fn start() {
    apply_theme(&theme());
    APP.with(|c| *c.borrow_mut() = Some(App::new()));

    let d: EventTarget = dom().into();
    on(&d, "click", |ev: Event| {
        let Some(t) = ev.target() else { return };
        // Klick auf den Dialog-Hintergrund schließt ihn
        if let Some(e) = t.dyn_ref::<Element>() {
            if e.id() == "modal" {
                with_app(|a| a.close_modal());
                return;
            }
        }
        let Some(e) = closest_with(&t, "data-act") else { return };
        if e.has_attribute("disabled") {
            return;
        }
        let act = e.get_attribute("data-act").unwrap_or_default();
        let id = e.get_attribute("data-id").and_then(|s| s.parse::<Id>().ok());
        let v = e.get_attribute("data-v").unwrap_or_default();
        if e.tag_name() == "A" && e.get_attribute("href").map(|h| h.starts_with("mailto:") || h.starts_with("tel:")).unwrap_or(false) {
            return;
        }
        ev.prevent_default();
        with_app(|a| a.action(&act, id, &v));
    });
    on(&d, "input", |ev: Event| {
        let Some(t) = ev.target() else { return };
        let Some(e) = t.dyn_ref::<Element>() else { return };
        if let Some(k) = e.get_attribute("data-search") {
            let v = element_value(e);
            with_app(|a| a.search(&k, &v));
            return;
        }
        if !is_change_field(e) {
            field_event(e);
        }
    });
    on(&d, "change", |ev: Event| {
        let Some(t) = ev.target() else { return };
        let Some(e) = t.dyn_ref::<Element>() else { return };
        if let Some(k) = e.get_attribute("data-search") {
            if is_change_field(e) {
                let v = element_value(e);
                with_app(|a| a.search(&k, &v));
            }
            return;
        }
        if is_change_field(e) {
            field_event(e);
        }
    });
    on(&d, "keydown", |ev: Event| {
        let Some(k) = ev.dyn_ref::<web_sys::KeyboardEvent>() else { return };
        if k.key() == "Escape" {
            with_app(|a| a.close_modal());
        } else if k.key() == "Enter" && is_modal_open() {
            // Enter im Dialog = Bestätigen (nicht in mehrzeiligen Feldern)
            let in_textarea = ev.target().and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok()).is_some();
            if !in_textarea {
                if let Some(b) = query("#modal-card [data-enter]") {
                    if let Ok(b) = b.dyn_into::<HtmlElement>() {
                        ev.prevent_default();
                        b.click();
                    }
                }
            }
        }
    });

    // Zurück-Taste von Android: window.wwpBack() → true, wenn die App selbst zurückgegangen ist
    let back = Closure::<dyn FnMut() -> bool>::new(|| with_app(|a| a.back()).unwrap_or(true));
    let _ = js_sys::Reflect::set(&window(), &"wwpBack".into(), back.as_ref());
    back.forget();

    storage::bind_files();
    signature::bind();
    dnd::bind();
    storage::init_storage();
    storage::register_service_worker();
    with_app(|a| a.render());
}
