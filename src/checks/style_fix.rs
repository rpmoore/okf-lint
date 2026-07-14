/// Mirrors `check_style`'s five rules, but rewrites content instead of diagnosing it.
/// Pipeline (each stage consumes the previous stage's output):
///   1. hard tabs -> spaces
///   2. trailing whitespace trimmed (also normalizes CRLF -> LF)
///   3. consecutive blank lines collapsed to one
///   4. overlong lines rewrapped, but only inside paragraphs that are unambiguously
///      plain text (no frontmatter, code fences, headings, tables, list items,
///      blockquotes, or links/URLs)
///   5. exactly one trailing newline
pub fn fix_style(content: &str, max_line_length: usize, tab_width: usize) -> String {
    if content.is_empty() {
        return content.to_string();
    }

    let mut lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();
    if content.ends_with('\n') {
        lines.pop();
    }

    let tab_replacement = " ".repeat(tab_width);
    for line in &mut lines {
        if line.contains('\t') {
            *line = line.replace('\t', &tab_replacement);
        }
        let trimmed_len = line.trim_end_matches([' ', '\r']).len();
        line.truncate(trimmed_len);
    }

    let lines = collapse_blank_runs(lines);
    let mut lines = rewrap_overlong_blocks(lines, max_line_length);

    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    let mut result = lines.join("\n");
    result.push('\n');
    result
}

fn collapse_blank_runs(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut blank_run = 0usize;
    for line in lines {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                out.push(String::new());
            }
        } else {
            blank_run = 0;
            out.push(line);
        }
    }
    out
}

fn rewrap_overlong_blocks(lines: Vec<String>, max_line_length: usize) -> Vec<String> {
    let skip = compute_skip_flags(&lines);

    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }

        let start = i;
        while i < lines.len() && !lines[i].trim().is_empty() {
            i += 1;
        }
        let block = &lines[start..i];
        let block_skip = &skip[start..i];

        let is_rewrap_candidate = !block_skip.iter().any(|&s| s);
        let has_overlong_line = block.iter().any(|l| l.chars().count() > max_line_length);

        if is_rewrap_candidate && has_overlong_line {
            out.extend(wrap_block(block, max_line_length));
        } else {
            out.extend(block.iter().cloned());
        }
    }
    out
}

fn wrap_block(block: &[String], max_line_length: usize) -> Vec<String> {
    let text = block.join(" ");
    let words: Vec<&str> = text.split_whitespace().collect();

    let mut out = Vec::new();
    let mut current = String::new();
    for word in words {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= max_line_length {
            current.push(' ');
            current.push_str(word);
        } else {
            out.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Per-line "do not rewrap this" flags: frontmatter, fenced code blocks, headings,
/// table rows, list items/blockquotes, and lines carrying a link or bare URL.
fn compute_skip_flags(lines: &[String]) -> Vec<bool> {
    let mut flags = vec![false; lines.len()];
    let mut idx = 0;

    if lines.first().map(|l| l.as_str()) == Some("---") {
        flags[0] = true;
        idx = 1;
        while idx < lines.len() {
            flags[idx] = true;
            let closed = lines[idx] == "---";
            idx += 1;
            if closed {
                break;
            }
        }
    }

    let mut in_fence = false;
    for i in idx..lines.len() {
        let trimmed = lines[i].trim_start();
        let is_fence_delim = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if is_fence_delim {
            flags[i] = true;
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            flags[i] = true;
            continue;
        }
        if is_heading(&lines[i])
            || lines[i].contains('|')
            || is_list_or_quote(&lines[i])
            || has_link_or_url(&lines[i])
        {
            flags[i] = true;
        }
    }

    flags
}

fn is_heading(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

fn is_list_or_quote(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with('>') {
        return true;
    }
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
        return true;
    }
    if t == "-" || t == "*" || t == "+" {
        return true;
    }
    let digits_end = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digits_end > 0 {
        let rest = &t[digits_end..];
        if rest.starts_with(". ") || rest.starts_with(") ") {
            return true;
        }
    }
    false
}

fn has_link_or_url(line: &str) -> bool {
    line.contains("](") || line.contains("http://") || line.contains("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::style::check_style;
    use crate::diagnostic::Rule;

    const MAX: usize = 100;

    fn fixture(name: &str, kind: &str) -> String {
        std::fs::read_to_string(format!("tests/fixtures/fmt/{name}/{kind}.md")).unwrap()
    }

    #[test]
    fn hard_tabs_are_expanded_to_spaces() {
        let before = fixture("hard_tabs", "before");
        let after = fixture("hard_tabs", "after");
        assert_eq!(fix_style(&before, MAX, 4), after);
    }

    #[test]
    fn trailing_whitespace_is_trimmed() {
        let before = fixture("trailing_whitespace", "before");
        let after = fixture("trailing_whitespace", "after");
        assert_eq!(fix_style(&before, MAX, 4), after);
    }

    #[test]
    fn consecutive_blank_lines_are_collapsed() {
        let before = fixture("consecutive_blank_lines", "before");
        let after = fixture("consecutive_blank_lines", "after");
        assert_eq!(fix_style(&before, MAX, 4), after);
    }

    #[test]
    fn trailing_newline_is_normalized() {
        let before = fixture("trailing_newline", "before");
        let after = fixture("trailing_newline", "after");
        assert_eq!(fix_style(&before, MAX, 4), after);
    }

    #[test]
    fn plain_paragraph_is_rewrapped() {
        let before = fixture("max_line_length", "before");
        let after = fixture("max_line_length", "after");
        assert_eq!(fix_style(&before, MAX, 4), after);
    }

    #[test]
    fn overlong_lines_in_skip_contexts_are_left_alone() {
        let before = fixture("max_line_length_skip", "before");
        // Nothing should change: table/code/heading/list/link lines aren't rewrapped.
        assert_eq!(fix_style(&before, MAX, 4), before);
    }

    #[test]
    fn empty_content_is_left_alone() {
        assert_eq!(fix_style("", MAX, 4), "");
    }

    #[test]
    fn already_clean_content_is_idempotent() {
        let content = "# Title\n\nSome text.\n";
        assert_eq!(fix_style(content, MAX, 4), content);
    }

    #[test]
    fn fix_then_check_reports_no_mechanical_style_violations() {
        let dirty = "# Title\t\n\n\n\nline with trailing space \n";
        let fixed = fix_style(dirty, MAX, 4);
        let diags = check_style(&fixed, MAX);
        assert!(diags.iter().all(|d| d.rule != Rule::StyleHardTab));
        assert!(
            diags
                .iter()
                .all(|d| d.rule != Rule::StyleTrailingWhitespace)
        );
        assert!(diags.iter().all(|d| d.rule != Rule::StyleTrailingNewline));
        assert!(
            diags
                .iter()
                .all(|d| d.rule != Rule::StyleConsecutiveBlankLines)
        );
    }

    #[test]
    fn fixing_twice_is_a_no_op() {
        let dirty = "line one\t\n\n\n\nline two   \n\n";
        let once = fix_style(dirty, MAX, 4);
        let twice = fix_style(&once, MAX, 4);
        assert_eq!(once, twice);
    }
}
