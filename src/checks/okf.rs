use crate::date::parse_ymd;
use crate::diagnostic::{Diagnostic, Rule};
use crate::frontmatter::{FrontmatterResult, split_frontmatter};
use serde_yaml_ng::{Mapping, Value};

const ATTESTED_COMPUTATION_TYPE: &str = "Attested Computation";
const VALID_STATUSES: [&str; 3] = ["draft", "stable", "deprecated"];

pub fn check_concept(content: &str) -> Vec<Diagnostic> {
    let yaml_text = match split_frontmatter(content) {
        FrontmatterResult::None | FrontmatterResult::Unclosed => {
            return vec![missing_frontmatter_diagnostic()];
        }
        FrontmatterResult::Found { yaml_text, .. } => yaml_text,
    };

    let parsed: Value = match serde_yaml_ng::from_str(&yaml_text) {
        Ok(value) => value,
        Err(_) => return vec![missing_frontmatter_diagnostic()],
    };

    let Value::Mapping(mapping) = parsed else {
        return vec![missing_frontmatter_diagnostic()];
    };

    let type_value = mapping.get("type").and_then(Value::as_str);
    let mut diagnostics = Vec::new();

    if !type_value.is_some_and(|s| !s.is_empty()) {
        diagnostics.push(Diagnostic {
            line: 1,
            rule: Rule::OkfMissingType,
            message: "frontmatter missing required non-empty 'type' field".to_string(),
        });
    }

    // OKF v0.2 (SPEC.md §5, §10) added optional provenance/trust/lifecycle
    // frontmatter families and the "Attested Computation" concept type. None
    // of these are required (§11 conformance is unchanged), but a present
    // field with the wrong shape is still a formatting error.
    check_sources(&mapping, &mut diagnostics);
    check_generated(&mapping, &mut diagnostics);
    check_verified(&mapping, &mut diagnostics);
    check_status(&mapping, &mut diagnostics);
    check_stale_after(&mapping, &mut diagnostics);
    check_attested_computation_runtime(type_value, &mapping, &mut diagnostics);

    diagnostics
}

fn missing_frontmatter_diagnostic() -> Diagnostic {
    Diagnostic {
        line: 1,
        rule: Rule::OkfMissingFrontmatter,
        message: "missing or invalid YAML frontmatter".to_string(),
    }
}

// §5.1: each `sources` entry must be a mapping with a non-empty `resource`.
fn check_sources(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    let Some(sources) = mapping.get("sources") else {
        return;
    };
    let Value::Sequence(entries) = sources else {
        diagnostics.push(sources_diagnostic(
            "'sources' must be a list of mappings".to_string(),
        ));
        return;
    };
    for (idx, entry) in entries.iter().enumerate() {
        let message = match entry {
            Value::Mapping(m) if has_non_empty_str(m, "resource") => continue,
            Value::Mapping(_) => {
                format!("'sources' entry {idx} is missing required non-empty 'resource' field")
            }
            _ => format!("'sources' entry {idx} must be a mapping"),
        };
        diagnostics.push(sources_diagnostic(message));
    }
}

fn sources_diagnostic(message: String) -> Diagnostic {
    Diagnostic {
        line: 1,
        rule: Rule::OkfInvalidSources,
        message,
    }
}

// §5.2: `generated.by` is required within `generated` when present.
fn check_generated(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    let Some(generated) = mapping.get("generated") else {
        return;
    };
    let by_ok = matches!(generated, Value::Mapping(m) if has_non_empty_str(m, "by"));
    if !by_ok {
        diagnostics.push(Diagnostic {
            line: 1,
            rule: Rule::OkfInvalidGenerated,
            message: "'generated' must be a mapping with a non-empty 'by' field".to_string(),
        });
    }
}

// §5.2: `verified` is a mapping, or a list of mappings (the "bare mapping"
// form consumers MUST treat as a one-element list), each with `by` and `at`.
fn check_verified(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    let Some(verified) = mapping.get("verified") else {
        return;
    };
    match verified {
        Value::Mapping(m) => {
            if !verified_entry_ok(m) {
                diagnostics.push(invalid_verified_diagnostic(None));
            }
        }
        Value::Sequence(entries) => {
            for (idx, entry) in entries.iter().enumerate() {
                let ok = matches!(entry, Value::Mapping(m) if verified_entry_ok(m));
                if !ok {
                    diagnostics.push(invalid_verified_diagnostic(Some(idx)));
                }
            }
        }
        _ => diagnostics.push(invalid_verified_diagnostic(None)),
    }
}

fn verified_entry_ok(m: &Mapping) -> bool {
    has_non_empty_str(m, "by") && m.get("at").is_some()
}

fn invalid_verified_diagnostic(idx: Option<usize>) -> Diagnostic {
    let message = match idx {
        Some(i) => format!("'verified' entry {i} must be a mapping with 'by' and 'at' fields"),
        None => {
            "'verified' must be a mapping with 'by' and 'at' fields, or a list of such mappings"
                .to_string()
        }
    };
    Diagnostic {
        line: 1,
        rule: Rule::OkfInvalidVerified,
        message,
    }
}

// §5.4: `status`, when present, must be one of the three defined values.
fn check_status(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    let Some(status) = mapping.get("status") else {
        return;
    };
    let ok = status.as_str().is_some_and(|s| VALID_STATUSES.contains(&s));
    if !ok {
        diagnostics.push(Diagnostic {
            line: 1,
            rule: Rule::OkfInvalidStatus,
            message: "'status' must be one of draft, stable, deprecated".to_string(),
        });
    }
}

// §5.5: `stale_after`, when present, must be a real YYYY-MM-DD calendar date.
fn check_stale_after(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    let Some(stale_after) = mapping.get("stale_after") else {
        return;
    };
    let ok = stale_after.as_str().is_some_and(|s| parse_ymd(s).is_some());
    if !ok {
        diagnostics.push(Diagnostic {
            line: 1,
            rule: Rule::OkfInvalidStaleAfter,
            message: "'stale_after' must be a valid YYYY-MM-DD date".to_string(),
        });
    }
}

// §10.2: `runtime` is required specifically for `type: Attested Computation`
// concepts; it's meaningless (and not checked) for any other type.
fn check_attested_computation_runtime(
    type_value: Option<&str>,
    mapping: &Mapping,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if type_value != Some(ATTESTED_COMPUTATION_TYPE) {
        return;
    }
    if !has_non_empty_str(mapping, "runtime") {
        diagnostics.push(Diagnostic {
            line: 1,
            rule: Rule::OkfAttestedComputationMissingRuntime,
            message: "'Attested Computation' concept is missing required non-empty 'runtime' field"
                .to_string(),
        });
    }
}

fn has_non_empty_str(mapping: &Mapping, key: &str) -> bool {
    mapping
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING_FRONTMATTER_PASS: &str =
        include_str!("../../tests/fixtures/okf/missing_frontmatter/pass/pass.md");
    const MISSING_FRONTMATTER_FAIL: &str =
        include_str!("../../tests/fixtures/okf/missing_frontmatter/fail/fail.md");
    const MISSING_TYPE_PASS: &str =
        include_str!("../../tests/fixtures/okf/missing_type/pass/pass.md");
    const MISSING_TYPE_FAIL: &str =
        include_str!("../../tests/fixtures/okf/missing_type/fail/fail.md");
    const V0_2_FULL_FRONTMATTER: &str =
        include_str!("../../tests/fixtures/okf/v0_2_compat/full_frontmatter.md");
    const V0_2_ATTESTED_COMPUTATION_BARE_VERIFIED: &str =
        include_str!("../../tests/fixtures/okf/v0_2_compat/attested_computation_bare_verified.md");
    const INVALID_SOURCES_PASS: &str =
        include_str!("../../tests/fixtures/okf/invalid_sources/pass/pass.md");
    const INVALID_SOURCES_FAIL: &str =
        include_str!("../../tests/fixtures/okf/invalid_sources/fail/fail.md");
    const INVALID_GENERATED_PASS: &str =
        include_str!("../../tests/fixtures/okf/invalid_generated/pass/pass.md");
    const INVALID_GENERATED_FAIL: &str =
        include_str!("../../tests/fixtures/okf/invalid_generated/fail/fail.md");
    const INVALID_VERIFIED_PASS: &str =
        include_str!("../../tests/fixtures/okf/invalid_verified/pass/pass.md");
    const INVALID_VERIFIED_FAIL: &str =
        include_str!("../../tests/fixtures/okf/invalid_verified/fail/fail.md");
    const INVALID_STATUS_PASS: &str =
        include_str!("../../tests/fixtures/okf/invalid_status/pass/pass.md");
    const INVALID_STATUS_FAIL: &str =
        include_str!("../../tests/fixtures/okf/invalid_status/fail/fail.md");
    const INVALID_STALE_AFTER_PASS: &str =
        include_str!("../../tests/fixtures/okf/invalid_stale_after/pass/pass.md");
    const INVALID_STALE_AFTER_FAIL: &str =
        include_str!("../../tests/fixtures/okf/invalid_stale_after/fail/fail.md");
    const ATTESTED_COMPUTATION_MISSING_RUNTIME_PASS: &str =
        include_str!("../../tests/fixtures/okf/attested_computation_missing_runtime/pass/pass.md");
    const ATTESTED_COMPUTATION_MISSING_RUNTIME_FAIL: &str =
        include_str!("../../tests/fixtures/okf/attested_computation_missing_runtime/fail/fail.md");

    #[test]
    fn missing_frontmatter_pass_fixture_has_no_diagnostics() {
        assert_eq!(check_concept(MISSING_FRONTMATTER_PASS), vec![]);
    }

    #[test]
    fn missing_frontmatter_fail_fixture_emits_rule_1() {
        assert_eq!(
            check_concept(MISSING_FRONTMATTER_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingFrontmatter,
                message: "missing or invalid YAML frontmatter".to_string(),
            }]
        );
    }

    #[test]
    fn missing_type_pass_fixture_has_no_diagnostics() {
        assert_eq!(check_concept(MISSING_TYPE_PASS), vec![]);
    }

    #[test]
    fn missing_type_fail_fixture_emits_rule_2() {
        assert_eq!(
            check_concept(MISSING_TYPE_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingType,
                message: "frontmatter missing required non-empty 'type' field".to_string(),
            }]
        );
    }

    #[test]
    fn invalid_sources_pass_fixture_has_no_diagnostics() {
        assert_eq!(check_concept(INVALID_SOURCES_PASS), vec![]);
    }

    #[test]
    fn invalid_sources_fail_fixture_emits_diagnostic() {
        assert_eq!(
            check_concept(INVALID_SOURCES_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidSources,
                message: "'sources' entry 0 is missing required non-empty 'resource' field"
                    .to_string(),
            }]
        );
    }

    #[test]
    fn sources_not_a_list_emits_diagnostic() {
        let content = "---\ntype: Metric\nsources: not-a-list\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidSources,
                message: "'sources' must be a list of mappings".to_string(),
            }]
        );
    }

    #[test]
    fn sources_entry_not_a_mapping_emits_distinct_diagnostic() {
        // A scalar list entry is a different problem than a mapping missing
        // 'resource' — the message must say so, not claim 'resource' is
        // missing from something that isn't a mapping at all.
        let content = "---\ntype: Metric\nsources:\n  - just-a-string\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidSources,
                message: "'sources' entry 0 must be a mapping".to_string(),
            }]
        );
    }

    #[test]
    fn invalid_generated_pass_fixture_has_no_diagnostics() {
        assert_eq!(check_concept(INVALID_GENERATED_PASS), vec![]);
    }

    #[test]
    fn invalid_generated_fail_fixture_emits_diagnostic() {
        assert_eq!(
            check_concept(INVALID_GENERATED_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidGenerated,
                message: "'generated' must be a mapping with a non-empty 'by' field".to_string(),
            }]
        );
    }

    #[test]
    fn invalid_verified_pass_fixture_has_no_diagnostics() {
        assert_eq!(check_concept(INVALID_VERIFIED_PASS), vec![]);
    }

    #[test]
    fn invalid_verified_fail_fixture_emits_diagnostic() {
        assert_eq!(
            check_concept(INVALID_VERIFIED_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidVerified,
                message: "'verified' must be a mapping with 'by' and 'at' fields, or a list of such mappings".to_string(),
            }]
        );
    }

    #[test]
    fn verified_list_entry_missing_at_emits_indexed_diagnostic() {
        let content = "---\ntype: Metric\nverified:\n  - { by: human:ahormati }\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidVerified,
                message: "'verified' entry 0 must be a mapping with 'by' and 'at' fields"
                    .to_string(),
            }]
        );
    }

    #[test]
    fn invalid_status_pass_fixture_has_no_diagnostics() {
        assert_eq!(check_concept(INVALID_STATUS_PASS), vec![]);
    }

    #[test]
    fn invalid_status_fail_fixture_emits_diagnostic() {
        assert_eq!(
            check_concept(INVALID_STATUS_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidStatus,
                message: "'status' must be one of draft, stable, deprecated".to_string(),
            }]
        );
    }

    #[test]
    fn invalid_stale_after_pass_fixture_has_no_diagnostics() {
        assert_eq!(check_concept(INVALID_STALE_AFTER_PASS), vec![]);
    }

    #[test]
    fn invalid_stale_after_fail_fixture_emits_diagnostic() {
        assert_eq!(
            check_concept(INVALID_STALE_AFTER_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfInvalidStaleAfter,
                message: "'stale_after' must be a valid YYYY-MM-DD date".to_string(),
            }]
        );
    }

    #[test]
    fn attested_computation_missing_runtime_pass_fixture_has_no_diagnostics() {
        assert_eq!(
            check_concept(ATTESTED_COMPUTATION_MISSING_RUNTIME_PASS),
            vec![]
        );
    }

    #[test]
    fn attested_computation_missing_runtime_fail_fixture_emits_diagnostic() {
        assert_eq!(
            check_concept(ATTESTED_COMPUTATION_MISSING_RUNTIME_FAIL),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfAttestedComputationMissingRuntime,
                message:
                    "'Attested Computation' concept is missing required non-empty 'runtime' field"
                        .to_string(),
            }]
        );
    }

    #[test]
    fn runtime_not_required_for_other_types() {
        let content = "---\ntype: Metric\n---\nbody";
        assert_eq!(check_concept(content), vec![]);
    }

    #[test]
    fn invalid_yaml_syntax_fires_rule_1() {
        // Unclosed flow mapping: a genuine YAML syntax error, distinct from
        // "well-formed but not a mapping" (covered separately below).
        let content = "---\ntype: {unclosed\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingFrontmatter,
                message: "missing or invalid YAML frontmatter".to_string(),
            }]
        );
    }

    #[test]
    fn unclosed_frontmatter_only_fires_rule_1() {
        let content = "---\ntype: concept\nno closing delimiter";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingFrontmatter,
                message: "missing or invalid YAML frontmatter".to_string(),
            }]
        );
    }

    #[test]
    fn non_mapping_frontmatter_fires_rule_1() {
        let content = "---\njust a string\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingFrontmatter,
                message: "missing or invalid YAML frontmatter".to_string(),
            }]
        );
    }

    #[test]
    fn non_string_type_value_fires_rule_2() {
        let content = "---\ntype: 5\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingType,
                message: "frontmatter missing required non-empty 'type' field".to_string(),
            }]
        );
    }

    #[test]
    fn list_type_value_fires_rule_2() {
        let content = "---\ntype: [a, b]\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingType,
                message: "frontmatter missing required non-empty 'type' field".to_string(),
            }]
        );
    }

    #[test]
    fn empty_string_type_value_fires_rule_2() {
        let content = "---\ntype: \"\"\n---\nbody";
        assert_eq!(
            check_concept(content),
            vec![Diagnostic {
                line: 1,
                rule: Rule::OkfMissingType,
                message: "frontmatter missing required non-empty 'type' field".to_string(),
            }]
        );
    }

    #[test]
    fn non_empty_type_value_has_no_diagnostics() {
        let content = "---\ntype: concept\n---\nbody";
        assert_eq!(check_concept(content), vec![]);
    }

    // OKF v0.2 (upstream SPEC.md, PR #227) added optional provenance/trust/
    // lifecycle frontmatter families (`sources`, `generated`, `verified`,
    // `status`, `stale_after`) and a new `Attested Computation` concept type
    // with its own optional fields (`runtime`, `parameters`, `computation`,
    // `executor`, `attester`). The change is additive and backward-compatible:
    // the conformance clause this checker implements is unchanged, and none
    // of these keys are validated here. These tests guard that regression —
    // a v0.2 concept exercising every new family must still lint clean.
    #[test]
    fn v0_2_full_frontmatter_has_no_diagnostics() {
        assert_eq!(check_concept(V0_2_FULL_FRONTMATTER), vec![]);
    }

    #[test]
    fn v0_2_attested_computation_with_bare_verified_mapping_has_no_diagnostics() {
        assert_eq!(
            check_concept(V0_2_ATTESTED_COMPUTATION_BARE_VERIFIED),
            vec![]
        );
    }
}
