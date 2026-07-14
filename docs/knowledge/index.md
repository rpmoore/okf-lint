# okf-lint Knowledge Base

* [Foundation](foundation.md) - shared data types, frontmatter parsing, and
  directory walking used by every check module
* [Concept checks](concept-checks.md) - OKF frontmatter/type rules for
  ordinary concept documents
* [Index checks](index-checks.md) - OKF frontmatter-placement and
  body-structure rules for `index.md` files
* [Log checks](log-checks.md) - OKF date-heading rule for `log.md` files
* [Style checks](style-checks.md) - generic markdown hygiene rules applied to
  every `.md` file, independent of OKF structure
* [Orchestration](orchestration.md) - file classification and `lint_bundle`,
  which dispatches to and merges output from all four check modules
* [CLI](cli.md) - argument parsing, stdout/stderr formatting, and exit-code mapping for
  the `okf-lint` binary
* [fmt](fmt.md) - the `fmt` subcommand: auto-corrects mechanical style violations in
  place, with a `--check` report-only mode
* [Integration tests](integration-tests.md) - whole-pipeline regression guard exercising
  classification, all check modules, and cross-file sort order together
* [Deployment](deployment.md) - the multistage `Dockerfile` that packages `okf-lint` as
  a static binary in a Chainguard distroless image
