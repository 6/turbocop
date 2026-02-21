use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutocorrectMode {
    Off,
    /// `-a` / `--autocorrect`: safe corrections only.
    Safe,
    /// `-A` / `--autocorrect-all`: all corrections including unsafe.
    All,
}

#[derive(Parser, Debug)]
#[command(name = "turbocop", version, about = "A fast Ruby linter")]
pub struct Args {
    /// Files or directories to lint
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(short, long, default_value = "progress", value_parser = ["progress", "text", "json", "github", "pacman", "quiet", "files", "emacs", "simple"])]
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

    /// Print comma-separated list of cops not covered by turbocop, then exit
    #[arg(long)]
    pub rubocop_only: bool,

    /// List all registered cop names, one per line, then exit
    #[arg(long)]
    pub list_cops: bool,

    /// List cops that support autocorrect, one per line, then exit
    #[arg(long)]
    pub list_autocorrectable_cops: bool,

    /// Read source from stdin, use PATH for display and config matching
    #[arg(long, value_name = "PATH")]
    pub stdin: Option<PathBuf>,

    /// Resolve gem paths and write lockfile to cache directory, then exit
    #[arg(long)]
    pub init: bool,

    /// Skip lockfile requirement (use bundler directly for gem resolution)
    #[arg(long)]
    pub no_cache: bool,

    /// Enable/disable file-level result caching [default: true]
    #[arg(long, default_value = "true", hide_default_value = true)]
    pub cache: String,

    /// Clear the result cache and exit
    #[arg(long)]
    pub cache_clear: bool,

    /// Minimum severity for a non-zero exit code (convention, warning, error, fatal, or C/W/E/F)
    #[arg(long, value_name = "SEVERITY", default_value = "convention")]
    pub fail_level: String,

    /// Stop after first file with offenses
    #[arg(short = 'F', long)]
    pub fail_fast: bool,

    /// Apply AllCops.Exclude to explicitly-passed files (by default, explicit files bypass exclusion)
    #[arg(long)]
    pub force_exclusion: bool,

    /// Print files that would be linted, then exit
    #[arg(short = 'L', long)]
    pub list_target_files: bool,

    /// Display cop names in offense output (accepted for RuboCop compatibility; always enabled)
    #[arg(short = 'D', long)]
    pub display_cop_names: bool,

    /// Use parallel processing (accepted for RuboCop compatibility; always enabled)
    #[arg(short = 'P', long)]
    pub parallel: bool,

    /// Load additional Ruby files (accepted for RuboCop compatibility; ignored)
    #[arg(short = 'r', long = "require")]
    pub require_libs: Vec<String>,

    /// Ignore all `# rubocop:disable` inline comments
    #[arg(long)]
    pub ignore_disable_comments: bool,

    /// Ignore all config files and use built-in defaults only
    #[arg(long)]
    pub force_default_config: bool,

    /// Autocorrect offenses (safe cops only)
    #[arg(short = 'a', long = "autocorrect")]
    pub autocorrect: bool,

    /// Autocorrect offenses (all cops, including unsafe)
    #[arg(short = 'A', long = "autocorrect-all")]
    pub autocorrect_all: bool,
}

impl Args {
    /// Resolve the autocorrect mode from CLI flags.
    /// `-A` takes precedence over `-a` (matching RuboCop behavior).
    pub fn autocorrect_mode(&self) -> AutocorrectMode {
        if self.autocorrect_all {
            AutocorrectMode::All
        } else if self.autocorrect {
            AutocorrectMode::Safe
        } else {
            AutocorrectMode::Off
        }
    }
}
