use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LineLength;

impl Cop for LineLength {
    fn name(&self) -> &'static str {
        "Layout/LineLength"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let max = config.get_usize("Max", 120);
        let allow_heredoc = config.get_bool("AllowHeredoc", true);
        let allow_uri = config.get_bool("AllowURI", true);
        let allow_qualified_name = config.get_bool("AllowQualifiedName", true);
        let uri_schemes = config.get_string_array("URISchemes")
            .unwrap_or_else(|| vec!["http".into(), "https".into()]);
        let allow_rbs = config.get_bool("AllowRBSInlineAnnotation", false);
        let allow_cop_directives = config.get_bool("AllowCopDirectives", true);
        let allowed_patterns = config.get_string_array("AllowedPatterns").unwrap_or_default();
        // SplitStrings is an autocorrection-only option; read it for config_audit compliance
        // but it has no effect on offense detection (only on how offenses are auto-fixed).
        let _split_strings = config.get_bool("SplitStrings", false);
        // Pre-compile allowed patterns (may include Ruby regex syntax like /pattern/)
        let compiled_patterns: Vec<regex::Regex> = allowed_patterns.iter()
            .filter_map(|p| {
                let pattern = normalize_ruby_regex(p);
                regex::Regex::new(&pattern).ok()
            })
            .collect();

        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();
        let mut heredoc_terminator: Option<Vec<u8>> = None;

        for (i, line) in lines.iter().enumerate() {
            // Track heredoc regions
            if let Some(ref terminator) = heredoc_terminator {
                let trimmed: Vec<u8> = line.iter().copied()
                    .skip_while(|&b| b == b' ' || b == b'\t')
                    .collect();
                if trimmed == *terminator || trimmed.strip_suffix(&[b'\r']).unwrap_or(&trimmed) == terminator.as_slice() {
                    heredoc_terminator = None;
                } else if allow_heredoc {
                    continue; // Skip length check inside heredoc
                }
            }

            // Detect heredoc openers — scan all `<<` occurrences since `<<` can
            // also be the shovel operator (e.g. `file << <<~HEREDOC`).
            if heredoc_terminator.is_none() {
                let mut search_from = 0;
                while let Some(rel_pos) = line[search_from..].windows(2).position(|w| w == b"<<") {
                    let pos = search_from + rel_pos;
                    search_from = pos + 2;
                    let after = &line[pos + 2..];
                    let after = if after.starts_with(b"~") || after.starts_with(b"-") {
                        &after[1..]
                    } else {
                        after
                    };
                    let (after, _) = if after.starts_with(b"'") || after.starts_with(b"\"") {
                        let quote = after[0];
                        if let Some(end) = after[1..].iter().position(|&b| b == quote) {
                            (&after[1..1 + end], true)
                        } else {
                            (after, false)
                        }
                    } else {
                        (after, false)
                    };
                    let ident: Vec<u8> = after.iter().copied()
                        .take_while(|&b| b.is_ascii_alphanumeric() || b == b'_')
                        .collect();
                    if !ident.is_empty() {
                        heredoc_terminator = Some(ident);
                        break;
                    }
                }
            }

            // RuboCop measures line length in characters, not bytes.
            // For multi-byte UTF-8 (e.g. accented chars), byte length > char length.
            let char_len = match std::str::from_utf8(line) {
                Ok(s) => s.chars().count(),
                Err(_) => line.len(), // fallback to bytes for invalid UTF-8
            };

            if char_len <= max {
                continue;
            }

            // AllowCopDirectives: skip lines that are only long because of a rubocop directive comment
            if allow_cop_directives {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    if let Some(comment_start) = line_str.find("# rubocop:") {
                        let without_directive_chars = line_str[..comment_start].trim_end().chars().count();
                        if without_directive_chars <= max {
                            continue;
                        }
                    }
                }
            }

            // AllowRBSInlineAnnotation: skip lines with RBS type annotation comments (#: ...)
            if allow_rbs {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    if let Some(comment_start) = line_str.find("#:") {
                        // Check that #: is actually an RBS annotation (preceded by space or at start)
                        let is_rbs = comment_start == 0
                            || line_str.as_bytes()[comment_start - 1] == b' '
                            || line_str.as_bytes()[comment_start - 1] == b'\t';
                        if is_rbs {
                            let without_rbs_chars = line_str[..comment_start].trim_end().chars().count();
                            if without_rbs_chars <= max {
                                continue;
                            }
                        }
                    }
                }
            }

            // AllowURI: skip lines containing a URI that makes them long.
            // Matches RuboCop's `allowed_position?` logic: the URI (after extension)
            // must start before `max` AND extend to the end of the line.
            if allow_uri {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    if uri_extends_to_end(line_str, &uri_schemes, max) {
                        continue;
                    }
                }
            }

            // AllowedPatterns: skip lines matching any pattern
            if !compiled_patterns.is_empty() {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    if compiled_patterns.iter().any(|re| re.is_match(line_str)) {
                        continue;
                    }
                }
            }

            // AllowQualifiedName: skip lines where a qualified name (Foo::Bar::Baz)
            // makes the line too long. Works like AllowURI: the qualified name must
            // start before max AND extend to the end of the line (after extending).
            if allow_qualified_name {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    if qualified_name_extends_to_end(line_str, max) {
                        continue;
                    }
                }
            }

            diagnostics.push(self.diagnostic(
                source,
                i + 1,
                max,
                format!("Line is too long. [{}/{}]", char_len, max),
            ));
        }
        diagnostics
    }
}

/// Check if the last qualified name match in the line extends to the end of the line
/// AND starts before `max`. This matches RuboCop's `allowed_position?` logic for
/// qualified names (e.g. `Foo::Bar::Baz`).
fn qualified_name_extends_to_end(line: &str, max: usize) -> bool {
    // Match qualified names: one or more uppercase-starting segments joined by ::
    // Pattern from RuboCop: /\b(?:[A-Z][A-Za-z0-9_]*::)+[A-Za-z_][A-Za-z0-9_]*\b/
    // Find the last occurrence
    let mut last_match: Option<(usize, usize)> = None; // (start, end) byte positions

    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for a word boundary followed by an uppercase letter
        let at_word_boundary = i == 0 || !is_word_char(bytes[i - 1]);
        if at_word_boundary && i < len && bytes[i].is_ascii_uppercase() {
            // Try to match a qualified name starting here
            if let Some(end) = match_qualified_name(bytes, i) {
                // Verify there's at least one :: (it's actually a qualified name)
                let segment = &bytes[i..end];
                if segment.windows(2).any(|w| w == b"::") {
                    last_match = Some((i, end));
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }

    let (start, end) = match last_match {
        Some(m) => m,
        None => return false,
    };

    // Extend end position (matching RuboCop's extend_end_position):
    // 1. YARD brace extension
    let mut end_pos = end;
    if line.contains('{') && line.ends_with('}') {
        if let Some(brace_pos) = line[end_pos..].rfind('}') {
            end_pos += brace_pos + 1;
        }
    }
    // 2. Extend to next word boundary
    let rest = &line[end_pos..];
    let non_ws_len = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    end_pos += non_ws_len;

    // Check allowed_position?: start_chars < max AND end_pos reaches end of line
    let start_chars = line[..start].chars().count();
    start_chars < max && end_pos >= line.len()
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Match a qualified name starting at position `start` in the byte slice.
/// Returns the end position if a valid qualified name is found, or None.
/// A qualified name is: UpperIdent(::Ident)+ where each segment starts with uppercase or underscore.
fn match_qualified_name(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    let len = bytes.len();

    // First segment: [A-Z][A-Za-z0-9_]*
    if pos >= len || !bytes[pos].is_ascii_uppercase() {
        return None;
    }
    pos += 1;
    while pos < len && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
        pos += 1;
    }

    let mut has_double_colon = false;

    // Subsequent segments: ::[A-Za-z_][A-Za-z0-9_]*
    loop {
        if pos + 1 < len && bytes[pos] == b':' && bytes[pos + 1] == b':' {
            pos += 2; // skip ::
            if pos >= len || !(bytes[pos].is_ascii_alphabetic() || bytes[pos] == b'_') {
                // :: not followed by valid identifier start -- backtrack
                return if has_double_colon { Some(pos - 2) } else { None };
            }
            has_double_colon = true;
            pos += 1;
            while pos < len && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
                pos += 1;
            }
        } else {
            break;
        }
    }

    // Verify word boundary at end
    if pos < len && is_word_char(bytes[pos]) {
        return None; // No word boundary
    }

    if has_double_colon { Some(pos) } else { None }
}

/// Normalize a Ruby regex pattern string for use with Rust's regex crate.
/// Strips `/` delimiters and converts Ruby-specific anchors.
fn normalize_ruby_regex(pattern: &str) -> String {
    let mut s = pattern.trim().to_string();

    // Strip surrounding / delimiters (and optional flags)
    if s.starts_with('/') {
        s.remove(0);
        if let Some(last_slash) = s.rfind('/') {
            s.truncate(last_slash);
        }
    }

    // Convert Ruby anchors
    s = s.replace("\\A", "^").replace("\\z", "$").replace("\\Z", "$");
    s
}

/// Check if the last URI match in the line, after extension, reaches the end of the line
/// AND starts before `max`. This matches RuboCop's `allowed_position?` + `extend_end_position`.
fn uri_extends_to_end(line: &str, schemes: &[String], max: usize) -> bool {
    // Find all URI starts (last match wins, matching RuboCop behavior)
    let mut last_start = None;
    for scheme in schemes {
        let prefix = format!("{scheme}://");
        let mut search_from = 0;
        while let Some(pos) = line[search_from..].find(&prefix) {
            let abs_pos = search_from + pos;
            last_start = match last_start {
                Some(prev) if prev > abs_pos => Some(prev),
                _ => Some(abs_pos),
            };
            search_from = abs_pos + prefix.len();
        }
    }

    let start = match last_start {
        Some(s) => s,
        None => return false,
    };

    // Find end of URI (first whitespace after URI start)
    let uri_end = start
        + line[start..]
            .find(|c: char| c.is_whitespace())
            .unwrap_or(line.len() - start);

    // Extend end position (matching RuboCop's extend_end_position):
    // 1. YARD brace extension: if line contains `{` and ends with `}`,
    //    extend from end_pos through the closing `}`.
    // 2. Extend to the end of the next non-whitespace run.
    let mut end_pos = uri_end;

    // Step 1: YARD brace extension — RuboCop checks /{(\s|\S)*}$/
    // which matches any line that has a `{` somewhere and ends with `}`.
    if line.contains('{') && line.ends_with('}') {
        if let Some(brace_pos) = line[end_pos..].rfind('}') {
            end_pos += brace_pos + 1; // include the closing `}`
        }
    }

    // Step 2: Extend to next word boundary — /^\S+(?=\s|$)/
    let rest = &line[end_pos..];
    let non_ws_len = rest
        .find(|c: char| c.is_whitespace())
        .unwrap_or(rest.len());
    end_pos += non_ws_len;

    // Check allowed_position?: start_chars < max AND end_pos reaches end of line
    let start_chars = line[..start].chars().count();
    start_chars < max && end_pos >= line.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(LineLength, "cops/layout/line_length");

    #[test]
    fn custom_max() {
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(10.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"short\nthis line is longer than ten\n".to_vec());
        let diags = LineLength.check_lines(&source, &config);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].location.column, 10);
        assert_eq!(diags[0].message, "Line is too long. [28/10]"); // all ASCII, so chars == bytes
    }

    #[test]
    fn exact_max_no_offense() {
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(5.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"12345\n".to_vec());
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.is_empty());
    }

    #[test]
    fn allow_heredoc_skips_heredoc_lines() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                ("AllowHeredoc".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = <<~TXT\n  this is a very long line inside a heredoc\nTXT\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        // Only the first line (x = <<~TXT) should be checked, heredoc body skipped
        assert!(diags.is_empty() || diags.iter().all(|d| d.location.line == 1));
    }

    #[test]
    fn disallow_heredoc_flags_heredoc_lines() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                ("AllowHeredoc".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = <<~TXT\n  this is a very long line inside heredoc\nTXT\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.iter().any(|d| d.location.line == 2), "Should flag long heredoc lines when AllowHeredoc is false");
    }

    #[test]
    fn allow_uri_skips_lines_with_url() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(20.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# https://example.com/very/long/path/to/something\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.is_empty(), "AllowURI should skip lines with long URIs");
    }

    #[test]
    fn allowed_patterns_skips_matching_lines() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                ("AllowedPatterns".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^\\s*#".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# This is a very long comment line that exceeds the max\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching lines");
    }

    #[test]
    fn allow_rbs_skips_type_annotations() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(20.into())),
                ("AllowRBSInlineAnnotation".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"def foo(x) #: (Integer) -> String\nend\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.is_empty(), "AllowRBSInlineAnnotation should skip lines with RBS type annotations");
    }

    #[test]
    fn disallow_rbs_flags_type_annotations() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(20.into())),
                ("AllowRBSInlineAnnotation".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"def foo(x) #: (Integer) -> String\nend\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(!diags.is_empty(), "Should flag long RBS lines when AllowRBSInlineAnnotation is false");
    }

    #[test]
    fn allow_cop_directives_skips_rubocop_comments() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(20.into())),
                ("AllowCopDirectives".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = 1 # rubocop:disable Layout/LineLength\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.is_empty(), "AllowCopDirectives should skip lines with rubocop directives");
    }

    #[test]
    fn allow_uri_with_brace_extension_to_end_of_line() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // URI in a line ending with } — the YARD brace extension should extend
        // the URI range to end of line, matching RuboCop's behavior.
        // The URI starts before max and braces extend to end of line.
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x { https://example.com/long/path }\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(diags.is_empty(), "AllowURI with YARD brace extension should skip lines where URI extends to end");
    }

    #[test]
    fn allow_uri_without_extension_to_end_flags_offense() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // URI that does NOT extend to end of line — should still flag
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = 'https://example.com' + more_stuff_here\n".to_vec(),
        );
        let diags = LineLength.check_lines(&source, &config);
        assert!(!diags.is_empty(), "AllowURI should flag when URI does not extend to end of line");
    }
}
