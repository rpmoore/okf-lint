#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    // OKF conformance, in this fixed order:
    OkfMissingFrontmatter,
    OkfMissingType,
    OkfIndexFrontmatterPlacement,
    OkfIndexBodyStructure,
    OkfLogDateHeading,
    // OKF v0.2 optional-family format checks, in this fixed order. These
    // fields are never required (§11 conformance is unchanged from v0.1);
    // these rules only fire when a document includes one and its shape
    // doesn't match what §5/§10 document.
    OkfInvalidSources,
    OkfInvalidGenerated,
    OkfInvalidVerified,
    OkfInvalidStatus,
    OkfInvalidStaleAfter,
    OkfAttestedComputationMissingRuntime,
    // Markdown style, in this fixed order:
    StyleLineLength,
    StyleTrailingWhitespace,
    StyleTrailingNewline,
    StyleConsecutiveBlankLines,
    StyleHardTab,
}

impl Rule {
    /// The OKF spec section this rule enforces, or `None` for the generic
    /// markdown-style rules, which are project convention rather than
    /// OKF-derived requirements and have no corresponding spec section.
    pub fn spec_url(&self) -> Option<&'static str> {
        match self {
            Rule::OkfMissingFrontmatter | Rule::OkfMissingType => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#41-frontmatter",
            ),
            Rule::OkfIndexFrontmatterPlacement | Rule::OkfIndexBodyStructure => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#8-index-files",
            ),
            Rule::OkfLogDateHeading => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#9-log-files",
            ),
            Rule::OkfInvalidSources => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#51-provenance-sources",
            ),
            Rule::OkfInvalidGenerated | Rule::OkfInvalidVerified => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#52-trust-generated-and-verified",
            ),
            Rule::OkfInvalidStatus => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#54-lifecycle-status",
            ),
            Rule::OkfInvalidStaleAfter => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#55-lifecycle-stale_after",
            ),
            Rule::OkfAttestedComputationMissingRuntime => Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#102-contract-fields",
            ),
            Rule::StyleLineLength
            | Rule::StyleTrailingWhitespace
            | Rule::StyleTrailingNewline
            | Rule::StyleConsecutiveBlankLines
            | Rule::StyleHardTab => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub line: usize,
    pub rule: Rule,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_line_sorts_okf_before_style() {
        let okf = Diagnostic {
            line: 5,
            rule: Rule::OkfMissingType,
            message: "a".to_string(),
        };
        let style = Diagnostic {
            line: 5,
            rule: Rule::StyleHardTab,
            message: "b".to_string(),
        };
        assert!(okf.rule < style.rule);
    }

    #[test]
    fn okf_rules_have_spec_urls() {
        assert_eq!(
            Rule::OkfMissingFrontmatter.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#41-frontmatter"
            )
        );
        assert_eq!(
            Rule::OkfMissingType.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#41-frontmatter"
            )
        );
        assert_eq!(
            Rule::OkfIndexFrontmatterPlacement.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#8-index-files"
            )
        );
        assert_eq!(
            Rule::OkfIndexBodyStructure.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#8-index-files"
            )
        );
        assert_eq!(
            Rule::OkfLogDateHeading.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#9-log-files"
            )
        );
        assert_eq!(
            Rule::OkfInvalidSources.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#51-provenance-sources"
            )
        );
        assert_eq!(
            Rule::OkfInvalidGenerated.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#52-trust-generated-and-verified"
            )
        );
        assert_eq!(
            Rule::OkfInvalidVerified.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#52-trust-generated-and-verified"
            )
        );
        assert_eq!(
            Rule::OkfInvalidStatus.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#54-lifecycle-status"
            )
        );
        assert_eq!(
            Rule::OkfInvalidStaleAfter.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#55-lifecycle-stale_after"
            )
        );
        assert_eq!(
            Rule::OkfAttestedComputationMissingRuntime.spec_url(),
            Some(
                "https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#102-contract-fields"
            )
        );
    }

    #[test]
    fn style_rules_have_no_spec_url() {
        assert_eq!(Rule::StyleLineLength.spec_url(), None);
        assert_eq!(Rule::StyleTrailingWhitespace.spec_url(), None);
        assert_eq!(Rule::StyleTrailingNewline.spec_url(), None);
        assert_eq!(Rule::StyleConsecutiveBlankLines.spec_url(), None);
        assert_eq!(Rule::StyleHardTab.spec_url(), None);
    }

    #[test]
    fn rule_declaration_order_is_fixed() {
        assert!(Rule::OkfMissingFrontmatter < Rule::OkfMissingType);
        assert!(Rule::OkfMissingType < Rule::OkfIndexFrontmatterPlacement);
        assert!(Rule::OkfIndexFrontmatterPlacement < Rule::OkfIndexBodyStructure);
        assert!(Rule::OkfIndexBodyStructure < Rule::OkfLogDateHeading);
        assert!(Rule::OkfLogDateHeading < Rule::OkfInvalidSources);
        assert!(Rule::OkfInvalidSources < Rule::OkfInvalidGenerated);
        assert!(Rule::OkfInvalidGenerated < Rule::OkfInvalidVerified);
        assert!(Rule::OkfInvalidVerified < Rule::OkfInvalidStatus);
        assert!(Rule::OkfInvalidStatus < Rule::OkfInvalidStaleAfter);
        assert!(Rule::OkfInvalidStaleAfter < Rule::OkfAttestedComputationMissingRuntime);
        assert!(Rule::OkfAttestedComputationMissingRuntime < Rule::StyleLineLength);
        assert!(Rule::StyleLineLength < Rule::StyleTrailingWhitespace);
        assert!(Rule::StyleTrailingWhitespace < Rule::StyleTrailingNewline);
        assert!(Rule::StyleTrailingNewline < Rule::StyleConsecutiveBlankLines);
        assert!(Rule::StyleConsecutiveBlankLines < Rule::StyleHardTab);
    }
}
