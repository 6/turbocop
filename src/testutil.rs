use crate::cop::{Cop, CopConfig};
use crate::parse::source::SourceFile;

/// An expected offense parsed from a fixture annotation.
#[derive(Debug)]
pub struct ExpectedOffense {
    pub line: usize,
    pub column: usize,
    pub cop_name: String,
    pub message: String,
}

struct RawAnnotation {
    column: usize,
    cop_name: String,
    message: String,
}

/// Try to parse an annotation line.
///
/// Annotation format: optional leading whitespace, then one or more `^` characters,
/// then a space, then `Department/CopName: Message`.
///
/// The column of the offense is the byte position of the first `^` in the line.
fn try_parse_annotation(line: &str) -> Option<RawAnnotation> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('^') {
        return None;
    }

    let caret_count = trimmed.bytes().take_while(|&b| b == b'^').count();
    let after_carets = &trimmed[caret_count..];
    if !after_carets.starts_with(' ') {
        return None;
    }

    let rest = after_carets[1..].trim_end();
    let colon_space = rest.find(": ")?;
    let cop_name = &rest[..colon_space];
    let message = &rest[colon_space + 2..];

    // Cop names must contain '/'
    if !cop_name.contains('/') {
        return None;
    }

    // Column = byte position of first '^' in the original line
    let column = line.len() - trimmed.len();

    Some(RawAnnotation {
        column,
        cop_name: cop_name.to_string(),
        message: message.to_string(),
    })
}

/// Parse fixture content into clean source bytes and expected offenses.
///
/// Annotation lines (lines containing `^^^` markers) are stripped from the source.
/// Line numbers in expected offenses refer to the clean source (1-indexed).
pub fn parse_fixture(raw: &[u8]) -> (Vec<u8>, Vec<ExpectedOffense>) {
    let text = std::str::from_utf8(raw).expect("fixture must be valid UTF-8");
    let elements: Vec<&str> = text.split('\n').collect();

    let mut source_lines: Vec<&str> = Vec::new();
    let mut expected: Vec<ExpectedOffense> = Vec::new();

    for element in &elements {
        if let Some(annotation) = try_parse_annotation(element) {
            // Annotation refers to the last source line added
            let source_line_number = source_lines.len(); // 1-indexed
            expected.push(ExpectedOffense {
                line: source_line_number,
                column: annotation.column,
                cop_name: annotation.cop_name,
                message: annotation.message,
            });
        } else {
            source_lines.push(element);
        }
    }

    let clean = source_lines.join("\n");
    (clean.into_bytes(), expected)
}

/// Run a cop on fixture bytes (with annotations) and assert offenses match.
pub fn assert_cop_offenses(cop: &dyn Cop, fixture_bytes: &[u8]) {
    assert_cop_offenses_with_config(cop, fixture_bytes, CopConfig::default());
}

/// Run a cop on fixture bytes with a specific config and assert offenses match.
pub fn assert_cop_offenses_with_config(cop: &dyn Cop, fixture_bytes: &[u8], config: CopConfig) {
    let (clean_source, expected) = parse_fixture(fixture_bytes);
    let source = SourceFile::from_bytes("test.rb", clean_source);
    let diagnostics = cop.check_lines(&source, &config);

    assert_eq!(
        diagnostics.len(),
        expected.len(),
        "Expected {} offense(s) but got {}.\nExpected:\n{}\nActual:\n{}",
        expected.len(),
        diagnostics.len(),
        expected
            .iter()
            .map(|e| format!("  {}:{} {}: {}", e.line, e.column, e.cop_name, e.message))
            .collect::<Vec<_>>()
            .join("\n"),
        diagnostics
            .iter()
            .map(|d| format!("  {d}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    for (diag, exp) in diagnostics.iter().zip(expected.iter()) {
        assert_eq!(
            diag.location.line, exp.line,
            "Line mismatch: expected {} got {} for {}",
            exp.line, diag.location.line, exp.cop_name
        );
        assert_eq!(
            diag.location.column, exp.column,
            "Column mismatch: expected {} got {} for {} at line {}",
            exp.column, diag.location.column, exp.cop_name, exp.line
        );
        assert_eq!(diag.cop_name, exp.cop_name, "Cop name mismatch");
        assert_eq!(diag.message, exp.message, "Message mismatch for {}", exp.cop_name);
    }
}

/// Assert a cop produces no offenses on the given source bytes.
pub fn assert_cop_no_offenses(cop: &dyn Cop, source_bytes: &[u8]) {
    assert_cop_no_offenses_with_config(cop, source_bytes, CopConfig::default());
}

/// Assert a cop produces no offenses on the given source bytes with a specific config.
pub fn assert_cop_no_offenses_with_config(cop: &dyn Cop, source_bytes: &[u8], config: CopConfig) {
    let source = SourceFile::from_bytes("test.rb", source_bytes.to_vec());
    let diagnostics = cop.check_lines(&source, &config);

    assert!(
        diagnostics.is_empty(),
        "Expected no offenses but got {}:\n{}",
        diagnostics.len(),
        diagnostics
            .iter()
            .map(|d| format!("  {d}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_annotation_with_carets() {
        let ann = try_parse_annotation("     ^^^ Layout/Foo: some message").unwrap();
        assert_eq!(ann.column, 5);
        assert_eq!(ann.cop_name, "Layout/Foo");
        assert_eq!(ann.message, "some message");
    }

    #[test]
    fn parse_annotation_at_column_zero() {
        let ann = try_parse_annotation("^^^ Style/Bar: msg").unwrap();
        assert_eq!(ann.column, 0);
        assert_eq!(ann.cop_name, "Style/Bar");
        assert_eq!(ann.message, "msg");
    }

    #[test]
    fn non_annotation_lines() {
        assert!(try_parse_annotation("x = 1").is_none());
        assert!(try_parse_annotation("# just a comment").is_none());
        assert!(try_parse_annotation("").is_none());
        // Carets without cop name
        assert!(try_parse_annotation("^^^ no slash here").is_none());
    }

    #[test]
    fn parse_fixture_strips_annotations() {
        let raw = b"x = 1\n     ^^^ Layout/Foo: msg\ny = 2\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"x = 1\ny = 2\n");
        assert_eq!(expected.len(), 1);
        assert_eq!(expected[0].line, 1);
        assert_eq!(expected[0].column, 5);
        assert_eq!(expected[0].cop_name, "Layout/Foo");
        assert_eq!(expected[0].message, "msg");
    }

    #[test]
    fn parse_fixture_multiple_annotations() {
        let raw = b"line1\n^^^ A/B: m1\n  ^^^ C/D: m2\nline2\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"line1\nline2\n");
        assert_eq!(expected.len(), 2);
        assert_eq!(expected[0].line, 1);
        assert_eq!(expected[0].column, 0);
        assert_eq!(expected[1].line, 1);
        assert_eq!(expected[1].column, 2);
    }

    #[test]
    fn parse_fixture_no_annotations() {
        let raw = b"x = 1\ny = 2\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"x = 1\ny = 2\n");
        assert!(expected.is_empty());
    }
}
