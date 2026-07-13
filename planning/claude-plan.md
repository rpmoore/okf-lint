# Implementation Plan: okf-lint

## 0. Purpose of this document

This is a self-contained implementation plan for `okf-lint`, a Rust CLI
linter. It assumes no prior context: everything needed to implement the
tool — architecture, module responsibilities, exact rule semantics, CLI
contract, dependencies, and test layout — is spelled out below. It is a
blueprint, not code: types are given as field lists, functions as
signatures with docstrings. Full bodies are left to implementation.

Repo starting state: bare `cargo new` skeleton — `Cargo.toml` with no
dependencies, `src/main.rs` printing "Hello, world!", edition 2024.

## 1. What we're building and why

`okf-lint <path>` recursively checks every `.md` file under `<path>`
(a directory tree called an "OKF bundle") against two independent rule
sets:

1. **OKF v0.1 conformance** — structural rules from the Open Knowledge
   Format spec (frontmatter presence, required `type` field, correct
   `index.md`/`log.md` structure). Only MUST-level rules from the
   upstream spec's §9 conformance clause are enforced — SHOULD-level
   guidance (recommended fields, cross-links, citations) is explicitly
   out of scope.
2. **Markdown hygiene** — generic style rules (line length, trailing
   whitespace, trailing-newline discipline, consecutive blank lines,
   hard tabs) applied to every `.md` file regardless of its OKF role.

The tool is meant for CI: it prints one compiler-style diagnostic line
per violation (`path:line: message`) and communicates pass/fail via
exit code, so it can gate a merge.

## 2. Module layout

```
src/
  main.rs            # entry point: parse CLI, run linter, map result to exit code + stderr
  cli.rs             # Cli struct (clap derive): <path> positional, --max-line-length flag
  diagnostic.rs       # Diagnostic type, Rule enum (fixed ordering), sort/format helpers
  frontmatter.rs      # shared "---"-delimited block splitter, used by both okf.rs and index_md.rs
  walk.rs             # bundle traversal: sorted, hidden-skipping, non-symlink-following .md file list
  lint.rs             # orchestration: classify each file, dispatch to the right checks, collect + sort diagnostics
  checks/
    mod.rs
    okf.rs            # concept-document frontmatter checks (rules 1 and 2)
    index_md.rs        # index.md frontmatter-placement and body-structure checks (rules 3 and 4)
    log_md.rs           # log.md date-heading check (rule 5)
    style.rs            # the 5 markdown hygiene checks, applied to every file
tests/
  fixtures/           # one pass/ and fail/ mini-bundle per rule (see §10)
  cli_tests.rs        # assert_cmd-based CLI integration tests
```

Rationale for this split: `frontmatter.rs` is shared because both
concept documents (checks/okf.rs) and `index.md` (checks/index_md.rs)
need to detect/parse the same `---`-delimited block, just with
different rules about what's allowed to be inside it. Keeping the
parsing logic in one place avoids two slightly-different
frontmatter-detection implementations drifting apart.

## 3. Core data types

### `diagnostic.rs`

```
struct Diagnostic {
    line: usize,     // 1-based; see per-rule line-number rules in §5/§6
    rule: Rule,       // used only for sort tie-breaking and internal grouping, never printed
    message: String,  // exact text from §5/§6, already formatted (e.g. with {N} substituted)
}
```

```
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

`Rule`'s declaration order is the tie-break order used when two
diagnostics share the same `(file, line)` — see §7.

A per-file diagnostic collector pairs each `Diagnostic` with the file's
path (relative to the bundle root, using `/` separators regardless of
OS) for final formatting and sorting: `(relative_path: PathBuf,
Diagnostic)`.

### `frontmatter.rs`

```
enum FrontmatterResult {
    None,                    // content doesn't start with a "---" line
    Unclosed,                // starts with "---" but no closing "---" line found
    Found { yaml_text: String, body_start_line: usize },
}

fn split_frontmatter(content: &str) -> FrontmatterResult
```
**Docstring intent:** `content` must literally begin with a line that
is exactly `---` (no leading blank lines, no trailing characters on
that line) for `Found`/`Unclosed` to apply; anything else is `None`.
When `Found`, `yaml_text` is the raw text between the two `---`
delimiter lines (not yet parsed as YAML — that's the caller's job,
since callers want different things: `okf.rs` parses it fully,
`index_md.rs` just needs to know which keys are present).
`body_start_line` is the 1-based line number of the first line after
the closing `---` (used to offset later body-line diagnostics).

### `walk.rs`

```
fn collect_md_files(root: &Path) -> Result<Vec<PathBuf>, LintError>
```
**Docstring intent:** recurse under `root` with `walkdir`, default
(non-`follow_links`) settings. Skip any directory or file whose name
starts with `.` (do not descend into it at all). Filter to files with
a `.md` extension. Convert to paths relative to `root`. Sort the
resulting list lexicographically before returning, so downstream
diagnostic ordering (§7) is deterministic regardless of filesystem
iteration order. I/O errors while walking (e.g. a permission-denied
subdirectory) are mapped to `LintError::Io`.

### `lint.rs`

```
enum LintError {
    PathNotFound(PathBuf),
    NotADirectory(PathBuf),
    Io { path: PathBuf, source: std::io::Error },
    InvalidUtf8(PathBuf),
}

fn lint_bundle(root: &Path, max_line_length: usize) -> Result<Vec<(PathBuf, Diagnostic)>, LintError>
```
**Docstring intent:** validate `root` exists and is a directory (else
`PathNotFound`/`NotADirectory`), call `collect_md_files`, then for each
file: read to a `String` (a non-UTF-8 file yields `InvalidUtf8`, any
other read failure yields `Io`) — **any `LintError` here aborts the
whole run immediately, returning `Err` with no partial diagnostics**
(per §8's exit-code-2 contract: usage/IO errors short-circuit rather
than being reported as partial lint output). For each successfully-read
file: classify it (§4), run the style checks (§6) unconditionally, run
the classification-appropriate structural checks (§5), and accumulate
all resulting `(relative_path, Diagnostic)` pairs. After all files are
processed, sort the full diagnostic list per §7 and return it.

## 4. File classification

For each relative path from `collect_md_files`:
- File name exactly `index.md` → **Index**. It is the *root* index iff
  the relative path has no parent component (i.e. it is directly
  `index.md`, not `sub/index.md`).
- File name exactly `log.md` → **Log**.
- Anything else → **Concept**.

(Filename comparison is exact/case-sensitive, per the OKF spec's
reserved-filename definitions.)

## 5. OKF conformance checks

### 5.1 Concept documents — `checks/okf.rs`

```
fn check_concept(content: &str) -> Vec<Diagnostic>
```
**Docstring intent, rule 1 (`OkfMissingFrontmatter`):** call
`split_frontmatter`. `None` or `Unclosed` → emit one diagnostic, line
1, message `missing or invalid YAML frontmatter`, and stop (rule 2
cannot apply without parseable frontmatter). `Found` → attempt to parse
`yaml_text` as YAML via `serde_yaml_ng`; a parse error, or a
successfully-parsed value that isn't a YAML mapping, is *also* rule 1
(same message, same line 1) — "unparseable" covers both syntactic YAML
errors and structurally-wrong-shape frontmatter (e.g. a YAML scalar or
list instead of a mapping).

**Rule 2 (`OkfMissingType`):** only reached if rule 1 did not fire.
Look up the `type` key in the parsed mapping. Missing key, or present
with an empty string, or present with a non-string YAML value (number,
bool, list, mapping, null) → emit one diagnostic, line 1, message
`frontmatter missing required non-empty 'type' field`.

### 5.2 `index.md` — `checks/index_md.rs`

```
fn check_index(content: &str, is_root: bool) -> Vec<Diagnostic>
```

**Rule 3 (`OkfIndexFrontmatterPlacement`):** call `split_frontmatter`.
- `None` → no violation, proceed to rule 4 using the whole `content` as
  the body (no offset).
- `Found` or `Unclosed` on a **non-root** index.md → emit one
  diagnostic, line 1, message `index.md must not contain frontmatter`.
  (An `Unclosed` block is treated the same as `Found` here: the file
  visibly starts a frontmatter block, which is itself the violation,
  regardless of whether it's syntactically well-formed — this is a
  plan-level decision filling a gap the spec didn't address, since the
  original spec only describes the fully-formed case.)
- `Found` on the **root** index.md → parse `yaml_text` as a YAML
  mapping. If parsing fails, or the mapping contains any key other than
  `okf_version`, emit one diagnostic, line 1, message `root index.md
  frontmatter may only contain 'okf_version'`. A mapping containing
  only `okf_version` (or an empty mapping) is fine.
- `Unclosed` on the **root** index.md → since it can't be parsed as a
  mapping at all, treat it as violating the same rule: emit `root
  index.md frontmatter may only contain 'okf_version'` at line 1 (same
  gap-filling rationale as above).

For rule 4, the body to scan is: the whole `content` if
`split_frontmatter` returned `None`; otherwise the text starting at
`body_start_line` (only reached when frontmatter placement didn't
already produce a rule-3 violation that makes the body irrelevant —
note rule 3 and rule 4 are independent checks and **both can fire** on
the same file, e.g. a non-root index.md with both bad frontmatter and a
stray paragraph; §7 already establishes the fixed ordering between
them).

**Rule 4 (`OkfIndexBodyStructure`):** scan the body line by line,
1-indexed starting from `body_start_line` (or 1). Maintain a single
boolean `in_list_item` (starts `false`, reset to `false` on every blank
line). For each non-blank line, in order:
- Matches `^#+ ` (one or more `#` then a space) → heading; valid, set
  `in_list_item = false`.
- Matches `^[*+-] ` (list marker then a space) → list item; valid, set
  `in_list_item = true`.
- Otherwise, if `in_list_item` is `true` **and** the line has at least
  2 leading space characters → continuation line; valid, `in_list_item`
  stays `true`.
- Otherwise → violation: emit a diagnostic at this line's number,
  message `index.md body line is not a heading or list item`;
  `in_list_item` becomes `false` (a violating line doesn't count as
  part of a list item for the purposes of a *following* line's
  continuation check).

Each violating line gets its own diagnostic (a multi-line stray
paragraph produces one diagnostic per line, not one per paragraph) —
consistent with the "one diagnostic per violated line" philosophy used
elsewhere (§7).

### 5.3 `log.md` — `checks/log_md.rs`

```
fn check_log(content: &str) -> Vec<Diagnostic>
```
**Docstring intent (rule 5, `OkfLogDateHeading`):** `log.md` has no
frontmatter handling in this linter (not one of the 5 conformance
rules) — scan the *entire* `content` (not body-offset) line by line for
lines matching exactly `^## (.*)$` (heading level exactly 2 — a `#`, or
a `###`+, heading is not inspected by this rule at all). For each such
heading line, take the captured text and validate it is exactly four
digits, `-`, two digits, `-`, two digits (regex `^\d{4}-\d{2}-\d{2}$`)
**and** parses as a real calendar date (reject e.g. `2026-02-30`). Use
the `chrono` crate's `NaiveDate::parse_from_str(text, "%Y-%m-%d")` for
calendar validation rather than hand-rolling leap-year math — this is a
small addition beyond what research/interview covered, justified by
being the standard, well-tested way to validate a calendar date in
Rust. A match failure (either the regex or the calendar parse) emits
one diagnostic at that heading's line number, message `log.md heading
is not a valid YYYY-MM-DD date`.

## 6. Markdown style checks — `checks/style.rs`

```
fn check_style(content: &str, max_line_length: usize) -> Vec<Diagnostic>
```

Applies uniformly to every `.md` file (Concept, Index, and Log alike),
independent of the OKF checks in §5.

Split `content` on `\n`. If `content` ends with `\n`, the split
produces one trailing empty string that is **not** a real line — drop
it before running the per-line checks below (rules 1, 2, 5). The
trailing-newline check (rule 3) instead inspects the raw content
directly (see below), not the split-line list.

1. **`StyleLineLength`.** For each real line (1-indexed), if
   `line.chars().count() > max_line_length` (Unicode scalar count, not
   byte length — a 2-byte UTF-8 character like `é` counts as 1), emit
   a diagnostic at that line, message `line exceeds maximum length of
   {max_line_length} characters ({actual} found)` with `{actual}` being
   the counted length.
2. **`StyleTrailingWhitespace`.** For each real line, if it ends with
   one or more of: space, tab, or `\r` (the `\r` case handles
   CRLF-terminated input, since we only split on `\n` — a CRLF line
   ends with `\r` after the split, which this rule treats identically
   to trailing spaces/tabs, effectively enforcing LF-only files), emit
   a diagnostic at that line, message `line has trailing whitespace`.
3. **`StyleTrailingNewline`.** Inspect `content` directly (not the
   split list): a 0-byte file is a violation (message below, line 1).
   Otherwise, violation if `content` does not end with exactly one
   `\n` — i.e. it doesn't end with `\n` at all, or it ends with `\n\n`
   (two or more trailing newlines, meaning trailing blank lines).
   Diagnostic at line 1, message `file must end with exactly one
   trailing newline`.
4. **`StyleConsecutiveBlankLines`.** Walk the real lines tracking a
   run-length counter of consecutive lines that are empty or
   whitespace-only (reset to 0 on any non-blank line). The rule is
   violated once a run reaches length 2; emit exactly **one** diagnostic
   per run, anchored at the line number of the *second* blank line in
   that run (the point at which "two or more consecutive" first
   becomes true) — not one diagnostic per additional blank line in a
   longer run. Message: `multiple consecutive blank lines`. (This
   one-diagnostic-per-run anchoring is a plan-level judgment call: the
   spec didn't define how many diagnostics a 5-blank-line run should
   produce, and one-per-run avoids diagnostic-count explosion for large
   gaps while still pinpointing where the run starts being invalid.)
5. **`StyleHardTab`.** For each real line, if it contains a `\t`
   character anywhere in its content (not just trailing — a tab
   mid-line also counts, and can co-fire with rule 2 if the tab is also
   trailing), emit a diagnostic at that line, message `line contains a
   hard tab character`.

All five checks run independently over the same line set — a single
line can produce diagnostics for multiple rules (e.g. rule 1 and rule
2 both firing on an over-length line with trailing whitespace).

## 7. Diagnostic ordering and formatting

After `lint_bundle` collects every `(relative_path, Diagnostic)` pair
across all files, sort the full list by:
1. `relative_path`, lexicographically (matches the traversal sort in
   `walk.rs`, so this is really just "stable regardless of walk
   order").
2. `Diagnostic.line`, ascending.
3. `Diagnostic.rule`'s declaration order in the `Rule` enum (§3) as a
   tie-break when the same file and line have multiple diagnostics —
   OKF conformance rules sort before style rules, and within each
   group, in the fixed order listed in §3.

Format each as: `{relative_path}:{line}: {message}` (relative_path uses
`/` separators). Print one per line to stdout, in this sorted order.

## 8. CLI — `cli.rs` and `main.rs`

```
#[derive(clap::Parser)]
struct Cli {
    path: std::path::PathBuf,                      // positional, the bundle root
    #[arg(long, default_value_t = 100)]
    max_line_length: u32,
}
```

`main.rs` responsibilities:
- Parse `Cli` via `clap`.
- Call `lint::lint_bundle(&cli.path, cli.max_line_length as usize)`.
- On `Err(LintError)`: print a human-readable message to **stderr**
  (not in `file:line:` diagnostic format — these are usage/IO errors,
  not lint findings) and exit with code **2**. This is checked before
  any diagnostics are printed — the run is aborted at the first such
  error, no partial diagnostic output is produced.
- On `Ok(diagnostics)`: print each formatted diagnostic (§7) to stdout.
  Exit **0** if `diagnostics` is empty, else exit **1**.

Exit code summary: `0` clean, `1` one-or-more lint diagnostics, `2`
usage/IO error (bad `<path>`, unreadable file, non-UTF-8 file content).

Flags: `--max-line-length <N>` (default 100) is the only flag. No other
CLI surface — no JSON output, no config file, no `--fix`, no ignore
globs (all explicitly out of scope, see §11).

## 9. Dependencies (`Cargo.toml` additions)

- `clap` (4.x, `derive` feature) — CLI parsing (§8).
- `walkdir` (2.x) — directory traversal (`walk.rs`).
- `serde_yaml_ng` (0.10.x) — frontmatter YAML parsing (`checks/okf.rs`,
  `checks/index_md.rs`). Chosen over `serde-saphyr` (newer but pre-1.0
  with expected API churn) and the archived original `serde_yaml`
  (deprecated) / `serde_yml` fork (RUSTSEC-2025-0068).
- `chrono` — calendar-date validation for `log.md` headings
  (`checks/log_md.rs`), specifically `NaiveDate::parse_from_str`.

Dev-dependencies (tests only):
- `assert_cmd` — run the compiled binary and assert exit codes/output
  in `tests/cli_tests.rs`.
- `predicates` — composable stdout/stderr assertions alongside
  `assert_cmd`.
- `insta` — snapshot testing for the whole-bundle integration test's
  multi-line diagnostic output (§10), reviewed via `cargo insta
  review`.

## 10. Testing strategy

Layout:
```
tests/
  fixtures/
    okf/
      missing_frontmatter/{pass/pass.md, fail/fail.md}
      missing_type/{pass/pass.md, fail/fail.md}
      index_frontmatter_placement/{pass_root/, fail_nonroot/, fail_root_extra_key/}
      index_body_structure/{pass/pass.md, fail/fail.md}
      log_date_heading/{pass/pass.md, fail/fail.md}
    style/
      max_line_length/{pass/pass.md, fail/fail.md}
      trailing_whitespace/{pass/pass.md, fail/fail.md}
      trailing_newline/{pass/pass.md, fail/fail.md}
      consecutive_blank_lines/{pass/pass.md, fail/fail.md}
      hard_tabs/{pass/pass.md, fail/fail.md}
    integration_bundle/       # a small multi-file tree: root index.md,
                               # a subdirectory with its own index.md,
                               # a couple of concept docs, a log.md —
                               # combining passing and failing cases
  cli_tests.rs
```

Each `pass/`/`fail/` pair under `okf/` and `style/` is itself two
single-file mini-bundles: the CLI is run with `pass/` or `fail/` as
`<path>` (not their shared parent), so exactly one file is checked per
test, isolating the rule under test. Putting `pass.md` and `fail.md`
as sibling files in the same directory would make a directory-rooted
CLI run check both at once, defeating the isolation — hence the
one-file-per-subdirectory split. The `pass_root/`, `fail_nonroot/`, and
`fail_root_extra_key/` fixtures under `index_frontmatter_placement/`
are small directories (root `index.md` plus, where relevant, a
subdirectory `index.md`) since rule 3's root-vs-non-root distinction
needs more than one file to exercise meaningfully.

Test coverage (mirrors spec §5/§8, `claude-spec.md`):
- Unit tests (inline `#[cfg(test)]` in each `checks/*.rs` module):
  directly call `check_concept` / `check_index` / `check_log` /
  `check_style` against fixture file contents (or inline string
  literals for trivial cases), asserting the exact `Diagnostic` set
  produced — one test per pass/fail fixture pair, per rule.
- Integration test (`tests/cli_tests.rs`, using `integration_bundle/`):
  run the full binary against the whole fixture tree, assert (via
  `insta::assert_snapshot!`) the complete sorted diagnostic output
  matches an approved snapshot.
- CLI exit-code tests (`tests/cli_tests.rs`, using `assert_cmd`):
  assert exit code `0` against a clean fixture bundle, `1` against a
  bundle with at least one violation, and `2` against a nonexistent
  `<path>`.
- `--max-line-length` override test: run against a fixture whose only
  violation is a line between the default (100) and a raised custom
  limit, asserting no diagnostic is produced when the custom limit is
  passed, and one is produced with the default.

## 11. Non-goals (explicit — do not implement without a new user decision)

- SHOULD-level OKF checks: recommended frontmatter fields
  (title/description/timestamp), cross-link validity, citation
  formatting, tag conventions, any `okf_version` enforcement beyond the
  root-only placement rule in §5.2.
- Autofix / `--fix` mode.
- JSON or other structured/machine-readable output formats.
- Configuration file support (`.okflintrc` or similar) — flags only.
- Ignore patterns / per-file or per-directory suppression.
- Markdown heading-structure linting (skipped levels, duplicate H1s).

## 12. Judgment calls made by this plan (flagged for visibility)

The spec + interview left a few gaps this plan had to close by
judgment rather than an explicit prior decision — listed here so a
reviewer can override them if they disagree:
- Treating an `Unclosed` frontmatter block in `index.md` the same as a
  `Found` one for rule 3 purposes (§5.2).
- Anchoring `StyleConsecutiveBlankLines` at the second blank line of a
  run, one diagnostic per run rather than per line (§6.4).
- Adding `chrono` as a dependency for calendar-date validation in
  `log.md` (§5.3) — not covered by the research/interview crate list.
- The fixed `Rule` enum ordering used as a same-line tie-break (§3, §7)
  — OKF rules before style rules, in the order both are listed
  throughout this document.
