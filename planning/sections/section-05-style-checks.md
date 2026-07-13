# section-05-style-checks: Markdown Style Checks

## Implementation notes (actual)

Implemented as planned in `src/checks/style.rs`, registered via `pub mod style;` in
`src/checks/mod.rs`. All 19 tests pass (`cargo test style`).

Deviations from plan:
- `trailing_newline/fail/fail.md` uses the "no trailing `\n` at all" variant (the plan
  offered a choice between that and ending in `\n\n`) — documented in
  `docs/knowledge/style-checks.md`.
- Added an early return (`if content.is_empty() { return diagnostics; }`) right after the
  trailing-newline check, so a 0-byte file produces zero real lines rather than one
  phantom blank line from `split('\n')`. Behavior-neutral for current rules (code review
  finding, not spec'd), but removes a latent trap for future rule additions.
- Added one extra test beyond the plan's list:
  `trailing_blank_lines_fire_both_newline_and_blank_run_rules`, covering content ending in
  `\n\n\n` (co-firing `StyleTrailingNewline` and `StyleConsecutiveBlankLines`).

Knowledge doc: `docs/knowledge/style-checks.md` (new), linked from
`docs/knowledge/index.md`.

## Dependencies

This section depends on **section-01-foundation** being complete, specifically:
- `src/diagnostic.rs` providing the `Diagnostic` struct and `Rule` enum (see below for the exact shape you'll need).

This section does **not** depend on section-02, section-03, or section-04, and can be implemented in parallel with them. It produces a single new file, `src/checks/style.rs`, which is self-contained and does not import `frontmatter.rs` or `walk.rs`.

Downstream, **section-06-orchestration** will call `check_style` unconditionally on every `.md` file (regardless of classification) and merge its output with the classification-specific structural checks.

## Background

`okf-lint` enforces two independent rule sets against `.md` files in an OKF bundle: OKF structural conformance (handled by sections 02-04) and generic markdown hygiene (this section). This section implements the 5 markdown hygiene rules, applied uniformly to **every** `.md` file in the bundle — Concept documents, `index.md`, and `log.md` alike — independent of file classification or OKF-specific structure.

### Required types from `diagnostic.rs` (section-01)

You will use these types, already defined by section-01:

```rust
struct Diagnostic {
    line: usize,     // 1-based
    rule: Rule,
    message: String, // exact text, already formatted (e.g. with {N} substituted)
}

enum Rule {
    // OKF conformance variants (defined by other sections) come first:
    OkfMissingFrontmatter,
    OkfMissingType,
    OkfIndexFrontmatterPlacement,
    OkfIndexBodyStructure,
    OkfLogDateHeading,
    // Markdown style, in this fixed order — these are the variants this section uses:
    StyleLineLength,
    StyleTrailingWhitespace,
    StyleTrailingNewline,
    StyleConsecutiveBlankLines,
    StyleHardTab,
}
```

The `Rule` enum's declaration order is used elsewhere (section-06/section-07) as a same-line tie-break for diagnostic sorting — OKF rules before style rules, and within the style group, in the exact order `StyleLineLength`, `StyleTrailingWhitespace`, `StyleTrailingNewline`, `StyleConsecutiveBlankLines`, `StyleHardTab`. Do not reorder these variants relative to each other. (If section-01 has not yet added these five variants to the `Rule` enum, add them in this order at the end of the enum as part of this section's work.)

## File to create

`src/checks/style.rs`

```rust
fn check_style(content: &str, max_line_length: usize) -> Vec<Diagnostic>
```

This should be registered in `src/checks/mod.rs` (created/extended by whichever section lands first; if it doesn't exist yet, create it with `pub mod style;` plus stub `pub mod okf;`, `pub mod index_md;`, `pub mod log_md;` if not already present — coordinate by only adding your own `pub mod style;` line if the file already exists).

## Implementation details (plan §6)

Applies uniformly to every `.md` file (Concept, Index, and Log alike), independent of the OKF checks in sections 02-04.

Split `content` on `\n`. If `content` ends with `\n`, the split produces one trailing empty string that is **not** a real line — drop it before running the per-line checks below (rules 1, 2, 5). The trailing-newline check (rule 3) instead inspects the raw content directly (see below), not the split-line list.

### 1. `StyleLineLength`

For each real line (1-indexed), if `line.chars().count() > max_line_length` (Unicode scalar count, not byte length — a 2-byte UTF-8 character like `é` counts as 1), emit a diagnostic at that line, message:

```
line exceeds maximum length of {max_line_length} characters ({actual} found)
```

with `{actual}` being the counted length.

### 2. `StyleTrailingWhitespace`

For each real line, if it ends with one or more of: space, tab, or `\r` (the `\r` case handles CRLF-terminated input, since we only split on `\n` — a CRLF line ends with `\r` after the split, which this rule treats identically to trailing spaces/tabs, effectively enforcing LF-only files), emit a diagnostic at that line, message:

```
line has trailing whitespace
```

### 3. `StyleTrailingNewline`

Inspect `content` directly (not the split list): a 0-byte file is a violation (message below, line 1). Otherwise, violation if `content` does not end with exactly one `\n` — i.e. it doesn't end with `\n` at all, or it ends with `\n\n` (two or more trailing newlines, meaning trailing blank lines). Diagnostic at line 1, message:

```
file must end with exactly one trailing newline
```

### 4. `StyleConsecutiveBlankLines`

Walk the real lines tracking a run-length counter of consecutive lines that are empty or whitespace-only (reset to 0 on any non-blank line). The rule is violated once a run reaches length 2; emit exactly **one** diagnostic per run, anchored at the line number of the *second* blank line in that run (the point at which "two or more consecutive" first becomes true) — not one diagnostic per additional blank line in a longer run. Message:

```
multiple consecutive blank lines
```

(This one-diagnostic-per-run anchoring is a plan-level judgment call: the spec didn't define how many diagnostics a 5-blank-line run should produce, and one-per-run avoids diagnostic-count explosion for large gaps while still pinpointing where the run starts being invalid.)

### 5. `StyleHardTab`

For each real line, if it contains a `\t` character anywhere in its content (not just trailing — a tab mid-line also counts, and can co-fire with rule 2 if the tab is also trailing), emit a diagnostic at that line, message:

```
line contains a hard tab character
```

All five checks run independently over the same line set — a single line can produce diagnostics for multiple rules (e.g. rule 1 and rule 2 both firing on an over-length line with trailing whitespace).

## Test fixtures to create

Under `tests/fixtures/style/`, create one `pass/pass.md` and `fail/fail.md` pair per rule. Each `pass/`/`fail/` pair is a single-file mini-bundle (the CLI, in later sections, will be run with `pass/` or `fail/` as `<path>`, not their shared parent — this isolates exactly one file per test run). Layout:

```
tests/fixtures/style/
  max_line_length/{pass/pass.md, fail/fail.md}
  trailing_whitespace/{pass/pass.md, fail/fail.md}
  trailing_newline/{pass/pass.md, fail/fail.md}
  consecutive_blank_lines/{pass/pass.md, fail/fail.md}
  hard_tabs/{pass/pass.md, fail/fail.md}
```

- `max_line_length/fail/fail.md`: contains at least one line exceeding the default 100-char limit.
- `max_line_length/pass/pass.md`: all lines at or under 100 chars.
- `trailing_whitespace/fail/fail.md`: at least one line ending in a trailing space or tab.
- `trailing_whitespace/pass/pass.md`: no trailing whitespace anywhere.
- `trailing_newline/fail/fail.md`: either has no trailing `\n`, or ends with `\n\n` (choose one; document which).
- `trailing_newline/pass/pass.md`: ends with exactly one `\n`.
- `consecutive_blank_lines/fail/fail.md`: contains a run of 2+ blank lines.
- `consecutive_blank_lines/pass/pass.md`: no run of 2+ consecutive blank lines.
- `hard_tabs/fail/fail.md`: contains a literal tab character somewhere in a line.
- `hard_tabs/pass/pass.md`: no tab characters.

## Tests (write first, per TDD plan §6)

Write these as fixture-backed tests plus inline-literal edge cases, in an inline `#[cfg(test)] mod tests` block within `src/checks/style.rs`:

- Test: each `style/*/pass/pass.md` → `check_style` returns no diagnostics.
- Test: each `style/*/fail/fail.md` → `check_style` returns exactly one diagnostic of the corresponding `Style*` rule, correct line and exact message text (including `{max_line_length}`/`{actual}` substitution for the line-length message).
- Test (inline literal): a line with a multi-byte UTF-8 character (e.g. `é`) counted by `chars().count()`, not byte length — construct a line whose byte length exceeds `max_line_length` but whose char count does not, and assert no diagnostic fires (and the inverse: a line whose char count exceeds the limit but byte length matters less).
- Test (inline literal): a CRLF-terminated line (`...\r\n`) — after splitting on `\n`, the line ends with `\r` → `StyleTrailingWhitespace` fires.
- Test (inline literal): a 0-byte file → `StyleTrailingNewline` violation at line 1.
- Test (inline literal): content with no trailing `\n` at all → `StyleTrailingNewline` violation.
- Test (inline literal): content ending in `\n\n` (blank line at EOF) → `StyleTrailingNewline` violation.
- Test (inline literal): content ending in exactly one `\n` → no `StyleTrailingNewline` violation.
- Test (inline literal): a run of exactly 2 blank lines → exactly one `StyleConsecutiveBlankLines` diagnostic, anchored at the second blank line's number.
- Test (inline literal): a run of 5 blank lines → exactly **one** `StyleConsecutiveBlankLines` diagnostic (not one per line, not one per pair), anchored at the second line of the run.
- Test (inline literal): two separate 2-blank-line runs in the same file (separated by non-blank content) → two separate `StyleConsecutiveBlankLines` diagnostics.
- Test (inline literal): a line with a tab both mid-line and trailing → both `StyleHardTab` and `StyleTrailingWhitespace` fire for that line.
- Test (inline literal): a single over-length line that also has trailing whitespace → both `StyleLineLength` and `StyleTrailingWhitespace` diagnostics are produced for that line (checks are independent, not mutually exclusive).

Test stubs are prose descriptions of what to assert, not implementations — flesh out each with a concrete fixture read (`std::fs::read_to_string`) or an inline `&str` literal, then assert on the returned `Vec<Diagnostic>`'s length, `rule`, `line`, and `message` fields.

## Notes / edge cases to keep in mind while implementing

- The trailing-newline check operates on raw `content`, not the post-split line vector — do not accidentally reuse the line-splitting logic for this rule.
- "Blank line" for rule 4 means empty *or* whitespace-only (e.g. a line containing only spaces/tabs counts as blank).
- The five checks are fully independent passes (or a single combined pass, implementation's choice) over the same real-line list; nothing about one rule should suppress or alter another rule's firing on the same line.
- `max_line_length` is a plain `usize` parameter to `check_style` — this section does not touch CLI wiring (that default-100 / `--max-line-length` flag belongs to section-07-cli); just accept it as a function argument.