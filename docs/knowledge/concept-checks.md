---
type: module
---

# Concept checks

OKF conformance rules for ordinary concept documents (any `.md` file that
isn't `index.md` or `log.md`).

## `src/checks/okf.rs`

- `check_concept(content: &str) -> Vec<Diagnostic>` — implements two rules:
  - **`OkfMissingFrontmatter`**: fires when `split_frontmatter` returns
    `None`/`Unclosed`, when the frontmatter YAML fails to parse, or when it
    parses to something other than a mapping. Stops here — rule 2 doesn't
    run.
  - **`OkfMissingType`**: only reached once frontmatter parsed to a mapping.
    Fires when the `type` key is missing, empty, or not a string.
- Both diagnostics are anchored at line 1 — there's no line-tracking in
  this module, unlike `index.md`/`log.md`/style checks.
- Pure function over `&str`; no file I/O. The orchestration layer is
  responsible for reading files and calling this for `Concept`-classified
  paths.
