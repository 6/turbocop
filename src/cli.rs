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

    /// Print comma-separated list of cops not covered by rblint, then exit
    #[arg(long)]
    pub rubocop_only: bool,

    /// List all registered cop names, one per line, then exit
    #[arg(long)]
    pub list_cops: bool,

    /// Read source from stdin, use PATH for display and config matching
    #[arg(long, value_name = "PATH")]
    pub stdin: Option<PathBuf>,

    /// Generate .rblint.cache with cached gem paths and exit
    #[arg(long)]
    pub init: bool,

    /// Skip .rblint.cache requirement (use bundler directly for gem resolution)
    #[arg(long)]
    pub no_cache: bool,

    /// Enable file-level result caching (skip re-linting unchanged files)
    #[arg(long)]
    pub cache: bool,

    /// Clear the result cache and exit
    #[arg(long)]
    pub cache_clear: bool,
}
