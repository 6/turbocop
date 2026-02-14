use crate::diagnostic::Diagnostic;
use crate::formatter::Formatter;

pub struct TextFormatter;

impl Formatter for TextFormatter {
    fn print(&self, diagnostics: &[Diagnostic], file_count: usize) {
        for d in diagnostics {
            println!("{d}");
        }
        let offense_word = if diagnostics.len() == 1 {
            "offense"
        } else {
            "offenses"
        };
        let file_word = if file_count == 1 { "file" } else { "files" };
        println!(
            "\n{file_count} {file_word} inspected, {} {offense_word} detected",
            diagnostics.len(),
        );
    }
}
