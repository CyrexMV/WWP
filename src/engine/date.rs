//! Datumsrechnung auf ISO-Daten („JJJJ-MM-TT“), wie sie `<input type="date">` liefert.

/// Tage seit 1970-01-01 (proleptischer gregorianischer Kalender).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Umkehrung von [`days_from_civil`].
pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// Zerlegt „JJJJ-MM-TT“.
pub fn parse(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.trim().splitn(3, '-');
    let y = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    let d = it.next()?.get(..2).unwrap_or("").parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

pub fn iso(y: i32, m: u32, d: u32) -> String {
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Tage seit 1970 für ein ISO-Datum.
pub fn to_days(s: &str) -> Option<i64> {
    let (y, m, d) = parse(s)?;
    Some(days_from_civil(y, m, d))
}

pub fn from_days(z: i64) -> String {
    let (y, m, d) = civil_from_days(z);
    iso(y, m, d)
}

/// Datum + `n` Tage.
pub fn add_days(s: &str, n: i64) -> String {
    match to_days(s) {
        Some(z) => from_days(z + n),
        None => s.to_string(),
    }
}

/// Tage von `a` bis `b` (positiv, wenn `b` später ist).
pub fn diff_days(a: &str, b: &str) -> Option<i64> {
    Some(to_days(b)? - to_days(a)?)
}

/// „12.11.2026“
pub fn german(s: &str) -> String {
    match parse(s) {
        Some((y, m, d)) => format!("{:02}.{:02}.{:04}", d, m, y),
        None => s.to_string(),
    }
}

/// „12.11.“ (ohne Jahr)
pub fn german_short(s: &str) -> String {
    match parse(s) {
        Some((_, m, d)) => format!("{:02}.{:02}.", d, m),
        None => s.to_string(),
    }
}

pub fn year(s: &str) -> Option<i32> {
    parse(s).map(|p| p.0)
}

pub fn month(s: &str) -> Option<u32> {
    parse(s).map(|p| p.1)
}

pub const MONTHS: [&str; 12] = [
    "Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember",
];

pub const MONTHS_SHORT: [&str; 12] = ["Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for z in [-1000, 0, 1, 365, 19_000, 20_769, 30_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn adding() {
        assert_eq!(add_days("2026-12-20", 14), "2027-01-03");
        assert_eq!(add_days("2028-02-28", 1), "2028-02-29");
        assert_eq!(german("2026-11-12"), "12.11.2026");
        assert_eq!(diff_days("2026-11-01", "2026-11-15"), Some(14));
        assert_eq!(parse("kaputt"), None);
    }
}
