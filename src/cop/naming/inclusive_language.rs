use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InclusiveLanguage;

/// A compiled flagged term ready for matching.
struct FlaggedTerm {
    name: String,
    pattern: String, // lowercase pattern to search for
    whole_word: bool,
    suggestions: Vec<String>,
}

impl Cop for InclusiveLanguage {
    fn name(&self) -> &'static str {
        "Naming/InclusiveLanguage"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let check_identifiers = config.get_bool("CheckIdentifiers", true);
        let check_constants = config.get_bool("CheckConstants", true);
        let check_variables = config.get_bool("CheckVariables", true);
        let check_strings = config.get_bool("CheckStrings", false);
        let check_symbols = config.get_bool("CheckSymbols", true);
        let check_comments = config.get_bool("CheckComments", true);
        let check_filepaths = config.get_bool("CheckFilepaths", true);

        // Build flagged terms from config or use defaults
        let terms = build_flagged_terms(config);
        if terms.is_empty() {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check filepath
        if check_filepaths {
            let path = source.path_str();
            let path_lower = path.to_lowercase();
            for term in &terms {
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

            for term in &terms {
                // Search for the term in the line
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
                        let msg = format_message(&term.name, &term.suggestions);
                        diagnostics.push(self.diagnostic(source, line_num, abs_pos, msg));
                    }

                    search_start = abs_pos + term.pattern.len();
                }
            }
        }

        diagnostics
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
                let pattern;

                if let Some(term_map) = value.as_mapping() {
                    // Check for Regex — we use the term name as a simple substring match
                    // since we can't execute Ruby regexps
                    if let Some(regex_val) = term_map.get(&serde_yml::Value::String("Regex".to_string())) {
                        // Extract the pattern from the regex string, use term name as fallback
                        let regex_str = regex_val.as_str().unwrap_or(&name);
                        // Try to extract a simple substring from the regex
                        pattern = simplify_regex(regex_str, &name);
                    } else {
                        pattern = name.to_lowercase();
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
                } else {
                    pattern = name.to_lowercase();
                }

                terms.push(FlaggedTerm {
                    name,
                    pattern,
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
            whole_word: false,
            suggestions: vec!["allowlist".to_string(), "permit".to_string()],
        },
        FlaggedTerm {
            name: "blacklist".to_string(),
            pattern: "blacklist".to_string(),
            whole_word: false,
            suggestions: vec!["denylist".to_string(), "block".to_string()],
        },
        FlaggedTerm {
            name: "slave".to_string(),
            pattern: "slave".to_string(),
            whole_word: true,
            suggestions: vec![
                "replica".to_string(),
                "secondary".to_string(),
                "follower".to_string(),
            ],
        },
    ]
}

/// Try to extract a simple substring from a Ruby regex pattern.
/// Check if a string contains a flagged term, respecting whole_word setting.
fn find_term(text: &str, term: &FlaggedTerm) -> Option<usize> {
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

fn simplify_regex(_regex_str: &str, fallback: &str) -> String {
    // Common patterns: '/white[-_\s]?list/' -> "whitelist" (approximate)
    // We just use the fallback term name as the search pattern
    fallback.to_lowercase()
}

fn is_whole_word(line: &str, pos: usize, len: usize) -> bool {
    let before_ok = pos == 0 || !line.as_bytes()[pos - 1].is_ascii_alphanumeric();
    let after_pos = pos + len;
    let after_ok = after_pos >= line.len() || !line.as_bytes()[after_pos].is_ascii_alphanumeric();
    before_ok && after_ok
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
}
