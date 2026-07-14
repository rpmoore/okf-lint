---
type: module
---

# Deployment

Container packaging for the `okf-lint` binary. No runtime logic lives here — this is
build/ship plumbing, kept separate from the check modules and CLI layer.

## `Dockerfile`

Two-stage build, both stages on Chainguard (Wolfi-based, minimal-CVE) images:

1. **`builder`** (`cgr.dev/chainguard/rust:latest-dev`) — has `cargo`/`rustc`. Copies
   `Cargo.toml`, `Cargo.lock`, and `src/` (nothing else — no `tests/`, no `docs/`, so
   changes to those don't bust the build cache), then runs
   `cargo build --release --locked --target "$TARGET"` (`$TARGET` = the host triple from
   `rustc -vV`) with `RUSTFLAGS="-C target-feature=+crt-static"`. `+crt-static`
   statically links glibc into the binary; every dependency in `Cargo.toml` (`clap`,
   `walkdir`, `serde_yaml_ng`, `chrono`) is pure Rust with no C FFI, so this works
   without a musl target or cross toolchain. The explicit `--target` is required, not
   cosmetic: without it cargo can't distinguish "host" from "target" artifacts, so
   `RUSTFLAGS` also gets applied to proc-macro crates (`clap_derive`) built for the
   host, and `+crt-static` breaks the `proc-macro` crate type — the build fails with
   `cannot produce proc-macro for clap_derive ... does not support these crate types`.
   `--locked` fails the build instead of silently drifting from `Cargo.lock`. The binary
   is copied to `/work/okf-lint` (out of the per-target path) so the final stage doesn't
   need to know `$TARGET`.
2. **final stage** (`cgr.dev/chainguard/static:latest`) — distroless: no shell, no
   package manager, no libc (comparable to `gcr.io/distroless/static`). Only the
   statically-linked binary is copied in, to `/usr/local/bin/okf-lint`. This is why the
   builder stage must produce a static binary — the final stage has nothing to
   dynamically link against. Runs as the image's non-root default user.

`ENTRYPOINT ["/usr/local/bin/okf-lint"]` — the container behaves like the CLI binary
itself; pass the same arguments documented in `docs/knowledge/cli.md`
(e.g. `docker run --rm -v "$PWD":/data okf-lint /data`).

`.dockerignore` excludes `target/`, `.git/`, `docs/`, `tests/`, `planning/` from the
build context.

## `.github/workflows/release.yml`

Runs on `release: published` (i.e. when a GitHub Release is created/published, not on
every push). Single job, steps run sequentially so a failure at any step stops the
pipeline before anything is pushed. Every validation step runs before either of the two
push/publish steps, since neither Docker Hub nor crates.io supports a rollback (crates.io
publishes are permanent; a bad Docker Hub push is merely undesirable, not atomic with the
crate publish) — see the note below on ordering:

1. **Version gate** — compares `github.event.release.tag_name` against the version in
   `Cargo.toml` (via `cargo metadata`), requiring `tag == "v$cargo_version"`. Fails the
   job with `::error::` before any build/publish work happens if they don't match —
   this is what stops a release being tagged without first bumping `Cargo.toml`. Pattern
   taken from [rpmoore/rdns's release workflow](https://github.com/rpmoore/rdns/blob/main/.github/workflows/release.yml).
2. Build (`cargo build --release --locked`) and test (`cargo test --release --locked`).
3. **`cargo publish --dry-run --locked`** — validates crate packaging (manifest,
   `license-file`, `readme`, included files) against crates.io's rules without
   publishing anything.
4. **Docker build validation** — `docker/build-push-action` with `push: false, load:
   true` builds the image and loads it into the runner's local Docker daemon (not
   Docker Hub) under both target tags, then a smoke-test step runs
   `docker run --rm rpmoore/okf-lint:$SHA --help` against it to confirm the binary
   actually executes in the distroless image, not just that the build succeeded.
5. Only once all of the above passes: `docker/login-action` (auth against
   `DOCKERHUB_PUSH`, a Docker Hub personal access token for the `rpmoore` account), then
   `docker push` for both the `${{ github.sha }}` and `latest` tags — pushing the exact
   image already validated in step 4, not a rebuild.
6. `cargo publish --locked` to crates.io, authenticated via `CARGO_REGISTRY_TOKEN` set
   from the `CRATES_API_KEY` repo secret. Deliberately last: crates.io publishes can
   never be undone or reused (only yanked), while a bad Docker Hub push can simply be
   overwritten by re-running the job. Keeping it last means a failure anywhere earlier
   never leaves an unpublishable crate version behind.

This ordering doesn't make the two pushes atomic — no distributed transaction spans
Docker Hub and crates.io — but it minimizes the chance of a split state and makes the
job safely re-runnable: the Docker push step is idempotent (same content, same tags),
so if the job fails after that step but before `cargo publish`, re-running the whole job
just repushes the image and then completes the crate publish.

Note this workflow does not reuse the `Dockerfile`'s musl-free static-link trick for
anything beyond the container image — `cargo publish` and the plain `cargo
build`/`cargo test` steps run on the runner's default toolchain, unrelated to how the
container binary gets linked.
