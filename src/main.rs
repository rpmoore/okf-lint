mod checks;
mod cli;
mod diagnostic;
mod frontmatter;
mod lint;
mod walk;

use clap::Parser;
use cli::Cli;
use lint::LintError;
use std::io::{self, ErrorKind, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let diagnostics = match lint::lint_bundle(&cli.path, cli.max_line_length as usize) {
        Ok(diagnostics) => diagnostics,
        Err(err) => {
            eprintln!("error: {}", format_error(&err));
            return ExitCode::from(2);
        }
    };

    if diagnostics.is_empty() {
        return ExitCode::from(0);
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for (path, diagnostic) in &diagnostics {
        if let Err(err) = writeln!(
            handle,
            "{}:{}: {}",
            path.display(),
            diagnostic.line,
            diagnostic.message
        ) {
            if err.kind() == ErrorKind::BrokenPipe {
                return ExitCode::from(1);
            }
            eprintln!("error: failed to write to stdout: {err}");
            return ExitCode::from(2);
        }
    }
    ExitCode::from(1)
}

fn format_error(err: &LintError) -> String {
    match err {
        LintError::PathNotFound(path) => format!("cannot access path: {}", path.display()),
        LintError::NotADirectory(path) => format!("not a directory: {}", path.display()),
        LintError::Io { path, source } => {
            format!("failed to read {}: {}", path.display(), source)
        }
        LintError::InvalidUtf8(path) => format!("file is not valid UTF-8: {}", path.display()),
    }
}
