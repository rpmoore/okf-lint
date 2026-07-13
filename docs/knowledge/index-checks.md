---
type: module
---

# Index checks

OKF conformance rules for `index.md` files.

## `src/checks/index_md.rs`

- `check_index(content: &str, is_root: bool) -> Vec<Diagnostic>` — implements
  two rules. `is_root` is supplied by the caller (file classification lives
  in `lint.rs`, not here); this module has no notion of paths.
  - **`OkfIndexFrontmatterPlacement`**: non-root `index.md` must not have
    frontmatter (`Found` or `Unclosed` both count as "has frontmatter" — a
    visible `---` opening line is itself the violation, malformed or not).
    Root `index.md` may have frontmatter only if it parses as a YAML mapping
    containing no key other than `okf_version` (an empty mapping, or no
    frontmatter at all, is fine). Root `Unclosed` frontmatter is treated the
    same as a bad mapping, since it can't be parsed at all.
  - **`OkfIndexBodyStructure`**: scans the body (from `body_start_line`, or
    line 1 if there's no frontmatter) line by line. A boolean
    `in_list_item` tracks whether the previous non-blank line was a list
    item; it resets on every blank line. Each non-blank line must be a
    heading (`^#+ `), a list item (`^[*+-] `), or — only while
    `in_list_item` is true — a continuation line indented by 2+ spaces.
    Anything else is one diagnostic per violating line.
  - **Judgment call**: when frontmatter is `Unclosed`, rule 4 is skipped
    entirely — `FrontmatterResult::Unclosed` carries no `body_start_line`,
    so there's no well-defined body to scan; the rest of the file is
    treated as part of the broken frontmatter block. Confirmed with the
    plan owner during section-03's code review.
- Both rules are independent and can both fire on the same file (e.g. a
  non-root `index.md` with bad frontmatter and a stray body paragraph).
