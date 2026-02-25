use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ScriptPermission;

impl Cop for ScriptPermission {
    fn name(&self) -> &'static str {
        "Lint/ScriptPermission"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Only check files that start with a shebang
        let first_line = match source.lines().next() {
            Some(l) => l,
            None => return,
        };

        if !first_line.starts_with(b"#!") {
            return;
        }

        // Check actual file permissions (Unix-only)
        let path = source.path_str();

        // Skip stdin or synthetic paths (used in tests)
        if path == "test.rb" || path == "(stdin)" || path.is_empty() {
            return;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            match std::fs::metadata(path) {
                Ok(metadata) => {
                    let mode = metadata.permissions().mode();
                    // Check if any execute bit is set
                    if mode & 0o111 != 0 {
                        return; // Already executable
                    }
                }
                Err(_) => return, // Can't check, skip
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix platforms, skip this check
            return;
        }

        #[allow(unreachable_code)]
        {
            let basename = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            diagnostics.push(self.diagnostic(
                source,
                1,
                0,
                format!("Script file {basename} doesn't have execute permission."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_script(name: &str, content: &[u8], mode: u32) -> String {
        let path = format!("/tmp/nitrocop-test/{}", name);
        let dir = std::path::Path::new(&path).parent().unwrap();
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(&path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(&path, perms).unwrap();
        }
        let _ = mode;
        path
    }

    #[test]
    fn offense_fixture() {
        let path = make_temp_script(
            "test_script1.rb",
            b"#!/usr/bin/env ruby\nputs 'hello'\n",
            0o644,
        );
        let source = SourceFile::from_bytes(&path, std::fs::read(&path).unwrap());
        let config = CopConfig::default();
        let mut diags = Vec::new();
        ScriptPermission.check_lines(&source, &config, &mut diags, None);
        #[cfg(unix)]
        assert_eq!(diags.len(), 1, "Should flag non-executable script");
        #[cfg(not(unix))]
        assert!(diags.is_empty());
    }

    #[test]
    fn offense_fixture_2() {
        let path = make_temp_script("test_script2.rb", b"#!/usr/bin/ruby\nputs 'test'\n", 0o644);
        let source = SourceFile::from_bytes(&path, std::fs::read(&path).unwrap());
        let config = CopConfig::default();
        let mut diags = Vec::new();
        ScriptPermission.check_lines(&source, &config, &mut diags, None);
        #[cfg(unix)]
        assert_eq!(diags.len(), 1, "Should flag non-executable script");
        #[cfg(not(unix))]
        assert!(diags.is_empty());
    }

    #[test]
    fn offense_fixture_3() {
        let path = make_temp_script(
            "test_script3.rb",
            b"#!/usr/bin/env ruby\nclass Foo; end\n",
            0o600,
        );
        let source = SourceFile::from_bytes(&path, std::fs::read(&path).unwrap());
        let config = CopConfig::default();
        let mut diags = Vec::new();
        ScriptPermission.check_lines(&source, &config, &mut diags, None);
        #[cfg(unix)]
        assert_eq!(diags.len(), 1, "Should flag non-executable script");
        #[cfg(not(unix))]
        assert!(diags.is_empty());
    }

    #[test]
    fn no_offense_executable() {
        let path = make_temp_script(
            "exec_script.rb",
            b"#!/usr/bin/env ruby\nputs 'hello'\n",
            0o755,
        );
        let source = SourceFile::from_bytes(&path, std::fs::read(&path).unwrap());
        let config = CopConfig::default();
        let mut diags = Vec::new();
        ScriptPermission.check_lines(&source, &config, &mut diags, None);
        assert!(diags.is_empty(), "Should not flag executable script");
    }

    #[test]
    fn no_offense_no_shebang() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"puts 'hello'\nx = 1\ny = 2\nz = 3\na = 4\nb = 5\n".to_vec(),
        );
        let config = CopConfig::default();
        let mut diags = Vec::new();
        ScriptPermission.check_lines(&source, &config, &mut diags, None);
        assert!(diags.is_empty());
    }
}
