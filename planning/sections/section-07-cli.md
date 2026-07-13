# section-07-cli: CLI entry point (`cli.rs`, `main.rs`)

## Dependencies

This section requires:
- **section-01-foundation**: `Cargo.toml` must already have the `clap` (4.x, `derive` feature) dependency added, plus dev-dependencies `assert_cmd` and `predicates` (needed for this section's tests). It also provides `diagnostic.rs`'s `Diagnostic`/`Rule` types used indirectly via `lint::lint_bundle`'s return type.
- **section-06-orchestration**: provides `lint::lint_bundle(root: &Path, max_line_length: usize) -> Result<Vec<(PathBuf, Diagnostic)>, LintError>` and the `LintError` enum (`PathNotFound(PathBuf)`, `NotADirectory(PathBuf)`, `Io { path: PathBuf, source: std::io::Error }`, `InvalidUtf8(PathBuf)`), and the diagnostic sort ordering (already applied by `lint_bundle` before returning) plus the `{relative_path}:{line}: {message}` formatting convention (relative_path uses `/` separators).

Do not reimplement any of the above — this section only wires the CLI/entry-point layer on top of it.

## What this section builds

Two files:
- `src/cli.rs` — the `clap`-derived `Cli` argument struct.
- `src/main.rs` — the binary entry point: parse `Cli`, call `lint::lint_bundle`, map the result to stdout/stderr output and a process exit code.

Plus `tests/cli_tests.rs` — CLI-level integration tests using `assert_cmd` and `predicates`, exercising the compiled binary directly (not library-level unit tests). Note: `tests/cli_tests.rs` is also touched by section-08-integration-tests, which adds the whole-bundle `insta`-snapshot test to the same file — this section should create the file with the tests listed below; section-08 will append to it later.

## Background: the CLI contract

`okf-lint <path>` recursively checks every `.md` file under `<path>` (a directory tree called an "OKF bundle") against OKF-conformance rules and generic markdown-hygiene rules. It is meant for CI use: it prints one compiler-style diagnostic line per violation (`path:line: message`) to stdout, and communicates pass/fail via process exit code.

## Implementation details

### `src/cli.rs`

```rust
#[derive(clap::Parser)]
struct Cli {
    path: std::path::PathBuf,                      // positional, the bundle root
    #[arg(long, default_value_t = 100)]
    max_line_length: u32,
}
```

That's the entire CLI surface: a single positional `<path>` argument (the bundle root to lint) and a single `--max-line-length <N>` flag (default `100`). No other flags — no JSON output, no config file, no `--fix`, no ignore globs (all explicitly out of scope per the plan's non-goals). The struct/fields should be `pub` as needed so `main.rs` can use them (or kept in the same crate root — whichever matches how section-01 structured `src/main.rs`'s module declarations, e.g. `mod cli;`).

### `src/main.rs`

Responsibilities, in order:
1. Parse `Cli` via `clap` (`Cli::parse()`).
2. Call `lint::lint_bundle(&cli.path, cli.max_line_length as usize)`.
3. On `Err(LintError)`: print a human-readable message to **stderr** (not in `file:line:` diagnostic format — these are usage/IO errors, not lint findings) and exit with process code **2**. This check happens before any diagnostics are printed — the run is aborted at the first such error, and **no partial diagnostic output is produced** (this matters even though `lint_bundle` itself already guarantees no partial results are returned on `Err`; `main.rs` must not print anything to stdout in this branch).
4. On `Ok(diagnostics)`: print each formatted diagnostic to stdout, one per line, in the order `lint_bundle` returned them (already sorted per the orchestration section's ordering rules — no re-sorting needed here). Format is `{relative_path}:{line}: {message}` as already produced by the diagnostic formatting helper from earlier sections; do not reformat manually if a formatter already exists (e.g. a `Display` impl on `Diagnostic` or a helper function) — otherwise construct the string directly matching that exact format. Exit **0** if `diagnostics` is empty, else exit **1**.

Exit code summary:
- `0` — clean, no diagnostics.
- `1` — one or more lint diagnostics were found (diagnostics printed to stdout).
- `2` — usage/IO error: bad `<path>` (doesn't exist, or exists but isn't a directory), unreadable file, or non-UTF-8 file content (error message on stderr, stdout empty).

The human-readable stderr messages for each `LintError` variant are not prescribed exactly by the plan beyond "human-readable" — pick clear, one-line messages (e.g. for `PathNotFound(path)`, something like `error: path does not exist: {path}`). Tests below only assert that stderr is non-empty / exit code 2 / stdout empty for the not-found case, not an exact message string, so exact wording is an implementation choice.

## Tests (write these first)

Create `tests/cli_tests.rs`. Use `assert_cmd::Command::cargo_bin("okf-lint")` to invoke the compiled binary, and `predicates` for composable stdout/stderr/exit-code assertions.

- **Test: nonexistent path.** Run the binary with a `<path>` argument pointing at a path that does not exist on disk. Assert: exit code `2`, stderr is non-empty (a human-readable message), and stdout is **empty** (no partial diagnostics leak out even though there's nothing to lint).
- **Test: clean bundle → exit 0.** Run against a fixture directory known to have zero violations across all rules. Assert: exit code `0`, stdout is empty.
  - Reuse an existing pass-fixture directory from earlier sections rather than inventing a new one — e.g. `tests/fixtures/okf/missing_frontmatter/pass/` (a single-file mini-bundle whose one file has no violations under any rule) is a reasonable candidate, since fixture files built for section-02 through section-05's rule-specific pass cases should also incidentally be clean under every *other* rule (verify this holds for whichever fixture is chosen; if not, use/create a fixture explicitly designed to be all-around clean).
- **Test: bundle with violation(s) → exit 1.** Run against a fixture directory known to contain at least one violation (e.g. an existing `fail/` fixture from an earlier section, such as `tests/fixtures/okf/missing_frontmatter/fail/`). Assert: exit code `1`, and stdout contains the expected diagnostic line(s) (use `predicates::str::contains` on the known expected message text/format for that fixture's rule, per the message strings already defined in the relevant check module).
- **Test: `--max-line-length` override.** Use a fixture whose only violation is a line whose length sits strictly between the default (100) and a raised custom limit — e.g. a line of 120 characters, with the test run using `--max-line-length 150`. This fixture doesn't already exist in earlier sections' fixture sets (those target the *default* 100-char limit) — create a small dedicated fixture for this test, e.g. `tests/fixtures/cli/max_line_length_override/fail.md` containing a single line of exactly 120 characters (plus a valid trailing newline, and otherwise clean of all other style/OKF violations so no other diagnostic pollutes the assertion). Assert two invocations:
  - With `--max-line-length 150`: exit code `0`, stdout empty (no diagnostic — the 120-char line is under the raised limit).
  - Without the flag (default `100`): exit code `1`, stdout contains a `StyleLineLength` diagnostic (message text per the style-checks section: `line exceeds maximum length of 100 characters (120 found)`).
- **Test: default `--max-line-length` behaves as if 100 was passed explicitly.** A regression guard on `default_value_t = 100` in the `Cli` struct — e.g. run the same fixture from the previous test with no `--max-line-length` flag at all and confirm it produces the identical diagnostic/exit code as explicitly passing `--max-line-length 100` (this can be folded into the previous test's "without the flag" case, or written as a standalone assertion comparing the two invocations' outputs for equality).

Do not write the whole-bundle `insta`-snapshot integration test here — that belongs to section-08-integration-tests, which builds `tests/fixtures/integration_bundle/` and appends its own test(s) to this same `tests/cli_tests.rs` file.

## File paths summary

- Create: `src/cli.rs`
- Create/modify: `src/main.rs` (replace the existing `println!("Hello, world!")` skeleton entirely)
- Create: `tests/cli_tests.rs`
- Create: `tests/fixtures/cli/max_line_length_override/fail.md` (single 120-character line, clean otherwise, valid trailing newline)
- Modify: crate root module declarations (e.g. `mod cli;` / `mod lint;` wiring in `main.rs`) as needed so `main.rs` can reference `cli::Cli` and `lint::lint_bundle`.

## As-built notes (post code-review)

Implemented as planned, plus two fixes from code review (see
`planning/implementation/code_review/section-07-{diff,review,interview}.md`):

- **Broken-pipe safety**: `main.rs` writes diagnostics through a single locked
  `io::stdout()` handle with `writeln!` (not per-line `println!`), and treats
  `ErrorKind::BrokenPipe` as a clean exit-1 rather than a panic — relevant since this
  tool's output is meant to be piped in CI (e.g. `okf-lint dir | head`).
- **Error-message wording**: `LintError::PathNotFound` covers both "path doesn't exist"
  and "path exists but unreadable" (permission-denied) — `lint_bundle` collapses both
  into that variant via a single `std::fs::metadata` call. The stderr message was
  worded neutrally as `"cannot access path: {path}"` rather than asserting the path is
  missing.
- `tests/cli_tests.rs` additionally asserts stderr is empty on all success/violation-path
  tests (not just the error-path test), per the plan's stdout-only/stderr-only contract.
- Added `docs/knowledge/cli.md` per CLAUDE.md's per-section knowledge-doc requirement
  (not called out in the original plan) and linked it from `docs/knowledge/index.md`.

Final test count: 5 tests in `tests/cli_tests.rs`, all passing; full suite (82 tests) and
`cargo clippy --all-targets` clean of new warnings.