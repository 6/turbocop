pub mod cache;
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
    // Validate --fail-level early
    let fail_level = diagnostic::Severity::from_str(&args.fail_level).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid --fail-level '{}'. Expected: convention, warning, error, fatal (or C, W, E, F)",
            args.fail_level
        )
    })?;

    let target_dir = args.paths.first().map(|p| {
        if p.is_file() {
            p.parent().unwrap_or(p)
        } else {
            p.as_path()
        }
    });

    let registry = CopRegistry::default_registry();

    // --list-cops: print all registered cop names and exit (no config needed)
    if args.list_cops {
        let mut names: Vec<&str> = registry.cops().iter().map(|c| c.name()).collect();
        names.sort();
        for name in names {
            println!("{name}");
        }
        return Ok(0);
    }

    // --cache-clear: remove result cache directory and exit
    if args.cache_clear {
        match cache::clear_cache() {
            Ok(()) => {
                eprintln!("Result cache cleared.");
                return Ok(0);
            }
            Err(e) => {
                anyhow::bail!("Failed to clear result cache: {e}");
            }
        }
    }

    // --init: resolve gem paths and write .turbocop.cache
    if args.init {
        let config_start = std::time::Instant::now();
        let config = load_config(args.config.as_deref(), target_dir, None)?;
        let config_elapsed = config_start.elapsed();

        let gem_paths = config::gem_path::drain_resolved_paths();
        let lock_dir = config
            .config_dir()
            .unwrap_or_else(|| target_dir.unwrap_or(std::path::Path::new(".")));
        config::lockfile::write_lock(&gem_paths, lock_dir)?;

        eprintln!(
            "Created .turbocop.cache ({} gems cached in {config_elapsed:.0?})",
            gem_paths.len()
        );
        for (name, path) in &gem_paths {
            eprintln!("  {name}: {}", path.display());
        }
        return Ok(0);
    }

    // Determine whether to use lockfile:
    // --no-lock, --rubocop-only, --list-target-files, and --stdin bypass the lockfile requirement
    let use_cache =
        !args.no_cache && !args.rubocop_only && !args.list_target_files && args.stdin.is_none();

    // Load config — use lockfile if available
    let config_start = std::time::Instant::now();
    let config = if use_cache {
        // Try to find config dir for lockfile lookup
        let lock_dir = target_dir.unwrap_or(std::path::Path::new("."));
        match config::lockfile::read_lock(lock_dir) {
            Ok(lock) => {
                config::lockfile::check_freshness(&lock, lock_dir)?;
                if args.debug {
                    eprintln!(
                        "debug: using .turbocop.cache ({} cached gems)",
                        lock.gems.len()
                    );
                }
                load_config(args.config.as_deref(), target_dir, Some(&lock.gems))?
            }
            Err(e) => {
                // If lockfile is missing, fail with helpful message
                anyhow::bail!("{e}");
            }
        }
    } else {
        load_config(args.config.as_deref(), target_dir, None)?
    };
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
        return if result
            .diagnostics
            .iter()
            .any(|d| d.severity >= fail_level)
        {
            Ok(1)
        } else {
            Ok(0)
        };
    }

    let discovered = discover_files(&args.paths, &config)?;

    // --list-target-files (-L): print files that would be linted, then exit
    if args.list_target_files {
        let cop_filters = config.build_cop_filters(&registry);
        for file in &discovered.files {
            if cop_filters.is_globally_excluded(file) {
                let is_explicit = discovered.explicit.contains(file)
                    || file
                        .canonicalize()
                        .ok()
                        .is_some_and(|c| discovered.explicit.contains(&c));
                if args.force_exclusion || !is_explicit {
                    continue;
                }
            }
            println!("{}", file.display());
        }
        return Ok(0);
    }

    if args.debug {
        eprintln!("debug: {} files to lint", discovered.files.len());
        eprintln!("debug: {} cops registered", registry.len());
    }

    let result = run_linter(&discovered, &config, &registry, &args);
    let formatter = create_formatter(&args.format);
    formatter.print(&result.diagnostics, result.file_count);

    if result
        .diagnostics
        .iter()
        .any(|d| d.severity >= fail_level)
    {
        Ok(1)
    } else {
        Ok(0)
    }
}
