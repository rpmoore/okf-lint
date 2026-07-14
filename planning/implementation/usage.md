# Usage Guide: okf-lint

A Rust CLI that recursively lints every `.md` file in an OKF (Open Knowledge Format)
bundle against 5 OKF-conformance rules and 5 generic markdown-hygiene rules, emitting
compiler-style diagnostics for CI use.

## Quick Start

```bash
cargo build --release
./target/release/okf-lint path/to/bundle
```

Exit codes:
- `0` — clean, no diagnostics.
- `1` — one or more lint violations found (diagnostics printed to stdout).
- `2` — usage/IO error (bad path, unreadable file, non-UTF-8 content) — message on stderr, stdout empty.

Flags:
- `<path>` (positional, required) — the bundle root directory to lint.
- `--max-line-length <N>` (default `100`) — override the max line length for the style check.

```bash
okf-lint docs/okf-bundle --max-line-length 120
```

## Example Output

```
$ okf-lint tests/fixtures/integration_bundle
concept-a.md:7: line has trailing whitespace
concept-b.md:1: missing or invalid YAML frontmatter (spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#41-frontmatter)
log.md:7: log.md heading is not a valid YYYY-MM-DD date (spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#7-log-files-optional)
sub/index.md:5: index.md body line is not a heading or list item (spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#6-index-files)
$ echo $?
1
```

Each line is `{relative_path}:{line}: {message}`, sorted by path, then line, then rule
(OKF rules before style rules). Paths are relative to the bundle root you passed in.
OKF-rule violations carry a trailing `(spec: {url})` pointing at the exact OKF SPEC.md
section they violate; style-rule violations never do, since those rules aren't OKF
requirements (see Rules tables below).

## Rules

OKF conformance (fires per-file based on classification: `index.md` at bundle root vs.
nested, `log.md`, or any other `.md` = a "concept" doc):

| Rule | Applies to | Trigger | Spec link |
|---|---|---|---|
| `OkfMissingFrontmatter` | concept docs | no/invalid YAML frontmatter block | `#41-frontmatter` |
| `OkfMissingType` | concept docs | frontmatter present but missing/empty `type` | `#41-frontmatter` |
| `OkfIndexFrontmatterPlacement` | `index.md` | root: frontmatter has keys other than `okf_version`; nested: any frontmatter at all | `#6-index-files` |
| `OkfIndexBodyStructure` | `index.md` | body line isn't a heading, list item, or list continuation; or a list item appears before any heading | `#6-index-files` |
| `OkfLogDateHeading` | `log.md` | a `## ` heading isn't a valid `YYYY-MM-DD` calendar date, or valid dates aren't newest-first | `#7-log-files-optional` |

Every OKF diagnostic's stdout line ends with `(spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md{anchor})`.

Markdown style (applies uniformly to every `.md` file, regardless of OKF role — none of
these are OKF spec requirements, so none carry a spec link):

| Rule | Trigger |
|---|---|
| `StyleLineLength` | line exceeds `--max-line-length` (default 100) |
| `StyleTrailingWhitespace` | line ends with a space/tab/`\r` |
| `StyleTrailingNewline` | file doesn't end with exactly one trailing `\n` |
| `StyleConsecutiveBlankLines` | 2+ consecutive blank lines |
| `StyleHardTab` | line contains a literal tab character |

## API Reference (library internals, if embedding)

- `lint::lint_bundle(root: &Path, max_line_length: usize) -> Result<Vec<(PathBuf, Diagnostic)>, LintError>`
  — the whole-pipeline entry point: walks `root`, classifies each `.md` file, runs the
  applicable checks, and returns a fully sorted diagnostic list. `LintError` variants:
  `PathNotFound`, `NotADirectory`, `Io { path, source }`, `InvalidUtf8`.
- `diagnostic::Diagnostic { line, rule, message }` / `diagnostic::Rule` (10-variant enum,
  `Ord` reflects the fixed OKF-then-style declaration order used for tie-breaks).
- `walk::collect_md_files(root: &Path) -> Result<Vec<PathBuf>, LintError>` — sorted,
  root-relative `.md` file discovery. Does **not** skip dot-directories (e.g. `.hidden/`) —
  the OKF spec has no hidden-file exception, so hidden concept docs are checked too.
- `frontmatter::split_frontmatter(content: &str) -> FrontmatterResult` — shared YAML
  frontmatter block detection (`None` / `Found { yaml_text, body_start_line }` / `Unclosed`).
- `checks::{okf, index_md, log_md, style}` — one module per rule family; each exposes a
  `check_*(content, ...) -> Vec<Diagnostic>` function.

See `docs/knowledge/*.md` for detailed per-module documentation (one doc per section:
foundation, concept-checks, index-checks, log-checks, style-checks, orchestration, cli,
integration-tests).

## Testing

```bash
cargo test          # 84 tests: unit tests per module + CLI integration tests
cargo clippy --all-targets
```

The integration snapshot test (`tests/cli_tests.rs`) uses `insta` — if you ever need to
regenerate it after an intentional behavior change, install `cargo-insta` and run
`cargo insta review` (this environment didn't have the CLI installed, so the current
snapshot was hand-verified and promoted manually; see
`planning/implementation/code_review/section-08-review.md` for how).
