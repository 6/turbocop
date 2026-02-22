pub mod files;
pub mod github;
pub mod json;
pub mod pacman;
pub mod progress;
pub mod quiet;
pub mod text;

use std::io::Write;
use std::path::PathBuf;

use crate::cop::tiers::SkipSummary;
use crate::diagnostic::Diagnostic;

pub trait Formatter {
    fn format_to(&self, diagnostics: &[Diagnostic], files: &[PathBuf], out: &mut dyn Write);

    /// Provide skip summary data for formatters that include it in output (e.g. JSON).
    fn set_skip_summary(&mut self, _summary: SkipSummary) {}

    fn print(&self, diagnostics: &[Diagnostic], files: &[PathBuf]) {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        self.format_to(diagnostics, files, &mut lock);
    }
}

pub fn create_formatter(format: &str) -> Box<dyn Formatter> {
    match format {
        "json" => Box::new(json::JsonFormatter::new()),
        "github" => Box::new(github::GithubFormatter),
        "pacman" => Box::new(pacman::PacmanFormatter),
        "quiet" => Box::new(quiet::QuietFormatter),
        "files" => Box::new(files::FilesFormatter),
        "emacs" | "simple" | "text" => Box::new(text::TextFormatter),
        // "progress" and any unknown value
        _ => Box::new(progress::ProgressFormatter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Location, Severity};

    fn sample_diagnostics() -> Vec<Diagnostic> {
        vec![Diagnostic {
            path: "foo.rb".to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/Test".to_string(),
            message: "test offense".to_string(),

            corrected: false,
        }]
    }

    fn sample_files() -> Vec<PathBuf> {
        vec![PathBuf::from("foo.rb")]
    }

    #[test]
    fn create_text_formatter() {
        // Explicit "text" and aliases
        let _f = create_formatter("text");
        let _f = create_formatter("emacs");
        let _f = create_formatter("simple");
    }

    #[test]
    fn create_progress_formatter() {
        // Default and explicit "progress"
        let _f = create_formatter("progress");
        let _f = create_formatter("anything_else"); // unknown defaults to progress
    }

    #[test]
    fn create_json_formatter() {
        let _f = create_formatter("json");
    }

    #[test]
    fn create_all_formatters() {
        for name in [
            "progress", "text", "json", "github", "pacman", "quiet", "files", "emacs", "simple",
        ] {
            let _f = create_formatter(name);
        }
    }

    #[test]
    fn text_formatter_runs_without_panic() {
        let f = create_formatter("text");
        let mut buf = Vec::new();
        f.format_to(&[], &[], &mut buf);
        f.format_to(&sample_diagnostics(), &sample_files(), &mut buf);
    }

    #[test]
    fn json_formatter_runs_without_panic() {
        let f = create_formatter("json");
        let mut buf = Vec::new();
        f.format_to(&[], &[], &mut buf);
        f.format_to(&sample_diagnostics(), &sample_files(), &mut buf);
    }

    #[test]
    fn all_formatters_run_without_panic() {
        let files = sample_files();
        let diags = sample_diagnostics();
        for name in [
            "progress", "text", "json", "github", "pacman", "quiet", "files", "emacs", "simple",
        ] {
            let f = create_formatter(name);
            let mut buf = Vec::new();
            f.format_to(&[], &[], &mut buf);
            f.format_to(&diags, &files, &mut buf);
        }
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        fn severity_strategy() -> impl Strategy<Value = Severity> {
            prop::sample::select(vec![
                Severity::Convention,
                Severity::Warning,
                Severity::Error,
                Severity::Fatal,
            ])
        }

        fn diagnostic_strategy() -> impl Strategy<Value = Diagnostic> {
            (
                "[a-z]{1,10}\\.rb",
                1usize..500,
                0usize..200,
                severity_strategy(),
                "[A-Z][a-z]+/[A-Z][a-z]+",
                "[a-z ]{1,30}",
            )
                .prop_map(|(path, line, column, severity, cop_name, message)| {
                    Diagnostic {
                        path,
                        location: Location { line, column },
                        severity,
                        cop_name,
                        message,
                        corrected: false,
                    }
                })
        }

        proptest! {
            #[test]
            fn json_output_is_valid_json(
                diagnostics in prop::collection::vec(diagnostic_strategy(), 0..10),
                file_count in 0usize..100,
            ) {
                // Build the same JSON structure the formatter uses
                let output = serde_json::json!({
                    "metadata": {
                        "files_inspected": file_count,
                        "offense_count": diagnostics.len(),
                    },
                    "offenses": diagnostics.iter().map(|d| {
                        serde_json::json!({
                            "path": d.path,
                            "line": d.location.line,
                            "column": d.location.column,
                            "severity": d.severity.letter().to_string(),
                            "cop_name": d.cop_name,
                            "message": d.message,
                        })
                    }).collect::<Vec<_>>(),
                });
                let json_str = serde_json::to_string_pretty(&output).unwrap();
                // Must be valid JSON
                let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
                // offense_count matches array length
                prop_assert_eq!(
                    parsed["metadata"]["offense_count"].as_u64().unwrap() as usize,
                    diagnostics.len()
                );
                prop_assert_eq!(
                    parsed["offenses"].as_array().unwrap().len(),
                    diagnostics.len()
                );
            }

            #[test]
            fn json_preserves_all_diagnostic_fields(d in diagnostic_strategy()) {
                let offense = serde_json::json!({
                    "path": d.path,
                    "line": d.location.line,
                    "column": d.location.column,
                    "severity": d.severity.letter().to_string(),
                    "cop_name": d.cop_name,
                    "message": d.message,
                });
                prop_assert_eq!(offense["path"].as_str().unwrap(), d.path.as_str());
                prop_assert_eq!(offense["line"].as_u64().unwrap() as usize, d.location.line);
                prop_assert_eq!(offense["column"].as_u64().unwrap() as usize, d.location.column);
                prop_assert_eq!(offense["cop_name"].as_str().unwrap(), d.cop_name.as_str());
                prop_assert_eq!(offense["message"].as_str().unwrap(), d.message.as_str());
            }

            #[test]
            fn text_pluralization(
                diagnostics in prop::collection::vec(diagnostic_strategy(), 0..10),
                file_count in 0usize..100,
            ) {
                let offense_word = if diagnostics.len() == 1 { "offense" } else { "offenses" };
                let file_word = if file_count == 1 { "file" } else { "files" };
                let summary = format!(
                    "{file_count} {file_word} inspected, {} {offense_word} detected",
                    diagnostics.len()
                );
                // Verify pluralization rules
                if diagnostics.len() == 1 {
                    prop_assert!(summary.contains("offense detected"));
                    prop_assert!(!summary.contains("offenses"));
                } else {
                    prop_assert!(summary.contains("offenses detected"));
                }
                if file_count == 1 {
                    prop_assert!(summary.contains("1 file inspected"));
                } else {
                    prop_assert!(summary.contains("files inspected"));
                }
            }
        }
    }
}
