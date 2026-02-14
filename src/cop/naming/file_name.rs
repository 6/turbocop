use std::path::Path;

use crate::cop::util::is_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FileName;

/// Well-known Ruby files that don't follow snake_case convention.
const ALLOWED_NAMES: &[&str] = &["Gemfile", "Rakefile", "Guardfile", "Capfile", "Berksfile"];

impl Cop for FileName {
    fn name(&self) -> &'static str {
        "Naming/FileName"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let path = Path::new(source.path_str());
        let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Allow well-known Ruby files
        if ALLOWED_NAMES.contains(&file_stem) {
            return Vec::new();
        }

        // Also allow if the full filename (no extension) is in the allowed list
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if ALLOWED_NAMES.contains(&file_name) {
            return Vec::new();
        }

        if is_snake_case(file_stem.as_bytes()) {
            return Vec::new();
        }

        vec![self.diagnostic(
            source,
            1,
            0,
            format!(
                "The name of this source file (`{file_stem}`) should use snake_case."
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_scenario_fixture_tests!(
        FileName, "cops/naming/file_name",
        camel_case = "camel_case.rb",
        bad_name = "bad_name.rb",
        with_dash = "with_dash.rb",
    );

    #[test]
    fn offense_bad_filename() {
        let source = SourceFile::from_bytes("BadFile.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].cop_name, "Naming/FileName");
        assert!(diags[0].message.contains("BadFile"));
    }

    #[test]
    fn offense_camel_case_filename() {
        let source = SourceFile::from_bytes("MyClass.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn no_offense_good_filename() {
        let source = SourceFile::from_bytes("good_file.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_gemfile() {
        let source = SourceFile::from_bytes("Gemfile", b"source 'https://rubygems.org'\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_rakefile() {
        let source = SourceFile::from_bytes("Rakefile", b"task :default\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_test_rb() {
        // The standard fixture test path
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let diags = FileName.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }
}
