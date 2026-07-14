use crate::diagnostic::{Diagnostic, Rule};

/// A table row can't be shortened without breaking its column structure, so
/// `StyleLineLength` exempts them rather than reporting an unfixable violation.
/// Shared with `style_fix.rs`, which uses the same definition to decide what
/// `fix_style` must leave alone. A `|` inside an inline code span (`` `...` ``)
/// doesn't count — that's prose using a literal pipe character, not a table
/// delimiter, so it must not blanket-exempt the line from length checks.
pub(crate) fn is_table_row(line: &str) -> bool {
    strip_inline_code(line).contains('|')
}

/// Removes the contents of every inline code span (`` `...` ``) from `line`,
/// keeping everything outside of backticks. An unterminated trailing backtick
/// span is stripped to the end of the line, since there's no closing tick to
/// find a boundary at.
fn strip_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for ch in line.chars() {
        if ch == '`' {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            out.push(ch);
        }
    }
    out
}

/// Runs the 5 markdown hygiene rules (line length, trailing whitespace, trailing
/// newline, consecutive blank lines, hard tabs) uniformly over `content`, independent
/// of any OKF-specific structural checks. `StyleLineLength` is the one exception to
/// "uniform": table rows are exempt (see `is_table_row`).
pub fn check_style(content: &str, max_line_length: usize) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if content.is_empty() || !content.ends_with('\n') || content.ends_with("\n\n") {
        diagnostics.push(Diagnostic {
            line: 1,
            rule: Rule::StyleTrailingNewline,
            message: "file must end with exactly one trailing newline".to_string(),
        });
    }

    if content.is_empty() {
        return diagnostics;
    }

    let mut lines: Vec<&str> = content.split('\n').collect();
    if content.ends_with('\n') {
        lines.pop();
    }

    let mut blank_run = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;

        let char_count = line.chars().count();
        if char_count > max_line_length && !is_table_row(line) {
            diagnostics.push(Diagnostic {
                line: line_no,
                rule: Rule::StyleLineLength,
                message: format!(
                    "line exceeds maximum length of {max_line_length} characters ({char_count} found)"
                ),
            });
        }

        if line.ends_with(' ') || line.ends_with('\t') || line.ends_with('\r') {
            diagnostics.push(Diagnostic {
                line: line_no,
                rule: Rule::StyleTrailingWhitespace,
                message: "line has trailing whitespace".to_string(),
            });
        }

        if line.contains('\t') {
            diagnostics.push(Diagnostic {
                line: line_no,
                rule: Rule::StyleHardTab,
                message: "line contains a hard tab character".to_string(),
            });
        }

        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run == 2 {
                diagnostics.push(Diagnostic {
                    line: line_no,
                    rule: Rule::StyleConsecutiveBlankLines,
                    message: "multiple consecutive blank lines".to_string(),
                });
            }
        } else {
            blank_run = 0;
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: usize = 100;

    fn fixture(name: &str, kind: &str) -> String {
        std::fs::read_to_string(format!("tests/fixtures/style/{name}/{kind}/{kind}.md")).unwrap()
    }

    #[test]
    fn pass_fixtures_have_no_diagnostics() {
        for name in [
            "max_line_length",
            "trailing_whitespace",
            "trailing_newline",
            "consecutive_blank_lines",
            "hard_tabs",
        ] {
            let content = fixture(name, "pass");
            let diags = check_style(&content, MAX);
            assert!(diags.is_empty(), "{name} pass fixture produced {diags:?}");
        }
    }

    #[test]
    fn max_line_length_fail_fixture() {
        let content = fixture("max_line_length", "fail");
        let diags = check_style(&content, MAX);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::StyleLineLength);
        assert_eq!(diags[0].line, 1);
        assert_eq!(
            diags[0].message,
            "line exceeds maximum length of 100 characters (105 found)"
        );
    }

    #[test]
    fn trailing_whitespace_fail_fixture() {
        let content = fixture("trailing_whitespace", "fail");
        let diags = check_style(&content, MAX);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::StyleTrailingWhitespace);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[0].message, "line has trailing whitespace");
    }

    #[test]
    fn trailing_newline_fail_fixture() {
        let content = fixture("trailing_newline", "fail");
        let diags = check_style(&content, MAX);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::StyleTrailingNewline);
        assert_eq!(diags[0].line, 1);
        assert_eq!(
            diags[0].message,
            "file must end with exactly one trailing newline"
        );
    }

    #[test]
    fn consecutive_blank_lines_fail_fixture() {
        let content = fixture("consecutive_blank_lines", "fail");
        let diags = check_style(&content, MAX);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::StyleConsecutiveBlankLines);
        assert_eq!(diags[0].line, 3);
        assert_eq!(diags[0].message, "multiple consecutive blank lines");
    }

    #[test]
    fn hard_tabs_fail_fixture() {
        let content = fixture("hard_tabs", "fail");
        let diags = check_style(&content, MAX);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::StyleHardTab);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[0].message, "line contains a hard tab character");
    }

    #[test]
    fn multibyte_char_counted_not_bytes() {
        // "é" is 2 bytes, 1 char. 60 chars = 120 bytes but only 60 chars: under 100-char limit.
        let line = "é".repeat(60);
        let content = format!("{line}\n");
        let diags = check_style(&content, MAX);
        assert!(diags.iter().all(|d| d.rule != Rule::StyleLineLength));

        // 101 chars of "é": char count (101) exceeds the 100-char limit even though
        // byte length (202) is what a naive byte-length check would also flag anyway;
        // the point is char count, not bytes, drives the decision.
        let line = "é".repeat(101);
        let content = format!("{line}\n");
        let diags = check_style(&content, MAX);
        let line_len_diag = diags
            .iter()
            .find(|d| d.rule == Rule::StyleLineLength)
            .expect("expected StyleLineLength diagnostic");
        assert_eq!(
            line_len_diag.message,
            "line exceeds maximum length of 100 characters (101 found)"
        );
    }

    #[test]
    fn crlf_line_triggers_trailing_whitespace() {
        let content = "first line\r\nsecond line\n";
        let diags = check_style(content, MAX);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::StyleTrailingWhitespace);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn zero_byte_file_violates_trailing_newline() {
        let diags = check_style("", MAX);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, Rule::StyleTrailingNewline);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn no_trailing_newline_at_all_violates() {
        let diags = check_style("no newline at end", MAX);
        assert!(diags.iter().any(|d| d.rule == Rule::StyleTrailingNewline));
    }

    #[test]
    fn double_trailing_newline_violates() {
        let diags = check_style("content\n\n", MAX);
        assert!(diags.iter().any(|d| d.rule == Rule::StyleTrailingNewline));
    }

    #[test]
    fn single_trailing_newline_is_fine() {
        let diags = check_style("content\n", MAX);
        assert!(diags.iter().all(|d| d.rule != Rule::StyleTrailingNewline));
    }

    #[test]
    fn exactly_two_blank_lines_anchors_on_second() {
        let content = "a\n\n\nb\n";
        let diags = check_style(content, MAX);
        let blank_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == Rule::StyleConsecutiveBlankLines)
            .collect();
        assert_eq!(blank_diags.len(), 1);
        assert_eq!(blank_diags[0].line, 3);
    }

    #[test]
    fn five_blank_line_run_produces_one_diagnostic() {
        let content = "a\n\n\n\n\n\nb\n";
        let diags = check_style(content, MAX);
        let blank_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == Rule::StyleConsecutiveBlankLines)
            .collect();
        assert_eq!(blank_diags.len(), 1);
        assert_eq!(blank_diags[0].line, 3);
    }

    #[test]
    fn two_separate_blank_runs_produce_two_diagnostics() {
        let content = "a\n\n\nb\n\n\nc\n";
        let diags = check_style(content, MAX);
        let blank_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == Rule::StyleConsecutiveBlankLines)
            .collect();
        assert_eq!(blank_diags.len(), 2);
        assert_eq!(blank_diags[0].line, 3);
        assert_eq!(blank_diags[1].line, 6);
    }

    #[test]
    fn tab_mid_line_and_trailing_fires_both_rules() {
        let content = "foo\tbar\t\n";
        let diags = check_style(content, MAX);
        assert!(diags.iter().any(|d| d.rule == Rule::StyleHardTab));
        assert!(
            diags
                .iter()
                .any(|d| d.rule == Rule::StyleTrailingWhitespace)
        );
    }

    #[test]
    fn trailing_blank_lines_fire_both_newline_and_blank_run_rules() {
        // "content\n\n\n": double/extra trailing newline AND a 2-line blank run at EOF.
        // The two checks are independent, so both fire on the same content.
        let content = "content\n\n\n";
        let diags = check_style(content, MAX);
        assert!(diags.iter().any(|d| d.rule == Rule::StyleTrailingNewline));
        assert!(
            diags
                .iter()
                .any(|d| d.rule == Rule::StyleConsecutiveBlankLines)
        );
    }

    #[test]
    fn overlength_line_with_trailing_whitespace_fires_both_rules() {
        let content = format!("{} \n", "a".repeat(101));
        let diags = check_style(&content, MAX);
        assert!(diags.iter().any(|d| d.rule == Rule::StyleLineLength));
        assert!(
            diags
                .iter()
                .any(|d| d.rule == Rule::StyleTrailingWhitespace)
        );
    }

    #[test]
    fn overlength_table_row_is_exempt_from_line_length_rule() {
        let content = format!("| {} |\n", "a".repeat(101));
        let diags = check_style(&content, MAX);
        assert!(diags.iter().all(|d| d.rule != Rule::StyleLineLength));
    }

    #[test]
    fn overlength_prose_line_with_inline_code_pipe_is_not_exempt() {
        // A `|` inside a code span (e.g. documenting `foo | bar`) is not a table
        // delimiter — the line must still be flagged as overlong.
        let content = format!("Some text with a `foo | bar` example {}\n", "a".repeat(80));
        let diags = check_style(&content, MAX);
        assert!(diags.iter().any(|d| d.rule == Rule::StyleLineLength));
    }

    #[test]
    fn overlength_table_row_still_fires_other_style_rules() {
        let content = format!("| {} | \n", "a".repeat(101));
        let diags = check_style(&content, MAX);
        assert!(diags.iter().all(|d| d.rule != Rule::StyleLineLength));
        assert!(
            diags
                .iter()
                .any(|d| d.rule == Rule::StyleTrailingWhitespace)
        );
    }
}
