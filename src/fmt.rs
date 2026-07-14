use crate::checks::style_fix::fix_style;
use crate::diagnostic::Diagnostic;
use crate::lint::{self, LintError};
use crate::walk::collect_md_files;
use std::path::{Path, PathBuf};

pub struct FmtOutcome {
    pub changed_files: Vec<PathBuf>,
    pub remaining: Vec<(PathBuf, Diagnostic)>,
}

pub fn run_fmt(
    root: &Path,
    max_line_length: usize,
    tab_width: usize,
    check: bool,
    include_hidden: bool,
) -> Result<FmtOutcome, LintError> {
    let metadata =
        std::fs::metadata(root).map_err(|_| LintError::PathNotFound(root.to_path_buf()))?;
    if !metadata.is_dir() {
        return Err(LintError::NotADirectory(root.to_path_buf()));
    }

    let relative_paths = collect_md_files(root, include_hidden)?;

    let mut changed_files = Vec::new();
    for relative in &relative_paths {
        let full_path = root.join(relative);
        let bytes = std::fs::read(&full_path).map_err(|source| LintError::Io {
            path: full_path.clone(),
            source,
        })?;
        let content =
            String::from_utf8(bytes).map_err(|_| LintError::InvalidUtf8(full_path.clone()))?;

        let fixed = fix_style(&content, max_line_length, tab_width);
        if fixed != content {
            changed_files.push(relative.clone());
            if !check {
                std::fs::write(&full_path, &fixed).map_err(|source| LintError::Io {
                    path: full_path.clone(),
                    source,
                })?;
            }
        }
    }

    let remaining = if check {
        Vec::new()
    } else {
        lint::lint_bundle(root, max_line_length, include_hidden)?
    };

    Ok(FmtOutcome {
        changed_files,
        remaining,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Rule;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn fmt_root_not_found() {
        let missing = Path::new("/nonexistent/does/not/exist/okf-lint-fmt-test");
        let result = run_fmt(missing, 100, 4, false, false);
        assert!(matches!(result, Err(LintError::PathNotFound(_))));
    }

    #[test]
    fn fmt_root_is_a_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("not_a_dir.md");
        fs::write(&file_path, "hello").unwrap();

        let result = run_fmt(&file_path, 100, 4, false, false);
        assert!(matches!(result, Err(LintError::NotADirectory(_))));
    }

    #[test]
    fn fmt_writes_fixed_content_and_reports_no_remaining_style_diagnostics() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("a.md"),
            "---\ntype: concept\n---\nline with trailing space \n",
        )
        .unwrap();

        let outcome = run_fmt(dir.path(), 100, 4, false, false).unwrap();

        assert_eq!(outcome.changed_files, vec![PathBuf::from("a.md")]);
        let written = fs::read_to_string(dir.path().join("a.md")).unwrap();
        assert_eq!(
            written,
            "---\ntype: concept\n---\nline with trailing space\n"
        );
        assert!(
            outcome
                .remaining
                .iter()
                .all(|(_, d)| d.rule != Rule::StyleTrailingWhitespace)
        );
    }

    #[test]
    fn fmt_check_mode_does_not_write_files() {
        let dir = TempDir::new().unwrap();
        let original = "---\ntype: concept\n---\nline with trailing space \n";
        fs::write(dir.path().join("a.md"), original).unwrap();

        let outcome = run_fmt(dir.path(), 100, 4, true, false).unwrap();

        assert_eq!(outcome.changed_files, vec![PathBuf::from("a.md")]);
        assert!(outcome.remaining.is_empty());
        let untouched = fs::read_to_string(dir.path().join("a.md")).unwrap();
        assert_eq!(untouched, original);
    }

    #[test]
    fn fmt_include_hidden_toggles_hidden_file_fixing() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".hidden")).unwrap();
        fs::write(
            dir.path().join(".hidden/notes.md"),
            "line with trailing space \n",
        )
        .unwrap();

        let default_outcome = run_fmt(dir.path(), 100, 4, false, false).unwrap();
        assert!(default_outcome.changed_files.is_empty());
        let untouched = fs::read_to_string(dir.path().join(".hidden/notes.md")).unwrap();
        assert_eq!(untouched, "line with trailing space \n");

        let included_outcome = run_fmt(dir.path(), 100, 4, false, true).unwrap();
        assert_eq!(
            included_outcome.changed_files,
            vec![PathBuf::from(".hidden/notes.md")]
        );
        let fixed = fs::read_to_string(dir.path().join(".hidden/notes.md")).unwrap();
        assert_eq!(fixed, "line with trailing space\n");
    }

    #[test]
    fn fmt_clean_bundle_has_no_changed_files() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("a.md"),
            "---\ntype: concept\n---\n# Title\n",
        )
        .unwrap();

        let outcome = run_fmt(dir.path(), 100, 4, false, false).unwrap();

        assert!(outcome.changed_files.is_empty());
    }
}
