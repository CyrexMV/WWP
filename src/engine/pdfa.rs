//! Bausteine für PDF/A-3b (Archiv-PDF, Grundlage für ZUGFeRD/Factur-X):
//! sRGB-Farbprofil (ICC v2) für den OutputIntent und XMP-Metadaten inkl. Factur-X-Erweiterung.

fn s15(v: f64) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_be_bytes()
}

fn xyz(x: f64, y: f64, z: f64) -> Vec<u8> {
    let mut v = b"XYZ \0\0\0\0".to_vec();
    v.extend_from_slice(&s15(x));
    v.extend_from_slice(&s15(y));
    v.extend_from_slice(&s15(z));
    v
}

fn desc(text: &str) -> Vec<u8> {
    let mut v = b"desc\0\0\0\0".to_vec();
    v.extend_from_slice(&((text.len() + 1) as u32).to_be_bytes());
    v.extend_from_slice(text.as_bytes());
    v.push(0);
    v.extend_from_slice(&[0; 4]); // Unicode-Sprachcode
    v.extend_from_slice(&[0; 4]); // Unicode-Länge
    v.extend_from_slice(&[0; 2]); // ScriptCode-Code
    v.push(0); // ScriptCode-Länge
    v.extend_from_slice(&[0; 67]);
    v
}

/// Kompaktes sRGB-Profil (ICC v2.1, Monitorprofil, Gamma 2,2, D50-angepasste Primärfarben).
pub fn srgb_icc() -> Vec<u8> {
    let mut trc = b"curv\0\0\0\0".to_vec();
    trc.extend_from_slice(&1u32.to_be_bytes());
    trc.extend_from_slice(&0x0233u16.to_be_bytes()); // 2,2 als u8Fixed8
    trc.extend_from_slice(&[0, 0]);
    let mut cprt = b"text\0\0\0\0".to_vec();
    cprt.extend_from_slice(b"No copyright, use freely\0");
    let daten: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"desc", desc("sRGB IEC61966-2.1")),
        (b"cprt", cprt),
        (b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        (b"rXYZ", xyz(0.4361, 0.2225, 0.0139)),
        (b"gXYZ", xyz(0.3851, 0.7169, 0.0971)),
        (b"bXYZ", xyz(0.1431, 0.0606, 0.7141)),
        (b"rTRC", trc.clone()),
    ];
    let n_tags = daten.len() + 2; // gTRC/bTRC teilen sich die Daten von rTRC
    let mut offset = 128 + 4 + n_tags * 12;
    let mut table = Vec::new();
    let mut body = Vec::new();
    let mut trc_pos = (0, 0);
    for (sig, d) in &daten {
        while !(offset + body.len() - (128 + 4 + n_tags * 12)).is_multiple_of(4) {
            body.push(0);
        }
        let pos = 128 + 4 + n_tags * 12 + body.len();
        table.extend_from_slice(*sig);
        table.extend_from_slice(&(pos as u32).to_be_bytes());
        table.extend_from_slice(&(d.len() as u32).to_be_bytes());
        if *sig == b"rTRC" {
            trc_pos = (pos, d.len());
        }
        body.extend_from_slice(d);
    }
    for sig in [b"gTRC", b"bTRC"] {
        table.extend_from_slice(sig);
        table.extend_from_slice(&(trc_pos.0 as u32).to_be_bytes());
        table.extend_from_slice(&(trc_pos.1 as u32).to_be_bytes());
    }
    while body.len() % 4 != 0 {
        body.push(0);
    }
    offset = 128 + 4 + table.len() + body.len();
    let mut h = Vec::with_capacity(offset);
    h.extend_from_slice(&(offset as u32).to_be_bytes());
    h.extend_from_slice(&[0; 4]); // CMM
    h.extend_from_slice(&0x0210_0000u32.to_be_bytes());
    h.extend_from_slice(b"mntr");
    h.extend_from_slice(b"RGB ");
    h.extend_from_slice(b"XYZ ");
    for v in [2026u16, 1, 1, 0, 0, 0] {
        h.extend_from_slice(&v.to_be_bytes());
    }
    h.extend_from_slice(b"acsp");
    h.extend_from_slice(&[0; 4]); // Plattform
    h.extend_from_slice(&[0; 4]); // Flags
    h.extend_from_slice(&[0; 4]); // Hersteller
    h.extend_from_slice(&[0; 4]); // Modell
    h.extend_from_slice(&[0; 8]); // Attribute
    h.extend_from_slice(&[0; 4]); // Rendering Intent: perceptual
    h.extend_from_slice(&s15(0.9642));
    h.extend_from_slice(&s15(1.0));
    h.extend_from_slice(&s15(0.8249));
    h.extend_from_slice(&[0; 4]); // Ersteller
    h.extend_from_slice(&[0; 44]); // Profil-ID + reserviert
    debug_assert_eq!(h.len(), 128);
    h.extend_from_slice(&(n_tags as u32).to_be_bytes());
    h.extend_from_slice(&table);
    h.extend_from_slice(&body);
    h
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// XMP-Metadaten für PDF/A-3b; mit `facturx` zusätzlich die Factur-X/ZUGFeRD-Angaben.
pub fn xmp(title: &str, author: &str, producer: &str, datum_xmp: &str, facturx: Option<(&str, &str)>) -> String {
    let mut x = String::from("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    x += "<rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\"><pdfaid:part>3</pdfaid:part><pdfaid:conformance>B</pdfaid:conformance></rdf:Description>\n";
    x += &format!(
        "<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:format>application/pdf</dc:format><dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>",
        esc(title)
    );
    if !author.is_empty() {
        x += &format!("<dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>", esc(author));
    }
    x += "</rdf:Description>\n";
    x += &format!("<rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\"><pdf:Producer>{}</pdf:Producer></rdf:Description>\n", esc(producer));
    x += &format!(
        "<rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"><xmp:CreatorTool>{}</xmp:CreatorTool><xmp:CreateDate>{d}</xmp:CreateDate><xmp:ModifyDate>{d}</xmp:ModifyDate><xmp:MetadataDate>{d}</xmp:MetadataDate></rdf:Description>\n",
        esc(producer),
        d = datum_xmp
    );
    if let Some((datei, stufe)) = facturx {
        x += "<rdf:Description rdf:about=\"\" xmlns:pdfaExtension=\"http://www.aiim.org/pdfa/ns/extension/\" xmlns:pdfaSchema=\"http://www.aiim.org/pdfa/ns/schema#\" xmlns:pdfaProperty=\"http://www.aiim.org/pdfa/ns/property#\"><pdfaExtension:schemas><rdf:Bag><rdf:li rdf:parseType=\"Resource\"><pdfaSchema:schema>Factur-X PDFA Extension Schema</pdfaSchema:schema><pdfaSchema:namespaceURI>urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#</pdfaSchema:namespaceURI><pdfaSchema:prefix>fx</pdfaSchema:prefix><pdfaSchema:property><rdf:Seq>";
        for (name, beschr) in [
            ("DocumentFileName", "name of the embedded XML invoice file"),
            ("DocumentType", "INVOICE"),
            ("Version", "The actual version of the Factur-X XML schema"),
            ("ConformanceLevel", "The conformance level of the embedded Factur-X data"),
        ] {
            x += &format!("<rdf:li rdf:parseType=\"Resource\"><pdfaProperty:name>{name}</pdfaProperty:name><pdfaProperty:valueType>Text</pdfaProperty:valueType><pdfaProperty:category>external</pdfaProperty:category><pdfaProperty:description>{beschr}</pdfaProperty:description></rdf:li>");
        }
        x += "</rdf:Seq></pdfaSchema:property></rdf:li></rdf:Bag></pdfaExtension:schemas></rdf:Description>\n";
        x += &format!(
            "<rdf:Description rdf:about=\"\" xmlns:fx=\"urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#\"><fx:DocumentType>INVOICE</fx:DocumentType><fx:DocumentFileName>{}</fx:DocumentFileName><fx:Version>1.0</fx:Version><fx:ConformanceLevel>{}</fx:ConformanceLevel></rdf:Description>\n",
            esc(datei),
            esc(stufe)
        );
    }
    x += "</rdf:RDF>\n</x:xmpmeta>\n";
    // Füllraum für spätere Änderungen in-place (üblich in XMP-Paketen)
    for _ in 0..20 {
        x += "                                                                                \n";
    }
    x += "<?xpacket end=\"w\"?>";
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icc_profil() {
        let p = srgb_icc();
        assert_eq!(u32::from_be_bytes([p[0], p[1], p[2], p[3]]) as usize, p.len());
        assert_eq!(&p[12..16], b"mntr");
        assert_eq!(&p[16..20], b"RGB ");
        assert_eq!(&p[36..40], b"acsp");
        let n = u32::from_be_bytes([p[128], p[129], p[130], p[131]]) as usize;
        assert_eq!(n, 9);
        for i in 0..n {
            let e = 132 + i * 12;
            let off = u32::from_be_bytes([p[e + 4], p[e + 5], p[e + 6], p[e + 7]]) as usize;
            let len = u32::from_be_bytes([p[e + 8], p[e + 9], p[e + 10], p[e + 11]]) as usize;
            assert!(off.is_multiple_of(4) && off + len <= p.len(), "Tag {i}");
        }
    }

    #[test]
    fn xmp_wohlgeformt() {
        let x = xmp("Rechnung RE-1 & Co", "Müller <GmbH>", "Zento", "2026-11-13T00:00:00+00:00", Some(("factur-x.xml", "EN 16931")));
        assert!(x.contains("<pdfaid:part>3</pdfaid:part>"));
        assert!(x.contains("RE-1 &amp; Co"));
        assert!(x.contains("Müller &lt;GmbH&gt;"));
        assert!(x.contains("<fx:ConformanceLevel>EN 16931</fx:ConformanceLevel>"));
    }
}
