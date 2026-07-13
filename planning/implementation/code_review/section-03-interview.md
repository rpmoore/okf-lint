# Code review interview: section-03-index-checks

## Asked

**Q: Unclosed frontmatter on index.md — skip rule-4 body scan entirely, or still scan the rest of
the file as body?**

Decision: **Keep skip.** No closing `---` means no well-defined body start; the rest of the file
is treated as part of the broken frontmatter block, not a scannable body. Already covered by
`root_unclosed_frontmatter_emits_root_diagnostic` and
`nonroot_unclosed_frontmatter_emits_nonroot_diagnostic`. No code change needed.

## Auto-fixed (low-risk, applied without asking)

1. Added a test with a multi-line stray paragraph (2+ consecutive violating lines) to lock in the
   spec's "one diagnostic per line, not one per paragraph" behavior.
2. Added tests for root index.md with no frontmatter (`FrontmatterResult::None`) and root index.md
   with an empty frontmatter block (`Value::Null` path in `root_frontmatter_ok`).

## Let go

- Nit: `scan_body`'s `continue`-based line skipping vs `.skip(start_line - 1)`. Style only, no
  behavior difference, not worth churn.
