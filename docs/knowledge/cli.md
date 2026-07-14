---
type: module
---

# CLI

The binary entry point: parses arguments, dispatches to `lint::lint_bundle` or
`fmt::run_fmt`, and translates the result into stdout/stderr output and a process exit
code. Everything else (walking, checks, sorting, fixing) belongs to earlier
sections/`docs/knowledge/fmt.md` — this layer only formats and exits.

## `src/cli.rs`

- `Cli` (`clap::Parser`) — `#[command(subcommand)] command: Option<Command>`, plus a
  flat `path: Option<PathBuf>` and `--max-line-length <N>` (`u32`, default 100) used
  only when `command` is `None`. This keeps bare `okf-lint <path>` working exactly as
  it did before subcommands existed, as an implicit `lint`.
- `Command` (`clap::Subcommand`): `Lint(LintArgs)` and `Fmt(FmtArgs)`.
  - `LintArgs` — `path: PathBuf`, `--max-line-length <N>` (default 100). Identical
    shape to the pre-subcommand flat `Cli`.
  - `FmtArgs` — `path: PathBuf`, `--max-line-length <N>` (default 100),
    `--tab-width <N>` (`u32`, default 4, spaces per hard tab when expanding),
    `--check` (bool flag, report-only — see `docs/knowledge/fmt.md`).

## `src/main.rs`

- `main() -> ExitCode` dispatches on `cli.command`: `Some(Command::Lint(args))` and
  `None` (with `cli.path` present) both call `run_lint`; `None` with no `cli.path`
  prints a usage error to stderr and exits **2**; `Some(Command::Fmt(args))` calls
  `run_fmt_command`.
- `run_lint(path, max_line_length) -> ExitCode`:
  1. `lint::lint_bundle(path, max_line_length)`.
  2. On `Err(LintError)`: `format_error` renders a human-readable one-line message to
     stderr, exit **2**. Returns immediately — no stdout is written on this path, even
     though `lint_bundle` itself already guarantees no partial results on `Err`.
  3. On `Ok(diagnostics)`: if empty, exit **0** with no output. Otherwise
     `print_diagnostics`, exit **1**.
- `print_diagnostics(diagnostics) -> Result<(), ExitCode>` — factored out of the old
  inline `main` loop so both `run_lint` and `run_fmt_command` (for its residual
  diagnostics) share identical output formatting. Writes each diagnostic to stdout as
  `{path}:{line}: {message}` via a single locked `io::stdout()` handle, not per-line
  `println!`.
- If `diagnostic.rule.spec_url()` returns `Some(url)`, the line gets a trailing
  `" (spec: {url})"` pointing at the exact OKF SPEC.md section the rule enforces
  (e.g. `#41-frontmatter`, `#6-index-files`, `#7-log-files-optional`). Style-rule
  diagnostics (line length, trailing whitespace, etc.) never get this suffix — they
  aren't OKF requirements, so there's no spec section to link to. See
  `docs/knowledge/foundation.md` for `Rule::spec_url`.
- Diagnostics are printed in the order `lint_bundle` returns them (already sorted) — no
  re-sorting here. `path.display()` is used directly; this relies on the project's
  Unix-only assumption (paths use `/` natively) rather than special-casing separators.
- Broken pipe handling: writing diagnostics uses `writeln!` on a locked stdout handle
  rather than `println!`, so that when the write fails with `ErrorKind::BrokenPipe`
  (e.g. `okf-lint dir | head`) the process exits cleanly with code 1 instead of
  panicking. Any other write error exits 2 with a stderr message. `run_fmt_command`'s
  `would reformat: {path}` output (its `--check` mode) uses the same broken-pipe
  handling directly, not `print_diagnostics`.
- `format_error` maps each `LintError` variant to a stderr message. Note
  `LintError::PathNotFound` covers both "doesn't exist" and "exists but unreadable"
  (permission-denied) cases — `lint_bundle` collapses both into that one variant via a
  single `std::fs::metadata` call — so the message text is deliberately the neutral
  "cannot access path: {path}" rather than an assertion that the path is missing.
  `run_fmt_command` reuses `format_error` for the same `LintError` from `run_fmt`.
- `run_fmt_command` (see `docs/knowledge/fmt.md` for `fmt::run_fmt` itself) maps
  `FmtOutcome` to exit codes: `--check` mode exits 0 (no changed files) or 1 (prints
  `would reformat:` lines, writes nothing); default mode fixes files in place then
  exits 0 (no remaining diagnostics) or 1 (prints them via `print_diagnostics`).

## Tests: `tests/cli_tests.rs`

Integration tests using `assert_cmd`/`predicates` against the compiled binary (not
library-level unit tests): nonexistent path (exit 2, non-empty stderr, empty stdout),
clean bundle (exit 0, empty stdout/stderr), a bundle with a known violation (exit 1,
diagnostic text on stdout, empty stderr), and `--max-line-length` override behavior
(including a regression check that the default flag value produces output identical to
passing `--max-line-length 100` explicitly). All success/violation-path tests also
assert stderr is empty, since a stray debug print on the happy path wouldn't otherwise
be caught.

Also covers the `Cli` subcommand restructure and `fmt`: bare `okf-lint <path>` vs.
explicit `okf-lint lint <path>` produce identical stdout/exit code (regression guard for
the optional-subcommand-with-flat-fallback shape); `okf-lint fmt <path>` fixes a
`TempDir`-copied fixture in place and reports remaining (unfixable) diagnostics after
fixing; `okf-lint fmt <path> --check` reports `would reformat:` without writing;
`fmt` on a clean bundle and on a nonexistent path mirror `lint`'s exit codes. `fmt`
tests write to `tempfile::TempDir` copies rather than the checked-in fixtures, since
`fmt` mutates files in place.

The `--max-line-length` override test uses a dedicated fixture,
`tests/fixtures/cli/max_line_length_override/fail.md` — frontmatter + heading + a single
120-character body line, otherwise clean under every other style/OKF rule so no other
diagnostic pollutes the assertion.

`tests/cli_tests.rs` is also touched by section-08-integration-tests, which appends a
whole-bundle `insta`-snapshot test to this same file.
