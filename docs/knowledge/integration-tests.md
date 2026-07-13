---
type: module
---

# Integration tests

The whole-pipeline regression guard: exercises the compiled `okf-lint` binary against a
multi-file bundle, verifying that classification, all five check modules, cross-file
diagnostic sorting, and CLI formatting all interact correctly together. No earlier
section's fixtures do this — every prior fixture is a single-rule, single-file mini-bundle
designed to isolate one rule at a time.

## `tests/fixtures/integration_bundle/`

```
index.md              # root index.md — clean (no frontmatter, heading + list body)
log.md                 # one valid date heading, one invalid (2026-02-30 → OkfLogDateHeading)
concept-a.md            # valid frontmatter/type, one line with trailing whitespace
concept-b.md            # missing frontmatter entirely → OkfMissingFrontmatter (rule 2 correctly
                          # does not also fire, since check_concept short-circuits on FrontmatterResult::None)
sub/
  index.md               # non-root index, no frontmatter, one stray paragraph line → OkfIndexBodyStructure
  concept-c.md            # valid frontmatter, fully clean (no diagnostics)
```

Deliberately seeded violations (4 total, hand-traced against each check module and
cross-checked in code review):

| File | Line | Rule | Message |
|---|---|---|---|
| `concept-a.md` | 7 | `StyleTrailingWhitespace` | line has trailing whitespace |
| `concept-b.md` | 1 | `OkfMissingFrontmatter` | missing or invalid YAML frontmatter |
| `log.md` | 7 | `OkfLogDateHeading` | log.md heading is not a valid YYYY-MM-DD date |
| `sub/index.md` | 5 | `OkfIndexBodyStructure` | index.md body line is not a heading or list item |

Coverage is intentionally partial (4 of 10 `Rule` variants) — every rule already has
dedicated unit-level fixture coverage from earlier sections; this bundle's job is only to
prove the *interaction* (classification + cross-file sort + CLI formatting), not to
re-exercise every rule again.

## Tests: `tests/cli_tests.rs` (appended, not a new file — see `docs/knowledge/cli.md`)

- `integration_bundle_whole_output_matches_snapshot` — runs the binary against the bundle
  and asserts the full stdout via `insta::assert_snapshot!`. The approved snapshot lives at
  `tests/snapshots/cli_tests__integration_bundle_whole_output_matches_snapshot.snap`.
  Since no `cargo-insta` CLI is installed in this environment, the pending `.snap.new` was
  reviewed by hand-tracing every diagnostic against the check-module source (see table
  above) and promoted to `.snap` manually (rename + strip the `assertion_line:` metadata
  field, matching what `cargo insta accept` produces) rather than via `cargo insta review`.
- `integration_bundle_exits_1` — asserts exit code 1 against the same bundle (it's
  deliberately seeded with violations).

Diagnostic order in the snapshot (`concept-a.md` < `concept-b.md` < `log.md` <
`sub/index.md`) matches `lint_bundle`'s sort (path, then line, then `Rule` declaration
order) exactly — root `index.md` and `sub/concept-c.md` are absent from the output since
they produce zero diagnostics.
