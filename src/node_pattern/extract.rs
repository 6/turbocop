//! Ruby source pattern extraction.
//!
//! Extracts `def_node_matcher` and `def_node_search` patterns from Ruby source.

#[derive(Debug, Clone)]
pub struct ExtractedPattern {
    pub kind: PatternKind,
    pub method_name: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    Matcher,
    Search,
}

pub fn extract_patterns(source: &str) -> Vec<ExtractedPattern> {
    let mut results = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        for (prefix, kind) in [
            ("def_node_matcher", PatternKind::Matcher),
            ("def_node_search", PatternKind::Search),
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix(':') {
                    if let Some(comma_pos) = rest.find(',') {
                        let method_name = rest[..comma_pos].trim().to_string();
                        let after_comma = rest[comma_pos + 1..].trim();

                        if after_comma.starts_with("<<~") {
                            // Heredoc form — read until the delimiter
                            let delimiter = after_comma
                                .trim_start_matches("<<~")
                                .trim()
                                .trim_matches('\'')
                                .trim_matches('"');
                            let mut pattern_lines = Vec::new();
                            i += 1;
                            while i < lines.len() {
                                let line = lines[i].trim();
                                if line == delimiter {
                                    break;
                                }
                                pattern_lines.push(line);
                                i += 1;
                            }
                            let pattern = pattern_lines.join("\n");
                            results.push(ExtractedPattern {
                                kind,
                                method_name,
                                pattern,
                            });
                        } else if after_comma.starts_with('\'')
                            || after_comma.starts_with('"')
                        {
                            // Inline string form
                            let quote = after_comma.as_bytes()[0] as char;
                            let inner = &after_comma[1..];
                            if let Some(end) = inner.rfind(quote) {
                                let pattern = inner[..end].to_string();
                                results.push(ExtractedPattern {
                                    kind,
                                    method_name,
                                    pattern,
                                });
                            }
                        }
                    }
                }
            }
        }

        i += 1;
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_heredoc_pattern() {
        let source = r#"
        def_node_matcher :expect?, <<~PATTERN
          (send nil? :expect ...)
        PATTERN
        "#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "expect?");
        assert_eq!(patterns[0].kind, PatternKind::Matcher);
        assert!(patterns[0].pattern.contains("send nil? :expect"));
    }

    #[test]
    fn test_extract_inline_pattern() {
        let source = r#"def_node_search :gem_declarations, '(send nil? :gem str ...)'"#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "gem_declarations");
        assert_eq!(patterns[0].kind, PatternKind::Search);
    }

    #[test]
    fn test_extract_multiple_patterns() {
        let source = r#"
        def_node_matcher :expect?, <<~PATTERN
          (send nil? :expect ...)
        PATTERN

        def_node_matcher :expect_block?, <<~PATTERN
          (block #expect? (args) _body)
        PATTERN
        "#;

        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].method_name, "expect?");
        assert_eq!(patterns[1].method_name, "expect_block?");
    }

    #[test]
    fn test_extract_with_double_quotes() {
        let source = "def_node_matcher :my_match?, \"(send _ :foo ...)\"";
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "my_match?");
        assert_eq!(patterns[0].pattern, "(send _ :foo ...)");
    }

    #[test]
    fn test_extract_no_patterns() {
        let source = "class MyCop < Base\n  def on_send(node)\n  end\nend";
        let patterns = extract_patterns(source);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_extract_search_pattern() {
        let source = r#"def_node_search :find_all_sends, '(send _ :puts ...)'"#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].kind, PatternKind::Search);
        assert_eq!(patterns[0].method_name, "find_all_sends");
    }

    #[test]
    fn test_extract_heredoc_with_single_quotes() {
        let source =
            "def_node_matcher :my_matcher?, <<~'PATTERN'\n  (send _ :bar)\nPATTERN";
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "my_matcher?");
        assert!(patterns[0].pattern.contains("send"));
    }
}
