use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    // Only used when `command` is None: bare `okf-lint <path>` is an implicit
    // `lint` invocation, kept for backward compatibility with the pre-subcommand CLI.
    pub path: Option<PathBuf>,
    #[arg(long, default_value_t = 100)]
    pub max_line_length: u32,
    /// Walk into hidden (dot-prefixed) files and directories, e.g. `.git`. Off by
    /// default.
    #[arg(long)]
    pub include_hidden: bool,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Lint(LintArgs),
    Fmt(FmtArgs),
}

#[derive(clap::Args)]
pub struct LintArgs {
    pub path: PathBuf,
    #[arg(long, default_value_t = 100)]
    pub max_line_length: u32,
    /// Walk into hidden (dot-prefixed) files and directories, e.g. `.git`. Off by
    /// default.
    #[arg(long)]
    pub include_hidden: bool,
}

#[derive(clap::Args)]
pub struct FmtArgs {
    pub path: PathBuf,
    #[arg(long, default_value_t = 100)]
    pub max_line_length: u32,
    #[arg(long, default_value_t = 4)]
    pub tab_width: u32,
    #[arg(long)]
    pub check: bool,
    /// Walk into hidden (dot-prefixed) files and directories, e.g. `.git`. Off by
    /// default.
    #[arg(long)]
    pub include_hidden: bool,
}
