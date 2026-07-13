Now I have all necessary content. Let me write the self-contained section-06 content.

The section content is:

---

# Section 06: Orchestration

## Overview

This section implements file classification and the top-level orchestration function `lint_bundle`, which ties together all four check modules (concept, index, log, style) built in sections 02-05. This is the module that turns a bundle root path into the final, sorted list of diagnostics that the CLI (section 07) will print.

## Dependencies

This section requires the following to already exist:

- **From section-01-foundation**: `src/diagnostic.rs` (`Diagnostic` struct, `Rule` enum), `src/walk.rs` (`collect_md_files`).
- **From section-02-concept-checks**: `src/checks/okf.rs` — `fn check_concept(content: &str) -> Vec<Diagnostic>`.
- **From section-03-index-checks**: `src/checks/index_md.rs` — `fn check_index(content: &str, is_root: bool) -> Vec<Diagnostic>`.
- **From section-04-log-checks**: `src/checks/log_md.rs` — `fn check_log(content: &str) -> Vec<Diagnostic>`.
- **From section-05-style-checks**: `src/checks/style.rs` — `fn check_style(content: &str, max_line_length: usize) -> Vec<Diagnostic>`.

Do not re-implement or modify any of the above; only import and call them.

## Files to create

- `src/lint.rs` — new file. Contains `LintError`, file classification, and `lint_bundle`.

You will also need to ensure `src/main.rs` (or a `lib.rs`, whichever the foundation section established as the crate root wiring) declares `mod lint;` so this module is reachable — check how `mod` declarations for `checks`, `diagnostic`, `frontmatter`, and `walk` were wired in section-01 and follow the same pattern for `lint`.

## Background

### Module layout context

Per the full project's module layout:

```
src/
  main.rs            # entry point: parse CLI, run linter, map result to exit code + stderr
  cli.rs             # Cli struct (clap derive): <path> positional, --max-line-length flag
  diagnostic.rs      # Diagnostic type, Rule enum (fixed ordering), sort/format helpers
  frontmatter.rs     # shared "---"-delimited block splitter
  walk.rs            # bundle traversal: sorted, hidden-skipping, non-symlink-following .md file list
  lint.rs            # <-- THIS SECTION: orchestration: classify each file, dispatch to
                      #     the right checks, collect + sort diagnostics
  checks/
    mod.rs
    okf.rs            # concept-document frontmatter checks (rules 1 and 2)
    index_md.rs        # index.md frontmatter-placement and body-structure checks (rules 3 and 4)
    log_md.rs           # log.md date-heading check (rule 5)
    style.rs            # the 5 markdown hygiene checks, applied to every file
```

### `Diagnostic` and `Rule` (already defined in `diagnostic.rs`, for reference only — do not redefine)

```rust
struct Diagnostic {
    line: usize,     // 1-based; see per-rule line-number rules
    rule: Rule,       // used only for sort tie-breaking and internal grouping, never printed
    message: String,  // exact text, already formatted
}

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

`Rule`'s declaration order is the tie-break order used when two diagnostics share the same `(file, line)`.

A per-file diagnostic collector pairs each `Diagnostic` with the file's path (relative to the bundle root, using `/` separators regardless of OS) for final formatting and sorting: `(relative_path: PathBuf, Diagnostic)`.

### `collect_md_files` (already defined in `walk.rs`, for reference only)

```rust
fn collect_md_files(root: &Path) -> Result<Vec<PathBuf>, LintError>
```

Recurses under `root` with `walkdir`, default (non-`follow_links`) settings. Skips any directory or file whose name starts with `.` (does not descend into it at all). Filters to files with a `.md` extension. Converts to paths relative to `root`. Returns a lexicographically-sorted list. I/O errors while walking are mapped to `LintError::Io`. Note: `walk.rs` defines its own use of `LintError` — this section's `lint.rs` should define the canonical `LintError` enum (see below) that `walk.rs` also targets; confirm the exact ownership/location of `LintError` matches what section-01 already established (it may already live in `lint.rs` if section-01 stubbed it there, or the type may need to be introduced fresh in this section — check `src/walk.rs`'s existing `use` statements before duplicating).

## File classification (plan §4)

For each relative path from `collect_md_files`, classify it into one of three kinds:

- File name exactly `index.md` → **Index**. It is the *root* index iff the relative path has no parent component (i.e. it is directly `index.md`, not `sub/index.md`).
- File name exactly `log.md` → **Log**.
- Anything else → **Concept**.

Filename comparison is exact/case-sensitive, per the OKF spec's reserved-filename definitions. This means a filename that merely *contains* "index" or "log" as a substring (e.g. `reindex.md`, `catalog.md`) must classify as **Concept**, not Index/Log — guard against a substring-match bug by comparing the full file-name component exactly.

Suggested representation (a plan-level sketch, not mandatory — pick whatever shape is convenient for `lint_bundle`'s dispatch below):

```rust
enum FileKind {
    Concept,
    Index { is_root: bool },
    Log,
}

fn classify(relative_path: &Path) -> FileKind
```

## `lint.rs` core types and orchestration (plan §3, §7)

```rust
enum LintError {
    PathNotFound(PathBuf),
    NotADirectory(PathBuf),
    Io { path: PathBuf, source: std::io::Error },
    InvalidUtf8(PathBuf),
}

fn lint_bundle(root: &Path, max_line_length: usize) -> Result<Vec<(PathBuf, Diagnostic)>, LintError>
```

**Docstring intent:**

1. Validate `root` exists and is a directory. If it doesn't exist, return `Err(LintError::PathNotFound(root.to_path_buf()))`. If it exists but is not a directory, return `Err(LintError::NotADirectory(root.to_path_buf()))`.
2. Call `collect_md_files(root)`. Propagate any `LintError` it returns.
3. For each returned relative path:
   - Read the corresponding file (joined with `root`) to a `String`. A read that fails because the content isn't valid UTF-8 yields `Err(LintError::InvalidUtf8(path))`. Any other read failure (permission denied, file vanished, etc.) yields `Err(LintError::Io { path, source })`.
   - **Any `LintError` encountered while reading aborts the whole run immediately** — return `Err` right away with no partial diagnostics. This matches the exit-code-2 contract (usage/IO errors short-circuit rather than being reported as partial lint output) that the CLI section will build on.
4. For each successfully-read file:
   - Classify it per the classification rules above.
   - Run `check_style(&content, max_line_length)` **unconditionally** — style checks apply to every file regardless of classification.
   - Run the classification-appropriate structural check:
     - Concept → `check_concept(&content)`
     - Index → `check_index(&content, is_root)`
     - Log → `check_log(&content)`
   - Accumulate every diagnostic from both calls into the running `Vec<(PathBuf, Diagnostic)>`, paired with this file's relative path.
5. After all files are processed, sort the full diagnostic list per the ordering rules below, and return `Ok(sorted_list)`.

## Diagnostic ordering (plan §7)

After collecting every `(relative_path, Diagnostic)` pair across all files, sort the full list by:

1. `relative_path`, lexicographically (matches the traversal sort in `walk.rs`, so this is really just "stable regardless of walk order").
2. `Diagnostic.line`, ascending.
3. `Diagnostic.rule`'s declaration order in the `Rule` enum as a tie-break when the same file and line have multiple diagnostics — OKF conformance rules sort before style rules, and within each group, in the fixed order the enum lists them.

This sort should be applied as the final step of `lint_bundle` before returning `Ok(...)`. (Formatting each diagnostic as `{relative_path}:{line}: {message}` for stdout output, using `/` path separators, is the CLI's responsibility in section-07 — this section only needs to produce the correctly-sorted `Vec`, not print anything.)

## Tests (write these first, TDD)

All tests are inline `#[cfg(test)] mod tests` blocks. Tests for `lint_bundle` and classification build small ad hoc temp-dir bundles per test (e.g. via `tempfile`, or hand-rolled `std::fs` calls into a `TempDir`) rather than using `tests/fixtures/`, since these are orchestration-behavior tests independent of any single rule's fixture content.

### `lint.rs` — `lint_bundle`

- Test: `root` does not exist → `Err(LintError::PathNotFound)`.
- Test: `root` exists but is a file, not a directory → `Err(LintError::NotADirectory)`.
- Test: a non-UTF-8 `.md` file in the bundle → `Err(LintError::InvalidUtf8)`, and the error aborts the whole run (assert no diagnostics are returned alongside the error — the `Result` is `Err`, full stop).
- Test: every file in the bundle gets both the style checks and its classification-appropriate structural checks — e.g. a `log.md` with a bad date heading AND a hard tab both produce diagnostics from a single `lint_bundle` call.
- Test: the returned `Vec<(PathBuf, Diagnostic)>` is sorted per the ordering rules above across multiple files (cross-file ordering, not just within-file — this is a lighter-weight regression check with 2-3 files; full cross-file/cross-rule ordering is more thoroughly covered by the whole-bundle integration test in section-08, which depends on the CLI existing).

### File classification

Covered as part of the `lint_bundle` tests above and/or a small standalone `#[cfg(test)]` block next to the classification logic:

- Test: `index.md` at bundle root → classified Index, `is_root = true`.
- Test: `sub/index.md` → classified Index, `is_root = false`.
- Test: `log.md` (any depth) → classified Log.
- Test: any other filename, including a name that merely contains "index" or "log" as a substring (e.g. `reindex.md`, `catalog.md`) → classified Concept (guards against a substring-match bug instead of exact-name comparison).

### Diagnostic ordering (can live in `lint.rs` or `diagnostic.rs`, whichever already hosts the sort helper from section-01)

- Test: given an unsorted `Vec<(PathBuf, Diagnostic)>` spanning multiple files, multiple lines within a file, and multiple rules on the same `(file, line)`, sorting produces the exact order defined above (path, then line, then `Rule` declaration order) — assert against a hand-constructed expected order rather than a snapshot, since this is testing the sort function in isolation.
- Test: `format!("{path}:{line}: {message}")` output uses `/` path separators even when constructed from platform-native `PathBuf` components (relevant on Windows CI, if applicable). Note: if the formatting helper itself was already implemented in section-01's `diagnostic.rs`, this test may already exist there — only add it here if it doesn't yet exist and depends on data assembled by `lint_bundle`.

## Notes on scope

- This section does **not** implement the CLI (`cli.rs`, `main.rs`) or exit-code mapping — that's section-07, which depends on `lint::lint_bundle` existing and working.
- This section does **not** implement or modify any of the four check modules (`checks/okf.rs`, `checks/index_md.rs`, `checks/log_md.rs`, `checks/style.rs`) — only calls into their already-defined public functions.
- This section does **not** build fixture directories under `tests/fixtures/` — those belong to sections 02-05 (per-rule fixtures) and section-08 (the whole-bundle `integration_bundle/`). Tests here use ad hoc temp directories only.