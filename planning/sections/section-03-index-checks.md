# section-03-index-checks: `index.md` OKF Conformance Checks

## Summary

Implement `checks/index_md.rs` with a single public function
`check_index`, covering two OKF conformance rules that apply
specifically to `index.md` files:

- **Rule 3 — `OkfIndexFrontmatterPlacement`**: `index.md` files must
  not contain frontmatter, except that the *root* `index.md` may
  contain frontmatter with only an `okf_version` key.
- **Rule 4 — `OkfIndexBodyStructure`**: the body of `index.md` (the
  content after any frontmatter) must consist only of headings and
  list items (with valid continuation lines).

## Dependencies

This section depends on **section-01-foundation**, which must already
provide:

- `src/frontmatter.rs`: `split_frontmatter(content: &str) -> FrontmatterResult`,
  where
  ```rust
  enum FrontmatterResult {
      None,
      Unclosed,
      Found { yaml_text: String, body_start_line: usize },
  }
  ```
  `content` must literally begin with a line that is exactly `---`
  (no leading blank lines, no trailing characters) for
  `Found`/`Unclosed` to apply; anything else is `None`. `yaml_text` is
  the raw, unparsed text between the two `---` delimiter lines.
  `body_start_line` is the 1-based line number of the first line after
  the closing `---`.
- `src/diagnostic.rs`: the `Diagnostic` struct and `Rule` enum:
  ```rust
  struct Diagnostic {
      line: usize,     // 1-based
      rule: Rule,
      message: String, // exact text, already formatted
  }

  enum Rule {
      OkfMissingFrontmatter,
      OkfMissingType,
      OkfIndexFrontmatterPlacement,
      OkfIndexBodyStructure,
      OkfLogDateHeading,
      StyleLineLength,
      StyleTrailingWhitespace,
      StyleTrailingNewline,
      StyleConsecutiveBlankLines,
      StyleHardTab,
  }
  ```

This section does **not** need `walk.rs`, `lint.rs`, or the other
`checks/*.rs` modules — it is an independent, self-contained module
that only needs `frontmatter.rs` and `diagnostic.rs`. It does not
determine file classification (root vs. non-root `index.md`, or
whether a file is `index.md` at all) — that is the caller's
responsibility (handled later by `lint.rs` in
section-06-orchestration), so `check_index` takes `is_root: bool` as
an explicit parameter rather than inspecting a path itself.

YAML parsing uses the `serde_yaml_ng` crate, which section-01 will
have already added to `Cargo.toml`.

## File to create

`src/checks/index_md.rs`

(This assumes `src/checks/mod.rs` already exists from section-01 or
a sibling section; if it declares only `pub mod okf;` so far, add
`pub mod index_md;` to it. If `checks/mod.rs` does not yet exist,
create it with `pub mod index_md;`.)

## Public API

```rust
/// Runs OKF conformance rules 3 (OkfIndexFrontmatterPlacement) and 4
/// (OkfIndexBodyStructure) against the content of an index.md file.
/// `is_root` is true iff this index.md is directly at the bundle root
/// (no parent path component) — the caller determines this via file
/// classification (see lint.rs, section-06), not this function.
fn check_index(content: &str, is_root: bool) -> Vec<Diagnostic>
```

## Rule semantics

### Rule 3 — `OkfIndexFrontmatterPlacement`

Call `split_frontmatter(content)`:

- `FrontmatterResult::None` → no violation. Proceed to rule 4 using
  the whole `content` as the body (no line offset).
- `FrontmatterResult::Found` or `FrontmatterResult::Unclosed` on a
  **non-root** `index.md` → emit one diagnostic, line 1, message
  exactly `index.md must not contain frontmatter`. Note: `Unclosed` is
  treated the same as `Found` here — the file visibly starts a
  frontmatter block (a line that is exactly `---`), which is itself
  the violation regardless of whether the block is well-formed. This
  is a plan-level gap-filling decision (the original spec only
  describes the fully-formed case).
- `FrontmatterResult::Found` on the **root** `index.md` → parse
  `yaml_text` as a YAML mapping (via `serde_yaml_ng`). If parsing
  fails, or the resulting mapping contains any key other than
  `okf_version`, emit one diagnostic, line 1, message exactly `root
  index.md frontmatter may only contain 'okf_version'`. A mapping
  containing only `okf_version` (or an empty mapping) is fine — no
  violation.
- `FrontmatterResult::Unclosed` on the **root** `index.md` → since it
  can't be parsed as a mapping at all, treat it the same as the
  extra-key case above: emit `root index.md frontmatter may only
  contain 'okf_version'` at line 1 (same gap-filling rationale as the
  non-root `Unclosed` case).

### Rule 4 — `OkfIndexBodyStructure`

The body to scan is:
- The whole `content`, if `split_frontmatter` returned `None`.
- Otherwise, the text starting at `body_start_line` (i.e., this still
  runs even when rule 3 also fired — rule 3 and rule 4 are
  **independent checks and both can fire** on the same file, e.g. a
  non-root `index.md` with both bad frontmatter and a stray
  paragraph; the diagnostics ordering that puts rule 3 before rule 4
  on a shared line is established in `lint.rs`'s sort step, out of
  scope for this section).

Scan the body line by line, 1-indexed starting from `body_start_line`
(or 1 if there was no frontmatter). Maintain a single boolean
`in_list_item`, initialized `false`, reset to `false` on every blank
line. For each non-blank line, in order, apply the first matching
rule:

1. Matches `^#+ ` (one or more `#` then a space) → heading; valid, set
   `in_list_item = false`.
2. Matches `^[*+-] ` (a list marker `*`, `+`, or `-`, then a space) →
   list item; valid, set `in_list_item = true`.
3. Otherwise, if `in_list_item` is `true` **and** the line has at
   least 2 leading space characters → continuation line; valid,
   `in_list_item` stays `true`.
4. Otherwise → violation: emit a diagnostic at this line's number,
   message exactly `index.md body line is not a heading or list
   item`; `in_list_item` becomes `false` (a violating line does not
   count as part of a list item for a *following* line's continuation
   check).

Each violating line gets its own diagnostic — a multi-line stray
paragraph produces one diagnostic per line, not one per paragraph.

## Tests (write first, TDD)

Write these as fixture-backed tests (see fixture layout below) plus
inline-literal edge cases, in an inline `#[cfg(test)] mod tests` block
in `src/checks/index_md.rs`.

Fixture-backed:

- `pass_root/` (root `index.md`, no frontmatter, or `okf_version`-only
  frontmatter) → `check_index` (with `is_root = true`) returns no
  `OkfIndexFrontmatterPlacement` diagnostics.
- `fail_nonroot/` (non-root `index.md` with frontmatter) →
  `check_index` (with `is_root = false`) returns exactly one
  `OkfIndexFrontmatterPlacement` diagnostic, message `index.md must
  not contain frontmatter`.
- `fail_root_extra_key/` (root `index.md` with frontmatter containing
  a key besides `okf_version`) → `check_index` (with `is_root = true`)
  returns exactly one `OkfIndexFrontmatterPlacement` diagnostic,
  message `root index.md frontmatter may only contain 'okf_version'`.
- `index_body_structure/pass/pass.md` → no `OkfIndexBodyStructure`
  diagnostics.
- `index_body_structure/fail/fail.md` → one `OkfIndexBodyStructure`
  diagnostic per violating body line, at the correct line numbers.

Inline-literal edge cases:

- Root `index.md` with `Unclosed` frontmatter (a `---` line with no
  closing `---`) → same `root index.md frontmatter may only contain
  'okf_version'` diagnostic.
- Non-root `index.md` with `Unclosed` frontmatter → same `index.md
  must not contain frontmatter` diagnostic.
- Heading line, then list item, then an indented (2+ space)
  continuation line → no violation (continuation accepted).
- An indented line when `in_list_item` is `false` (no preceding list
  item) → violation (continuation only valid immediately after a list
  item).
- A blank line resets `in_list_item`, so a subsequent indented line is
  a violation, not treated as a continuation.
- A non-root `index.md` with **both** bad frontmatter and a stray body
  paragraph → both an `OkfIndexFrontmatterPlacement` and an
  `OkfIndexBodyStructure` diagnostic are produced (rules 3 and 4 are
  independent and both fire in a single `check_index` call).

## Fixtures to create

Under `tests/fixtures/okf/`:

```
index_frontmatter_placement/
  pass_root/          # small directory: a root index.md with no
                       # frontmatter, or okf_version-only frontmatter
  fail_nonroot/        # a directory containing (at minimum) an
                       # index.md with frontmatter, representing a
                       # non-root index.md (e.g. nested under a
                       # subdirectory, or just tested with is_root=false
                       # directly in the unit test — see note below)
  fail_root_extra_key/ # a root index.md whose frontmatter has a key
                       # other than okf_version
index_body_structure/
  pass/pass.md         # index.md body consisting only of valid
                        # headings/list items/continuations
  fail/fail.md         # index.md body containing at least one stray
                        # paragraph line that is not a heading/list
                        # item/valid continuation
```

Note on `pass_root/`, `fail_nonroot/`, and `fail_root_extra_key/`:
these are small directories (root `index.md` plus, where relevant, a
subdirectory `index.md`) because rule 3's root-vs-non-root distinction
is a property of a file's *path within a bundle*, not something
`check_index` itself computes — `check_index` just takes `is_root` as
a parameter. For this section's unit tests (which call `check_index`
directly, not through the CLI), it is sufficient to read the fixture
file's content and pass the appropriate `is_root` boolean explicitly;
the directory structure under `tests/fixtures/okf/
index_frontmatter_placement/` exists primarily to also serve
section-08-integration-tests and any later CLI-level tests, so build
it as a real mini-bundle directory (not just a loose file) even though
this section's own tests only need the file content.

Each `pass/`/`fail/` pair (and `pass_root/`/`fail_nonroot/`/
`fail_root_extra_key/`) should be usable as a standalone single-file
(or small) mini-bundle: if later run via the CLI with the fixture
directory as `<path>`, exactly the intended file(s) are checked.

## Out of scope for this section

- File classification (root vs. non-root determination from a real
  path, `index.md` vs. `log.md` vs. concept-doc dispatch) — that is
  section-06-orchestration's `lint.rs`.
- Wiring `check_index` into the overall `lint_bundle` pipeline —
  also section-06.
- Diagnostic sort ordering across files/rules (plan §7) — also
  section-06, though this section's diagnostics must use the correct
  `Rule` variants (`OkfIndexFrontmatterPlacement`,
  `OkfIndexBodyStructure`) so that later sorting works correctly.

---

**File paths touched by this section:**
- `/home/rpmoore/code/okf-lint/src/checks/index_md.rs` (create)
- `/home/rpmoore/code/okf-lint/src/checks/mod.rs` (modify: add `pub mod index_md;`)
- `/home/rpmoore/code/okf-lint/tests/fixtures/okf/index_frontmatter_placement/pass_root/` (create)
- `/home/rpmoore/code/okf-lint/tests/fixtures/okf/index_frontmatter_placement/fail_nonroot/` (create)
- `/home/rpmoore/code/okf-lint/tests/fixtures/okf/index_frontmatter_placement/fail_root_extra_key/` (create)
- `/home/rpmoore/code/okf-lint/tests/fixtures/okf/index_body_structure/pass/pass.md` (create)
- `/home/rpmoore/code/okf-lint/tests/fixtures/okf/index_body_structure/fail/fail.md` (create)
- `/home/rpmoore/code/okf-lint/docs/knowledge/index-checks.md` (create, per CLAUDE.md)
- `/home/rpmoore/code/okf-lint/docs/knowledge/index.md` (modify: add link)

## As-built notes

Implemented as planned, with these clarifications settled during code review:

- **Unclosed frontmatter and rule 4**: `FrontmatterResult::Unclosed` carries no
  `body_start_line`, so rule 4 (`OkfIndexBodyStructure`) is skipped entirely when frontmatter
  is unclosed — the whole remainder of the file is treated as part of the broken frontmatter
  block rather than a scannable body. Confirmed with the plan owner; a non-root `index.md`
  with unclosed frontmatter and a garbage body reports exactly one diagnostic
  (`OkfIndexFrontmatterPlacement`), not one per garbage line.
- `fail_nonroot/` was built as a two-file mini-bundle (`index.md` at the root, plain/valid;
  `sub/index.md` nested, with frontmatter) rather than a single loose file, matching the
  spec's note that these fixtures should double as real mini-bundles for later CLI/integration
  tests. The unit test in this section reads `fail_nonroot/sub/index.md` directly and calls
  `check_index(.., is_root: false)`.
- Test coverage extended beyond the plan's explicit list (added during code review) to also
  cover: a multi-line stray paragraph (locks in "one diagnostic per line, not per paragraph"),
  a root `index.md` with no frontmatter at all, and a root `index.md` with an empty
  frontmatter mapping (`Value::Null` path). Final count: 15 tests in
  `src/checks/index_md.rs` (up from the ~11 the plan enumerated).
- `check_index` is not yet called from anywhere (expected — wiring into `lint.rs` is
  section-06's job); `cargo clippy` reports it and its helpers as dead code, which is expected
  at this stage.