use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::formatter::Formatter;

pub struct FilesFormatter;

impl Formatter for FilesFormatter {
    fn format_to(&self, diagnostics: &[Diagnostic], _files: &[PathBuf], out: &mut dyn Write) {
        // Deduplicate and sort file paths
        let paths: BTreeSet<&str> = diagnostics.iter().map(|d| d.path.as_str()).collect();
        for path in paths {
            let _ = writeln!(out, "{path}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Location, Severity};

    fn make_diag(path: &str) -> Diagnostic {
        Diagnostic {
            path: path.to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/Test".to_string(),
            message: "test".to_string(),

            corrected: false,
        }
    }

    fn render(diagnostics: &[Diagnostic]) -> String {
        let mut buf = Vec::new();
        FilesFormatter.format_to(diagnostics, &[], &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn empty_produces_no_output() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn single_file() {
        let out = render(&[make_diag("foo.rb")]);
        assert_eq!(out, "foo.rb\n");
    }

    #[test]
    fn deduplicates() {
        let out = render(&[make_diag("foo.rb"), make_diag("foo.rb")]);
        assert_eq!(out, "foo.rb\n");
    }

    #[test]
    fn sorts_alphabetically() {
        let out = render(&[make_diag("c.rb"), make_diag("a.rb"), make_diag("b.rb")]);
        assert_eq!(out, "a.rb\nb.rb\nc.rb\n");
    }

    #[test]
    fn no_summary_line() {
        let out = render(&[make_diag("foo.rb")]);
        assert!(!out.contains("inspected"));
        assert!(!out.contains("offense"));
    }
}
