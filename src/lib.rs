pub mod cli;
pub mod config;
pub mod cop;
pub mod diagnostic;
pub mod formatter;
pub mod fs;
pub mod linter;
pub mod parse;

use anyhow::Result;

use cli::Args;
use config::load_config;
use cop::registry::CopRegistry;
use formatter::create_formatter;
use fs::discover_files;
use linter::run_linter;

/// Run the linter. Returns the exit code: 0 = clean, 1 = offenses found, 2 = error.
pub fn run(args: Args) -> Result<i32> {
    let config = load_config(args.config.as_deref())?;

    if args.debug {
        eprintln!(
            "debug: global excludes: {:?}",
            config.global_excludes()
        );
    }

    let files = discover_files(&args.paths, &config)?;

    if args.debug {
        eprintln!("debug: {} files to lint", files.len());
    }

    let registry = CopRegistry::default_registry();

    if args.debug {
        eprintln!("debug: {} cops registered", registry.len());
    }

    let result = run_linter(&files, &config, &registry, &args);
    let formatter = create_formatter(&args.format);
    formatter.print(&result.diagnostics, result.file_count);

    if result.diagnostics.is_empty() {
        Ok(0)
    } else {
        Ok(1)
    }
}
