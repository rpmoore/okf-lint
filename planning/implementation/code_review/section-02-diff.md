diff --git a/src/checks/mod.rs b/src/checks/mod.rs
new file mode 100644
index 0000000..0e912d6
--- /dev/null
+++ b/src/checks/mod.rs
@@ -0,0 +1 @@
+pub mod okf;
diff --git a/src/checks/okf.rs b/src/checks/okf.rs
new file mode 100644
index 0000000..ee4e099
--- /dev/null
+++ b/src/checks/okf.rs
@@ -0,0 +1,161 @@
+use crate::diagnostic::{Diagnostic, Rule};
+use crate::frontmatter::{FrontmatterResult, split_frontmatter};
+use serde_yaml_ng::Value;
+
+pub fn check_concept(content: &str) -> Vec<Diagnostic> {
+    let yaml_text = match split_frontmatter(content) {
+        FrontmatterResult::None | FrontmatterResult::Unclosed => {
+            return vec![missing_frontmatter_diagnostic()];
+        }
+        FrontmatterResult::Found { yaml_text, .. } => yaml_text,
+    };
+
+    let parsed: Value = match serde_yaml_ng::from_str(&yaml_text) {
+        Ok(value) => value,
+        Err(_) => return vec![missing_frontmatter_diagnostic()],
+    };
+
+    let Value::Mapping(mapping) = parsed else {
+        return vec![missing_frontmatter_diagnostic()];
+    };
+
+    let has_non_empty_type = mapping
+        .get(Value::String("type".to_string()))
+        .and_then(Value::as_str)
+        .is_some_and(|s| !s.is_empty());
+
+    if has_non_empty_type {
+        Vec::new()
+    } else {
+        vec![Diagnostic {
+            line: 1,
+            rule: Rule::OkfMissingType,
+            message: "frontmatter missing required non-empty 'type' field".to_string(),
+        }]
+    }
+}
+
+fn missing_frontmatter_diagnostic() -> Diagnostic {
+    Diagnostic {
+        line: 1,
+        rule: Rule::OkfMissingFrontmatter,
+        message: "missing or invalid YAML frontmatter".to_string(),
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    const MISSING_FRONTMATTER_PASS: &str =
+        include_str!("../../tests/fixtures/okf/missing_frontmatter/pass/pass.md");
+    const MISSING_FRONTMATTER_FAIL: &str =
+        include_str!("../../tests/fixtures/okf/missing_frontmatter/fail/fail.md");
+    const MISSING_TYPE_PASS: &str = include_str!("../../tests/fixtures/okf/missing_type/pass/pass.md");
+    const MISSING_TYPE_FAIL: &str = include_str!("../../tests/fixtures/okf/missing_type/fail/fail.md");
+
+    #[test]
+    fn missing_frontmatter_pass_fixture_has_no_diagnostics() {
+        assert_eq!(check_concept(MISSING_FRONTMATTER_PASS), vec![]);
+    }
+
+    #[test]
+    fn missing_frontmatter_fail_fixture_emits_rule_1() {
+        assert_eq!(
+            check_concept(MISSING_FRONTMATTER_FAIL),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfMissingFrontmatter,
+                message: "missing or invalid YAML frontmatter".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn missing_type_pass_fixture_has_no_diagnostics() {
+        assert_eq!(check_concept(MISSING_TYPE_PASS), vec![]);
+    }
+
+    #[test]
+    fn missing_type_fail_fixture_emits_rule_2() {
+        assert_eq!(
+            check_concept(MISSING_TYPE_FAIL),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfMissingType,
+                message: "frontmatter missing required non-empty 'type' field".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn unclosed_frontmatter_only_fires_rule_1() {
+        let content = "---\ntype: concept\nno closing delimiter";
+        assert_eq!(
+            check_concept(content),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfMissingFrontmatter,
+                message: "missing or invalid YAML frontmatter".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn non_mapping_frontmatter_fires_rule_1() {
+        let content = "---\njust a string\n---\nbody";
+        assert_eq!(
+            check_concept(content),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfMissingFrontmatter,
+                message: "missing or invalid YAML frontmatter".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn non_string_type_value_fires_rule_2() {
+        let content = "---\ntype: 5\n---\nbody";
+        assert_eq!(
+            check_concept(content),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfMissingType,
+                message: "frontmatter missing required non-empty 'type' field".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn list_type_value_fires_rule_2() {
+        let content = "---\ntype: [a, b]\n---\nbody";
+        assert_eq!(
+            check_concept(content),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfMissingType,
+                message: "frontmatter missing required non-empty 'type' field".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn empty_string_type_value_fires_rule_2() {
+        let content = "---\ntype: \"\"\n---\nbody";
+        assert_eq!(
+            check_concept(content),
+            vec![Diagnostic {
+                line: 1,
+                rule: Rule::OkfMissingType,
+                message: "frontmatter missing required non-empty 'type' field".to_string(),
+            }]
+        );
+    }
+
+    #[test]
+    fn non_empty_type_value_has_no_diagnostics() {
+        let content = "---\ntype: concept\n---\nbody";
+        assert_eq!(check_concept(content), vec![]);
+    }
+}
diff --git a/src/main.rs b/src/main.rs
index e2c26ca..658a8cc 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
+mod checks;
 mod diagnostic;
 mod frontmatter;
 mod lint;
diff --git a/tests/fixtures/okf/missing_frontmatter/fail/fail.md b/tests/fixtures/okf/missing_frontmatter/fail/fail.md
new file mode 100644
index 0000000..95e150f
--- /dev/null
+++ b/tests/fixtures/okf/missing_frontmatter/fail/fail.md
@@ -0,0 +1,3 @@
+# Orders
+
+Some content with no frontmatter at all.
diff --git a/tests/fixtures/okf/missing_frontmatter/pass/pass.md b/tests/fixtures/okf/missing_frontmatter/pass/pass.md
new file mode 100644
index 0000000..466aad0
--- /dev/null
+++ b/tests/fixtures/okf/missing_frontmatter/pass/pass.md
@@ -0,0 +1,7 @@
+---
+type: concept
+---
+
+# Orders
+
+A concept document with well-formed frontmatter.
diff --git a/tests/fixtures/okf/missing_type/fail/fail.md b/tests/fixtures/okf/missing_type/fail/fail.md
new file mode 100644
index 0000000..746abca
--- /dev/null
+++ b/tests/fixtures/okf/missing_type/fail/fail.md
@@ -0,0 +1,7 @@
+---
+title: Orders
+---
+
+# Schema
+
+Frontmatter is well-formed but missing the required type field.
diff --git a/tests/fixtures/okf/missing_type/pass/pass.md b/tests/fixtures/okf/missing_type/pass/pass.md
new file mode 100644
index 0000000..fa10553
--- /dev/null
+++ b/tests/fixtures/okf/missing_type/pass/pass.md
@@ -0,0 +1,7 @@
+---
+type: concept
+---
+
+# Orders
+
+A concept document with a non-empty type field.
