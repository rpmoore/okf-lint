diff --git a/src/cli.rs b/src/cli.rs
new file mode 100644
index 0000000..897fa6b
--- /dev/null
+++ b/src/cli.rs
@@ -0,0 +1,6 @@
+#[derive(clap::Parser)]
+pub struct Cli {
+    pub path: std::path::PathBuf,
+    #[arg(long, default_value_t = 100)]
+    pub max_line_length: u32,
+}
diff --git a/src/main.rs b/src/main.rs
index 658a8cc..9d0056a 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,9 +1,48 @@
 mod checks;
+mod cli;
 mod diagnostic;
 mod frontmatter;
 mod lint;
 mod walk;
 
-fn main() {
-    println!("Hello, world!");
+use clap::Parser;
+use cli::Cli;
+use lint::LintError;
+use std::process::ExitCode;
+
+fn main() -> ExitCode {
+    let cli = Cli::parse();
+
+    let diagnostics = match lint::lint_bundle(&cli.path, cli.max_line_length as usize) {
+        Ok(diagnostics) => diagnostics,
+        Err(err) => {
+            eprintln!("error: {}", format_error(&err));
+            return ExitCode::from(2);
+        }
+    };
+
+    if diagnostics.is_empty() {
+        return ExitCode::from(0);
+    }
+
+    for (path, diagnostic) in &diagnostics {
+        println!(
+            "{}:{}: {}",
+            path.display(),
+            diagnostic.line,
+            diagnostic.message
+        );
+    }
+    ExitCode::from(1)
+}
+
+fn format_error(err: &LintError) -> String {
+    match err {
+        LintError::PathNotFound(path) => format!("path does not exist: {}", path.display()),
+        LintError::NotADirectory(path) => format!("not a directory: {}", path.display()),
+        LintError::Io { path, source } => {
+            format!("failed to read {}: {}", path.display(), source)
+        }
+        LintError::InvalidUtf8(path) => format!("file is not valid UTF-8: {}", path.display()),
+    }
 }
diff --git a/tests/cli_tests.rs b/tests/cli_tests.rs
new file mode 100644
index 0000000..698d7e7
--- /dev/null
+++ b/tests/cli_tests.rs
@@ -0,0 +1,77 @@
+use assert_cmd::Command;
+use predicates::prelude::*;
+
+#[test]
+fn nonexistent_path_exits_2_with_stderr_and_empty_stdout() {
+    Command::cargo_bin("okf-lint")
+        .unwrap()
+        .arg("tests/fixtures/does_not_exist_okf_lint")
+        .assert()
+        .code(2)
+        .stderr(predicate::str::is_empty().not())
+        .stdout(predicate::str::is_empty());
+}
+
+#[test]
+fn clean_bundle_exits_0_with_empty_stdout() {
+    Command::cargo_bin("okf-lint")
+        .unwrap()
+        .arg("tests/fixtures/okf/missing_frontmatter/pass")
+        .assert()
+        .code(0)
+        .stdout(predicate::str::is_empty());
+}
+
+#[test]
+fn bundle_with_violation_exits_1_with_diagnostic() {
+    Command::cargo_bin("okf-lint")
+        .unwrap()
+        .arg("tests/fixtures/okf/missing_frontmatter/fail")
+        .assert()
+        .code(1)
+        .stdout(predicate::str::contains(
+            "missing or invalid YAML frontmatter",
+        ));
+}
+
+#[test]
+fn max_line_length_override_suppresses_violation() {
+    Command::cargo_bin("okf-lint")
+        .unwrap()
+        .args([
+            "tests/fixtures/cli/max_line_length_override",
+            "--max-line-length",
+            "150",
+        ])
+        .assert()
+        .code(0)
+        .stdout(predicate::str::is_empty());
+}
+
+#[test]
+fn default_max_line_length_matches_explicit_100() {
+    let default_output = Command::cargo_bin("okf-lint")
+        .unwrap()
+        .arg("tests/fixtures/cli/max_line_length_override")
+        .output()
+        .unwrap();
+
+    let explicit_output = Command::cargo_bin("okf-lint")
+        .unwrap()
+        .args([
+            "tests/fixtures/cli/max_line_length_override",
+            "--max-line-length",
+            "100",
+        ])
+        .output()
+        .unwrap();
+
+    assert_eq!(default_output.status.code(), Some(1));
+    assert_eq!(default_output.stdout, explicit_output.stdout);
+    assert_eq!(default_output.status.code(), explicit_output.status.code());
+    assert!(
+        String::from_utf8_lossy(&default_output.stdout).contains(
+            "line exceeds maximum length of 100 characters (120 found)"
+        )
+    );
+}
diff --git a/tests/fixtures/cli/max_line_length_override/fail.md b/tests/fixtures/cli/max_line_length_override/fail.md
new file mode 100644
index 0000000..7293579
--- /dev/null
+++ b/tests/fixtures/cli/max_line_length_override/fail.md
@@ -0,0 +1,7 @@
+---
+type: concept
+---
+
+# Title
+
+aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
