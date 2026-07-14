# Section 06 (orchestration) code review

Solid, faithful implementation. Classification, error handling, unconditional style-check
dispatch, and the sort comparator are all correct — in particular, `sort_diagnostics` correctly
tie-breaks on `rule` explicitly rather than falling back to `Diagnostic`'s derived `Ord` (which
would also compare `message`).

## Findings, ranked

1. **(High — CLAUDE.md compliance)** No knowledge doc for `lint.rs`/orchestration.
2. **(Medium — test gap)** No `lint_bundle`-level test exercising Concept/Index dispatch
   end-to-end, especially `is_root` threading through to `check_index`. `classify()` unit
   tests only cover the pure function, not the wiring inside `lint_bundle`'s match arm.
3. **(Minor — test gap)** No test forcing `std::fs::read` to fail for a non-UTF8 reason
   (e.g. chmod 000) to exercise `lint_bundle`'s own `LintError::Io` mapping, as opposed to
   relying on `walk.rs`'s directory-level IO test.
4. **(Nitpick, not fixed)** `sort_diagnostics` uses `PathBuf::cmp` (component-wise), which can
   diverge from raw-byte '/'-joined lexicographic order in edge cases. Plan explicitly anchors
   this sort to "matches the traversal sort in walk.rs" though, which already uses the same
   `PathBuf` ordering — so this matches spec as written, not a deviation. Left alone.
5. **(Nitpick)** `root.exists()` + `root.is_dir()` is two syscalls with a TOCTOU window;
   `std::fs::metadata` once would suffice.
6. **(Nitpick)** Manual push loop could be `results.extend(...)`.

## Auto-fixes applied

- Added `docs/knowledge/orchestration.md`, linked from `docs/knowledge/index.md`.
- Added `lint_bundle_dispatches_index_and_concept_checks_with_correct_is_root` test: builds a
  bundle with root `index.md` (bad frontmatter placement) and `sub/index.md` (also bad
  placement) plus a plain concept file with no frontmatter, asserting distinguishable
  diagnostics fire for both root and nested index files and for the concept file — verifying
  `is_root` threads correctly end-to-end, not just through the pure `classify()` function.
- Added `lint_bundle_read_permission_denied_is_io_error` test (`#[cfg(unix)]`), mirroring
  `walk.rs`'s existing pattern, with the same "skip if running privileged" guard.
- Replaced `root.exists()` + `root.is_dir()` with a single `std::fs::metadata` call.
- Replaced the manual push loop with `results.extend(...)`.

## Let go

- Path-sort component-wise-vs-byte-lexicographic nitpick (#4) — matches plan's explicit
  anchor to walk.rs's existing sort behavior, not a bug.
