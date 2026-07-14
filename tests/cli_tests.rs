use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn nonexistent_path_exits_2_with_stderr_and_empty_stdout() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/does_not_exist_okf_lint")
        .assert()
        .code(2)
        .stderr(predicate::str::is_empty().not())
        .stdout(predicate::str::is_empty());
}

#[test]
fn clean_bundle_exits_0_with_empty_stdout() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/okf/missing_frontmatter/pass")
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn bundle_with_violation_exits_1_with_diagnostic() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/okf/missing_frontmatter/fail")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "missing or invalid YAML frontmatter",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn okf_violation_stdout_contains_spec_link() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/okf/missing_frontmatter/fail")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "(spec: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#41-frontmatter)",
        ));
}

#[test]
fn style_violation_stdout_has_no_spec_link() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/cli/max_line_length_override")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("(spec:").not());
}

#[test]
fn max_line_length_override_suppresses_violation() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .args([
            "tests/fixtures/cli/max_line_length_override",
            "--max-line-length",
            "150",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn default_max_line_length_matches_explicit_100() {
    let default_output = Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/cli/max_line_length_override")
        .output()
        .unwrap();

    let explicit_output = Command::cargo_bin("okf-lint")
        .unwrap()
        .args([
            "tests/fixtures/cli/max_line_length_override",
            "--max-line-length",
            "100",
        ])
        .output()
        .unwrap();

    assert_eq!(default_output.status.code(), Some(1));
    assert_eq!(default_output.stdout, explicit_output.stdout);
    assert_eq!(default_output.status.code(), explicit_output.status.code());
    assert!(default_output.stderr.is_empty());
    assert!(explicit_output.stderr.is_empty());
    assert!(
        String::from_utf8_lossy(&default_output.stdout)
            .contains("line exceeds maximum length of 100 characters (120 found)")
    );
}

#[test]
fn integration_bundle_whole_output_matches_snapshot() {
    let output = Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/integration_bundle")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(stdout);
}

#[test]
fn integration_bundle_exits_1() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/integration_bundle")
        .assert()
        .code(1)
        .stderr(predicate::str::is_empty());
}

#[test]
fn bare_path_still_works_without_subcommand() {
    // Regression check for the Cli restructure: `okf-lint <path>` (no `lint`/`fmt`
    // subcommand) must behave exactly like it did before subcommands existed.
    Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/okf/missing_frontmatter/fail")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "missing or invalid YAML frontmatter",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn explicit_lint_subcommand_matches_bare_path() {
    let bare = Command::cargo_bin("okf-lint")
        .unwrap()
        .arg("tests/fixtures/okf/missing_frontmatter/fail")
        .output()
        .unwrap();
    let explicit = Command::cargo_bin("okf-lint")
        .unwrap()
        .args(["lint", "tests/fixtures/okf/missing_frontmatter/fail"])
        .output()
        .unwrap();

    assert_eq!(bare.stdout, explicit.stdout);
    assert_eq!(bare.status.code(), explicit.status.code());
}

#[test]
fn fmt_fixes_style_violations_in_place() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("a.md"),
        "---\ntype: concept\n---\nline with trailing space \n\n\nsecond\n",
    )
    .unwrap();

    Command::cargo_bin("okf-lint")
        .unwrap()
        .args(["fmt", dir.path().to_str().unwrap()])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let fixed = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert_eq!(
        fixed,
        "---\ntype: concept\n---\nline with trailing space\n\nsecond\n"
    );
}

#[test]
fn fmt_reports_remaining_diagnostics_it_could_not_fix() {
    let dir = TempDir::new().unwrap();
    // Trailing whitespace (fixable) plus a missing 'type' field (structural, not
    // a style rule, so fmt cannot and should not fix it).
    std::fs::write(
        dir.path().join("a.md"),
        "---\ntitle: no type here\n---\nline with trailing space \n",
    )
    .unwrap();

    Command::cargo_bin("okf-lint")
        .unwrap()
        .args(["fmt", dir.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "frontmatter missing required non-empty 'type' field",
        ))
        .stderr(predicate::str::is_empty());

    let fixed = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert_eq!(
        fixed,
        "---\ntitle: no type here\n---\nline with trailing space\n"
    );
}

#[test]
fn fmt_check_mode_reports_without_writing() {
    let dir = TempDir::new().unwrap();
    let original = "---\ntype: concept\n---\nline with trailing space \n";
    std::fs::write(dir.path().join("a.md"), original).unwrap();

    Command::cargo_bin("okf-lint")
        .unwrap()
        .args(["fmt", dir.path().to_str().unwrap(), "--check"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("would reformat: a.md"))
        .stderr(predicate::str::is_empty());

    let untouched = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert_eq!(untouched, original);
}

#[test]
fn fmt_check_mode_clean_bundle_exits_0() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .args([
            "fmt",
            "tests/fixtures/okf/missing_frontmatter/pass",
            "--check",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn fmt_clean_bundle_exits_0_with_empty_stdout() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .args(["fmt", "tests/fixtures/okf/missing_frontmatter/pass"])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn fmt_nonexistent_path_exits_2_with_stderr_and_empty_stdout() {
    Command::cargo_bin("okf-lint")
        .unwrap()
        .args(["fmt", "tests/fixtures/does_not_exist_okf_lint"])
        .assert()
        .code(2)
        .stderr(predicate::str::is_empty().not())
        .stdout(predicate::str::is_empty());
}
