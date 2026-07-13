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
