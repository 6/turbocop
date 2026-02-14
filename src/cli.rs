use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rblint", version, about = "A fast Ruby linter")]
pub struct Args {
    /// Files or directories to lint
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(short, long, default_value = "text", value_parser = ["text", "json"])]
    pub format: String,

    /// Run only the specified cops (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,

    /// Exclude the specified cops (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub except: Vec<String>,

    /// Disable color output
    #[arg(long)]
    pub no_color: bool,

    /// Enable debug output
    #[arg(long)]
    pub debug: bool,
}
