use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use ruby_prism::Visit;

use crate::cop::node_type::UNDEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle reported FP=6, FN=60.
///
/// FP=6: false positives were concentrated in string literals containing
/// symbol-like text (for example `":whitelist"`). nitrocop previously treated
/// those as symbols and flagged them even with `CheckStrings: false`.
///
/// FN=60: misses were concentrated in predicate/bang method identifiers with
/// flagged terms (for example `whitelisted?`, `blacklisted?`). The previous
/// token heuristic skipped all `?`/`!` suffix identifiers.
///
/// This implementation now uses parse-derived symbol ranges to distinguish real
/// symbol literals from string content, and keeps `?`/`!` skipping only for
/// non-definition contexts.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=12, FN=0.
///
/// **Root cause 1 (6 FPs): fid_token exception too narrow.**
/// `should_flag_code_token` skipped tFID tokens (identifiers ending in `?`/`!`)
/// only when NOT in a method definition. But RuboCop's `preprocess_check_config`
/// does not include `tFID` at all — ALL tFID tokens are skipped regardless of
/// context (method defs, standalone calls, etc.). Fixed by removing the
/// `!is_method_definition_name` exception.
///
/// **Root cause 2 (6 FPs): quoted symbols treated as CheckSymbols.**
/// Symbols like `:"errors.messages.content_type_whitelist_error"` have their
/// content tokenized as `tSTRING_CONTENT` in RuboCop's parser gem, so they
/// follow `CheckStrings` (false by default). nitrocop's `collect_symbol_ranges`
/// was including all symbols with `:` opening, including quoted forms `:"..."`
/// and `:'...'`. Fixed by excluding quoted symbols from symbol ranges — only
/// bare symbols (`:foo`) are now classified under CheckSymbols.
///
/// ## Corpus investigation (2026-03-09)
///
/// Corpus oracle reported FP=3, FN=46.
///
/// **Root cause 1 (3 FPs): string literals inside string interpolation treated as code.**
/// When a flagged term appears inside a string literal nested within `#{}` interpolation
/// (e.g., `params['blacklist']` inside `"...#{params['blacklist']}..."`), the previous
/// code classified the entire interpolation range as code. But RuboCop's parser gem
/// tokenizes nested strings as `tSTRING`/`tSTRING_CONTENT` which follow `CheckStrings`
/// (false by default). Fixed by checking `code_map.is_code()` within interpolation
/// ranges — nested string bytes are correctly classified as non-code by CodeMap.
/// For heredocs, CodeMap marks the entire body as non-code, so `is_heredoc()` is
/// used to preserve correct behavior for code within heredoc interpolation.
///
/// **Root cause 2 (46 FNs): blanket tFID skip included method definitions.**
/// The previous fix to skip ALL identifiers ending in `?`/`!` was too aggressive.
/// In RuboCop's parser gem, instance method definition names (`def foo?`)
/// are tokenized as `tIDENTIFIER` (checked), while method calls (`foo?`, `obj.foo?`)
/// and singleton definitions (`def self.foo?`) are tokenized as `tFID` (not checked).
/// Fixed by making `is_fid_token` context-aware: it now checks for `def`/`alias`
/// context via backward line scanning and only skips `?`/`!` identifiers in call
/// and singleton def contexts.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=3, FN=1.
///
/// **Root cause 1 (3 FPs): `def self.xxx?` singleton method definitions.**
/// In the Parser gem, `def foo?` tokenizes the method name as `tIDENTIFIER` (checked),
/// but `def self.foo?` tokenizes it as `tFID` (NOT checked). This was confirmed via
/// direct Parser gem tokenization tests. The previous investigation incorrectly stated
/// that both forms use `tIDENTIFIER`. The prior hypothesis of "blanket tFID skip causing
/// FN=193" was wrong — the FN=193 was likely from a different baseline or compounding
/// issue, since the Parser gem consistently uses `tFID` for singleton method names.
/// Fixed by making `is_in_instance_def_or_alias_context` return false for `def self.`
/// patterns, so `is_fid_token` correctly skips them.
///
/// **Root cause 2 (1 FN): bare symbol inside string interpolation not classified as symbol.**
/// `:slave_monitor_dsn` inside `"dsn=#{@opts[:slave_monitor_dsn]}"` was missed because
/// CodeMap marks symbol nodes as non-code, but the `classify_match` interpolation path
/// only checked `in_code` for the symbol branch. Since the symbol is non-code within
/// interpolation, it fell through to `check_strings` (false by default) instead of
/// `check_symbols` (true). Fixed by adding a `symbol_ranges` check in the `!in_code`
/// branch of the interpolation path.
///
/// ## Corpus investigation (2026-03-23) — extended corpus
///
/// Extended corpus reported FN=2 from `undef new_slave, new_safe_slave` in ruby/tk.
/// In Prism, `undef foo` creates `UndefNode` with `SymbolNode` children. The CodeMap
/// marks ALL SymbolNodes as non-code, so the `check_source` scanner classifies `undef`
/// method names as symbols/non-code, following `check_strings` (false by default) instead
/// of `check_identifiers` (true). RuboCop tokenizes `undef` arguments as `tIDENTIFIER`,
/// so they follow `check_identifiers`. Fix: added `check_node` handler for `UndefNode`
/// that iterates undef argument names and checks them under `CheckIdentifiers`.
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

#[cfg(test)]
fn clear_terms_cache() {
    TERMS_CACHE.lock().unwrap().clear();
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

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
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

        let symbol_ranges = collect_symbol_ranges(parse_result);
        let interpolation_code_ranges = collect_interpolation_code_ranges(parse_result);

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

        // should_check_code covers identifiers, constants, variables, symbols
        let should_check_code =
            check_identifiers || check_constants || check_variables || check_symbols;

        // Track byte offset for each line start to convert line-relative positions
        // to absolute byte offsets for CodeMap queries.
        let mut line_byte_start: usize = 0;

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1;
            let line_str = String::from_utf8_lossy(line);
            let line_lower = line_str.to_lowercase();

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
                        let byte_offset = line_byte_start + abs_pos;
                        let match_len = mat.end() - mat.start();

                        let should_flag = classify_match(
                            code_map,
                            byte_offset,
                            line,
                            abs_pos,
                            match_len,
                            check_comments,
                            check_strings,
                            check_symbols,
                            should_check_code,
                            &symbol_ranges,
                            &interpolation_code_ranges,
                        );
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
                        let byte_offset = line_byte_start + abs_pos;

                        let should_flag = classify_match(
                            code_map,
                            byte_offset,
                            line,
                            abs_pos,
                            term.pattern.len(),
                            check_comments,
                            check_strings,
                            check_symbols,
                            should_check_code,
                            &symbol_ranges,
                            &interpolation_code_ranges,
                        );

                        if should_flag
                            && (!term.whole_word
                                || is_whole_word(&line_lower, abs_pos, term.pattern.len()))
                        {
                            let msg = format_message(&term.name, &term.suggestions);
                            diagnostics.push(self.diagnostic(source, line_num, abs_pos, msg));
                        }

                        search_start = abs_pos + term.pattern.len();
                    }
                }
            }

            // Advance line_byte_start past this line + newline character
            line_byte_start += line.len() + 1;
        }
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[UNDEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Handle `undef new_slave, new_safe_slave` — Prism represents undef
        // arguments as SymbolNodes, which the CodeMap marks as non-code. But
        // RuboCop tokenizes them as tIDENTIFIER (checked under CheckIdentifiers).
        let check_identifiers = config.get_bool("CheckIdentifiers", true);
        if !check_identifiers {
            return;
        }
        let undef_node = match node.as_undef_node() {
            Some(u) => u,
            None => return,
        };
        let terms = get_or_build_terms(config);
        if terms.is_empty() {
            return;
        }
        for name_node in undef_node.names().iter() {
            let sym = match name_node.as_symbol_node() {
                Some(s) => s,
                None => continue,
            };
            let name_bytes = sym.unescaped();
            let name_str = match std::str::from_utf8(name_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let name_lower = name_str.to_lowercase();
            for term in terms.iter() {
                let matched = if let Some(ref re) = term.regex {
                    re.is_match(&name_lower).unwrap_or(false)
                } else {
                    find_term(&name_lower, term).is_some()
                };
                if matched {
                    let loc = sym.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let msg = format_message(&term.name, &term.suggestions);
                    diagnostics.push(self.diagnostic(source, line, column, msg));
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
                    if let Some(regex_val) =
                        term_map.get(serde_yml::Value::String("Regex".to_string()))
                    {
                        let regex_str = regex_val.as_str().unwrap_or("");
                        if let Some(compiled) = compile_ruby_regex(regex_str) {
                            regex = Some(compiled);
                        }
                        // Keep the name-based pattern as fallback for filepath checks
                    }

                    if let Some(ww) =
                        term_map.get(serde_yml::Value::String("WholeWord".to_string()))
                    {
                        whole_word = ww.as_bool().unwrap_or(false);
                    }

                    if let Some(sugg) =
                        term_map.get(serde_yml::Value::String("Suggestions".to_string()))
                    {
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
    pattern = pattern
        .replace("\\A", "^")
        .replace("\\z", "$")
        .replace("\\Z", "$");

    // Make the regex case-insensitive to match nitrocop's lowercase line matching
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

/// Classify a match at the given byte offset to determine if it should be flagged.
///
/// Maps CodeMap regions to RuboCop's token-based classification:
/// - Code identifiers → should_check_code (except hash labels, which are skipped)
/// - Comments → check_comments
/// - Symbols (`:foo`) → check_symbols
/// - Strings, heredocs, %i/%w, regex → check_strings
#[allow(clippy::too_many_arguments)]
fn classify_match(
    code_map: &CodeMap,
    byte_offset: usize,
    line: &[u8],
    line_pos: usize,
    match_len: usize,
    check_comments: bool,
    check_strings: bool,
    check_symbols: bool,
    should_check_code: bool,
    symbol_ranges: &[(usize, usize)],
    interpolation_code_ranges: &[(usize, usize)],
) -> bool {
    let in_code = code_map.is_code(byte_offset);
    let in_string = !code_map.is_not_string(byte_offset);
    // Not in code and not in string → must be a comment
    let in_comment = !in_code && !in_string;

    if in_comment {
        check_comments
    } else if in_string {
        if in_ranges(interpolation_code_ranges, byte_offset) {
            // RuboCop checks tokens inside string/heredoc interpolation (`#{...}`).
            // But nested string literals within interpolation (e.g., `params['blacklist']`)
            // are tokenized as tSTRING/tSTRING_CONTENT, which follow CheckStrings.
            //
            // For regular interpolated strings, CodeMap marks code within `#{}` as code
            // and nested strings as non-code. For heredocs, CodeMap marks the entire
            // body as non-code, so we use `is_heredoc()` to detect heredoc context
            // where interpolation code should still be treated as code.
            if in_code || code_map.is_heredoc(byte_offset) {
                if in_ranges(symbol_ranges, byte_offset) {
                    check_symbols
                } else {
                    should_flag_code_token(line, line_pos, match_len, should_check_code)
                }
            } else if in_ranges(symbol_ranges, byte_offset) {
                // Bare symbol inside interpolation (e.g., `"#{opts[:slave_dsn]}"`) —
                // CodeMap marks symbols as non-code, but the parser gem tokenizes
                // them as tSYMBOL which follows CheckSymbols.
                check_symbols
            } else {
                // Nested string within interpolation — follows CheckStrings
                check_strings
            }
        } else
        // In string_ranges: could be string, heredoc, regex, %i/%w, or symbol.
        // Symbol literal ranges are checked via parse_result ranges.
        if in_ranges(symbol_ranges, byte_offset) {
            check_symbols
        } else {
            check_strings
        }
    } else {
        // In code — skip hash labels (tLABEL in RuboCop, not checked)
        // and tFID tokens (identifiers ending in ! or ?) except method definitions.
        should_flag_code_token(line, line_pos, match_len, should_check_code)
    }
}

fn should_flag_code_token(
    line: &[u8],
    line_pos: usize,
    match_len: usize,
    should_check_code: bool,
) -> bool {
    if is_hash_label(line, line_pos, match_len) || is_fid_token(line, line_pos) {
        false
    } else {
        should_check_code
    }
}

/// Check if a match at `pos` of length `len` in `line` falls within a hash label.
/// Hash labels are identifier tokens followed by `:` (e.g., `auto_correct:`).
/// RuboCop tokenizes these as tLABEL which is not checked by the cop.
/// The match might be a substring of the label (e.g., `auto_correct` within
/// `safe_auto_correct:`), so we expand outward to find the full identifier.
fn is_hash_label(line: &[u8], pos: usize, _len: usize) -> bool {
    // Expand forward from pos to find the end of the identifier
    let mut end = pos;
    while end < line.len()
        && (line[end].is_ascii_alphanumeric()
            || line[end] == b'_'
            || line[end] == b'?'
            || line[end] == b'!')
    {
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

/// Check if the match at `pos` in `line` is part of a tFID token (function identifier
/// ending in `!` or `?`). RuboCop's parser gem tokenizes these as tFID which is NOT
/// in the cop's check_token? map, so they are silently skipped.
///
/// The parser gem uses tIDENTIFIER (checked) only for instance method definitions
/// (`def foo?`, `def foo!`) and alias contexts. Singleton method definitions
/// (`def self.foo?`) are tokenized as tFID (not checked), same as method calls.
fn is_fid_token(line: &[u8], pos: usize) -> bool {
    // Expand forward to find the end of the identifier
    let mut end = pos;
    while end < line.len() && (line[end].is_ascii_alphanumeric() || line[end] == b'_') {
        end += 1;
    }
    // Check if the identifier is followed by ! or ?
    if end >= line.len() || (line[end] != b'!' && line[end] != b'?') {
        return false;
    }

    // It ends with ?/! — check if it's in a plain `def` context (NOT `def self.`).
    // Only `def foo?` uses tIDENTIFIER (checked). `def self.foo?` uses tFID (not checked).
    if is_in_instance_def_or_alias_context(line, pos) {
        return false;
    }

    true
}

/// Check if the identifier at `pos` is a method name in an instance `def` or `alias` statement.
/// Returns true for `def foo?` and `alias foo? bar?`, but NOT for `def self.foo?`.
///
/// In the Parser gem, `def foo?` tokenizes the name as `tIDENTIFIER` (checked by RuboCop),
/// while `def self.foo?` tokenizes it as `tFID` (not checked). This function distinguishes
/// the two so that only instance method definitions are flagged.
fn is_in_instance_def_or_alias_context(line: &[u8], pos: usize) -> bool {
    // Expand backward to find the start of the full identifier
    let mut start = pos;
    while start > 0
        && (line[start - 1].is_ascii_alphanumeric()
            || line[start - 1] == b'_'
            || line[start - 1] == b'@')
    {
        start -= 1;
    }

    // Now look backward from identifier start, skipping whitespace
    let mut i = start;

    // Check for `def self.` pattern: if we find it, this is a singleton def → return false
    if i > 0 && line[i - 1] == b'.' {
        let mut j = i - 1; // skip '.'
        // Skip `self` or other receiver
        while j > 0
            && (line[j - 1].is_ascii_alphanumeric() || line[j - 1] == b'_' || line[j - 1] == b'@')
        {
            j -= 1;
        }
        // Skip whitespace before receiver
        while j > 0 && line[j - 1] == b' ' {
            j -= 1;
        }

        // Check if preceded by `def` keyword
        if j >= 3 && &line[j - 3..j] == b"def" && (j == 3 || !line[j - 4].is_ascii_alphanumeric()) {
            // This is `def self.foo?` (or `def SomeClass.foo?`) — tFID, not checked
            return false;
        }
    }

    // Check for plain `def foo?` (instance method — tIDENTIFIER, checked)
    while i > 0 && line[i - 1] == b' ' {
        i -= 1;
    }
    if i >= 3 && &line[i - 3..i] == b"def" && (i == 3 || !line[i - 4].is_ascii_alphanumeric()) {
        return true;
    }

    // Check if preceded by `alias` keyword (reset to before identifier)
    let mut k = start;
    while k > 0 && line[k - 1] == b' ' {
        k -= 1;
    }
    // The previous token in an alias is another identifier (the new name).
    // Skip it and any whitespace before it.
    while k > 0
        && (line[k - 1].is_ascii_alphanumeric()
            || line[k - 1] == b'_'
            || line[k - 1] == b'?'
            || line[k - 1] == b'!'
            || line[k - 1] == b'=')
    {
        k -= 1;
    }
    while k > 0 && line[k - 1] == b' ' {
        k -= 1;
    }
    if k >= 5 && &line[k - 5..k] == b"alias" && (k == 5 || !line[k - 6].is_ascii_alphanumeric()) {
        return true;
    }

    false
}

fn collect_symbol_ranges(parse_result: &ruby_prism::ParseResult<'_>) -> Vec<(usize, usize)> {
    struct SymbolRangeCollector {
        ranges: Vec<(usize, usize)>,
    }

    impl<'pr> Visit<'pr> for SymbolRangeCollector {
        fn visit_symbol_node(&mut self, node: &ruby_prism::SymbolNode<'pr>) {
            if let Some(open) = node.opening_loc() {
                let slice = open.as_slice();
                // Only include bare symbols (`:foo`), not quoted symbols (`:"foo"`, `:'foo'`).
                // RuboCop's parser tokenizes quoted symbol content as tSTRING_CONTENT,
                // which follows CheckStrings (false by default), not CheckSymbols.
                if slice.starts_with(b":")
                    && !slice.starts_with(b":\"")
                    && !slice.starts_with(b":'")
                {
                    let loc = node.location();
                    self.ranges.push((loc.start_offset(), loc.end_offset()));
                }
            }
        }

        fn visit_interpolated_symbol_node(
            &mut self,
            node: &ruby_prism::InterpolatedSymbolNode<'pr>,
        ) {
            if let Some(open) = node.opening_loc() {
                let slice = open.as_slice();
                // Same logic: skip quoted symbols (`:"..."` uses tSTRING_CONTENT in parser gem)
                if slice.starts_with(b":")
                    && !slice.starts_with(b":\"")
                    && !slice.starts_with(b":'")
                {
                    let loc = node.location();
                    self.ranges.push((loc.start_offset(), loc.end_offset()));
                }
            }
            ruby_prism::visit_interpolated_symbol_node(self, node);
        }

        fn visit_alias_method_node(&mut self, node: &ruby_prism::AliasMethodNode<'pr>) {
            // `alias foo bar` names are symbols without a leading `:`. RuboCop
            // still checks these under CheckSymbols.
            if let Some(sym) = node.new_name().as_symbol_node() {
                let loc = sym.location();
                self.ranges.push((loc.start_offset(), loc.end_offset()));
            }
            if let Some(sym) = node.old_name().as_symbol_node() {
                let loc = sym.location();
                self.ranges.push((loc.start_offset(), loc.end_offset()));
            }
            ruby_prism::visit_alias_method_node(self, node);
        }
    }

    let mut collector = SymbolRangeCollector { ranges: Vec::new() };
    collector.visit(&parse_result.node());
    collector.ranges.sort_unstable();
    merge_ranges(collector.ranges)
}

fn collect_interpolation_code_ranges(
    parse_result: &ruby_prism::ParseResult<'_>,
) -> Vec<(usize, usize)> {
    struct InterpolationCollector {
        ranges: Vec<(usize, usize)>,
    }

    impl<'pr> Visit<'pr> for InterpolationCollector {
        fn visit_embedded_statements_node(
            &mut self,
            node: &ruby_prism::EmbeddedStatementsNode<'pr>,
        ) {
            let loc = node.location();
            self.ranges.push((loc.start_offset(), loc.end_offset()));
            ruby_prism::visit_embedded_statements_node(self, node);
        }

        fn visit_embedded_variable_node(&mut self, node: &ruby_prism::EmbeddedVariableNode<'pr>) {
            let loc = node.location();
            self.ranges.push((loc.start_offset(), loc.end_offset()));
            ruby_prism::visit_embedded_variable_node(self, node);
        }
    }

    let mut collector = InterpolationCollector { ranges: Vec::new() };
    collector.visit(&parse_result.node());
    collector.ranges.sort_unstable();
    merge_ranges(collector.ranges)
}

fn merge_ranges(sorted: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in sorted {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

fn in_ranges(ranges: &[(usize, usize)], offset: usize) -> bool {
    ranges
        .binary_search_by(|&(start, end)| {
            if offset < start {
                std::cmp::Ordering::Greater
            } else if offset >= end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
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
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(InclusiveLanguage, "cops/naming/inclusive_language");

    #[test]
    fn regex_term_only_matches_at_start_of_line() {
        clear_terms_cache();
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
        let diags = run_cop_full_with_config(
            &InclusiveLanguage,
            b"accept all the things\n",
            config.clone(),
        );
        assert_eq!(diags.len(), 1, "Should flag 'accept ' at start of line");

        // "accept" in middle of line — should NOT match (regex has \\A anchor)
        let diags2 = run_cop_full_with_config(&InclusiveLanguage, b"we accept the terms\n", config);
        assert!(
            diags2.is_empty(),
            "Should NOT flag 'accept' in middle of line with \\A regex"
        );
    }

    #[test]
    fn regex_with_negative_lookahead() {
        clear_terms_cache();
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
        let diags = run_cop_full_with_config(
            &InclusiveLanguage,
            b"it registers offense when called\n",
            config.clone(),
        );
        assert_eq!(
            diags.len(),
            1,
            "Should flag 'registers offense' without exclusion suffix"
        );

        // "registers offenses" — should NOT match (negative lookahead excludes 's')
        let diags2 = run_cop_full_with_config(
            &InclusiveLanguage,
            b"it registers offenses when called\n",
            config,
        );
        assert!(
            diags2.is_empty(),
            "Should NOT flag 'registers offenses' (excluded by lookahead)"
        );
    }
}
