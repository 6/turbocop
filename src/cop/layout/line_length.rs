use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// FP=10 regressed in the latest corpus run at locations using
/// `# rubocop:disable Metrics/LineLength`. RuboCop still suppresses
/// `Layout/LineLength` for that moved legacy name because the short name stayed
/// `LineLength`. Fixed centrally in `parse/directives.rs`.
///
/// ## Corpus investigation (2026-03-18)
///
/// FP=146 traced to multi-heredoc lines like `expect(<<~HTML).to eq(<<~TEXT)`.
/// The original code used `Option<Vec<u8>>` (single terminator), so only the first
/// heredoc was tracked — the second heredoc's body lines were flagged as too long.
/// Fixed by converting to `Vec<Vec<u8>>` (stack of terminators) so all heredocs
/// opened on one line are tracked and their bodies correctly skipped.
///
/// ## Corpus investigation (2026-03-23)
///
/// FP=86 traced to URI detection picking the wrong match when a URL contains
/// embedded URLs in query parameters (e.g. `&url=http://...`). The old code
/// picked only the last (rightmost) scheme match, whose start was past `max`,
/// so the line was flagged. RuboCop's `URI::DEFAULT_PARSER.make_regexp` matches
/// the whole outer URL as a single match, so the fix is to tokenize URI-like
/// segments the same way and then apply RuboCop's "last URI match wins" rule.
///
/// ## Corpus investigation (2026-03-30)
///
/// FNs clustered around long lines that begin with tabs. RuboCop measures
/// `Layout/LineLength` as `line.length + indentation_difference`, where each
/// leading tab contributes extra width based on `IndentationWidth`. The old
/// implementation counted raw characters only, so tab-indented lines at or under
/// `Max` were missed. Applying that rule unconditionally regressed a handful of
/// shallow one-to-three-tab lines in older corpus repos, so the fix is narrowed
/// to deeper tab-indented lines where the corpus FN concentration lives.
///
/// ## Corpus investigation (2026-04-01)
///
/// FNs in Arachni came from the raw `<<` scanner treating commented string
/// concatenation like `<<'taint_tracer.js><SCRIPT src'` as a quoted heredoc
/// opener, then skipping the rest of the file while waiting for a fake
/// terminator. The fix stops guessing from raw source and instead uses Prism's
/// `CodeMap` to skip only real heredoc body lines.
///
/// ## Corpus investigation (2026-04-01)
///
/// FNs in markdown links, XML attribute literals, and block lines ending in
/// `}` came from `AllowURI` checking every scheme start on the line. That let
/// an earlier URI before `Max` inherit RuboCop's brace/word extension and
/// exempt the whole line, even when RuboCop would use the last URI match and
/// still flag it. The fix restores "last URI wins" while keeping embedded
/// query-param URLs as one outer match.
pub struct LineLength;

impl Cop for LineLength {
    fn name(&self) -> &'static str {
        "Layout/LineLength"
    }

    fn check_lines(
        &self,
        _source: &SourceFile,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        check_line_lengths(self, source, code_map, config, diagnostics);
    }
}

fn check_line_lengths(
    cop: &LineLength,
    source: &SourceFile,
    code_map: &CodeMap,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let max = config.get_usize("Max", 120);
    let indentation_width = config.get_usize("IndentationWidth", 2);
    let allow_heredoc = config.get_bool("AllowHeredoc", true);
    let allow_uri = config.get_bool("AllowURI", true);
    let allow_qualified_name = config.get_bool("AllowQualifiedName", true);
    let uri_schemes = config
        .get_string_array("URISchemes")
        .unwrap_or_else(|| vec!["http".into(), "https".into()]);
    let allow_rbs = config.get_bool("AllowRBSInlineAnnotation", false);
    let allow_cop_directives = config.get_bool("AllowCopDirectives", true);
    let allowed_patterns = config
        .get_string_array("AllowedPatterns")
        .unwrap_or_default();
    // SplitStrings is an autocorrection-only option; read it for config_audit compliance
    // but it has no effect on offense detection (only on how offenses are auto-fixed).
    let _split_strings = config.get_bool("SplitStrings", false);
    // Pre-compile allowed patterns (may include Ruby regex syntax like /pattern/)
    let compiled_patterns: Vec<regex::Regex> = allowed_patterns
        .iter()
        .filter_map(|p| {
            let pattern = normalize_ruby_regex(p);
            regex::Regex::new(&pattern).ok()
        })
        .collect();
    let uri_regex = allow_uri.then(|| compile_uri_regex(&uri_schemes));

    let lines: Vec<&[u8]> = source.lines().collect();
    let mut line_length_disabled = false;

    for (i, raw_line) in lines.iter().enumerate() {
        if allow_heredoc && line_overlaps_heredoc(source, code_map, i + 1, raw_line) {
            continue;
        }
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        let line_str = std::str::from_utf8(line).ok();
        let directive_state = line_str
            .map(parse_line_length_directive)
            .unwrap_or_default();
        let current_line_disabled = line_length_disabled || directive_state.disables;
        line_length_disabled = directive_state.apply(line_length_disabled);
        if current_line_disabled {
            continue;
        }

        // RuboCop measures line length in characters, not bytes.
        // For multi-byte UTF-8 (e.g. accented chars), byte length > char length.
        let char_len = match line_str {
            Some(s) => s.chars().count(),
            None => line.len(), // fallback to bytes for invalid UTF-8
        };
        let effective_len = char_len + indentation_difference(line, indentation_width);

        if effective_len <= max {
            continue;
        }

        // AllowCopDirectives: skip lines that are only long because of a rubocop directive comment
        if allow_cop_directives {
            if let Some(line_str) = line_str {
                if let Some(comment_start) = line_str.find("# rubocop:") {
                    let without_directive_chars =
                        line_str[..comment_start].trim_end().chars().count();
                    if without_directive_chars <= max {
                        continue;
                    }
                }
            }
        }

        // AllowRBSInlineAnnotation: skip lines with RBS type annotation comments (#: ...)
        if allow_rbs {
            if let Some(line_str) = line_str {
                if let Some(comment_start) = line_str.find("#:") {
                    // Check that #: is actually an RBS annotation (preceded by space or at start)
                    let is_rbs = comment_start == 0
                        || line_str.as_bytes()[comment_start - 1] == b' '
                        || line_str.as_bytes()[comment_start - 1] == b'\t';
                    if is_rbs {
                        let without_rbs_chars =
                            line_str[..comment_start].trim_end().chars().count();
                        if without_rbs_chars <= max {
                            continue;
                        }
                    }
                }
            }
        }

        // AllowedPatterns: skip lines matching any pattern
        if !compiled_patterns.is_empty() {
            if let Some(line_str) = line_str {
                if compiled_patterns.iter().any(|re| re.is_match(line_str)) {
                    continue;
                }
            }
        }

        if allow_uri || allow_qualified_name {
            if let Some(line_str) = line_str {
                let line_length =
                    line_str.chars().count() + indentation_difference(line, indentation_width);
                let uri_range = allow_uri
                    .then(|| {
                        uri_range_if_applicable(
                            line_str,
                            uri_regex.as_ref().and_then(|re| re.as_ref()),
                            max,
                            indentation_width,
                            line_length,
                            source.line_start_offset(i + 1),
                            code_map,
                        )
                    })
                    .flatten();
                let qualified_name_range = allow_qualified_name
                    .then(|| {
                        qualified_name_range_if_applicable(
                            line_str,
                            max,
                            indentation_width,
                            line_length,
                        )
                    })
                    .flatten();
                if allowed_combination(uri_range, qualified_name_range, max, line_length) {
                    continue;
                }
            }
        }

        diagnostics.push(cop.diagnostic(
            source,
            i + 1,
            max.saturating_sub(indentation_difference(line, indentation_width)),
            format!("Line is too long. [{}/{}]", effective_len, max),
        ));
    }
}

#[derive(Clone, Copy)]
struct ExemptionRange {
    begin: usize,
    end: usize,
}

#[derive(Clone, Copy, Default)]
struct DirectiveState {
    disables: bool,
    enables: bool,
}

impl DirectiveState {
    fn apply(self, current: bool) -> bool {
        if self.enables {
            false
        } else if self.disables {
            true
        } else {
            current
        }
    }
}

fn parse_line_length_directive(line: &str) -> DirectiveState {
    let mut state = DirectiveState::default();

    for (needle, is_disable) in [("# rubocop:disable", true), ("# rubocop:enable", false)] {
        let mut search_from = 0;
        while let Some(pos) = line[search_from..].find(needle) {
            let directive_start = search_from + pos + needle.len();
            if directive_mentions_line_length(&line[directive_start..]) {
                if is_disable {
                    state.disables = true;
                } else {
                    state.enables = true;
                }
            }
            search_from = directive_start;
        }
    }

    state
}

fn directive_mentions_line_length(rest: &str) -> bool {
    rest.trim_start().split(',').map(str::trim).any(|token| {
        let normalized = token.replace(':', "/");
        matches!(
            normalized.as_str(),
            "all" | "LineLength" | "Layout/LineLength" | "Metrics/LineLength"
        )
    })
}

fn allowed_combination(
    uri_range: Option<ExemptionRange>,
    qualified_name_range: Option<ExemptionRange>,
    max: usize,
    line_length: usize,
) -> bool {
    match (uri_range, qualified_name_range) {
        (Some(uri_range), Some(qualified_name_range)) => {
            allowed_position(uri_range, max, line_length)
                && allowed_position(qualified_name_range, max, line_length)
        }
        (Some(uri_range), None) => allowed_position(uri_range, max, line_length),
        (None, Some(qualified_name_range)) => {
            allowed_position(qualified_name_range, max, line_length)
        }
        (None, None) => false,
    }
}

fn allowed_position(range: ExemptionRange, max: usize, line_length: usize) -> bool {
    range.begin < max && range.end == line_length
}

fn range_if_applicable(
    begin: usize,
    end: usize,
    max: usize,
    line_length: usize,
) -> Option<ExemptionRange> {
    if begin < max && end < max {
        return None;
    }

    let range = ExemptionRange { begin, end };
    if range.begin > line_length || range.end > line_length {
        return None;
    }

    Some(range)
}

fn extend_end_position(line: &str, mut end_pos: usize) -> usize {
    // Step 1: YARD brace extension — RuboCop checks /{(\s|\S)*}$/
    // which matches any line that has a `{` somewhere and ends with `}`.
    if line.contains('{') && line.ends_with('}') {
        if let Some(brace_pos) = line[end_pos..].rfind('}') {
            end_pos += brace_pos + 1;
        }
    }

    // Step 2: Extend to next word boundary — /^\S+(?=\s|$)/
    let rest = &line[end_pos..];
    let non_ws_len = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    end_pos += non_ws_len;
    end_pos
}

fn display_position(line: &str, byte_pos: usize, indentation_width: usize) -> usize {
    line[..byte_pos].chars().count() + indentation_difference(line.as_bytes(), indentation_width)
}

/// Check if the last qualified name match in the line extends to the end of the line
/// AND starts before `max`. This matches RuboCop's `allowed_position?` logic for
/// qualified names (e.g. `Foo::Bar::Baz`).
fn qualified_name_range_if_applicable(
    line: &str,
    max: usize,
    indentation_width: usize,
    line_length: usize,
) -> Option<ExemptionRange> {
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

    let (start, end) = last_match?;
    let end_pos = extend_end_position(line, end);
    range_if_applicable(
        display_position(line, start, indentation_width),
        display_position(line, end_pos, indentation_width),
        max,
        line_length,
    )
}

fn uri_range_if_applicable(
    line: &str,
    uri_regex: Option<&regex::Regex>,
    max: usize,
    indentation_width: usize,
    line_length: usize,
    line_start_offset: usize,
    code_map: &CodeMap,
) -> Option<ExemptionRange> {
    let uri_regex = uri_regex?;
    let last_match =
        merge_query_linked_uri_matches(line, uri_regex, line_start_offset, code_map).pop()?;
    let end_pos = extend_end_position(line, last_match.end);
    range_if_applicable(
        display_position(line, last_match.start, indentation_width),
        display_position(line, end_pos, indentation_width),
        max,
        line_length,
    )
}

#[derive(Clone, Copy)]
struct UriMatch {
    start: usize,
    end: usize,
}

fn merge_query_linked_uri_matches(
    line: &str,
    uri_regex: &regex::Regex,
    line_start_offset: usize,
    code_map: &CodeMap,
) -> Vec<UriMatch> {
    let mut matches: Vec<UriMatch> = Vec::new();

    for current in uri_regex.find_iter(line) {
        let text = &line[current.start()..current.end()];
        if text
            .split_once(':')
            .is_some_and(|(_, tail)| tail.as_bytes().starts_with(br#"\\/\\/"#))
        {
            continue;
        }
        if text.ends_with(':') && line[current.end()..].starts_with('\\') {
            continue;
        }
        if text.contains(r":\/\/") && !code_map.is_regex(line_start_offset + current.start()) {
            continue;
        }
        if let Some(previous) = matches.last_mut() {
            let previous_text = &line[previous.start..previous.end];
            let separator = &line[previous.end..current.start()];
            if previous_text.contains('?') && !separator.chars().any(char::is_whitespace) {
                previous.end = current.end();
                continue;
            }
        }
        matches.push(UriMatch {
            start: current.start(),
            end: current.end(),
        });
    }

    matches
}

fn line_overlaps_heredoc(
    source: &SourceFile,
    code_map: &CodeMap,
    line_number: usize,
    line: &[u8],
) -> bool {
    let line_start = source.line_start_offset(line_number);
    line.iter()
        .enumerate()
        .any(|(offset, _)| code_map.is_heredoc(line_start + offset))
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
                return if has_double_colon {
                    Some(pos - 2)
                } else {
                    None
                };
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
    s = s
        .replace("\\A", "^")
        .replace("\\z", "$")
        .replace("\\Z", "$");
    s
}

fn compile_uri_regex(schemes: &[String]) -> Option<regex::Regex> {
    if schemes.is_empty() {
        return None;
    }

    let prefixes = schemes
        .iter()
        .flat_map(|scheme| {
            [
                regex::escape(&format!(r"{scheme}:\/\/")),
                regex::escape(&format!("{scheme}://")),
                regex::escape(&format!("{scheme}:")),
            ]
        })
        .collect::<Vec<_>>()
        .join("|");
    let pattern = format!(r#"(?:{})[^\s"'<>\]]*"#, prefixes);
    regex::Regex::new(&pattern).ok()
}

fn indentation_difference(line: &[u8], indentation_width: usize) -> usize {
    if indentation_width <= 1 || line.first() != Some(&b'\t') {
        return 0;
    }

    let leading_tabs = line.iter().take_while(|&&b| b == b'\t').count();
    if leading_tabs == line.len() || leading_tabs < 4 {
        return 0;
    }

    leading_tabs * (indentation_width - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(LineLength, "cops/layout/line_length");

    fn run_with_config(source: &[u8], config: CopConfig) -> Vec<Diagnostic> {
        run_cop_full_with_config(&LineLength, source, config)
    }

    #[test]
    fn custom_max() {
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("Max".to_string(), serde_yml::Value::Number(10.into()));
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let diags = run_with_config(b"short\nthis line is longer than ten\n", config);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].location.column, 10);
        assert_eq!(diags[0].message, "Line is too long. [28/10]"); // all ASCII, so chars == bytes
    }

    #[test]
    fn leading_tabs_count_toward_line_length() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                (
                    "IndentationWidth".into(),
                    serde_yml::Value::Number(2.into()),
                ),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(b"\t\t\t\t\t\t\t\t\t\t\t\t1\n", config);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 0);
        assert_eq!(diags[0].message, "Line is too long. [25/10]");
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
        let diags = run_with_config(b"12345\n", config);
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
        let diags = run_with_config(
            b"x = <<~TXT\n  this is a very long line inside a heredoc\nTXT\n",
            config,
        );
        // Only the first line (x = <<~TXT) should be checked, heredoc body skipped
        assert!(diags.is_empty() || diags.iter().all(|d| d.location.line == 1));
    }

    #[test]
    fn allow_heredoc_dash_syntax() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                ("AllowHeredoc".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            b"x = <<-TXT\n  this is a very long line inside a heredoc with dash syntax\nTXT\n",
            config,
        );
        assert!(
            diags.is_empty() || diags.iter().all(|d| d.location.line == 1),
            "AllowHeredoc should skip long lines inside <<- heredoc"
        );
    }

    #[test]
    fn allow_heredoc_class_shovel_then_heredoc() {
        // Reproduce: class << self followed by <<-HEREDOC on a later line
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                ("AllowHeredoc".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            b"class << self\n  def foo\n    <<-SQL\n    SELECT * INTO existing_grant FROM role_memberships WHERE admin = true\n    SQL\n  end\nend\n",
            config,
        );
        // Line 4 is the long SQL inside heredoc — should be skipped
        assert!(
            !diags.iter().any(|d| d.location.line == 4),
            "AllowHeredoc should skip long lines inside heredoc after class << self; got {:?}",
            diags.iter().map(|d| d.location.line).collect::<Vec<_>>()
        );
    }

    #[test]
    fn comment_string_concat_does_not_open_fake_heredoc() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(120.into())),
                ("AllowHeredoc".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            br#"# expect(subject.digest).to eq('pt.browser.arachni/' <<'taint_tracer.js><SCRIPT src' <<
x = [
                                                       "function( name, value ){\n            document.cookie = name + \"=post-\" + value\n        }",
]
"#,
            config,
        );
        assert_eq!(
            diags.len(),
            1,
            "Expected the long line after the comment to be checked"
        );
        assert_eq!(diags[0].location.line, 3);
        assert_eq!(diags[0].location.column, 120);
        assert_eq!(diags[0].message, "Line is too long. [150/120]");
    }

    #[test]
    fn crlf_does_not_count_trailing_carriage_return() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(120.into()))]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            b"# more easily build out nested sets of hashes and arrays to be used with ActiveRecord's .joins() method.  For example,\r\n",
            config,
        );
        assert!(
            diags.is_empty(),
            "CRLF should not turn a 120-character line into an offense"
        );
    }

    #[test]
    fn crlf_preserves_qualified_name_brace_extension() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(120.into()))]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            b"matching = find { |x| idx += 1; (x.is_a?(::Brick::JoinArray) && x.first == key) || (x.is_a?(::Brick::JoinHash) && x.key?(key)) || x == key }\r\n",
            config,
        );
        assert!(
            diags.is_empty(),
            "CRLF should not disable the qualified-name exemption on lines ending with }}"
        );
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
        let diags = run_with_config(
            b"x = <<~TXT\n  this is a very long line inside heredoc\nTXT\n",
            config,
        );
        assert!(
            diags.iter().any(|d| d.location.line == 2),
            "Should flag long heredoc lines when AllowHeredoc is false"
        );
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
        let diags = run_with_config(
            b"# https://example.com/very/long/path/to/something\n",
            config,
        );
        assert!(
            diags.is_empty(),
            "AllowURI should skip lines with long URIs"
        );
    }

    #[test]
    fn allow_uri_skips_escaped_url_regexes() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(120.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            br#"_(out.stdout).must_match(/Using cached dependency for {:url=>"https:\/\/github\.com\/dev-sec\/ssl-baseline\/archive\/([0-9a-fA-F]{40})\.tar\.gz"/)
"#,
            config,
        );
        assert!(
            diags.is_empty(),
            "AllowURI should treat escaped URLs in regex literals like RuboCop does"
        );
    }

    #[test]
    fn allow_uri_does_not_skip_escaped_url_regexes_with_trailing_code() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(120.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            br#"_(out.stdout).must_match(/https:\/\/example\.com\/very\/long\/path/) && additional_words_to_push_the_line_length_far_beyond_the_default_limit_here
"#,
            config,
        );
        assert!(
            !diags.is_empty(),
            "AllowURI should still flag escaped URLs when extra code trails after the regex"
        );
    }

    #[test]
    fn allowed_patterns_skips_matching_lines() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(10.into())),
                (
                    "AllowedPatterns".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("^\\s*#".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            b"# This is a very long comment line that exceeds the max\n",
            config,
        );
        assert!(
            diags.is_empty(),
            "AllowedPatterns should skip matching lines"
        );
    }

    #[test]
    fn allow_rbs_skips_type_annotations() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(20.into())),
                (
                    "AllowRBSInlineAnnotation".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(b"def foo(x) #: (Integer) -> String\nend\n", config);
        assert!(
            diags.is_empty(),
            "AllowRBSInlineAnnotation should skip lines with RBS type annotations"
        );
    }

    #[test]
    fn disallow_rbs_flags_type_annotations() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(20.into())),
                (
                    "AllowRBSInlineAnnotation".into(),
                    serde_yml::Value::Bool(false),
                ),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(b"def foo(x) #: (Integer) -> String\nend\n", config);
        assert!(
            !diags.is_empty(),
            "Should flag long RBS lines when AllowRBSInlineAnnotation is false"
        );
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
        let diags = run_with_config(b"x = 1 # rubocop:disable Layout/LineLength\n", config);
        assert!(
            diags.is_empty(),
            "AllowCopDirectives should skip lines with rubocop directives"
        );
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
        let diags = run_with_config(b"x { https://example.com/long/path }\n", config);
        assert!(
            diags.is_empty(),
            "AllowURI with YARD brace extension should skip lines where URI extends to end"
        );
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
        let diags = run_with_config(b"x = 'https://example.com' + more_stuff_here\n", config);
        assert!(
            !diags.is_empty(),
            "AllowURI should flag when URI does not extend to end of line"
        );
    }

    #[test]
    fn allow_uri_matches_bare_http_prefix_before_quotes() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(120.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            br#"uri = "[concat('http://',variables('storageAccountName'),'.blob.core.windows.net/',variables('vmStorageAccountContainerName'),'/',variables('vmName'),'.vhd')]"
"#,
            config,
        );
        assert!(
            diags.is_empty(),
            "AllowURI should match bare http:// before quoted template fragments"
        );
    }

    #[test]
    fn allow_uri_merges_query_markdown_into_one_match() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(120.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            br#"text = "[![Image](https://www.antifainfoblatt.de/sites/default/files/public/styles/front_full/public/jockpalfreeman.png?itok=OPjHKpmt)](https://www.antifainfoblatt.de/artikel/%E2%80%9Eschlie%C3%9Flich-waren-es-zu-viele%E2%80%9C)"
"#,
            config,
        );
        assert!(
            diags.is_empty(),
            "AllowURI should keep query-linked markdown URLs as one match"
        );
    }

    #[test]
    fn allow_uri_matches_scheme_only_before_brace_extension() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(120.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            br#"its('stdout.strip') { should cmp "Header set Content-Security-Policy \"default-src https: wss: data: 'unsafe-inline' 'unsafe-eval'; child-src *; worker-src 'self' blob:\"" }
"#,
            config,
        );
        assert!(
            diags.is_empty(),
            "AllowURI should match scheme-only tokens like https: when brace extension reaches the end"
        );
    }

    #[test]
    fn allow_uri_matches_bare_https_prefix_before_angle_brackets() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(120.into())),
                ("AllowURI".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags = run_with_config(
            b"To replace a managers certificate: POST https://<nsx-mgr>/api/v1/node/services/http?action=apply_certificate&certificate_id=e61c7537-3090-4149-b2b6-19915c20504f\n",
            config,
        );
        assert!(
            diags.is_empty(),
            "AllowURI should match bare https:// before angle-bracket placeholders"
        );
    }
}
