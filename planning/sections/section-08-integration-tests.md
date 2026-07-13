# section-08-integration-tests: Whole-bundle integration test (`tests/fixtures/integration_bundle/`, `insta` snapshot)

## Dependencies

This section requires:
- **section-01-foundation**: `Cargo.toml` must already have the `insta` dev-dependency added (used for snapshot testing here), alongside `assert_cmd` and `predicates` (already used by section-07's tests).
- **section-07-cli**: provides the compiled `okf-lint` binary (`src/cli.rs`, `src/main.rs`) with its full exit-code contract (`0` clean / `1` diagnostics found / `2` usage-IO error) and diagnostic output format (`{relative_path}:{line}: {message}`, one per stdout line, already sorted). It also creates `tests/cli_tests.rs` with the CLI-level tests (nonexistent path, clean bundle, bundle-with-violations, `--max-line-length` override). **This section appends to that same file** rather than creating a new one.
- Transitively, this section exercises every check module from section-02 through section-06 (concept, index, log, style checks, plus orchestration/classification and diagnostic sort ordering), since the integration bundle is designed to trigger a mix of rules across multiple files.

Do not reimplement any check logic, CLI parsing, or orchestration here — this section only builds a fixture tree and a whole-binary test that exercises the already-built pipeline end-to-end.

## What this section builds

1. A new fixture tree: `tests/fixtures/integration_bundle/` — a small multi-file OKF bundle combining passing and failing cases across multiple files and rules (details below).
2. Two tests appended to the existing `tests/cli_tests.rs` (created by section-07):
   - A whole-bundle `insta::assert_snapshot!` test asserting the exact, fully-sorted stdout diagnostic output.
   - An exit-code test asserting `integration_bundle/` (which is deliberately seeded with at least one violation) produces exit code `1`.

## Background: why a separate integration fixture

Every earlier section's fixtures are single-rule, single-file mini-bundles (a `pass/` or `fail/` directory containing exactly one `.md` file), designed to isolate one rule at a time. This section is the first to test the tool "for real": a directory tree with several files of different roles (root index, subdirectory index, concept docs, log), where classification (§4 of the plan — file-name-based dispatch to Concept/Index/Log checks), cross-file diagnostic ordering, and path-relative formatting all interact simultaneously. No earlier section's fixtures exercise that interaction, hence the dedicated `integration_bundle/`.

## Fixture design: `tests/fixtures/integration_bundle/`

Per the plan (§10): "a small multi-file tree: root `index.md`, a subdirectory with its own `index.md`, a couple of concept docs, a `log.md` — combining passing and failing cases."

Concretely, build a tree along these lines (exact file contents are an implementation choice, but must satisfy the constraints below):

```
tests/fixtures/integration_bundle/
  index.md              # root index.md
  log.md                # log.md
  concept-a.md           # a concept document
  sub/
    index.md              # non-root index.md
    concept-b.md           # another concept document
```

Design constraints, to make the snapshot meaningful and stable:
- **At least one violation must exist overall** (required for the exit-code-1 test). Simplest approach: make most files pass cleanly, and seed one or two deliberate violations in specific files so the snapshot output is small and easy to review (e.g. one concept doc missing frontmatter, or one line in a concept doc exceeding the default 100-char line length).
- **Cover a mix of rule categories** across the tree so the snapshot is a meaningful regression guard — e.g.:
  - Root `index.md`: valid (no frontmatter, or frontmatter containing only `okf_version`), with a clean heading/list-item body — passes rules 3 and 4.
  - `sub/index.md`: something that demonstrates rule 3 or 4 firing (e.g. a stray paragraph line that isn't a heading or list item, to trigger `OkfIndexBodyStructure`) — this is a non-root index, so avoid giving it frontmatter unless the intent is specifically to also demonstrate rule 3 (`index.md must not contain frontmatter`).
  - `concept-a.md`: valid frontmatter with a `type` field — passes rules 1 and 2, but could carry a style violation (e.g. trailing whitespace on one line) to demonstrate the style checks apply uniformly regardless of file role.
  - `concept-b.md`: missing frontmatter entirely — triggers rule 1 (`OkfMissingFrontmatter`), demonstrating that rule 2 is correctly *not* also emitted for the same file (per §5.1: rule 2 only fires if rule 1 did not).
  - `log.md`: at least one `## YYYY-MM-DD` heading that's valid, and optionally one invalid date heading (e.g. `## 2026-02-30`) to demonstrate rule 5 (`OkfLogDateHeading`).
- Keep the bundle small and deliberate — this is a regression-guard snapshot, not a stress test. Every seeded violation should be traceable by a reviewer running `cargo insta review`.
- All files must end with a single trailing newline and otherwise be clean of *unintended* style violations (don't accidentally add hard tabs, extra blank lines, or over-length lines outside of the ones deliberately seeded), so the snapshot only reflects intentional violations.

## Tests (write these first)

Append to `tests/cli_tests.rs` (created by section-07 — do not create a new file):

- **Test: whole-bundle snapshot.** Run the compiled binary (`assert_cmd::Command::cargo_bin("okf-lint")`) against `tests/fixtures/integration_bundle/` and capture stdout. Assert the full output via `insta::assert_snapshot!(stdout)`. On first run this creates a pending `.snap.new` file; review and approve it with `cargo insta review` before merging (there is no "correct" snapshot to hand-author — the plan's contract is that the snapshot output must be manually verified once, then locked in as a regression guard). When reviewing/approving, verify:
  - Relative paths are correct and use `/` separators (not OS-specific separators, even though this test presumably runs on Linux where it wouldn't visibly differ — the convention still matters for portability of the fixture/test itself).
  - Every diagnostic line matches the `{relative_path}:{line}: {message}` format from §7 of the plan.
  - Cross-file ordering is correct: diagnostics sorted first by `relative_path` lexicographically, then by `line` ascending, then by `Rule`'s declared enum order (OKF rules before style rules) as a same-line tie-break.
  - Every deliberately-seeded violation in the bundle produces exactly the diagnostic(s) it's meant to — no missing diagnostics, and no spurious extras leaking from unrelated files or rules that shouldn't fire (e.g. confirm `concept-b.md`'s missing-frontmatter case does *not* also produce a rule-2 `OkfMissingType` diagnostic).
- **Test: exit code for `integration_bundle/`.** Run the binary against the same fixture and assert exit code `1` (the fixture is deliberately designed to contain at least one violation, per the fixture design above).

Both tests use the same `assert_cmd`/`predicates` machinery already established in section-07's tests in this file — no new test-harness setup is needed beyond adding `insta` usage for the snapshot assertion.

## File paths summary

- Create: `tests/fixtures/integration_bundle/index.md`
- Create: `tests/fixtures/integration_bundle/log.md`
- Create: `tests/fixtures/integration_bundle/concept-a.md`
- Create: `tests/fixtures/integration_bundle/concept-b.md`
- Create: `tests/fixtures/integration_bundle/sub/index.md`
- Create: `tests/fixtures/integration_bundle/sub/concept-b.md` (or similarly named second concept doc under `sub/`, per the "couple of concept docs" requirement — exact naming/placement is an implementation choice as long as the constraints above are satisfied)
- Modify: `tests/cli_tests.rs` (append the two tests above to the file created by section-07)
- Create (on first test run, via `cargo insta review`): `tests/snapshots/*.snap` — the approved `insta` snapshot file(s) for the whole-bundle test, to be committed once reviewed.

## As-built notes (post code-review)

Implemented largely as planned, with these deviations (see
`planning/implementation/code_review/section-08-{diff,review,interview}.md`):

- **`sub/concept-b.md` renamed to `sub/concept-c.md`** (and its heading / the link in
  `sub/index.md` updated to match). The original name collided with the root-level
  `concept-b.md` (which deliberately fails rule 1) despite opposite behavior (this one is
  fully clean) — code review flagged this as confusing for a human scanning the fixture
  tree. Renaming didn't change the snapshot content since this file produces zero
  diagnostics either way.
- **No `cargo-insta` CLI available** in this environment. The pending `.snap.new` was
  reviewed by hand-tracing all four seeded diagnostics against the check-module source
  (line numbers, messages, and cross-file sort order all independently verified) rather
  than via `cargo insta review`, then promoted to `.snap` by renaming and stripping the
  `assertion_line:` metadata field. `cargo test` was run afterward and confirmed the
  promoted snapshot passes byte-for-byte.
- Added `docs/knowledge/integration-tests.md` per CLAUDE.md's per-section knowledge-doc
  requirement (not called out in the original plan) and linked it from
  `docs/knowledge/index.md`.
- Rule coverage in the snapshot is intentionally partial (4 of 10 `Rule` variants) — every
  rule already has dedicated unit-level fixture coverage from earlier sections; code review
  confirmed this is consistent with the plan's own "keep it small and deliberate, not a
  stress test" framing rather than a gap.

Final test count: 7 tests in `tests/cli_tests.rs` (5 from section-07 + 2 new), all passing;
full workspace suite (84 tests total) and `cargo clippy --all-targets` clean of new
warnings.