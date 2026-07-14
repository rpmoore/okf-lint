# Code Review: Section 02 - Concept Checks

Overall the rule logic in src/checks/okf.rs correctly implements the plan's two-rule cascade (OkfMissingFrontmatter for None/Unclosed/parse-error/non-mapping, OkfMissingType for missing/empty/non-string `type`), diagnostics are correctly anchored at line 1, and the fixture layout (tests/fixtures/okf/{missing_frontmatter,missing_type}/{pass,fail}/*.md) matches the plan's pass/fail sibling-directory requirement. Tests assert full Diagnostic contents via `vec![]`/`vec![Diagnostic{..}]` equality rather than just counts, per the plan's philosophy. That said, several things a reviewer should push back on:

1. **Missing knowledge-doc update (direct CLAUDE.md violation).** CLAUDE.md mandates updating `docs/knowledge/` for the section of code touched, creating a doc if none exists. Section-01 established this convention. This diff adds a whole new module (`src/checks/mod.rs`, `src/checks/okf.rs`, `check_concept`, two Rule variants' behavior) with zero corresponding docs/knowledge entry and no update to `docs/knowledge/index.md`'s module list.

2. **Untested code path: the actual YAML parse-error branch.** `check_concept` has three ways to hit `OkfMissingFrontmatter`: (a) None/Unclosed, (b) a genuine YAML syntax error, (c) a successfully-parsed non-mapping value. Tests cover (a) and (c) but no test ever constructs YAML that fails to parse (the `Err(_) => ...` arm at line 28).

3. **Unidiomatic/wasteful key lookup.** `mapping.get(Value::String("type".to_string()))` allocates a String + wraps it in a Value on every call. `serde_yaml_ng`'s `Mapping::get` is generic over `Index`, implemented for `str` directly — `mapping.get("type")` is simpler and allocation-free.

4. **Formatting inconsistency suggesting `cargo fmt` wasn't run.** `MISSING_TYPE_PASS`/`FAIL` const lines are ~105 chars wide, exceeding the 100-col rustfmt default, inconsistent with the wrapped siblings two lines above.
