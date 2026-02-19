pub mod cli;
pub mod config;
pub mod cop;
pub mod diagnostic;
pub mod formatter;
pub mod fs;
pub mod linter;
pub mod parse;

#[cfg(test)]
pub mod testutil;

use std::collections::HashSet;
use std::io::Read;

use anyhow::Result;

use cli::Args;
use config::load_config;
use cop::registry::CopRegistry;
use formatter::create_formatter;
use fs::discover_files;
use linter::{lint_source, run_linter};
use parse::source::SourceFile;

/// Run the linter. Returns the exit code: 0 = clean, 1 = offenses found, 2 = error.
pub fn run(args: Args) -> Result<i32> {
    let target_dir = args.paths.first().map(|p| {
        if p.is_file() {
            p.parent().unwrap_or(p)
        } else {
            p.as_path()
        }
    });
    let config_start = std::time::Instant::now();
    let config = load_config(args.config.as_deref(), target_dir)?;
    let config_elapsed = config_start.elapsed();

    if args.debug {
        eprintln!("debug: config loading total: {config_elapsed:.0?}");

        if let Some(dir) = config.config_dir() {
            eprintln!("debug: config loaded from: {}", dir.display());
        } else {
            eprintln!("debug: no config file found");
        }
        eprintln!(
            "debug: global excludes: {:?}",
            config.global_excludes()
        );
    }

    let registry = CopRegistry::default_registry();

    // --list-cops: print all registered cop names and exit
    if args.list_cops {
        let mut names: Vec<&str> = registry.cops().iter().map(|c| c.name()).collect();
        names.sort();
        for name in names {
            println!("{name}");
        }
        return Ok(0);
    }

    // --rubocop-only: print uncovered cops and exit
    if args.rubocop_only {
        let covered: HashSet<&str> = registry.cops().iter().map(|c| c.name()).collect();
        let mut remaining: Vec<String> = config
            .enabled_cop_names()
            .into_iter()
            .filter(|name| !covered.contains(name.as_str()))
            .collect();
        remaining.sort();
        println!("{}", remaining.join(","));
        return Ok(0);
    }

    // --stdin: read from stdin and lint a single file
    if let Some(ref display_path) = args.stdin {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        let source = SourceFile::from_string(display_path.clone(), input);
        let result = lint_source(&source, &config, &registry, &args);
        let formatter = create_formatter(&args.format);
        formatter.print(&result.diagnostics, result.file_count);
        return if result.diagnostics.is_empty() {
            Ok(0)
        } else {
            Ok(1)
        };
    }

    let files = discover_files(&args.paths, &config)?;

    if args.debug {
        eprintln!("debug: {} files to lint", files.len());
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
