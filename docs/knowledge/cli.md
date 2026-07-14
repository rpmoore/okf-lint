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
  flat `path: Option<PathBuf>`, `--max-line-length <N>` (`u32`, default 100), and
  `--include-hidden` (bool flag), all used only when `command` is `None`. This keeps
  bare `okf-lint <path>` working exactly as it did before subcommands existed, as an
  implicit `lint`.
- `Command` (`clap::Subcommand`): `Lint(LintArgs)`, `Fmt(FmtArgs)`, and `Version` (no
  args).
  - `LintArgs` — `path: PathBuf`, `--max-line-length <N>` (default 100),
    `--include-hidden` (bool flag). Identical shape to the pre-subcommand flat `Cli`.
  - `FmtArgs` — `path: PathBuf`, `--max-line-length <N>` (default 100),
    `--tab-width <N>` (`u32`, default 4, spaces per hard tab when expanding),
    `--check` (bool flag, report-only — see `docs/knowledge/fmt.md`),
    `--include-hidden` (bool flag).
- `--include-hidden` defaults to `false` on every surface (top-level, `lint`, `fmt`):
  dot-prefixed files/directories (e.g. `.git`, `.github`) are pruned during traversal
  unless the flag is passed, per `walk::collect_md_files`'s `include_hidden` parameter
  (`docs/knowledge/foundation.md`). Threaded straight through to
  `lint::lint_bundle`/`fmt::run_fmt` — no CLI-layer logic beyond passing the bool
  along.

## `src/main.rs`

- `main() -> ExitCode` dispatches on `cli.command`: `Some(Command::Lint(args))` and
  `None` (with `cli.path` present) both call `run_lint`; `None` with no `cli.path`
  prints a usage error to stderr and exits **2**; `Some(Command::Fmt(args))` calls
  `run_fmt_command`.
- `run_lint(path, max_line_length, include_hidden) -> ExitCode`:
  1. `lint::lint_bundle(path, max_line_length, include_hidden)`.
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
  re-sorting here. `to_slash_path(path)` is used instead of `path.display()`: it joins
  `path.components()` with `/` explicitly, so stdout stays `/`-separated even on
  Windows (where `Path::display()` would emit `\`), matching the CLI output contract
  and the committed `insta` snapshots. `run_fmt_command`'s `would reformat: {path}`
  output (`--check` mode) uses the same helper.
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
- `run_version() -> ExitCode` (`Some(Command::Version)`): prints three lines —
  `okf-lint {CARGO_PKG_VERSION}` (from `env!`, i.e. `Cargo.toml`'s `version`), `arch:
  {ARCH}` (`std::env::consts::ARCH`, a compile-time constant reflecting the actual
  target architecture the binary was built for, correct even when cross-compiled/built
  under emulation — see `docs/knowledge/deployment.md`), and `commit: {sha}` (from
  `env!("OKF_LINT_GIT_SHA")`, set by `build.rs` at compile time — see below). Always
  exits **0**; uses the same locked-stdout/`writeln!`/`BrokenPipe` handling as
  `print_diagnostics`, for the same reason (`okf-lint version | head`).

## `build.rs`

Runs at compile time, before `src/main.rs` is built, to resolve the git commit embedded
in `run_version`'s `commit:` line via `println!("cargo:rustc-env=OKF_LINT_GIT_SHA={sha}")`
(an env var only `env!` in `main.rs` can see — this is the standard way to bake a
compile-time value into a Rust binary without a separate crate). Three sources, tried in
order, since no single one covers every way this crate gets built:
1. `OKF_LINT_GIT_SHA` env var, if set non-empty — lets the `Dockerfile` inject the sha
   explicitly (its build context has no `.git`; see `docs/knowledge/deployment.md`).
2. `git rev-parse HEAD` — works for local `cargo build`/`cargo test`/`cargo install
   --path .` from this checkout, where `.git` is present.
3. `.cargo_vcs_info.json`'s `"sha1"` field, hand-parsed via string splitting (no JSON
   dependency for one field) rather than a full parser — `cargo package`/`cargo
   publish` writes this file into the tarball with the sha of the commit that was
   packaged. This is the *only* source that resolves for `cargo install okf-lint` (from
   crates.io): that build runs from an extracted tarball with no `.git` directory, so
   source 2 always misses there. Confirmed by building an extracted `cargo package`
   tarball directly: without this fallback the tarball build always reported `commit:
   unknown`.

Falls back to the literal string `"unknown"` if none of the three resolve (e.g. building
from a source tree with no `.git` and no `.cargo_vcs_info.json`, which is unlikely in
practice but not an error).

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

`--include-hidden` coverage: a bundle with a violation only inside a `.hidden/`
directory exits 0 by default (hidden dir never walked) and exits 1 with the
diagnostic on stdout when `--include-hidden` is passed; the `fmt` equivalent asserts
`--include-hidden` fixes a style violation inside `.hidden/` in place.

The `--max-line-length` override test uses a dedicated fixture,
`tests/fixtures/cli/max_line_length_override/fail.md` — frontmatter + heading + a single
120-character body line, otherwise clean under every other style/OKF rule so no other
diagnostic pollutes the assertion.

`tests/cli_tests.rs` is also touched by section-08-integration-tests, which appends a
whole-bundle `insta`-snapshot test to this same file.

`version_command_reports_version_arch_and_commit` asserts the exact first two lines
(`okf-lint {env!("CARGO_PKG_VERSION")}` and `arch: {std::env::consts::ARCH}`, both
computed the same way in the test as in `main.rs` so the assertion doesn't need
updating on a version bump or when run on a different architecture) and that the third
line's `commit: ` value is a 40-char hex string — not a pinned sha, since that changes
every commit, but this checkout has `.git` so `build.rs` never falls back to
`"unknown"` here.
