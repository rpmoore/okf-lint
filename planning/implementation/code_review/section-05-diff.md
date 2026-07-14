diff --git a/src/checks/mod.rs b/src/checks/mod.rs
index 15979ff..455f8e0 100644
--- a/src/checks/mod.rs
+++ b/src/checks/mod.rs
@@ -1,3 +1,4 @@
 pub mod index_md;
 pub mod log_md;
 pub mod okf;
+pub mod style;
diff --git a/src/checks/style.rs b/src/checks/style.rs
new file mode 100644
index 0000000..fcec2dc
--- /dev/null
+++ b/src/checks/style.rs
@@ -0,0 +1,259 @@
+use crate::diagnostic::{Diagnostic, Rule};
+
+pub fn check_style(content: &str, max_line_length: usize) -> Vec<Diagnostic> {
+    let mut diagnostics = Vec::new();
+
+    if content.is_empty() || !content.ends_with('\n') || content.ends_with("\n\n") {
+        diagnostics.push(Diagnostic {
+            line: 1,
+            rule: Rule::StyleTrailingNewline,
+            message: "file must end with exactly one trailing newline".to_string(),
+        });
+    }
+
+    let mut lines: Vec<&str> = content.split('\n').collect();
+    if content.ends_with('\n') {
+        lines.pop();
+    }
+
+    let mut blank_run = 0usize;
+    for (idx, line) in lines.iter().enumerate() {
+        let line_no = idx + 1;
+
+        let char_count = line.chars().count();
+        if char_count > max_line_length {
+            diagnostics.push(Diagnostic {
+                line: line_no,
+                rule: Rule::StyleLineLength,
+                message: format!(
+                    "line exceeds maximum length of {max_line_length} characters ({char_count} found)"
+                ),
+            });
+        }
+
+        if line.ends_with(' ') || line.ends_with('\t') || line.ends_with('\r') {
+            diagnostics.push(Diagnostic {
+                line: line_no,
+                rule: Rule::StyleTrailingWhitespace,
+                message: "line has trailing whitespace".to_string(),
+            });
+        }
+
+        if line.contains('\t') {
+            diagnostics.push(Diagnostic {
+                line: line_no,
+                rule: Rule::StyleHardTab,
+                message: "line contains a hard tab character".to_string(),
+            });
+        }
+
+        if line.trim().is_empty() {
+            blank_run += 1;
+            if blank_run == 2 {
+                diagnostics.push(Diagnostic {
+                    line: line_no,
+                    rule: Rule::StyleConsecutiveBlankLines,
+                    message: "multiple consecutive blank lines".to_string(),
+                });
+            }
+        } else {
+            blank_run = 0;
+        }
+    }
+
+    diagnostics
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    const MAX: usize = 100;
+
+    fn fixture(name: &str, kind: &str) -> String {
+        std::fs::read_to_string(format!("tests/fixtures/style/{name}/{kind}/{kind}.md")).unwrap()
+    }
+
+    #[test]
+    fn pass_fixtures_have_no_diagnostics() {
+        for name in [
+            "max_line_length",
+            "trailing_whitespace",
+            "trailing_newline",
+            "consecutive_blank_lines",
+            "hard_tabs",
+        ] {
+            let content = fixture(name, "pass");
+            let diags = check_style(&content, MAX);
+            assert!(diags.is_empty(), "{name} pass fixture produced {diags:?}");
+        }
+    }
+
+    #[test]
+    fn max_line_length_fail_fixture() {
+        let content = fixture("max_line_length", "fail");
+        let diags = check_style(&content, MAX);
+        assert_eq!(diags.len(), 1);
+        assert_eq!(diags[0].rule, Rule::StyleLineLength);
+        assert_eq!(diags[0].line, 1);
+        assert_eq!(
+            diags[0].message,
+            "line exceeds maximum length of 100 characters (105 found)"
+        );
+    }
+
+    #[test]
+    fn trailing_whitespace_fail_fixture() {
+        let content = fixture("trailing_whitespace", "fail");
+        let diags = check_style(&content, MAX);
+        assert_eq!(diags.len(), 1);
+        assert_eq!(diags[0].rule, Rule::StyleTrailingWhitespace);
+        assert_eq!(diags[0].line, 1);
+        assert_eq!(diags[0].message, "line has trailing whitespace");
+    }
+
+    #[test]
+    fn trailing_newline_fail_fixture() {
+        let content = fixture("trailing_newline", "fail");
+        let diags = check_style(&content, MAX);
+        assert_eq!(diags.len(), 1);
+        assert_eq!(diags[0].rule, Rule::StyleTrailingNewline);
+        assert_eq!(diags[0].line, 1);
+        assert_eq!(
+            diags[0].message,
+            "file must end with exactly one trailing newline"
+        );
+    }
+
+    #[test]
+    fn consecutive_blank_lines_fail_fixture() {
+        let content = fixture("consecutive_blank_lines", "fail");
+        let diags = check_style(&content, MAX);
+        assert_eq!(diags.len(), 1);
+        assert_eq!(diags[0].rule, Rule::StyleConsecutiveBlankLines);
+        assert_eq!(diags[0].line, 3);
+        assert_eq!(diags[0].message, "multiple consecutive blank lines");
+    }
+
+    #[test]
+    fn hard_tabs_fail_fixture() {
+        let content = fixture("hard_tabs", "fail");
+        let diags = check_style(&content, MAX);
+        assert_eq!(diags.len(), 1);
+        assert_eq!(diags[0].rule, Rule::StyleHardTab);
+        assert_eq!(diags[0].line, 1);
+        assert_eq!(diags[0].message, "line contains a hard tab character");
+    }
+
+    #[test]
+    fn multibyte_char_counted_not_bytes() {
+        // "é" is 2 bytes, 1 char. 60 chars = 120 bytes but only 60 chars: under 100-char limit.
+        let line = "é".repeat(60);
+        let content = format!("{line}\n");
+        let diags = check_style(&content, MAX);
+        assert!(diags.iter().all(|d| d.rule != Rule::StyleLineLength));
+
+        // 101 chars of "é": char count (101) exceeds the 100-char limit even though
+        // byte length (202) is what a naive byte-length check would also flag anyway;
+        // the point is char count, not bytes, drives the decision.
+        let line = "é".repeat(101);
+        let content = format!("{line}\n");
+        let diags = check_style(&content, MAX);
+        let line_len_diag = diags
+            .iter()
+            .find(|d| d.rule == Rule::StyleLineLength)
+            .expect("expected StyleLineLength diagnostic");
+        assert_eq!(
+            line_len_diag.message,
+            "line exceeds maximum length of 100 characters (101 found)"
+        );
+    }
+
+    #[test]
+    fn crlf_line_triggers_trailing_whitespace() {
+        let content = "first line\r\nsecond line\n";
+        let diags = check_style(content, MAX);
+        assert_eq!(diags.len(), 1);
+        assert_eq!(diags[0].rule, Rule::StyleTrailingWhitespace);
+        assert_eq!(diags[0].line, 1);
+    }
+
+    #[test]
+    fn zero_byte_file_violates_trailing_newline() {
+        let diags = check_style("", MAX);
+        assert_eq!(diags.len(), 1);
+        assert_eq!(diags[0].rule, Rule::StyleTrailingNewline);
+        assert_eq!(diags[0].line, 1);
+    }
+
+    #[test]
+    fn no_trailing_newline_at_all_violates() {
+        let diags = check_style("no newline at end", MAX);
+        assert!(diags.iter().any(|d| d.rule == Rule::StyleTrailingNewline));
+    }
+
+    #[test]
+    fn double_trailing_newline_violates() {
+        let diags = check_style("content\n\n", MAX);
+        assert!(diags.iter().any(|d| d.rule == Rule::StyleTrailingNewline));
+    }
+
+    #[test]
+    fn single_trailing_newline_is_fine() {
+        let diags = check_style("content\n", MAX);
+        assert!(diags.iter().all(|d| d.rule != Rule::StyleTrailingNewline));
+    }
+
+    #[test]
+    fn exactly_two_blank_lines_anchors_on_second() {
+        let content = "a\n\n\nb\n";
+        let diags = check_style(content, MAX);
+        let blank_diags: Vec<_> = diags
+            .iter()
+            .filter(|d| d.rule == Rule::StyleConsecutiveBlankLines)
+            .collect();
+        assert_eq!(blank_diags.len(), 1);
+        assert_eq!(blank_diags[0].line, 3);
+    }
+
+    #[test]
+    fn five_blank_line_run_produces_one_diagnostic() {
+        let content = "a\n\n\n\n\n\nb\n";
+        let diags = check_style(content, MAX);
+        let blank_diags: Vec<_> = diags
+            .iter()
+            .filter(|d| d.rule == Rule::StyleConsecutiveBlankLines)
+            .collect();
+        assert_eq!(blank_diags.len(), 1);
+        assert_eq!(blank_diags[0].line, 3);
+    }
+
+    #[test]
+    fn two_separate_blank_runs_produce_two_diagnostics() {
+        let content = "a\n\n\nb\n\n\nc\n";
+        let diags = check_style(content, MAX);
+        let blank_diags: Vec<_> = diags
+            .iter()
+            .filter(|d| d.rule == Rule::StyleConsecutiveBlankLines)
+            .collect();
+        assert_eq!(blank_diags.len(), 2);
+        assert_eq!(blank_diags[0].line, 3);
+        assert_eq!(blank_diags[1].line, 6);
+    }
+
+    #[test]
+    fn tab_mid_line_and_trailing_fires_both_rules() {
+        let content = "foo\tbar\t\n";
+        let diags = check_style(content, MAX);
+        assert!(diags.iter().any(|d| d.rule == Rule::StyleHardTab));
+        assert!(diags.iter().any(|d| d.rule == Rule::StyleTrailingWhitespace));
+    }
+
+    #[test]
+    fn overlength_line_with_trailing_whitespace_fires_both_rules() {
+        let content = format!("{} \n", "a".repeat(101));
+        let diags = check_style(&content, MAX);
+        assert!(diags.iter().any(|d| d.rule == Rule::StyleLineLength));
+        assert!(diags.iter().any(|d| d.rule == Rule::StyleTrailingWhitespace));
+    }
+}
diff --git a/tests/fixtures/style/consecutive_blank_lines/fail/fail.md b/tests/fixtures/style/consecutive_blank_lines/fail/fail.md
new file mode 100644
index 0000000..7ae853d
--- /dev/null
+++ b/tests/fixtures/style/consecutive_blank_lines/fail/fail.md
@@ -0,0 +1,4 @@
+Line one.
+
+
+Line two.
diff --git a/tests/fixtures/style/consecutive_blank_lines/pass/pass.md b/tests/fixtures/style/consecutive_blank_lines/pass/pass.md
new file mode 100644
index 0000000..06c235c
--- /dev/null
+++ b/tests/fixtures/style/consecutive_blank_lines/pass/pass.md
@@ -0,0 +1,3 @@
+Line one.
+
+Line two.
diff --git a/tests/fixtures/style/hard_tabs/fail/fail.md b/tests/fixtures/style/hard_tabs/fail/fail.md
new file mode 100644
index 0000000..69ce270
--- /dev/null
+++ b/tests/fixtures/style/hard_tabs/fail/fail.md
@@ -0,0 +1 @@
+Line with	tab in middle.
diff --git a/tests/fixtures/style/hard_tabs/pass/pass.md b/tests/fixtures/style/hard_tabs/pass/pass.md
new file mode 100644
index 0000000..d77dd8a
--- /dev/null
+++ b/tests/fixtures/style/hard_tabs/pass/pass.md
@@ -0,0 +1 @@
+Line without tabs.
diff --git a/tests/fixtures/style/max_line_length/fail/fail.md b/tests/fixtures/style/max_line_length/fail/fail.md
new file mode 100644
index 0000000..42db6df
--- /dev/null
+++ b/tests/fixtures/style/max_line_length/fail/fail.md
@@ -0,0 +1 @@
+aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
diff --git a/tests/fixtures/style/max_line_length/pass/pass.md b/tests/fixtures/style/max_line_length/pass/pass.md
new file mode 100644
index 0000000..d0b5104
--- /dev/null
+++ b/tests/fixtures/style/max_line_length/pass/pass.md
@@ -0,0 +1 @@
+This line is under the limit.
diff --git a/tests/fixtures/style/trailing_newline/fail/fail.md b/tests/fixtures/style/trailing_newline/fail/fail.md
new file mode 100644
index 0000000..4b4a6ea
--- /dev/null
+++ b/tests/fixtures/style/trailing_newline/fail/fail.md
@@ -0,0 +1 @@
+This file has no trailing newline.
\ No newline at end of file
diff --git a/tests/fixtures/style/trailing_newline/pass/pass.md b/tests/fixtures/style/trailing_newline/pass/pass.md
new file mode 100644
index 0000000..f934226
--- /dev/null
+++ b/tests/fixtures/style/trailing_newline/pass/pass.md
@@ -0,0 +1 @@
+This file ends properly.
diff --git a/tests/fixtures/style/trailing_whitespace/fail/fail.md b/tests/fixtures/style/trailing_whitespace/fail/fail.md
new file mode 100644
index 0000000..d0e363e
--- /dev/null
+++ b/tests/fixtures/style/trailing_whitespace/fail/fail.md
@@ -0,0 +1 @@
+This line has trailing whitespace 
diff --git a/tests/fixtures/style/trailing_whitespace/pass/pass.md b/tests/fixtures/style/trailing_whitespace/pass/pass.md
new file mode 100644
index 0000000..6074fdf
--- /dev/null
+++ b/tests/fixtures/style/trailing_whitespace/pass/pass.md
@@ -0,0 +1 @@
+This line has no trailing whitespace.
