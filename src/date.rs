use chrono::NaiveDate;

/// Parses `text` as a strict `YYYY-MM-DD` calendar date: exactly 10 ASCII
/// characters in digit-digit-digit-digit-dash-digit-digit-dash-digit-digit
/// shape, and a real calendar date (rejects e.g. `2026-02-30`). Shared by
/// `log.md` date headings and the OKF v0.2 `stale_after` frontmatter field.
pub fn parse_ymd(text: &str) -> Option<NaiveDate> {
    if !is_date_shape(text) {
        return None;
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

fn is_date_shape(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, &b)| {
        if i == 4 || i == 7 {
            b == b'-'
        } else {
            b.is_ascii_digit()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_date_parses() {
        assert_eq!(
            parse_ymd("2026-05-22"),
            NaiveDate::from_ymd_opt(2026, 5, 22)
        );
    }

    #[test]
    fn calendar_invalid_date_matching_shape_is_none() {
        assert_eq!(parse_ymd("2026-02-30"), None);
    }

    #[test]
    fn wrong_shape_is_none() {
        assert_eq!(parse_ymd("2026/05/22"), None);
        assert_eq!(parse_ymd("May 22 2026"), None);
        assert_eq!(parse_ymd("2026-05-22 "), None);
        assert_eq!(parse_ymd(""), None);
    }
}
