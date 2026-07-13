# okf-lint: a linter for the Open Knowledge Format

`okf-lint` is a Rust CLI tool that walks a directory tree (an "OKF
bundle") and validates it against the **Open Knowledge Format (OKF)
v0.1** specification, plus a small set of general-purpose markdown
hygiene rules. It is designed to run in CI: it prints compiler-style
diagnostics and exits with a nonzero status if any violations are
found.

This document is self-contained. It does not assume the reader has
access to the upstream OKF spec or any prior conversation — everything
needed to implement the linter is inlined below.

## 1. Background: the OKF format

OKF (github.com/GoogleCloudPlatform/knowledge-catalog, `okf/SPEC.md`) is
a minimal, markdown + YAML-frontmatter format for representing curated
knowledge. The full spec covers a lot of soft, advisory guidance (SHOULD
recommendations, cross-linking conventions, etc.) that is **out of
scope for v1** of this linter — see §7 (Non-goals). This linter only
enforces the following structural facts about the format:

- A **bundle** is a directory tree of `.md` files. Subdirectories group
  related concepts and may nest arbitrarily.
- Two filenames are **reserved** at any level of the hierarchy:
  `index.md` and `log.md`. They have the defined structures below and
  must not be treated as ordinary "concept" documents.
- Every other `.md` file is a **concept document**. A concept document
  consists of a YAML frontmatter block delimited by a line containing
  exactly `---`, the YAML content, and a closing line containing
  exactly `---`, followed by a markdown body.
- **`index.md` structure:** an `index.md` file normally has no
  frontmatter at all. The one exception is the bundle-root `index.md`,
  which MAY have a frontmatter block containing only an `okf_version`
  key. The body of any `index.md` is one or more sections, each a
  markdown heading (`#`, `##`, etc.) followed by a bulleted list
  (`*` or `-` list items) of markdown links to other concepts or
  subdirectories. No other content (stray paragraphs, tables, etc.) is
  permitted in the body.
- **`log.md` structure:** a `log.md` file has no frontmatter. Its body
  is a flat sequence of date headings at the `##` level, each in
  `YYYY-MM-DD` ISO 8601 form, newest first, each followed by a bulleted
  list of change entries.

The upstream spec's conformance clause (§9), which is the authoritative
source for what this linter treats as a hard error, states:

> A bundle is conformant with OKF v0.1 if:
> 1. Every non-reserved `.md` file in the tree contains a parseable YAML
>    frontmatter block.
> 2. Every frontmatter block contains a non-empty `type` field.
> 3. Every reserved filename (`index.md`, `log.md`) follows the
>    structure described in §6 and §7 respectively when present.

This linter implements exactly those three rules (broken into the
discrete checks in §2 below) and nothing beyond them at the OKF level.
Everything else in the upstream spec — recommended fields like `title`/
`description`/`timestamp`, cross-link conventions, citations, tag
conventions — is advisory (SHOULD-level) and is explicitly **not**
checked by v1.

## 2. Requirements: OKF conformance checks (errors)

Each of these is a hard error. All are derived from OKF §9 above.

1. **Missing or unparseable frontmatter.** A non-reserved `.md` file
   whose content does not begin with a `---` line, or whose frontmatter
   block's closing `---` is never found, or whose content between the
   delimiters is not valid YAML.
   - Message: `missing or invalid YAML frontmatter`
   - Example failing input (`tables/orders.md`):
     ```markdown
     # Orders

     Some content with no frontmatter at all.
     ```

2. **Missing or empty `type` field.** A non-reserved `.md` file has a
   parseable frontmatter block, but the `type` key is absent, present
   with an empty string, or present with a non-string value (e.g. a
   number or list).
   - Message: `frontmatter missing required non-empty 'type' field`
   - Example failing input:
     ```markdown
     ---
     title: Orders
     ---

     # Schema
     ...
     ```

3. **Frontmatter in a non-root `index.md`.** Any `index.md` that is not
   at the bundle root, but contains a `---`-delimited frontmatter block.
   Also applies to the root `index.md` if its frontmatter contains any
   key other than `okf_version`.
   - Message: `index.md must not contain frontmatter` (non-root) or
     `root index.md frontmatter may only contain 'okf_version'` (root)
   - Example failing input (`tables/index.md`):
     ```markdown
     ---
     type: Index
     ---

     # Tables

     * [Orders](orders.md) - order records
     ```

4. **Malformed `index.md` body.** Any non-blank line in an `index.md`
   body that is neither a markdown heading (starts with one or more `#`
   followed by a space) nor a markdown list item (starts with `*`, `-`,
   or `+` followed by a space) nor a continuation/indented line of a
   list item. A bare paragraph is the canonical failing case.
   - Message: `index.md body line is not a heading or list item`
   - Example failing input:
     ```markdown
     # Tables

     This directory contains sales tables.

     * [Orders](orders.md) - order records
     ```
     (The paragraph line is the violation.)

5. **Malformed `log.md` date heading.** Any `##`-level heading in a
   `log.md` body whose text is not a valid `YYYY-MM-DD` date (must match
   the pattern exactly — four digits, `-`, two digits, `-`, two digits,
   and parse as a real calendar date).
   - Message: `log.md heading is not a valid YYYY-MM-DD date`
   - Example failing input:
     ```markdown
     # Directory Update Log

     ## May 22 2026
     * **Update**: Added new table reference.
     ```

## 3. Requirements: markdown style checks (errors)

These apply to **every** `.md` file in the bundle — concept documents,
`index.md`, and `log.md` alike, since they are generic hygiene rules,
not OKF structural rules.

1. **Max line length.** Any line (including blank-trimmed trailing
   whitespace, see rule 2) whose character count exceeds the configured
   maximum. Default: **100** characters. Overridable via the
   `--max-line-length <N>` CLI flag.
   - Message: `line exceeds maximum length of {N} characters ({actual}
     found)`

2. **Trailing whitespace.** Any line ending in one or more space or tab
   characters before the newline.
   - Message: `line has trailing whitespace`

3. **Trailing newline discipline.** The file does not end with exactly
   one newline character — i.e. either it has no trailing newline at
   all, or it ends with two or more consecutive newlines (trailing
   blank lines).
   - Message: `file must end with exactly one trailing newline`

4. **Consecutive blank lines.** Two or more consecutive blank lines
   anywhere within the file body (a run of 2+ lines that are empty or
   contain only whitespace).
   - Message: `multiple consecutive blank lines`

5. **Hard tabs.** Any line containing a literal tab character (`\t`)
   anywhere in its content.
   - Message: `line contains a hard tab character`

## 4. CLI behavior

- **Invocation:** `okf-lint <path>`, where `<path>` is the root
  directory of the bundle to check. The tool recurses into all
  subdirectories and checks every `.md` file found.
- **Diagnostics format:** one line per violation, compiler-style:
  `{relative/path/to/file.md}:{line}: {message}`
  Lines are relative to the `<path>` argument. If a check is not
  inherently line-scoped (e.g. rule 2 in §2, or rule 3 in §3), report it
  at line `1`.
- **Exit code:** `0` if zero diagnostics were emitted across the whole
  run; `1` if one or more diagnostics were emitted.
- **Flags:**
  - `--max-line-length <N>`: override the default of 100 for the
    max-line-length check (§3.1).
- **Explicitly out of scope for v1** (do not implement, and do not let
  a later planning phase add these without a new decision from the
  user):
  - JSON or other machine-readable output formats.
  - Config files (`.okflintrc` or similar) — flags only.
  - Autofix (`--fix`).
  - Any SHOULD-level OKF checks (recommended fields, cross-link
    validation, citation formatting, tag conventions).
  - Ignore-globs / per-file suppression.
  - Heading-structure linting (skipped heading levels, multiple H1s).

## 5. Testing expectations

The eventual TDD implementation plan should include:

- One fixture bundle (or minimal fixture file) per OKF conformance
  check in §2, in two variants: a passing case and a minimal failing
  case that triggers exactly that check and no others.
- One fixture file per markdown style check in §3, likewise in passing
  and minimal-failing variants.
- At least one whole-bundle integration test combining multiple concept
  documents, an `index.md`, and a `log.md`, asserting the full set of
  diagnostics produced matches expectations.
- A CLI-level test asserting the process exit code is `0` for a clean
  bundle and `1` for a bundle with at least one violation.
- A test for the `--max-line-length` flag overriding the default.

## 6. Non-goals

To keep v1 scope fixed, the following are explicitly **not** part of
this linter and should not be added during implementation planning
without a new decision from the user:

- SHOULD-level OKF checks (recommended frontmatter fields, cross-link
  validity, citation formatting, tag conventions, `okf_version`
  enforcement beyond the root-only placement rule in §2.3).
- Autofix / `--fix` mode.
- JSON or other structured output formats.
- Configuration file support.
- Ignore patterns / per-file or per-directory suppression.
- Markdown heading-structure linting (skipped levels, duplicate H1s).

## Next step

Once this spec is reviewed, generate the full implementation plan with:

```
/deep-plan @planning/okf-lint-spec.md
```
