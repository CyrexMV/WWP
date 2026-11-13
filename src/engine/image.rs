//! Bilder für das PDF-Logo: PNG dekodieren, JPEG-Größe lesen.

/// PNG → (Breite, Höhe, RGBA).
pub fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let data = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => data.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        _ => return None,
    };
    Some((w, h, rgba))
}

/// Breite, Höhe und Farbkanäle eines JPEG (aus dem SOF-Abschnitt).
pub fn jpeg_info(b: &[u8]) -> Option<(u32, u32, u8)> {
    if b.len() < 4 || b[0] != 0xFF || b[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = b[i + 1];
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) || m == 0xFF {
            i += if m == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if matches!(m, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            let h = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
            let w = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
            return Some((w, h, b[i + 9]));
        }
        i += 2 + len;
    }
    None
}

pub fn is_png(b: &[u8]) -> bool {
    b.len() > 8 && &b[..8] == b"\x89PNG\r\n\x1a\n"
}

pub fn is_jpeg(b: &[u8]) -> bool {
    b.len() > 3 && b[0] == 0xFF && b[1] == 0xD8
}

pub fn is_pdf(b: &[u8]) -> bool {
    b.starts_with(b"%PDF")
}

/// Bildgröße (PNG/JPEG) ohne vollständiges Dekodieren.
pub fn size(b: &[u8]) -> Option<(u32, u32)> {
    if let Some((w, h, _)) = jpeg_info(b) {
        return Some((w, h));
    }
    if is_png(b) && b.len() > 24 {
        return Some((u32::from_be_bytes([b[16], b[17], b[18], b[19]]), u32::from_be_bytes([b[20], b[21], b[22], b[23]])));
    }
    None
}
