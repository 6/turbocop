use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// An expected offense parsed from a fixture annotation.
#[derive(Debug, Clone)]
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
///
/// This intentionally rejects lines that merely contain `^` in other contexts
/// (e.g., Ruby XOR `x ^ y`, caret in strings) because:
/// - The `^` must be the first non-whitespace character
/// - Must be followed by ` Department/CopName: message` (with `/` and `: `)
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

    // Cop names must contain '/' (e.g., Layout/Foo, Style/Bar)
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
/// Annotation lines (lines starting with `^^^` markers after optional whitespace)
/// are stripped from the source. Line numbers in expected offenses refer to the
/// clean source (1-indexed).
///
/// # Convention
///
/// Annotations must appear *after* the source line they reference. The annotated
/// line number is the count of source lines seen so far (i.e., the previous
/// non-annotation line).
///
/// # Panics
///
/// Panics if an annotation appears before any source line, which would produce
/// an invalid line number of 0.
pub fn parse_fixture(raw: &[u8]) -> (Vec<u8>, Vec<ExpectedOffense>) {
    let text = std::str::from_utf8(raw).expect("fixture must be valid UTF-8");
    let elements: Vec<&str> = text.split('\n').collect();

    let mut source_lines: Vec<&str> = Vec::new();
    let mut expected: Vec<ExpectedOffense> = Vec::new();

    for (raw_idx, element) in elements.iter().enumerate() {
        if let Some(annotation) = try_parse_annotation(element) {
            assert!(
                !source_lines.is_empty(),
                "Annotation on raw line {} appears before any source line. \
                 Annotations must follow the source line they reference.\n\
                 Line: {:?}",
                raw_idx + 1,
                element,
            );
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

/// Run a cop on raw source bytes and return the diagnostics.
///
/// Use this for custom assertions where the standard `assert_cop_offenses`
/// helpers don't fit (e.g., checking severity, partial matching, or
/// testing cops that depend on raw byte layout like TrailingEmptyLines).
pub fn run_cop(cop: &dyn Cop, source_bytes: &[u8]) -> Vec<Diagnostic> {
    run_cop_with_config(cop, source_bytes, CopConfig::default())
}

/// Run a cop on raw source bytes with a specific config and return diagnostics.
pub fn run_cop_with_config(
    cop: &dyn Cop,
    source_bytes: &[u8],
    config: CopConfig,
) -> Vec<Diagnostic> {
    let source = SourceFile::from_bytes("test.rb", source_bytes.to_vec());
    cop.check_lines(&source, &config)
}

/// Run a cop on fixture bytes (with annotations) and assert offenses match.
pub fn assert_cop_offenses(cop: &dyn Cop, fixture_bytes: &[u8]) {
    assert_cop_offenses_with_config(cop, fixture_bytes, CopConfig::default());
}

/// Run a cop on fixture bytes with a specific config and assert offenses match.
///
/// Both expected and actual diagnostics are sorted by (line, column) before
/// comparison, so annotation order in the fixture doesn't need to match the
/// cop's emission order.
pub fn assert_cop_offenses_with_config(cop: &dyn Cop, fixture_bytes: &[u8], config: CopConfig) {
    let (clean_source, mut expected) = parse_fixture(fixture_bytes);
    let source = SourceFile::from_bytes("test.rb", clean_source);
    let mut diagnostics = cop.check_lines(&source, &config);

    // Sort both for order-independent comparison
    expected.sort_by_key(|e| (e.line, e.column));
    diagnostics.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    assert_eq!(
        diagnostics.len(),
        expected.len(),
        "Expected {} offense(s) but got {}.\nExpected:\n{}\nActual:\n{}",
        expected.len(),
        diagnostics.len(),
        format_expected(&expected),
        format_diagnostics(&diagnostics),
    );

    for (i, (diag, exp)) in diagnostics.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            diag.location.line, exp.line,
            "Offense #{}: line mismatch (expected {} got {})\n  expected: {}:{} {}: {}\n  actual:   {d}",
            i + 1, exp.line, diag.location.line,
            exp.line, exp.column, exp.cop_name, exp.message,
            d = diag,
        );
        assert_eq!(
            diag.location.column, exp.column,
            "Offense #{}: column mismatch (expected {} got {})\n  expected: {}:{} {}: {}\n  actual:   {d}",
            i + 1, exp.column, diag.location.column,
            exp.line, exp.column, exp.cop_name, exp.message,
            d = diag,
        );
        assert_eq!(
            diag.cop_name, exp.cop_name,
            "Offense #{}: cop name mismatch\n  expected: {}\n  actual:   {}",
            i + 1, exp.cop_name, diag.cop_name,
        );
        assert_eq!(
            diag.message, exp.message,
            "Offense #{}: message mismatch for {}\n  expected: {:?}\n  actual:   {:?}",
            i + 1, exp.cop_name, exp.message, diag.message,
        );
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
        format_diagnostics(&diagnostics),
    );
}

fn format_expected(expected: &[ExpectedOffense]) -> String {
    expected
        .iter()
        .map(|e| format!("  {}:{} {}: {}", e.line, e.column, e.cop_name, e.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| format!("  {d}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Annotation parser unit tests ----

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
    fn parse_annotation_single_caret() {
        let ann = try_parse_annotation("^ Layout/X: m").unwrap();
        assert_eq!(ann.column, 0);
        assert_eq!(ann.cop_name, "Layout/X");
        assert_eq!(ann.message, "m");
    }

    #[test]
    fn parse_annotation_many_carets() {
        let ann =
            try_parse_annotation("^^^^^^^^^^ Layout/LineLength: Line is too long. [130/120]")
                .unwrap();
        assert_eq!(ann.column, 0);
        assert_eq!(ann.message, "Line is too long. [130/120]");
    }

    #[test]
    fn parse_annotation_message_with_special_chars() {
        let ann = try_parse_annotation("^^^ Style/Foo: Use `bar` instead of 'baz'.").unwrap();
        assert_eq!(ann.message, "Use `bar` instead of 'baz'.");
    }

    // ---- False-positive rejection tests ----

    #[test]
    fn rejects_non_annotation_lines() {
        assert!(try_parse_annotation("x = 1").is_none());
        assert!(try_parse_annotation("# just a comment").is_none());
        assert!(try_parse_annotation("").is_none());
        assert!(try_parse_annotation("   ").is_none());
    }

    #[test]
    fn rejects_ruby_xor_operator() {
        // Ruby XOR: x ^ y — caret is NOT the first non-whitespace char
        assert!(try_parse_annotation("x ^ y").is_none());
        assert!(try_parse_annotation("result = a ^ b").is_none());
    }

    #[test]
    fn rejects_carets_without_cop_name() {
        // Must have Department/Name format
        assert!(try_parse_annotation("^^^ no slash here").is_none());
        assert!(try_parse_annotation("^^^ justtext").is_none());
    }

    #[test]
    fn rejects_carets_without_space_after() {
        assert!(try_parse_annotation("^^^Layout/Foo: msg").is_none());
    }

    #[test]
    fn rejects_carets_without_colon_space() {
        assert!(try_parse_annotation("^^^ Layout/Foo msg").is_none());
        assert!(try_parse_annotation("^^^ Layout/Foo:msg").is_none());
    }

    #[test]
    fn rejects_ruby_regex_with_carets() {
        // /^foo/ — not an annotation because it starts with /
        assert!(try_parse_annotation("/^foo/").is_none());
    }

    #[test]
    fn rejects_caret_in_string() {
        assert!(try_parse_annotation("  puts \"^hello\"").is_none());
    }

    // ---- parse_fixture tests ----

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
    fn parse_fixture_multiple_annotations_same_line() {
        let raw = b"line1\n^^^ A/B: m1\n  ^^^ C/D: m2\nline2\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"line1\nline2\n");
        assert_eq!(expected.len(), 2);
        // Both reference source line 1
        assert_eq!(expected[0].line, 1);
        assert_eq!(expected[0].column, 0);
        assert_eq!(expected[1].line, 1);
        assert_eq!(expected[1].column, 2);
    }

    #[test]
    fn parse_fixture_annotations_on_different_lines() {
        let raw = b"line1\n     ^^^ A/B: m1\nline2\n  ^^^ C/D: m2\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"line1\nline2\n");
        assert_eq!(expected.len(), 2);
        assert_eq!(expected[0].line, 1);
        assert_eq!(expected[1].line, 2);
    }

    #[test]
    fn parse_fixture_no_annotations() {
        let raw = b"x = 1\ny = 2\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"x = 1\ny = 2\n");
        assert!(expected.is_empty());
    }

    #[test]
    fn parse_fixture_no_trailing_newline() {
        let raw = b"x = 1\n     ^^^ A/B: m";
        let (clean, expected) = parse_fixture(raw);
        // Annotation is last, no trailing source line → no trailing newline
        assert_eq!(clean, b"x = 1");
        assert_eq!(expected.len(), 1);
        assert_eq!(expected[0].line, 1);
    }

    #[test]
    fn parse_fixture_preserves_trailing_whitespace_in_source() {
        // Trailing spaces on source line must be preserved in clean output
        let raw = b"x = 1   \n        ^^^ Layout/Foo: msg\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"x = 1   \n");
        assert_eq!(expected.len(), 1);
        assert_eq!(expected[0].column, 8);
    }

    #[test]
    fn parse_fixture_empty_source_lines_preserved() {
        // Empty lines in source (e.g., blank lines) must be kept
        let raw = b"\n^^^ A/B: m\nx = 1\n";
        let (clean, expected) = parse_fixture(raw);
        assert_eq!(clean, b"\nx = 1\n");
        assert_eq!(expected.len(), 1);
        assert_eq!(expected[0].line, 1); // the empty line
    }

    #[test]
    #[should_panic(expected = "Annotation on raw line 1 appears before any source line")]
    fn parse_fixture_annotation_before_source_panics() {
        let raw = b"^^^ A/B: should panic\nx = 1\n";
        parse_fixture(raw);
    }

    // ---- run_cop helper tests ----

    #[test]
    fn run_cop_returns_diagnostics() {
        use crate::cop::layout::trailing_whitespace::TrailingWhitespace;
        let diags = run_cop(&TrailingWhitespace, b"x = 1  \n");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
        assert_eq!(diags[0].cop_name, "Layout/TrailingWhitespace");
    }

    #[test]
    fn run_cop_with_config_applies_config() {
        use crate::cop::layout::line_length::LineLength;
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(10.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let diags = run_cop_with_config(&LineLength, b"short\nthis is longer\n", config);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn run_cop_no_offenses_returns_empty() {
        use crate::cop::layout::trailing_whitespace::TrailingWhitespace;
        let diags = run_cop(&TrailingWhitespace, b"x = 1\ny = 2\n");
        assert!(diags.is_empty());
    }

    // ---- assert helper tests ----

    #[test]
    fn assert_cop_offenses_with_config_works() {
        use crate::cop::layout::line_length::LineLength;
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(10.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        // "longer than ten" = 15 chars, exceeds Max:10, offense at column 10
        let fixture = b"short\nlonger than ten\n          ^^^^^ Layout/LineLength: Line is too long. [15/10]\n";
        assert_cop_offenses_with_config(&LineLength, fixture, config);
    }

    #[test]
    fn assert_cop_no_offenses_with_config_works() {
        use crate::cop::layout::line_length::LineLength;
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(200.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        assert_cop_no_offenses_with_config(&LineLength, b"short line\n", config);
    }
}
