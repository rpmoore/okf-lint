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

## License

Apache-2.0, see [LICENSE](LICENSE.md).
