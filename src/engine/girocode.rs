//! GiroCode (EPC-QR-Code, Standard EPC069-12): Überweisungsdaten als QR-Code,
//! den Banking-Apps scannen können. Fehlerkorrektur „M“ wie vom Standard gefordert.

use super::money::Cents;
use qrcodegen::{QrCode, QrCodeEcc};

/// IBAN ohne Leerzeichen, in Großbuchstaben.
pub fn iban_clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase()
}

/// IBAN-Prüfsumme (ISO 13616, Modulo 97).
pub fn iban_valid(s: &str) -> bool {
    let i = iban_clean(s);
    if i.len() < 15 || i.len() > 34 || !i.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    let b = i.as_bytes();
    if !b[0].is_ascii_alphabetic() || !b[1].is_ascii_alphabetic() || !b[2].is_ascii_digit() || !b[3].is_ascii_digit() {
        return false;
    }
    if i.starts_with("DE") && i.len() != 22 {
        return false;
    }
    let rearranged = format!("{}{}", &i[4..], &i[..4]);
    let mut rem: u32 = 0;
    for c in rearranged.chars() {
        let v = c.to_digit(36).unwrap_or(0);
        rem = if v >= 10 { (rem * 100 + v) % 97 } else { (rem * 10 + v) % 97 };
    }
    rem == 1
}

fn cut(s: &str, n: usize) -> String {
    s.trim().replace(['\n', '\r'], " ").chars().take(n).collect()
}

/// Inhalt des EPC-QR-Codes; `None`, wenn Angaben fehlen oder ungültig sind.
pub fn epc_payload(name: &str, iban: &str, bic: &str, betrag: Cents, zweck: &str) -> Option<String> {
    let name = cut(name, 70);
    if name.is_empty() || !iban_valid(iban) || betrag <= 0 || betrag > 99_999_999_999 {
        return None;
    }
    let bic: String = bic.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    let bic = if bic.len() == 8 || bic.len() == 11 { bic } else { String::new() };
    Some(format!(
        "BCD\n002\n1\nSCT\n{}\n{}\n{}\nEUR{}.{:02}\n\n\n{}",
        bic,
        name,
        iban_clean(iban),
        betrag / 100,
        betrag % 100,
        cut(zweck, 140)
    ))
}

/// QR-Matrix (true = dunkel), Zeile für Zeile.
pub fn matrix(payload: &str) -> Option<Vec<Vec<bool>>> {
    let qr = QrCode::encode_text(payload, QrCodeEcc::Medium).ok()?;
    let n = qr.size();
    Some((0..n).map(|y| (0..n).map(|x| qr.get_module(x, y)).collect()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iban() {
        assert!(iban_valid("DE89 3704 0044 0532 0130 00"));
        assert!(iban_valid("de89370400440532013000"));
        assert!(!iban_valid("DE89 3704 0044 0532 0130 01"));
        assert!(!iban_valid("DE89"));
        assert!(iban_valid("AT61 1904 3002 3457 3201"));
    }

    #[test]
    fn payload() {
        let p = epc_payload("Muster GmbH", "DE89 3704 0044 0532 0130 00", "COBADEFFXXX", 12_345, "Rechnung RE-2026-0001").unwrap();
        assert_eq!(p, "BCD\n002\n1\nSCT\nCOBADEFFXXX\nMuster GmbH\nDE89370400440532013000\nEUR123.45\n\n\nRechnung RE-2026-0001");
        assert!(epc_payload("", "DE89370400440532013000", "", 100, "x").is_none());
        assert!(epc_payload("A", "DE00", "", 100, "x").is_none());
        assert!(epc_payload("A", "DE89370400440532013000", "", 0, "x").is_none());
        let m = matrix(&p).unwrap();
        assert!(m.len() >= 25 && m.iter().all(|r| r.len() == m.len()));
    }
}
