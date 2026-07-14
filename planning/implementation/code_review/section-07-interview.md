# Code Review Interview: section-07-cli

## Asked

1. **Broken pipe panic.** `println!` per-diagnostic panics on SIGPIPE (e.g. `okf-lint dir | head`), a realistic CI-piping scenario.
   - **Decision: Fix.** Lock stdout once via `io::stdout().lock()`, use `writeln!`, ignore `ErrorKind::BrokenPipe`.

2. **Misleading PathNotFound message.** `lint_bundle` maps permission-denied root dirs into `LintError::PathNotFound` too, so stderr says "path does not exist" even for permission errors.
   - **Decision: Soften wording.** Change message to "cannot access path: {path}" — accurate for both not-found and permission-denied, no `lint.rs` changes needed.

## Auto-fix (no tradeoffs, applied without asking)

3. **Missing `docs/knowledge/` doc.** CLAUDE.md requires an OKF knowledge doc per touched section; every prior section has one but section-07 didn't get one. Create `docs/knowledge/cli.md` and link it from `docs/knowledge/index.md`.

4. **Missing stderr-empty assertions.** Success/violation-path tests (`clean_bundle_exits_0`, `bundle_with_violation_exits_1`, `max_line_length_override_suppresses_violation`, `default_max_line_length_matches_explicit_100`) didn't assert stderr is empty, per the plan's own "stderr-only errors" contract. Add `.stderr(predicate::str::is_empty())` to the `assert_cmd`-based tests (the `Command::output()`-based default-vs-explicit test gets an explicit `assert!(...stderr...is_empty())`).

## Let go (nitpicks, not worth interviewing)

- Portability: `path.display()` uses OS-native separators (Windows would break `/`-convention); project already has Unix-only assumptions elsewhere (`#[cfg(unix)]` tests).
- No doc comments on `Cli` struct fields (affects `--help` text only).
- `ExitCode::from(0)` vs `ExitCode::SUCCESS` — stylistic, no behavior difference.
