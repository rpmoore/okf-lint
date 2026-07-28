---
type: module
---

# Log checks

OKF conformance rule for `log.md` files.

## `src/checks/log_md.rs`

- `check_log(content: &str) -> Vec<Diagnostic>` — implements one rule (two checks, both under
  `Rule::OkfLogDateHeading`):
  - **Date validity**: every level-2 (`##`) heading must be a real calendar date in
    `YYYY-MM-DD` format. Headings at other levels (`#`, `###`, ...) are not inspected at all.
  - **Newest-first ordering** (spec §9: "a flat list of date-grouped entries, newest first"):
    a running `last_valid_date: Option<NaiveDate>` is updated after every calendar-valid
    heading; if the next valid date is *strictly greater* (i.e. more recent) than the last one
    seen, that's an order violation — "log.md date headings must be in newest-first
    (descending) order". Equal consecutive dates are allowed (multiple entries same day).
    Shape/calendar-invalid headings are skipped for ordering purposes — they're already
    flagged on their own line — and don't reset `last_valid_date`, so the valid dates on
    either side of a bad one are still compared to each other.
- Scans the *whole* content, unlike `index.md`'s rule 4 — there's no frontmatter/body offset
  here, since `log.md` has no frontmatter handling in this linter.
- No `regex` crate dependency in this project, so the heading match is hand-rolled: the
  `^## (.*)$` pattern is `line.strip_prefix("## ")`. This correctly excludes `#
  text` and `### text`/deeper — in both cases the third byte isn't a space, so the prefix
  strip fails and the line is skipped entirely (never falls through to date-validating a
  level-1/3+ heading).
- The strict `YYYY-MM-DD` shape-and-calendar check itself — including the byte-indexed shape
  scan and the `chrono::NaiveDate::parse_from_str` call that catches calendar-invalid dates
  like `2026-02-30` — no longer lives in this module. `check_log` calls
  `crate::date::parse_ymd(text)` (see `docs/knowledge/foundation.md`), shared with the OKF
  v0.2 `stale_after` field check (`checks/okf.rs`) since both need the identical notion of a
  valid `YYYY-MM-DD` date. Both failure modes (bad shape, or right shape but chrono rejects it)
  collapse to `parse_ymd` returning `None`, which produces the identical diagnostic here.
- Known limitation (shared with `index_md.rs`/`frontmatter.rs`): content is split purely on
  `\n`, so a CRLF-terminated heading line carries a trailing `\r` into the captured text and
  will be flagged as an invalid date. Not handled specially here, consistent with the rest of
  the codebase.
