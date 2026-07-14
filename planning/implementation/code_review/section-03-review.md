# Code review: section-03-index-checks

Overall the implementation is faithful to the spec and free of crashes/panics (no unwraps on
untrusted input, CRLF-safe via strip_cr, correct 1-based line numbering, correct Rule
variants/messages verbatim). The core logic in src/checks/index_md.rs (check_index,
root_frontmatter_ok, scan_body, is_heading, is_list_item, leading_space_count) matches the spec's
rule-3/rule-4 semantics, including the Found/Unclosed/None dispatch, the is_root branching, and
the first-match-wins heading/list-item/continuation/violation ordering in scan_body. No dead-code
or wiring issues beyond what's expected for this stage.

## Findings

1. **(Medium)** Missing test coverage for the plan's explicit "one diagnostic per line, not per
   paragraph" claim. Neither `fail.md` nor the inline
   `nonroot_with_bad_frontmatter_and_stray_paragraph_emits_both` test exercises two or more
   consecutive violating lines. A regression that collapsed a multi-line stray paragraph into a
   single diagnostic would pass every existing test.

2. **(Low)** Root "no frontmatter" and "empty frontmatter mapping" paths are untested.
   `pass_root/index.md` only covers the okf_version-only case; `FrontmatterResult::None` on a root
   file, and `root_frontmatter_ok`'s `Ok(Value::Null) => true` branch, are never exercised.

3. **(Informational)** The judgment call to skip rule 4 entirely when frontmatter is `Unclosed` is
   reasonable and well-pinned-down by tests, but is a real (if small) coverage reduction: a
   non-root file with unclosed frontmatter and a garbage body now reports only one diagnostic
   total. Spec text only defines `body_start_line` for the `Found` variant, so this interpretation
   is defensible, but should be confirmed rather than left as an implementer-only inference.

4. **(Nit)** `scan_body` walks every line of `content` and discards pre-body lines with `continue`
   rather than `.skip(start_line - 1)`. Style only, not a correctness issue.

No issues found with: YAML mapping-key-check logic, heading/list-item regex translations,
continuation-line leading-space counting, blank-line reset semantics, or diagnostic push ordering.
