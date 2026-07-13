#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontmatterResult {
    None,
    Unclosed,
    Found {
        yaml_text: String,
        body_start_line: usize,
    },
}

// Trims a trailing '\r' so CRLF-terminated files are treated the same as
// LF-terminated ones when detecting the "---" delimiter line.
fn strip_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

pub fn split_frontmatter(content: &str) -> FrontmatterResult {
    let mut lines = content.split('\n');
    match lines.next().map(strip_cr) {
        Some("---") => {}
        _ => return FrontmatterResult::None,
    }

    let mut yaml_lines = Vec::new();
    let mut consumed = 1; // the opening "---" line
    for line in lines {
        consumed += 1;
        if strip_cr(line) == "---" {
            return FrontmatterResult::Found {
                yaml_text: yaml_lines.join("\n"),
                body_start_line: consumed + 1,
            };
        }
        yaml_lines.push(line);
    }

    FrontmatterResult::Unclosed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_leading_delimiter_is_none() {
        assert_eq!(split_frontmatter("# Title\n\nbody"), FrontmatterResult::None);
    }

    #[test]
    fn unclosed_block_is_unclosed() {
        assert_eq!(
            split_frontmatter("---\ntype: concept\nbody without closing"),
            FrontmatterResult::Unclosed
        );
    }

    #[test]
    fn well_formed_block_is_found() {
        let content = "---\ntype: concept\n---\n# Body\n";
        match split_frontmatter(content) {
            FrontmatterResult::Found {
                yaml_text,
                body_start_line,
            } => {
                assert_eq!(yaml_text, "type: concept");
                assert_eq!(body_start_line, 4);
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn leading_blank_line_before_delimiter_is_none() {
        assert_eq!(
            split_frontmatter("\n---\ntype: concept\n---\nbody"),
            FrontmatterResult::None
        );
    }

    #[test]
    fn delimiter_with_trailing_characters_is_none() {
        assert_eq!(
            split_frontmatter("--- \ntype: concept\n---\nbody"),
            FrontmatterResult::None
        );
    }

    #[test]
    fn crlf_line_endings_are_treated_like_lf() {
        let content = "---\r\ntype: concept\r\n---\r\n# Body\r\n";
        match split_frontmatter(content) {
            FrontmatterResult::Found {
                yaml_text,
                body_start_line,
            } => {
                assert_eq!(yaml_text, "type: concept\r");
                assert_eq!(body_start_line, 4);
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }
}
