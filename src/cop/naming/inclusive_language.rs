use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InclusiveLanguage;

/// Global cache of compiled flagged terms, keyed by CopConfig pointer.
/// Since base configs are long-lived (entire lint run), the pointers are stable.
/// This avoids recompiling fancy_regex patterns for every file (~1.3s savings on rubocop repo).
static TERMS_CACHE: LazyLock<Mutex<HashMap<usize, Arc<Vec<FlaggedTerm>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_or_build_terms(config: &CopConfig) -> Arc<Vec<FlaggedTerm>> {
    let key = config as *const CopConfig as usize;
    let mut cache = TERMS_CACHE.lock().unwrap();
    if let Some(terms) = cache.get(&key) {
        return Arc::clone(terms);
    }
    let terms = Arc::new(build_flagged_terms(config));
    cache.insert(key, Arc::clone(&terms));
    terms
}

/// A compiled flagged term ready for matching.
struct FlaggedTerm {
    name: String,
    /// Plain substring to search for (lowercase). Used when no regex is set.
    pattern: String,
    /// Compiled regex from the `Regex` config key. When set, this is used
    /// instead of the plain `pattern` for matching. Uses fancy-regex to
    /// support lookahead/lookbehind from Ruby regexes.
    regex: Option<fancy_regex::Regex>,
    whole_word: bool,
    suggestions: Vec<String>,
}

impl Cop for InclusiveLanguage {
    fn name(&self) -> &'static str {
        "Naming/InclusiveLanguage"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        let check_identifiers = config.get_bool("CheckIdentifiers", true);
        let check_constants = config.get_bool("CheckConstants", true);
        let check_variables = config.get_bool("CheckVariables", true);
        let check_strings = config.get_bool("CheckStrings", false);
        let check_symbols = config.get_bool("CheckSymbols", true);
        let check_comments = config.get_bool("CheckComments", true);
        let check_filepaths = config.get_bool("CheckFilepaths", true);

        // Build flagged terms from config or use defaults (cached per config pointer)
        let terms = get_or_build_terms(config);
        if terms.is_empty() {
            return;
        }


        // Check filepath
        if check_filepaths {
            let path = source.path_str();
            let path_lower = path.to_lowercase();
            for term in terms.iter() {
                if let Some(_pos) = find_term(&path_lower, term) {
                    let msg = format_message(&term.name, &term.suggestions);
                    diagnostics.push(self.diagnostic(source, 1, 0, msg));
                }
            }
        }

        // Check each line
        let should_check_code = check_identifiers || check_constants || check_variables
            || check_strings || check_symbols;

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1;
            let line_str = String::from_utf8_lossy(line);
            let line_lower = line_str.to_lowercase();

            // Check if line has a comment portion
            let comment_start = find_comment_start(line);

            for term in terms.iter() {
                // Use regex matching if available, otherwise substring search
                if let Some(ref re) = term.regex {
                    // fancy_regex::find_iter returns Result items
                    for mat_result in re.find_iter(&line_lower) {
                        let mat: fancy_regex::Match = match mat_result {
                            Ok(m) => m,
                            Err(_) => break,
                        };
                        let abs_pos = mat.start();
                        let in_comment = comment_start.is_some_and(|cs| abs_pos >= cs);
                        let should_flag = if in_comment {
                            check_comments
                        } else {
                            should_check_code
                        };
                        if should_flag {
                            let msg = format_message(&term.name, &term.suggestions);
                            diagnostics.push(self.diagnostic(source, line_num, abs_pos, msg));
                        }
                    }
                } else {
                    // Plain substring search
                    let mut search_start = 0;
                    while let Some(pos) = line_lower[search_start..].find(&term.pattern) {
                        let abs_pos = search_start + pos;

                        let in_comment = comment_start.is_some_and(|cs| abs_pos >= cs);

                        let should_flag = if in_comment {
                            check_comments
                        } else {
                            should_check_code
                        };

                        if should_flag && (!term.whole_word || is_whole_word(&line_lower, abs_pos, term.pattern.len())) {
                            // Skip hash label syntax (e.g., `auto_correct:`).
                            // RuboCop's token-based detection uses tLABEL for these,
                            // which is not in its check_token mapping.
                            if !in_comment && is_hash_label(line, abs_pos, term.pattern.len()) {
                                search_start = abs_pos + term.pattern.len();
                                continue;
                            }
                            let msg = format_message(&term.name, &term.suggestions);
                            diagnostics.push(self.diagnostic(source, line_num, abs_pos, msg));
                        }

                        search_start = abs_pos + term.pattern.len();
                    }
                }
            }
        }

    }
}

fn build_flagged_terms(config: &CopConfig) -> Vec<FlaggedTerm> {
    // Try to read FlaggedTerms from config
    if let Some(val) = config.options.get("FlaggedTerms") {
        if let Some(mapping) = val.as_mapping() {
            let mut terms = Vec::new();
            for (key, value) in mapping.iter() {
                let name = match key.as_str() {
                    Some(s) => s.to_string(),
                    None => continue,
                };

                let mut whole_word = false;
                let mut suggestions = Vec::new();
                let pattern = name.to_lowercase();
                let mut regex = None;

                if let Some(term_map) = value.as_mapping() {
                    // Check for Regex key — compile as actual regex for matching
                    if let Some(regex_val) = term_map.get(&serde_yml::Value::String("Regex".to_string())) {
                        let regex_str = regex_val.as_str().unwrap_or("");
                        if let Some(compiled) = compile_ruby_regex(regex_str) {
                            regex = Some(compiled);
                        }
                        // Keep the name-based pattern as fallback for filepath checks
                    }

                    if let Some(ww) = term_map.get(&serde_yml::Value::String("WholeWord".to_string())) {
                        whole_word = ww.as_bool().unwrap_or(false);
                    }

                    if let Some(sugg) = term_map.get(&serde_yml::Value::String("Suggestions".to_string())) {
                        if let Some(seq) = sugg.as_sequence() {
                            for item in seq {
                                if let Some(s) = item.as_str() {
                                    suggestions.push(s.to_string());
                                }
                            }
                        }
                    }
                }

                // If we have a regex, we don't need whole_word (regex handles boundaries)
                if regex.is_some() {
                    whole_word = false;
                }

                terms.push(FlaggedTerm {
                    name,
                    pattern,
                    regex,
                    whole_word,
                    suggestions,
                });
            }
            return terms;
        }
    }

    // Default terms
    vec![
        FlaggedTerm {
            name: "whitelist".to_string(),
            pattern: "whitelist".to_string(),
            regex: None,
            whole_word: false,
            suggestions: vec!["allowlist".to_string(), "permit".to_string()],
        },
        FlaggedTerm {
            name: "blacklist".to_string(),
            pattern: "blacklist".to_string(),
            regex: None,
            whole_word: false,
            suggestions: vec!["denylist".to_string(), "block".to_string()],
        },
        FlaggedTerm {
            name: "slave".to_string(),
            pattern: "slave".to_string(),
            regex: None,
            whole_word: true,
            suggestions: vec![
                "replica".to_string(),
                "secondary".to_string(),
                "follower".to_string(),
            ],
        },
    ]
}

/// Compile a Ruby regex string (e.g., `/\Aaccept /` or `registers offense(?!\(|s)`)
/// into a Rust `regex::Regex`. Handles Ruby-specific syntax:
/// - Strips surrounding `/` delimiters
/// - Converts `\A` (Ruby start-of-string) to `^` (start-of-line, since we match per-line)
/// - Converts `\z` / `\Z` (Ruby end-of-string) to `$`
fn compile_ruby_regex(ruby_str: &str) -> Option<fancy_regex::Regex> {
    let mut pattern = ruby_str.trim().to_string();
    if pattern.is_empty() {
        return None;
    }

    // Strip surrounding / delimiters (and optional flags like /i)
    if pattern.starts_with('/') {
        pattern.remove(0);
        // Remove trailing / and any flags
        if let Some(last_slash) = pattern.rfind('/') {
            pattern.truncate(last_slash);
        }
    }

    if pattern.is_empty() {
        return None;
    }

    // Convert Ruby regex anchors to Rust equivalents
    // \A → ^ (start of string → start of line, since we match per-line)
    // \z / \Z → $ (end of string → end of line)
    pattern = pattern.replace("\\A", "^").replace("\\z", "$").replace("\\Z", "$");

    // Make the regex case-insensitive to match turbocop's lowercase line matching
    let case_insensitive = format!("(?i){pattern}");
    fancy_regex::Regex::new(&case_insensitive).ok()
}

/// Check if a string contains a flagged term, respecting whole_word setting and regex.
fn find_term(text: &str, term: &FlaggedTerm) -> Option<usize> {
    if let Some(ref re) = term.regex {
        return re.find(text).ok().flatten().map(|m| m.start());
    }
    let mut start = 0;
    while let Some(pos) = text[start..].find(&term.pattern) {
        let abs = start + pos;
        if !term.whole_word || is_whole_word(text, abs, term.pattern.len()) {
            return Some(abs);
        }
        start = abs + term.pattern.len();
    }
    None
}

fn is_whole_word(line: &str, pos: usize, len: usize) -> bool {
    let before_ok = pos == 0 || !line.as_bytes()[pos - 1].is_ascii_alphanumeric();
    let after_pos = pos + len;
    let after_ok = after_pos >= line.len() || !line.as_bytes()[after_pos].is_ascii_alphanumeric();
    before_ok && after_ok
}

/// Check if a match at `pos` of length `len` in `line` falls within a hash label.
/// Hash labels are identifier tokens followed by `:` (e.g., `auto_correct:`).
/// RuboCop tokenizes these as tLABEL which is not checked by the cop.
/// The match might be a substring of the label (e.g., `auto_correct` within
/// `safe_auto_correct:`), so we expand outward to find the full identifier.
fn is_hash_label(line: &[u8], pos: usize, _len: usize) -> bool {
    // Expand forward from pos to find the end of the identifier
    let mut end = pos;
    while end < line.len() && (line[end].is_ascii_alphanumeric() || line[end] == b'_' || line[end] == b'?' || line[end] == b'!') {
        end += 1;
    }
    // Check if the identifier is followed by `:` (label syntax)
    if end >= line.len() || line[end] != b':' {
        return false;
    }
    // Must not be followed by another `:` (would be `::` constant path)
    let after_colon = end + 1;
    if after_colon < line.len() && line[after_colon] == b':' {
        return false;
    }
    true
}

fn find_comment_start(line: &[u8]) -> Option<usize> {
    // Simple heuristic: find first # that's not inside a string
    let mut in_single = false;
    let mut in_double = false;
    for (i, &b) in line.iter().enumerate() {
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'#' if !in_single && !in_double => return Some(i),
            _ => {}
        }
    }
    None
}

fn format_message(term: &str, suggestions: &[String]) -> String {
    if suggestions.is_empty() {
        format!("Use inclusive language instead of `{term}`.")
    } else if suggestions.len() == 1 {
        format!(
            "Use inclusive language instead of `{term}`. Suggested alternative: `{}`.",
            suggestions[0]
        )
    } else {
        let alts = suggestions
            .iter()
            .map(|s| format!("`{s}`"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Use inclusive language instead of `{term}`. Suggested alternatives: {alts}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(InclusiveLanguage, "cops/naming/inclusive_language");

    #[test]
    fn regex_term_only_matches_at_start_of_line() {
        let mut flagged = serde_yml::Mapping::new();
        let mut accept_map = serde_yml::Mapping::new();
        accept_map.insert(
            serde_yml::Value::String("Regex".into()),
            serde_yml::Value::String("/\\Aaccept /".into()),
        );
        let mut suggestions = Vec::new();
        suggestions.push(serde_yml::Value::String("accepts ".into()));
        accept_map.insert(
            serde_yml::Value::String("Suggestions".into()),
            serde_yml::Value::Sequence(suggestions),
        );
        flagged.insert(
            serde_yml::Value::String("accept".into()),
            serde_yml::Value::Mapping(accept_map),
        );

        let config = CopConfig {
            options: HashMap::from([
                ("FlaggedTerms".into(), serde_yml::Value::Mapping(flagged)),
                ("CheckStrings".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };

        // "accept " at start of line — should match
        let source = SourceFile::from_bytes("test.rb", b"accept all the things\n".to_vec());
        let mut diags = Vec::new();
        InclusiveLanguage.check_lines(&source, &config, &mut diags);
        assert_eq!(diags.len(), 1, "Should flag 'accept ' at start of line");

        // "accept" in middle of line — should NOT match (regex has \\A anchor)
        let source2 = SourceFile::from_bytes("test.rb", b"we accept the terms\n".to_vec());
        let mut diags2 = Vec::new();
        InclusiveLanguage.check_lines(&source2, &config, &mut diags2);
        assert!(diags2.is_empty(), "Should NOT flag 'accept' in middle of line with \\A regex");
    }

    #[test]
    fn regex_with_negative_lookahead() {
        let mut flagged = serde_yml::Mapping::new();
        let mut term_map = serde_yml::Mapping::new();
        term_map.insert(
            serde_yml::Value::String("Regex".into()),
            serde_yml::Value::String("/registers offense(?!\\(|s)/".into()),
        );
        let mut suggestions = Vec::new();
        suggestions.push(serde_yml::Value::String("registers an offense".into()));
        term_map.insert(
            serde_yml::Value::String("Suggestions".into()),
            serde_yml::Value::Sequence(suggestions),
        );
        flagged.insert(
            serde_yml::Value::String("registers offense".into()),
            serde_yml::Value::Mapping(term_map),
        );

        let config = CopConfig {
            options: HashMap::from([
                ("FlaggedTerms".into(), serde_yml::Value::Mapping(flagged)),
                ("CheckStrings".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };

        // "registers offense" without ( or s — should match
        let source = SourceFile::from_bytes("test.rb", b"it registers offense when called\n".to_vec());
        let mut diags = Vec::new();
        InclusiveLanguage.check_lines(&source, &config, &mut diags);
        assert_eq!(diags.len(), 1, "Should flag 'registers offense' without exclusion suffix");

        // "registers offenses" — should NOT match (negative lookahead excludes 's')
        let source2 = SourceFile::from_bytes("test.rb", b"it registers offenses when called\n".to_vec());
        let mut diags2 = Vec::new();
        InclusiveLanguage.check_lines(&source2, &config, &mut diags2);
        assert!(diags2.is_empty(), "Should NOT flag 'registers offenses' (excluded by lookahead)");
    }
}
