use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::formatter::Formatter;

pub struct PacmanFormatter;

/// Pac-Man character
const PACMAN: char = '\u{15E7}'; // ᗧ
/// Ghost character (file with offenses)
const GHOST: char = '\u{15E3}'; // ᗣ
/// Pacdot (clean file)
const PACDOT: char = '\u{2022}'; // •

impl Formatter for PacmanFormatter {
    fn format_to(&self, diagnostics: &[Diagnostic], files: &[PathBuf], out: &mut dyn Write) {
        let file_count = files.len();

        // Collect files with offenses
        let offense_files: HashSet<&str> = diagnostics.iter().map(|d| d.path.as_str()).collect();

        // Build the pacman line
        let mut line = String::new();
        line.push(PACMAN);
        for f in files {
            let path_str = f.to_string_lossy();
            if offense_files.contains(path_str.as_ref()) {
                line.push(GHOST);
            } else {
                line.push(PACDOT);
            }
        }
        let _ = writeln!(out, "{line}");

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
        let _ = writeln!(
            out,
            "\n{file_count} {file_word} inspected, {} {offense_word} detected",
            diagnostics.len(),
        );
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
        }
    }

    fn render(diagnostics: &[Diagnostic], files: &[PathBuf]) -> String {
        let mut buf = Vec::new();
        PacmanFormatter.format_to(diagnostics, files, &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn all_clean_shows_pacdots() {
        let files = vec![PathBuf::from("a.rb"), PathBuf::from("b.rb")];
        let out = render(&[], &files);
        let first_line = out.lines().next().unwrap();
        // Pacman + 2 pacdots
        assert_eq!(first_line, format!("{PACMAN}{PACDOT}{PACDOT}"));
    }

    #[test]
    fn offense_file_shows_ghost() {
        let files = vec![PathBuf::from("a.rb"), PathBuf::from("b.rb"), PathBuf::from("c.rb")];
        let diags = vec![make_diag("b.rb")];
        let out = render(&diags, &files);
        let first_line = out.lines().next().unwrap();
        assert_eq!(first_line, format!("{PACMAN}{PACDOT}{GHOST}{PACDOT}"));
    }

    #[test]
    fn summary_line() {
        let files = vec![PathBuf::from("a.rb"), PathBuf::from("b.rb")];
        let diags = vec![make_diag("a.rb")];
        let out = render(&diags, &files);
        assert!(out.contains("2 files inspected, 1 offense detected"));
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
        };
        let out = render(&[d], &files);
        assert!(out.contains("foo.rb:5:3: W: Lint/Bad: bad thing"));
    }

    #[test]
    fn empty_files() {
        let out = render(&[], &[]);
        let first_line = out.lines().next().unwrap();
        // Just pacman, no dots
        assert_eq!(first_line, format!("{PACMAN}"));
    }
}
