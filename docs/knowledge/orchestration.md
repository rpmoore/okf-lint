---
type: module
---

# Orchestration

Ties together file classification, the four check modules (`okf.rs`, `index_md.rs`,
`log_md.rs`, `style.rs`), and the traversal in `walk.rs` into a single entry point that
turns a bundle root path into the final, sorted diagnostic list.

## `src/lint.rs`

- `LintError` (`PathNotFound`, `NotADirectory`, `Io { path, source }`, `InvalidUtf8`) — stubbed
  by section-01, wired up here.
- `FileKind` (private) — `Concept`, `Index { is_root: bool }`, `Log`. `classify(relative_path)`
  matches the file name component exactly (`"index.md"` / `"log.md"`, case-sensitive), so a
  name that merely contains "index" or "log" as a substring (`reindex.md`, `catalog.md`) falls
  through to `Concept`. `is_root` is true iff the relative path's parent is `None` or an empty
  component (i.e. the file is directly at the bundle root, not `sub/index.md`).
- `lint_bundle(root: &Path, max_line_length: usize) -> Result<Vec<(PathBuf, Diagnostic)>, LintError>`:
  1. `std::fs::metadata(root)` once — maps any error (including "doesn't exist") to
     `PathNotFound`, then checks `is_dir()` for `NotADirectory`. A single syscall instead of
     separate `exists()`/`is_dir()` calls, avoiding a TOCTOU window between them.
  2. `collect_md_files(root)` (from `walk.rs`) — propagates any `LintError` as-is.
  3. For each relative path: read the joined full path as bytes, then `String::from_utf8`.
     Non-UTF-8 content maps to `InvalidUtf8(full_path)`; any other read failure maps to
     `Io { path: full_path, source }`. Either aborts the whole run immediately via `?` — no
     partial diagnostics are accumulated on error, matching the exit-code-2 contract the CLI
     (section-07) will build on. Both error variants use the *full* (root-joined) path, not the
     relative one, matching the convention `walk.rs` already established for `LintError::Io`.
  4. `check_style(&content, max_line_length)` runs unconditionally on every file, then the
     classification-appropriate structural check (`check_concept`/`check_index`/`check_log`) is
     appended.
  5. `sort_diagnostics` sorts the full `(PathBuf, Diagnostic)` list by path (`PathBuf::cmp`,
     matching the same ordering `walk.rs` already uses for traversal — not a raw byte-lexicographic
     '/'-joined string comparison, per the plan's explicit cross-reference to walk.rs's sort),
     then `Diagnostic.line` ascending, then `Diagnostic.rule` explicitly (not `Diagnostic`'s
     derived `Ord`, which would also tie-break on `message` — `sort_diagnostics` compares `rule`
     directly so same-file/same-line diagnostics from different rules land in `Rule`
     declaration order regardless of message content).
- Not this section's job: printing/formatting diagnostics (`{path}:{line}: {message}` with `/`
  separators) and CLI wiring — both belong to section-07.
