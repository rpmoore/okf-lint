# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project state

This is a fresh Rust binary crate (`cargo new` skeleton) named `okf-lint`, edition 2024. No dependencies, no README, no source beyond `src/main.rs` printing "Hello, world!". No architecture exists yet — the name suggests a linter for OKF (Open Knowledge Foundation?) data, but this has not been implemented.

## Commands

- Build: `cargo build`
- Run: `cargo run`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Knowledge docs (OKF)

After any code change, update the OKF knowledge docs in `docs/knowledge/` for the section of code touched, following the OKF spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md. If no doc exists yet for that section, create one (YAML frontmatter with required `type` field, per spec).
