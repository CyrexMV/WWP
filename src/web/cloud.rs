//! Cloud mit Supabase: Konto (E-Mail + Passwort), Betrieb mit Team (Chef/Monteur, Einladungscode)
//! und Abgleich im Hintergrund. Die App bleibt offline voll nutzbar; Änderungen werden übertragen,
//! sobald Internet da ist. Zugriffsregeln (Row Level Security) liegen in der Datenbank.

use super::storage::{self, load_file};
use super::views::{btn, ic};
use super::{dom, el, esc, set_save_state, set_timeout, window, with_app, App};
use crate::engine::model::{Data, Datei};
use crate::engine::sync::{self, Stand, Zeile, EINZELN, LISTEN, MONTEUR};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt::Write as _;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Headers, Request, RequestInit, Response};

const URL: &str = "https://imayjcxcbhjdftaqvjfj.supabase.co";
/// Öffentlicher Schlüssel (darf in der App stehen – geschützt wird über Row Level Security)
const KEY: &str = "sb_publishable_8PURkGfvG_RaZ6Msl7Y2fg_uw61zoTz";
const LS_SITZUNG: &str = "zento-sitzung";
const LS_STAND: &str = "zento-stand";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Sitzung {
    pub access: String,
    pub refresh: String,
    /// Ablauf des Zugangs (ms seit 1970)
    pub ablauf: f64,
    pub user_id: String,
    pub email: String,
    pub betrieb: String,
    pub betrieb_name: String,
    pub rolle: String,
}

#[derive(Default)]
struct Zustand {
    sitzung: Option<Sitzung>,
    stand: Stand,
    laeuft: bool,
    nochmal: bool,
    /// Uhrzeit des letzten erfolgreichen Abgleichs
    zuletzt: String,
    fehler: String,
    geplant: u32,
    /// Daten kamen an, während ein Feld bearbeitet wurde → später neu zeichnen
    neu_zeichnen: bool,
}

thread_local!(static Z: RefCell<Zustand> = RefCell::new(Zustand::default()));

fn ls_get(k: &str) -> Option<String> {
    window().local_storage().ok().flatten().and_then(|s| s.get_item(k).ok().flatten())
}
fn ls_set(k: &str, v: &str) {
    if let Ok(Some(s)) = window().local_storage() {
        let _ = s.set_item(k, v);
    }
}
fn ls_del(k: &str) {
    if let Ok(Some(s)) = window().local_storage() {
        let _ = s.remove_item(k);
    }
}

fn sitzung() -> Option<Sitzung> {
    Z.with(|z| z.borrow().sitzung.clone())
}
fn sitzung_setzen(s: Option<Sitzung>) {
    match &s {
        Some(x) => ls_set(LS_SITZUNG, &serde_json::to_string(x).unwrap_or_default()),
        None => ls_del(LS_SITZUNG),
    }
    Z.with(|z| z.borrow_mut().sitzung = s);
}
fn stand_speichern() {
    let json = Z.with(|z| serde_json::to_string(&z.borrow().stand).unwrap_or_default());
    ls_set(LS_STAND, &json);
}

/// Angemeldete Rolle im Betrieb („chef“, „monteur“) – leer, wenn nicht verbunden.
pub fn rolle() -> String {
    sitzung().map(|s| s.rolle).unwrap_or_default()
}
/// Über die Cloud als Monteur angemeldet → Monteur-Modus ist Pflicht.
pub fn ist_monteur() -> bool {
    rolle() == "monteur"
}
pub fn verbunden() -> bool {
    sitzung().is_some_and(|s| !s.betrieb.is_empty())
}

fn typen() -> (Vec<&'static str>, Vec<&'static str>) {
    if ist_monteur() {
        (MONTEUR.to_vec(), MONTEUR.iter().copied().filter(|t| *t != "firma").collect())
    } else {
        let alle: Vec<&str> = LISTEN.iter().chain(EINZELN.iter()).copied().collect();
        (alle.clone(), alle)
    }
}

fn jetzt() -> f64 {
    js_sys::Date::now()
}

fn uhrzeit() -> String {
    let d = js_sys::Date::new_0();
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

struct Antwort {
    status: u16,
    text: String,
}

async fn senden(methode: &str, pfad: &str, body: Option<JsValue>, token: Option<&str>, kopf: &[(&str, &str)]) -> Result<Response, String> {
    let init = RequestInit::new();
    init.set_method(methode);
    let h = Headers::new().map_err(|_| "Headers")?;
    let _ = h.set("apikey", KEY);
    if let Some(t) = token {
        let _ = h.set("Authorization", &format!("Bearer {t}"));
    }
    for (k, v) in kopf {
        let _ = h.set(k, v);
    }
    init.set_headers(&h);
    if let Some(b) = body {
        init.set_body(&b);
    }
    let req = Request::new_with_str_and_init(&format!("{URL}{pfad}"), &init).map_err(|_| "Anfrage ungültig".to_string())?;
    let r = JsFuture::from(window().fetch_with_request(&req)).await.map_err(|_| "offline".to_string())?;
    r.dyn_into::<Response>().map_err(|_| "Antwort ungültig".to_string())
}

async fn json(methode: &str, pfad: &str, body: Option<String>, token: Option<&str>, kopf: &[(&str, &str)]) -> Result<Antwort, String> {
    let mut k: Vec<(&str, &str)> = vec![("Content-Type", "application/json")];
    k.extend_from_slice(kopf);
    let r = senden(methode, pfad, body.map(|b| JsValue::from_str(&b)), token, &k).await?;
    let status = r.status();
    let text = match r.text() {
        Ok(p) => JsFuture::from(p).await.ok().and_then(|v| v.as_string()).unwrap_or_default(),
        Err(_) => String::new(),
    };
    Ok(Antwort { status, text })
}

/// Fehlertext aus einer Supabase-Antwort.
fn fehlertext(a: &Antwort) -> String {
    let v: serde_json::Value = serde_json::from_str(&a.text).unwrap_or_default();
    let roh = ["msg", "message", "error_description", "error"]
        .iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .unwrap_or("")
        .to_string();
    let l = roh.to_lowercase();
    if l.contains("invalid login") {
        "E-Mail oder Passwort falsch".into()
    } else if l.contains("email not confirmed") {
        "Bitte zuerst die E-Mail bestätigen (Link im Postfach), dann anmelden".into()
    } else if l.contains("already registered") || l.contains("already been registered") {
        "Für diese E-Mail gibt es schon ein Konto – bitte anmelden".into()
    } else if l.contains("password") && l.contains("6") {
        "Das Passwort muss mindestens 6 Zeichen haben".into()
    } else if l.contains("einladungscode") {
        "Einladungscode ungültig oder abgelaufen".into()
    } else if roh.is_empty() {
        format!("Fehler {}", a.status)
    } else {
        roh
    }
}

fn sitzung_aus(text: &str, alt: Option<&Sitzung>) -> Option<Sitzung> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let access = v.get("access_token")?.as_str()?.to_string();
    let refresh = v.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let dauer = v.get("expires_in").and_then(|x| x.as_f64()).unwrap_or(3600.0);
    let user = v.get("user");
    let mut s = alt.cloned().unwrap_or_default();
    s.access = access;
    s.refresh = refresh;
    s.ablauf = jetzt() + dauer * 1000.0;
    if let Some(u) = user {
        s.user_id = u.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        s.email = u.get("email").and_then(|x| x.as_str()).unwrap_or("").to_string();
    }
    Some(s)
}

/// Gültigen Zugang besorgen (bei Bedarf erneuern).
async fn token() -> Result<String, String> {
    let s = sitzung().ok_or("nicht angemeldet")?;
    if s.ablauf - jetzt() > 60_000.0 {
        return Ok(s.access);
    }
    let a = json("POST", "/auth/v1/token?grant_type=refresh_token", Some(serde_json::json!({ "refresh_token": s.refresh }).to_string()), None, &[]).await?;
    if a.status >= 400 {
        if a.status == 400 || a.status == 401 {
            sitzung_setzen(None);
            with_app(|app| {
                app.toast("Anmeldung abgelaufen – bitte unter Einstellungen → Konto & Team neu anmelden");
                app.render();
            });
        }
        return Err(fehlertext(&a));
    }
    let neu = sitzung_aus(&a.text, Some(&s)).ok_or("Antwort ungültig")?;
    let t = neu.access.clone();
    sitzung_setzen(Some(neu));
    Ok(t)
}

async fn rpc(name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let t = token().await?;
    let a = json("POST", &format!("/rest/v1/rpc/{name}"), Some(args.to_string()), Some(&t), &[]).await?;
    if a.status >= 400 {
        return Err(fehlertext(&a));
    }
    Ok(serde_json::from_str(&a.text).unwrap_or(serde_json::Value::Null))
}

// ---------------------------------------------------------------------------
// Start, Konto, Betrieb
// ---------------------------------------------------------------------------

/// Beim Start: gespeicherte Sitzung laden, regelmäßig abgleichen.
pub fn init() {
    let s = ls_get(LS_SITZUNG).and_then(|j| serde_json::from_str::<Sitzung>(&j).ok());
    let st = ls_get(LS_STAND).and_then(|j| serde_json::from_str::<Stand>(&j).ok()).unwrap_or_default();
    Z.with(|z| {
        let mut z = z.borrow_mut();
        z.sitzung = s;
        z.stand = st;
    });
    fn takt() {
        if verbunden() && dom().visibility_state() == web_sys::VisibilityState::Visible {
            abgleichen();
        }
        set_timeout(60_000, takt);
    }
    // Mitgliedschaft beim Start prüfen (entfernt? Betrieb gelöscht? Rolle geändert?)
    if sitzung().is_some_and(|s| !s.betrieb.is_empty()) {
        spawn_local(async {
            let vorher = rolle();
            if betrieb_laden().await.is_ok() && rolle() != vorher {
                if rolle().is_empty() {
                    Z.with(|z| z.borrow_mut().stand = Stand::default());
                    ls_del(LS_STAND);
                }
                with_app(|a| {
                    a.toast(if rolle().is_empty() { "Du bist keinem Betrieb mehr zugeordnet – der Abgleich ist beendet" } else { "Deine Rolle im Betrieb hat sich geändert" });
                    a.go_root(if ist_monteur() { super::View::MeinTag } else { super::View::Start });
                });
            }
        });
    }
    set_timeout(1500, takt);
    super::on(&window().into(), "online", |_| abgleichen());
    super::on(&dom().into(), "focusout", |_| {
        set_timeout(50, || {
            let neu = Z.with(|z| z.borrow().neu_zeichnen);
            if neu && !feld_aktiv() {
                Z.with(|z| z.borrow_mut().neu_zeichnen = false);
                with_app(|a| a.render());
            }
        })
    });
}

/// Nach einer lokalen Änderung (verzögert) abgleichen.
pub fn planen() {
    if !verbunden() {
        return;
    }
    let g = Z.with(|z| {
        let mut z = z.borrow_mut();
        z.geplant = z.geplant.wrapping_add(1);
        z.geplant
    });
    set_timeout(2500, move || {
        if Z.with(|z| z.borrow().geplant) == g {
            abgleichen();
        }
    });
}

fn feld_aktiv() -> bool {
    dom().active_element().map(|e| matches!(e.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")).unwrap_or(false)
}

fn eingabe(id: &str) -> String {
    super::query(&format!("#{id}")).map(|e| super::element_value(&e)).unwrap_or_default()
}

/// Nach der Anmeldung: Mitgliedschaft laden.
async fn betrieb_laden() -> Result<(), String> {
    let s = sitzung().ok_or("nicht angemeldet")?;
    let t = token().await?;
    let a = json("GET", &format!("/rest/v1/mitglieder?select=betrieb_id,rolle,betriebe(name)&user_id=eq.{}", s.user_id), None, Some(&t), &[]).await?;
    if a.status >= 400 {
        return Err(fehlertext(&a));
    }
    let v: serde_json::Value = serde_json::from_str(&a.text).unwrap_or_default();
    let mut s = sitzung().unwrap_or(s);
    match v.as_array().and_then(|x| x.first()) {
        Some(m) => {
            s.betrieb = m.get("betrieb_id").and_then(|x| x.as_str()).unwrap_or("").into();
            s.rolle = m.get("rolle").and_then(|x| x.as_str()).unwrap_or("").into();
            s.betrieb_name = m.get("betriebe").and_then(|b| b.get("name")).and_then(|x| x.as_str()).unwrap_or("").into();
        }
        None => {
            s.betrieb.clear();
            s.rolle.clear();
        }
    }
    sitzung_setzen(Some(s));
    Ok(())
}

/// Gerätenummer holen (eindeutige ID-Bereiche je Gerät).
async fn geraet_anmelden() -> Result<(), String> {
    let s = sitzung().ok_or("nicht angemeldet")?;
    let n = rpc("geraet_anmelden", serde_json::json!({ "b": s.betrieb })).await?;
    let nr = n.as_u64().ok_or("keine Gerätenummer")?;
    with_app(|a| {
        a.data.geraet = nr;
        a.save_now();
    });
    Ok(())
}

impl App {
    fn konto_fehler(&mut self, e: &str) {
        if let Some(f) = el("konto-fehler") {
            f.set_text_content(Some(e));
        }
        self.toast(e);
    }

    pub fn cloud_action(&mut self, act: &str, v: &str) -> bool {
        match act {
            "konto-modus" => {
                self.ui.konto_modus = v.to_string();
                self.render();
            }
            "konto-login" | "konto-registrieren" => {
                let (email, pw) = (eingabe("k-email").trim().to_string(), eingabe("k-pw"));
                if email.is_empty() || pw.is_empty() {
                    self.konto_fehler("Bitte E-Mail und Passwort eingeben");
                    return true;
                }
                let neu = act == "konto-registrieren";
                set_save_state("Verbinde …");
                spawn_local(async move {
                    let pfad = if neu { "/auth/v1/signup" } else { "/auth/v1/token?grant_type=password" };
                    let r = json("POST", pfad, Some(serde_json::json!({ "email": email, "password": pw }).to_string()), None, &[]).await;
                    match r {
                        Err(_) => {
                            with_app(|a| a.konto_fehler("Keine Verbindung – bitte Internet prüfen"));
                        }
                        Ok(a) if a.status >= 400 => {
                            let t = fehlertext(&a);
                            with_app(|app| app.konto_fehler(&t));
                        }
                        Ok(a) => match sitzung_aus(&a.text, None) {
                            Some(s) => {
                                sitzung_setzen(Some(s));
                                let r = betrieb_laden().await;
                                with_app(|app| {
                                    if let Err(e) = &r {
                                        app.toast(e);
                                    }
                                    app.render();
                                });
                                if verbunden() {
                                    nach_beitritt(false).await;
                                }
                            }
                            None => {
                                with_app(|app| {
                                    app.ui.konto_modus = "login".into();
                                    app.render();
                                    app.konto_fehler("Fast geschafft: Bitte den Bestätigungslink in deiner E-Mail öffnen und dich dann hier anmelden.");
                                });
                            }
                        },
                    }
                    set_save_state("Gespeichert");
                });
            }
            "betrieb-anlegen" => {
                let name = eingabe("k-firma");
                let ich = eingabe("k-name");
                spawn_local(async move {
                    match rpc("betrieb_anlegen", serde_json::json!({ "firmenname": name, "mein_name": ich })).await {
                        Ok(_) => {
                            let _ = betrieb_laden().await;
                            nach_beitritt(false).await;
                        }
                        Err(e) => {
                            with_app(|a| a.konto_fehler(&e));
                        }
                    }
                });
            }
            "betrieb-beitreten" => {
                let code = eingabe("k-code").trim().to_uppercase();
                let ich = eingabe("k-name");
                if code.len() < 6 {
                    self.konto_fehler("Bitte den Einladungscode eingeben");
                    return true;
                }
                self.ask(
                    "Betrieb beitreten?",
                    "Die Daten auf diesem Gerät werden durch die Daten des Betriebs ersetzt. Falls hier eigene Daten liegen: vorher ein Backup speichern.",
                    "Beitreten",
                    false,
                    move |_| {
                        spawn_local(async move {
                            match rpc("einladung_einloesen", serde_json::json!({ "einladungscode": code, "mein_name": ich })).await {
                                Ok(_) => {
                                    let _ = betrieb_laden().await;
                                    nach_beitritt(true).await;
                                }
                                Err(e) => {
                                    with_app(|a| a.konto_fehler(&e));
                                }
                            }
                        });
                    },
                );
            }
            "einladen" => {
                let rolle = if v == "chef" { "chef" } else { "monteur" }.to_string();
                spawn_local(async move {
                    let b = sitzung().map(|s| s.betrieb).unwrap_or_default();
                    match rpc("einladung_erstellen", serde_json::json!({ "b": b, "neue_rolle": rolle })).await {
                        Ok(c) => {
                            let code = c.as_str().unwrap_or("").to_string();
                            with_app(|a| a.einladung_zeigen(&code, &rolle));
                        }
                        Err(e) => {
                            with_app(|a| a.toast(&e));
                        }
                    }
                });
            }
            "sync-jetzt" => abgleichen(),
            "betrieb-loeschen" => {
                self.ask(
                    "Betrieb in der Cloud löschen?",
                    "Alle Daten des Betriebs in der Cloud werden endgültig gelöscht, alle Mitglieder verlieren den Zugang. Die Daten auf diesem Gerät bleiben erhalten.",
                    "Endgültig löschen",
                    true,
                    |_| {
                        spawn_local(async {
                            let b = sitzung().map(|s| s.betrieb).unwrap_or_default();
                            let r = match token().await {
                                Ok(t) => json("DELETE", &format!("/rest/v1/betriebe?id=eq.{b}"), None, Some(&t), &[]).await,
                                Err(e) => Err(e),
                            };
                            match r {
                                Ok(a) if a.status < 400 => {
                                    if let Some(mut s) = sitzung() {
                                        s.betrieb.clear();
                                        s.rolle.clear();
                                        s.betrieb_name.clear();
                                        sitzung_setzen(Some(s));
                                    }
                                    Z.with(|z| z.borrow_mut().stand = Stand::default());
                                    ls_del(LS_STAND);
                                    with_app(|a| {
                                        a.data.geraet = 0;
                                        a.save_now();
                                        a.render();
                                        a.toast("Betrieb in der Cloud gelöscht");
                                    });
                                }
                                Ok(a) => {
                                    let t = fehlertext(&a);
                                    with_app(|app| app.toast(&t));
                                }
                                Err(e) => {
                                    with_app(|app| app.toast(&e));
                                }
                            }
                        });
                    },
                );
            }
            "konto-abmelden" => {
                self.ask(
                    "Abmelden?",
                    "Die Daten bleiben auf diesem Gerät, werden aber nicht mehr abgeglichen.",
                    "Abmelden",
                    true,
                    |a| {
                        let t = sitzung().map(|s| s.access).unwrap_or_default();
                        spawn_local(async move {
                            let _ = json("POST", "/auth/v1/logout", None, Some(&t), &[]).await;
                        });
                        sitzung_setzen(None);
                        Z.with(|z| z.borrow_mut().stand = Stand::default());
                        ls_del(LS_STAND);
                        a.render();
                        a.toast("Abgemeldet");
                    },
                );
            }
            "konto-info" => self.konto_modal(),
            _ => return false,
        }
        true
    }

    fn einladung_zeigen(&mut self, code: &str, rolle: &str) {
        let wer = if rolle == "chef" { "weiteres Gerät (Chef)" } else { "Monteur" };
        let text = format!(
            "Hallo,%0A%0Abitte installiere Zento (https://cyrexmv.github.io/WWP/), lege unter Einstellungen → Konto & Team ein Konto an und tritt mit diesem Code bei:%0A%0A{code}%0A%0AGültig 7 Tage."
        );
        let html = format!(
            "<h3>Einladung für {wer}</h3><p class=\"muted\">Auf dem anderen Gerät: Zento öffnen → Einstellungen → <b>Konto &amp; Team</b> → Konto anlegen → „Einladungscode“ eingeben.</p><div class=\"code-gross\">{}</div><p class=\"hint\">Gültig 7 Tage, nur einmal verwendbar.</p><div class=\"modal-actions\"><a class=\"btn ghost\" href=\"mailto:?subject=Einladung%20zu%20Zento&body={text}\">{}<span>Per E-Mail</span></a>{}</div>",
            esc(code),
            ic("mail"),
            btn("modal-close", "check", "Fertig", "primary", None, "")
        );
        self.modal(&html);
    }

    /// Kontofenster für Monteure (dort gibt es keine Einstellungen).
    fn konto_modal(&mut self) {
        let s = sitzung().unwrap_or_default();
        let z = Z.with(|z| (z.borrow().zuletzt.clone(), z.borrow().fehler.clone()));
        let html = format!(
            "<h3>Konto</h3><p>{}<br><span class=\"muted\">{} · Monteur</span></p><p class=\"muted small\">{}</p><div class=\"modal-actions\">{}{}{}</div>",
            esc(&s.email),
            esc(&s.betrieb_name),
            if !z.1.is_empty() { esc(&z.1) } else if z.0.is_empty() { "Noch nicht abgeglichen".into() } else { format!("Zuletzt abgeglichen um {}", z.0) },
            btn("konto-abmelden", "logout", "Abmelden", "ghost danger-text", None, ""),
            btn("sync-jetzt", "undo", "Jetzt abgleichen", "ghost", None, ""),
            btn("modal-close", "", "Schließen", "primary", None, "")
        );
        self.modal(&html);
    }

    /// Einstellungen → Konto & Team
    pub fn sec_konto(&self) -> String {
        let s = sitzung();
        let mut h = String::new();
        let fehler = "<p class=\"konto-fehler\" id=\"konto-fehler\"></p>";
        match s {
            None => {
                let neu = self.ui.konto_modus != "login";
                let _ = write!(
                    h,
                    "<section class=\"group\"><div class=\"group-head\"><h3>Daten in der Cloud</h3><p>Mit einem Konto sind deine Daten sicher in der Cloud (Server in Frankfurt) und auf allen Geräten gleich – Chef im Büro, Monteure mit dem Tablet. Ohne Internet arbeitet die App ganz normal weiter und gleicht später ab.</p></div><div class=\"group-body\"><div class=\"pad-top\"><div class=\"segmented\"><button class=\"{}\" data-act=\"konto-modus\" data-v=\"neu\">Konto anlegen</button><button class=\"{}\" data-act=\"konto-modus\" data-v=\"login\">Anmelden</button></div></div><div class=\"form\"><label class=\"field\"><span>E-Mail</span><input type=\"email\" id=\"k-email\" autocomplete=\"email\" inputmode=\"email\"></label><label class=\"field\"><span>Passwort{}</span><input type=\"password\" id=\"k-pw\" autocomplete=\"{}\"></label></div>{fehler}<div class=\"pad-top\">{}</div></div></section>",
                    if neu { "active" } else { "" },
                    if neu { "" } else { "active" },
                    if neu { " (mind. 6 Zeichen)" } else { "" },
                    if neu { "new-password" } else { "current-password" },
                    if neu { btn("konto-registrieren", "check", "Konto anlegen", "primary", None, "") } else { btn("konto-login", "check", "Anmelden", "primary", None, "") }
                );
            }
            Some(s) if s.betrieb.is_empty() => {
                let _ = write!(
                    h,
                    "<section class=\"group\"><div class=\"group-head\"><h3>Angemeldet als {}</h3><p>Jetzt noch einen Betrieb anlegen – oder mit dem Einladungscode deines Chefs beitreten.</p></div><div class=\"group-body\"><div class=\"form\"><label class=\"field wide\"><span>Dein Name</span><input type=\"text\" id=\"k-name\" placeholder=\"Vor- und Nachname\"></label></div></div></section>",
                    esc(&s.email)
                );
                let _ = write!(
                    h,
                    "<section class=\"group\"><div class=\"group-head\"><h3>Neuen Betrieb anlegen</h3><p>Du wirst Chef. Die Daten auf diesem Gerät werden in die Cloud übertragen.</p></div><div class=\"group-body\"><div class=\"form\"><label class=\"field wide\"><span>Firmenname</span><input type=\"text\" id=\"k-firma\" value=\"{}\"></label></div><div class=\"pad-top\">{}</div></div></section>",
                    esc(self.data.firma.name.trim()),
                    btn("betrieb-anlegen", "building", "Betrieb anlegen", "primary", None, "")
                );
                let _ = write!(
                    h,
                    "<section class=\"group\"><div class=\"group-head\"><h3>Mit Einladungscode beitreten</h3><p>Den Code bekommst du vom Chef (Einstellungen → Konto &amp; Team → Einladen).</p></div><div class=\"group-body\"><div class=\"form\"><label class=\"field wide\"><span>Einladungscode</span><input type=\"text\" id=\"k-code\" autocapitalize=\"characters\" placeholder=\"z. B. K7Q2MZ4P\"></label></div>{fehler}<div class=\"pad-top\">{}{}</div></div></section>",
                    btn("betrieb-beitreten", "users", "Beitreten", "primary", None, ""),
                    btn("konto-abmelden", "logout", "Abmelden", "ghost", None, "")
                );
            }
            Some(s) => {
                let z = Z.with(|z| (z.borrow().zuletzt.clone(), z.borrow().fehler.clone(), z.borrow().laeuft));
                let status = if z.2 {
                    "<span class=\"status-pill info\">Gleicht ab …</span>".to_string()
                } else if !z.1.is_empty() {
                    format!("<span class=\"status-pill warn\">{} {}</span>", ic("alert"), esc(&z.1))
                } else if z.0.is_empty() {
                    "<span class=\"status-pill info\">Noch nicht abgeglichen</span>".to_string()
                } else {
                    format!("<span class=\"status-pill ok\">{} Abgeglichen um {}</span>", ic("check"), z.0)
                };
                let _ = write!(
                    h,
                    "<section class=\"group\"><div class=\"group-head row-head\"><div><h3>{}</h3><p>{} · {}</p></div>{}</div><div class=\"group-body\"><div class=\"opt-row\"><div><b>Abgleich</b><span id=\"sync-status\">{status}</span></div>{}</div></div></section>",
                    esc(if s.betrieb_name.is_empty() { "Betrieb" } else { &s.betrieb_name }),
                    esc(&s.email),
                    if s.rolle == "chef" { "Chef" } else { "Monteur" },
                    btn("konto-abmelden", "logout", "Abmelden", "ghost sm", None, ""),
                    btn("sync-jetzt", "undo", "Jetzt abgleichen", "ghost sm", None, "")
                );
                if s.rolle == "chef" {
                    let _ = write!(
                        h,
                        "<section class=\"group\"><div class=\"group-head\"><h3>Team</h3><p>Monteure sehen Kunden, Anlagen, Wartungen und Termine – keine Preise, Rechnungen oder Finanzen. Das regelt der Server, nicht nur die App.</p></div><div class=\"group-body\"><div class=\"rows\" id=\"team-liste\"><p class=\"muted pad\">Lade …</p></div><div class=\"pad-top\">{}{}</div></div></section>",
                        btn("einladen", "users", "Monteur einladen", "primary", None, "monteur"),
                        btn("einladen", "plus", "Weiteres Chef-Gerät", "ghost", None, "chef")
                    );
                    let _ = write!(
                        h,
                        "<section class=\"group danger-zone\"><div class=\"group-head row-head\"><div><h3>Betrieb in der Cloud löschen</h3><p>Löscht alle Cloud-Daten des Betriebs und alle Zugänge. Die Daten auf diesem Gerät bleiben.</p></div>{}</div></section>",
                        btn("betrieb-loeschen", "trash", "Löschen", "danger sm", None, "")
                    );
                }
            }
        }
        h
    }

    /// Teamliste nachladen (nach dem Zeichnen der Einstellungen).
    pub fn team_laden(&self) {
        if el("team-liste").is_none() {
            return;
        }
        spawn_local(async {
            let Ok(t) = token().await else { return };
            let b = sitzung().map(|s| s.betrieb).unwrap_or_default();
            let Ok(a) = json("GET", &format!("/rest/v1/mitglieder?select=name,email,rolle&betrieb_id=eq.{b}&order=rolle"), None, Some(&t), &[]).await else { return };
            let v: serde_json::Value = serde_json::from_str(&a.text).unwrap_or_default();
            let mut h = String::new();
            for m in v.as_array().into_iter().flatten() {
                let name = m.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let email = m.get("email").and_then(|x| x.as_str()).unwrap_or("");
                let rolle = m.get("rolle").and_then(|x| x.as_str()).unwrap_or("");
                let _ = write!(
                    h,
                    "<div class=\"row static\"><div><b>{}</b><span>{}</span></div><span class=\"badge st-{}\">{}</span></div>",
                    esc(if name.is_empty() { email } else { name }),
                    esc(email),
                    if rolle == "chef" { "versendet" } else { "anfrage" },
                    if rolle == "chef" { "Chef" } else { "Monteur" }
                );
            }
            super::set_html("team-liste", &h);
        });
    }
}

/// Nach Anlegen/Beitreten: Gerät anmelden, ggf. lokale Daten ersetzen, ersten Abgleich starten.
async fn nach_beitritt(ersetzen: bool) {
    if ersetzen {
        with_app(|a| {
            let alt = std::mem::take(&mut a.data);
            a.data = Data::new();
            a.data.nav_reihenfolge = alt.nav_reihenfolge;
            a.data.nav_aus = alt.nav_aus;
            a.data.tipps_aus = alt.tipps_aus;
            a.set_logo_bytes(None);
            a.save_now();
        });
    }
    Z.with(|z| z.borrow_mut().stand = Stand::default());
    stand_speichern();
    if let Err(e) = geraet_anmelden().await {
        with_app(|a| a.toast(&e));
        return;
    }
    with_app(|a| {
        if ist_monteur() {
            a.go_root(super::View::MeinTag);
        } else {
            a.render();
        }
        a.toast("Verbunden – Daten werden abgeglichen");
    });
    abgleichen();
}

// ---------------------------------------------------------------------------
// Abgleich
// ---------------------------------------------------------------------------

/// Abgleich starten (läuft er schon, folgt direkt danach ein weiterer).
pub fn abgleichen() {
    if !verbunden() {
        return;
    }
    let schon = Z.with(|z| {
        let mut z = z.borrow_mut();
        if z.laeuft {
            z.nochmal = true;
            true
        } else {
            z.laeuft = true;
            false
        }
    });
    if schon {
        return;
    }
    spawn_local(async {
        let ergebnis = abgleich_lauf().await;
        let nochmal = Z.with(|z| {
            let mut z = z.borrow_mut();
            z.laeuft = false;
            match &ergebnis {
                Ok(()) => {
                    z.fehler.clear();
                    z.zuletzt = uhrzeit();
                }
                Err(e) => z.fehler = e.clone(),
            }
            std::mem::take(&mut z.nochmal)
        });
        stand_speichern();
        match &ergebnis {
            Ok(()) => set_save_state(&format!("Abgeglichen {}", uhrzeit())),
            Err(e) if e == "offline" => set_save_state("Offline – wird später übertragen"),
            Err(_) => set_save_state("Abgleich fehlgeschlagen"),
        }
        if let Some(s) = el("sync-status") {
            let html = match &ergebnis {
                Ok(()) => format!("<span class=\"status-pill ok\">{} Abgeglichen um {}</span>", ic("check"), uhrzeit()),
                Err(e) => format!("<span class=\"status-pill warn\">{} {}</span>", ic("alert"), esc(e)),
            };
            s.set_inner_html(&html);
        }
        if nochmal {
            abgleichen();
        }
    });
}

async fn abgleich_lauf() -> Result<(), String> {
    let s = sitzung().ok_or("nicht angemeldet")?;
    let b = s.betrieb.clone();
    let (lesen, schreiben) = typen();
    // 1. Holen
    loop {
        let t = token().await?;
        let cursor = Z.with(|z| z.borrow().stand.cursor.clone());
        let filter = if cursor.is_empty() { String::new() } else { format!("&geaendert=gt.{}", String::from(js_sys::encode_uri_component(&cursor))) };
        let a = json(
            "GET",
            &format!("/rest/v1/eintraege?select=typ,id,daten,geloescht,geaendert&betrieb_id=eq.{b}{filter}&order=geaendert.asc&limit=500"),
            None,
            Some(&t),
            &[],
        )
        .await?;
        if a.status >= 400 {
            return Err(fehlertext(&a));
        }
        let zeilen: Vec<Zeile> = serde_json::from_str(&a.text).map_err(|_| "Antwort unlesbar".to_string())?;
        let anzahl = zeilen.len();
        if anzahl > 0 {
            let lesen = lesen.clone();
            with_app(|app| {
                let n = Z.with(|z| sync::anwenden(&mut app.data, &zeilen, &mut z.borrow_mut().stand, &lesen));
                if n > 0 {
                    app.save_now();
                    if feld_aktiv() {
                        Z.with(|z| z.borrow_mut().neu_zeichnen = true);
                    } else {
                        app.render();
                    }
                }
            });
        }
        if anzahl < 500 {
            break;
        }
    }
    // 2. Hochladen
    let aenderungen = with_app(|app| Z.with(|z| sync::aenderungen(&app.data, &z.borrow().stand, &lesen, &schreiben))).unwrap_or_default();
    for teil in aenderungen.chunks(200) {
        let t = token().await?;
        let a = json(
            "POST",
            "/rest/v1/eintraege?on_conflict=betrieb_id,typ,id",
            Some(sync::zeilen_json(&b, teil)),
            Some(&t),
            &[("Prefer", "resolution=merge-duplicates,return=minimal")],
        )
        .await?;
        if a.status >= 400 {
            return Err(fehlertext(&a));
        }
        Z.with(|z| sync::hochgeladen(&mut z.borrow_mut().stand, teil));
    }
    // 3. Dateien
    dateien_abgleichen(&b).await
}

async fn dateien_abgleichen(b: &str) -> Result<(), String> {
    let monteur = ist_monteur();
    let liste = with_app(|a| sync::datei_arten(&a.data)).unwrap_or_default();
    for (id, art) in liste {
        if monteur && art == "beleg" {
            continue;
        }
        let pfad = format!("{b}/{art}/{id}");
        let oben = Z.with(|z| z.borrow().stand.dateien_oben.contains(&id));
        let lokal = load_file(id).await;
        match lokal {
            Some(bytes) if !oben => {
                // hochladen (Monteure nur Fotos)
                if monteur && art != "foto" {
                    continue;
                }
                let mime = with_app(|a| a.data.datei(Some(id)).map(|f| f.mime.clone())).flatten().unwrap_or_else(|| "application/octet-stream".into());
                let t = token().await?;
                let r = senden(
                    "POST",
                    &format!("/storage/v1/object/dateien/{pfad}"),
                    Some(js_sys::Uint8Array::from(&bytes[..]).into()),
                    Some(&t),
                    &[("Content-Type", &mime), ("x-upsert", "true")],
                )
                .await?;
                if r.status() < 400 {
                    Z.with(|z| z.borrow_mut().stand.dateien_oben.push(id));
                }
            }
            Some(_) => {}
            None => {
                // herunterladen
                let t = token().await?;
                let r = senden("GET", &format!("/storage/v1/object/authenticated/dateien/{pfad}"), None, Some(&t), &[]).await?;
                if r.status() >= 400 {
                    continue;
                }
                let mime = r.headers().get("content-type").ok().flatten().unwrap_or_default();
                let Ok(p) = r.array_buffer() else { continue };
                let Ok(buf) = JsFuture::from(p).await else { continue };
                let bytes = js_sys::Uint8Array::new(&buf).to_vec();
                if storage::datei_ablegen(id, &bytes) {
                    Z.with(|z| z.borrow_mut().stand.dateien_oben.push(id));
                    with_app(|a| {
                        if a.data.datei(Some(id)).is_none() {
                            a.data.dateien.push(Datei { id, name: format!("{art}-{id}"), mime: mime.clone(), groesse: bytes.len() as u64 });
                        }
                        if art == "logo" {
                            a.set_logo_bytes(Some(bytes.clone()));
                            a.render();
                        } else if art == "foto" {
                            super::wartung::fotos_laden();
                        }
                    });
                }
            }
        }
    }
    Ok(())
}
