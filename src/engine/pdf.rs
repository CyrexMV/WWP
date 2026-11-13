//! PDF-Erzeugung ohne externe Bibliotheken: Angebote/Rechnungen im DIN-5008-Briefbogen
//! (Logo, Anschriftfeld, Infoblock, Positionstabelle, Steuerausweis, Fußzeile mit Bankdaten)
//! sowie die Jahresübersicht der Finanzen.
//!
//! Schriften: die PDF-Standardschriften Helvetica/Helvetica-Bold in WinAnsi-Kodierung
//! (Umlaute, ß, € werden korrekt dargestellt) – dadurch sind die Dateien klein und überall lesbar.

use super::date;
use super::image;
use super::model::{Data, DokArt, Dokument, Firma, Kunde};
use super::money::{fmt_eur, fmt_num, fmt_qty, Cents};
use super::report::Auswertung;
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// PDF-Grundgerüst
// ---------------------------------------------------------------------------

struct Pdf {
    buf: Vec<u8>,
    offsets: Vec<usize>,
}

impl Pdf {
    fn new() -> Pdf {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
        Pdf { buf, offsets: Vec::new() }
    }

    fn reserve(&mut self) -> usize {
        self.offsets.push(0);
        self.offsets.len()
    }

    fn obj(&mut self, id: usize, body: &str) {
        self.offsets[id - 1] = self.buf.len();
        self.buf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", id, body).as_bytes());
    }

    fn stream(&mut self, id: usize, dict: &str, data: &[u8]) {
        let z = compress_to_vec_zlib(data, 6);
        self.offsets[id - 1] = self.buf.len();
        self.buf.extend_from_slice(format!("{} 0 obj\n<< {} /Filter /FlateDecode /Length {} >>\nstream\n", id, dict, z.len()).as_bytes());
        self.buf.extend_from_slice(&z);
        self.buf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn stream_raw(&mut self, id: usize, dict: &str, data: &[u8]) {
        self.offsets[id - 1] = self.buf.len();
        self.buf.extend_from_slice(format!("{} 0 obj\n<< {} /Length {} >>\nstream\n", id, dict, data.len()).as_bytes());
        self.buf.extend_from_slice(data);
        self.buf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn finish(mut self, root: usize, info: usize) -> Vec<u8> {
        let xref = self.buf.len();
        let mut s = format!("xref\n0 {}\n0000000000 65535 f \n", self.offsets.len() + 1);
        for o in &self.offsets {
            let _ = writeln!(s, "{:010} 00000 n ", o);
        }
        let _ = write!(
            s,
            "trailer\n<< /Size {} /Root {} 0 R /Info {} 0 R >>\nstartxref\n{}\n%%EOF\n",
            self.offsets.len() + 1,
            root,
            info,
            xref
        );
        self.buf.extend_from_slice(s.as_bytes());
        self.buf
    }
}

fn n(v: f32) -> String {
    let s = format!("{:.2}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.to_string()
    }
}

/// Zeichen → WinAnsi (cp1252).
fn winansi(ch: char) -> u8 {
    match ch {
        '€' => 0x80,
        '‚' => 0x82,
        '„' => 0x84,
        '…' => 0x85,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '™' => 0x99,
        '\t' => b' ',
        c if (c as u32) >= 0x20 && (c as u32) < 0x7f => c as u8,
        c if (c as u32) >= 0xa0 && (c as u32) <= 0xff => c as u32 as u8,
        _ => b'?',
    }
}

fn pdf_string(s: &str) -> String {
    let mut out = String::from("(");
    for ch in s.chars() {
        let b = winansi(ch);
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(b as char);
            }
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{:03o}", b);
            }
        }
    }
    out.push(')');
    out
}

/// Text als UTF-16BE-String (für Metadaten wie Titel/Autor).
fn pdf_utf16(s: &str) -> String {
    let mut out = String::from("<FEFF");
    for u in s.encode_utf16() {
        let _ = write!(out, "{:04X}", u);
    }
    out.push('>');
    out
}

// Laufweiten der Standardschriften (1/1000 em), Zeichen 32–126.
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278,
    278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722,
    667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556,
    556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];
const HELV_B: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333,
    333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722,
    667, 944, 667, 667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611, 611,
    611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

fn char_width(ch: char, bold: bool) -> u16 {
    let t = if bold { &HELV_B } else { &HELV };
    let base = match ch {
        'ä' | 'á' | 'à' | 'â' | 'å' | 'ã' => 'a',
        'Ä' | 'Á' | 'À' | 'Â' | 'Å' | 'Ã' => 'A',
        'ö' | 'ó' | 'ò' | 'ô' | 'õ' | 'ø' => 'o',
        'Ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ø' => 'O',
        'ü' | 'ú' | 'ù' | 'û' => 'u',
        'Ü' | 'Ú' | 'Ù' | 'Û' => 'U',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ç' => 'c',
        'Ç' => 'C',
        'ñ' => 'n',
        'ß' => return 611,
        '€' => return 556,
        '…' | '—' | '™' => return 1000,
        '–' => return 556,
        '„' | '“' | '”' => return if bold { 500 } else { 333 },
        '‚' | '‘' | '’' => return if bold { 278 } else { 222 },
        '•' => return 350,
        '²' | '³' => return 333,
        '§' => return 556,
        '°' => return 400,
        '·' => return 278,
        '×' => return 584,
        c => c,
    };
    let c = base as u32;
    if (32..127).contains(&c) {
        t[(c - 32) as usize]
    } else {
        556
    }
}

/// Textbreite in Punkt.
fn text_width(s: &str, size: f32, bold: bool) -> f32 {
    s.chars().map(|c| char_width(c, bold) as f32).sum::<f32>() * size / 1000.0
}

const MM: f32 = 72.0 / 25.4;
const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;

/// Zeilenumbruch nach Breite (mm); harte Umbrüche bleiben erhalten.
fn wrap(s: &str, width_mm: f32, size: f32, bold: bool) -> Vec<String> {
    let max = width_mm * MM;
    let mut out = Vec::new();
    for para in s.replace('\r', "").split('\n') {
        if para.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in para.split(' ') {
            let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if text_width(&cand, size, bold) <= max || line.is_empty() && text_width(word, size, bold) <= max {
                line = cand;
                continue;
            }
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            // überlanges Wort hart trennen
            let mut cur = String::new();
            for ch in word.chars() {
                cur.push(ch);
                if text_width(&cur, size, bold) > max {
                    cur.pop();
                    out.push(std::mem::take(&mut cur));
                    cur.push(ch);
                }
            }
            line = cur;
        }
        out.push(line);
    }
    out
}

fn hex_color(s: &str) -> [f32; 3] {
    let h = s.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let Ok(v) = u32::from_str_radix(h, 16) {
            return [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0];
        }
    }
    [0.18, 0.36, 0.83]
}

fn mix_white(c: [f32; 3], t: f32) -> [f32; 3] {
    [1.0 - (1.0 - c[0]) * t, 1.0 - (1.0 - c[1]) * t, 1.0 - (1.0 - c[2]) * t]
}

fn rgb(c: [f32; 3]) -> String {
    format!("{} {} {}", n(c[0]), n(c[1]), n(c[2]))
}

const BLACK: [f32; 3] = [0.12, 0.13, 0.16];
const GRAY: [f32; 3] = [0.42, 0.44, 0.49];
const LINE: [f32; 3] = [0.82, 0.83, 0.86];

/// Seiteninhalt in Millimeter-Koordinaten (y von oben).
#[derive(Default)]
struct Canvas {
    c: String,
}

impl Canvas {
    fn text(&mut self, x: f32, y: f32, size: f32, bold: bool, color: [f32; 3], s: &str) {
        if s.is_empty() {
            return;
        }
        let _ = writeln!(
            self.c,
            "BT /{} {} Tf {} rg {} {} Td {} Tj ET",
            if bold { "F2" } else { "F1" },
            n(size),
            rgb(color),
            n(x * MM),
            n((PAGE_H - y) * MM),
            pdf_string(s)
        );
    }

    fn text_right(&mut self, x: f32, y: f32, size: f32, bold: bool, color: [f32; 3], s: &str) {
        let w = text_width(s, size, bold) / MM;
        self.text(x - w, y, size, bold, color, s);
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 3]) {
        let _ = writeln!(self.c, "{} rg {} {} {} {} re f", rgb(color), n(x * MM), n((PAGE_H - y - h) * MM), n(w * MM), n(h * MM));
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: [f32; 3]) {
        let _ = writeln!(
            self.c,
            "{} RG {} w {} {} m {} {} l S",
            rgb(color),
            n(width),
            n(x1 * MM),
            n((PAGE_H - y1) * MM),
            n(x2 * MM),
            n((PAGE_H - y2) * MM)
        );
    }

    fn image(&mut self, name: &str, x: f32, y: f32, w: f32, h: f32) {
        let _ = writeln!(self.c, "q {} 0 0 {} {} {} cm /{} Do Q", n(w * MM), n(h * MM), n(x * MM), n((PAGE_H - y - h) * MM), name);
    }
}

/// Logo als XObject: (Objekt-ID, Breite px, Höhe px).
fn embed_logo(pdf: &mut Pdf, bytes: &[u8]) -> Option<(usize, u32, u32)> {
    if let Some((w, h, comps)) = image::jpeg_info(bytes) {
        let id = pdf.reserve();
        let cs = match comps {
            1 => "/DeviceGray",
            4 => "/DeviceCMYK /Decode [1 0 1 0 1 0 1 0]",
            _ => "/DeviceRGB",
        };
        pdf.stream_raw(
            id,
            &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace {cs} /BitsPerComponent 8 /Filter /DCTDecode"),
            bytes,
        );
        return Some((id, w, h));
    }
    let (w, h, rgba) = image::decode_png(bytes)?;
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let smask = if rgba.chunks_exact(4).any(|p| p[3] < 255) {
        let sid = pdf.reserve();
        let a: Vec<u8> = rgba.chunks_exact(4).map(|p| p[3]).collect();
        pdf.stream(sid, &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8"), &a);
        format!(" /SMask {sid} 0 R")
    } else {
        String::new()
    };
    let id = pdf.reserve();
    pdf.stream(id, &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8{smask}"), &rgb);
    Some((id, w, h))
}

/// Seiten zu einem PDF zusammensetzen.
fn assemble(pages: Vec<Canvas>, logo: Option<&[u8]>, title: &str, author: &str) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let catalog = pdf.reserve();
    let pages_id = pdf.reserve();
    let f1 = pdf.reserve();
    let f2 = pdf.reserve();
    let info = pdf.reserve();
    pdf.obj(f1, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    pdf.obj(f2, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>");
    let logo_obj = logo.and_then(|b| embed_logo(&mut pdf, b)).map(|l| l.0);
    let xobj = logo_obj.map(|id| format!(" /XObject << /Logo {id} 0 R >>")).unwrap_or_default();
    let resources = format!("<< /Font << /F1 {f1} 0 R /F2 {f2} 0 R >>{xobj} >>");
    let mut kids = Vec::new();
    for p in pages {
        let cid = pdf.reserve();
        pdf.stream(cid, "", p.c.as_bytes());
        let pid = pdf.reserve();
        pdf.obj(
            pid,
            &format!(
                "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {} {}] /Resources {resources} /Contents {cid} 0 R >>",
                n(PAGE_W * MM),
                n(PAGE_H * MM)
            ),
        );
        kids.push(format!("{pid} 0 R"));
    }
    pdf.obj(pages_id, &format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), kids.len()));
    pdf.obj(catalog, &format!("<< /Type /Catalog /Pages {pages_id} 0 R >>"));
    pdf.obj(info, &format!("<< /Title {} /Author {} /Producer (WWP Buero) >>", pdf_utf16(title), pdf_utf16(author)));
    pdf.finish(catalog, info)
}

/// Größe des Logos (mm), eingepasst in max. Breite × max. Höhe.
fn logo_box(bytes: &[u8], max_w: f32, max_h: f32) -> Option<(f32, f32)> {
    let (w, h) = image::size(bytes)?;
    if w == 0 || h == 0 {
        return None;
    }
    let s = (max_w / w as f32).min(max_h / h as f32);
    Some((w as f32 * s, h as f32 * s))
}

// ---------------------------------------------------------------------------
// Angebot / Rechnung
// ---------------------------------------------------------------------------

const LEFT: f32 = 25.0;
const RIGHT: f32 = 190.0;
const BOTTOM: f32 = 268.0;

/// Fußzeile mit Kontakt-, Bank- und Steuerdaten.
fn footer(c: &mut Canvas, f: &Firma, page: usize, total: usize) {
    c.line(LEFT, 274.0, RIGHT, 274.0, 0.4, LINE);
    let cols: [Vec<String>; 4] = [
        vec![f.name.clone(), f.zusatz.clone(), f.strasse.clone(), f.plz_ort.clone()],
        vec![
            if f.telefon.is_empty() { String::new() } else { format!("Tel. {}", f.telefon) },
            f.email.clone(),
            f.web.clone(),
        ],
        vec![
            f.bank.clone(),
            if f.iban.is_empty() { String::new() } else { format!("IBAN {}", f.iban) },
            if f.bic.is_empty() { String::new() } else { format!("BIC {}", f.bic) },
        ],
        vec![
            if f.steuernummer.is_empty() { String::new() } else { format!("Steuernr. {}", f.steuernummer) },
            if f.ust_id.is_empty() { String::new() } else { format!("USt-IdNr. {}", f.ust_id) },
        ],
    ];
    let xs = [LEFT, 64.0, 104.0, 157.0];
    let ws = [37.0, 38.0, 51.0, 33.0];
    for (i, col) in cols.iter().enumerate() {
        let mut y = 278.0;
        for l in col.iter().filter(|l| !l.trim().is_empty()) {
            for part in wrap(l, ws[i], 6.8, false) {
                c.text(xs[i], y, 6.8, false, GRAY, &part);
                y += 3.1;
            }
        }
    }
    if total > 1 {
        c.text_right(RIGHT, 291.0, 7.0, false, GRAY, &format!("Seite {page} von {total}"));
    }
}

/// Falz- und Lochmarken am linken Rand (DIN 5008).
fn fold_marks(c: &mut Canvas) {
    c.line(4.0, 105.0, 8.0, 105.0, 0.3, LINE);
    c.line(4.0, 148.5, 9.0, 148.5, 0.3, LINE);
    c.line(4.0, 210.0, 8.0, 210.0, 0.3, LINE);
}

struct Cols {
    desc_x: f32,
    desc_w: f32,
    menge_r: f32,
    einheit_x: f32,
    ust_r: Option<f32>,
    preis_r: f32,
    gesamt_r: f32,
}

fn columns(mixed: bool) -> Cols {
    if mixed {
        Cols { desc_x: 35.0, desc_w: 68.0, menge_r: 118.0, einheit_x: 120.0, ust_r: Some(143.0), preis_r: 166.0, gesamt_r: RIGHT }
    } else {
        Cols { desc_x: 35.0, desc_w: 78.0, menge_r: 128.0, einheit_x: 130.0, ust_r: None, preis_r: 166.0, gesamt_r: RIGHT }
    }
}

fn table_header(c: &mut Canvas, y: f32, cols: &Cols, accent: [f32; 3]) -> f32 {
    c.rect(LEFT, y, RIGHT - LEFT, 7.0, mix_white(accent, 0.12));
    let ty = y + 4.8;
    let s = 8.5;
    c.text(LEFT + 1.5, ty, s, true, BLACK, "Pos.");
    c.text(cols.desc_x, ty, s, true, BLACK, "Beschreibung");
    c.text_right(cols.menge_r, ty, s, true, BLACK, "Menge");
    c.text(cols.einheit_x, ty, s, true, BLACK, "Einheit");
    if let Some(u) = cols.ust_r {
        c.text_right(u, ty, s, true, BLACK, "USt.");
    }
    c.text_right(cols.preis_r, ty, s, true, BLACK, "Einzelpreis");
    c.text_right(cols.gesamt_r - 1.5, ty, s, true, BLACK, "Gesamt");
    y + 7.0
}

/// Angebot oder Rechnung als PDF.
pub fn dokument_pdf(data: &Data, doc: &Dokument, logo: Option<&[u8]>) -> Vec<u8> {
    let f = &data.firma;
    let kunde: Option<&Kunde> = data.kunde(doc.kunde_id);
    let accent = hex_color(&f.farbe);
    let sum = doc.summen();
    let mixed = !doc.kleinunternehmer && {
        let mut r: Vec<u8> = doc.positionen.iter().map(|p| p.ust).collect();
        r.sort_unstable();
        r.dedup();
        r.len() > 1
    };
    let cols = columns(mixed);
    let logo = logo.filter(|b| image::is_png(b) || image::is_jpeg(b));

    let mut pages: Vec<Canvas> = Vec::new();
    let mut c = Canvas::default();
    fold_marks(&mut c);

    // --- Kopf: Logo bzw. Firmenname -------------------------------------
    let mut logo_drawn = false;
    if let Some(bytes) = logo {
        if let Some((w, h)) = logo_box(bytes, f.logo_breite_mm.clamp(15.0, 90.0), 28.0) {
            c.image("Logo", RIGHT - w, 12.0, w, h);
            logo_drawn = true;
        }
    }
    if !logo_drawn && !f.name.trim().is_empty() {
        c.text_right(RIGHT, 22.0, 17.0, true, accent, f.name.trim());
        if !f.zusatz.trim().is_empty() {
            c.text_right(RIGHT, 28.0, 9.0, false, GRAY, f.zusatz.trim());
        }
    }

    // --- Anschriftfeld ----------------------------------------------------
    let abs = f.absenderzeile();
    if !abs.is_empty() {
        let line = wrap(&abs, 80.0, 7.0, false).into_iter().next().unwrap_or_default();
        c.text(LEFT, 49.0, 7.0, false, GRAY, &line);
        c.line(LEFT, 50.2, LEFT + (text_width(&line, 7.0, false) / MM).min(80.0), 50.2, 0.3, GRAY);
    }
    let mut y = 57.0;
    for l in doc.empfaenger.lines().take(7) {
        for part in wrap(l, 80.0, 10.0, false).into_iter().take(2) {
            c.text(LEFT, y, 10.0, false, BLACK, &part);
            y += 4.6;
        }
    }

    // --- Infoblock -------------------------------------------------------
    let mut info: Vec<(&str, String)> = Vec::new();
    let nr_label = match doc.art {
        DokArt::Angebot => "Angebotsnr.",
        DokArt::Rechnung => "Rechnungsnr.",
    };
    info.push((nr_label, doc.nummer.clone()));
    info.push(("Datum", date::german(&doc.datum)));
    if let Some(k) = kunde {
        if !k.nr.is_empty() {
            info.push(("Kundennr.", k.nr.clone()));
        }
        if !k.ust_id.trim().is_empty() {
            info.push(("Ihre USt-IdNr.", k.ust_id.trim().to_string()));
        }
    }
    if !doc.leistung.trim().is_empty() {
        info.push((if doc.art == DokArt::Rechnung { "Leistungsdatum" } else { "Ausführung" }, doc.leistung.trim().to_string()));
    }
    if !doc.faellig.is_empty() {
        match doc.art {
            DokArt::Angebot => info.push(("Gültig bis", date::german(&doc.faellig))),
            DokArt::Rechnung if doc.storno_von.is_none() => info.push(("Zahlbar bis", date::german(&doc.faellig))),
            _ => {}
        }
    }
    if let Some(orig) = data.dokument(doc.storno_von) {
        info.push(("Storno zu", orig.nummer.clone()));
    }
    let mut iy = 57.0;
    for (k, v) in &info {
        c.text(125.0, iy, 8.5, false, GRAY, k);
        c.text_right(RIGHT, iy, 8.5, false, BLACK, v);
        iy += 4.8;
    }

    // --- Titel, Betreff, Einleitung ----------------------------------------
    let mut y = 103.0f32.max(iy + 6.0);
    c.text(LEFT, y, 16.0, true, BLACK, &format!("{} {}", doc.titel(), doc.nummer));
    y += 7.0;
    if !doc.betreff.trim().is_empty() {
        for l in wrap(doc.betreff.trim(), RIGHT - LEFT, 10.5, true) {
            c.text(LEFT, y, 10.5, true, BLACK, &l);
            y += 5.0;
        }
    }
    y += 3.0;
    let einleitung = doc.fill(&doc.einleitung, f, kunde);
    for l in wrap(einleitung.trim_end(), RIGHT - LEFT, 10.0, false) {
        if y > BOTTOM {
            pages.push(std::mem::take(&mut c));
            y = 25.0;
        }
        c.text(LEFT, y, 10.0, false, BLACK, &l);
        y += 4.9;
    }
    y += 4.0;

    // --- Positionen --------------------------------------------------------
    let new_page = |pages: &mut Vec<Canvas>, c: &mut Canvas| -> f32 {
        pages.push(std::mem::take(c));
        25.0
    };
    if y + 20.0 > BOTTOM {
        y = new_page(&mut pages, &mut c);
    }
    y = table_header(&mut c, y, &cols, accent);
    let size = 9.5;
    let lh = 4.5;
    let mut pos_nr = 0;
    for p in &doc.positionen {
        if p.text.trim().is_empty() && p.preis == 0 {
            continue;
        }
        pos_nr += 1;
        let text = p.text.trim_end();
        let multi = text.contains('\n');
        let mut lines: Vec<(String, bool)> = Vec::new();
        for (i, raw) in text.split('\n').enumerate() {
            let bold = multi && i == 0;
            for l in wrap(raw, cols.desc_w, size, bold) {
                lines.push((l, bold));
            }
        }
        if lines.is_empty() {
            lines.push((String::new(), false));
        }
        let row_h = lines.len() as f32 * lh + 3.4;
        if y + row_h.min(40.0) > BOTTOM {
            y = new_page(&mut pages, &mut c);
            y = table_header(&mut c, y, &cols, accent);
        }
        let ty = y + 5.0;
        c.text(LEFT + 1.5, ty, size, false, GRAY, &pos_nr.to_string());
        c.text_right(cols.menge_r, ty, size, false, BLACK, &fmt_qty(p.menge));
        c.text(cols.einheit_x, ty, size, false, BLACK, &p.einheit);
        if let Some(u) = cols.ust_r {
            c.text_right(u, ty, size, false, BLACK, &format!("{} %", p.ust));
        }
        c.text_right(cols.preis_r, ty, size, false, BLACK, &fmt_num(p.preis));
        c.text_right(cols.gesamt_r - 1.5, ty, size, false, BLACK, &fmt_num(p.netto()));
        let mut ly = ty;
        for (l, bold) in &lines {
            if ly > BOTTOM + 2.0 {
                y = new_page(&mut pages, &mut c);
                y = table_header(&mut c, y, &cols, accent);
                ly = y + 5.0;
            }
            c.text(cols.desc_x, ly, size, *bold, BLACK, l);
            ly += lh;
        }
        y = ly - lh + 3.4;
        c.line(LEFT, y, RIGHT, y, 0.35, LINE);
    }

    // --- Summen -------------------------------------------------------------
    let rows = 2 + sum.steuern.len();
    if y + rows as f32 * 6.0 + 10.0 > BOTTOM {
        y = new_page(&mut pages, &mut c);
    }
    y += 6.0;
    let lx = 118.0;
    let show_net = !sum.steuern.is_empty();
    if show_net {
        c.text(lx, y, 9.5, false, BLACK, "Summe netto");
        c.text_right(RIGHT - 1.5, y, 9.5, false, BLACK, &fmt_eur(sum.netto));
        y += 5.2;
        for (rate, basis, tax) in &sum.steuern {
            let label = if sum.steuern.len() > 1 {
                format!("zzgl. {} % USt. auf {}", rate, fmt_num(*basis))
            } else {
                format!("zzgl. {} % USt.", rate)
            };
            c.text(lx, y, 9.5, false, BLACK, &label);
            c.text_right(RIGHT - 1.5, y, 9.5, false, BLACK, &fmt_eur(*tax));
            y += 5.2;
        }
    }
    y += 0.6;
    c.rect(lx - 2.0, y - 4.6, RIGHT - lx + 2.0, 7.4, mix_white(accent, 0.12));
    c.line(lx - 2.0, y - 4.6, RIGHT, y - 4.6, 0.8, accent);
    c.text(lx, y + 0.3, 10.5, true, BLACK, if doc.art == DokArt::Angebot { "Angebotssumme" } else { "Gesamtbetrag" });
    c.text_right(RIGHT - 1.5, y + 0.3, 10.5, true, BLACK, &fmt_eur(sum.brutto));
    y += 10.0;
    if doc.kleinunternehmer {
        c.text(LEFT, y, 8.5, false, GRAY, "Gemäß § 19 UStG wird keine Umsatzsteuer berechnet.");
        y += 6.0;
    }

    // --- Schlusstext, Gruß ---------------------------------------------------
    let schluss = doc.fill(&doc.schluss, f, kunde);
    let mut tail: Vec<String> = Vec::new();
    if !schluss.trim().is_empty() {
        tail.extend(wrap(schluss.trim_end(), RIGHT - LEFT, 10.0, false));
        tail.push(String::new());
    }
    tail.push("Mit freundlichen Grüßen".into());
    if !f.name.trim().is_empty() {
        tail.push(f.name.trim().to_string());
    }
    y += 2.0;
    for l in tail {
        if y > BOTTOM + 2.0 {
            y = new_page(&mut pages, &mut c);
        }
        c.text(LEFT, y, 10.0, false, BLACK, &l);
        y += 4.9;
    }
    pages.push(c);

    let total = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        footer(p, f, i + 1, total);
    }
    let title = format!("{} {}", doc.titel(), doc.nummer);
    assemble(pages, logo, &title, &f.name)
}

// ---------------------------------------------------------------------------
// Jahresübersicht Finanzen
// ---------------------------------------------------------------------------

pub fn auswertung_pdf(data: &Data, a: &Auswertung, heute: &str, logo: Option<&[u8]>) -> Vec<u8> {
    let f = &data.firma;
    let accent = hex_color(&f.farbe);
    let logo = logo.filter(|b| image::is_png(b) || image::is_jpeg(b));
    let mut pages: Vec<Canvas> = Vec::new();
    let mut c = Canvas::default();
    if let Some(bytes) = logo {
        if let Some((w, h)) = logo_box(bytes, f.logo_breite_mm.clamp(15.0, 60.0), 20.0) {
            c.image("Logo", RIGHT - w, 12.0, w, h);
        }
    }
    let mut y = 25.0;
    c.text(LEFT, y, 18.0, true, BLACK, &format!("Finanzübersicht {}", a.jahr));
    y += 7.0;
    c.text(LEFT, y, 9.5, false, GRAY, &format!("{} · erstellt am {}", f.name.trim(), date::german(heute)));
    y += 12.0;

    let row = |c: &mut Canvas, y: f32, cells: &[(&str, bool)], bold: bool| {
        let xs = [LEFT + 1.5, 95.0, 125.0, 155.0, RIGHT - 1.5];
        for (i, (t, right)) in cells.iter().enumerate() {
            if *right {
                c.text_right(xs[i + 1].min(RIGHT - 1.5), y, 9.0, bold, BLACK, t);
            } else {
                c.text(xs[i], y, 9.0, bold, BLACK, t);
            }
        }
    };

    // Kennzahlen
    c.rect(LEFT, y - 5.0, RIGHT - LEFT, 7.0, mix_white(accent, 0.12));
    row(&mut c, y, &[("Zusammenfassung", false), ("Netto", true), ("USt.", true), ("Brutto", true)], true);
    y += 7.0;
    let lines = [
        ("Betriebseinnahmen", a.ein),
        ("Betriebsausgaben", a.aus),
    ];
    for (l, s) in lines {
        row(&mut c, y, &[(l, false), (&fmt_eur(s.netto), true), (&fmt_eur(s.steuer), true), (&fmt_eur(s.brutto), true)], false);
        y += 5.5;
    }
    c.line(LEFT, y - 3.5, RIGHT, y - 3.5, 0.4, LINE);
    y += 1.5;
    row(&mut c, y, &[("Gewinn (Einnahmen − Ausgaben, netto)", false), ("", true), ("", true), (&fmt_eur(a.gewinn()), true)], true);
    y += 5.5;
    if !f.kleinunternehmer {
        row(&mut c, y, &[("Umsatzsteuer − Vorsteuer (Zahllast)", false), ("", true), ("", true), (&fmt_eur(a.zahllast()), true)], false);
        y += 5.5;
    }
    if a.neutral_ein != 0 || a.neutral_aus != 0 {
        row(&mut c, y, &[("Gewinnneutral (Privat, Finanzamt): Ein / Aus", false), ("", true), (&fmt_eur(a.neutral_ein), true), (&fmt_eur(a.neutral_aus), true)], false);
        y += 5.5;
    }
    y += 8.0;

    // Monate
    c.rect(LEFT, y - 5.0, RIGHT - LEFT, 7.0, mix_white(accent, 0.12));
    row(&mut c, y, &[("Monat", false), ("Einnahmen netto", true), ("Ausgaben netto", true), ("Ergebnis", true)], true);
    y += 7.0;
    for (i, m) in a.monate.iter().enumerate() {
        row(
            &mut c,
            y,
            &[(date::MONTHS[i], false), (&fmt_eur(m.ein.netto), true), (&fmt_eur(m.aus.netto), true), (&fmt_eur(m.ein.netto - m.aus.netto), true)],
            false,
        );
        y += 5.2;
    }
    y += 6.0;

    // Quartale (Umsatzsteuer-Voranmeldung)
    if !f.kleinunternehmer {
        c.rect(LEFT, y - 5.0, RIGHT - LEFT, 7.0, mix_white(accent, 0.12));
        row(&mut c, y, &[("Quartal", false), ("Umsatzsteuer", true), ("Vorsteuer", true), ("Zahllast", true)], true);
        y += 7.0;
        for q in 1..=4 {
            let (u, v) = a.quartal(q);
            row(&mut c, y, &[(&format!("Q{q}"), false), (&fmt_eur(u), true), (&fmt_eur(v), true), (&fmt_eur(u - v), true)], false);
            y += 5.2;
        }
        y += 6.0;
    }

    // Kategorien
    for (titel, list) in [("Einnahmen nach Kategorie", &a.kat_ein), ("Ausgaben nach Kategorie", &a.kat_aus)] {
        if list.is_empty() {
            continue;
        }
        if y + 20.0 > BOTTOM {
            pages.push(std::mem::take(&mut c));
            y = 25.0;
        }
        c.rect(LEFT, y - 5.0, RIGHT - LEFT, 7.0, mix_white(accent, 0.12));
        row(&mut c, y, &[(titel, false), ("Netto", true), ("USt.", true), ("Brutto", true)], true);
        y += 7.0;
        for (k, s, neutral) in list.iter() {
            if y > BOTTOM {
                pages.push(std::mem::take(&mut c));
                y = 25.0;
            }
            let name = if *neutral { format!("{k} (gewinnneutral)") } else { k.clone() };
            row(&mut c, y, &[(&name, false), (&fmt_eur(s.netto), true), (&fmt_eur(s.steuer), true), (&fmt_eur(s.brutto), true)], false);
            y += 5.2;
        }
        y += 6.0;
    }
    if y + 10.0 > BOTTOM {
        pages.push(std::mem::take(&mut c));
        y = 25.0;
    }
    for l in wrap(
        "Vereinfachte Übersicht auf Basis der erfassten Zahlungen (Zufluss-/Abflussprinzip). Sie ersetzt keine Steuerberatung; Abschreibungen, Fahrtenbuch u. Ä. sind nicht berücksichtigt.",
        RIGHT - LEFT,
        7.5,
        false,
    ) {
        c.text(LEFT, y, 7.5, false, GRAY, &l);
        y += 3.5;
    }
    pages.push(c);
    let total = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        footer(p, f, i + 1, total);
    }
    assemble(pages, logo, &format!("Finanzübersicht {}", a.jahr), &f.name)
}

/// Kurzer Betrag für Diagramme: „12,3 Tsd.“
pub fn kurz(c: Cents) -> String {
    let e = c as f64 / 100.0;
    if e.abs() >= 1_000_000.0 {
        format!("{} Mio.", format!("{:.1}", e / 1_000_000.0).replace('.', ","))
    } else if e.abs() >= 10_000.0 {
        format!("{:.0} Tsd.", e / 1000.0)
    } else if e.abs() >= 1000.0 {
        format!("{} Tsd.", format!("{:.1}", e / 1000.0).replace('.', ","))
    } else {
        format!("{:.0}", e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::*;
    use crate::engine::report;

    fn sample() -> (Data, Id) {
        let mut d = Data::new();
        d.firma.name = "Müller Haustechnik".into();
        d.firma.strasse = "Werkstraße 5".into();
        d.firma.plz_ort = "80331 München".into();
        d.firma.iban = "DE89 3704 0044 0532 0130 00".into();
        d.firma.steuernummer = "143/123/45678".into();
        let k = d.neuer_kunde("2026-11-12");
        let kd = d.kunde_mut(k).unwrap();
        kd.firma = "Bäckerei Schön".into();
        kd.strasse = "Am Markt 3".into();
        kd.plz = "80331".into();
        kd.ort = "München".into();
        let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-11-12");
        let doc = d.dokument_mut(r).unwrap();
        doc.betreff = "Wartung Heizungsanlage (Größe „XL“)".into();
        doc.positionen = (0..40)
            .map(|i| Position {
                text: format!("Arbeitsleistung Nr. {i}\nInkl. Anfahrt, Prüfung und Dokumentation – sehr ausführlich beschrieben, damit die Zeile umbricht"),
                menge: 1.5,
                einheit: "Std.".into(),
                preis: 6_500,
                ust: if i % 7 == 0 { 7 } else { 19 },
            })
            .collect();
        (d, r)
    }

    #[test]
    fn invoice_pdf_is_valid() {
        let (d, r) = sample();
        let doc = d.dokument(Some(r)).unwrap();
        let bytes = dokument_pdf(&d, doc, None);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF\n"));
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("/Count "));
        // mehrere Seiten bei 40 Positionen
        let count: usize = s.split("/Count ").nth(1).unwrap().split(' ').next().unwrap().parse().unwrap();
        assert!(count >= 3, "Seiten: {count}");
    }

    #[test]
    fn with_png_logo() {
        let (mut d, r) = sample();
        d.firma.kleinunternehmer = true;
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, 4, 2);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[200u8; 4 * 4 * 2]).unwrap();
        }
        let doc = d.dokument(Some(r)).unwrap().clone();
        let bytes = dokument_pdf(&d, &doc, Some(&png_bytes));
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("/Logo"));
        assert!(s.contains("/SMask"));
    }

    #[test]
    fn wrapping() {
        let l = wrap("Ein sehr langer Text der umbrechen muss weil er nicht passt", 30.0, 10.0, false);
        assert!(l.len() > 1);
        assert!(l.iter().all(|x| text_width(x, 10.0, false) <= 30.0 * MM + 0.01));
        assert_eq!(wrap("a\n\nb", 50.0, 10.0, false), vec!["a", "", "b"]);
        assert_eq!(pdf_string("ä(€)"), "(\\344\\(\\200\\))");
    }

    #[test]
    fn report_pdf() {
        let (mut d, _) = sample();
        let e = d.neue_buchung(BuchArt::Einnahme, "2026-03-01");
        d.buchung_mut(e).unwrap().brutto = 50_000;
        let a = report::auswertung(&d, 2026);
        let bytes = auswertung_pdf(&d, &a, "2026-11-12", None);
        assert!(bytes.starts_with(b"%PDF"));
    }
}
