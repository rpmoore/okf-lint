use crate::diagnostic::{Diagnostic, Rule};
use chrono::NaiveDate;

/// Runs OKF conformance rule 5 (OkfLogDateHeading) against the content of a
/// log.md file: every level-2 (`##`) heading must be a real calendar date in
/// YYYY-MM-DD format, and valid dates must appear newest-first (§7). Headings
/// at other levels are not inspected.
pub fn check_log(content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut last_valid_date: Option<NaiveDate> = None;

    for (idx, line) in content.split('\n').enumerate() {
        let Some(text) = line.strip_prefix("## ") else {
            continue;
        };
        let line_no = idx + 1;

        if !is_date_shape(text) {
            diagnostics.push(invalid_date_diagnostic(line_no));
            continue;
        }

        match NaiveDate::parse_from_str(text, "%Y-%m-%d") {
            Err(_) => diagnostics.push(invalid_date_diagnostic(line_no)),
            Ok(date) => {
                // A shape/calendar-invalid heading in between two valid dates
                // is already flagged on its own line above and doesn't reset
                // ordering tracking — the two valid dates on either side of
                // it must still be compared against each other.
                if last_valid_date.is_some_and(|last| date > last) {
                    diagnostics.push(Diagnostic {
                        line: line_no,
                        rule: Rule::OkfLogDateHeading,
                        message: "log.md date headings must be in newest-first (descending) order"
                            .to_string(),
                    });
                }
                last_valid_date = Some(date);
            }
        }
    }

    diagnostics
}

fn invalid_date_diagnostic(line: usize) -> Diagnostic {
    Diagnostic {
        line,
        rule: Rule::OkfLogDateHeading,
        message: "log.md heading is not a valid YYYY-MM-DD date".to_string(),
    }
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

    const PASS: &str = include_str!("../../tests/fixtures/okf/log_date_heading/pass/pass.md");
    const FAIL: &str = include_str!("../../tests/fixtures/okf/log_date_heading/fail/fail.md");

    #[test]
    fn pass_fixture_has_no_diagnostics() {
        assert_eq!(check_log(PASS), vec![]);
    }

    #[test]
    fn fail_fixture_emits_one_diagnostic_at_correct_line() {
        assert_eq!(
            check_log(FAIL),
            vec![Diagnostic {
                line: 5,
                rule: Rule::OkfLogDateHeading,
                message: "log.md heading is not a valid YYYY-MM-DD date".to_string(),
            }]
        );
    }

    #[test]
    fn valid_date_heading_has_no_diagnostic() {
        assert_eq!(check_log("## 2026-05-22\n"), vec![]);
    }

    #[test]
    fn calendar_invalid_date_matching_regex_shape_is_a_violation() {
        assert_eq!(
            check_log("## 2026-02-30\n"),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfLogDateHeading,
                message: "log.md heading is not a valid YYYY-MM-DD date".to_string(),
            }]
        );
    }

    #[test]
    fn non_level_2_headings_are_ignored() {
        assert_eq!(check_log("# 2026-05-22 not a level 2 heading\n"), vec![]);
        assert_eq!(check_log("### 2026-05-22 not a level 2 heading\n"), vec![]);
    }

    #[test]
    fn trailing_text_after_date_is_a_violation() {
        assert_eq!(
            check_log("## 2026-05-22 Updates\n"),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfLogDateHeading,
                message: "log.md heading is not a valid YYYY-MM-DD date".to_string(),
            }]
        );
    }

    #[test]
    fn multiple_headings_emit_one_diagnostic_per_invalid_heading() {
        // Valid dates (2026-03-03, 2026-01-01) are kept newest-first here so
        // this test only exercises per-heading date-validity, not ordering.
        let content = "## 2026-03-03\n## Not A Date\n\n## 2026-02-30\n## 2026-01-01\n";
        assert_eq!(
            check_log(content),
            vec![
                Diagnostic {
                    line: 2,
                    rule: Rule::OkfLogDateHeading,
                    message: "log.md heading is not a valid YYYY-MM-DD date".to_string(),
                },
                Diagnostic {
                    line: 4,
                    rule: Rule::OkfLogDateHeading,
                    message: "log.md heading is not a valid YYYY-MM-DD date".to_string(),
                },
            ]
        );
    }

    #[test]
    fn no_level_2_headings_has_no_diagnostics() {
        assert_eq!(check_log("# Title\n\n### Sub\n\nbody text\n"), vec![]);
    }

    #[test]
    fn out_of_order_valid_dates_emit_order_violation() {
        assert_eq!(
            check_log("## 2026-01-01\n\n## 2026-02-01\n"),
            vec![Diagnostic {
                line: 3,
                rule: Rule::OkfLogDateHeading,
                message: "log.md date headings must be in newest-first (descending) order"
                    .to_string(),
            }]
        );
    }

    #[test]
    fn newest_first_order_has_no_diagnostics() {
        assert_eq!(check_log("## 2026-02-01\n\n## 2026-01-01\n"), vec![]);
    }

    #[test]
    fn equal_consecutive_dates_are_not_an_order_violation() {
        assert_eq!(check_log("## 2026-01-01\n\n## 2026-01-01\n"), vec![]);
    }

    #[test]
    fn invalid_date_between_valid_dates_does_not_reset_order_tracking() {
        assert_eq!(
            check_log("## 2026-01-01\n## Not A Date\n## 2026-02-01\n"),
            vec![
                Diagnostic {
                    line: 2,
                    rule: Rule::OkfLogDateHeading,
                    message: "log.md heading is not a valid YYYY-MM-DD date".to_string(),
                },
                Diagnostic {
                    line: 3,
                    rule: Rule::OkfLogDateHeading,
                    message: "log.md date headings must be in newest-first (descending) order"
                        .to_string(),
                },
            ]
        );
    }
}
