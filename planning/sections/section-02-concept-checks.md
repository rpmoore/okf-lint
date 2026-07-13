# section-02-concept-checks

## Objective

Implement `check_concept`, the OKF conformance checker for "concept
documents" — any `.md` file in an OKF bundle that is not `index.md` or
`log.md` (see file classification in the orchestration section). This
covers two rules:

- **Rule 1 — `OkfMissingFrontmatter`**: the file must begin with a
  well-formed `---`-delimited YAML frontmatter block whose parsed value
  is a YAML mapping.
- **Rule 2 — `OkfMissingType`**: that frontmatter mapping must contain a
  `type` key with a non-empty string value.

## Dependencies

This section depends on **section-01-foundation**, which must already
provide:

- `src/diagnostic.rs` with:
  ```rust
  struct Diagnostic {
      line: usize,     // 1-based
      rule: Rule,
      message: String,
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
  (This section only uses the `OkfMissingFrontmatter` and
  `OkfMissingType` variants; the rest exist for other sections and for
  the fixed tie-break ordering.)
- `src/frontmatter.rs` with:
  ```rust
  enum FrontmatterResult {
      None,                    // content doesn't start with a "---" line
      Unclosed,                // starts with "---" but no closing "---" line found
      Found { yaml_text: String, body_start_line: usize },
  }

  fn split_frontmatter(content: &str) -> FrontmatterResult
  ```
  `content` must literally begin with a line that is exactly `---` (no
  leading blank lines, no trailing characters on that line) for
  `Found`/`Unclosed` to apply; anything else is `None`. When `Found`,
  `yaml_text` is the raw, unparsed text between the two `---` delimiter
  lines.
- The `serde_yaml_ng` (0.10.x) dependency already added to `Cargo.toml`.

Do not re-implement or duplicate any of the above — import them from
their respective modules (`crate::diagnostic::{Diagnostic, Rule}`,
`crate::frontmatter::{split_frontmatter, FrontmatterResult}`).

## Files to create/modify

- `src/checks/mod.rs` — create if it doesn't already exist (as an empty
  module file / `pub mod okf;` declaration owner); add `pub mod okf;`
  to it.
- `src/checks/okf.rs` — new file, contains `check_concept` and its
  inline `#[cfg(test)] mod tests`.
- `tests/fixtures/okf/missing_frontmatter/pass/pass.md`
- `tests/fixtures/okf/missing_frontmatter/fail/fail.md`
- `tests/fixtures/okf/missing_type/pass/pass.md`
- `tests/fixtures/okf/missing_type/fail/fail.md`

Each `pass/`/`fail/` pair is its own single-file mini-bundle: `pass.md`
and `fail.md` live in **separate sibling directories** (`pass/` vs
`fail/`), not side by side in the same directory. This matters for the
CLI-level integration tests in later sections (running the CLI against
`pass/` or `fail/` as `<path>` checks exactly one file), but for this
section's own unit tests you'll just read the fixture file contents
directly with `std::fs::read_to_string` and pass them to
`check_concept`.

## Function signature

```rust
fn check_concept(content: &str) -> Vec<Diagnostic>
```

## Rule semantics

### Rule 1 — `OkfMissingFrontmatter`

Call `split_frontmatter(content)`.

- `FrontmatterResult::None` or `FrontmatterResult::Unclosed` → emit
  exactly one diagnostic: line `1`, message `missing or invalid YAML
  frontmatter`, and **stop** — rule 2 does not run in this case (there's
  no parseable frontmatter for it to inspect).
- `FrontmatterResult::Found { yaml_text, .. }` → attempt to parse
  `yaml_text` as YAML via `serde_yaml_ng`.
  - A YAML syntax/parse error → **also** rule 1: same message (`missing
    or invalid YAML frontmatter`), same line `1`. Stop (no rule 2).
  - A successfully-parsed value that is **not** a YAML mapping (e.g. a
    scalar, a list) → **also** rule 1, same message/line. Stop (no rule
    2).
  - A successfully-parsed YAML mapping → rule 1 does not fire; proceed
    to rule 2.

"Unparseable" for rule 1's purposes covers both syntactic YAML errors
and structurally-wrong-shape frontmatter (mapping required, anything
else is a violation).

### Rule 2 — `OkfMissingType`

Only reached when rule 1 did not fire (i.e. frontmatter was `Found` and
parsed to a mapping). Look up the `type` key in the parsed mapping:

- Key missing entirely → violation.
- Key present with an empty string value (`type: ""`) → violation.
- Key present with any non-string YAML value (number, bool, list,
  mapping, null) → violation.
- Key present with a non-empty string value → **no** violation, `Vec`
  is empty.

On violation: emit exactly one diagnostic, line `1`, message
`frontmatter missing required non-empty 'type' field`.

Only ever 0 or 1 diagnostics total come out of `check_concept` for a
given input (rule 1 and rule 2 are mutually exclusive by construction —
rule 2 can't fire without rule 1 having already passed).

## Tests (write first, per TDD)

Write these as `#[cfg(test)] mod tests` inside `src/checks/okf.rs`
(fixture-backed tests read fixture files with
`std::fs::read_to_string`, relative to the crate's `tests/fixtures/`
directory via e.g. `include_str!` or a runtime path join — pick
whichever convention section-01 established, or default to
`include_str!("../../tests/fixtures/okf/.../pass.md")`-style relative
includes since these are compile-time-known fixture paths).

Fixture-backed:

- `missing_frontmatter/pass/pass.md` → `check_concept` returns no
  `OkfMissingFrontmatter`/`OkfMissingType` diagnostics. (This fixture
  should have well-formed frontmatter with a non-empty `type` field, so
  it also implicitly passes rule 2 — build it as a normal valid concept
  doc.)
- `missing_frontmatter/fail/fail.md` → exactly one
  `OkfMissingFrontmatter` diagnostic at line 1 with the exact message
  `missing or invalid YAML frontmatter`. (Build this fixture with no
  frontmatter at all, or an unclosed block.)
- `missing_type/pass/pass.md` → no diagnostics (valid frontmatter, `type`
  present and non-empty).
- `missing_type/fail/fail.md` → exactly one `OkfMissingType` diagnostic
  at line 1 with the exact message `frontmatter missing required
  non-empty 'type' field`. (Build this fixture with well-formed
  frontmatter that's missing `type`, or has it empty.)

Inline-literal edge cases (construct content as Rust string literals,
no fixture files needed):

- Frontmatter present but not closed (`FrontmatterResult::Unclosed`) →
  `OkfMissingFrontmatter` fires, and `OkfMissingType` does **not** also
  fire (assert the returned `Vec` has length 1, only the missing-
  frontmatter diagnostic).
- Frontmatter parses as valid YAML but the top-level value is a scalar
  or a list, not a mapping (e.g. `---\njust a string\n---\nbody`) →
  `OkfMissingFrontmatter` (the structurally-wrong-shape case).
- `type` present with a non-string value, e.g. `type: 5` or
  `type: [a, b]` → `OkfMissingType`.
- `type` present as an empty string, `type: ""` → `OkfMissingType`.
- `type` present and non-empty (e.g. `type: concept`) → no diagnostics
  at all.

Each test should assert on the exact `Diagnostic` contents (line
number, `Rule` variant, and exact message string) — not just the count
— per the plan's "assert the exact `Diagnostic` set produced" testing
philosophy.

## Implementation notes

- `check_concept` is a pure function over `&str` — no file I/O inside
  it. File reading is the caller's (orchestration section's)
  responsibility.
- Use `serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml_text)` (or
  equivalent) to parse, then match on whether the resulting `Value` is
  `Value::Mapping(_)`; if so, look up the `type` key inside that
  mapping and check whether its value is `Value::String(s)` with
  `!s.is_empty()`.
- Both diagnostics this module ever produces are anchored at line `1`
  — there is no line-tracking logic needed here, unlike the style
  checks or `index.md`/`log.md` checks in other sections.
- Keep the two rules as sequential logic inside one function (rule 2's
  check only runs in the mapping-parsed branch of rule 1's logic) —
  don't split into two separately-callable functions, since `check_concept`
  is the single public entry point this module exposes (matching the
  signature above and the module layout in the plan).

## Implementation Notes (post-review)

Implemented as specified, with the following additions from code review:

- **Key lookup simplified.** `mapping.get("type")` (via `serde_yaml_ng`'s
  `Index` impl for `str`) instead of allocating a `Value::String("type")`
  per call.
- **New test: `invalid_yaml_syntax_fires_rule_1`.** The original test set
  covered `None`/`Unclosed` and "well-formed but not a mapping" as ways
  into `OkfMissingFrontmatter`, but never exercised a genuine YAML syntax
  error (e.g. an unclosed flow mapping) — added to close that gap.
- **`docs/knowledge/concept-checks.md`** added and linked from
  `docs/knowledge/index.md`, per CLAUDE.md.