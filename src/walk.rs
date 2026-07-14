use crate::lint::LintError;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// True for any entry below `root` (depth > 0) whose name starts with `.` — the root
/// itself is never treated as hidden, even if its own path is dot-prefixed (e.g. a
/// tempdir). Used with `filter_entry` so `WalkDir` prunes hidden directories instead
/// of just skipping their contents one file at a time — this keeps traversal out of
/// `.git` and similar heavy directories entirely, not just out of the final results.
fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() > 0
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with('.'))
}

pub fn collect_md_files(root: &Path) -> Result<Vec<PathBuf>, LintError> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = entry.map_err(|err| {
            let path = err.path().unwrap_or(root).to_path_buf();
            let source = err
                .into_io_error()
                .unwrap_or_else(|| std::io::Error::other("directory walk failed"));
            LintError::Io { path, source }
        })?;

        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_path_buf();
        files.push(relative);
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn dot_prefixed_root_itself_is_walked() {
        // tempfile::TempDir defaults to a '.'-prefixed directory name.
        let root = TempDir::new().unwrap();
        assert!(
            root.path()
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.')),
            "expected tempfile to produce a dot-prefixed root for this test to be meaningful"
        );
        fs::write(root.path().join("a.md"), "").unwrap();

        let files = collect_md_files(root.path()).unwrap();

        assert_eq!(files, vec![PathBuf::from("a.md")]);
    }

    #[test]
    fn only_md_files_returned() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("a.md"), "").unwrap();
        fs::write(root.path().join("b.txt"), "").unwrap();

        let files = collect_md_files(root.path()).unwrap();

        assert_eq!(files, vec![PathBuf::from("a.md")]);
    }

    #[test]
    fn hidden_files_and_directories_are_excluded() {
        // Interview Q2 (planning/claude-spec.md §5): hidden files/dirs are skipped
        // entirely, not just excluded from results — traversal must not descend into
        // them (e.g. `.git`) at all.
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join(".hidden_dir")).unwrap();
        fs::write(root.path().join(".hidden_dir/inside.md"), "").unwrap();
        fs::write(root.path().join(".hidden.md"), "").unwrap();
        fs::write(root.path().join("visible.md"), "").unwrap();

        let files = collect_md_files(root.path()).unwrap();

        assert_eq!(files, vec![PathBuf::from("visible.md")]);
    }

    #[test]
    fn paths_are_relative_to_root() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("sub")).unwrap();
        fs::write(root.path().join("sub/nested.md"), "").unwrap();

        let files = collect_md_files(root.path()).unwrap();

        assert_eq!(files, vec![PathBuf::from("sub/nested.md")]);
        assert!(!files[0].is_absolute());
    }

    #[test]
    fn results_are_sorted_lexicographically() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("z.md"), "").unwrap();
        fs::write(root.path().join("a.md"), "").unwrap();
        fs::write(root.path().join("m.md"), "").unwrap();

        let files = collect_md_files(root.path()).unwrap();

        assert_eq!(
            files,
            vec![
                PathBuf::from("a.md"),
                PathBuf::from("m.md"),
                PathBuf::from("z.md"),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn permission_denied_subdirectory_is_io_error() {
        use std::os::unix::fs::PermissionsExt;

        let root = TempDir::new().unwrap();
        let blocked = root.path().join("blocked");
        fs::create_dir_all(&blocked).unwrap();
        fs::write(blocked.join("secret.md"), "").unwrap();
        fs::write(root.path().join("visible.md"), "").unwrap();

        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();

        // Root (and some CI containers) bypass Unix permission bits, so the
        // chmod above would have no effect. Skip rather than assert on a
        // codepath that wasn't actually exercised.
        let actually_blocked = fs::read_dir(&blocked).is_err();

        let result = collect_md_files(root.path());

        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755)).unwrap();

        if actually_blocked {
            assert!(matches!(result, Err(LintError::Io { .. })));
        } else {
            eprintln!(
                "skipping permission_denied_subdirectory_is_io_error: \
                 running with privileges that bypass Unix permission bits"
            );
        }
    }
}
