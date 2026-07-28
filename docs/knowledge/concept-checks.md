---
type: module
---

# Concept checks

OKF conformance rules for ordinary concept documents (any `.md` file that
isn't `index.md` or `log.md`).

## `src/checks/okf.rs`

- `check_concept(content: &str) -> Vec<Diagnostic>` — implements the two
  core conformance rules plus six v0.2 optional-field format checks. All are
  independent (unlike the v0.1 pair, a later check still runs even if an
  earlier one fires) and all diagnostics are anchored at line 1 — there's no
  line-tracking in this module, unlike `index.md`/`log.md`/style checks.
  - **`OkfMissingFrontmatter`**: fires when `split_frontmatter` returns
    `None`/`Unclosed`, when the frontmatter YAML fails to parse, or when it
    parses to something other than a mapping. Stops here — nothing else in
    this function runs.
  - **`OkfMissingType`**: fires when the `type` key is missing, empty, or
    not a string. `type_value: Option<&str>` is captured once up front (also
    feeding the Attested-Computation `runtime` check below) rather than
    re-extracted per rule.
  - **OKF v0.2** (SPEC.md §5, §10) added optional provenance/trust/lifecycle
    frontmatter families and a new `Attested Computation` concept type. Both
    are additive — §11 conformance is unchanged, so none of these keys are
    ever *required* — but a field that IS present with the wrong shape is a
    formatting error:
    - **`OkfInvalidSources`**: `sources`, if present, must be a YAML
      sequence; each entry must be a mapping with a non-empty `resource`
      (§5.1). One diagnostic per malformed entry, message includes the
      0-based index (e.g. `'sources' entry 0 is missing required...`), or a
      single un-indexed diagnostic if `sources` itself isn't a sequence.
    - **`OkfInvalidGenerated`**: `generated`, if present, must be a mapping
      with a non-empty `by` (§5.2's `generated.by: REQUIRED`).
    - **`OkfInvalidVerified`**: `verified`, if present, must be a mapping
      with `by`+`at`, or a sequence of such mappings — the spec's "bare
      mapping MUST be treated as a one-element list" rule (§5.2) means both
      shapes are accepted. One diagnostic per malformed list entry (indexed)
      or a single message for a malformed bare mapping / wrong top-level
      type.
    - **`OkfInvalidStatus`**: `status`, if present, must be exactly `draft`,
      `stable`, or `deprecated` (§5.4).
    - **`OkfInvalidStaleAfter`**: `stale_after`, if present, must parse as a
      real `YYYY-MM-DD` calendar date via `crate::date::parse_ymd` (§5.5) —
      same shape+parse logic `log.md` date headings use (`src/date.rs`).
    - **`OkfAttestedComputationMissingRuntime`**: only checked when
      `type == "Attested Computation"` exactly; every other type is
      untouched. Fires when `runtime` is missing or empty (§10.2:
      `runtime: REQUIRED for this type`).
  - `sources`/`generated`/`verified` entries validate structurally only
    (right YAML shape, required sub-key present and non-empty) — no deeper
    semantic validation (e.g. `at` is checked for presence, not parsed as a
    real ISO 8601 datetime; unlike `stale_after`, which does get full date
    parsing since a malformed calendar date is unambiguously wrong).
- Pure function over `&str`; no file I/O. The orchestration layer is
  responsible for reading files and calling this for `Concept`-classified
  paths.
