## section-04-log-checks

### Dependencies

This section depends on **section-01-foundation**, which must be
completed first. From that section this module uses:

- `diagnostic.rs`'s `Diagnostic` struct and `Rule` enum (already
  defined with the `OkfLogDateHeading` variant included in its fixed
  declaration order — do not redefine or reorder these; just import
  them).
- No dependency on `frontmatter.rs` — `log.md` has no frontmatter
  handling in this linter.

This section does **not** depend on section-02, section-03, or
section-05 — all four check modules are independent and
parallelizable after section-01.

### What you're building

`src/checks/log_md.rs`, implementing rule 5
(`Rule::OkfLogDateHeading`): validating that every level-2 (`##`)
heading in a `log.md` file is a real calendar date in `YYYY-MM-DD`
format.

Add `src/checks/mod.rs` module declaration for `log_md` if not already
present from a sibling section (declare `pub mod log_md;` — coordinate
naming with whichever check-module section runs first; the `mod.rs`
file itself may already exist with other `pub mod` lines from
section-02/03/05, in which case just add this line to it).

This module also needs the `chrono` crate (added to `Cargo.toml` in
section-01 per the plan's dependency list — confirm it's present;
if section-01 didn't add it, add `chrono` to `[dependencies]` in
`Cargo.toml` as part of this section).

### Background: `Diagnostic` and `Rule` (from section-01, for reference)

```rust
struct Diagnostic {
    line: usize,     // 1-based
    rule: Rule,       // used only for sort tie-breaking and internal grouping, never printed
    message: String,  // exact text, already formatted
}

enum Rule {
    // OKF conformance, in this fixed order:
    OkfMissingFrontmatter,
    OkfMissingType,
    OkfIndexFrontmatterPlacement,
    OkfIndexBodyStructure,
    OkfLogDateHeading,
    // Markdown style, in this fixed order:
    StyleLineLength,
    StyleTrailingWhitespace,
    StyleTrailingNewline,
    StyleConsecutiveBlankLines,
    StyleHardTab,
}
```

`check_log` only ever constructs `Diagnostic { line, rule:
Rule::OkfLogDateHeading, message }`.

### Tests FIRST

Write these tests before the implementation, as an inline
`#[cfg(test)] mod tests` block in `src/checks/log_md.rs`.

Fixture-backed tests use `tests/fixtures/okf/log_date_heading/{pass,fail}/`
(create these fixture files as part of this section — see "Fixtures"
below). Both fixture directories are single-file mini-bundles
(`pass/pass.md` and `fail/fail.md`), matching the pattern used by the
other OKF-rule fixtures in this project.

- Test: `pass/pass.md` → `check_log` returns no
  `OkfLogDateHeading` diagnostics.
- Test: `fail/fail.md` → `check_log` returns exactly one
  `OkfLogDateHeading` diagnostic, at the offending heading's line
  number, with the exact message text `log.md heading is not a valid
  YYYY-MM-DD date`.
- Test (inline literal): a `##` heading with valid `YYYY-MM-DD` text
  (e.g. `## 2026-05-22`) → no diagnostic.
- Test (inline literal): a `##` heading with a calendar-invalid date
  that matches the regex shape (e.g. `## 2026-02-30`) → one
  diagnostic (regex match alone is insufficient; `chrono`'s calendar
  validation must catch this — this is the reason `chrono` is a
  dependency rather than a hand-rolled regex-only check).
- Test (inline literal): a `#` (level-1) heading with non-date text,
  and a `###` (level-3) heading with non-date text → **no** diagnostic
  in either case (the rule only inspects headings that are exactly
  level 2; other heading levels are out of scope for this rule
  entirely).
- Test (inline literal): a `##` heading with extra trailing text after
  the date (e.g. `## 2026-05-22 Updates`) → one diagnostic (the date
  pattern must match the entire captured heading text exactly via
  `^\d{4}-\d{2}-\d{2}$` anchoring, not merely contain a valid date
  substring).
- Test (inline literal, recommended addition for completeness): a
  file with multiple `##` headings, some valid dates and some not →
  exactly one diagnostic per invalid heading, each at its own correct
  line number (the rule is per-heading-line, not per-file).
- Test (inline literal, recommended addition): `check_log` called on
  content with no `##` headings at all → empty diagnostic vec (no
  false positives from level-1/level-3-only files).

### Implementation details

Function signature:

```rust
fn check_log(content: &str) -> Vec<Diagnostic>
```

**Docstring intent (rule 5, `OkfLogDateHeading`):** `log.md` has no
frontmatter handling in this linter (it is not one of the 5
conformance rules) — scan the *entire* `content` (not body-offset,
unlike `index.md`'s rule 4) line by line for lines matching exactly
`^## (.*)$` (heading level exactly 2 — a single `#`, or `###` or
deeper, heading is not inspected by this rule at all). For each such
heading line, take the captured text (the `(.*)` group — everything
after `## `) and validate it is exactly four digits, `-`, two digits,
`-`, two digits (regex `^\d{4}-\d{2}-\d{2}$`) **and** parses as a real
calendar date (reject e.g. `2026-02-30`, which matches the regex shape
but isn't a valid date). Use the `chrono` crate's
`NaiveDate::parse_from_str(text, "%Y-%m-%d")` for calendar validation
rather than hand-rolling leap-year math — this is the standard,
well-tested way to validate a calendar date in Rust, and was added as
a small justified dependency beyond what the original
research/interview covered.

A match failure — either the regex fails to match the captured heading
text, or the regex matches but `NaiveDate::parse_from_str` fails —
emits one diagnostic at that heading line's 1-based line number, with
message exactly:

```
log.md heading is not a valid YYYY-MM-DD date
```

Line numbering: split `content` on `\n` (same convention as elsewhere
in the codebase); the 1-based index of the line within that split is
the diagnostic's `line`. Note this scan is over the raw content lines,
not filtered for a trailing empty string the way `checks/style.rs`
does — a trailing empty line from a final `\n` won't match the `## `
pattern anyway, so no special handling is needed here.

Practical notes on the two validation steps:
1. First check the line matches `^## (.*)$` — i.e. starts with
   exactly `## ` (two `#` characters and a single space) and capture
   everything after that space to end of line. Lines starting with
   `# ` (one `#`) or `### ` (three or more `#`) must not be treated as
   matches of this pattern at all — do not fall through to
   date-validating a level-1 or level-3+ heading's text.
2. Then check the captured text against the regex
   `^\d{4}-\d{2}-\d{2}$` — this rejects any trailing/leading extra
   text, whitespace, or wrong digit grouping.
3. Then, only if the regex matched, additionally validate via
   `chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")`, treating an
   `Err` result the same as a regex failure (one diagnostic, same
   message).

Both failure paths (regex fails, or regex passes but `chrono` parse
fails) produce the identical diagnostic — the implementation does not
need to distinguish them in the message text, only internally in
control flow.

### Fixtures to create

`tests/fixtures/okf/log_date_heading/pass/pass.md` — a `log.md`-style
file with one or more `##` headings, all valid `YYYY-MM-DD` dates
(e.g. `## 2026-01-15`), plus arbitrary body text under each. May also
include a `#` or `###` heading with non-date text to confirm those are
ignored, though this isn't required for the pass case to be valid.

`tests/fixtures/okf/log_date_heading/fail/fail.md` — a `log.md`-style
file containing at least one `##` heading with invalid date text
(either wrong format or calendar-invalid, e.g. `## 2026-02-30` or
`## Not A Date`), positioned so the test can assert the diagnostic's
line number precisely.

Both fixtures are run as single-file mini-bundles per the project
convention (CLI/`check_log` invoked with `pass/` or `fail/` as the
root, isolating the rule under test — though for this section's unit
tests, `check_log` is called directly on the fixture file's contents
read via `std::fs::read_to_string`, not through the full CLI/orchestration
layer, since orchestration is section-06's responsibility).

### File paths touched by this section

- Create: `src/checks/log_md.rs` (the `check_log` function and its
  inline `#[cfg(test)] mod tests`).
- Modify: `src/checks/mod.rs` — add `pub mod log_md;` if not already
  present.
- Modify (if needed): `Cargo.toml` — ensure `chrono` is present in
  `[dependencies]`.
- Create: `tests/fixtures/okf/log_date_heading/pass/pass.md`
- Create: `tests/fixtures/okf/log_date_heading/fail/fail.md`

### Out of scope for this section

- File classification (deciding whether a given file *is* `log.md`,
  vs. `index.md` or a concept doc) — that's section-06-orchestration's
  responsibility (plan §4). This section's `check_log` is a pure
  function operating on already-known-to-be-`log.md` content.
- Dispatching `check_log` alongside the style checks for a given file
  — also section-06's responsibility (plan §3's `lint_bundle`
  docstring, §7's diagnostic ordering).
- CLI wiring, exit codes — section-07.