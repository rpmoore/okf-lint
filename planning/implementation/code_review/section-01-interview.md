# Code Review Interview: Section 01 - Foundation

**Date:** 2026-07-13

## Discussed with User

### CRLF handling in `split_frontmatter`

**Issue:** `split_frontmatter` splits on `\n` and does exact `line == "---"` matching. A CRLF-terminated file has `"---\r"` as the delimiter line, so it silently fails to detect valid frontmatter (misclassified as `None`/`Unclosed`, not surfaced as an error). Not addressed by the plan/spec.

**Decision:** Trim a trailing `\r` before comparing delimiter lines and before treating a line as blank, so CRLF is handled the same as LF — consistent with how `checks/style.rs` (later sections) already treats `\r` as a line-ending artifact rather than meaningful content.

**Status:** Fixing.

## Auto-Fixes (no discussion needed)

1. **CLAUDE.md `docs/knowledge/` requirement.** Project instructions (CLAUDE.md) mandate creating/updating OKF knowledge docs under `docs/knowledge/` for the section of code touched, after every code change. This is a hard process requirement, not a judgment call — creating `docs/knowledge/index.md` and `docs/knowledge/foundation.md` documenting `diagnostic.rs`, `frontmatter.rs`, `lint.rs`, `walk.rs`.

2. **Permission-denied test fragile under root.** `permission_denied_subdirectory_is_io_error` silently doesn't test what it claims when run as root (root bypasses Unix permission bits, so `chmod 0o000` has no effect). Adding a runtime check: after `chmod`, verify the directory is actually unreadable before asserting on the walk result; if it's still readable (running as root), skip the test with an explanation rather than asserting on a codepath that wasn't actually exercised.

3. **Temp-dir cleanup skipped on panic.** The hand-rolled temp-dir helper in `walk.rs` tests uses `fs::remove_dir_all` at the end of the test body, which is skipped if an earlier assertion panics, leaking `okf-lint-walk-test-*` directories. Switching to the `tempfile` crate (already a transitive dependency via `insta`/`assert_cmd`; the plan's own text explicitly permits adding it as a direct dev-dependency) for RAII-based cleanup that runs even on panic.

4. **`#[allow(dead_code)]` suppresses intended warnings.** The plan says to "expect and ignore" (i.e., leave visible) the dead-code warnings on `LintError`'s three not-yet-used variants until section-06 wires them up. The blanket `#[allow(dead_code)]` on the enum silences that signal — removing it so the warnings show as intended (they don't fail the build).
