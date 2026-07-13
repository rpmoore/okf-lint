---
type: module
---

# CLI

The binary entry point: parses arguments, calls `lint::lint_bundle`, and translates the
result into stdout/stderr output and a process exit code. Everything else (walking,
checks, sorting) belongs to earlier sections — this layer only formats and exits.

## `src/cli.rs`

- `Cli` (`clap::Parser`) — `path: PathBuf` (positional, the bundle root) and
  `--max-line-length <N>` (`u32`, `default_value_t = 100`). No other flags — no JSON
  output, config file, `--fix`, or ignore globs.

## `src/main.rs`

- `main() -> ExitCode`:
  1. `Cli::parse()`.
  2. `lint::lint_bundle(&cli.path, cli.max_line_length as usize)`.
  3. On `Err(LintError)`: `format_error` renders a human-readable one-line message to
     stderr, exit **2**. Returns immediately — no stdout is written on this path, even
     though `lint_bundle` itself already guarantees no partial results on `Err`.
  4. On `Ok(diagnostics)`: if empty, exit **0** with no output. Otherwise write each
     diagnostic to stdout as `{path}:{line}: {message}` (via a single locked
     `io::stdout()` handle, not per-line `println!`) and exit **1**.
- Diagnostics are printed in the order `lint_bundle` returns them (already sorted) — no
  re-sorting here. `path.display()` is used directly; this relies on the project's
  Unix-only assumption (paths use `/` natively) rather than special-casing separators.
- Broken pipe handling: writing diagnostics uses `writeln!` on a locked stdout handle
  rather than `println!`, so that when the write fails with `ErrorKind::BrokenPipe`
  (e.g. `okf-lint dir | head`) the process exits cleanly with code 1 instead of
  panicking. Any other write error exits 2 with a stderr message.
- `format_error` maps each `LintError` variant to a stderr message. Note
  `LintError::PathNotFound` covers both "doesn't exist" and "exists but unreadable"
  (permission-denied) cases — `lint_bundle` collapses both into that one variant via a
  single `std::fs::metadata` call — so the message text is deliberately the neutral
  "cannot access path: {path}" rather than an assertion that the path is missing.

## Tests: `tests/cli_tests.rs`

Integration tests using `assert_cmd`/`predicates` against the compiled binary (not
library-level unit tests): nonexistent path (exit 2, non-empty stderr, empty stdout),
clean bundle (exit 0, empty stdout/stderr), a bundle with a known violation (exit 1,
diagnostic text on stdout, empty stderr), and `--max-line-length` override behavior
(including a regression check that the default flag value produces output identical to
passing `--max-line-length 100` explicitly). All success/violation-path tests also
assert stderr is empty, since a stray debug print on the happy path wouldn't otherwise
be caught.

The `--max-line-length` override test uses a dedicated fixture,
`tests/fixtures/cli/max_line_length_override/fail.md` — frontmatter + heading + a single
120-character body line, otherwise clean under every other style/OKF rule so no other
diagnostic pollutes the assertion.

`tests/cli_tests.rs` is also touched by section-08-integration-tests, which appends a
whole-bundle `insta`-snapshot test to this same file.
