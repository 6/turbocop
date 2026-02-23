//! Ruby source pattern extraction.
//!
//! Extracts `def_node_matcher` and `def_node_search` patterns from Ruby source.

use std::path::Path;

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

/// Extract the method name from after the prefix.
///
/// Handles both `:method_name` and `'self.method_name'` / `"self.method_name"` forms.
/// Returns `(method_name, rest_after_comma)` or `None` if no match.
// Lifetime is needed for clarity: the returned &str borrows from the input.
#[allow(clippy::needless_lifetimes)]
fn parse_method_name_and_rest<'a>(rest: &'a str) -> Option<(String, &'a str)> {
    let rest = rest.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        // Symbol form: :method_name, ...
        let comma_pos = rest.find(',')?;
        let method_name = rest[..comma_pos].trim().to_string();
        let after_comma = rest[comma_pos + 1..].trim();
        Some((method_name, after_comma))
    } else if rest.starts_with('\'') || rest.starts_with('"') {
        // String form: 'self.method_name', ... or "self.method_name", ...
        let quote = rest.as_bytes()[0] as char;
        let inner = &rest[1..];
        let end = inner.find(quote)?;
        let method_name = inner[..end].trim().to_string();
        let after_name = inner[end + 1..].trim();
        let after_comma = after_name.strip_prefix(',')?.trim();
        Some((method_name, after_comma))
    } else {
        None
    }
}

/// Strip trailing Ruby comments from a heredoc delimiter line.
///
/// `<<~PATTERN # rubocop:disable ...` → `PATTERN`
fn strip_heredoc_trailing_comment(delimiter: &str) -> &str {
    if let Some(hash_pos) = delimiter.find('#') {
        delimiter[..hash_pos].trim()
    } else {
        delimiter
    }
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
                if let Some((method_name, after_comma)) = parse_method_name_and_rest(rest) {
                    if after_comma.starts_with("<<~") {
                        // Heredoc form — read until the delimiter
                        let raw_delim = after_comma.trim_start_matches("<<~");
                        let raw_delim = strip_heredoc_trailing_comment(raw_delim);
                        let delimiter = raw_delim.trim().trim_matches('\'').trim_matches('"');
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
                    } else if after_comma.starts_with('\'') || after_comma.starts_with('"') {
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

        i += 1;
    }

    results
}

/// Convert a vendor cop file path to a `Department/CopName` string.
///
/// Examples:
/// - `vendor/rubocop/lib/rubocop/cop/style/nil_comparison.rb` → `Style/NilComparison`
/// - `vendor/rubocop-rails/lib/rubocop/cop/rails/find_by.rb` → `Rails/FindBy`
/// - `vendor/rubocop-rspec/lib/rubocop/cop/rspec/expect_change.rb` → `RSpec/ExpectChange`
pub fn cop_name_from_path(path: &Path) -> Option<String> {
    let path_str = path.to_str()?;

    // Find the `cop/` segment
    let cop_idx = path_str.find("/cop/")?;
    let after_cop = &path_str[cop_idx + 5..]; // skip "/cop/"

    // Strip .rb extension
    let after_cop = after_cop.strip_suffix(".rb")?;

    // Split into segments: e.g. "style/nil_comparison" → ["style", "nil_comparison"]
    let segments: Vec<&str> = after_cop.split('/').collect();
    if segments.len() < 2 {
        return None;
    }

    // Convert each segment from snake_case to PascalCase
    let pascal_segments: Vec<String> = segments.iter().map(|s| snake_to_pascal(s)).collect();

    Some(pascal_segments.join("/"))
}

/// Convert a snake_case string to PascalCase.
///
/// Handles special acronyms like `rspec` → `RSpec`.
fn snake_to_pascal(s: &str) -> String {
    // Special-case known acronyms
    match s {
        "rspec" => return "RSpec".to_string(),
        "rspec_rails" => return "RSpecRails".to_string(),
        _ => {}
    }

    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect()
}

/// Walk all vendor gem cop directories and extract NodePattern definitions.
///
/// Returns `(cop_name, pattern)` pairs. Skips files that fail to read.
pub fn walk_vendor_patterns(vendor_root: &Path) -> Vec<(String, ExtractedPattern)> {
    let mut results = Vec::new();

    // Known vendor gem directories
    let gems = [
        "rubocop",
        "rubocop-rails",
        "rubocop-rspec",
        "rubocop-performance",
        "rubocop-factory_bot",
        "rubocop-rspec_rails",
    ];

    for gem in &gems {
        let cop_dir = vendor_root.join(gem).join("lib/rubocop/cop");
        if !cop_dir.is_dir() {
            continue;
        }
        walk_dir_recursive(&cop_dir, &mut results);
    }

    results
}

fn walk_dir_recursive(dir: &Path, results: &mut Vec<(String, ExtractedPattern)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir_recursive(&path, results);
        } else if path.extension().is_some_and(|e| e == "rb") {
            if let Ok(source) = std::fs::read_to_string(&path) {
                let patterns = extract_patterns(&source);
                for p in patterns {
                    if let Some(cop_name) = cop_name_from_path(&path) {
                        results.push((cop_name, p));
                    }
                }
            }
        }
    }
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
        let source = "def_node_matcher :my_matcher?, <<~'PATTERN'\n  (send _ :bar)\nPATTERN";
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "my_matcher?");
        assert!(patterns[0].pattern.contains("send"));
    }

    #[test]
    fn test_extract_heredoc_with_trailing_comment() {
        let source = r#"
          def_node_matcher :my_matcher?, <<~PATTERN # rubocop:disable InternalAffairs/NodeMatcherDirective
            (send nil? :assert_equal $_ $_ $_?)
          PATTERN
        "#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "my_matcher?");
        assert!(patterns[0].pattern.contains("assert_equal"));
    }

    #[test]
    fn test_extract_heredoc_matcher_delimiter() {
        let source = "def_node_matcher :my_match?, <<~MATCHER\n  (send _ :foo)\nMATCHER";
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "my_match?");
        assert!(patterns[0].pattern.contains("send"));
    }

    #[test]
    fn test_extract_heredoc_ruby_delimiter() {
        let source = "def_node_matcher :my_match?, <<~RUBY\n  (send _ :bar)\nRUBY";
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].pattern.contains("send"));
    }

    #[test]
    fn test_extract_self_method_name() {
        let source = r#"
          def_node_matcher 'self.minitest_assertion', <<~PATTERN
            (send nil? :assert_equal $_ $_)
          PATTERN
        "#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "self.minitest_assertion");
        assert!(patterns[0].pattern.contains("assert_equal"));
    }

    #[test]
    fn test_extract_self_method_name_inline() {
        let source = r#"def_node_matcher 'self.my_check', '(send nil? :foo)'"#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "self.my_check");
    }

    #[test]
    fn test_cop_name_from_path_style() {
        let path = Path::new("vendor/rubocop/lib/rubocop/cop/style/nil_comparison.rb");
        assert_eq!(
            cop_name_from_path(path),
            Some("Style/NilComparison".to_string())
        );
    }

    #[test]
    fn test_cop_name_from_path_rails() {
        let path = Path::new("vendor/rubocop-rails/lib/rubocop/cop/rails/find_by.rb");
        assert_eq!(cop_name_from_path(path), Some("Rails/FindBy".to_string()));
    }

    #[test]
    fn test_cop_name_from_path_rspec() {
        let path = Path::new("vendor/rubocop-rspec/lib/rubocop/cop/rspec/expect_change.rb");
        assert_eq!(
            cop_name_from_path(path),
            Some("RSpec/ExpectChange".to_string())
        );
    }

    #[test]
    fn test_cop_name_from_path_performance() {
        let path = Path::new(
            "vendor/rubocop-performance/lib/rubocop/cop/performance/string_replacement.rb",
        );
        assert_eq!(
            cop_name_from_path(path),
            Some("Performance/StringReplacement".to_string())
        );
    }

    #[test]
    fn test_cop_name_from_path_no_cop_segment() {
        let path = Path::new("vendor/rubocop/lib/rubocop/runner.rb");
        assert_eq!(cop_name_from_path(path), None);
    }

    #[test]
    fn test_cop_name_from_path_single_segment() {
        // Only one segment after cop/ — should return None
        let path = Path::new("vendor/rubocop/lib/rubocop/cop/base.rb");
        assert_eq!(cop_name_from_path(path), None);
    }

    #[test]
    fn test_snake_to_pascal() {
        assert_eq!(snake_to_pascal("nil_comparison"), "NilComparison");
        assert_eq!(snake_to_pascal("find_by"), "FindBy");
        assert_eq!(snake_to_pascal("style"), "Style");
        assert_eq!(snake_to_pascal("rspec"), "RSpec");
    }

    #[test]
    fn test_strip_heredoc_trailing_comment() {
        assert_eq!(
            strip_heredoc_trailing_comment("PATTERN # rubocop:disable Foo"),
            "PATTERN"
        );
        assert_eq!(strip_heredoc_trailing_comment("PATTERN"), "PATTERN");
        assert_eq!(
            strip_heredoc_trailing_comment("MATCHER # comment"),
            "MATCHER"
        );
    }
}
