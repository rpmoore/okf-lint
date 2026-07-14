diff --git a/src/lint.rs b/src/lint.rs
index c9a207b..b3a62ef 100644
--- a/src/lint.rs
+++ b/src/lint.rs
@@ -1,8 +1,11 @@
-use std::path::PathBuf;
+use crate::checks::index_md::check_index;
+use crate::checks::log_md::check_log;
+use crate::checks::okf::check_concept;
+use crate::checks::style::check_style;
+use crate::diagnostic::Diagnostic;
+use crate::walk::collect_md_files;
+use std::path::{Path, PathBuf};
 
-// PathNotFound/NotADirectory/InvalidUtf8 are unused until section-06's
-// lint_bundle wires them up — expect (and don't suppress) dead-code warnings
-// on those variants until then.
 #[derive(Debug)]
 pub enum LintError {
     PathNotFound(PathBuf),
@@ -13,3 +16,213 @@ pub enum LintError {
     },
     InvalidUtf8(PathBuf),
 }
+
+#[derive(Debug, PartialEq, Eq)]
+enum FileKind {
+    Concept,
+    Index { is_root: bool },
+    Log,
+}
+
+fn classify(relative_path: &Path) -> FileKind {
+    match relative_path.file_name().and_then(|name| name.to_str()) {
+        Some("index.md") => FileKind::Index {
+            is_root: relative_path.parent().is_none_or(|p| p.as_os_str().is_empty()),
+        },
+        Some("log.md") => FileKind::Log,
+        _ => FileKind::Concept,
+    }
+}
+
+fn sort_diagnostics(results: &mut [(PathBuf, Diagnostic)]) {
+    results.sort_by(|(path_a, diag_a), (path_b, diag_b)| {
+        path_a
+            .cmp(path_b)
+            .then_with(|| diag_a.line.cmp(&diag_b.line))
+            .then_with(|| diag_a.rule.cmp(&diag_b.rule))
+    });
+}
+
+pub fn lint_bundle(
+    root: &Path,
+    max_line_length: usize,
+) -> Result<Vec<(PathBuf, Diagnostic)>, LintError> {
+    if !root.exists() {
+        return Err(LintError::PathNotFound(root.to_path_buf()));
+    }
+    if !root.is_dir() {
+        return Err(LintError::NotADirectory(root.to_path_buf()));
+    }
+
+    let relative_paths = collect_md_files(root)?;
+
+    let mut results = Vec::new();
+    for relative in &relative_paths {
+        let full_path = root.join(relative);
+        let bytes = std::fs::read(&full_path).map_err(|source| LintError::Io {
+            path: full_path.clone(),
+            source,
+        })?;
+        let content =
+            String::from_utf8(bytes).map_err(|_| LintError::InvalidUtf8(full_path.clone()))?;
+
+        let mut diagnostics = check_style(&content, max_line_length);
+        diagnostics.extend(match classify(relative) {
+            FileKind::Concept => check_concept(&content),
+            FileKind::Index { is_root } => check_index(&content, is_root),
+            FileKind::Log => check_log(&content),
+        });
+
+        for diagnostic in diagnostics {
+            results.push((relative.clone(), diagnostic));
+        }
+    }
+
+    sort_diagnostics(&mut results);
+    Ok(results)
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::diagnostic::Rule;
+    use std::fs;
+    use tempfile::TempDir;
+
+    #[test]
+    fn classify_root_index() {
+        assert_eq!(
+            classify(Path::new("index.md")),
+            FileKind::Index { is_root: true }
+        );
+    }
+
+    #[test]
+    fn classify_nested_index() {
+        assert_eq!(
+            classify(Path::new("sub/index.md")),
+            FileKind::Index { is_root: false }
+        );
+    }
+
+    #[test]
+    fn classify_log_any_depth() {
+        assert_eq!(classify(Path::new("log.md")), FileKind::Log);
+        assert_eq!(classify(Path::new("sub/log.md")), FileKind::Log);
+    }
+
+    #[test]
+    fn classify_substring_matches_are_concept_not_index_or_log() {
+        assert_eq!(classify(Path::new("reindex.md")), FileKind::Concept);
+        assert_eq!(classify(Path::new("catalog.md")), FileKind::Concept);
+    }
+
+    #[test]
+    fn lint_bundle_root_not_found() {
+        let missing = Path::new("/nonexistent/does/not/exist/okf-lint-test");
+        let result = lint_bundle(missing, 100);
+        assert!(matches!(result, Err(LintError::PathNotFound(_))));
+    }
+
+    #[test]
+    fn lint_bundle_root_is_a_file() {
+        let dir = TempDir::new().unwrap();
+        let file_path = dir.path().join("not_a_dir.md");
+        fs::write(&file_path, "hello").unwrap();
+
+        let result = lint_bundle(&file_path, 100);
+        assert!(matches!(result, Err(LintError::NotADirectory(_))));
+    }
+
+    #[test]
+    fn lint_bundle_non_utf8_file_aborts_whole_run() {
+        let dir = TempDir::new().unwrap();
+        fs::write(dir.path().join("valid.md"), "# hello\n").unwrap();
+        // 0x80 alone is not valid UTF-8.
+        fs::write(dir.path().join("bad.md"), [0x80, 0x81]).unwrap();
+
+        let result = lint_bundle(dir.path(), 100);
+        assert!(matches!(result, Err(LintError::InvalidUtf8(_))));
+    }
+
+    #[test]
+    fn lint_bundle_runs_style_and_structural_checks_on_every_file() {
+        let dir = TempDir::new().unwrap();
+        // Bad date heading (structural, OkfLogDateHeading) AND a hard tab (style).
+        fs::write(dir.path().join("log.md"), "## not-a-date\n\tindented\n").unwrap();
+
+        let results = lint_bundle(dir.path(), 100).unwrap();
+
+        assert!(
+            results
+                .iter()
+                .any(|(_, d)| d.rule == Rule::OkfLogDateHeading)
+        );
+        assert!(results.iter().any(|(_, d)| d.rule == Rule::StyleHardTab));
+    }
+
+    #[test]
+    fn lint_bundle_results_are_sorted_across_files() {
+        let dir = TempDir::new().unwrap();
+        fs::write(dir.path().join("z.md"), "no frontmatter\n").unwrap();
+        fs::write(dir.path().join("a.md"), "no frontmatter\n").unwrap();
+
+        let results = lint_bundle(dir.path(), 100).unwrap();
+
+        assert!(results.len() >= 2);
+        assert_eq!(results[0].0, PathBuf::from("a.md"));
+        assert_eq!(results[results.len() - 1].0, PathBuf::from("z.md"));
+    }
+
+    #[test]
+    fn sort_diagnostics_orders_by_path_then_line_then_rule() {
+        let mut results = vec![
+            (
+                PathBuf::from("b.md"),
+                Diagnostic {
+                    line: 1,
+                    rule: Rule::StyleHardTab,
+                    message: "x".to_string(),
+                },
+            ),
+            (
+                PathBuf::from("a.md"),
+                Diagnostic {
+                    line: 5,
+                    rule: Rule::StyleHardTab,
+                    message: "x".to_string(),
+                },
+            ),
+            (
+                PathBuf::from("a.md"),
+                Diagnostic {
+                    line: 2,
+                    rule: Rule::StyleHardTab,
+                    message: "x".to_string(),
+                },
+            ),
+            (
+                PathBuf::from("a.md"),
+                Diagnostic {
+                    line: 2,
+                    rule: Rule::OkfMissingType,
+                    message: "y".to_string(),
+                },
+            ),
+        ];
+
+        sort_diagnostics(&mut results);
+
+        let expected_order: Vec<(PathBuf, usize, Rule)> = vec![
+            (PathBuf::from("a.md"), 2, Rule::OkfMissingType),
+            (PathBuf::from("a.md"), 2, Rule::StyleHardTab),
+            (PathBuf::from("a.md"), 5, Rule::StyleHardTab),
+            (PathBuf::from("b.md"), 1, Rule::StyleHardTab),
+        ];
+        let actual_order: Vec<(PathBuf, usize, Rule)> = results
+            .iter()
+            .map(|(path, diag)| (path.clone(), diag.line, diag.rule))
+            .collect();
+        assert_eq!(actual_order, expected_order);
+    }
+}
