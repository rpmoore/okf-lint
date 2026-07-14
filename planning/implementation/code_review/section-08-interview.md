# Code Review Interview: section-08-integration-tests

No user interview needed — all real findings were auto-fixable with no tradeoffs; the rest were scope-creep nitpicks explicitly out of scope per the plan.

## Auto-fix (no tradeoffs, applied without asking)

1. **Missing `docs/knowledge/` doc.** CLAUDE.md requires a doc per touched section; section-08 didn't get one. Create `docs/knowledge/integration-tests.md` and link it from `docs/knowledge/index.md`.
2. **Confusing fixture naming.** `concept-b.md` (root, fails rule 1) and `sub/concept-b.md` (fully clean) share a name despite opposite behavior. Rename `sub/concept-b.md` to `sub/concept-c.md` and update `sub/index.md`'s link text accordingly.

## Let go (explicitly out of scope per the plan, not worth interviewing)

- Thin rule coverage (4/10 `Rule` variants in the snapshot) — plan's fixture design is explicitly illustrative, and every rule already has dedicated unit-level coverage from earlier sections.
- No same-line multi-rule tie-break exercised at the integration layer — already unit-tested in `lint.rs`/`diagnostic.rs`.
- Root `index.md`'s "okf_version-only frontmatter" pass path not exercised here — covered by section-03's dedicated fixtures.
- `deep_implement_config.json`'s stale `test_command: "uv run pytest"` — deep-implement session state, not a project deliverable; setup script's own preflight already correctly detected `cargo test`.

## Verified, not a real finding

- Reviewer initially flagged the manually-promoted `.snap` file as unverified. Confirmed this is not an issue: `cargo test --test cli_tests` was run after promotion and all 7 tests passed (including the snapshot test), and the full 84-test workspace suite plus `cargo clippy --all-targets` were also run clean.
