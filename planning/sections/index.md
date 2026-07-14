<!-- PROJECT_CONFIG
runtime: rust-cargo
test_command: cargo test
END_PROJECT_CONFIG -->

<!-- SECTION_MANIFEST
section-01-foundation
section-02-concept-checks
section-03-index-checks
section-04-log-checks
section-05-style-checks
section-06-orchestration
section-07-cli
section-08-integration-tests
END_MANIFEST -->

# Implementation Sections Index

Splits `claude-plan.md` / `claude-plan-tdd.md` into implementation
units. Section numbers/names below match `claude-plan.md`'s own
section numbering where noted.

## Dependency Graph

| Section | Depends On | Blocks | Parallelizable |
|---------|------------|--------|-----------------|
| section-01-foundation | - | 02, 03, 04, 05 | Yes (first) |
| section-02-concept-checks | 01 | 06 | Yes (with 03, 04, 05) |
| section-03-index-checks | 01 | 06 | Yes (with 02, 04, 05) |
| section-04-log-checks | 01 | 06 | Yes (with 02, 03, 05) |
| section-05-style-checks | 01 | 06 | Yes (with 02, 03, 04) |
| section-06-orchestration | 02, 03, 04, 05 | 07 | No |
| section-07-cli | 06 | 08 | No |
| section-08-integration-tests | 07 | - | No |

## Execution Order

1. section-01-foundation (no dependencies)
2. section-02-concept-checks, section-03-index-checks,
   section-04-log-checks, section-05-style-checks (parallel after 01 —
   each is an independent `checks/*.rs` module touching disjoint files)
3. section-06-orchestration (requires all four check modules)
4. section-07-cli (requires orchestration's `lint_bundle`)
5. section-08-integration-tests (requires the full CLI binary)

## Section Summaries

### section-01-foundation
Add all `Cargo.toml` dependencies (plan §9: `clap`, `walkdir`,
`serde_yaml_ng`, `chrono`, plus dev-deps `assert_cmd`, `predicates`,
`insta`). Create `diagnostic.rs` (`Diagnostic`, `Rule` — plan §3),
`frontmatter.rs` (`split_frontmatter`, `FrontmatterResult` — plan §3),
and `walk.rs` (`collect_md_files` — plan §3). These are the shared,
dependency-free building blocks every other module and check imports.
Confirms `cargo build` / `cargo test` succeed on the new dependency set
before any check logic is written (TDD plan's "Dependency/setup
verification" gate).

### section-02-concept-checks
`checks/okf.rs`: `check_concept`, implementing rules 1
(`OkfMissingFrontmatter`) and 2 (`OkfMissingType`) — plan §5.1. Uses
`frontmatter.rs` and `diagnostic.rs` from section-01. Includes the
`missing_frontmatter` and `missing_type` fixture pairs (plan §10) and
associated unit tests (TDD plan §5.1).

### section-03-index-checks
`checks/index_md.rs`: `check_index`, implementing rules 3
(`OkfIndexFrontmatterPlacement`) and 4 (`OkfIndexBodyStructure`) — plan
§5.2. Uses `frontmatter.rs` and `diagnostic.rs` from section-01.
Includes the `index_frontmatter_placement` and `index_body_structure`
fixtures and associated unit tests (TDD plan §5.2).

### section-04-log-checks
`checks/log_md.rs`: `check_log`, implementing rule 5
(`OkfLogDateHeading`) — plan §5.3, using `chrono` for calendar-date
validation. Uses `diagnostic.rs` from section-01. Includes the
`log_date_heading` fixture pair and associated unit tests (TDD plan
§5.3).

### section-05-style-checks
`checks/style.rs`: `check_style`, implementing all 5 markdown hygiene
rules (line length, trailing whitespace, trailing newline, consecutive
blank lines, hard tabs) — plan §6. Uses `diagnostic.rs` from
section-01. Includes all five `style/*` fixture pairs and associated
unit tests (TDD plan §6).

### section-06-orchestration
File classification (plan §4) and `lint.rs`: `LintError`,
`lint_bundle` — plan §3/§7, dispatching to all four check modules from
sections 02-05, applying diagnostic sort ordering (plan §7). Requires
every check module to exist first. Includes classification tests and
`lint_bundle`-level tests (TDD plan §4, §7, and the `lint.rs` block).

### section-07-cli
`cli.rs` (`Cli` struct) and `main.rs` (entry point, exit-code mapping)
— plan §8. Requires `lint::lint_bundle` from section-06. Includes the
CLI-level tests in `tests/cli_tests.rs` for exit codes 0/1/2 and the
`--max-line-length` override (TDD plan §8).

### section-08-integration-tests
Builds `tests/fixtures/integration_bundle/` (plan §10) and the
whole-bundle `insta`-snapshot integration test plus its exit-code
assertion (TDD plan §10). Requires the fully working CLI binary from
section-07, since it exercises the tool end-to-end rather than any
single module.
