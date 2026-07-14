diff --git a/tests/cli_tests.rs b/tests/cli_tests.rs
index c9df15a..2e1e009 100644
--- a/tests/cli_tests.rs
+++ b/tests/cli_tests.rs
@@ -80,3 +80,25 @@ fn default_max_line_length_matches_explicit_100() {
         )
     );
 }
+
+#[test]
+fn integration_bundle_whole_output_matches_snapshot() {
+    let output = Command::cargo_bin("okf-lint")
+        .unwrap()
+        .arg("tests/fixtures/integration_bundle")
+        .output()
+        .unwrap();
+
+    let stdout = String::from_utf8(output.stdout).unwrap();
+    insta::assert_snapshot!(stdout);
+}
+
+#[test]
+fn integration_bundle_exits_1() {
+    Command::cargo_bin("okf-lint")
+        .unwrap()
+        .arg("tests/fixtures/integration_bundle")
+        .assert()
+        .code(1)
+        .stderr(predicate::str::is_empty());
+}
diff --git a/tests/fixtures/integration_bundle/concept-a.md b/tests/fixtures/integration_bundle/concept-a.md
new file mode 100644
index 0000000..86ed94a
--- /dev/null
+++ b/tests/fixtures/integration_bundle/concept-a.md
@@ -0,0 +1,7 @@
+---
+type: concept
+---
+
+# Concept A
+
+Some text with trailing space. 
diff --git a/tests/fixtures/integration_bundle/concept-b.md b/tests/fixtures/integration_bundle/concept-b.md
new file mode 100644
index 0000000..0e14cb3
--- /dev/null
+++ b/tests/fixtures/integration_bundle/concept-b.md
@@ -0,0 +1,3 @@
+# Concept B
+
+Some content, no frontmatter.
diff --git a/tests/fixtures/integration_bundle/index.md b/tests/fixtures/integration_bundle/index.md
new file mode 100644
index 0000000..5668c1b
--- /dev/null
+++ b/tests/fixtures/integration_bundle/index.md
@@ -0,0 +1,5 @@
+# Root
+
+- [Concept A](concept-a.md)
+- [Concept B](concept-b.md)
+- [Sub](sub/index.md)
diff --git a/tests/fixtures/integration_bundle/log.md b/tests/fixtures/integration_bundle/log.md
new file mode 100644
index 0000000..a507631
--- /dev/null
+++ b/tests/fixtures/integration_bundle/log.md
@@ -0,0 +1,9 @@
+# Log
+
+## 2026-01-01
+
+Initial entry.
+
+## 2026-02-30
+
+Bad date.
diff --git a/tests/fixtures/integration_bundle/sub/concept-b.md b/tests/fixtures/integration_bundle/sub/concept-b.md
new file mode 100644
index 0000000..d669118
--- /dev/null
+++ b/tests/fixtures/integration_bundle/sub/concept-b.md
@@ -0,0 +1,7 @@
+---
+type: concept
+---
+
+# Sub Concept B
+
+Clean content with no violations.
diff --git a/tests/fixtures/integration_bundle/sub/index.md b/tests/fixtures/integration_bundle/sub/index.md
new file mode 100644
index 0000000..1653a76
--- /dev/null
+++ b/tests/fixtures/integration_bundle/sub/index.md
@@ -0,0 +1,5 @@
+# Sub Index
+
+- [Concept B](concept-b.md)
+
+This is a stray paragraph line.
diff --git a/tests/snapshots/cli_tests__integration_bundle_whole_output_matches_snapshot.snap b/tests/snapshots/cli_tests__integration_bundle_whole_output_matches_snapshot.snap
new file mode 100644
index 0000000..31bf730
--- /dev/null
+++ b/tests/snapshots/cli_tests__integration_bundle_whole_output_matches_snapshot.snap
@@ -0,0 +1,8 @@
+---
+source: tests/cli_tests.rs
+expression: stdout
+---
+concept-a.md:7: line has trailing whitespace
+concept-b.md:1: missing or invalid YAML frontmatter
+log.md:7: log.md heading is not a valid YYYY-MM-DD date
+sub/index.md:5: index.md body line is not a heading or list item
