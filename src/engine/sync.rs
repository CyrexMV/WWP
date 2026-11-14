//! Abgleich mit der Cloud (Supabase): Die App arbeitet lokal weiter und tauscht im Hintergrund
//! einzelne Datensätze aus. Jeder Datensatz ist ein Eintrag (Typ, ID, JSON).
//!
//! Regeln:
//! - `stand` merkt sich je Eintrag den Prüfwert, der zuletzt mit dem Server übereinstimmte.
//! - Vom Server kommende Änderungen werden übernommen, wenn der Eintrag lokal seitdem nicht
//!   geändert wurde; lokale Änderungen gehen beim nächsten Hochladen an den Server (zuletzt gewinnt).
//! - Nummernkreise werden zusammengeführt (jeweils der höhere Zähler).

use super::model::{Data, Id};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Listen der App, die abgeglichen werden.
pub const LISTEN: &[&str] = &["kunden", "auftraege", "dokumente", "buchungen", "katalog", "dateien", "termine", "anlagen", "wartungen"];
/// Einzelobjekte (ID 0).
pub const EINZELN: &[&str] = &["firma", "zaehler"];
/// Was ein Monteur abgleicht (Server lässt nur diese zu; Firma nur lesend).
pub const MONTEUR: &[&str] = &["kunden", "termine", "anlagen", "wartungen", "firma", "zaehler"];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Stand {
    /// „typ:id“ → Prüfwert des zuletzt abgeglichenen Inhalts
    pub werte: BTreeMap<String, u64>,
    /// Zeitstempel des zuletzt geholten Server-Eintrags
    pub cursor: String,
    /// bereits hochgeladene Dateien
    pub dateien_oben: Vec<Id>,
}

/// Ein Eintrag vom Server.
#[derive(Deserialize, Clone, Debug)]
pub struct Zeile {
    pub typ: String,
    pub id: i64,
    #[serde(default)]
    pub daten: Value,
    #[serde(default)]
    pub geloescht: bool,
    #[serde(default)]
    pub geaendert: String,
}

/// Ein Eintrag zum Hochladen (`daten = None` → gelöscht).
#[derive(Clone, Debug, PartialEq)]
pub struct Aenderung {
    pub typ: String,
    pub id: Id,
    pub daten: Option<Value>,
}

fn schluessel(typ: &str, id: Id) -> String {
    format!("{typ}:{id}")
}

/// FNV-1a über den kanonischen JSON-Text.
pub fn pruefwert(v: &Value) -> u64 {
    let s = serde_json::to_string(v).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn erlaubt(typ: &str, typen: &[&str]) -> bool {
    typen.contains(&typ)
}

/// Alle abzugleichenden Einträge der lokalen Daten.
pub fn eintraege(d: &Data, typen: &[&str]) -> BTreeMap<String, (String, Id, Value)> {
    let v = serde_json::to_value(d).unwrap_or(Value::Null);
    let mut out = BTreeMap::new();
    for typ in LISTEN.iter().filter(|t| erlaubt(t, typen)) {
        if let Some(arr) = v.get(*typ).and_then(|a| a.as_array()) {
            for e in arr {
                if let Some(id) = e.get("id").and_then(|i| i.as_u64()) {
                    out.insert(schluessel(typ, id), (typ.to_string(), id, e.clone()));
                }
            }
        }
    }
    for typ in EINZELN.iter().filter(|t| erlaubt(t, typen)) {
        if let Some(e) = v.get(*typ) {
            out.insert(schluessel(typ, 0), (typ.to_string(), 0, e.clone()));
        }
    }
    out
}

/// Lokale Änderungen seit dem letzten Abgleich. `schreibbar` begrenzt, was hochgeladen wird.
pub fn aenderungen(d: &Data, stand: &Stand, typen: &[&str], schreibbar: &[&str]) -> Vec<Aenderung> {
    let lokal = eintraege(d, typen);
    let mut v = Vec::new();
    for (k, (typ, id, wert)) in &lokal {
        if !erlaubt(typ, schreibbar) {
            continue;
        }
        if stand.werte.get(k) != Some(&pruefwert(wert)) {
            v.push(Aenderung { typ: typ.clone(), id: *id, daten: Some(wert.clone()) });
        }
    }
    for k in stand.werte.keys() {
        if !lokal.contains_key(k) {
            let (typ, id) = k.split_once(':').unwrap_or(("", "0"));
            if erlaubt(typ, schreibbar) && !EINZELN.contains(&typ) {
                v.push(Aenderung { typ: typ.into(), id: id.parse().unwrap_or(0), daten: None });
            }
        }
    }
    v
}

/// Nach erfolgreichem Hochladen: Stand fortschreiben.
pub fn hochgeladen(stand: &mut Stand, a: &[Aenderung]) {
    for x in a {
        let k = schluessel(&x.typ, x.id);
        match &x.daten {
            Some(w) => {
                stand.werte.insert(k, pruefwert(w));
            }
            None => {
                stand.werte.remove(&k);
            }
        }
    }
}

/// Server-Einträge übernehmen. Gibt die Zahl geänderter lokaler Einträge zurück.
pub fn anwenden(d: &mut Data, zeilen: &[Zeile], stand: &mut Stand, typen: &[&str]) -> usize {
    if zeilen.is_empty() {
        return 0;
    }
    let lokal = eintraege(d, typen);
    let mut v = serde_json::to_value(&*d).unwrap_or(Value::Null);
    let mut n = 0;
    for z in zeilen {
        if z.geaendert.as_str() > stand.cursor.as_str() {
            stand.cursor = z.geaendert.clone();
        }
        if !erlaubt(&z.typ, typen) || z.id < 0 {
            continue;
        }
        let id = z.id as Id;
        let k = schluessel(&z.typ, id);
        let fern = if z.geloescht { None } else { Some(pruefwert(&z.daten)) };
        let lokal_wert = lokal.get(&k).map(|e| pruefwert(&e.2));
        let gemerkt = stand.werte.get(&k).copied();
        let einzeln = EINZELN.contains(&z.typ.as_str());
        // Zähler: immer den jeweils höheren Stand übernehmen
        if z.typ == "zaehler" {
            if let (Some(obj), Some(fern_obj)) = (v.get_mut("zaehler").and_then(|o| o.as_object_mut()), z.daten.as_object()) {
                for (key, val) in fern_obj {
                    let f = val.as_u64().unwrap_or(0);
                    let l = obj.get(key).and_then(|x| x.as_u64()).unwrap_or(0);
                    if f > l {
                        obj.insert(key.clone(), Value::from(f));
                        n += 1;
                    }
                }
            }
            stand.werte.insert(k, fern.unwrap_or(0));
            continue;
        }
        if lokal_wert == fern {
            match fern {
                Some(f) => stand.werte.insert(k, f),
                None => stand.werte.remove(&k),
            };
            continue;
        }
        // lokal seit dem letzten Abgleich geändert/gelöscht oder neu angelegt? → lokale Fassung behalten.
        // Einzelobjekte (Firma) gibt es auf jedem Gerät – ohne gemeinsamen Stand gilt der Server.
        let lokal_geaendert = match gemerkt {
            Some(g) => lokal_wert != Some(g),
            None => lokal_wert.is_some() && !einzeln,
        };
        if lokal_geaendert {
            continue;
        }
        if EINZELN.contains(&z.typ.as_str()) {
            if let (Some(obj), false) = (v.as_object_mut(), z.geloescht) {
                obj.insert(z.typ.clone(), z.daten.clone());
                n += 1;
            }
        } else if let Some(arr) = v.get_mut(z.typ.as_str()).and_then(|a| a.as_array_mut()) {
            let pos = arr.iter().position(|e| e.get("id").and_then(|i| i.as_u64()) == Some(id));
            match (pos, z.geloescht) {
                (Some(p), true) => {
                    arr.remove(p);
                }
                (Some(p), false) => arr[p] = z.daten.clone(),
                (None, false) => arr.push(z.daten.clone()),
                (None, true) => {}
            }
            n += 1;
        }
        match fern {
            Some(f) => stand.werte.insert(k, f),
            None => stand.werte.remove(&k),
        };
    }
    if n > 0 {
        if let Ok(neu) = serde_json::from_value::<Data>(v) {
            // lokale Einstellungen (Seitenleiste, Hinweise, Gerät) bleiben erhalten
            let lokal_alt = std::mem::replace(d, neu);
            d.nav_reihenfolge = lokal_alt.nav_reihenfolge;
            d.nav_aus = lokal_alt.nav_aus;
            d.tipps_aus = lokal_alt.tipps_aus;
            d.letztes_backup = lokal_alt.letztes_backup;
            d.geraet = lokal_alt.geraet;
            d.next_id = lokal_alt.next_id;
        }
    }
    n
}

/// JSON für das Hochladen (`eintraege`-Zeilen).
pub fn zeilen_json(betrieb: &str, a: &[Aenderung]) -> String {
    let arr: Vec<Value> = a
        .iter()
        .map(|x| {
            let mut m = Map::new();
            m.insert("betrieb_id".into(), Value::from(betrieb));
            m.insert("typ".into(), Value::from(x.typ.clone()));
            m.insert("id".into(), Value::from(x.id));
            m.insert("daten".into(), x.daten.clone().unwrap_or(Value::Object(Map::new())));
            m.insert("geloescht".into(), Value::from(x.daten.is_none()));
            Value::Object(m)
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".into())
}

/// Dateien und ihre Ablage („logo“, „beleg“, „foto“) – für Upload/Download.
pub fn datei_arten(d: &Data) -> Vec<(Id, &'static str)> {
    let mut v = Vec::new();
    if let Some(l) = d.firma.logo {
        v.push((l, "logo"));
    }
    for b in &d.buchungen {
        if let Some(f) = b.beleg {
            v.push((f, "beleg"));
        }
    }
    for a in &d.anlagen {
        if let Some(f) = a.foto {
            v.push((f, "foto"));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Data, DokArt};

    const ALLE: &[&str] = &["kunden", "auftraege", "dokumente", "buchungen", "katalog", "dateien", "termine", "anlagen", "wartungen", "firma", "zaehler"];

    /// Simuliert den Server: hält die zuletzt hochgeladenen Zeilen.
    fn hoch(server: &mut Vec<Zeile>, a: &[Aenderung], zeit: &str) {
        for x in a {
            server.retain(|z| !(z.typ == x.typ && z.id == x.id as i64));
            server.push(Zeile { typ: x.typ.clone(), id: x.id as i64, daten: x.daten.clone().unwrap_or(Value::Null), geloescht: x.daten.is_none(), geaendert: zeit.into() });
        }
    }
    fn seit<'a>(server: &'a [Zeile], cursor: &str) -> Vec<Zeile> {
        server.iter().filter(|z| z.geaendert.as_str() > cursor).cloned().collect()
    }

    #[test]
    fn zwei_geraete() {
        let mut server: Vec<Zeile> = Vec::new();
        // Gerät A (Chef) legt Daten an und lädt hoch
        let mut a = Data::new();
        a.geraet = 1;
        a.firma.name = "Heizung Muster".into();
        let k = a.neuer_kunde("2026-11-13");
        a.kunde_mut(k).unwrap().firma = "Bäckerei Schön".into();
        a.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-11-13");
        let mut sa = Stand::default();
        let ch = aenderungen(&a, &sa, ALLE, ALLE);
        assert!(ch.len() >= 4);
        hoch(&mut server, &ch, "t1");
        hochgeladen(&mut sa, &ch);
        assert!(aenderungen(&a, &sa, ALLE, ALLE).is_empty());

        // Gerät B (Monteur) holt nur seine Typen
        let mut b = Data::new();
        b.geraet = 2;
        let mut sb = Stand::default();
        let n = anwenden(&mut b, &seit(&server, &sb.cursor), &mut sb, MONTEUR);
        assert!(n >= 2);
        assert_eq!(b.kunden.len(), 1);
        assert_eq!(b.firma.name, "Heizung Muster");
        assert!(b.dokumente.is_empty(), "Monteur bekommt keine Rechnungen");
        assert_eq!(sb.cursor, "t1");
        // B ändert den Kunden und legt eine Anlage an – IDs kollidieren nicht
        b.kunde_mut(k).unwrap().telefon = "089 1".into();
        let an = b.neue_anlage(Some(k));
        assert!(an > (1 << 40));
        let ch = aenderungen(&b, &sb, MONTEUR, &["kunden", "termine", "anlagen", "wartungen"]);
        assert_eq!(ch.len(), 2);
        hoch(&mut server, &ch, "t2");
        hochgeladen(&mut sb, &ch);

        // A holt: Kunde aktualisiert, Anlage neu, Rechnung bleibt
        let n = anwenden(&mut a, &seit(&server, &sa.cursor), &mut sa, ALLE);
        assert_eq!(n, 2);
        assert_eq!(a.kunde(Some(k)).unwrap().telefon, "089 1");
        assert_eq!(a.anlagen.len(), 1);
        assert_eq!(a.dokumente.len(), 1);
        assert!(aenderungen(&a, &sa, ALLE, ALLE).is_empty(), "nach dem Holen nichts doppelt hochladen");

        // gleichzeitige Änderung: lokale Fassung bleibt und wird hochgeladen
        a.kunde_mut(k).unwrap().ort = "München".into();
        b.kunde_mut(k).unwrap().ort = "Berlin".into();
        let chb = aenderungen(&b, &sb, MONTEUR, MONTEUR);
        hoch(&mut server, &chb, "t3");
        hochgeladen(&mut sb, &chb);
        anwenden(&mut a, &seit(&server, &sa.cursor), &mut sa, ALLE);
        assert_eq!(a.kunde(Some(k)).unwrap().ort, "München");
        let cha = aenderungen(&a, &sa, ALLE, ALLE);
        assert_eq!(cha.len(), 1);
        hoch(&mut server, &cha, "t4");
        hochgeladen(&mut sa, &cha);
        anwenden(&mut b, &seit(&server, &sb.cursor), &mut sb, MONTEUR);
        assert_eq!(b.kunde(Some(k)).unwrap().ort, "München");

        // Löschen kommt an
        a.kunden.retain(|x| x.id != k);
        let ch = aenderungen(&a, &sa, ALLE, ALLE);
        assert_eq!(ch, vec![Aenderung { typ: "kunden".into(), id: k, daten: None }]);
        hoch(&mut server, &ch, "t5");
        hochgeladen(&mut sa, &ch);
        anwenden(&mut b, &seit(&server, &sb.cursor), &mut sb, MONTEUR);
        assert!(b.kunden.is_empty());
    }

    #[test]
    fn zaehler_zusammenfuehren() {
        let mut a = Data::new();
        a.zaehler.insert("RE-2026".into(), 7);
        a.zaehler.insert("WA-2026".into(), 2);
        let mut s = Stand::default();
        let fern = Zeile { typ: "zaehler".into(), id: 0, daten: serde_json::json!({"RE-2026": 5, "WA-2026": 4, "AB-2026": 1}), geloescht: false, geaendert: "t1".into() };
        anwenden(&mut a, &[fern], &mut s, ALLE);
        assert_eq!(a.zaehler["RE-2026"], 7);
        assert_eq!(a.zaehler["WA-2026"], 4);
        assert_eq!(a.zaehler["AB-2026"], 1);
    }
}
