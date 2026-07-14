# okf-lint

A CLI linter for [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
(OKF) documentation bundles. Walks a directory of markdown files, checks
frontmatter, `index.md`/`log.md` structure, and general markdown hygiene
(line length, trailing whitespace, hard tabs, etc.), and emits
compiler-style diagnostics with a nonzero exit code on violations — built
for CI.

## Install

```bash
cargo install okf-lint
```

## Usage

Lint a bundle:

```bash
okf-lint path/to/bundle
```

```
concept-a.md:7: line has trailing whitespace
concept-b.md:1: missing or invalid YAML frontmatter (spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#41-frontmatter)
log.md:7: log.md heading is not a valid YYYY-MM-DD date (spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#7-log-files-optional)
```

Exit codes: `0` clean, `1` violations found, `2` usage/IO error.

Auto-fix the mechanical style issues (trailing whitespace, hard tabs,
consecutive blank lines, line wrapping) in place, then report whatever's
left:

```bash
okf-lint fmt path/to/bundle
```

Check what `fmt` would change without writing anything (CI-friendly):

```bash
okf-lint fmt path/to/bundle --check
```

Other flags (available on both the bare/`lint` and `fmt` forms):

- `--max-line-length <N>` (default `100`)
- `--include-hidden` — also walk dot-prefixed files/directories (e.g.
  `.git`), which are skipped by default

```bash
okf-lint path/to/bundle --max-line-length 120 --include-hidden
```

## Docker

Images are published to [Docker Hub](https://hub.docker.com/r/rpmoore/okf-lint) as
multi-platform (`linux/amd64` and `linux/arm64`) builds, tagged `latest` and per-commit
SHA — `docker pull` resolves the right architecture automatically:

```bash
docker pull rpmoore/okf-lint
```

The container's entrypoint *is* the `okf-lint` binary, so it takes the same arguments
and subcommands as the CLI (see [Usage](#usage) above). Mount the directory you want to
scan into the container as a volume, then pass that mount path as the argument. To lint
this project's own OKF docs, which live under `docs/knowledge`, for example:

```bash
docker run --rm -v "$PWD/docs/knowledge":/data rpmoore/okf-lint /data
```

`fmt --check` works the same way, since like `lint` it only reads:

```bash
docker run --rm -v "$PWD/docs/knowledge":/data rpmoore/okf-lint fmt /data --check
```

Writing fixes back with plain `fmt` is different: the image runs as a non-root user
(uid `65532`, inherited from the `cgr.dev/chainguard/static` base), so it typically
can't write to a bind-mounted host directory owned by your own user — add
`--user "$(id -u):$(id -g)"` so the container writes as you instead:

```bash
docker run --rm -v "$PWD/docs/knowledge":/data --user "$(id -u):$(id -g)" rpmoore/okf-lint fmt /data
```

If you're only linting (not writing fixes back with `fmt`), mount read-only with
`-v "$PWD/docs/knowledge":/data:ro`.

## License

Apache-2.0, see [LICENSE](LICENSE.md).
