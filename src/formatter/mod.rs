pub mod json;
pub mod text;

use crate::diagnostic::Diagnostic;

pub trait Formatter {
    fn print(&self, diagnostics: &[Diagnostic], file_count: usize);
}

pub fn create_formatter(format: &str) -> Box<dyn Formatter> {
    match format {
        "json" => Box::new(json::JsonFormatter),
        _ => Box::new(text::TextFormatter),
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
        }]
    }

    #[test]
    fn create_text_formatter() {
        // Default and explicit "text" both return TextFormatter
        let _f = create_formatter("text");
        let _f = create_formatter("anything_else");
    }

    #[test]
    fn create_json_formatter() {
        let _f = create_formatter("json");
    }

    #[test]
    fn text_formatter_runs_without_panic() {
        let f = create_formatter("text");
        // Just verify it doesn't panic with empty and non-empty diagnostics
        f.print(&[], 0);
        f.print(&sample_diagnostics(), 1);
    }

    #[test]
    fn json_formatter_runs_without_panic() {
        let f = create_formatter("json");
        f.print(&[], 0);
        f.print(&sample_diagnostics(), 1);
    }
}
