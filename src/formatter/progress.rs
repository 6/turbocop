use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use crate::diagnostic::{Diagnostic, Severity};
use crate::formatter::Formatter;

pub struct ProgressFormatter;

impl Formatter for ProgressFormatter {
    fn format_to(&self, diagnostics: &[Diagnostic], files: &[PathBuf], out: &mut dyn Write) {
        let file_count = files.len();

        // Build map of file path -> worst severity
        let mut worst_by_file: HashMap<&str, Severity> = HashMap::new();
        for d in diagnostics {
            worst_by_file
                .entry(&d.path)
                .and_modify(|s| {
                    if d.severity > *s {
                        *s = d.severity;
                    }
                })
                .or_insert(d.severity);
        }

        // Print progress line: one char per file
        let progress: String = files
            .iter()
            .map(|f| {
                let path_str = f.to_string_lossy();
                match worst_by_file.get(path_str.as_ref()) {
                    Some(severity) => severity.letter(),
                    None => '.',
                }
            })
            .collect();
        let _ = writeln!(out, "{progress}");

        // Print offense details
        for d in diagnostics {
            let _ = writeln!(out, "{d}");
        }

        // Summary
        let offense_word = if diagnostics.len() == 1 {
            "offense"
        } else {
            "offenses"
        };
        let file_word = if file_count == 1 { "file" } else { "files" };
        let corrected_count = diagnostics.iter().filter(|d| d.corrected).count();
        if corrected_count > 0 {
            let corrected_word = if corrected_count == 1 { "offense" } else { "offenses" };
            let _ = writeln!(
                out,
                "\n{file_count} {file_word} inspected, {} {offense_word} detected, {corrected_count} {corrected_word} corrected",
                diagnostics.len(),
            );
        } else {
            let _ = writeln!(
                out,
                "\n{file_count} {file_word} inspected, {} {offense_word} detected",
                diagnostics.len(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Location, Severity};

    fn make_diag(path: &str, sev: Severity) -> Diagnostic {
        Diagnostic {
            path: path.to_string(),
            location: Location { line: 1, column: 0 },
            severity: sev,
            cop_name: "Style/Test".to_string(),
            message: "test".to_string(),

            corrected: false,
        }
    }

    fn render(diagnostics: &[Diagnostic], files: &[PathBuf]) -> String {
        let mut buf = Vec::new();
        ProgressFormatter.format_to(diagnostics, files, &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn all_clean_files_show_dots() {
        let files = vec![PathBuf::from("a.rb"), PathBuf::from("b.rb"), PathBuf::from("c.rb")];
        let out = render(&[], &files);
        assert!(out.starts_with("...\n"));
        assert!(out.contains("3 files inspected, 0 offenses detected"));
    }

    #[test]
    fn offense_file_shows_severity_letter() {
        let files = vec![PathBuf::from("a.rb"), PathBuf::from("b.rb"), PathBuf::from("c.rb")];
        let diags = vec![make_diag("b.rb", Severity::Convention)];
        let out = render(&diags, &files);
        assert!(out.starts_with(".C.\n"));
    }

    #[test]
    fn worst_severity_wins() {
        let files = vec![PathBuf::from("a.rb")];
        let diags = vec![
            make_diag("a.rb", Severity::Convention),
            make_diag("a.rb", Severity::Error),
        ];
        let out = render(&diags, &files);
        assert!(out.starts_with("E\n"));
    }

    #[test]
    fn mixed_files() {
        let files = vec![
            PathBuf::from("a.rb"),
            PathBuf::from("b.rb"),
            PathBuf::from("c.rb"),
            PathBuf::from("d.rb"),
        ];
        let diags = vec![
            make_diag("a.rb", Severity::Convention),
            make_diag("c.rb", Severity::Warning),
        ];
        let out = render(&diags, &files);
        assert!(out.starts_with("C.W.\n"));
        assert!(out.contains("4 files inspected, 2 offenses detected"));
    }

    #[test]
    fn offense_details_included() {
        let files = vec![PathBuf::from("foo.rb")];
        let d = Diagnostic {
            path: "foo.rb".to_string(),
            location: Location { line: 5, column: 3 },
            severity: Severity::Warning,
            cop_name: "Lint/Bad".to_string(),
            message: "bad thing".to_string(),

            corrected: false,
        };
        let out = render(&[d], &files);
        assert!(out.contains("foo.rb:5:3: W: Lint/Bad: bad thing"));
    }

    #[test]
    fn empty_files() {
        let out = render(&[], &[]);
        assert!(out.starts_with("\n")); // empty progress line
        assert!(out.contains("0 files inspected, 0 offenses detected"));
    }
}
