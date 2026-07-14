# Code Review: section-07-cli

Overall the implementation is small and mostly matches the plan's contract (exit codes 0/1/2, stdout carries only diagnostics, error branch returns before any stdout write, `Cli` struct matches the spec exactly, fixture line is verified to be exactly 120 chars). Issues found, roughly in priority order:

1. **(Process instructions, medium)** CLAUDE.md mandates updating `docs/knowledge/` for the section of code touched on every code change, creating a new doc if none exists. This diff adds no `docs/knowledge/cli.md` (or similar) and doesn't touch `docs/knowledge/index.md` to link one, unlike every prior section's diff (foundation.md, concept-checks.md, index-checks.md, log-checks.md, style-checks.md, orchestration.md all exist). This is a real gap against the repo's own required workflow, not just the section plan.

2. **(Robustness, medium)** `src/main.rs`'s diagnostic-printing loop calls `println!` once per diagnostic. `println!` panics on a broken pipe (e.g. `okf-lint big-bundle | head`), which is exactly the CI/terminal-piping usage pattern this tool is designed for. A tool designed to emit many compiler-style lines to stdout should write through a single locked `io::stdout()` handle and either ignore/handle `ErrorKind::BrokenPipe` or use `writeln!` and swallow the error, rather than relying on the panicking `println!` macro. Also inefficient (re-locking stdout per line) versus locking once.

3. **(Correctness of surfaced error text, medium)** `format_error` renders `LintError::PathNotFound(path)` as `"path does not exist: {path}"`. But `lint_bundle` (src/lint.rs) maps *any* `std::fs::metadata(root)` failure — including permission-denied — into `PathNotFound`. So a permission-denied root directory prints a misleading "path does not exist" message instead of a permission error. Root cause is in section-06's `lint.rs`, but section-07's error-message text bakes in an assumption not actually guaranteed by the variant, and no test catches this.

4. **(Test coverage gap, medium)** The plan's contract explicitly calls out "stdout-only diagnostics, stderr-only errors". The `nonexistent_path` test checks stdout is empty on error, but none of `clean_bundle_exits_0`, `bundle_with_violation_exits_1`, or the `--max-line-length` tests assert that **stderr is empty** on the success/violation paths. A stray `eprintln!`/debug print introduced later on the happy path would silently pass all tests.

5. **(Portability, low)** `path.display()` uses the OS-native path separator. Section-06's contract says relative paths use `/` separators; on Windows this would render diagnostics with backslashes. The project already has `#[cfg(unix)]`-gated tests elsewhere so it's likely Unix-only in practice, but nothing enforces/documents that at the CLI-output layer.

6. **(Polish, low)** `Cli` struct fields have no doc comments, so `--help` output will lack per-argument descriptions.

7. **(Polish, low)** `ExitCode::from(0)` could be `ExitCode::SUCCESS` for readability; not a bug.

## What's solid

- `Cli` struct matches the plan's example verbatim (pub fields, `default_value_t = 100`).
- `main.rs`'s control flow correctly guarantees no stdout output on the `Err` path.
- Diagnostic format string `{path}:{line}: {message}` matches convention; no pre-existing `Display`/formatter was reimplemented (confirmed none exists in `diagnostic.rs`).
- `max_line_length_override/fail.md` fixture verified to be exactly 120 characters, clean under every style/OKF rule otherwise.
- Reused `missing_frontmatter/pass`/`fail` fixtures' expected diagnostic message text cross-checked against `checks/okf.rs` literals and matches.

Files reviewed: `src/cli.rs`, `src/main.rs`, `tests/cli_tests.rs`, `tests/fixtures/cli/max_line_length_override/fail.md`, `src/lint.rs`, `src/walk.rs`, `src/checks/style.rs`, `src/checks/okf.rs`, `docs/knowledge/index.md`.
