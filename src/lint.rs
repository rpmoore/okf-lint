use std::path::PathBuf;

// PathNotFound/NotADirectory/InvalidUtf8 are unused until section-06's
// lint_bundle wires them up — expect (and don't suppress) dead-code warnings
// on those variants until then.
#[derive(Debug)]
pub enum LintError {
    PathNotFound(PathBuf),
    NotADirectory(PathBuf),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidUtf8(PathBuf),
}
