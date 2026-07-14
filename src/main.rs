mod checks;
mod cli;
mod diagnostic;
mod fmt;
mod frontmatter;
mod lint;
mod walk;

use clap::Parser;
use cli::{Cli, Command, FmtArgs};
use diagnostic::Diagnostic;
use lint::LintError;
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Lint(args)) => run_lint(&args.path, args.max_line_length as usize),
        Some(Command::Fmt(args)) => run_fmt_command(&args),
        None => match cli.path {
            Some(path) => run_lint(&path, cli.max_line_length as usize),
            None => {
                eprintln!("error: the following required argument was not provided: <PATH>");
                ExitCode::from(2)
            }
        },
    }
}

fn run_lint(path: &Path, max_line_length: usize) -> ExitCode {
    let diagnostics = match lint::lint_bundle(path, max_line_length) {
        Ok(diagnostics) => diagnostics,
        Err(err) => {
            eprintln!("error: {}", format_error(&err));
            return ExitCode::from(2);
        }
    };

    if diagnostics.is_empty() {
        return ExitCode::from(0);
    }

    match print_diagnostics(&diagnostics) {
        Ok(()) => ExitCode::from(1),
        Err(code) => code,
    }
}

fn run_fmt_command(args: &FmtArgs) -> ExitCode {
    let outcome = match fmt::run_fmt(
        &args.path,
        args.max_line_length as usize,
        args.tab_width as usize,
        args.check,
    ) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("error: {}", format_error(&err));
            return ExitCode::from(2);
        }
    };

    if args.check {
        if outcome.changed_files.is_empty() {
            return ExitCode::from(0);
        }

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        for path in &outcome.changed_files {
            if let Err(err) = writeln!(handle, "would reformat: {}", path.display()) {
                if err.kind() == ErrorKind::BrokenPipe {
                    return ExitCode::from(1);
                }
                eprintln!("error: failed to write to stdout: {err}");
                return ExitCode::from(2);
            }
        }
        return ExitCode::from(1);
    }

    if outcome.remaining.is_empty() {
        return ExitCode::from(0);
    }

    match print_diagnostics(&outcome.remaining) {
        Ok(()) => ExitCode::from(1),
        Err(code) => code,
    }
}

/// Writes each `{path}:{line}: {message}` diagnostic line to stdout. Returns `Err`
/// with the exit code the caller should use immediately (broken pipe -> 1, any other
/// write failure -> 2) instead of continuing to write further diagnostics.
fn print_diagnostics(diagnostics: &[(PathBuf, Diagnostic)]) -> Result<(), ExitCode> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for (path, diagnostic) in diagnostics {
        let spec_suffix = match diagnostic.rule.spec_url() {
            Some(url) => format!(" (spec: {url})"),
            None => String::new(),
        };
        if let Err(err) = writeln!(
            handle,
            "{}:{}: {}{}",
            path.display(),
            diagnostic.line,
            diagnostic.message,
            spec_suffix
        ) {
            if err.kind() == ErrorKind::BrokenPipe {
                return Err(ExitCode::from(1));
            }
            eprintln!("error: failed to write to stdout: {err}");
            return Err(ExitCode::from(2));
        }
    }
    Ok(())
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
