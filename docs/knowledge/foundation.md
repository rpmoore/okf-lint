---
type: module
---

# Foundation

The dependency-free building blocks every `okf-lint` check module and the
orchestration layer import.

## `src/diagnostic.rs`

- `Rule` — enum of every check rule (5 OKF conformance + 5 markdown style),
  in a fixed declaration order. That order is load-bearing: it is the
  tie-break used when two diagnostics land on the same `(file, line)`.
- `Rule::spec_url(&self) -> Option<&'static str>` — the OKF SPEC.md section
  (with anchor) a given OKF rule enforces, e.g. `OkfMissingFrontmatter` and
  `OkfMissingType` both point at `#41-frontmatter`. Returns `None` for the 5
  style rules, since those are project convention rather than OKF-derived —
  the `None` is itself meaningful output (see `src/main.rs`).
- `Diagnostic { line, rule, message }` — one lint finding.

## `src/frontmatter.rs`

- `split_frontmatter(content: &str) -> FrontmatterResult` — detects and
  extracts a `---`-delimited YAML frontmatter block. Returns `None` (no
  block), `Unclosed` (opening `---` with no closing `---`), or
  `Found { yaml_text, body_start_line }`.
- The opening/closing `---` delimiter comparison tolerates a trailing `\r`
  (`strip_cr`) so CRLF-terminated files are detected the same as
  LF-terminated ones. Every line pushed into `yaml_text` is also passed
  through `strip_cr`, not just the delimiter lines — otherwise CRLF input
  would leave embedded `\r` characters inside the extracted YAML payload,
  making downstream YAML parsing depend on how the parser treats stray
  carriage returns instead of behaving identically to LF input.
- Shared by the concept-document checker and the `index.md` checker, since
  both need the same delimiter-detection logic with different rules about
  what's allowed inside.

## `src/lint.rs`

- `LintError` — the crate's top-level error type (`PathNotFound`,
  `NotADirectory`, `Io`, `InvalidUtf8`). Only a stub in this section; the
  `Io` variant is used by `walk.rs`. The orchestration function
  (`lint_bundle`) that constructs the other variants is added later.

## `src/walk.rs`

- `collect_md_files(root: &Path, include_hidden: bool) -> Result<Vec<PathBuf>, LintError>`
  — recursively collects every `.md` file under `root`, returning root-relative
  paths sorted lexicographically for deterministic diagnostic ordering. By
  default (`include_hidden = false`), dot-prefixed files/directories below
  `root` (depth > 0) are excluded via `WalkDir::filter_entry` — per the
  planning spec's traversal contract (`planning/claude-spec.md` §5, interview
  Q2), hidden entries like `.git` or `.github` are skipped entirely and never
  descended into (`filter_entry` prunes them, so this isn't just a post-hoc
  filter — it avoids walking heavy hidden trees like `.git` at all). `root`
  itself is never treated as hidden even if its own path is dot-prefixed
  (e.g. a tempdir in tests). Passing `include_hidden = true` disables the
  prune, walking hidden entries too — surfaced as the CLI's
  `--include-hidden` flag (`docs/knowledge/cli.md`), threaded through
  `lint_bundle` and `run_fmt` down to this function.
