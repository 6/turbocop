use std::io::Write;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::formatter::Formatter;

pub struct TextFormatter;

impl Formatter for TextFormatter {
    fn format_to(&self, diagnostics: &[Diagnostic], files: &[PathBuf], out: &mut dyn Write) {
        let file_count = files.len();
        for d in diagnostics {
            let _ = writeln!(out, "{d}");
        }
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

    fn make_diag(path: &str, line: usize, col: usize, sev: Severity, cop: &str, msg: &str) -> Diagnostic {
        Diagnostic {
            path: path.to_string(),
            location: Location { line, column: col },
            severity: sev,
            cop_name: cop.to_string(),
            message: msg.to_string(),

            corrected: false,
        }
    }

    fn render(diagnostics: &[Diagnostic], files: &[PathBuf]) -> String {
        let mut buf = Vec::new();
        TextFormatter.format_to(diagnostics, files, &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn empty_output() {
        let out = render(&[], &[]);
        assert_eq!(out, "\n0 files inspected, 0 offenses detected\n");
    }

    #[test]
    fn single_offense() {
        let d = make_diag("foo.rb", 3, 5, Severity::Convention, "Style/Foo", "bad style");
        let out = render(&[d], &[PathBuf::from("foo.rb")]);
        assert!(out.contains("foo.rb:3:5: C: Style/Foo: bad style"));
        assert!(out.contains("1 file inspected, 1 offense detected"));
    }

    #[test]
    fn multiple_offenses_pluralization() {
        let d1 = make_diag("a.rb", 1, 0, Severity::Convention, "X/Y", "m1");
        let d2 = make_diag("b.rb", 2, 0, Severity::Warning, "X/Z", "m2");
        let files = vec![PathBuf::from("a.rb"), PathBuf::from("b.rb")];
        let out = render(&[d1, d2], &files);
        assert!(out.contains("2 files inspected, 2 offenses detected"));
    }

    #[test]
    fn corrected_offense_shows_corrected_prefix() {
        let mut d = make_diag("foo.rb", 1, 5, Severity::Convention, "Style/Foo", "bad");
        d.corrected = true;
        let out = render(&[d], &[PathBuf::from("foo.rb")]);
        assert!(out.contains("[Corrected] foo.rb:1:5:"), "Expected [Corrected] prefix, got: {out}");
    }

    #[test]
    fn summary_includes_corrected_count() {
        let mut d1 = make_diag("a.rb", 1, 0, Severity::Convention, "X/Y", "m1");
        d1.corrected = true;
        let d2 = make_diag("a.rb", 2, 0, Severity::Convention, "X/Z", "m2");
        let files = vec![PathBuf::from("a.rb")];
        let out = render(&[d1, d2], &files);
        assert!(
            out.contains("2 offenses detected, 1 offense corrected"),
            "Expected corrected count in summary, got: {out}"
        );
    }
}
