#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    // OKF conformance, in this fixed order:
    OkfMissingFrontmatter,
    OkfMissingType,
    OkfIndexFrontmatterPlacement,
    OkfIndexBodyStructure,
    OkfLogDateHeading,
    // Markdown style, in this fixed order:
    StyleLineLength,
    StyleTrailingWhitespace,
    StyleTrailingNewline,
    StyleConsecutiveBlankLines,
    StyleHardTab,
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
    fn rule_declaration_order_is_fixed() {
        assert!(Rule::OkfMissingFrontmatter < Rule::OkfMissingType);
        assert!(Rule::OkfMissingType < Rule::OkfIndexFrontmatterPlacement);
        assert!(Rule::OkfIndexFrontmatterPlacement < Rule::OkfIndexBodyStructure);
        assert!(Rule::OkfIndexBodyStructure < Rule::OkfLogDateHeading);
        assert!(Rule::OkfLogDateHeading < Rule::StyleLineLength);
        assert!(Rule::StyleLineLength < Rule::StyleTrailingWhitespace);
        assert!(Rule::StyleTrailingWhitespace < Rule::StyleTrailingNewline);
        assert!(Rule::StyleTrailingNewline < Rule::StyleConsecutiveBlankLines);
        assert!(Rule::StyleConsecutiveBlankLines < Rule::StyleHardTab);
    }
}
