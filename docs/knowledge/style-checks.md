---
type: module
---

# Style checks

Generic markdown hygiene rules, applied uniformly to **every** `.md` file in the
bundle (Concept, `index.md`, `log.md` alike), independent of the OKF structural
checks in `okf.rs`/`index_md.rs`/`log_md.rs`.

## `src/checks/style.rs`

- `check_style(content: &str, max_line_length: usize) -> Vec<Diagnostic>` — implements 5
  rules, checked in this order per real line: `StyleLineLength`, `StyleTrailingWhitespace`,
  `StyleHardTab`, then a running `StyleConsecutiveBlankLines` tally. `StyleTrailingNewline` is
  checked once up front against raw `content`, not the per-line list. Insertion order into the
  returned `Vec` does not need to match the canonical `Rule` declaration order — `Rule` and
  `Diagnostic` both derive `Ord` in the spec'd variant order, so any downstream stable sort by
  `(line, rule)` normalizes the final order regardless of how `check_style` pushed them.
- **Real lines**: `content.split('\n')`, with the one trailing empty string dropped when
  `content` ends with `\n` (that phantom entry isn't a real line — it's an artifact of
  `split` on a delimiter-terminated string). If `content` is empty, `check_style` returns
  immediately after the trailing-newline check — there are zero real lines, not a
  single phantom blank one.
- **`StyleLineLength`**: `line.chars().count() > max_line_length` — Unicode scalar count,
  not byte length, so multi-byte UTF-8 (e.g. `é`) counts as 1 char each. Exempts table rows
  (`is_table_row`, `pub(crate)` in `style.rs`): a table can't be shortened without breaking
  its column structure, so flagging it would report a violation the user has no reasonable
  way to fix. `is_table_row` strips inline code spans (`` `...` ``) via `strip_inline_code`
  before checking for a `|` — a pipe inside backticks (e.g. prose documenting `` `foo | bar` ``)
  is a literal character, not a table delimiter, and must not blanket-exempt the line. Only
  a `|` outside of any code span counts. `is_table_row` is shared with `style_fix.rs`, which
  uses the same definition to decide what `fix_style` must leave un-rewrapped — the two stay
  in sync by construction, not by convention.
- **`StyleTrailingWhitespace`**: line ends with space, tab, or `\r`. The `\r` case is what
  catches CRLF-terminated input, since content is only ever split on `\n`.
- **`StyleTrailingNewline`**: checked against raw `content`, not the split line list. Violates
  if `content` is empty, doesn't end with `\n` at all, or ends with `\n\n` (trailing blank
  line(s)). The fixture at `tests/fixtures/style/trailing_newline/fail/fail.md` exercises the
  "no trailing `\n` at all" branch specifically (chosen over the `\n\n` alternative the plan
  offered, since it's a strictly simpler single-condition fixture).
- **`StyleConsecutiveBlankLines`**: a line is blank if `line.trim().is_empty()`. Tracks a
  run-length counter across real lines, reset on any non-blank line. Emits exactly one
  diagnostic per run, anchored at the run's *second* blank line (not one per line in longer
  runs) — a plan-level judgment call to avoid diagnostic-count explosion on large gaps.
- **`StyleHardTab`**: line contains `\t` anywhere, not just trailing — can co-fire with
  `StyleTrailingWhitespace` if the tab is also the last character.
- All 5 checks are independent passes over the same real-line list; nothing about one rule
  suppresses another firing on the same line (e.g. an over-length line with trailing
  whitespace produces both diagnostics) — except `StyleLineLength` vs. table rows, which is
  a deliberate exemption rather than incidental suppression. A table row can still fire
  `StyleTrailingWhitespace`/`StyleHardTab`/etc.; only line-length is exempt.
- `max_line_length` is a plain parameter — no CLI wiring here (default 100 / `--max-line-length`
  belongs to `section-07-cli`).

Test fixtures live under `tests/fixtures/style/<rule>/{pass,fail}/{pass,fail}.md`, one pair per
rule, each an isolated single-file mini-bundle.
