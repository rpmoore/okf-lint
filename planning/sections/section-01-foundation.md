# section-01-foundation

## Overview

This is the foundation section for `okf-lint`, a Rust CLI linter that checks Markdown files in an "OKF bundle" (a directory tree) against OKF v0.1 conformance rules and generic Markdown hygiene rules. This section has **no dependencies on other sections** — it establishes the shared, dependency-free building blocks (`Diagnostic`/`Rule` types, frontmatter parsing, and directory walking) that every other module and check imports.

**Repo starting state:** bare `cargo new` skeleton — `Cargo.toml` with no dependencies, `src/main.rs` printing `"Hello, world!"`, edition 2024.

**What this section blocks:** section-02 (concept checks), section-03 (index checks), section-04 (log checks), and section-05 (style checks) all depend on this section's `diagnostic.rs` and `frontmatter.rs`/`walk.rs`.

**Full target module layout** (for context — this section creates only the files marked below; the rest are built by later sections):

```
src/
  main.rs            # [later: section-07] entry point
  cli.rs             # [later: section-07] Cli struct
  diagnostic.rs       # [THIS SECTION] Diagnostic type, Rule enum, sort/format helpers
  frontmatter.rs      # [THIS SECTION] shared "---"-delimited block splitter
  walk.rs             # [THIS SECTION] bundle traversal
  lint.rs             # [later: section-06] orchestration
  checks/
    mod.rs             # [later: sections 02-05]
    okf.rs              # [later: section-02]
    index_md.rs          # [later: section-03]
    log_md.rs             # [later: section-04]
    style.rs               # [later: section-05]
tests/
  fixtures/           # [later: sections 02-08]
  cli_tests.rs        # [later: section-07/08]
```

## 1. `Cargo.toml` dependencies

Add the following to `Cargo.toml`. This section's completion gate is that `cargo build` and `cargo test` succeed with these dependencies present (a placeholder `#[test] fn placeholder() {}` is acceptable if no real tests exist yet at the moment of the check, but this section itself will add real tests below, so the placeholder is just a sanity note, not a requirement).

Runtime dependencies:
- `clap` (4.x, `derive` feature) — CLI parsing (used starting section-07, but add the dependency now).
- `walkdir` (2.x) — directory traversal, used by `walk.rs` in this section.
- `serde_yaml_ng` (0.10.x) — frontmatter YAML parsing (used starting section-02/03, but add the dependency now). Chosen over `serde-saphyr` (newer but pre-1.0 with expected API churn) and the archived original `serde_yaml` (deprecated) / `serde_yml` fork (RUSTSEC-2025-0068).
- `chrono` — calendar-date validation for `log.md` headings (used starting section-04, but add the dependency now), specifically `NaiveDate::parse_from_str`.

Dev-dependencies (tests only):
- `assert_cmd` — run the compiled binary and assert exit codes/output (used starting section-07/08).
- `predicates` — composable stdout/stderr assertions alongside `assert_cmd` (used starting section-07/08).
- `insta` — snapshot testing for the whole-bundle integration test's multi-line diagnostic output (used starting section-08), reviewed via `cargo insta review`.

Even though several of these dependencies aren't exercised by code written in this section, add them all now so the dependency set is locked in and later sections (which run in parallel) don't need to touch `Cargo.toml` concurrently.

**Dependency/setup verification gate (TDD requirement, do this first):** After adding `clap`, `walkdir`, `serde_yaml_ng`, `chrono` and the dev-dependencies (`assert_cmd`, `predicates`, `insta`) to `Cargo.toml`, run `cargo build` and `cargo test` and confirm they succeed (with `src/main.rs` still just printing `"Hello, world!"` at this point) — this confirms the dependency set resolves and compiles on a clean skeleton before any real module code is written.

## 2. `src/diagnostic.rs`

### Types to implement

```rust
struct Diagnostic {
    line: usize,     // 1-based; see per-rule line-number rules (defined in later sections' check modules)
    rule: Rule,       // used only for sort tie-breaking and internal grouping, never printed
    message: String,  // exact message text, already formatted (e.g. with {N} substituted)
}
```

```rust
enum Rule {
    // OKF conformance, in this fixed order:
    OkfMissingFrontmatter,
    OkfMissingType,
    OkfIndexFrontmatterPlacement,
    OkfIndexBodyStructure,
    OkfLogDateHeading,
    // Markdown style, in this fixed order:
    StyleLineLength,
    StyleTrailingWhitespace,
    StyleTrailingNewline,
    StyleConsecutiveBlankLines,
    StyleHardTab,
}
```

`Rule`'s declaration order **is load-bearing**: it is the tie-break order used when two diagnostics share the same `(file, line)`, applied by the orchestration layer built in section-06. Do not reorder these variants once written — later sections and their tests depend on this exact order. `Derive` whatever traits are needed for equality/ordering/debug printing in tests (e.g. `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Debug`, `Clone`, `Copy` on `Rule`; `PartialEq`, `Debug`, `Clone` at minimum on `Diagnostic`).

A per-file diagnostic collector (used by later orchestration code, not defined here) pairs each `Diagnostic` with the file's path (relative to the bundle root, using `/` separators regardless of OS) for final formatting and sorting: `(relative_path: PathBuf, Diagnostic)`. This section does not need to implement the sort/format helpers over that pair type — that's section-06's job (`lint.rs` §7 sort logic) — but `diagnostic.rs` should expose whatever is needed on `Rule`/`Diagnostic` (e.g. `Ord`) for that later code to sort by rule order.

### Tests to write first (inline `#[cfg(test)] mod tests` in `diagnostic.rs`)

- Test: two `Diagnostic`s on the same `(file, line)` sort with the OKF rule before the style rule, per `Rule`'s declared enum order. (Since the file/line pairing type itself isn't defined until section-06, this test can operate directly on `Rule` values / discriminants, or construct two `Diagnostic`s with the same `line` and compare their `rule` ordering.)
- Test: `Rule`'s declared order places all `Okf*` variants before all `Style*` variants, and within each group matches the order listed above (this is a direct assertion on `Rule as usize` / enum discriminant order, guarding against accidental reordering during implementation).

## 3. `src/frontmatter.rs`

### Types/functions to implement

```rust
enum FrontmatterResult {
    None,                    // content doesn't start with a "---" line
    Unclosed,                // starts with "---" but no closing "---" line found
    Found { yaml_text: String, body_start_line: usize },
}

fn split_frontmatter(content: &str) -> FrontmatterResult
```

**Docstring intent:** `content` must literally begin with a line that is exactly `---` (no leading blank lines, no trailing characters on that line) for `Found`/`Unclosed` to apply; anything else is `None`. When `Found`, `yaml_text` is the raw text between the two `---` delimiter lines (not yet parsed as YAML — that's the caller's job, since callers want different things: the concept-document checker in section-02 parses it fully, the index.md checker in section-03 just needs to know which keys are present). `body_start_line` is the 1-based line number of the first line after the closing `---` (used to offset later body-line diagnostics in section-03).

This module is intentionally a pure string-parsing function with no I/O and no dependency on `diagnostic.rs` — it's shared by both `checks/okf.rs` (section-02) and `checks/index_md.rs` (section-03), which apply different rules about what's allowed inside the detected block. Keeping the detection logic in one place avoids two slightly-different frontmatter-detection implementations drifting apart.

### Tests to write first (inline `#[cfg(test)] mod tests`, using inline string literals — no fixture files needed, these are pure-function unit tests)

- Test: content not starting with a `---` line → `FrontmatterResult::None`.
- Test: content starting with `---` but with no closing `---` line → `FrontmatterResult::Unclosed`.
- Test: well-formed `---`\<yaml\>`---`\<body\> content → `Found` with the correct `yaml_text` (exact text between delimiters, unparsed) and correct `body_start_line`.
- Test: a leading blank line before `---` → `None` (delimiter must be the literal first line).
- Test: a `---` line with trailing characters (e.g. `--- `) → `None` (must be exactly `---`, not a prefix match).

## 4. `src/walk.rs`

### Function to implement

```rust
fn collect_md_files(root: &Path) -> Result<Vec<PathBuf>, LintError>
```

**`LintError` lives in `src/lint.rs`, not `walk.rs`.** This section must create a **stub** `src/lint.rs` containing only the `LintError` enum (section-06 extends this same file later with the full `lint_bundle` orchestration function — it does not redefine or move this enum):

```rust
enum LintError {
    PathNotFound(PathBuf),
    NotADirectory(PathBuf),
    Io { path: PathBuf, source: std::io::Error },
    InvalidUtf8(PathBuf),
}
```

Only the `Io` variant is exercised by this section's code (`walk.rs`); `PathNotFound`, `NotADirectory`, and `InvalidUtf8` are unused until section-06 builds `lint_bundle`. Expect (and ignore) "variant is never constructed"-style dead-code warnings for those three in the meantime — warnings do not fail `cargo build` or `cargo test`. `walk.rs` references this type via `use crate::lint::LintError;`.

**Docstring intent:** recurse under `root` with `walkdir`, default (non-`follow_links`) settings. Skip any directory or file whose name starts with `.` (do not descend into it at all). Filter to files with a `.md` extension. Convert to paths relative to `root`. Sort the resulting list lexicographically before returning, so downstream diagnostic ordering (established in section-06/section-07) is deterministic regardless of filesystem iteration order. I/O errors while walking (e.g. a permission-denied subdirectory) are mapped to `LintError::Io`.

### Tests to write first (inline `#[cfg(test)] mod tests`, using `tempfile`-style ad hoc directories — or a small helper building a temp dir tree — not `tests/fixtures/`, since these are walk-behavior tests independent of file content)

- Test: a mix of `.md` and non-`.md` files → only `.md` files returned.
- Test: a dotfile/dot-directory (e.g. `.git/`, `.hidden.md`) → excluded entirely, including anything nested under a dot-directory.
- Test: returned paths are relative to `root`, not absolute.
- Test: returned list is sorted lexicographically regardless of the order files were created on disk.
- Test: a permission-denied subdirectory during walk → `LintError::Io` is produced (may need a platform-conditional test, e.g. `#[cfg(unix)]` using `std::fs::Permissions`).

(If you do not have a `tempfile` crate available and don't want to add one, use `std::env::temp_dir()` plus a uniquely-named subdirectory created/torn down per test with `std::fs`, or `std::fs::create_dir_all` under a per-test-unique path — either approach is acceptable as long as tests clean up after themselves and don't collide when run in parallel. Adding `tempfile` as a dev-dependency is also acceptable if preferred; it wasn't listed in the plan's dependency list but is a common, low-risk testing utility — note it explicitly in code comments if added, since it's not called out in the original plan's §9.)

## 5. File classification (context only — not implemented in this section)

For reference/context (implemented by section-06, not here): every relative path from `collect_md_files` will eventually be classified as **Index** (filename exactly `index.md`, with a further **root** vs **non-root** distinction based on whether the relative path has a parent component), **Log** (filename exactly `log.md`), or **Concept** (anything else), using exact/case-sensitive filename comparison. This section does not need to implement classification — it's mentioned here only so the walking/path-relativization behavior of `collect_md_files` (returning paths relative to `root`) is understood in terms of why it matters downstream.

## Section completion checklist

1. Add all dependencies to `Cargo.toml` listed in §1 above.
2. Run `cargo build` and `cargo test` to confirm the skeleton still compiles with the new dependency set.
3. Create `src/diagnostic.rs` with `Diagnostic` and `Rule` as specified in §2, plus its two inline tests.
4. Create `src/frontmatter.rs` with `FrontmatterResult` and `split_frontmatter` as specified in §3, plus its five inline tests.
5. Create a **stub** `src/lint.rs` containing only the `LintError` enum, as specified in §4 above (section-06 later extends this same file with `lint_bundle`; do not create this stub inside `walk.rs`).
6. Create `src/walk.rs` with `collect_md_files` (using `LintError` from `src/lint.rs`) as specified in §4, plus its five inline tests.
7. Wire `mod diagnostic;`, `mod frontmatter;`, `mod lint;`, `mod walk;` into `src/main.rs` (or a future `lib.rs` if the implementer prefers a lib+bin split — the plan doesn't mandate one, `main.rs` module declarations are sufficient) so the modules compile as part of the crate.
8. Run `cargo build`, `cargo test`, and `cargo clippy` to confirm everything compiles cleanly and all inline tests pass before handing off to sections 02-05.

## Implementation Notes (post-review)

Implemented as specified, with the following additions from code review:

- **CRLF tolerance in `split_frontmatter`.** The delimiter-line comparison
  (`"---"`) now trims a trailing `\r` before matching, so CRLF-terminated
  files are detected the same as LF-terminated ones. Not covered by the
  original plan; added per user decision during code review. New test:
  `crlf_line_endings_are_treated_like_lf`.
- **Root-walk bug found via review.** `collect_md_files`'s dot-file
  exclusion originally applied to the walked root path itself, not just
  entries nested under it. This only surfaces when the root's own
  directory name starts with `.` (e.g. `tempfile::TempDir`'s default
  naming) — the fix restricts the dot-check to `entry.depth() > 0`. New
  regression test: `dot_prefixed_root_itself_is_still_walked`.
- **`tempfile` added as a direct dev-dependency** (the plan explicitly
  permitted this as an alternative to hand-rolled temp dirs) — `walk.rs`
  tests now use `tempfile::TempDir` for RAII cleanup instead of manual
  `fs::remove_dir_all` calls that would be skipped on an early panic.
- **`permission_denied_subdirectory_is_io_error`** now checks whether the
  `chmod 0o000` actually made the directory unreadable before asserting,
  and skips (rather than falsely passing/failing) when run with
  privileges that bypass Unix permission bits (e.g. root in some CI
  containers).
- **`LintError`'s dead-code warnings are left unsuppressed** (no
  `#[allow(dead_code)]`), per the plan's own note to "expect and ignore"
  them until section-06 wires up the unused variants.
- **`docs/knowledge/index.md` and `docs/knowledge/foundation.md`** were
  added per the project's CLAUDE.md requirement to document each
  touched section in the OKF knowledge base — not called for by this
  plan, but a standing project-level instruction.