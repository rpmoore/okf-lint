/// Mirrors `check_style`'s five rules, but rewrites content instead of diagnosing it.
/// Pipeline (each stage consumes the previous stage's output):
///   1. hard tabs -> spaces
///   2. trailing whitespace trimmed (also normalizes CRLF -> LF)
///   3. consecutive blank lines collapsed to one
///   4. overlong lines rewrapped. Frontmatter, fenced code, headings, tables, and
///      blockquotes are left alone entirely. Plain paragraphs and list items *are*
///      rewrapped: a `[text](url)` link or bare URL is treated as one unsplittable
///      token (never broken across lines), and list item continuation lines get a
///      hanging indent matching the marker width (`"- "` -> 2 spaces, `"10. "` -> 4).
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

        if !is_rewrap_candidate || !has_overlong_line {
            out.extend(block.iter().cloned());
        } else if block.iter().any(|l| list_marker_prefix(l).is_some()) {
            out.extend(wrap_list_block(block, max_line_length));
        } else {
            out.extend(wrap_paragraph_block(block, max_line_length));
        }
    }
    out
}

fn wrap_paragraph_block(block: &[String], max_line_length: usize) -> Vec<String> {
    let text = block.join(" ");
    pack(&tokenize(&text), max_line_length, "", "")
}

/// Splits a block into list items (each line matching `list_marker_prefix` starts a
/// new item; subsequent non-marker lines are folded in as that item's continuation
/// text) and wraps each item independently with a hanging indent under its marker.
fn wrap_list_block(block: &[String], max_line_length: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0;
    while idx < block.len() {
        let line = &block[idx];
        let Some(marker) = list_marker_prefix(line) else {
            // Stray non-marker line before any marker in this block: nothing sane to
            // hang it off of, so leave it as-is.
            out.push(line.clone());
            idx += 1;
            continue;
        };

        let mut text = line[marker.len()..].to_string();
        idx += 1;
        while idx < block.len() && list_marker_prefix(&block[idx]).is_none() {
            text.push(' ');
            text.push_str(block[idx].trim_start());
            idx += 1;
        }

        let indent = " ".repeat(marker.chars().count());
        out.extend(pack(&tokenize(&text), max_line_length, &marker, &indent));
    }
    out
}

/// Greedily packs `tokens` into lines of at most `max_line_length` chars. The first
/// line starts with `first_prefix` (a list marker, or `""` for a plain paragraph);
/// every subsequent line starts with `indent`. No token is ever split, even if a
/// single token exceeds `max_line_length` on its own (e.g. a long link or URL).
fn pack(
    tokens: &[String],
    max_line_length: usize,
    first_prefix: &str,
    indent: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = first_prefix.to_string();
    let mut has_content = false;
    for token in tokens {
        if !has_content {
            current.push_str(token);
            has_content = true;
        } else if current.chars().count() + 1 + token.chars().count() <= max_line_length {
            current.push(' ');
            current.push_str(token);
        } else {
            out.push(std::mem::take(&mut current));
            current = indent.to_string();
            current.push_str(token);
            has_content = true;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Splits `text` into wrap tokens, treating a `[link text](url)` span as one
/// unsplittable token (even though it may contain internal spaces) so rewrapping
/// never breaks link syntax across lines. Bare URLs stay intact automatically, since
/// they contain no whitespace for a plain whitespace split to break on.
fn tokenize(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if bytes[i] == b'['
            && let Some(end) = find_link_end(text, i)
        {
            tokens.push(text[i..end].to_string());
            i = end;
            continue;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            if bytes[i] == b'[' && i > start && find_link_end(text, i).is_some() {
                break;
            }
            i += 1;
        }
        tokens.push(text[start..i].to_string());
    }
    tokens
}

/// If `text[start..]` begins a well-formed `[...](...)` span (`start` must point at
/// `'['`), returns the index just past its closing `)`. Otherwise `None`.
fn find_link_end(text: &str, start: usize) -> Option<usize> {
    debug_assert_eq!(text.as_bytes()[start], b'[');
    let close_bracket = text[start + 1..].find(']')? + start + 1;
    if text.as_bytes().get(close_bracket + 1) != Some(&b'(') {
        return None;
    }
    let close_paren = text[close_bracket + 2..].find(')')? + close_bracket + 2;
    Some(close_paren + 1)
}

/// Per-line "do not rewrap this" flags: frontmatter, fenced code blocks, headings,
/// table rows, and blockquotes. List items and plain paragraphs are handled by
/// `wrap_list_block`/`wrap_paragraph_block` instead of being skipped.
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
            || crate::checks::style::is_table_row(&lines[i])
            || is_blockquote(&lines[i])
        {
            flags[i] = true;
        }
    }

    flags
}

fn is_heading(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

fn is_blockquote(line: &str) -> bool {
    line.trim_start().starts_with('>')
}

/// Returns the marker prefix (leading whitespace + marker + exactly one space) if
/// `line` starts a bullet (`- `/`* `/`+ `) or ordered (`1. `/`1) `) list item.
fn list_marker_prefix(line: &str) -> Option<String> {
    let leading_ws_len = line.len() - line.trim_start().len();
    let rest = &line[leading_ws_len..];

    for bullet in ["- ", "* ", "+ "] {
        if rest.starts_with(bullet) {
            return Some(line[..leading_ws_len + bullet.len()].to_string());
        }
    }

    let digits_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digits_end > 0 {
        let after_digits = &rest[digits_end..];
        for sep in [". ", ") "] {
            if after_digits.starts_with(sep) {
                let marker_len = digits_end + sep.len();
                return Some(line[..leading_ws_len + marker_len].to_string());
            }
        }
    }

    None
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
        // Nothing should change: heading/table/code/blockquote lines aren't rewrapped.
        assert_eq!(fix_style(&before, MAX, 4), before);
    }

    #[test]
    fn list_items_are_rewrapped_with_hanging_indent() {
        let before = fixture("max_line_length_list", "before");
        let after = fixture("max_line_length_list", "after");
        assert_eq!(fix_style(&before, MAX, 4), after);
    }

    #[test]
    fn links_are_wrapped_as_a_single_unsplittable_token() {
        let before = fixture("max_line_length_link", "before");
        let after = fixture("max_line_length_link", "after");
        assert_eq!(fix_style(&before, MAX, 4), after);
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
