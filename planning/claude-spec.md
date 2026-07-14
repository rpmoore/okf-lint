# okf-lint: combined specification

This document merges the original spec (`okf-lint-spec.md`), research
findings (`claude-research.md`), and interview answers
(`claude-interview.md`) into one complete, decision-closed
specification for the implementation plan. It supersedes the original
spec file wherever they overlap; where the original spec is silent,
this document adds the missing decision.

## 1. Summary

`okf-lint` is a Rust CLI that walks a directory tree (an "OKF bundle")
and validates it against the Open Knowledge Format (OKF) v0.1
specification's MUST-level conformance rules, plus a fixed set of
markdown hygiene rules. CI-friendly: compiler-style diagnostics,
nonzero exit on violations.

## 2. OKF background

(Unchanged from original spec §1 — see `okf-lint-spec.md` §1 for the
full inlined restatement of bundle structure, reserved filenames,
concept-document structure, `index.md`/`log.md` structure, and the
upstream §9 conformance clause. Not repeated here to avoid drift; the
implementation plan should treat `okf-lint-spec.md` §1 as authoritative
background.)

## 3. OKF conformance checks (errors)

Unchanged from original spec §2, checks 1–5 (missing/invalid
frontmatter, missing/empty `type`, frontmatter in non-root index.md,
malformed index.md body, malformed log.md date heading). See
`okf-lint-spec.md` §2 for exact trigger conditions, messages, and
examples — all five carry forward as-is.

**Added precision (from interview Q5):** for rule 4 (malformed
`index.md` body), a line counts as a valid "continuation" of the
preceding list item (not a violation) if it is indented by **2 or more
leading spaces** and immediately follows a list-item line (directly, or
via other continuation lines of the same item). Any non-blank line that
is not a heading, not a list item, and not a >=2-space-indented
continuation of a list item is the violation.

## 4. Markdown style checks (errors)

Unchanged from original spec §3, rules 1–5 (max line length, trailing
whitespace, trailing newline discipline, consecutive blank lines, hard
tabs). Applies to every `.md` file. See `okf-lint-spec.md` §3 for exact
messages.

**Added precision (from interview):**
- **Q8 — line length counting:** count by Unicode scalar values
  (`char` count, i.e. `line.chars().count()`), not byte length. A
  multi-byte UTF-8 character (e.g. `é`) counts as 1 toward the limit.
- **Q9 — CRLF handling:** files are read as raw text and split on `\n`
  only (not `\r\n`). A line ending in `\r` before the `\n` is caught by
  the existing trailing-whitespace rule (3.2) — `\r` is treated the same
  as a trailing space/tab. No separate CRLF-specific rule; this is
  simply the existing rule applied to an already-`\n`-split line that
  still has a trailing `\r`. Net effect: CRLF-terminated files are
  flagged as having trailing whitespace on every line, functionally
  enforcing LF-only line endings.
- **Q3 — empty file (0 bytes):** an empty `.md` file has zero trailing
  newlines, and is therefore a violation of style rule 3.3 ("file must
  end with exactly one trailing newline"), reported at line 1. No
  special-case exemption.

## 5. Directory traversal (new — not in original spec)

- Recurse under `<path>` using `walkdir` (see research §3), filtering to
  files with a `.md` extension.
- **Skip hidden entries:** any directory or file whose name starts with
  `.` (e.g. `.git`, `.github`) is skipped entirely — do not descend into
  it, do not lint any `.md` file inside it. (Interview Q2.)
- **Do not follow symlinks** — `walkdir`'s default (non-`follow_links`)
  behavior. Symlinked files/dirs are not traversed into. (Interview Q2.)
- Non-`.md` files are ignored entirely (not checked by any rule).
- Collect all matching file paths and **sort them** (lexicographic path
  sort) before checking, to guarantee deterministic diagnostic
  ordering (see §7 below) independent of filesystem iteration order.

## 6. CLI behavior

Carries forward original spec §4 (invocation `okf-lint <path>`,
diagnostics format `{relative/path}:{line}: {message}`, `--max-line-length
<N>` flag default 100, out-of-scope list). **Additions from interview:**

- **Exit codes (interview Q4):**
  - `0` — clean run, zero diagnostics.
  - `1` — one or more lint diagnostics were emitted (bundle is
    non-conformant and/or has style violations).
  - `2` — usage/IO error: `<path>` does not exist, `<path>` is not a
    directory, or a file under `<path>` could not be read as valid
    UTF-8 (or any other I/O error reading a file). These are reported
    to **stderr** (not as `file:line:` diagnostics) and short-circuit
    the run — a single such error is enough to exit 2 immediately
    without attempting to produce partial lint output. This is
    distinct from exit 1 so CI can tell "tool couldn't run" apart from
    "tool ran and found violations."
- **Multiple diagnostics per line (interview Q6):** if a single line
  trips more than one rule (e.g. both too-long and trailing-whitespace),
  emit one diagnostic line per violated rule — do not stop at the first
  match on a line.
- **Diagnostic ordering (interview Q7):** the full diagnostic output for
  a run is sorted by file path (matching the traversal sort in §5),
  then by line number within a file. Where a single file:line has
  multiple diagnostics (per the point above), their relative order
  should be stable and deterministic (e.g. OKF-check diagnostics before
  style-check diagnostics, in the fixed rule order given in §3/§4) — the
  plan should pick and document one fixed sub-order.

## 7. Crate choices (from research, confirmed in interview)

- **YAML parsing:** `serde_yaml_ng` (interview Q1) — parse frontmatter
  into a `serde_yaml_ng::Value`, check for a `type` key mapping to a
  non-empty string.
- **CLI parsing:** `clap` 4.6.x, derive API (research §2).
- **Directory walking:** `walkdir` 2.5.0 (research §3 / interview Q2).
- **Testing:** `assert_cmd` + `predicates` + `insta` for CLI-level tests
  (research §4).

## 8. Testing expectations

Carries forward original spec §5 unchanged (fixture-per-rule,
pass/fail variants, whole-bundle integration test, CLI exit-code test,
`--max-line-length` override test). **Layout (from research):** unit
tests inline in `src/` modules; integration/CLI tests in `tests/`
using fixture bundles/files under `tests/fixtures/`.

## 9. Non-goals

Unchanged from original spec §6 — no SHOULD-level OKF checks, no
autofix, no JSON output, no config files, no ignore patterns, no
heading-structure linting.

## 10. Open items intentionally left to the plan-writing step

None — all ambiguities surfaced during research/interview were
resolved with explicit decisions above (§5–§7). The implementation plan
should not need to make further judgment calls; if it finds one, that's
a signal this spec missed something and should be revisited.
