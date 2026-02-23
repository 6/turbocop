use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;

use crate::cop::tiers::SkipSummary;
use crate::diagnostic::Diagnostic;
use crate::formatter::Formatter;

pub struct JsonFormatter {
    skip_summary: Option<SkipSummary>,
}

impl JsonFormatter {
    // Default impl not useful; formatter is always explicitly constructed.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { skip_summary: None }
    }
}

#[derive(Serialize)]
struct JsonOutput {
    metadata: Metadata,
    offenses: Vec<Offense>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<SkippedOutput>,
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

#[derive(Serialize)]
struct SkippedOutput {
    preview_gated: Vec<String>,
    unimplemented: Vec<String>,
    outside_baseline: Vec<String>,
    total: usize,
}

impl Formatter for JsonFormatter {
    fn set_skip_summary(&mut self, summary: SkipSummary) {
        self.skip_summary = Some(summary);
    }

    fn format_to(&self, diagnostics: &[Diagnostic], files: &[PathBuf], out: &mut dyn Write) {
        let corrected_count = diagnostics.iter().filter(|d| d.corrected).count();

        let skipped = self.skip_summary.as_ref().map(|s| SkippedOutput {
            total: s.total(),
            preview_gated: s.preview_gated.clone(),
            unimplemented: s.unimplemented.clone(),
            outside_baseline: s.outside_baseline.clone(),
        });

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
            skipped,
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
        JsonFormatter::new().format_to(diagnostics, files, &mut buf);
        String::from_utf8(buf).unwrap()
    }

    fn render_with_skips(
        diagnostics: &[Diagnostic],
        files: &[PathBuf],
        summary: SkipSummary,
    ) -> String {
        let mut f = JsonFormatter::new();
        f.set_skip_summary(summary);
        let mut buf = Vec::new();
        f.format_to(diagnostics, files, &mut buf);
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
    fn no_skipped_field_without_summary() {
        let out = render(&[], &[]);
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert!(parsed.get("skipped").is_none());
    }

    #[test]
    fn skipped_field_present_with_summary() {
        let summary = SkipSummary {
            preview_gated: vec!["Rails/Pluck".into()],
            unimplemented: vec!["Custom/Foo".into(), "Custom/Bar".into()],
            outside_baseline: vec!["Unknown/Baz".into()],
        };
        let out = render_with_skips(&[], &[], summary);
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        let skipped = &parsed["skipped"];
        assert_eq!(skipped["total"], 4);
        assert_eq!(skipped["preview_gated"].as_array().unwrap().len(), 1);
        assert_eq!(skipped["unimplemented"].as_array().unwrap().len(), 2);
        assert_eq!(skipped["outside_baseline"].as_array().unwrap().len(), 1);
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

    #[test]
    fn corrected_field_serialized() {
        let d1 = Diagnostic {
            path: "a.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/Foo".to_string(),
            message: "fixed".to_string(),
            corrected: true,
        };
        let d2 = Diagnostic {
            path: "a.rb".to_string(),
            location: Location { line: 2, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/Bar".to_string(),
            message: "not fixed".to_string(),
            corrected: false,
        };
        let out = render(&[d1, d2], &[PathBuf::from("a.rb")]);
        let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(parsed["metadata"]["corrected_count"], 1);
        assert_eq!(parsed["offenses"][0]["corrected"], true);
        assert_eq!(parsed["offenses"][1]["corrected"], false);
    }
}
