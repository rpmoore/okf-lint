use assert_cmd::Command;
use predicates::prelude::*;

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
        String::from_utf8_lossy(&default_output.stdout).contains(
            "line exceeds maximum length of 100 characters (120 found)"
        )
    );
}
