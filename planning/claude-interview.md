# Interview: okf-lint

## Q1: Research flagged two YAML crate options. Which for frontmatter parsing?

**A:** `serde_yaml_ng` (Recommended) — stable, API-identical to old
`serde_yaml`, used by Nushell. Stalled (no commits since May 2024) but
complete for basic map/string parsing.

## Q2: Directory traversal: how to handle hidden dirs (.git, .github) and symlinks under `<path>`?

**A:** Skip hidden dirs, don't follow symlinks (Recommended) — skip any
dir/file starting with `.`, do not follow symlinks. Avoids linting
`.git` internals or infinite loops.

## Q3: Empty .md file (0 bytes): does it trigger 'file must end with exactly one trailing newline'?

**A:** Yes, treat as violation (Recommended) — 0 bytes has 0 trailing
newlines, so it fails the "exactly one newline" rule like any other
file missing a trailing newline.

## Q4: `<path>` argument errors: what if it doesn't exist, isn't a directory, or a file inside it isn't valid UTF-8?

**A:** Print error to stderr, exit 2 (Recommended) — reserve exit 1 for
"lint violations found", use a distinct exit code (2) for usage/IO
errors so CI can tell scope-error apart from lint-failure.

## Q5: index.md rule 4: what counts as a valid 'continuation' line under a list item (e.g. wrapped text)?

**A:** Indented by >=2 spaces under a list item (Recommended) — any line
indented at least 2 spaces immediately following a list item is treated
as part of that item, not a violation.

## Q6: Multiple diagnostics on the same line (e.g. line is both too long AND has trailing whitespace): report separately?

**A:** Yes, one diagnostic line per violation (Recommended) — same
`file:line` can appear multiple times, once per distinct rule violated
— matches compiler-diagnostic conventions.

## Q7: Diagnostic output ordering across the whole run?

**A:** Sorted by file path, then by line number (Recommended) —
deterministic, matches walkdir+sort traversal order already planned;
needed for stable snapshot tests.

## Q8: Line length count: by Unicode scalar value (chars) or by byte length?

**A:** Unicode scalar count / chars (Recommended) — matches editor
"column count" intuition, e.g. 'é' counts as 1 char even though it's 2
bytes in UTF-8.

## Q9: CRLF line endings (\r\n): treat the \r as trailing whitespace, or normalize/ignore it?

**A:** Treat `\r` as trailing whitespace (Recommended) — simplest: split
on `\n`, then any line ending in `\r` (or space/tab) trips rule 3.2.
Effectively enforces LF-only files.
