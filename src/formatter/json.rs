use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;

use crate::diagnostic::Diagnostic;
use crate::formatter::Formatter;

pub struct JsonFormatter;

#[derive(Serialize)]
struct JsonOutput {
    metadata: Metadata,
    offenses: Vec<Offense>,
}

#[derive(Serialize)]
struct Metadata {
    files_inspected: usize,
    offense_count: usize,
    corrected_count: usize,
}

#[derive(Serialize)]
struct Offense {
    path: String,
    line: usize,
    column: usize,
    severity: String,
    cop_name: String,
    message: String,
    corrected: bool,
}

impl Formatter for JsonFormatter {
    fn format_to(&self, diagnostics: &[Diagnostic], files: &[PathBuf], out: &mut dyn Write) {
        let corrected_count = diagnostics.iter().filter(|d| d.corrected).count();
        let output = JsonOutput {
            metadata: Metadata {
                files_inspected: files.len(),
                offense_count: diagnostics.len(),
                corrected_count,
            },
            offenses: diagnostics
                .iter()
                .map(|d| Offense {
                    path: d.path.clone(),
                    line: d.location.line,
                    column: d.location.column,
                    severity: d.severity.letter().to_string(),
                    cop_name: d.cop_name.clone(),
                    message: d.message.clone(),
                    corrected: d.corrected,
                })
                .collect(),
        };
        // Safe to unwrap: our types always serialize successfully
        let _ = writeln!(out, "{}", serde_json::to_string_pretty(&output).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Location, Severity};

    fn render(diagnostics: &[Diagnostic], files: &[PathBuf]) -> String {
        let mut buf = Vec::new();
        JsonFormatter.format_to(diagnostics, files, &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn empty_produces_valid_json() {
        let out = render(&[], &[]);
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(parsed["metadata"]["files_inspected"], 0);
        assert_eq!(parsed["metadata"]["offense_count"], 0);
        assert_eq!(parsed["offenses"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn offense_fields_present() {
        let d = Diagnostic {
            path: "foo.rb".to_string(),
            location: Location { line: 3, column: 5 },
            severity: Severity::Warning,
            cop_name: "Style/Foo".to_string(),
            message: "bad".to_string(),

            corrected: false,
        };
        let out = render(&[d], &[PathBuf::from("foo.rb")]);
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(parsed["metadata"]["files_inspected"], 1);
        assert_eq!(parsed["metadata"]["offense_count"], 1);
        let offense = &parsed["offenses"][0];
        assert_eq!(offense["path"], "foo.rb");
        assert_eq!(offense["line"], 3);
        assert_eq!(offense["column"], 5);
        assert_eq!(offense["severity"], "W");
        assert_eq!(offense["cop_name"], "Style/Foo");
        assert_eq!(offense["message"], "bad");
    }
}
