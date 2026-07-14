diff --git a/src/checks/index_md.rs b/src/checks/index_md.rs
new file mode 100644
index 0000000..6f4d807
--- /dev/null
+++ b/src/checks/index_md.rs
@@ -0,0 +1,266 @@
+use crate::diagnostic::{Diagnostic, Rule};
+use crate::frontmatter::{FrontmatterResult, split_frontmatter};
+use serde_yaml_ng::Value;
+
+/// Runs OKF conformance rules 3 (OkfIndexFrontmatterPlacement) and 4
+/// (OkfIndexBodyStructure) against the content of an index.md file.
+/// `is_root` is true iff this index.md is directly at the bundle root
+/// (no parent path component) — the caller determines this via file
+/// classification (see lint.rs, section-06), not this function.
+pub fn check_index(content: &str, is_root: bool) -> Vec<Diagnostic> {
+    let mut diagnostics = Vec::new();
+
+    match split_frontmatter(content) {
+        FrontmatterResult::None => {
+            scan_body(content, 1, &mut diagnostics);
+        }
+        FrontmatterResult::Found {
+            yaml_text,
+            body_start_line,
+        } => {
+            if is_root {
+                if !root_frontmatter_ok(&yaml_text) {
+                    diagnostics.push(root_frontmatter_diagnostic());
+                }
+            } else {
+                diagnostics.push(nonroot_frontmatter_diagnostic());
+            }
+            scan_body(content, body_start_line, &mut diagnostics);
+        }
+        FrontmatterResult::Unclosed => {
+            // The frontmatter block never closes, so there is no well-defined
+            // body to scan under rule 4 — the whole remainder of the file is
+            // inside the incomplete block.
+            if is_root {
+                diagnostics.push(root_frontmatter_diagnostic());
+            } else {
+                diagnostics.push(nonroot_frontmatter_diagnostic());
+            }
+        }
+    }
+
+    diagnostics
+}
+
+fn root_frontmatter_ok(yaml_text: &str) -> bool {
+    match serde_yaml_ng::from_str::<Value>(yaml_text) {
+        Ok(Value::Mapping(mapping)) => mapping.iter().all(|(k, _)| k.as_str() == Some("okf_version")),
+        Ok(Value::Null) => true,
+        _ => false,
+    }
+}
+
+fn root_frontmatter_diagnostic() -> Diagnostic {
+    Diagnostic {
+        line: 1,
+        rule: Rule::OkfIndexFrontmatterPlacement,
+        message: "root index.md frontmatter may only contain 'okf_version'".to_string(),
+    }
+}
+
+fn nonroot_frontmatter_diagnostic() -> Diagnostic {
+    Diagnostic {
+        line: 1,
+        rule: Rule::OkfIndexFrontmatterPlacement,
+        message: "index.md must not contain frontmatter".to_string(),
+    }
+}
+
+fn scan_body(content: &str, start_line: usize, diagnostics: &mut Vec<Diagnostic>) {
+    let mut in_list_item = false;
+    for (idx, raw_line) in content.split('\n').enumerate() {
+        let line_no = idx + 1;
+        if line_no < start_line {
+            continue;
+        }
+        let line = strip_cr(raw_line);
+
+        if line.trim().is_empty() {
+            in_list_item = false;
+            continue;
+        }
+
+        if is_heading(line) {
+            in_list_item = false;
+        } else if is_list_item(line) {
+            in_list_item = true;
+        } else if in_list_item && leading_space_count(line) >= 2 {
+            // Continuation line: valid, in_list_item stays true.
+        } else {
+            diagnostics.push(Diagnostic {
+                line: line_no,
+                rule: Rule::OkfIndexBodyStructure,
+                message: "index.md body line is not a heading or list item".to_string(),
+            });
+            in_list_item = false;
+        }
+    }
+}
+
+fn is_heading(line: &str) -> bool {
+    let hashes = line.chars().take_while(|&c| c == '#').count();
+    hashes >= 1 && line.as_bytes().get(hashes) == Some(&b' ')
+}
+
+fn is_list_item(line: &str) -> bool {
+    let mut chars = line.chars();
+    match chars.next() {
+        Some('*') | Some('+') | Some('-') => chars.next() == Some(' '),
+        _ => false,
+    }
+}
+
+fn leading_space_count(line: &str) -> usize {
+    line.chars().take_while(|&c| c == ' ').count()
+}
+
+// Trims a trailing '\r' so CRLF-terminated files are treated the same as
+// LF-terminated ones when scanning body lines.
+fn strip_cr(line: &str) -> &str {
+    line.strip_suffix('\r').unwrap_or(line)
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    const PASS_ROOT: &str =
+        include_str!("../../tests/fixtures/okf/index_frontmatter_placement/pass_root/index.md");
+    const FAIL_NONROOT: &str = include_str!(
+        "../../tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/sub/index.md"
+    );
+    const FAIL_ROOT_EXTRA_KEY: &str = include_str!(
+        "../../tests/fixtures/okf/index_frontmatter_placement/fail_root_extra_key/index.md"
+    );
+    const BODY_PASS: &str = include_str!("../../tests/fixtures/okf/index_body_structure/pass/pass.md");
+    const BODY_FAIL: &str = include_str!("../../tests/fixtures/okf/index_body_structure/fail/fail.md");
+
+    #[test]
+    fn pass_root_has_no_frontmatter_placement_diagnostics() {
+        let diagnostics = check_index(PASS_ROOT, true);
+        assert!(
+            !diagnostics
+                .iter()
+                .any(|d| d.rule == Rule::OkfIndexFrontmatterPlacement)
+        );
+    }
+
+    #[test]
+    fn fail_nonroot_emits_frontmatter_placement_diagnostic() {
+        assert_eq!(
+            check_index(FAIL_NONROOT, false),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfIndexFrontmatterPlacement,
+                message: "index.md must not contain frontmatter".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn fail_root_extra_key_emits_frontmatter_placement_diagnostic() {
+        assert_eq!(
+            check_index(FAIL_ROOT_EXTRA_KEY, true),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfIndexFrontmatterPlacement,
+                message: "root index.md frontmatter may only contain 'okf_version'".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn index_body_structure_pass_has_no_diagnostics() {
+        assert_eq!(check_index(BODY_PASS, true), vec![]);
+    }
+
+    #[test]
+    fn index_body_structure_fail_emits_diagnostic_per_line() {
+        assert_eq!(
+            check_index(BODY_FAIL, true),
+            vec![Diagnostic {
+                line: 3,
+                rule: Rule::OkfIndexBodyStructure,
+                message: "index.md body line is not a heading or list item".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn root_unclosed_frontmatter_emits_root_diagnostic() {
+        let content = "---\nokf_version: 1\nno closing delimiter";
+        assert_eq!(
+            check_index(content, true),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfIndexFrontmatterPlacement,
+                message: "root index.md frontmatter may only contain 'okf_version'".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn nonroot_unclosed_frontmatter_emits_nonroot_diagnostic() {
+        let content = "---\ntitle: nested\nno closing delimiter";
+        assert_eq!(
+            check_index(content, false),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfIndexFrontmatterPlacement,
+                message: "index.md must not contain frontmatter".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn heading_list_item_then_continuation_is_valid() {
+        let content = "# Title\n- Item\n  continuation\n";
+        assert_eq!(check_index(content, true), vec![]);
+    }
+
+    #[test]
+    fn indented_line_without_preceding_list_item_is_violation() {
+        let content = "# Title\n  stray indented line\n";
+        assert_eq!(
+            check_index(content, true),
+            vec![Diagnostic {
+                line: 2,
+                rule: Rule::OkfIndexBodyStructure,
+                message: "index.md body line is not a heading or list item".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn blank_line_resets_list_item_state() {
+        let content = "# Title\n- Item\n\n  stray indented line\n";
+        assert_eq!(
+            check_index(content, true),
+            vec![Diagnostic {
+                line: 4,
+                rule: Rule::OkfIndexBodyStructure,
+                message: "index.md body line is not a heading or list item".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn nonroot_with_bad_frontmatter_and_stray_paragraph_emits_both() {
+        let content = "---\ntitle: nested\n---\n# Title\n\nstray paragraph\n";
+        assert_eq!(
+            check_index(content, false),
+            vec![
+                Diagnostic {
+                    line: 1,
+                    rule: Rule::OkfIndexFrontmatterPlacement,
+                    message: "index.md must not contain frontmatter".to_string(),
+                },
+                Diagnostic {
+                    line: 6,
+                    rule: Rule::OkfIndexBodyStructure,
+                    message: "index.md body line is not a heading or list item".to_string(),
+                },
+            ]
+        );
+    }
+}
diff --git a/src/checks/mod.rs b/src/checks/mod.rs
index 0e912d6..7c2cac3 100644
--- a/src/checks/mod.rs
+++ b/src/checks/mod.rs
@@ -1 +1,2 @@
+pub mod index_md;
 pub mod okf;
diff --git a/tests/fixtures/okf/index_body_structure/fail/fail.md b/tests/fixtures/okf/index_body_structure/fail/fail.md
new file mode 100644
index 0000000..dc6ec29
--- /dev/null
+++ b/tests/fixtures/okf/index_body_structure/fail/fail.md
@@ -0,0 +1,5 @@
+# Title
+
+This is a stray paragraph.
+
+- Item one
diff --git a/tests/fixtures/okf/index_body_structure/pass/pass.md b/tests/fixtures/okf/index_body_structure/pass/pass.md
new file mode 100644
index 0000000..8f29039
--- /dev/null
+++ b/tests/fixtures/okf/index_body_structure/pass/pass.md
@@ -0,0 +1,10 @@
+# Title
+
+- Item one
+  continuation line
+- Item two
+
+## Subheading
+
+* Alt marker item
++ Plus marker item
diff --git a/tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/index.md b/tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/index.md
new file mode 100644
index 0000000..656f5b7
--- /dev/null
+++ b/tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/index.md
@@ -0,0 +1,3 @@
+# Root Index
+
+- [Sub](sub/index.md)
diff --git a/tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/sub/index.md b/tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/sub/index.md
new file mode 100644
index 0000000..9f148ba
--- /dev/null
+++ b/tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/sub/index.md
@@ -0,0 +1,6 @@
+---
+title: nested
+---
+# Sub Index
+
+- [Foo](foo.md)
diff --git a/tests/fixtures/okf/index_frontmatter_placement/fail_root_extra_key/index.md b/tests/fixtures/okf/index_frontmatter_placement/fail_root_extra_key/index.md
new file mode 100644
index 0000000..c5bc42e
--- /dev/null
+++ b/tests/fixtures/okf/index_frontmatter_placement/fail_root_extra_key/index.md
@@ -0,0 +1,7 @@
+---
+okf_version: 1
+title: Extra
+---
+# Root Index
+
+- [Foo](foo.md)
diff --git a/tests/fixtures/okf/index_frontmatter_placement/pass_root/index.md b/tests/fixtures/okf/index_frontmatter_placement/pass_root/index.md
new file mode 100644
index 0000000..eb639e5
--- /dev/null
+++ b/tests/fixtures/okf/index_frontmatter_placement/pass_root/index.md
@@ -0,0 +1,7 @@
+---
+okf_version: 1
+---
+# Root Index
+
+- [Foo](foo.md)
+- [Bar](bar.md)
