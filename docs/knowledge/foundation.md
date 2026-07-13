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
- `Diagnostic { line, rule, message }` — one lint finding.

## `src/frontmatter.rs`

- `split_frontmatter(content: &str) -> FrontmatterResult` — detects and
  extracts a `---`-delimited YAML frontmatter block. Returns `None` (no
  block), `Unclosed` (opening `---` with no closing `---`), or
  `Found { yaml_text, body_start_line }`.
- The opening/closing `---` delimiter comparison tolerates a trailing `\r`
  so CRLF-terminated files are detected the same as LF-terminated ones.
- Shared by the concept-document checker and the `index.md` checker, since
  both need the same delimiter-detection logic with different rules about
  what's allowed inside.

## `src/lint.rs`

- `LintError` — the crate's top-level error type (`PathNotFound`,
  `NotADirectory`, `Io`, `InvalidUtf8`). Only a stub in this section; the
  `Io` variant is used by `walk.rs`. The orchestration function
  (`lint_bundle`) that constructs the other variants is added later.

## `src/walk.rs`

- `collect_md_files(root: &Path) -> Result<Vec<PathBuf>, LintError>` —
  recursively collects every `.md` file under `root`, skipping any
  dotfile/dot-directory entirely, returning root-relative paths sorted
  lexicographically for deterministic diagnostic ordering.
