diff --git a/.gitignore b/.gitignore
new file mode 100644
index 0000000..ea8c4bf
--- /dev/null
+++ b/.gitignore
@@ -0,0 +1 @@
+/target
diff --git a/Cargo.toml b/Cargo.toml
index 04fdf9f..ef5d54a 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -4,3 +4,12 @@ version = "0.1.0"
 edition = "2024"
 
 [dependencies]
+clap = { version = "4", features = ["derive"] }
+walkdir = "2"
+serde_yaml_ng = "0.10"
+chrono = "0.4"
+
+[dev-dependencies]
+assert_cmd = "2"
+predicates = "3"
+insta = "1"
diff --git a/src/diagnostic.rs b/src/diagnostic.rs
new file mode 100644
index 0000000..1ff5f86
--- /dev/null
+++ b/src/diagnostic.rs
@@ -0,0 +1,55 @@
+#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
+pub enum Rule {
+    // OKF conformance, in this fixed order:
+    OkfMissingFrontmatter,
+    OkfMissingType,
+    OkfIndexFrontmatterPlacement,
+    OkfIndexBodyStructure,
+    OkfLogDateHeading,
+    // Markdown style, in this fixed order:
+    StyleLineLength,
+    StyleTrailingWhitespace,
+    StyleTrailingNewline,
+    StyleConsecutiveBlankLines,
+    StyleHardTab,
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
+pub struct Diagnostic {
+    pub line: usize,
+    pub rule: Rule,
+    pub message: String,
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn same_line_sorts_okf_before_style() {
+        let okf = Diagnostic {
+            line: 5,
+            rule: Rule::OkfMissingType,
+            message: "a".to_string(),
+        };
+        let style = Diagnostic {
+            line: 5,
+            rule: Rule::StyleHardTab,
+            message: "b".to_string(),
+        };
+        assert!(okf.rule < style.rule);
+    }
+
+    #[test]
+    fn rule_declaration_order_is_fixed() {
+        assert!(Rule::OkfMissingFrontmatter < Rule::OkfMissingType);
+        assert!(Rule::OkfMissingType < Rule::OkfIndexFrontmatterPlacement);
+        assert!(Rule::OkfIndexFrontmatterPlacement < Rule::OkfIndexBodyStructure);
+        assert!(Rule::OkfIndexBodyStructure < Rule::OkfLogDateHeading);
+        assert!(Rule::OkfLogDateHeading < Rule::StyleLineLength);
+        assert!(Rule::StyleLineLength < Rule::StyleTrailingWhitespace);
+        assert!(Rule::StyleTrailingWhitespace < Rule::StyleTrailingNewline);
+        assert!(Rule::StyleTrailingNewline < Rule::StyleConsecutiveBlankLines);
+        assert!(Rule::StyleConsecutiveBlankLines < Rule::StyleHardTab);
+    }
+}
diff --git a/src/frontmatter.rs b/src/frontmatter.rs
new file mode 100644
index 0000000..cf6441a
--- /dev/null
+++ b/src/frontmatter.rs
@@ -0,0 +1,81 @@
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum FrontmatterResult {
+    None,
+    Unclosed,
+    Found {
+        yaml_text: String,
+        body_start_line: usize,
+    },
+}
+
+pub fn split_frontmatter(content: &str) -> FrontmatterResult {
+    let mut lines = content.split('\n');
+    match lines.next() {
+        Some("---") => {}
+        _ => return FrontmatterResult::None,
+    }
+
+    let mut yaml_lines = Vec::new();
+    let mut consumed = 1; // the opening "---" line
+    for line in lines {
+        consumed += 1;
+        if line == "---" {
+            return FrontmatterResult::Found {
+                yaml_text: yaml_lines.join("\n"),
+                body_start_line: consumed + 1,
+            };
+        }
+        yaml_lines.push(line);
+    }
+
+    FrontmatterResult::Unclosed
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn no_leading_delimiter_is_none() {
+        assert_eq!(split_frontmatter("# Title\n\nbody"), FrontmatterResult::None);
+    }
+
+    #[test]
+    fn unclosed_block_is_unclosed() {
+        assert_eq!(
+            split_frontmatter("---\ntype: concept\nbody without closing"),
+            FrontmatterResult::Unclosed
+        );
+    }
+
+    #[test]
+    fn well_formed_block_is_found() {
+        let content = "---\ntype: concept\n---\n# Body\n";
+        match split_frontmatter(content) {
+            FrontmatterResult::Found {
+                yaml_text,
+                body_start_line,
+            } => {
+                assert_eq!(yaml_text, "type: concept");
+                assert_eq!(body_start_line, 4);
+            }
+            other => panic!("expected Found, got {other:?}"),
+        }
+    }
+
+    #[test]
+    fn leading_blank_line_before_delimiter_is_none() {
+        assert_eq!(
+            split_frontmatter("\n---\ntype: concept\n---\nbody"),
+            FrontmatterResult::None
+        );
+    }
+
+    #[test]
+    fn delimiter_with_trailing_characters_is_none() {
+        assert_eq!(
+            split_frontmatter("--- \ntype: concept\n---\nbody"),
+            FrontmatterResult::None
+        );
+    }
+}
diff --git a/src/lint.rs b/src/lint.rs
new file mode 100644
index 0000000..e188c91
--- /dev/null
+++ b/src/lint.rs
@@ -0,0 +1,13 @@
+use std::path::PathBuf;
+
+#[derive(Debug)]
+#[allow(dead_code)] // PathNotFound/NotADirectory/InvalidUtf8 unused until section-06's lint_bundle
+pub enum LintError {
+    PathNotFound(PathBuf),
+    NotADirectory(PathBuf),
+    Io {
+        path: PathBuf,
+        source: std::io::Error,
+    },
+    InvalidUtf8(PathBuf),
+}
diff --git a/src/main.rs b/src/main.rs
index e7a11a9..e2c26ca 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,8 @@
+mod diagnostic;
+mod frontmatter;
+mod lint;
+mod walk;
+
 fn main() {
     println!("Hello, world!");
 }
diff --git a/src/walk.rs b/src/walk.rs
new file mode 100644
index 0000000..f34f1a0
--- /dev/null
+++ b/src/walk.rs
@@ -0,0 +1,138 @@
+use crate::lint::LintError;
+use std::path::{Path, PathBuf};
+use walkdir::WalkDir;
+
+pub fn collect_md_files(root: &Path) -> Result<Vec<PathBuf>, LintError> {
+    let mut files = Vec::new();
+
+    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
+        entry
+            .file_name()
+            .to_str()
+            .map(|name| !name.starts_with('.'))
+            .unwrap_or(true)
+    });
+
+    for entry in walker {
+        let entry = entry.map_err(|err| {
+            let path = err.path().unwrap_or(root).to_path_buf();
+            let source = err
+                .into_io_error()
+                .unwrap_or_else(|| std::io::Error::other("directory walk failed"));
+            LintError::Io { path, source }
+        })?;
+
+        if !entry.file_type().is_file() {
+            continue;
+        }
+        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("md") {
+            continue;
+        }
+
+        let relative = entry
+            .path()
+            .strip_prefix(root)
+            .unwrap_or(entry.path())
+            .to_path_buf();
+        files.push(relative);
+    }
+
+    files.sort();
+    Ok(files)
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use std::fs;
+
+    fn temp_dir(name: &str) -> PathBuf {
+        let dir = std::env::temp_dir().join(format!(
+            "okf-lint-walk-test-{name}-{}",
+            std::process::id()
+        ));
+        let _ = fs::remove_dir_all(&dir);
+        fs::create_dir_all(&dir).unwrap();
+        dir
+    }
+
+    #[test]
+    fn only_md_files_returned() {
+        let root = temp_dir("mix");
+        fs::write(root.join("a.md"), "").unwrap();
+        fs::write(root.join("b.txt"), "").unwrap();
+
+        let files = collect_md_files(&root).unwrap();
+
+        assert_eq!(files, vec![PathBuf::from("a.md")]);
+        fs::remove_dir_all(&root).unwrap();
+    }
+
+    #[test]
+    fn dot_directories_are_excluded() {
+        let root = temp_dir("dotdir");
+        fs::create_dir_all(root.join(".git")).unwrap();
+        fs::write(root.join(".git/inside.md"), "").unwrap();
+        fs::write(root.join(".hidden.md"), "").unwrap();
+        fs::write(root.join("visible.md"), "").unwrap();
+
+        let files = collect_md_files(&root).unwrap();
+
+        assert_eq!(files, vec![PathBuf::from("visible.md")]);
+        fs::remove_dir_all(&root).unwrap();
+    }
+
+    #[test]
+    fn paths_are_relative_to_root() {
+        let root = temp_dir("relative");
+        fs::create_dir_all(root.join("sub")).unwrap();
+        fs::write(root.join("sub/nested.md"), "").unwrap();
+
+        let files = collect_md_files(&root).unwrap();
+
+        assert_eq!(files, vec![PathBuf::from("sub/nested.md")]);
+        assert!(!files[0].is_absolute());
+        fs::remove_dir_all(&root).unwrap();
+    }
+
+    #[test]
+    fn results_are_sorted_lexicographically() {
+        let root = temp_dir("sorted");
+        fs::write(root.join("z.md"), "").unwrap();
+        fs::write(root.join("a.md"), "").unwrap();
+        fs::write(root.join("m.md"), "").unwrap();
+
+        let files = collect_md_files(&root).unwrap();
+
+        assert_eq!(
+            files,
+            vec![
+                PathBuf::from("a.md"),
+                PathBuf::from("m.md"),
+                PathBuf::from("z.md"),
+            ]
+        );
+        fs::remove_dir_all(&root).unwrap();
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn permission_denied_subdirectory_is_io_error() {
+        use std::os::unix::fs::PermissionsExt;
+
+        let root = temp_dir("perm");
+        let blocked = root.join("blocked");
+        fs::create_dir_all(&blocked).unwrap();
+        fs::write(blocked.join("secret.md"), "").unwrap();
+        fs::write(root.join("visible.md"), "").unwrap();
+
+        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
+
+        let result = collect_md_files(&root);
+
+        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755)).unwrap();
+        fs::remove_dir_all(&root).unwrap();
+
+        assert!(matches!(result, Err(LintError::Io { .. })));
+    }
+}
