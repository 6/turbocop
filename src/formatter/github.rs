use std::io::Write;
use std::path::PathBuf;

use crate::diagnostic::{Diagnostic, Severity};
use crate::formatter::Formatter;

pub struct GithubFormatter;

impl Formatter for GithubFormatter {
    fn format_to(&self, diagnostics: &[Diagnostic], _files: &[PathBuf], out: &mut dyn Write) {
        for d in diagnostics {
            let level = match d.severity {
                Severity::Convention | Severity::Warning => "warning",
                Severity::Error | Severity::Fatal => "error",
            };
            let _ = writeln!(
                out,
                "::{level} file={},line={},col={}::{}: {}",
                d.path, d.location.line, d.location.column, d.cop_name, d.message,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Location, Severity};

    fn render(diagnostics: &[Diagnostic]) -> String {
        let mut buf = Vec::new();
        GithubFormatter.format_to(diagnostics, &[], &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn empty_produces_no_output() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn convention_uses_warning_level() {
        let d = Diagnostic {
            path: "foo.rb".to_string(),
            location: Location { line: 3, column: 5 },
            severity: Severity::Convention,
            cop_name: "Style/Foo".to_string(),
            message: "bad style".to_string(),

            corrected: false,
        };
        let out = render(&[d]);
        assert_eq!(
            out,
            "::warning file=foo.rb,line=3,col=5::Style/Foo: bad style\n"
        );
    }

    #[test]
    fn warning_uses_warning_level() {
        let d = Diagnostic {
            path: "bar.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Warning,
            cop_name: "Lint/X".to_string(),
            message: "warn".to_string(),

            corrected: false,
        };
        let out = render(&[d]);
        assert!(out.starts_with("::warning "));
    }

    #[test]
    fn error_uses_error_level() {
        let d = Diagnostic {
            path: "baz.rb".to_string(),
            location: Location {
                line: 10,
                column: 2,
            },
            severity: Severity::Error,
            cop_name: "Lint/Y".to_string(),
            message: "err".to_string(),

            corrected: false,
        };
        let out = render(&[d]);
        assert_eq!(out, "::error file=baz.rb,line=10,col=2::Lint/Y: err\n");
    }

    #[test]
    fn fatal_uses_error_level() {
        let d = Diagnostic {
            path: "x.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Fatal,
            cop_name: "Lint/Z".to_string(),
            message: "fatal".to_string(),

            corrected: false,
        };
        let out = render(&[d]);
        assert!(out.starts_with("::error "));
    }

    #[test]
    fn multiple_offenses() {
        let d1 = Diagnostic {
            path: "a.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "A/B".to_string(),
            message: "m1".to_string(),

            corrected: false,
        };
        let d2 = Diagnostic {
            path: "b.rb".to_string(),
            location: Location { line: 2, column: 1 },
            severity: Severity::Error,
            cop_name: "C/D".to_string(),
            message: "m2".to_string(),

            corrected: false,
        };
        let out = render(&[d1, d2]);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("::warning"));
        assert!(lines[1].starts_with("::error"));
    }
}
