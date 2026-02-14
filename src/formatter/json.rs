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
}

#[derive(Serialize)]
struct Offense {
    path: String,
    line: usize,
    column: usize,
    severity: String,
    cop_name: String,
    message: String,
}

impl Formatter for JsonFormatter {
    fn print(&self, diagnostics: &[Diagnostic], file_count: usize) {
        let output = JsonOutput {
            metadata: Metadata {
                files_inspected: file_count,
                offense_count: diagnostics.len(),
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
                })
                .collect(),
        };
        // Safe to unwrap: our types always serialize successfully
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }
}
