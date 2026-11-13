//! PDF-Erzeugung ohne externe Bibliotheken: Angebote/Rechnungen im DIN-5008-Briefbogen
//! (Logo, Anschriftfeld, Infoblock, Positionstabelle, Steuerausweis, Fußzeile mit Bankdaten)
//! sowie die Jahresübersicht der Finanzen.
//!
//! Schriften: die PDF-Standardschriften Helvetica/Helvetica-Bold in WinAnsi-Kodierung
//! (Umlaute, ß, € werden korrekt dargestellt) – dadurch sind die Dateien klein und überall lesbar.

use super::date;
use super::image;
use super::erechnung;
use super::fonts;
use super::girocode;
use super::pdfa;
use super::ttf;
use super::anlagen::{Pruefung, Wartung};
use super::model::{briefanrede, positionsnummern, zwischensummen, Data, DokArt, Dokument, Firma, Kunde, PosArt, Unterschrift, UNTERSCHRIFT_VERHAELTNIS};
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
        buf.extend_from_slice(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n");
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
        // Dokument-ID (PDF/A verlangt /ID im Trailer): Prüfsumme über den Inhalt
        let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
        let mut h2: u64 = 0x8422_2325_cbf2_9ce4;
        for &b in &self.buf {
            h1 = (h1 ^ b as u64).wrapping_mul(0x0100_0000_01b3);
            h2 = (h2 ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(5);
        }
        let id = format!("{h1:016X}{h2:016X}");
        let xref = self.buf.len();
        let mut s = format!("xref\n0 {}\n0000000000 65535 f \n", self.offsets.len() + 1);
        for o in &self.offsets {
            let _ = writeln!(s, "{:010} 00000 n ", o);
        }
        let _ = write!(
            s,
            "trailer\n<< /Size {} /Root {} 0 R /Info {} 0 R /ID [<{id}> <{id}>] >>\nstartxref\n{}\n%%EOF\n",
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
/// Eingebettete XML-Rechnung (ZUGFeRD/Factur-X).
pub struct Anhang<'a> {
    pub name: &'a str,
    pub daten: &'a [u8],
    /// Factur-X-Profil, z. B. „EN 16931“
    pub stufe: &'a str,
}

/// TrueType-Schrift einbetten (PDF/A verlangt eingebettete Schriftprogramme); liefert die Objekt-ID.
fn embed_font(pdf: &mut Pdf, data: &[u8], name: &str, bold: bool) -> usize {
    let font = ttf::Font::parse(data).expect("eingebettete Schrift");
    let m = font.metrics();
    let widths = font.pdf_widths().iter().map(|w| w.to_string()).collect::<Vec<_>>().join(" ");
    let file = pdf.reserve();
    pdf.stream(file, &format!("/Length1 {}", data.len()), data);
    let desc = pdf.reserve();
    pdf.obj(
        desc,
        &format!(
            "<< /Type /FontDescriptor /FontName /{name} /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {} /FontFile2 {file} 0 R >>",
            m.bbox[0],
            m.bbox[1],
            m.bbox[2],
            m.bbox[3],
            n(m.italic_angle),
            m.ascent,
            m.descent,
            m.cap_height,
            if bold { 140 } else { 80 }
        ),
    );
    let id = pdf.reserve();
    pdf.obj(
        id,
        &format!("<< /Type /Font /Subtype /TrueType /BaseFont /{name} /FirstChar 32 /LastChar 255 /Widths [{widths}] /FontDescriptor {desc} 0 R /Encoding /WinAnsiEncoding >>"),
    );
    id
}

/// Seiten zu einem PDF/A-3b zusammensetzen (eingebettete Schriften, sRGB-Farbprofil, XMP);
/// optional mit eingebetteter XML-Rechnung (ZUGFeRD/Factur-X).
fn assemble(pages: Vec<Canvas>, logo: Option<&[u8]>, title: &str, author: &str, datum: &str, anhang: Option<Anhang>) -> Vec<u8> {
    const PRODUCER: &str = "Zento";
    let mut pdf = Pdf::new();
    let catalog = pdf.reserve();
    let pages_id = pdf.reserve();
    let info = pdf.reserve();
    let f1 = embed_font(&mut pdf, fonts::REGULAR, "WWPAAA+LiberationSans", false);
    let f2 = embed_font(&mut pdf, fonts::BOLD, "WWPAAB+LiberationSans-Bold", true);
    // CMYK-JPEGs passen nicht zum sRGB-Profil von PDF/A und werden ausgelassen
    let logo = logo.filter(|b| image::jpeg_info(b).map(|i| i.2 != 4).unwrap_or(true));
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

    // Datum für Info und XMP (müssen übereinstimmen)
    let d = if date::parse(datum).is_some() { datum.replace('-', "") } else { "20260101".into() };
    let pdf_datum = format!("D:{d}000000+00'00'");
    let xmp_datum = format!("{}-{}-{}T00:00:00+00:00", &d[0..4], &d[4..6], &d[6..8]);

    // Farbprofil (OutputIntent)
    let icc = pdf.reserve();
    pdf.stream(icc, "/N 3", &pdfa::srgb_icc());
    let oi = pdf.reserve();
    pdf.obj(
        oi,
        &format!("<< /Type /OutputIntent /S /GTS_PDFA1 /OutputConditionIdentifier (sRGB IEC61966-2.1) /Info (sRGB IEC61966-2.1) /RegistryName (http://www.color.org) /DestOutputProfile {icc} 0 R >>"),
    );
    // eingebettete Rechnung
    let mut extra = String::new();
    let mut fx = None;
    if let Some(a) = &anhang {
        let ef = pdf.reserve();
        pdf.stream(
            ef,
            &format!("/Type /EmbeddedFile /Subtype /text#2Fxml /Params << /ModDate ({pdf_datum}) /Size {} >>", a.daten.len()),
            a.daten,
        );
        let fs = pdf.reserve();
        pdf.obj(
            fs,
            &format!(
                "<< /Type /Filespec /F {name} /UF {name} /Desc (Factur-X/ZUGFeRD-Rechnung) /AFRelationship /Alternative /EF << /F {ef} 0 R /UF {ef} 0 R >> >>",
                name = pdf_string(a.name)
            ),
        );
        extra = format!(" /Names << /EmbeddedFiles << /Names [{} {fs} 0 R] >> >> /AF [{fs} 0 R]", pdf_string(a.name));
        fx = Some((a.name, a.stufe));
    }
    let meta = pdf.reserve();
    let xmp = pdfa::xmp(title, author.trim(), PRODUCER, &xmp_datum, fx);
    pdf.stream_raw(meta, "/Type /Metadata /Subtype /XML", xmp.as_bytes());
    pdf.obj(
        catalog,
        &format!("<< /Type /Catalog /Pages {pages_id} 0 R /Metadata {meta} 0 R /OutputIntents [{oi} 0 R] /Lang (de-DE) /ViewerPreferences << /DisplayDocTitle true >>{extra} >>"),
    );
    let autor = if author.trim().is_empty() { String::new() } else { format!(" /Author {}", pdf_utf16(author.trim())) };
    pdf.obj(
        info,
        &format!("<< /Title {}{autor} /Creator ({PRODUCER}) /Producer ({PRODUCER}) /CreationDate ({pdf_datum}) /ModDate ({pdf_datum}) >>", pdf_utf16(title)),
    );
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
    /// Einzelpreis/Gesamt (rechte Kante); `None` beim Lieferschein
    preise: Option<(f32, f32)>,
}

fn columns(mixed: bool, preise: bool) -> Cols {
    if !preise {
        return Cols { desc_x: 35.0, desc_w: 118.0, menge_r: 168.0, einheit_x: 171.0, ust_r: None, preise: None };
    }
    if mixed {
        Cols { desc_x: 35.0, desc_w: 68.0, menge_r: 118.0, einheit_x: 120.0, ust_r: Some(143.0), preise: Some((166.0, RIGHT)) }
    } else {
        Cols { desc_x: 35.0, desc_w: 78.0, menge_r: 128.0, einheit_x: 130.0, ust_r: None, preise: Some((166.0, RIGHT)) }
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
    if let Some((preis_r, gesamt_r)) = cols.preise {
        c.text_right(preis_r, ty, s, true, BLACK, "Einzelpreis");
        c.text_right(gesamt_r - 1.5, ty, s, true, BLACK, "Gesamt");
    }
    y + 7.0
}

/// Briefkopf: Logo bzw. Firmenname, Absenderzeile, Anschriftfeld und Infoblock.
/// Liefert die y-Position (mm), ab der der Brieftext beginnen kann.
fn briefkopf(c: &mut Canvas, f: &Firma, logo: Option<&[u8]>, empfaenger: &str, info: &[(String, String)]) -> f32 {
    let accent = hex_color(&f.farbe);
    fold_marks(c);
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
    let abs = f.absenderzeile();
    if !abs.is_empty() {
        let line = wrap(&abs, 80.0, 7.0, false).into_iter().next().unwrap_or_default();
        c.text(LEFT, 49.0, 7.0, false, GRAY, &line);
        c.line(LEFT, 50.2, LEFT + (text_width(&line, 7.0, false) / MM).min(80.0), 50.2, 0.3, GRAY);
    }
    let mut y = 57.0;
    for l in empfaenger.lines().take(7) {
        for part in wrap(l, 80.0, 10.0, false).into_iter().take(2) {
            c.text(LEFT, y, 10.0, false, BLACK, &part);
            y += 4.6;
        }
    }
    let mut iy = 57.0;
    for (k, v) in info {
        c.text(125.0, iy, 8.5, false, GRAY, k);
        c.text_right(RIGHT, iy, 8.5, false, BLACK, v);
        iy += 4.8;
    }
    103.0f32.max(iy + 6.0)
}

/// Neue Seite beginnen; liefert die obere Textkante.
fn new_page(pages: &mut Vec<Canvas>, c: &mut Canvas) -> f32 {
    pages.push(std::mem::take(c));
    25.0
}

/// Fließtext mit Seitenumbruch.
fn absatz(pages: &mut Vec<Canvas>, c: &mut Canvas, mut y: f32, text: &str) -> f32 {
    for l in wrap(text.trim_end(), RIGHT - LEFT, 10.0, false) {
        if y > BOTTOM + 2.0 {
            y = new_page(pages, c);
        }
        c.text(LEFT, y, 10.0, false, BLACK, &l);
        y += 4.9;
    }
    y
}

/// Grußformel.
fn gruss(pages: &mut Vec<Canvas>, c: &mut Canvas, mut y: f32, f: &Firma) -> f32 {
    let mut tail = vec!["Mit freundlichen Grüßen".to_string()];
    if !f.name.trim().is_empty() {
        tail.push(f.name.trim().to_string());
    }
    if y + 10.0 > BOTTOM + 2.0 {
        y = new_page(pages, c);
    }
    for l in tail {
        c.text(LEFT, y, 10.0, false, BLACK, &l);
        y += 4.9;
    }
    y
}

/// Kantenlänge des GiroCodes (mm).
const QR_SIZE: f32 = 26.0;

/// GiroCode-Inhalt für einen Betrag, falls eingeschaltet und die Bankdaten gültig sind.
fn giro_payload(f: &Firma, betrag: Cents, zweck: &str) -> Option<String> {
    if !f.girocode {
        return None;
    }
    girocode::epc_payload(&f.name, &f.iban, &f.bic, betrag, zweck)
}

/// GiroCode mit Beschriftung zeichnen (obere linke Ecke x/y); liefert die Unterkante.
fn girocode_block(c: &mut Canvas, x: f32, y: f32, payload: &str) -> Option<f32> {
    let m = girocode::matrix(payload)?;
    let n = m.len() as f32;
    let mod_mm = QR_SIZE / n;
    let mut path = String::new();
    for (row, cells) in m.iter().enumerate() {
        let mut col = 0;
        while col < cells.len() {
            if !cells[col] {
                col += 1;
                continue;
            }
            let start = col;
            while col < cells.len() && cells[col] {
                col += 1;
            }
            let px = (x + start as f32 * mod_mm) * MM;
            let py = (PAGE_H - y - (row + 1) as f32 * mod_mm) * MM;
            // minimal überlappen, damit keine Haarlinien zwischen den Modulen entstehen
            let _ = write!(path, "{} {} {} {} re ", n4(px), n4(py), n4((col - start) as f32 * mod_mm * MM + 0.05), n4(mod_mm * MM + 0.05));
        }
    }
    let _ = writeln!(c.c, "0 0 0 rg {}f", path);
    c.text(x, y + QR_SIZE + 3.6, 7.0, true, BLACK, "Bezahlen per Banking-App");
    c.text(x, y + QR_SIZE + 6.6, 6.8, false, GRAY, "GiroCode scannen – Daten sind ausgefüllt");
    Some(y + QR_SIZE + 7.5)
}

fn n4(v: f32) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

/// Summenzeile im rechten Bereich.
fn summe_zeile(c: &mut Canvas, lx: f32, y: f32, label: &str, wert: &str) {
    c.text(lx, y, 9.5, false, BLACK, label);
    c.text_right(RIGHT - 1.5, y, 9.5, false, BLACK, wert);
}

/// Hervorgehobene Endsumme.
fn summe_total(c: &mut Canvas, lx: f32, y: f32, label: &str, wert: &str, accent: [f32; 3]) {
    c.rect(lx - 2.0, y - 4.6, RIGHT - lx + 2.0, 7.4, mix_white(accent, 0.12));
    c.line(lx - 2.0, y - 4.6, RIGHT, y - 4.6, 0.8, accent);
    c.text(lx, y + 0.3, 10.5, true, BLACK, label);
    c.text_right(RIGHT - 1.5, y + 0.3, 10.5, true, BLACK, wert);
}

/// Seiten eines Angebots / einer Rechnung (mit Fußzeilen).
fn dokument_seiten(data: &Data, doc: &Dokument, logo: Option<&[u8]>) -> Vec<Canvas> {
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
    let preise = doc.art.mit_preisen();
    let cols = columns(mixed && preise, preise);

    let mut pages: Vec<Canvas> = Vec::new();
    let mut c = Canvas::default();

    // --- Kopf -------------------------------------------------------------
    let mut info: Vec<(String, String)> = Vec::new();
    let nr_label = match doc.art {
        DokArt::Angebot => "Angebotsnr.",
        DokArt::Rechnung => "Rechnungsnr.",
        DokArt::Auftragsbestaetigung => "Bestätigungsnr.",
        DokArt::Lieferschein => "Lieferschein-Nr.",
    };
    info.push((nr_label.into(), doc.nummer.clone()));
    info.push(("Datum".into(), date::german(&doc.datum)));
    if let Some(k) = kunde {
        if !k.nr.is_empty() {
            info.push(("Kundennr.".into(), k.nr.clone()));
        }
        if !k.ust_id.trim().is_empty() {
            info.push(("Ihre USt-IdNr.".into(), k.ust_id.trim().to_string()));
        }
    }
    if !doc.leistung.trim().is_empty() {
        let l = if matches!(doc.art, DokArt::Rechnung | DokArt::Lieferschein) { "Leistungsdatum" } else { "Ausführung" };
        info.push((l.into(), doc.leistung.trim().to_string()));
    }
    if !doc.faellig.is_empty() {
        match doc.art {
            DokArt::Angebot => info.push(("Gültig bis".into(), date::german(&doc.faellig))),
            DokArt::Rechnung if doc.storno_von.is_none() => info.push(("Zahlbar bis".into(), date::german(&doc.faellig))),
            _ => {}
        }
    }
    if let Some(orig) = data.dokument(doc.storno_von) {
        info.push(("Storno zu".into(), orig.nummer.clone()));
    }
    if doc.art != DokArt::Angebot {
        if let Some(an) = data.dokument(doc.aus_angebot) {
            info.push(("Zu Angebot".into(), an.nummer.clone()));
        }
    }
    let mut y = briefkopf(&mut c, f, logo, &doc.empfaenger, &info);

    // --- Titel, Betreff, Einleitung ----------------------------------------
    c.text(LEFT, y, 16.0, true, BLACK, &format!("{} {}", doc.titel(), doc.nummer));
    y += 7.0;
    if !doc.betreff.trim().is_empty() {
        for l in wrap(doc.betreff.trim(), RIGHT - LEFT, 10.5, true) {
            c.text(LEFT, y, 10.5, true, BLACK, &l);
            y += 5.0;
        }
    }
    y += 3.0;
    y = absatz(&mut pages, &mut c, y, &doc.fill(&doc.einleitung, f, kunde));
    y += 4.0;

    // --- Positionen --------------------------------------------------------
    if y + 20.0 > BOTTOM {
        y = new_page(&mut pages, &mut c);
    }
    y = table_header(&mut c, y, &cols, accent);
    let size = 9.5;
    let lh = 4.5;
    let nummern = positionsnummern(&doc.positionen);
    let zws = zwischensummen(&doc.positionen);
    // Zwischensumme einer Titelgruppe ausgeben, bevor der nächste Titel beginnt
    let zwischensumme = |c: &mut Canvas, y: f32, titel_idx: usize| -> f32 {
        let Some((_, summe)) = zws.iter().find(|z| z.0 == titel_idx) else { return y };
        let Some((_, gesamt_r)) = cols.preise else { return y };
        let titel = doc.positionen[titel_idx].text.lines().next().unwrap_or("").trim();
        let label = format!("Zwischensumme {}", wrap(titel, 70.0, 9.0, true).into_iter().next().unwrap_or_default());
        c.text_right(gesamt_r - 30.0, y + 4.6, 9.0, true, BLACK, &label);
        c.text_right(gesamt_r - 1.5, y + 4.6, 9.0, true, BLACK, &fmt_num(*summe));
        c.line(LEFT, y + 6.8, RIGHT, y + 6.8, 0.35, LINE);
        y + 6.8
    };
    let mut offener_titel: Option<usize> = None;
    for (idx, p) in doc.positionen.iter().enumerate() {
        if p.text.trim().is_empty() && p.preis == 0 {
            continue;
        }
        if p.art == PosArt::Titel {
            if let Some(t) = offener_titel.take() {
                y = zwischensumme(&mut c, y, t);
            }
            offener_titel = Some(idx);
        }
        let text = p.text.trim_end();
        let multi = text.contains('\n');
        let mut lines: Vec<(String, bool, bool)> = Vec::new(); // (Text, fett, grau)
        let breite = if p.art == PosArt::Titel || p.art == PosArt::Text { RIGHT - cols.desc_x - 2.0 } else { cols.desc_w };
        for (i, raw) in text.split('\n').enumerate() {
            let bold = p.art == PosArt::Titel || (multi && i == 0 && p.art != PosArt::Text);
            let raw = if p.art == PosArt::Optional && i == 0 { format!("Optional: {raw}") } else { raw.to_string() };
            for l in wrap(&raw, breite, size, bold) {
                lines.push((l, bold, p.art == PosArt::Text));
            }
        }
        if p.mit_preis() && p.rabatt > 0.0 {
            let r = format!("{}", (p.rabatt * 100.0).round() / 100.0).replace('.', ",");
            lines.push((format!("abzgl. {r} % Rabatt ({})", fmt_num(p.rabatt_betrag())), false, true));
        }
        if p.art == PosArt::Optional {
            lines.push(("Optionale Position – nicht in der Summe enthalten".into(), false, true));
        }
        if lines.is_empty() {
            lines.push((String::new(), false, false));
        }
        let row_h = lines.len() as f32 * lh + 3.4;
        if y + row_h.min(40.0) > BOTTOM {
            y = new_page(&mut pages, &mut c);
            y = table_header(&mut c, y, &cols, accent);
        }
        let ty = y + 5.0;
        if p.art == PosArt::Titel {
            c.rect(LEFT, y + 0.8, RIGHT - LEFT, row_h - 1.6, mix_white(accent, 0.05));
        }
        c.text(LEFT + 1.5, ty, size, p.art == PosArt::Titel, GRAY, &nummern[idx]);
        if p.mit_preis() {
            c.text_right(cols.menge_r, ty, size, false, BLACK, &fmt_qty(p.menge));
            c.text(cols.einheit_x, ty, size, false, BLACK, &p.einheit);
            if let Some(u) = cols.ust_r {
                c.text_right(u, ty, size, false, BLACK, &format!("{} %", p.ust));
            }
            if let Some((preis_r, gesamt_r)) = cols.preise {
                c.text_right(preis_r, ty, size, false, BLACK, &fmt_num(p.preis));
                let g = if p.art == PosArt::Optional { format!("({})", fmt_num(p.netto())) } else { fmt_num(p.netto()) };
                c.text_right(gesamt_r - 1.5, ty, size, false, if p.art == PosArt::Optional { GRAY } else { BLACK }, &g);
            }
        }
        let mut ly = ty;
        for (l, bold, grau) in &lines {
            if ly > BOTTOM + 2.0 {
                y = new_page(&mut pages, &mut c);
                y = table_header(&mut c, y, &cols, accent);
                ly = y + 5.0;
            }
            c.text(cols.desc_x, ly, if *grau { size - 0.8 } else { size }, *bold, if *grau { GRAY } else { BLACK }, l);
            ly += lh;
        }
        y = ly - lh + 3.4;
        c.line(LEFT, y, RIGHT, y, 0.35, LINE);
    }
    if let Some(t) = offener_titel {
        y = zwischensumme(&mut c, y, t);
    }

    // --- Summen (+ GiroCode links daneben) -----------------------------------
    if preise {
        let qr = if doc.art == DokArt::Rechnung && doc.storno_von.is_none() {
            giro_payload(f, sum.brutto, &format!("Rechnung {}", doc.nummer))
        } else {
            None
        };
        let rows = 2 + sum.steuern.len() + if sum.rabatt > 0 { 2 } else { 0 };
        let block_h = (rows as f32 * 6.0 + 10.0).max(if qr.is_some() { QR_SIZE + 12.0 } else { 0.0 });
        if y + block_h > BOTTOM {
            y = new_page(&mut pages, &mut c);
        }
        y += 6.0;
        let qr_bottom = qr.as_deref().and_then(|p| girocode_block(&mut c, LEFT, y - 3.5, p));
        let lx = 118.0;
        if sum.rabatt > 0 {
            summe_zeile(&mut c, lx, y, "Zwischensumme", &fmt_eur(sum.positionen));
            y += 5.2;
            let label = if doc.rabatt_betrag > 0 {
                "abzgl. Rabatt".to_string()
            } else {
                format!("abzgl. {} % Rabatt", format!("{}", (doc.rabatt_prozent * 100.0).round() / 100.0).replace('.', ","))
            };
            summe_zeile(&mut c, lx, y, &label, &fmt_eur(-sum.rabatt));
            y += 5.2;
        }
        if !sum.steuern.is_empty() {
            summe_zeile(&mut c, lx, y, "Summe netto", &fmt_eur(sum.netto));
            y += 5.2;
            for (rate, basis, tax) in &sum.steuern {
                let label = if sum.steuern.len() > 1 {
                    format!("zzgl. {} % USt. auf {}", rate, fmt_num(*basis))
                } else {
                    format!("zzgl. {} % USt.", rate)
                };
                summe_zeile(&mut c, lx, y, &label, &fmt_eur(*tax));
                y += 5.2;
            }
        }
        y += 0.6;
        let total_label = match doc.art {
            DokArt::Angebot => "Angebotssumme",
            DokArt::Auftragsbestaetigung => "Auftragssumme",
            _ => "Gesamtbetrag",
        };
        summe_total(&mut c, lx, y, total_label, &fmt_eur(sum.brutto), accent);
        y += 10.0;
        if doc.kleinunternehmer {
            c.text(lx - 2.0, y, 8.0, false, GRAY, "Gemäß § 19 UStG wird keine Umsatzsteuer berechnet.");
            y += 6.0;
        }
        let skonto = doc.skonto_betrag();
        if skonto > 0 && doc.storno_von.is_none() {
            let pct = format!("{}", (doc.skonto_prozent * 100.0).round() / 100.0).replace('.', ",");
            let t = format!(
                "Bei Zahlung bis {}: {} % Skonto ({}), Zahlbetrag {}",
                date::german(&doc.skonto_bis()),
                pct,
                fmt_eur(skonto),
                fmt_eur(sum.brutto - skonto)
            );
            for l in wrap(&t, RIGHT - lx + 2.0, 8.0, false) {
                c.text(lx - 2.0, y, 8.0, false, GRAY, &l);
                y += 3.8;
            }
            y += 2.0;
        }
        if let Some(b) = qr_bottom {
            y = y.max(b + 5.0);
        }
    }

    if !preise {
        y += 8.0;
    }

    // --- Schlusstext, Gruß ---------------------------------------------------
    y += 2.0;
    let schluss = doc.fill(&doc.schluss, f, kunde);
    if !schluss.trim().is_empty() {
        y = absatz(&mut pages, &mut c, y, &schluss);
        y += 4.9;
    }
    if doc.art == DokArt::Lieferschein || doc.art == DokArt::Auftragsbestaetigung || doc.unterschrift.is_some() {
        y = unterschrift_block(&mut pages, &mut c, y, doc);
    }
    gruss(&mut pages, &mut c, y, f);
    pages.push(c);

    let total = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        footer(p, f, i + 1, total);
    }
    pages
}

/// Breite des Unterschriftenfelds im PDF (mm).
const SIG_W: f32 = 70.0;

/// Unterschriftsfeld: vorhandene Unterschrift als Vektor-Tinte, sonst leere Linien zum Unterschreiben.
fn unterschrift_block(pages: &mut Vec<Canvas>, c: &mut Canvas, y: f32, doc: &Dokument) -> f32 {
    let titel = match doc.art {
        DokArt::Angebot => "Angebot angenommen – Auftrag erteilt",
        DokArt::Auftragsbestaetigung => "Auftrag bestätigt",
        DokArt::Lieferschein => "Leistung erhalten und abgenommen",
        DokArt::Rechnung => "",
    };
    unterschrift_feld(pages, c, y, titel, doc.unterschrift.as_ref(), "Unterschrift Auftraggeber (Name in Druckbuchstaben)")
}

/// Unterschrift (Vektor-Tinte) oder leere Linien zum Unterschreiben.
fn unterschrift_feld(pages: &mut Vec<Canvas>, c: &mut Canvas, mut y: f32, titel: &str, unterschrift: Option<&Unterschrift>, leer_label: &str) -> f32 {
    let sig_h = SIG_W / UNTERSCHRIFT_VERHAELTNIS;
    if y + sig_h + 16.0 > BOTTOM {
        y = new_page(pages, c);
    }
    c.text(LEFT, y, 9.0, true, BLACK, titel);
    let top = y + 1.5;
    let base = top + sig_h;
    match unterschrift {
        Some(u) => {
            let _ = writeln!(c.c, "q 1 J 1 j {} RG", rgb([0.08, 0.12, 0.35]));
            for strich in &u.striche {
                if strich.len() == 1 {
                    let p = strich[0];
                    let (x, yy) = (LEFT + p[0] * SIG_W, top + p[1] * SIG_W);
                    let _ = writeln!(c.c, "{} w {} {} m {} {} l S", n(0.9 * MM), n(x * MM), n((PAGE_H - yy) * MM), n((x + 0.05) * MM), n((PAGE_H - yy) * MM));
                }
                for w in strich.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    let druck = ((a[2] + b[2]) * 0.5).clamp(0.05, 1.0);
                    let breite = (0.25 + 0.6 * druck) * MM;
                    let _ = writeln!(
                        c.c,
                        "{} w {} {} m {} {} l S",
                        n(breite),
                        n((LEFT + a[0] * SIG_W) * MM),
                        n((PAGE_H - (top + a[1] * SIG_W)) * MM),
                        n((LEFT + b[0] * SIG_W) * MM),
                        n((PAGE_H - (top + b[1] * SIG_W)) * MM)
                    );
                }
            }
            c.c.push_str("Q\n");
            c.line(LEFT, base + 1.0, LEFT + SIG_W, base + 1.0, 0.4, GRAY);
            let unter = [u.ort.trim().to_string(), date::german(&u.datum), u.name.trim().to_string()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            c.text(LEFT, base + 4.8, 8.0, false, GRAY, &unter);
        }
        None => {
            let lb = top + sig_h * 0.75;
            c.line(LEFT, lb, LEFT + 50.0, lb, 0.4, GRAY);
            c.line(LEFT + 60.0, lb, RIGHT, lb, 0.4, GRAY);
            c.text(LEFT, lb + 3.8, 8.0, false, GRAY, "Ort, Datum");
            c.text(LEFT + 60.0, lb + 3.8, 8.0, false, GRAY, leer_label);
            return lb + 12.0;
        }
    }
    base + 13.0
}

/// Wartungsprotokoll als PDF: Anlage, Prüfpunkte, Messwerte, Arbeiten, Mängel, nächste Wartung, Unterschrift.
pub fn wartung_pdf(data: &Data, w: &Wartung, logo: Option<&[u8]>) -> Vec<u8> {
    let f = &data.firma;
    let accent = hex_color(&f.farbe);
    let logo = logo.filter(|b| image::is_png(b) || image::is_jpeg(b));
    let kunde = data.kunde(w.kunde_id);
    let anlage = data.anlage(w.anlage_id);
    let mut pages: Vec<Canvas> = Vec::new();
    let mut c = Canvas::default();
    let mut info = vec![("Protokoll-Nr.".to_string(), w.nummer.clone()), ("Datum".to_string(), date::german(&w.datum))];
    if let Some(k) = kunde {
        if !k.nr.is_empty() {
            info.push(("Kundennr.".into(), k.nr.clone()));
        }
    }
    if !w.monteur.trim().is_empty() {
        info.push(("Monteur".into(), w.monteur.trim().to_string()));
    }
    let empf = kunde.map(|k| k.anschrift()).unwrap_or_default();
    let mut y = briefkopf(&mut c, f, logo, &empf, &info);
    c.text(LEFT, y, 15.0, true, BLACK, "Wartungsprotokoll");
    y += 9.0;

    // Anlage
    let abschnitt = |c: &mut Canvas, y: f32, titel: &str| {
        c.rect(LEFT, y - 4.6, RIGHT - LEFT, 6.6, mix_white(accent, 0.12));
        c.text(LEFT + 1.5, y, 9.5, true, BLACK, titel);
    };
    abschnitt(&mut c, y, "Anlage");
    y += 6.5;
    if let Some(a) = anlage {
        let felder = [
            ("Art", a.art.label().to_string()),
            ("Hersteller / Modell", [a.hersteller.trim(), a.modell.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ")),
            ("Seriennummer", a.seriennummer.trim().to_string()),
            ("Baujahr", a.baujahr.trim().to_string()),
            ("Leistung", a.leistung.trim().to_string()),
            ("Standort", a.standort.trim().to_string()),
        ];
        let sichtbar: Vec<_> = felder.iter().filter(|(_, v)| !v.is_empty()).collect();
        for (i, (k, v)) in sichtbar.iter().enumerate() {
            let x = if i % 2 == 0 { LEFT + 1.5 } else { 108.0 };
            c.text(x, y, 8.5, false, GRAY, k);
            c.text(x + 30.0, y, 9.0, false, BLACK, &wrap(v, 52.0, 9.0, false).into_iter().next().unwrap_or_default());
            if i % 2 == 1 || i + 1 == sichtbar.len() {
                y += 5.2;
            }
        }
    } else {
        c.text(LEFT + 1.5, y, 9.0, false, GRAY, "Anlage nicht mehr vorhanden");
        y += 5.2;
    }
    y += 4.0;

    // Prüfpunkte
    abschnitt(&mut c, y, "Durchgeführte Prüfungen");
    c.text_right(RIGHT - 1.5, y, 8.5, true, BLACK, "Ergebnis");
    y += 6.5;
    for p in &w.pruefpunkte {
        let zeilen = wrap(&p.text, 120.0, 9.0, false);
        let notiz = if p.notiz.trim().is_empty() { Vec::new() } else { wrap(p.notiz.trim(), 116.0, 8.5, false) };
        let hoehe = 4.8 * zeilen.len() as f32 + 4.4 * notiz.len() as f32 + 0.8;
        if y + hoehe > BOTTOM {
            y = new_page(&mut pages, &mut c);
        }
        let farbe = match p.status {
            Pruefung::Ok => [0.06, 0.55, 0.33],
            Pruefung::Mangel => [0.80, 0.18, 0.22],
            _ => GRAY,
        };
        for (i, z) in zeilen.iter().enumerate() {
            c.text(LEFT + 1.5, y, 9.0, false, BLACK, z);
            if i == 0 {
                c.text_right(RIGHT - 1.5, y, 9.0, true, farbe, p.status.label());
            }
            y += 4.8;
        }
        for z in notiz {
            c.text(LEFT + 5.0, y, 8.5, false, GRAY, &z);
            y += 4.4;
        }
        c.line(LEFT, y - 3.3, RIGHT, y - 3.3, 0.2, LINE);
        y += 0.8;
    }
    y += 3.0;

    // Messwerte
    let mw: Vec<_> = w.messwerte.iter().filter(|m| !m.wert.trim().is_empty()).collect();
    if !mw.is_empty() {
        if y + 14.0 > BOTTOM {
            y = new_page(&mut pages, &mut c);
        }
        abschnitt(&mut c, y, "Messwerte");
        y += 6.5;
        for (i, m) in mw.iter().enumerate() {
            if y > BOTTOM {
                y = new_page(&mut pages, &mut c);
            }
            let x = if i % 2 == 0 { LEFT + 1.5 } else { 108.0 };
            c.text(x, y, 8.5, false, GRAY, &m.name);
            c.text_right(x + 79.0, y, 9.0, true, BLACK, &format!("{} {}", m.wert.trim(), m.einheit));
            if i % 2 == 1 || i + 1 == mw.len() {
                y += 5.2;
            }
        }
        y += 4.0;
    }

    // Arbeiten, Mängel
    for (titel, text) in [("Durchgeführte Arbeiten / Bemerkungen", &w.arbeiten), ("Festgestellte Mängel / Empfehlungen", &w.maengel)] {
        if text.trim().is_empty() {
            continue;
        }
        if y + 14.0 > BOTTOM {
            y = new_page(&mut pages, &mut c);
        }
        abschnitt(&mut c, y, titel);
        y += 7.0;
        y = absatz(&mut pages, &mut c, y, text.trim());
        y += 4.0;
    }
    if let Some(n) = anlage.and_then(|a| a.naechste_wartung()) {
        if y + 8.0 > BOTTOM {
            y = new_page(&mut pages, &mut c);
        }
        let (jj, mm, _) = date::parse(&n).unwrap_or((0, 0, 0));
        c.text(LEFT, y, 9.5, true, accent, &format!("Nächste Wartung empfohlen: {:02}/{}", mm, jj));
        y += 9.0;
    }
    y += 2.0;
    let monteur = if w.monteur.trim().is_empty() { f.name.trim().to_string() } else { w.monteur.trim().to_string() };
    if !monteur.is_empty() {
        c.text(LEFT, y, 9.0, false, GRAY, &format!("Durchgeführt von: {monteur}"));
        y += 8.0;
    }
    unterschrift_feld(&mut pages, &mut c, y, "Wartung durchgeführt – bestätigt durch den Kunden", w.unterschrift.as_ref(), "Unterschrift Kunde");
    pages.push(c);
    let total = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        footer(p, f, i + 1, total);
    }
    assemble(pages, logo, &format!("Wartungsprotokoll {}", w.nummer), &f.name, &w.datum, None)
}

/// Angebot oder Rechnung als PDF.
pub fn dokument_pdf(data: &Data, doc: &Dokument, logo: Option<&[u8]>) -> Vec<u8> {
    let logo = logo.filter(|b| image::is_png(b) || image::is_jpeg(b));
    let pages = dokument_seiten(data, doc, logo);
    assemble(pages, logo, &format!("{} {}", doc.titel(), doc.nummer), &data.firma.name, &doc.datum, None)
}

/// Rechnung als ZUGFeRD-/Factur-X-PDF (PDF/A-3b mit eingebetteter XML-Rechnung, Profil EN 16931).
pub fn zugferd_pdf(data: &Data, doc: &Dokument, logo: Option<&[u8]>) -> Vec<u8> {
    let logo = logo.filter(|b| image::is_png(b) || image::is_jpeg(b));
    let pages = dokument_seiten(data, doc, logo);
    let xml = erechnung::cii_xml(data, doc, erechnung::Profil::Zugferd);
    let anhang = Anhang { name: "factur-x.xml", daten: xml.as_bytes(), stufe: "EN 16931" };
    assemble(pages, logo, &format!("{} {}", doc.titel(), doc.nummer), &data.firma.name, &doc.datum, Some(anhang))
}

// ---------------------------------------------------------------------------
// Zahlungserinnerung / Mahnung
// ---------------------------------------------------------------------------

/// Mahnschreiben (Index `idx` in `doc.mahnungen`) mit angehängter Rechnungskopie.
pub fn mahnung_pdf(data: &Data, doc: &Dokument, idx: usize, logo: Option<&[u8]>) -> Option<Vec<u8>> {
    let m = doc.mahnungen.get(idx)?;
    let f = &data.firma;
    let kunde = data.kunde(doc.kunde_id);
    let accent = hex_color(&f.farbe);
    let logo = logo.filter(|b| image::is_png(b) || image::is_jpeg(b));
    let brutto = doc.summen().brutto;
    let bisherige: Cents = doc.mahnungen[..idx].iter().map(|x| x.gebuehr + x.zinsen).sum();
    let bezahlt = (brutto + bisherige - m.offen_vorher).max(0);

    let mut pages: Vec<Canvas> = Vec::new();
    let mut c = Canvas::default();
    let mut info: Vec<(String, String)> = vec![
        ("Rechnungsnr.".into(), doc.nummer.clone()),
        ("Rechnungsdatum".into(), date::german(&doc.datum)),
    ];
    if let Some(k) = kunde {
        if !k.nr.is_empty() {
            info.push(("Kundennr.".into(), k.nr.clone()));
        }
    }
    info.push(("Datum".into(), date::german(&m.datum)));
    info.push(("Zahlbar bis".into(), date::german(&m.frist)));
    let mut y = briefkopf(&mut c, f, logo, &doc.empfaenger, &info);

    c.text(LEFT, y, 16.0, true, BLACK, m.label());
    y += 7.0;
    c.text(LEFT, y, 10.5, true, BLACK, &format!("Rechnung {} vom {}", doc.nummer, date::german(&doc.datum)));
    y += 8.0;
    let anrede = kunde.map(briefanrede).unwrap_or_else(|| "Sehr geehrte Damen und Herren,".into());
    y = absatz(&mut pages, &mut c, y, &anrede);
    y += 4.9;
    let text = doc.fill_mahnung(&f.mahnstufe(m.stufe).2, f, kunde, m);
    y = absatz(&mut pages, &mut c, y, &text);
    y += 6.0;

    // Aufstellung
    let mut zeilen: Vec<(String, Cents)> = vec![(format!("Rechnungsbetrag (fällig seit {})", date::german(&doc.faellig)), brutto)];
    if bisherige > 0 {
        zeilen.push(("Mahnkosten aus früheren Schreiben".into(), bisherige));
    }
    if bezahlt > 0 {
        zeilen.push(("abzüglich bereits gezahlt".into(), -bezahlt));
    }
    if m.gebuehr > 0 {
        zeilen.push(("Mahngebühr".into(), m.gebuehr));
    }
    if m.zinsen > 0 {
        let satz = format!("{:.2}", m.zins_satz).replace('.', ",");
        zeilen.push((format!("Verzugszinsen ({satz} % p. a., {} Tage)", m.zins_tage), m.zinsen));
    }
    let qr = giro_payload(f, m.gesamt(), &format!("Rechnung {}", doc.nummer));
    let block_h = (zeilen.len() as f32 * 5.2 + 14.0).max(if qr.is_some() { QR_SIZE + 12.0 } else { 0.0 });
    if y + block_h > BOTTOM {
        y = new_page(&mut pages, &mut c);
    }
    let qr_bottom = qr.as_deref().and_then(|p| girocode_block(&mut c, LEFT, y - 3.5, p));
    let lx = if qr.is_some() { 62.0 } else { 100.0 };
    for (l, v) in &zeilen {
        summe_zeile(&mut c, lx, y, l, &fmt_eur(*v));
        y += 5.2;
    }
    y += 0.6;
    summe_total(&mut c, lx, y, "Zu zahlender Betrag", &fmt_eur(m.gesamt()), accent);
    y += 10.0;
    if let Some(b) = qr_bottom {
        y = y.max(b + 5.0);
    }
    if !f.iban.trim().is_empty() {
        let bank = format!(
            "Bankverbindung: {}{}IBAN {}{} · Verwendungszweck: Rechnung {}",
            f.bank.trim(),
            if f.bank.trim().is_empty() { "" } else { " · " },
            f.iban.trim(),
            if f.bic.trim().is_empty() { String::new() } else { format!(" · BIC {}", f.bic.trim()) },
            doc.nummer
        );
        for l in wrap(&bank, RIGHT - LEFT, 8.5, false) {
            c.text(LEFT, y, 8.5, false, GRAY, &l);
            y += 4.2;
        }
        y += 3.0;
    }
    y += 2.0;
    gruss(&mut pages, &mut c, y, f);
    pages.push(c);
    let total = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        footer(p, f, i + 1, total);
    }
    // Rechnungskopie anhängen
    for mut p in dokument_seiten(data, doc, logo) {
        p.text_right(RIGHT, 8.0, 9.0, true, GRAY, "KOPIE");
        pages.push(p);
    }
    Some(assemble(pages, logo, &format!("{} zu Rechnung {}", m.label(), doc.nummer), &f.name, &m.datum, None))
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
    assemble(pages, logo, &format!("Finanzübersicht {}", a.jahr), &f.name, heute, None)
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
                    ..Default::default()
                })
            .collect();
        (d, r)
    }

    #[test]
    fn invoice_pdf_is_valid() {
        let (d, r) = sample();
        let doc = d.dokument(Some(r)).unwrap();
        let bytes = dokument_pdf(&d, doc, None);
        assert!(bytes.starts_with(b"%PDF-1.7"));
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
    fn girocode_and_dunning_letter() {
        let (mut d, r) = sample();
        d.firma.bic = "SSKMDEMMXXX".into();
        d.dokument_mut(r).unwrap().status = DokStatus::Versendet;
        let doc = d.dokument(Some(r)).unwrap().clone();
        let with_qr = dokument_pdf(&d, &doc, None);
        d.firma.girocode = false;
        let without = dokument_pdf(&d, &doc, None);
        assert!(with_qr.len() > without.len(), "GiroCode fehlt im PDF");
        d.firma.girocode = true;
        let i = d.mahnung_erstellen(r, 1, "2026-12-01", "2026-12-11", 500, true).unwrap();
        d.zahlung_erfassen(r, "2026-12-05", 1_000, "Bar");
        let doc = d.dokument(Some(r)).unwrap().clone();
        let pdf = mahnung_pdf(&d, &doc, i, None).unwrap();
        let s = String::from_utf8_lossy(&pdf);
        assert!(pdf.starts_with(b"%PDF"));
        // Mahnschreiben + Rechnungskopie (mehrseitig)
        let count: usize = s.split("/Count ").nth(1).unwrap().split(' ').next().unwrap().parse().unwrap();
        assert!(count >= 4, "Seiten: {count}");
        assert!(mahnung_pdf(&d, &doc, 5, None).is_none());
    }

    #[test]
    fn delivery_note_without_prices_and_signature() {
        let (mut d, r) = sample();
        let ls = d.folgedokument(r, DokArt::Lieferschein, "2026-11-12").unwrap();
        let ohne = dokument_pdf(&d, d.dokument(Some(ls)).unwrap(), None);
        assert!(ohne.starts_with(b"%PDF"));
        let u = Unterschrift {
            striche: vec![(0..40).map(|i| [i as f32 / 40.0, 0.1 + (i as f32 * 0.3).sin() * 0.05, 0.5]).collect()],
            name: "Anna Schön".into(),
            ort: "München".into(),
            datum: "2026-11-12".into(),
        };
        assert!(d.unterschreiben(ls, u, "2026-11-12"));
        let mit = dokument_pdf(&d, d.dokument(Some(ls)).unwrap(), None);
        assert!(mit.len() > ohne.len());
        let ab = d.folgedokument(r, DokArt::Auftragsbestaetigung, "2026-11-12").unwrap();
        assert!(dokument_pdf(&d, d.dokument(Some(ab)).unwrap(), None).starts_with(b"%PDF"));
    }

    #[test]
    fn zugferd_pdfa3() {
        let (mut d, r) = sample();
        d.firma.email = "info@muster.de".into();
        d.firma.telefon = "089 1".into();
        let doc = d.dokument(Some(r)).unwrap().clone();
        let pdf = zugferd_pdf(&d, &doc, None);
        let s = String::from_utf8_lossy(&pdf);
        assert!(pdf.starts_with(b"%PDF-1.7"));
        for teil in ["/OutputIntents", "/GTS_PDFA1", "/AFRelationship /Alternative", "(factur-x.xml)", "/Subtype /text#2Fxml", "<pdfaid:part>3</pdfaid:part>", "<fx:ConformanceLevel>EN 16931</fx:ConformanceLevel>", "/FontFile2", "/ID [<"] {
            assert!(s.contains(teil), "fehlt: {teil}");
        }
        assert!(!s.contains("/BaseFont /Helvetica"), "nur eingebettete Schriften");
        // normale PDFs sind ebenfalls PDF/A (ohne Anhang)
        let normal = String::from_utf8_lossy(&dokument_pdf(&d, &doc, None)).to_string();
        assert!(normal.contains("/OutputIntents") && !normal.contains("factur-x.xml"));
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

    #[test]
    fn wartungsprotokoll_pdf() {
        let mut d = Data::new();
        d.firma.name = "Heizung Muster".into();
        let k = d.neuer_kunde("2026-01-01");
        let a = d.neue_anlage(Some(k));
        d.anlage_mut(a).unwrap().hersteller = "Vaillant".into();
        let w = d.wartung_starten(a, "Max Monteur", "2026-11-13").unwrap();
        {
            let x = d.wartung_mut(w).unwrap();
            x.pruefpunkte[0].status = crate::engine::anlagen::Pruefung::Mangel;
            x.pruefpunkte[0].notiz = "Brennerdichtung porös – Austausch empfohlen".into();
            x.messwerte[0].wert = "48".into();
            x.arbeiten = "Wartung nach Herstellervorgabe durchgeführt.".into();
        }
        d.wartung_abschliessen(w);
        let pdf = wartung_pdf(&d, d.wartung(Some(w)).unwrap(), None);
        assert!(pdf.starts_with(b"%PDF"));
        assert!(pdf.len() > 2_000);
    }
}
