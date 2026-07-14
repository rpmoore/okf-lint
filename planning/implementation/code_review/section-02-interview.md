# Code Review Interview: Section 02 - Concept Checks

**Date:** 2026-07-13

## Discussed with User

None — all findings were low-risk, unambiguous fixes with no real tradeoffs.

## Auto-Fixes (no discussion needed)

1. **docs/knowledge update.** Add `docs/knowledge/concept-checks.md` documenting `check_concept` and the two rules it implements, and link it from `docs/knowledge/index.md`, per CLAUDE.md's mandate (same convention established in section-01).
2. **Untested YAML parse-error branch.** Added a test that feeds `check_concept` a frontmatter block with syntactically invalid YAML (unclosed flow mapping) to exercise the `Err(_)` arm.
3. **Simplify key lookup.** Changed `mapping.get(Value::String("type".to_string()))` to `mapping.get("type")`, using `serde_yaml_ng::Mapping::get`'s generic `Index` impl for `str` — avoids an allocation per call.
4. **`cargo fmt`.** Ran `cargo fmt` to fix line-width inconsistency in the test module's fixture constants.
