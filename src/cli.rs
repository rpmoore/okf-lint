#[derive(clap::Parser)]
pub struct Cli {
    pub path: std::path::PathBuf,
    #[arg(long, default_value_t = 100)]
    pub max_line_length: u32,
}
