---
type: module
---

# Dev tooling

Local developer-workflow plumbing, kept separate from the check modules, CLI layer, and
[deployment](deployment.md) pipeline.

## `justfile`

- `just fmt` — the local pre-commit sanity pass, run manually before pushing:
  1. `cargo clippy` — lint the Rust source.
  2. `cargo fmt` — reformat the Rust source.
  3. `cargo run -- fmt docs/knowledge` — dogfoods the project's own `fmt` subcommand
     (`docs/knowledge/fmt.md`) against this repository's own knowledge base, so the
     OKF docs stay self-consistent with the tool that validates them. Uses `cargo run`
     rather than an installed `okf-lint` binary so the recipe always exercises the
     current working tree, not whatever version happens to be on `PATH`.
