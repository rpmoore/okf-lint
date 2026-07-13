# Research: okf-lint

No existing codebase to research — repo is a bare `cargo new` skeleton
(`Cargo.toml` with no deps, `src/main.rs` printing "Hello, world!").
Findings below are web research on current (2026) Rust crate choices for
each subsystem the spec requires.

## 1. YAML frontmatter parsing

Need: parse a `---`-delimited YAML block into something checkable for a
non-empty string `type` field.

- Original `serde_yaml` is archived/deprecated (March 2024) — do not use.
- `serde_yml` (a fork) is flagged by RUSTSEC-2025-0068 (unsound C-FFI in
  versions ≤0.0.12) and itself now recommends migrating away — avoid.
- `serde_yaml_ng` 0.10.0 — clean, API-identical continuation of
  `serde_yaml`. Adopted by projects like Nushell. No commits since May
  2024 (stalled but stable/complete for basic parsing).
- `serde-saphyr` — new, actively released (as recent as this month),
  typed deserialization straight into a struct with no intermediate
  `Value` DOM, panic-free, DoS-budget-guarded. Pre-1.0 (0.0.x), some API
  churn expected, lacks advanced serde features (untagged enums,
  flattening) — not needed here.

**Candidates for the plan to choose between:** `serde_yaml_ng` (safer,
zero-migration-friction, stalled-but-done) vs `serde-saphyr` (more
modern approach, but pre-1.0). Given this linter only needs "parse YAML
map, check `type` key is a non-empty string" — not complex serde
features — either is viable. Recommend `serde_yaml_ng` for stability
given pre-1.0 churn risk of the alternative, but flag as an open
decision for the plan-writing step.

Sources: users.rust-lang.org serde-yaml deprecation thread, docs.rs/serde_yml,
github.com/acatton/serde-yaml-ng, github.com/bourumir-wyngs/serde-saphyr.

## 2. CLI argument parsing

**`clap` 4.6.x, derive API.** Positional args inferred from field order,
no attribute needed:

```rust
#[derive(clap::Parser)]
struct Cli {
    path: std::path::PathBuf,
    #[arg(long, default_value_t = 100)]
    max_line_length: u32,
}
```

`default_value_t` (typed) preferred over `default_value` (string) for
non-`String` types. `clap` remains the de facto standard for this shape
of CLI (one positional + one flag).

Source: docs.rs/clap derive tutorial, rust.code-maven.com clap defaults.

## 3. Directory tree walking

**`walkdir` 2.5.0**, not `ignore`. The `ignore` crate (maintained by the
ripgrep team) silently skips `.gitignore`-matched files — wrong for a
linter, since a `.md` file listed in `.gitignore` would be silently
skipped from checks. `walkdir` gives raw, predictable traversal; combine
with a filter on `.md` extension and an explicit sort of collected paths
for deterministic, reproducible diagnostic ordering (needed since the
spec's diagnostics format is order-sensitive for tests).

Source: github.com/BurntSushi/walkdir, crates.io/crates/walkdir.

## 4. CLI integration testing

**`assert_cmd` 2.2.2 + `predicates` 3.1.4 + `insta` 1.48.0.** Still the
standard current combo (Rust CLI book, 2025 blog coverage). `assert_cmd`
runs the compiled binary and asserts exit code; `predicates` gives
composable stdout/stderr assertions; `insta` snapshot-tests multi-line
diagnostic output (avoids unwieldy hand-written string-equality
assertions for the spec's whole-bundle integration test in §5).
Pattern: `assert_cmd::Command::cargo_bin("okf-lint")` piped into
`insta::assert_snapshot!()`, reviewed via `cargo insta review`.

Source: alexwlchan.net (2025) testing Rust CLI apps with assert_cmd,
rust-cli.github.io/book testing chapter, blog.logrocket.com insta guide.

## Testing conventions (new project, no existing convention to follow)

- Standard `cargo test` layout: unit tests inline in `src/` modules
  (`#[cfg(test)] mod tests`), integration/CLI tests in `tests/`.
- Fixture bundles for OKF conformance / markdown style checks (spec §5)
  live under `tests/fixtures/` as real small directory trees / files —
  simplest way to give each check a genuine passing and minimal-failing
  case without a runtime fixture-generation layer.
- CLI-level exit-code and diagnostic-output tests use `assert_cmd`
  against `tests/fixtures/` bundles, with `insta` snapshots for full
  diagnostic-output-format assertions.
