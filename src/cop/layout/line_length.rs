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
        let allow_qualified_name = config.get_bool("AllowQualifiedName", false);
        let uri_schemes = config.get_string_array("URISchemes")
            .unwrap_or_else(|| vec!["http".into(), "https".into()]);
        let allow_rbs = config.get_bool("AllowRBSInlineAnnotation", false);
        let allow_cop_directives = config.get_bool("AllowCopDirectives", true);
        let allowed_patterns = config.get_string_array("AllowedPatterns").unwrap_or_default();
        // SplitStrings is an autocorrection-only option; read it for config_audit compliance
        // but it has no effect on offense detection (only on how offenses are auto-fixed).
        let _split_strings = config.get_bool("SplitStrings", false);
        // Pre-compile allowed patterns
        let compiled_patterns: Vec<regex::Regex> = allowed_patterns.iter()
            .filter_map(|p| regex::Regex::new(p).ok())
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

            // Detect heredoc openers
            if heredoc_terminator.is_none() {
                if let Some(pos) = line.windows(2).position(|w| w == b"<<") {
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
                    }
                }
            }

            if line.len() <= max {
                continue;
            }

            // AllowCopDirectives: skip lines that are only long because of a rubocop directive comment
            if allow_cop_directives {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    if let Some(comment_start) = line_str.find("# rubocop:") {
                        let without_directive = &line[..comment_start].iter()
                            .rposition(|&b| b != b' ' && b != b'\t')
                            .map_or(0, |p| p + 1);
                        if *without_directive <= max {
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
                            || line[comment_start - 1] == b' '
                            || line[comment_start - 1] == b'\t';
                        if is_rbs {
                            let without_rbs = &line[..comment_start].iter()
                                .rposition(|&b| b != b' ' && b != b'\t')
                                .map_or(0, |p| p + 1);
                            if *without_rbs <= max {
                                continue;
                            }
                        }
                    }
                }
            }

            // AllowURI: skip lines containing a URI that makes them long
            if allow_uri {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    let has_long_uri = uri_schemes.iter().any(|scheme| {
                        let prefix = format!("{scheme}://");
                        if let Some(start) = line_str.find(&prefix) {
                            let uri_end = line_str[start..].find(|c: char| c.is_whitespace()).unwrap_or(line_str.len() - start);
                            let without_uri_len = line.len() - uri_end;
                            without_uri_len <= max
                        } else {
                            false
                        }
                    });
                    if has_long_uri {
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

            // AllowQualifiedName: skip lines that are long only because of a qualified name (Foo::Bar::Baz)
            if allow_qualified_name {
                if let Ok(line_str) = std::str::from_utf8(line) {
                    let stripped = line_str.trim();
                    if stripped.contains("::") && !stripped.contains(' ') && stripped.len() > max {
                        continue;
                    }
                }
            }

            diagnostics.push(self.diagnostic(
                source,
                i + 1,
                max,
                format!("Line is too long. [{}/{}]", line.len(), max),
            ));
        }
        diagnostics
    }
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
        assert_eq!(diags[0].message, "Line is too long. [28/10]");
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
}
