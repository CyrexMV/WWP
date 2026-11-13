//! Geldbeträge in Cent (ganzzahlig, keine Rundungsfehler) und deutsche Zahlenformate.

/// Betrag in Cent.
pub type Cents = i64;

fn group(mut n: u64) -> String {
    let mut parts = Vec::new();
    loop {
        if n < 1000 {
            parts.push(n.to_string());
            break;
        }
        parts.push(format!("{:03}", n % 1000));
        n /= 1000;
    }
    parts.reverse();
    parts.join(".")
}

/// „1.234,56“
pub fn fmt_num(c: Cents) -> String {
    let neg = c < 0;
    let a = c.unsigned_abs();
    format!("{}{},{:02}", if neg { "-" } else { "" }, group(a / 100), a % 100)
}

/// „1.234,56 €“
pub fn fmt_eur(c: Cents) -> String {
    format!("{} €", fmt_num(c))
}

/// Menge: „1“, „1,5“, „0,25“ (höchstens drei Nachkommastellen).
pub fn fmt_qty(q: f64) -> String {
    let s = format!("{:.3}", q);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    let s = if s == "-0" { "0" } else { s };
    s.replace('.', ",")
}

/// Zahl im deutschen oder englischen Format lesen: „1.234,56“, „1234,5“, „1234.56“, „12 €“.
pub fn parse_decimal(s: &str) -> Option<f64> {
    let t: String = s.chars().filter(|c| !c.is_whitespace() && *c != '€').collect();
    if t.is_empty() {
        return None;
    }
    let norm = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else if t.matches('.').count() > 1 {
        t.replace('.', "")
    } else if let Some(p) = t.find('.') {
        // „1.234“ (genau drei Ziffern nach dem Punkt) ist im Deutschen ein Tausenderpunkt
        if t.len() - p - 1 == 3 && !t.starts_with('.') && !t.starts_with("-.") {
            t.replace('.', "")
        } else {
            t
        }
    } else {
        t
    };
    norm.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Geldbetrag lesen und auf Cent runden.
pub fn parse_amount(s: &str) -> Option<Cents> {
    parse_decimal(s).map(|v| (v * 100.0).round() as Cents)
}

/// Cent × Menge, kaufmännisch gerundet.
pub fn times(c: Cents, q: f64) -> Cents {
    (c as f64 * q).round() as Cents
}

/// Steuer auf einen Nettobetrag.
pub fn tax_of_net(net: Cents, rate: u8) -> Cents {
    (net as f64 * rate as f64 / 100.0).round() as Cents
}

/// Netto aus Brutto.
pub fn net_of_gross(gross: Cents, rate: u8) -> Cents {
    (gross as f64 * 100.0 / (100.0 + rate as f64)).round() as Cents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt_eur(123_456), "1.234,56 €");
        assert_eq!(fmt_eur(-5), "-0,05 €");
        assert_eq!(fmt_num(100_000_000), "1.000.000,00");
        assert_eq!(fmt_qty(1.5), "1,5");
        assert_eq!(fmt_qty(2.0), "2");
    }

    #[test]
    fn parses() {
        assert_eq!(parse_amount("1.234,56"), Some(123_456));
        assert_eq!(parse_amount("1234,5"), Some(123_450));
        assert_eq!(parse_amount("12.50"), Some(1250));
        assert_eq!(parse_amount("1.250"), Some(125_000));
        assert_eq!(parse_amount("99 €"), Some(9900));
        assert_eq!(parse_amount("abc"), None);
        assert_eq!(parse_decimal("0,25"), Some(0.25));
    }

    #[test]
    fn taxes() {
        assert_eq!(tax_of_net(10_000, 19), 1900);
        assert_eq!(net_of_gross(11_900, 19), 10_000);
        assert_eq!(times(1999, 3.0), 5997);
    }
}
