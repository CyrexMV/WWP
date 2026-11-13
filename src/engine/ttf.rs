//! Minimaler TrueType-Leser für das Einbetten von Schriften in PDF/A:
//! Metriken (Laufweiten, Ober-/Unterlänge, Begrenzungsrahmen), Zeichen→Glyph-Zuordnung (cmap 3/1)
//! und eine Teilmenge (Subset), die nur die Glyphen des WinAnsi-Zeichensatzes behält.

use std::collections::{BTreeMap, BTreeSet};

fn u16_at(b: &[u8], o: usize) -> u16 {
    b.get(o..o + 2).map(|x| u16::from_be_bytes([x[0], x[1]])).unwrap_or(0)
}
fn i16_at(b: &[u8], o: usize) -> i16 {
    u16_at(b, o) as i16
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]])).unwrap_or(0)
}

/// Unicode-Zeichen der WinAnsi-Codes 128–159.
const WINANSI_80: [u32; 32] = [
    0x20AC, 0, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0, 0x017D, 0, 0, 0x2018, 0x2019, 0x201C,
    0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0, 0x017E, 0x0178,
];

/// Unicode eines WinAnsi-Codes (0 = nicht belegt).
pub fn winansi_unicode(code: u8) -> u32 {
    match code {
        0x20..=0x7E => code as u32,
        0x80..=0x9F => WINANSI_80[(code - 0x80) as usize],
        0xA0..=0xFF => code as u32,
        _ => 0,
    }
}

pub struct Font<'a> {
    pub data: &'a [u8],
    tables: BTreeMap<[u8; 4], (usize, usize)>,
    pub units_per_em: u16,
    pub num_glyphs: u16,
}

pub struct Metrics {
    pub bbox: [i16; 4],
    pub ascent: i16,
    pub descent: i16,
    pub cap_height: i16,
    pub italic_angle: f32,
}

impl<'a> Font<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Font<'a>> {
        let n = u16_at(data, 4) as usize;
        let mut tables = BTreeMap::new();
        for i in 0..n {
            let o = 12 + i * 16;
            let tag: [u8; 4] = data.get(o..o + 4)?.try_into().ok()?;
            let off = u32_at(data, o + 8) as usize;
            let len = u32_at(data, o + 12) as usize;
            if off + len > data.len() {
                return None;
            }
            tables.insert(tag, (off, len));
        }
        let head = tables.get(b"head")?.0;
        let maxp = tables.get(b"maxp")?.0;
        Some(Font { data, units_per_em: u16_at(data, head + 18), num_glyphs: u16_at(data, maxp + 4), tables })
    }

    fn table(&self, tag: &[u8; 4]) -> Option<&'a [u8]> {
        let (o, l) = *self.tables.get(tag)?;
        Some(&self.data[o..o + l])
    }

    /// Glyph-Index eines Unicode-Zeichens (cmap 3/1, Format 4).
    pub fn glyph(&self, ch: u32) -> u16 {
        let Some(cmap) = self.table(b"cmap") else { return 0 };
        let n = u16_at(cmap, 2) as usize;
        for i in 0..n {
            let rec = 4 + i * 8;
            let (pid, eid, off) = (u16_at(cmap, rec), u16_at(cmap, rec + 2), u32_at(cmap, rec + 4) as usize);
            if pid != 3 || eid != 1 || u16_at(cmap, off) != 4 {
                continue;
            }
            let seg2 = u16_at(cmap, off + 6) as usize;
            let ends = off + 14;
            let starts = ends + seg2 + 2;
            let deltas = starts + seg2;
            let ranges = deltas + seg2;
            for s in 0..seg2 / 2 {
                let end = u16_at(cmap, ends + s * 2) as u32;
                if ch > end {
                    continue;
                }
                let start = u16_at(cmap, starts + s * 2) as u32;
                if ch < start {
                    return 0;
                }
                let delta = u16_at(cmap, deltas + s * 2);
                let ro = u16_at(cmap, ranges + s * 2) as usize;
                if ro == 0 {
                    return (ch as u16).wrapping_add(delta);
                }
                let gi = ranges + s * 2 + ro + (ch - start) as usize * 2;
                let g = u16_at(cmap, gi);
                return if g == 0 { 0 } else { g.wrapping_add(delta) };
            }
        }
        0
    }

    /// Laufweite eines Glyphs in Schrifteinheiten.
    pub fn advance(&self, gid: u16) -> u16 {
        let (Some(hhea), Some(hmtx)) = (self.table(b"hhea"), self.table(b"hmtx")) else { return 0 };
        let n = u16_at(hhea, 34).max(1);
        let i = gid.min(n - 1) as usize;
        u16_at(hmtx, i * 4)
    }

    /// Laufweiten der WinAnsi-Codes 32–255 in 1/1000 em (für /Widths im PDF).
    pub fn pdf_widths(&self) -> Vec<u32> {
        (32u8..=255)
            .map(|c| {
                let u = winansi_unicode(c);
                let g = if u == 0 { 0 } else { self.glyph(u) };
                (self.advance(g) as u32 * 1000 + self.units_per_em as u32 / 2) / self.units_per_em.max(1) as u32
            })
            .collect()
    }

    pub fn metrics(&self) -> Metrics {
        let head = self.table(b"head").unwrap_or(&[]);
        let os2 = self.table(b"OS/2").unwrap_or(&[]);
        let post = self.table(b"post").unwrap_or(&[]);
        let scale = |v: i16| ((v as i32 * 1000) / self.units_per_em.max(1) as i32) as i16;
        let italic = i16_at(post, 4) as f32 + u16_at(post, 6) as f32 / 65536.0;
        Metrics {
            bbox: [scale(i16_at(head, 36)), scale(i16_at(head, 38)), scale(i16_at(head, 40)), scale(i16_at(head, 42))],
            ascent: scale(i16_at(os2, 68)),
            descent: scale(i16_at(os2, 70)),
            cap_height: if u16_at(os2, 0) >= 2 { scale(i16_at(os2, 88)) } else { scale(i16_at(os2, 68)) * 7 / 10 },
            italic_angle: italic,
        }
    }

    fn glyph_range(&self, gid: u16) -> (usize, usize) {
        let (Some(head), Some(loca)) = (self.table(b"head"), self.table(b"loca")) else { return (0, 0) };
        let long = i16_at(head, 50) == 1;
        let g = gid as usize;
        if long {
            (u32_at(loca, g * 4) as usize, u32_at(loca, g * 4 + 4) as usize)
        } else {
            (u16_at(loca, g * 2) as usize * 2, u16_at(loca, g * 2 + 2) as usize * 2)
        }
    }

    /// Teilmenge: alle Tabellen bleiben, aber nur die Glyphen der WinAnsi-Zeichen (plus Bausteine
    /// zusammengesetzter Glyphen und .notdef) behalten ihre Umrisse. Glyph-Nummern bleiben gleich.
    pub fn subset_winansi(&self) -> Option<Vec<u8>> {
        let glyf = self.table(b"glyf")?;
        let mut keep: BTreeSet<u16> = BTreeSet::new();
        keep.insert(0);
        for c in 32u8..=255 {
            let u = winansi_unicode(c);
            if u != 0 {
                keep.insert(self.glyph(u));
            }
        }
        // Bausteine zusammengesetzter Glyphen
        let mut todo: Vec<u16> = keep.iter().copied().collect();
        while let Some(g) = todo.pop() {
            let (a, b) = self.glyph_range(g);
            if b <= a || b > glyf.len() || i16_at(glyf, a) >= 0 {
                continue;
            }
            let mut o = a + 10;
            loop {
                let flags = u16_at(glyf, o);
                let comp = u16_at(glyf, o + 2);
                if keep.insert(comp) {
                    todo.push(comp);
                }
                o += 4 + if flags & 1 != 0 { 4 } else { 2 };
                o += if flags & 8 != 0 {
                    2
                } else if flags & 0x40 != 0 {
                    4
                } else if flags & 0x80 != 0 {
                    8
                } else {
                    0
                };
                if flags & 0x20 == 0 || o >= b {
                    break;
                }
            }
        }
        let mut new_glyf = Vec::new();
        let mut loca = Vec::with_capacity((self.num_glyphs as usize + 1) * 4);
        for g in 0..self.num_glyphs {
            loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
            if keep.contains(&g) {
                let (a, b) = self.glyph_range(g);
                if b > a && b <= glyf.len() {
                    new_glyf.extend_from_slice(&glyf[a..b]);
                    while new_glyf.len() % 4 != 0 {
                        new_glyf.push(0);
                    }
                }
            }
        }
        loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());

        let mut head = self.table(b"head")?.to_vec();
        head[50..52].copy_from_slice(&1i16.to_be_bytes()); // lange loca
        head[8..12].copy_from_slice(&[0, 0, 0, 0]);
        const KEEP: [&[u8; 4]; 13] = [b"OS/2", b"cmap", b"cvt ", b"fpgm", b"gasp", b"glyf", b"head", b"hhea", b"hmtx", b"loca", b"maxp", b"name", b"prep"];
        let mut out_tables: Vec<([u8; 4], Vec<u8>)> = Vec::new();
        for tag in KEEP {
            let data = match tag {
                b"glyf" => new_glyf.clone(),
                b"loca" => loca.clone(),
                b"head" => head.clone(),
                _ => match self.table(tag) {
                    Some(t) => t.to_vec(),
                    None => continue,
                },
            };
            out_tables.push((*tag, data));
        }
        // post Version 3 (ohne Glyphnamen)
        let mut post = self.table(b"post").map(|p| p[..32.min(p.len())].to_vec()).unwrap_or_else(|| vec![0; 32]);
        post.resize(32, 0);
        post[0..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
        out_tables.push((*b"post", post));
        out_tables.sort_by_key(|a| a.0);

        let n = out_tables.len() as u16;
        let mut es = 0u16;
        while (1u16 << (es + 1)) <= n {
            es += 1;
        }
        let sr = (1u16 << es) * 16;
        let mut out = Vec::new();
        out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        out.extend_from_slice(&n.to_be_bytes());
        out.extend_from_slice(&sr.to_be_bytes());
        out.extend_from_slice(&es.to_be_bytes());
        out.extend_from_slice(&(n * 16 - sr).to_be_bytes());
        let mut offset = 12 + out_tables.len() * 16;
        let mut dir = Vec::new();
        let mut body = Vec::new();
        let mut head_pos = 0;
        for (tag, data) in &out_tables {
            if tag == b"head" {
                head_pos = offset;
            }
            dir.extend_from_slice(tag);
            dir.extend_from_slice(&checksum(data).to_be_bytes());
            dir.extend_from_slice(&(offset as u32).to_be_bytes());
            dir.extend_from_slice(&(data.len() as u32).to_be_bytes());
            body.extend_from_slice(data);
            while body.len() % 4 != 0 {
                body.push(0);
            }
            offset = 12 + out_tables.len() * 16 + body.len();
        }
        out.extend_from_slice(&dir);
        out.extend_from_slice(&body);
        let adj = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[head_pos + 8..head_pos + 12].copy_from_slice(&adj.to_be_bytes());
        Some(out)
    }
}

fn checksum(b: &[u8]) -> u32 {
    let mut s = 0u32;
    for c in b.chunks(4) {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        s = s.wrapping_add(u32::from_be_bytes(w));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eingebettete_schriften() {
        for (data, bold) in [(super::super::fonts::REGULAR, false), (super::super::fonts::BOLD, true)] {
            let f = Font::parse(data).expect("Schrift lesbar");
            assert!(f.glyph('A' as u32) > 0 && f.glyph('ä' as u32) > 0 && f.glyph(0x20AC) > 0);
            let w = f.pdf_widths();
            assert_eq!(w.len(), 224);
            // Liberation Sans hat dieselben Laufweiten wie Helvetica/Arial
            assert_eq!(w[(b'A' - 32) as usize], if bold { 722 } else { 667 });
            assert_eq!(w[(b'0' - 32) as usize], 556);
            // alle WinAnsi-Zeichen haben einen Glyphen (PDF/A verbietet Verweise auf .notdef)
            for c in 32u8..=255 {
                let u = winansi_unicode(c);
                if u != 0 {
                    assert!(f.glyph(u) > 0, "Zeichen {c:#x} fehlt");
                }
            }
            let m = f.metrics();
            assert!(m.ascent > 700 && m.descent < 0 && m.cap_height > 600);
        }
    }
}
