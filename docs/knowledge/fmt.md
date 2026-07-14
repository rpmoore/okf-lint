---
type: module
---

# fmt

Auto-corrects the mechanical subset of the style rules (`src/checks/style.rs`) in
place, instead of only reporting them. Structural OKF rules (frontmatter, index body
structure, log date heading) are semantic, not formatting, and stay out of scope —
`fmt` never touches them, though it does surface them (and any style violation it
couldn't safely fix) as residual diagnostics after fixing what it can.

## `src/checks/style_fix.rs`

- `fix_style(content: &str, max_line_length: usize, tab_width: usize) -> String` — pure
  content-to-content transform, no I/O. Mirrors `check_style`'s per-rule logic but
  rewrites instead of diagnosing. Pipeline, each stage consuming the previous stage's
  output:
  1. **Hard tabs → spaces**: every `\t` replaced with `tab_width` spaces, on every
     line unconditionally — matches `check_style`'s own unconditional per-line tab
     check (no code-fence exemption there either, so none here).
  2. **Trailing whitespace trimmed**: trailing `' '`, `'\t'`, `'\r'` removed from every
     line. This incidentally normalizes CRLF → LF too, since a line-final `\r` is
     exactly what `check_style` flags as trailing whitespace.
  3. **Consecutive blank lines collapsed**: any run of 2+ whitespace-only lines
     collapses to a single empty line.
  4. **Overlong-line rewrap**: lines are grouped into maximal segments of contiguous
     non-blank lines that share the same skip flag (`compute_skip_flags`) — a segment
     boundary is also cut wherever the flag changes, not only at blank lines, so a
     paragraph directly adjacent to a skipped line (e.g. right after a closing `---`
     frontmatter delimiter or a heading, with no blank line between) is still its own
     rewrappable segment rather than being dragged into the skip. A segment is left
     completely alone — overlong lines included — if its lines are frontmatter, fenced
     code (` ``` `/`~~~`), a heading (`#`), or a table row (`style::is_table_row`, shared
     with `check_style`'s `StyleLineLength` exemption — see `docs/knowledge/style-checks.md`).
     Blockquotes (`>`) are likewise left alone. Everything else *is* rewrapped:
     - **Plain paragraphs** (`wrap_paragraph_block`): the block's lines are joined and
       greedily repacked into lines ≤ `max_line_length` chars.
     - **List items** (`wrap_list_block`, triggered when any line in the block matches
       `list_marker_prefix` — `-`/`*`/`+`/`1.`/`1)`): each item (marker line plus any
       non-marker continuation lines that follow it) is repacked independently, with
       continuation lines given a hanging indent equal to the marker's width (`"- "` →
       2 spaces, `"10. "` → 4) so wrapped bullets don't get folded into one paragraph.
     - Both wrap paths tokenize via `tokenize`, which treats a whole `[text](url)` span
       as one unsplittable token — even though it may contain internal spaces — so a
       link is never broken across lines. Bare URLs stay intact automatically (no
       internal whitespace to split on). A single token that's still longer than
       `max_line_length` on its own (a long link, a long word) gets its own
       over-length line — unavoidable, tokens are never split mid-token.
  5. **Trailing newline normalized**: any trailing blank lines are dropped, then
     exactly one `\n` is appended. Empty input (`""`) is returned unchanged — there's
     no content to format, so `fmt` leaves a still-nonconformant empty file alone
     rather than fabricating a bare newline.
- `fix_style` is idempotent: running it twice produces the same output as running it
  once, and running `check_style` on its output reports none of the 5 style rules
  (table-row `StyleLineLength` is exempt in `check_style` itself, not just skipped by
  `fix_style` — see `docs/knowledge/style-checks.md`).

## `src/fmt.rs`

- `FmtOutcome { changed_files: Vec<PathBuf>, remaining: Vec<(PathBuf, Diagnostic)> }`.
- `run_fmt(root, max_line_length, tab_width, check) -> Result<FmtOutcome, LintError>` —
  reuses `walk::collect_md_files` and the same read/UTF-8-validate logic as
  `lint::lint_bundle`, so a bundle root that doesn't exist, isn't a directory, or
  contains non-UTF-8 fails with the same `LintError` variants `lint` would produce.
  For each file: compute `fix_style(&content, ...)`; if it differs from the original,
  the file is recorded in `changed_files`, and — unless `check` is `true` — written
  back with `std::fs::write` (files that don't need a fix are never touched, to avoid
  needless mtime churn). After the fix pass, when `check` is `false`,
  `lint::lint_bundle` is re-run against `root` (files on disk are now fixed) to
  populate `remaining` with whatever `fmt` couldn't or shouldn't fix — overlong lines
  inside frontmatter/code/headings/blockquotes (table rows don't appear here at all,
  since `check_style` itself exempts them) and all structural OKF rules. When `check`
  is `true`, `remaining` is left empty; `--check` mode is about "what would change",
  not full diagnostics.

## `src/cli.rs` / `src/main.rs`

- `Cli` now carries an optional `#[command(subcommand)] command: Option<Command>`
  alongside its original flat `path`/`--max-line-length` fields. When `command` is
  `None`, the flat fields are used — bare `okf-lint <path>` behaves exactly as before
  subcommands existed. `Command::Lint(LintArgs)` is the explicit spelling of the same
  thing. `Command::Fmt(FmtArgs)` adds `--tab-width <N>` (default 4) and `--check`
  (bool flag) on top of `path`/`--max-line-length`.
- `main()` dispatches on `cli.command`; a bare invocation with no `command` and no
  `path` prints a usage error and exits **2**.
- `print_diagnostics` was factored out of the old inline `main` loop so both `lint`'s
  output and `fmt`'s residual-diagnostics output share the exact same
  `{path}:{line}: {message}` formatting, spec-link suffix, and broken-pipe handling.
- `fmt` exit codes: **2** on `LintError` (same `format_error` path as `lint`). With
  `--check`: **0** if `changed_files` is empty (no output); otherwise one
  `would reformat: {path}` line per changed file on stdout, exit **1** — no files are
  written in this mode. Without `--check`: files are fixed in place first, then **0**
  if `remaining` is empty (silent, matching `lint`'s clean-bundle behavior); otherwise
  `remaining` is printed via `print_diagnostics`, exit **1**.

## Tests

- `src/checks/style_fix.rs` unit tests use fixture pairs under
  `tests/fixtures/fmt/<rule>/{before,after}.md`, one pair per pipeline stage above:
  `max_line_length_skip/before.md` (heading, table, fenced code, blockquote — each
  deliberately over `max_line_length` — byte-for-byte unchanged by `fix_style`),
  `max_line_length_list/{before,after}.md` (mixed bullet/ordered list, hanging-indent
  wrap), `max_line_length_link/{before,after}.md` (a paragraph with a link, the link
  staying one unsplit token), `max_line_length_adjacent_skip/{before,after}.md`
  (paragraphs directly adjacent to a skipped frontmatter/heading line, no blank line
  between, still get wrapped), plus idempotency/no-op checks.
- `src/fmt.rs` unit tests cover the same `LintError` cases as `lint_bundle`
  (nonexistent root, root-is-a-file) plus in-place fixing, `--check` leaving files
  untouched, and a clean bundle producing no changed files.
- `tests/cli_tests.rs` adds `fmt`-specific integration tests (in-place fix, residual
  diagnostics after fixing, `--check` mode, clean-bundle exit 0, nonexistent-path exit
  2) plus regression coverage that bare `okf-lint <path>` and explicit
  `okf-lint lint <path>` produce identical output — guarding the `Cli` subcommand
  restructure.
