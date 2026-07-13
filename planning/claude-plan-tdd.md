# TDD Plan: okf-lint

Companion to `claude-plan.md`. For each implementation unit below,
write the listed tests **before** writing the corresponding code. Test
stubs are prose descriptions of what to assert, not implementations —
see `claude-plan.md` §10 for the fixture layout referenced throughout
(`tests/fixtures/okf/...`, `tests/fixtures/style/...`,
`tests/fixtures/integration_bundle/`) and §9 for the crates used below
(`assert_cmd`, `predicates`, `insta`).

Section numbers mirror `claude-plan.md`.

## 3. Core data types — `diagnostic.rs`

Inline `#[cfg(test)] mod tests` in `diagnostic.rs`:

- Test: two `Diagnostic`s on the same `(file, line)` sort with the OKF
  rule before the style rule, per `Rule`'s declared enum order.
- Test: `Rule`'s declared order places all `Okf*` variants before all
  `Style*` variants, and within each group matches the order listed in
  §3 of the plan (this is a direct assertion on `Rule as usize` / enum
  discriminant order, guarding against accidental reordering during
  implementation).

## `frontmatter.rs` — `split_frontmatter`

Inline `#[cfg(test)] mod tests`, using inline string literals (no
fixture files needed — these are pure-function unit tests):

- Test: content not starting with a `---` line → `FrontmatterResult::None`.
- Test: content starting with `---` but with no closing `---` line →
  `FrontmatterResult::Unclosed`.
- Test: well-formed `---`\<yaml\>`---`\<body\> content → `Found` with
  the correct `yaml_text` (exact text between delimiters, unparsed) and
  correct `body_start_line`.
- Test: a leading blank line before `---` → `None` (delimiter must be
  the literal first line).
- Test: a `---` line with trailing characters (e.g. `--- `) → `None`
  (must be exactly `---`, not a prefix match).

## `walk.rs` — `collect_md_files`

Inline `#[cfg(test)] mod tests`, using `tempfile`-style ad hoc
directories (or a small helper building a temp dir tree) — not the
`tests/fixtures/` files, since these are walk-behavior tests
independent of file content:

- Test: a mix of `.md` and non-`.md` files → only `.md` files returned.
- Test: a dotfile/dot-directory (e.g. `.git/`, `.hidden.md`) → excluded
  entirely, including anything nested under a dot-directory.
- Test: returned paths are relative to `root`, not absolute.
- Test: returned list is sorted lexicographically regardless of the
  order files were created on disk.
- Test: a permission-denied subdirectory during walk → `LintError::Io`
  is produced (may need a platform-conditional test, e.g. `#[cfg(unix)]`
  using `std::fs::Permissions`).

## `lint.rs` — `lint_bundle`

Inline `#[cfg(test)] mod tests`, using small ad hoc temp-dir bundles
built per test:

- Test: `root` does not exist → `Err(LintError::PathNotFound)`.
- Test: `root` exists but is a file, not a directory →
  `Err(LintError::NotADirectory)`.
- Test: a non-UTF-8 `.md` file in the bundle → `Err(LintError::InvalidUtf8)`,
  and the error aborts the whole run (assert no diagnostics are
  returned alongside the error — the `Result` is `Err`, full stop).
- Test: every file in the bundle gets both the style checks (§6) and
  its classification-appropriate structural checks (§5) — e.g. a
  `log.md` with a bad date heading AND a hard tab both produce
  diagnostics from a single `lint_bundle` call.
- Test: the returned `Vec<(PathBuf, Diagnostic)>` is sorted per §7
  across multiple files (cross-file ordering, not just within-file —
  covered more thoroughly by the integration test below, but include
  one `lint_bundle`-level test with 2-3 files to catch regressions
  close to the unit under test).

## 4. File classification

Covered as part of `lint.rs` tests above and/or a small standalone
`#[cfg(test)]` block next to the classification logic:

- Test: `index.md` at bundle root → classified Index, `is_root = true`.
- Test: `sub/index.md` → classified Index, `is_root = false`.
- Test: `log.md` (any depth) → classified Log.
- Test: any other filename, including a name that merely contains
  "index" or "log" as a substring (e.g. `reindex.md`, `catalog.md`) →
  classified Concept (guards against a substring-match bug instead of
  exact-name comparison).

## 5.1 Concept documents — `checks/okf.rs` (`check_concept`)

Fixture-backed tests (`tests/fixtures/okf/missing_frontmatter/{pass,fail}/`
and `tests/fixtures/okf/missing_type/{pass,fail}/`), one test per
pass/fail fixture, plus inline-literal edge cases:

- Test: `missing_frontmatter/pass/pass.md` → `check_concept` returns no
  `OkfMissingFrontmatter`/`OkfMissingType` diagnostics.
- Test: `missing_frontmatter/fail/fail.md` → exactly one
  `OkfMissingFrontmatter` diagnostic at line 1, exact message text.
- Test: `missing_type/pass/pass.md` → no diagnostics.
- Test: `missing_type/fail/fail.md` → exactly one `OkfMissingType`
  diagnostic at line 1, exact message text.
- Test (inline literal): frontmatter present but not closed
  (`Unclosed`) → `OkfMissingFrontmatter`, and rule 2 does **not** also
  fire (stops after rule 1, per docstring intent).
- Test (inline literal): frontmatter parses as valid YAML but is a
  scalar/list, not a mapping → `OkfMissingFrontmatter` (the
  "structurally-wrong-shape" case called out in §5.1).
- Test (inline literal): `type` present with a non-string value (e.g.
  `type: 5` or `type: [a, b]`) → `OkfMissingType`.
- Test (inline literal): `type` present as an empty string (`type: ""`)
  → `OkfMissingType`.
- Test (inline literal): `type` present and non-empty → no diagnostics.

## 5.2 `index.md` — `checks/index_md.rs` (`check_index`)

Fixture-backed (`tests/fixtures/okf/index_frontmatter_placement/{pass_root,fail_nonroot,fail_root_extra_key}/`
and `tests/fixtures/okf/index_body_structure/{pass,fail}/`), plus
inline-literal edge cases for the rule-3/rule-4 interaction:

- Test: `pass_root/` (root `index.md`, no frontmatter or
  `okf_version`-only frontmatter) → no `OkfIndexFrontmatterPlacement`
  diagnostics.
- Test: `fail_nonroot/` (non-root `index.md` with frontmatter) →
  exactly one `OkfIndexFrontmatterPlacement` diagnostic, `index.md must
  not contain frontmatter`.
- Test: `fail_root_extra_key/` (root `index.md` with frontmatter
  containing a key besides `okf_version`) → exactly one
  `OkfIndexFrontmatterPlacement` diagnostic, `root index.md frontmatter
  may only contain 'okf_version'`.
- Test (inline literal): root `index.md` with `Unclosed` frontmatter →
  same `root index.md frontmatter may only contain 'okf_version'`
  diagnostic (gap-filling rule from §5.2/§12).
- Test (inline literal): non-root `index.md` with `Unclosed`
  frontmatter → same `index.md must not contain frontmatter` diagnostic.
- Test: `index_body_structure/pass/pass.md` → no
  `OkfIndexBodyStructure` diagnostics.
- Test: `index_body_structure/fail/fail.md` → one
  `OkfIndexBodyStructure` diagnostic per violating body line, at the
  correct line numbers.
- Test (inline literal): heading line, then list item, then an indented
  (2+ space) continuation line → no violation (continuation accepted).
- Test (inline literal): an indented line when `in_list_item` is
  `false` (no preceding list item) → violation (continuation only
  valid immediately after a list item).
- Test (inline literal): a blank line resets `in_list_item`, so a
  subsequent indented line is a violation, not treated as a
  continuation.
- Test (inline literal): a non-root `index.md` with **both** bad
  frontmatter and a stray body paragraph → both an
  `OkfIndexFrontmatterPlacement` and an `OkfIndexBodyStructure`
  diagnostic are produced (rules 3 and 4 are independent and both fire).

## 5.3 `log.md` — `checks/log_md.rs` (`check_log`)

Fixture-backed (`tests/fixtures/okf/log_date_heading/{pass,fail}/`),
plus inline-literal edge cases:

- Test: `pass/pass.md` → no `OkfLogDateHeading` diagnostics.
- Test: `fail/fail.md` → one `OkfLogDateHeading` diagnostic at the
  offending heading's line, exact message text.
- Test (inline literal): a `##` heading with valid `YYYY-MM-DD` text →
  no diagnostic.
- Test (inline literal): a `##` heading with a calendar-invalid date
  matching the regex shape (e.g. `2026-02-30`) → diagnostic (regex
  match alone is insufficient; `chrono` calendar validation catches
  this).
- Test (inline literal): a `#` (level-1) or `###` (level-3) heading
  with non-date text → **no** diagnostic (rule only inspects exact
  `##` level).
- Test (inline literal): a `##` heading with extra trailing text after
  the date (e.g. `## 2026-05-22 Updates`) → diagnostic (must match the
  date pattern exactly, not just contain it — confirm this against the
  `^\d{4}-\d{2}-\d{2}$` anchoring in §5.3).

## 6. Markdown style checks — `checks/style.rs` (`check_style`)

Fixture-backed, one pair per rule
(`tests/fixtures/style/{max_line_length,trailing_whitespace,trailing_newline,consecutive_blank_lines,hard_tabs}/{pass,fail}/`),
plus inline-literal edge cases:

- Test: each `style/*/pass/pass.md` → `check_style` returns no
  diagnostics.
- Test: each `style/*/fail/fail.md` → `check_style` returns exactly one
  diagnostic of the corresponding `Style*` rule, correct line and exact
  message text (including `{max_line_length}`/`{actual}` substitution
  for the line-length message).
- Test (inline literal): a line with a multi-byte UTF-8 character
  (e.g. `é`) counted by `chars().count()`, not byte length — construct
  a line whose byte length exceeds `max_line_length` but whose char
  count does not, and assert no diagnostic fires (and the inverse).
- Test (inline literal): a CRLF-terminated line (`...\r\n`) → after
  splitting on `\n`, the line ends with `\r` → `StyleTrailingWhitespace`
  fires.
- Test (inline literal): a 0-byte file → `StyleTrailingNewline`
  violation at line 1.
- Test (inline literal): content with no trailing `\n` at all →
  `StyleTrailingNewline` violation.
- Test (inline literal): content ending in `\n\n` (blank line at EOF) →
  `StyleTrailingNewline` violation.
- Test (inline literal): content ending in exactly one `\n` → no
  `StyleTrailingNewline` violation.
- Test (inline literal): a run of exactly 2 blank lines → exactly one
  `StyleConsecutiveBlankLines` diagnostic, anchored at the second blank
  line's number.
- Test (inline literal): a run of 5 blank lines → exactly **one**
  `StyleConsecutiveBlankLines` diagnostic (not one per line, not one
  per pair), anchored at the second line of the run.
- Test (inline literal): two separate 2-blank-line runs in the same
  file (separated by non-blank content) → two separate
  `StyleConsecutiveBlankLines` diagnostics.
- Test (inline literal): a line with a tab both mid-line and trailing →
  both `StyleHardTab` and `StyleTrailingWhitespace` fire for that line.
- Test (inline literal): a single over-length line that also has
  trailing whitespace → both `StyleLineLength` and
  `StyleTrailingWhitespace` diagnostics are produced for that line
  (checks are independent, not mutually exclusive).

## 7. Diagnostic ordering and formatting

Covered primarily by the whole-bundle integration test below. One
targeted inline unit test alongside `lint.rs` or `diagnostic.rs`:

- Test: given an unsorted `Vec<(PathBuf, Diagnostic)>` spanning
  multiple files, multiple lines within a file, and multiple rules on
  the same `(file, line)`, sorting produces the exact order defined by
  §7 (path, then line, then `Rule` declaration order) — assert against
  a hand-constructed expected order rather than a snapshot, since this
  is testing the sort function in isolation.
- Test: `format!("{path}:{line}: {message}")` output uses `/`
  path separators even when constructed from platform-native
  `PathBuf` components (relevant on Windows CI, if applicable).

## 8. CLI — `cli.rs` and `main.rs`

`tests/cli_tests.rs`, using `assert_cmd::Command::cargo_bin("okf-lint")`
and `predicates`:

- Test: running against a nonexistent path → exit code `2`, a
  human-readable message on stderr, and **empty** stdout (no partial
  diagnostics).
- Test: running against a clean fixture bundle → exit code `0`, empty
  stdout.
- Test: running against a bundle with at least one violation → exit
  code `1`, stdout contains the expected diagnostic line(s).
- Test: `--max-line-length` override — run against a fixture whose only
  violation is a line whose length sits between 100 and a raised custom
  limit (e.g. a 120-char line with `--max-line-length 150`): asserts
  **no** diagnostic/exit-code-0 with the custom flag, and exit code `1`
  with a diagnostic when run without the flag (default 100).
- Test: default `--max-line-length` (no flag passed) behaves as if 100
  was passed explicitly (regression guard on `default_value_t`).

## 10. Whole-bundle integration test

`tests/cli_tests.rs`, using `tests/fixtures/integration_bundle/` (root
`index.md`, a subdirectory with its own `index.md`, a couple of concept
docs, and a `log.md`, combining passing and failing cases per the
fixture description in §10):

- Test: run the compiled binary against `integration_bundle/` and
  assert the full sorted stdout output via `insta::assert_snapshot!`,
  reviewed with `cargo insta review` before merging. Snapshot should be
  checked for: correct relative paths, correct `/`-separator
  formatting, correct cross-file/cross-rule ordering per §7, and that
  every seeded violation in the bundle produces exactly the diagnostics
  it's meant to (no missing, no spurious extras from unrelated files).
- Test: exit code for `integration_bundle/` (which the fixture is
  designed to contain at least one violation in) is `1`.

## Dependency/setup verification (not a runtime test, but a build gate)

- After adding `clap`, `walkdir`, `serde_yaml_ng`, `chrono` and the
  dev-dependencies (`assert_cmd`, `predicates`, `insta`) to
  `Cargo.toml` per §9, `cargo build` and `cargo test` (with no test
  files yet, or with stub `#[test] fn placeholder() {}` bodies) must
  succeed before writing real tests — confirms the dependency set
  resolves and compiles on a clean skeleton.
