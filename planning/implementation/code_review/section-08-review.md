# Code Review: section-08-integration-tests

Cross-checked every fixture against `src/checks/okf.rs`, `src/checks/index_md.rs`, `src/checks/log_md.rs`, `src/checks/style.rs`, `src/frontmatter.rs`, and the sort/classify logic in `src/lint.rs`. The four claimed diagnostics are all correct and land on the right lines:

- `concept-a.md:7` "line has trailing whitespace" — frontmatter with non-empty `type` (passes rules 1/2), line 7 ends with a literal trailing space.
- `concept-b.md:1` "missing or invalid YAML frontmatter" — no leading `---`, `split_frontmatter` returns `None`, `check_concept` short-circuits before checking `type`, so rule 2 correctly does not also fire (satisfies the plan's explicit callout).
- `log.md:7` "... not a valid YYYY-MM-DD date" — `## 2026-02-30` has correct date-shape but fails `chrono::NaiveDate::parse_from_str`.
- `sub/index.md:5` "index.md body line is not a heading or list item" — non-root index (`is_root=false`), no frontmatter, stray trailing paragraph after a blank-line reset of `in_list_item`.

All other files (root `index.md`, `sub/concept-b.md`) produce zero diagnostics as traced. Cross-file sort order in the `.snap` matches `sort_diagnostics`'s (path, then line, then `Rule` enum order) exactly. Trailing-newline hygiene verified file-by-file, clean.

## Snapshot verification note

The reviewer initially flagged that hand-promoting `.snap.new` → `.snap` (since no `cargo-insta` CLI is installed) might not have been confirmed against an actual `cargo test` run. **This was in fact verified**: `cargo test --test cli_tests` was run after promoting the snapshot and all 7 tests passed, including `integration_bundle_whole_output_matches_snapshot`, and the full workspace suite (84 tests) plus `cargo clippy --all-targets` were also run clean. No action needed.

## Findings

1. **(Medium, auto-fix)** No `docs/knowledge/*.md` doc was added for this section, inconsistent with every prior section and CLAUDE.md's explicit per-change requirement.
2. **(Low, auto-fix)** `concept-b.md` (root, missing-frontmatter) and `sub/concept-b.md` (clean) share a filename despite opposite behavior — confusing for a reviewer scanning the tree. Rename the `sub/` one to `concept-c.md`.
3. **(Low, let go)** Only 4 of 10 `Rule` variants appear in the snapshot; `OkfIndexFrontmatterPlacement`, `OkfMissingType`, and several style rules are unexercised at the integration layer. The plan's fixture-design constraints are explicitly illustrative ("along these lines... exact contents are an implementation choice"), and all rules already have dedicated unit-level fixture coverage from earlier sections — expanding this bundle further is scope creep on a "keep it small and deliberate" regression guard.
4. **(Low, let go)** No same-line multi-rule tie-break is exercised in the integration snapshot (every violating line has exactly one diagnostic). Already unit-tested in `lint.rs`/`diagnostic.rs`; not worth manufacturing a contrived fixture line just to duplicate that coverage at the integration layer.
5. **(Low, let go)** Root `index.md` has no frontmatter, so the "frontmatter containing only `okf_version`" pass path (`root_frontmatter_ok`) isn't exercised here — it's covered by section-03's dedicated fixtures already.
6. **(Info, no action)** `planning/implementation/deep_implement_config.json`'s `test_command` field reads `"uv run pytest"`, stale from a template default — this is deep-implement session state, not part of the project deliverable, and the setup script's own preflight already correctly detected `cargo test` for this Rust project. Not a code issue.

## What's solid

- Fixture line numbers and diagnostic messages all hand-traced correctly against the check-module source.
- Sort order in the snapshot matches `lint.rs`'s tie-break rules exactly.
- Both appended tests (`integration_bundle_whole_output_matches_snapshot`, `integration_bundle_exits_1`) are idiomatic and consistent with the existing `assert_cmd` style in `tests/cli_tests.rs`.
- No unintended style violations (extra blank lines, tabs, over-length lines) in any fixture file.

Files reviewed: `tests/fixtures/integration_bundle/**`, `tests/cli_tests.rs`, `tests/snapshots/cli_tests__integration_bundle_whole_output_matches_snapshot.snap`, `src/checks/*.rs`, `src/frontmatter.rs`, `src/lint.rs`.
