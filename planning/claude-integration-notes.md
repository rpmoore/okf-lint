# Integration Notes — Codex Review (iteration 1)

Source: `reviews/iteration-1-codex.md`

## Integrated

- **Fixture pass/fail isolation bug (claude-plan.md §10).** Codex correctly
  caught an internal inconsistency: the tree overview (§2, line 52) says
  "one pass/ and fail/ mini-bundle per rule," but the detailed §10 layout
  put `pass.md` and `fail.md` as sibling files directly inside each rule's
  fixture directory. Since the plan's own test-isolation claim depends on
  running the CLI against a fixture's *containing directory*, siblings
  would mean a directory-rooted CLI run picks up both files at once —
  exactly the failure mode the plan claims to avoid ("exactly one file
  checked per test"). Fixed by nesting each fixture under `pass/` and
  `fail/` subdirectories so each is genuinely a single-file mini-bundle,
  and updated the accompanying prose to explain why the split is
  necessary. This matches spec §5's requirement that each failing fixture
  "triggers exactly that check and no others."

## Not integrated

- **`deep_plan_config.json` hardcoded local paths.** Codex flagged the
  `/home/rpmoore/...` absolute paths in `planning/deep_plan_config.json`
  as non-portable / machine-identifying. This file is deep-plan tooling
  bookkeeping (written by `setup-planning-session.py` to track the
  planning session), not part of the okf-lint implementation plan or
  codebase the plan describes — it has no bearing on what gets built or
  how the fixtures/tests work. Out of scope for this plan; not something
  the implementation plan should "fix." (If it matters, it's a
  deep-plan-tooling concern, not an okf-lint one — flagging for the user
  rather than editing plan content over it.)
