use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

use crate::config::ResolvedConfig;

/// Discover Ruby files from the given paths, respecting .gitignore
/// and AllCops.Exclude patterns.
pub fn discover_files(paths: &[PathBuf], config: &ResolvedConfig) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            // Direct file paths bypass extension filtering
            files.push(path.clone());
        } else if path.is_dir() {
            let dir_files = walk_directory(path, config)?;
            files.extend(dir_files);
        } else {
            anyhow::bail!("path does not exist: {}", path.display());
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

/// Exposed for testing only.
fn walk_directory(dir: &Path, config: &ResolvedConfig) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(dir);
    builder.hidden(true).git_ignore(true).git_global(true);

    // NOTE: We intentionally do NOT use the `ignore` crate's OverrideBuilder
    // for AllCops.Exclude patterns. The OverrideBuilder uses gitignore-style
    // override semantics where `!pattern` = whitelist (include), not exclude,
    // and positive patterns exclude ALL non-matching files. Instead, we filter
    // discovered files manually against the global exclude GlobSet, which is
    // already compiled in CopFilterSet::is_globally_excluded().

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.context("error walking directory")?;
        let path = entry.path();
        if path.is_file() && is_ruby_file(path) {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

/// RuboCop-compatible Ruby file extensions (from AllCops.Include defaults).
const RUBY_EXTENSIONS: &[&str] = &[
    "rb", "arb", "axlsx", "builder", "fcgi", "gemfile", "gemspec", "god", "jb", "jbuilder",
    "mspec", "opal", "pluginspec", "podspec", "rabl", "rake", "rbuild", "rbw", "rbx", "ru",
    "ruby", "schema", "spec", "thor", "watchr",
];

/// Extensionless filenames that RuboCop treats as Ruby (from AllCops.Include defaults).
const RUBY_FILENAMES: &[&str] = &[
    ".irbrc",
    ".pryrc",
    ".simplecov",
    "buildfile",
    "Appraisals",
    "Berksfile",
    "Brewfile",
    "Buildfile",
    "Capfile",
    "Cheffile",
    "Dangerfile",
    "Deliverfile",
    "Fastfile",
    "Gemfile",
    "Guardfile",
    "Jarfile",
    "Mavenfile",
    "Podfile",
    "Puppetfile",
    "Rakefile",
    "rakefile",
    "Schemafile",
    "Snapfile",
    "Steepfile",
    "Thorfile",
    "Vagabondfile",
    "Vagrantfile",
];

fn is_ruby_file(path: &Path) -> bool {
    // Check by extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if RUBY_EXTENSIONS.iter().any(|&r| r.eq_ignore_ascii_case(ext)) {
            return true;
        }
    }
    // Check by filename (for extensionless Ruby files like Gemfile)
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if RUBY_FILENAMES.contains(&name) {
            return true;
        }
        // Also match *Fastfile pattern (e.g., Matchfile, Appfile that end in "Fastfile")
        if name.ends_with("Fastfile") || name.ends_with("fastfile") {
            return true;
        }
    }
    // For extensionless files not in the known list, check for Ruby shebang.
    // This catches scripts like bin/console, bin/rails, etc.
    if path.extension().is_none() {
        if has_ruby_shebang(path) {
            return true;
        }
    }
    false
}

/// Check if a file starts with a Ruby shebang line (e.g. `#!/usr/bin/env ruby`).
/// Only reads the first line to avoid expensive I/O during file discovery.
fn has_ruby_shebang(path: &Path) -> bool {
    use std::io::{BufRead, BufReader};
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line).is_err() {
        return false;
    }
    first_line.starts_with("#!") && first_line.contains("ruby")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_config;
    use std::fs;

    fn setup_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("turbocop_test_fs_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discovers_rb_files_in_directory() {
        let dir = setup_dir("discover");
        fs::write(dir.join("a.rb"), "").unwrap();
        fs::write(dir.join("b.rb"), "").unwrap();
        fs::write(dir.join("c.txt"), "").unwrap();

        let config = load_config(Some(Path::new("/nonexistent")), None, None).unwrap();
        let files = discover_files(&[dir.clone()], &config).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.extension().unwrap() == "rb"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn direct_file_bypasses_extension_filter() {
        let dir = setup_dir("direct");
        let txt = dir.join("script");
        fs::write(&txt, "puts 'hi'").unwrap();

        let config = load_config(Some(Path::new("/nonexistent")), None, None).unwrap();
        let files = discover_files(&[txt.clone()], &config).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], txt);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonexistent_path_errors() {
        let config = load_config(Some(Path::new("/nonexistent")), None, None).unwrap();
        let result = discover_files(&[PathBuf::from("/no/such/path")], &config);
        assert!(result.is_err());
    }

    #[test]
    fn results_are_sorted_and_deduped() {
        let dir = setup_dir("sorted");
        fs::write(dir.join("z.rb"), "").unwrap();
        fs::write(dir.join("a.rb"), "").unwrap();
        fs::write(dir.join("m.rb"), "").unwrap();

        let config = load_config(Some(Path::new("/nonexistent")), None, None).unwrap();
        let files = discover_files(&[dir.clone()], &config).unwrap();

        let names: Vec<_> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["a.rb", "m.rb", "z.rb"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_ruby_shebang_files() {
        let dir = setup_dir("shebang");
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(dir.join("app.rb"), "puts 'hi'").unwrap();
        fs::write(bin.join("console"), "#!/usr/bin/env ruby\nputs 'hi'\n").unwrap();
        fs::write(bin.join("setup"), "#!/bin/bash\necho hi\n").unwrap();
        fs::write(bin.join("server"), "#!/usr/bin/env ruby\nputs 'serve'\n").unwrap();

        let config = load_config(Some(Path::new("/nonexistent")), None, None).unwrap();
        let files = discover_files(&[dir.clone()], &config).unwrap();

        assert_eq!(files.len(), 3, "Should find app.rb + 2 ruby shebang scripts");
        let names: Vec<_> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"app.rb".to_string()));
        assert!(names.contains(&"console".to_string()));
        assert!(names.contains(&"server".to_string()));
        assert!(!names.contains(&"setup".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn debug_doorkeeper_bin_console() {
        use ignore::WalkBuilder;

        let doorkeeper_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("bench/repos/doorkeeper");
        if !doorkeeper_dir.exists() {
            eprintln!("Skipping: doorkeeper not cloned");
            return;
        }

        let bin_console = doorkeeper_dir.join("bin/console");
        assert!(bin_console.exists(), "bin/console must exist");
        assert!(has_ruby_shebang(&bin_console), "bin/console must have ruby shebang");
        assert!(is_ruby_file(&bin_console), "bin/console must be detected as ruby file");

        // Walk with same settings as walk_directory
        let mut builder = WalkBuilder::new(&doorkeeper_dir);
        builder.hidden(true).git_ignore(true).git_global(true);

        let mut found_bin_console = false;
        let mut all_bin_files = Vec::new();
        for entry in builder.build() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.starts_with(doorkeeper_dir.join("bin")) {
                all_bin_files.push(path.to_path_buf());
            }
            if path == bin_console {
                found_bin_console = true;
            }
        }
        eprintln!("All entries under bin/: {:?}", all_bin_files);
        eprintln!("Found bin/console: {}", found_bin_console);

        // Now try without git_global
        let mut builder2 = WalkBuilder::new(&doorkeeper_dir);
        builder2.hidden(true).git_ignore(true).git_global(false);

        let mut found_without_global = false;
        for entry in builder2.build() {
            let entry = entry.unwrap();
            if entry.path() == bin_console {
                found_without_global = true;
            }
        }
        eprintln!("Found bin/console without git_global: {}", found_without_global);

        // Try without git_ignore too
        let mut builder3 = WalkBuilder::new(&doorkeeper_dir);
        builder3.hidden(true).git_ignore(false).git_global(false);

        let mut found_without_gitignore = false;
        for entry in builder3.build() {
            let entry = entry.unwrap();
            if entry.path() == bin_console {
                found_without_gitignore = true;
            }
        }
        eprintln!("Found bin/console without any git ignoring: {}", found_without_gitignore);

        // Try without parents
        let mut builder4 = WalkBuilder::new(&doorkeeper_dir);
        builder4.hidden(true).git_ignore(true).git_global(true).parents(false);

        let mut found_without_parents = false;
        for entry in builder4.build() {
            let entry = entry.unwrap();
            if entry.path() == bin_console {
                found_without_parents = true;
            }
        }
        eprintln!("Found bin/console without parents: {}", found_without_parents);

        assert!(found_bin_console, "Walker must yield bin/console");
    }

    #[test]
    fn discovers_nested_rb_files() {
        let dir = setup_dir("nested");
        let sub = dir.join("lib");
        fs::create_dir_all(&sub).unwrap();
        fs::write(dir.join("top.rb"), "").unwrap();
        fs::write(sub.join("nested.rb"), "").unwrap();

        let config = load_config(Some(Path::new("/nonexistent")), None, None).unwrap();
        let files = discover_files(&[dir.clone()], &config).unwrap();

        assert_eq!(files.len(), 2);
        fs::remove_dir_all(&dir).ok();
    }
}
