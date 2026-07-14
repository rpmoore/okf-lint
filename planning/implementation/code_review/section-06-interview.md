# Section 06 (orchestration) review interview

No findings had real tradeoffs or security implications requiring user input — all
actionable ones auto-fixed.

## Auto-fixes applied

1. Created `docs/knowledge/orchestration.md`, linked from `docs/knowledge/index.md`.
2. Added `lint_bundle_dispatches_index_and_concept_checks_with_correct_is_root` test,
   exercising root index.md, nested sub/index.md, and a concept file end-to-end through
   `lint_bundle` (not just the pure `classify()` function).
3. Added `lint_bundle_read_permission_denied_is_io_error` test (unix-gated, mirrors
   walk.rs's existing pattern with a privilege-skip guard).
4. Replaced `root.exists()` + `root.is_dir()` with a single `std::fs::metadata` call
   (avoids TOCTOU window, one syscall instead of two).
5. Replaced manual push loop with `results.extend(...)`.

## Let go

- Path-sort component-wise-vs-byte-lexicographic nitpick — plan explicitly anchors this
  sort to "matches the traversal sort in walk.rs", which already uses the same `PathBuf`
  ordering. Not a deviation from spec.

Verified: `cargo test` → 77 passed, 0 failed. `cargo clippy --all-targets` → only expected
dead-code warnings (main.rs doesn't call lint_bundle yet; that's section-07).
