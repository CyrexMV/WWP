//! Speicherung (IndexedDB), Dateien (Logo, Belege, Backups) und Ausgabe:
//! Speichern, Öffnen und Teilen – über die Android-Brücke `WWPAndroid` oder im Browser.

use super::{dom, el, set_save_state, set_timeout, today, window, with_app, App, View};
use crate::engine::backup::{b64_encode, make_backup, parse_backup};
use crate::engine::{image, Data, Datei, Id};
use js_sys::{Array, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, HtmlInputElement, IdbDatabase, IdbOpenDbRequest, IdbRequest, IdbTransactionMode, Url};

const DB_NAME: &str = "wwp";
const DB_VERSION: u32 = 1;
const STORE_DATA: &str = "data";
const STORE_FILES: &str = "files";

// ---------------------------------------------------------------------------
// IndexedDB
// ---------------------------------------------------------------------------

fn request_promise(req: &IdbRequest) -> Promise {
    let req = req.clone();
    Promise::new(&mut |resolve, reject| {
        let r2 = req.clone();
        let ok = Closure::once_into_js(move || {
            let _ = resolve.call1(&JsValue::NULL, &r2.result().unwrap_or(JsValue::UNDEFINED));
        });
        let err = Closure::once_into_js(move || {
            let _ = reject.call0(&JsValue::NULL);
        });
        req.set_onsuccess(Some(ok.unchecked_ref()));
        req.set_onerror(Some(err.unchecked_ref()));
    })
}

async fn open_db() -> Result<IdbDatabase, JsValue> {
    let factory = window().indexed_db()?.ok_or(JsValue::NULL)?;
    let req: IdbOpenDbRequest = factory.open_with_u32(DB_NAME, DB_VERSION)?;
    let r2 = req.clone();
    let upgrade = Closure::once_into_js(move || {
        if let Ok(db) = r2.result() {
            let db: IdbDatabase = db.unchecked_into();
            for s in [STORE_DATA, STORE_FILES] {
                if !db.object_store_names().contains(s) {
                    let _ = db.create_object_store(s);
                }
            }
        }
    });
    req.set_onupgradeneeded(Some(upgrade.unchecked_ref()));
    let db = JsFuture::from(request_promise(&req)).await?;
    Ok(db.unchecked_into())
}

async fn idb_get(db: &IdbDatabase, store: &str, key: &str) -> Option<JsValue> {
    let tx = db.transaction_with_str_and_mode(store, IdbTransactionMode::Readonly).ok()?;
    let req = tx.object_store(store).ok()?.get(&JsValue::from_str(key)).ok()?;
    let v = JsFuture::from(request_promise(&req)).await.ok()?;
    if v.is_undefined() || v.is_null() {
        None
    } else {
        Some(v)
    }
}

fn idb_put(db: &IdbDatabase, store: &str, key: &str, value: &JsValue) -> bool {
    (|| -> Result<(), JsValue> {
        let tx = db.transaction_with_str_and_mode(store, IdbTransactionMode::Readwrite)?;
        tx.object_store(store)?.put_with_key(value, &JsValue::from_str(key))?;
        Ok(())
    })()
    .is_ok()
}

fn idb_delete(db: &IdbDatabase, store: &str, key: &str) {
    if let Ok(tx) = db.transaction_with_str_and_mode(store, IdbTransactionMode::Readwrite) {
        if let Ok(s) = tx.object_store(store) {
            let _ = s.delete(&JsValue::from_str(key));
        }
    }
}

fn idb_clear(db: &IdbDatabase, store: &str) {
    if let Ok(tx) = db.transaction_with_str_and_mode(store, IdbTransactionMode::Readwrite) {
        if let Ok(s) = tx.object_store(store) {
            let _ = s.clear();
        }
    }
}

/// Datei aus dem Speicher lesen.
pub async fn load_file(id: Id) -> Option<Vec<u8>> {
    let db = with_app(|a| a.db.clone()).flatten()?;
    let v = idb_get(&db, STORE_FILES, &id.to_string()).await?;
    Some(Uint8Array::new(&v).to_vec())
}

impl App {
    pub fn save_now(&mut self) {
        let json = self.data.to_json();
        let ok = match &self.db {
            Some(db) => idb_put(db, STORE_DATA, "db", &JsValue::from_str(&json)),
            None => false,
        };
        // Notfall-Kopie im localStorage (ohne Dateien), falls IndexedDB nicht verfügbar ist
        if !ok {
            if let Ok(Some(s)) = window().local_storage() {
                let _ = s.set_item("wwp-data", &json);
            }
        }
        set_save_state(if ok { "Gespeichert" } else { "Nur lokal gesichert" });
    }

    /// Datei ablegen und Metadaten eintragen; liefert die neue ID.
    pub fn store_file(&mut self, name: &str, mime: &str, bytes: &[u8]) -> Option<Id> {
        let db = self.db.clone()?;
        let id = self.data.new_id();
        if !idb_put(&db, STORE_FILES, &id.to_string(), &Uint8Array::from(bytes).into()) {
            return None;
        }
        self.data.dateien.push(Datei { id, name: name.into(), mime: mime.into(), groesse: bytes.len() as u64 });
        Some(id)
    }

    /// Datei löschen, wenn sie nirgends mehr verwendet wird.
    pub fn drop_file(&mut self, id: Id) {
        if self.data.datei_benutzt(id) {
            return;
        }
        self.data.dateien.retain(|f| f.id != id);
        if let Some(db) = &self.db {
            idb_delete(db, STORE_FILES, &id.to_string());
        }
    }

    pub fn set_logo_bytes(&mut self, bytes: Option<Vec<u8>>) {
        if let Some(u) = self.logo_url.take() {
            let _ = Url::revoke_object_url(&u);
        }
        self.logo_url = bytes.as_ref().and_then(|b| blob_url(b, if image::is_png(b) { "image/png" } else { "image/jpeg" }));
        self.logo = bytes;
    }

    // --- Backup ----------------------------------------------------------

    pub fn create_backup(&mut self, share: bool) {
        let data = self.data.clone();
        let ids: Vec<Id> = data.dateien.iter().map(|f| f.id).collect();
        spawn_local(async move {
            let mut files = Vec::new();
            for id in ids {
                if let Some(b) = load_file(id).await {
                    files.push((id, b));
                }
            }
            let heute = today();
            let json = make_backup(&data, &files, &heute);
            let name = format!("Zento-Backup-{}.wwpbackup", heute);
            if share {
                share_file(json.as_bytes(), "application/json", &name, "", &name, "Backup von Zento");
            } else {
                save_file(json.as_bytes(), "application/json", &name);
            }
        });
        self.data.letztes_backup = today();
        self.changed();
        if self.view() == super::View::Start {
            self.render();
        }
    }

    fn ask_restore(&mut self, text: String) {
        match parse_backup(&text) {
            Ok((data, files, when)) => {
                let msg = format!(
                    "Backup vom {} mit {} Kunden, {} Dokumenten und {} Buchungen. Die aktuellen Daten werden dabei ERSETZT.",
                    crate::engine::date::german(&when),
                    data.kunden.len(),
                    data.dokumente.len(),
                    data.buchungen.len()
                );
                self.ask("Backup wiederherstellen?", &msg, "Wiederherstellen", true, move |a| a.restore(data, files));
            }
            Err(e) => self.toast(&e),
        }
    }

    pub fn restore(&mut self, data: Data, files: Vec<(Id, Vec<u8>)>) {
        if let Some(db) = &self.db {
            idb_clear(db, STORE_FILES);
            for (id, b) in &files {
                idb_put(db, STORE_FILES, &id.to_string(), &Uint8Array::from(b.as_slice()).into());
            }
        }
        let logo = data.firma.logo.and_then(|l| files.iter().find(|f| f.0 == l).map(|f| f.1.clone()));
        self.data = data;
        self.set_logo_bytes(logo);
        self.save_now();
        self.go_root(View::Start);
        self.toast("Backup wiederhergestellt");
    }

    /// Alle Daten löschen.
    pub fn wipe(&mut self) {
        if let Some(db) = &self.db {
            idb_clear(db, STORE_FILES);
        }
        self.data = Data::new();
        self.set_logo_bytes(None);
        self.save_now();
        self.go_root(View::Start);
        self.toast("Alle Daten gelöscht");
    }
}

pub fn init_storage() {
    set_timeout(6000, hide_loading);
    spawn_local(async {
        match open_db().await {
            Ok(db) => {
                let json = idb_get(&db, STORE_DATA, "db").await.and_then(|v| v.as_string());
                let fallback = window().local_storage().ok().flatten().and_then(|s| s.get_item("wwp-data").ok().flatten());
                let data = json.or(fallback).map(|s| Data::from_json(&s));
                let logo_id = match &data {
                    Some(Ok(d)) => d.firma.logo,
                    _ => None,
                };
                let logo = match logo_id {
                    Some(id) => idb_get(&db, STORE_FILES, &id.to_string()).await.map(|v| Uint8Array::new(&v).to_vec()),
                    None => None,
                };
                with_app(|a| {
                    a.db = Some(db);
                    match data {
                        Some(Ok(d)) => a.data = d,
                        Some(Err(e)) => a.toast(&e),
                        None => {}
                    }
                    a.set_logo_bytes(logo);
                    a.loaded = true;
                    a.render();
                });
            }
            Err(_) => {
                let fallback = window().local_storage().ok().flatten().and_then(|s| s.get_item("wwp-data").ok().flatten());
                with_app(|a| {
                    if let Some(Ok(d)) = fallback.map(|s| Data::from_json(&s)) {
                        a.data = d;
                    }
                    a.loaded = true;
                    a.toast("Datenbank nicht verfügbar – Dateien (Logo, Belege) können nicht gespeichert werden");
                    a.render();
                });
            }
        }
        hide_loading();
    });
    super::on(&dom().into(), "visibilitychange", |_| {
        if dom().visibility_state() == web_sys::VisibilityState::Hidden {
            with_app(|a| a.save_now());
        }
    });
}

fn hide_loading() {
    if let Some(l) = el("loading") {
        let _ = l.class_list().add_1("done");
    }
}

pub fn register_service_worker() {
    if android_bridge().is_some() {
        return;
    }
    let nav = window().navigator();
    let has = js_sys::Reflect::get(&nav, &"serviceWorker".into()).map(|v| !v.is_undefined()).unwrap_or(false);
    let https = window().location().protocol().map(|p| p == "https:").unwrap_or(false);
    let local = window().location().hostname().map(|h| h == "localhost" || h == "127.0.0.1").unwrap_or(false);
    if has && (https || local) {
        let _ = nav.service_worker().register("sw.js");
    }
}

// ---------------------------------------------------------------------------
// Dateiauswahl
// ---------------------------------------------------------------------------

/// Dateiauswahl öffnen (außerhalb des laufenden Klick-Handlers, sonst würde dieser erneut ausgelöst).
pub fn pick_file(input_id: &str) {
    let id = input_id.to_string();
    set_timeout(0, move || {
        if let Some(i) = el(&id) {
            i.click();
        }
    });
}

async fn read_file(file: web_sys::File) -> Option<Vec<u8>> {
    let buf = JsFuture::from(file.array_buffer()).await.ok()?;
    Some(Uint8Array::new(&buf).to_vec())
}

fn on_file(input_id: &'static str, f: impl Fn(String, String, Vec<u8>) + 'static + Clone) {
    let Some(input) = el(input_id) else { return };
    let input: HtmlInputElement = input.unchecked_into();
    let i2 = input.clone();
    super::on(&input.into(), "change", move |_| {
        let Some(file) = i2.files().and_then(|l| l.get(0)) else { return };
        i2.set_value("");
        let f = f.clone();
        spawn_local(async move {
            let name = file.name();
            let mime = file.type_();
            if file.size() > 25.0 * 1024.0 * 1024.0 {
                super::toast("Datei ist zu groß (max. 25 MB)");
                return;
            }
            if let Some(bytes) = read_file(file).await {
                f(name, mime, bytes);
            }
        });
    });
}

pub fn bind_files() {
    on_file("file-logo", |_name, _mime, bytes| {
        with_app(|a| {
            if !image::is_png(&bytes) && !image::is_jpeg(&bytes) {
                a.toast("Bitte ein PNG- oder JPG-Bild wählen");
                return;
            }
            let old = a.data.firma.logo.take();
            let mime = if image::is_png(&bytes) { "image/png" } else { "image/jpeg" };
            let ext = if image::is_png(&bytes) { "png" } else { "jpg" };
            match a.store_file(&format!("logo.{ext}"), mime, &bytes) {
                Some(id) => {
                    a.data.firma.logo = Some(id);
                    a.set_logo_bytes(Some(bytes));
                    a.toast("Logo übernommen");
                }
                None => {
                    a.data.firma.logo = old;
                    a.toast("Logo konnte nicht gespeichert werden");
                    return;
                }
            }
            if let Some(o) = old {
                a.drop_file(o);
            }
            a.changed();
            a.render();
        });
    });
    on_file("file-beleg", |name, mime, bytes| {
        with_app(|a| {
            let offen = match a.view() {
                View::Buchung(id) => Some(id),
                _ => None,
            };
            let Some(bid) = a.beleg_ziel.take().or(offen) else { return };
            let mime = if mime.is_empty() {
                if image::is_pdf(&bytes) {
                    "application/pdf".to_string()
                } else if image::is_png(&bytes) {
                    "image/png".to_string()
                } else {
                    "image/jpeg".to_string()
                }
            } else {
                mime
            };
            let Some(fid) = a.store_file(&name, &mime, &bytes) else {
                a.toast("Beleg konnte nicht gespeichert werden");
                return;
            };
            let old = a.data.buchung_mut(bid).and_then(|b| b.beleg.replace(fid));
            if let Some(o) = old {
                a.drop_file(o);
            }
            a.changed();
            a.render();
            a.toast("Beleg angehängt");
        });
    });
    on_file("file-restore", |_name, _mime, bytes| {
        let text = String::from_utf8_lossy(&bytes).to_string();
        with_app(|a| a.ask_restore(text));
    });
}

// ---------------------------------------------------------------------------
// Ausgabe: Android-Brücke oder Browser
// ---------------------------------------------------------------------------

pub fn android_bridge() -> Option<js_sys::Object> {
    let v = js_sys::Reflect::get(&window(), &"WWPAndroid".into()).ok()?;
    if v.is_undefined() || v.is_null() {
        None
    } else {
        Some(v.unchecked_into())
    }
}

/// Methode der Android-Brücke aufrufen; `None`, wenn es sie nicht gibt.
fn bridge_call(method: &str, args: &[JsValue]) -> Option<JsValue> {
    let b = android_bridge()?;
    let f = js_sys::Reflect::get(&b, &method.into()).ok()?.dyn_into::<js_sys::Function>().ok()?;
    let arr = Array::new();
    for a in args {
        arr.push(a);
    }
    f.apply(&b, &arr).ok()
}

fn blob(bytes: &[u8], mime: &str) -> Option<Blob> {
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let opts = BlobPropertyBag::new();
    opts.set_type(mime);
    Blob::new_with_u8_array_sequence_and_options(&parts, &opts).ok()
}

pub fn blob_url(bytes: &[u8], mime: &str) -> Option<String> {
    Url::create_object_url_with_blob(&blob(bytes, mime)?).ok()
}

fn click_download(url: &str, name: &str) {
    let (url, name) = (url.to_string(), name.to_string());
    set_timeout(0, move || {
        let a: HtmlAnchorElement = dom().create_element("a").unwrap().unchecked_into();
        a.set_href(&url);
        a.set_download(&name);
        let _ = dom().body().unwrap().append_child(&a);
        a.click();
        a.remove();
    });
}

/// Datei speichern: Android → „Download/Zento“, Browser → Download.
pub fn save_file(bytes: &[u8], mime: &str, name: &str) {
    if let Some(r) = bridge_call("saveFile", &[name.into(), mime.into(), b64_encode(bytes).into()]) {
        if !r.as_bool().unwrap_or(false) {
            super::toast("Speichern fehlgeschlagen");
        }
        return;
    }
    let Some(url) = blob_url(bytes, mime) else { return };
    click_download(&url, name);
    super::toast(&format!("{name} heruntergeladen"));
    set_timeout(20_000, move || {
        let _ = Url::revoke_object_url(&url);
    });
}

/// Datei ansehen: Android → PDF-/Bildbetrachter, Browser → neuer Tab.
pub fn open_file(bytes: &[u8], mime: &str, name: &str) {
    if let Some(r) = bridge_call("openFile", &[name.into(), mime.into(), b64_encode(bytes).into()]) {
        if !r.as_bool().unwrap_or(false) {
            super::toast("Keine App zum Öffnen gefunden – Datei wird gespeichert");
            save_file(bytes, mime, name);
        }
        return;
    }
    let Some(url) = blob_url(bytes, mime) else { return };
    let opened = window().open_with_url_and_target(&url, "_blank").ok().flatten().is_some();
    if !opened {
        click_download(&url, name);
    }
    set_timeout(120_000, move || {
        let _ = Url::revoke_object_url(&url);
    });
}

/// Webadresse in einer anderen App öffnen (z. B. Google Maps).
pub fn open_url(url: &str) {
    if let Some(r) = bridge_call("openUrl", &[url.into()]) {
        if r.as_bool().unwrap_or(false) {
            return;
        }
    }
    let _ = window().open_with_url_and_target(url, "_blank");
}

/// Termin in die Kalender-App übernehmen (Android), sonst als .ics-Datei speichern.
pub fn kalender_eintragen(t: &crate::engine::termine::Termin, beschreibung: &str, ics: &str) {
    let (y, m, d) = crate::engine::date::parse(&t.datum).unwrap_or((2026, 1, 1));
    let (ye, me, de) = crate::engine::date::parse(&t.letzter_tag()).unwrap_or((y, m, d));
    let hm = |s: &str, std: (i32, i32)| -> (i32, i32) {
        let mut it = s.trim().split(':');
        match (it.next().and_then(|h| h.parse().ok()), it.next().and_then(|m| m.parse().ok())) {
            (Some(h), Some(m)) => (h, m),
            _ => std,
        }
    };
    let (beginn, ende) = if t.ganztag() {
        // ganztägig: Mitternacht UTC (so erwartet es der Android-Kalender)
        let b = js_sys::Date::utc(y as f64, (m - 1) as f64);
        let b = b + (d as f64 - 1.0) * 86_400_000.0;
        let e = js_sys::Date::utc(ye as f64, (me - 1) as f64) + de as f64 * 86_400_000.0;
        (b, e)
    } else {
        let (h1, m1) = hm(&t.von, (9, 0));
        let (h2, m2) = hm(&t.bis, (h1 + 1, m1));
        let b = js_sys::Date::new_with_year_month_day_hr_min(y as u32, m as i32 - 1, d as i32, h1, m1).get_time();
        let e = js_sys::Date::new_with_year_month_day_hr_min(ye as u32, me as i32 - 1, de as i32, h2, m2).get_time();
        (b, e.max(b + 15.0 * 60_000.0))
    };
    let args = [
        JsValue::from_str(&t.titel),
        JsValue::from_f64(beginn),
        JsValue::from_f64(ende),
        JsValue::from_bool(t.ganztag()),
        JsValue::from_str(t.ort.trim()),
        JsValue::from_str(beschreibung),
    ];
    if let Some(r) = bridge_call("calendarInsert", &args) {
        if r.as_bool().unwrap_or(false) {
            return;
        }
    }
    let name = format!("Termin-{}.ics", t.datum);
    save_file(ics.as_bytes(), "text/calendar", &name);
}

/// Datei per E-Mail/Teilen-Menü versenden. `to`, `subject`, `text` füllen die E-Mail vor.
pub fn share_file(bytes: &[u8], mime: &str, name: &str, to: &str, subject: &str, text: &str) {
    let b64 = b64_encode(bytes);
    if let Some(r) = bridge_call("shareFile", &[name.into(), mime.into(), b64.clone().into(), to.into(), subject.into(), text.into()]) {
        if !r.as_bool().unwrap_or(false) {
            super::toast("Teilen fehlgeschlagen");
        }
        return;
    }
    // Browser: Web Share API (Datei + Text), sonst Download + E-Mail-Programm öffnen
    let (name, mime, to, subject, text) = (name.to_string(), mime.to_string(), to.to_string(), subject.to_string(), text.to_string());
    spawn_local(async move {
        let f = js_sys::Reflect::get(&window(), &"wwpShare".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok());
        let args = Array::of5(&b64.clone().into(), &mime.clone().into(), &name.clone().into(), &subject.clone().into(), &text.clone().into());
        let shared = match f.and_then(|f| f.apply(&JsValue::NULL, &args).ok()) {
            Some(p) => JsFuture::from(Promise::from(p)).await.ok().and_then(|v| v.as_bool()).unwrap_or(false),
            None => false,
        };
        if !shared {
            if let Some(bytes) = crate::engine::backup::b64_decode(&b64) {
                save_file(&bytes, &mime, &name);
            }
            if !to.is_empty() || !subject.is_empty() {
                let url = format!("mailto:{}?subject={}&body={}", to, String::from(js_sys::encode_uri_component(&subject)), String::from(js_sys::encode_uri_component(&text)));
                let _ = window().location().set_href(&url);
                super::toast("PDF wurde heruntergeladen – bitte an die E-Mail anhängen");
            }
        }
    });
}
