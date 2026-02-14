use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::overrides::OverrideBuilder;
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

    // Apply AllCops.Exclude patterns as overrides
    let global_excludes = config.global_excludes();
    if !global_excludes.is_empty() {
        let mut overrides = OverrideBuilder::new(dir);
        for pattern in global_excludes {
            // ignore crate overrides: prefix with ! to exclude
            overrides
                .add(&format!("!{pattern}"))
                .with_context(|| format!("invalid exclude pattern: {pattern}"))?;
        }
        let overrides = overrides.build().context("failed to build overrides")?;
        builder.overrides(overrides);
    }

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.context("error walking directory")?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rb") {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_config;
    use std::fs;

    fn setup_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rblint_test_fs_{name}"));
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

        let config = load_config(Some(Path::new("/nonexistent"))).unwrap();
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

        let config = load_config(Some(Path::new("/nonexistent"))).unwrap();
        let files = discover_files(&[txt.clone()], &config).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], txt);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonexistent_path_errors() {
        let config = load_config(Some(Path::new("/nonexistent"))).unwrap();
        let result = discover_files(&[PathBuf::from("/no/such/path")], &config);
        assert!(result.is_err());
    }

    #[test]
    fn results_are_sorted_and_deduped() {
        let dir = setup_dir("sorted");
        fs::write(dir.join("z.rb"), "").unwrap();
        fs::write(dir.join("a.rb"), "").unwrap();
        fs::write(dir.join("m.rb"), "").unwrap();

        let config = load_config(Some(Path::new("/nonexistent"))).unwrap();
        let files = discover_files(&[dir.clone()], &config).unwrap();

        let names: Vec<_> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["a.rb", "m.rb", "z.rb"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_nested_rb_files() {
        let dir = setup_dir("nested");
        let sub = dir.join("lib");
        fs::create_dir_all(&sub).unwrap();
        fs::write(dir.join("top.rb"), "").unwrap();
        fs::write(sub.join("nested.rb"), "").unwrap();

        let config = load_config(Some(Path::new("/nonexistent"))).unwrap();
        let files = discover_files(&[dir.clone()], &config).unwrap();

        assert_eq!(files.len(), 2);
        fs::remove_dir_all(&dir).ok();
    }
}
