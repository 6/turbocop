use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10, updated 2026-03-29)
///
/// **Round 1 (FP=4, FN=87):** FPs were `.when(...)` Arel method calls.
/// Fixed by checking for `.` or `&.` before keyword. FNs were missing
/// "space before" checks; added broad before/after detection.
///
/// **Round 2 (FP=634, FN=38):** Massive FPs from text-based scanning
/// hitting non-keyword uses of keyword-named identifiers:
/// - `@case`, `@in`, `@next`, `@end`, `@begin` etc. — instance/class/global
///   variables with keyword names. Fixed by treating `@` and `$` as word-
///   boundary characters in `is_word_before`.
/// - `Pry::rescue`, `Pango::EllipsizeMode::END` — constant-path method calls.
///   Fixed by extending `is_method_call` to detect `::` before keyword.
/// - `:end`, `:begin`, `:rescue` — symbol literals. Added `is_symbol_literal`.
/// - `{ case: 1, end: 2 }` — hash keys. Added `is_hash_key`.
/// - `ensure!`, `next!`, `break?` — method names. Added `!`/`?` suffix check.
/// - `end[0]`, `end.method` — chaining after `end`. RuboCop never checks
///   "space after" for `end`, only "space before". Fixed by skipping
///   space-after check for `end` keyword.
///
/// **Round 3 (FP=16):** FPs from camping's minified Ruby, e.g.
/// `def app_name;"Camping"end`. RuboCop is AST-based and only checks
/// "space before `end`" for specific node types: begin..end, do..end blocks,
/// if/unless/case, and while/until/for with `do`. It does NOT check `end`
/// for def/class/module/singleton-class nodes. Fixed by walking the AST
/// with `EndSkipCollector` to collect `end` positions from those node types
/// and skipping them during text-based scanning.
///
/// **Round 4 (FP=4, FN=41):**
/// - FP: Method names with digits before `in` keyword (e.g. `ft2in`, `yd2in`
///   in prawnpdf/prawn). `is_word_before` now walks backwards past digits to
///   check for a preceding letter/underscore — if found, it's an identifier.
/// - FN: `defined?SafeYAML` — `is_word_end` treated `?S` as continuation of
///   an identifier. Fixed by special-casing keywords ending in `?` as always
///   having a word boundary (since `?` can't appear mid-identifier in Ruby).
/// - FN: `super!=true` — `!` suffix check incorrectly skipped as method name.
///   Fixed by not skipping when `!` is followed by `=` (making it `!=` operator).
/// - FN: `chop!until`, `jruby?do` — `!`/`?` in `accepted_before` was unconditional.
///   Now only accepted when preceded by non-alphanumeric (unary operator context),
///   not when preceded by alphanumeric (method-name suffix context).
/// - FN: `->do` — `>` in `accepted_before` was unconditional. Now checks that
///   `>` is not preceded by `-` (lambda literal `->` vs comparison operator).
/// - FN: `return(1)` — `return` is correctly NOT in `ACCEPT_LEFT_PAREN`,
///   matching RuboCop which flags `return(1)` as "space after missing".
/// - Remaining FN (~41): Most are `if(`, `case(`, `elsif(` patterns in
///   api-umbrella and other repos. The cop correctly detects these in unit
///   tests; remaining FN likely from config resolution or code_map issues
///   in the corpus pipeline.
///
/// **Round 5 (FP=0, FN=36):**
/// - FN: `]:super` in camping's minified Ruby — ternary `condition ? val :super`.
///   `is_symbol_literal` incorrectly treated `:super` as a symbol because it
///   only checked for `::` prefix. Fixed by also rejecting `:` when immediately
///   preceded by `)`, `]`, or `}` (expression-ending delimiters that never
///   precede symbol literals). Fixes 2 camping FN.
/// - Also fixed: offense.rb annotation column for `->do` (was off by 1).
///
/// **Round 6 (FP=0, FN=0):**
/// - FN: All 34 remaining FN (across 7 repos) caused by `is_method_call`
///   crossing newlines into comments. The function skips whitespace (including
///   newlines) to find the preceding token, and would match `.` at the end of
///   a comment sentence (e.g., `# some explanation.` on the previous line).
///   Fixed by passing `code_map` to `is_method_call` and requiring the `.` or
///   `::` to be in code (not inside a comment/string).
///
/// **Round 7 (local follow-up on 2026-03-29):**
/// - `verify_cop_locations.py` showed the earlier 30 FP corpus mismatches were
///   already fixed on this branch; only 2 FN remained.
/// - FN: `do:w` and `when:new_ring` were both suppressed by the text-only
///   `is_hash_key` heuristic, which treated any `keyword:` as a label.
/// - Fixed by replacing that heuristic with AST-collected label-key positions
///   from Prism `AssocNode`s. Real labels like `case:` still skip, but symbol
///   literals after keywords (`when:new_ring`, `do:w`) are now flagged.
/// - FP regression guard: RuboCop accepts `then'foo'` / `then:bar` inside
///   `when` branches because it only checks `then` for `if` nodes. We now skip
///   `WhenNode.then_keyword_loc()` positions while still checking `if ... then`.
///
/// **Round 8 (2026-04-01):**
/// - FP: Prism still exposes several non-keyword source ranges that the
///   text scanner would hit, even though RuboCop never checks them:
///   keyword-named call selectors (`.or(...)`, `.not(...)`) including across
///   comments/interpolation, keyword parameters in method definitions
///   (`if:`, `return:`, `do:`), and post-condition `begin ... end while(...)`
///   loops.
/// - Fixed by broadening AST collection from just hash labels/`when then`/`end`
///   to exact Prism-backed skip sets for those constructs.
///
/// **Round 9 (2026-04-01):**
/// - FN: `before(:each)do`, `RSpec.describe(SomeObject)do`,
///   `Squib::Deck.new(...)do`, `CSV.generate(...)do`, and similar
///   `call(...)do` blocks were being skipped entirely for the missing-space-
///   before check.
/// - Root cause: a Prism visitor treated every `CallNode` with parentheses and
///   a `do` block as a false-positive shape, but RuboCop flags the general
///   pattern (`foo(1)do`) as an offense.
/// - Fixed by removing that skip and keeping only the Prism-backed exclusions
///   RuboCop actually accepts.
pub struct SpaceAroundKeyword;

/// Keywords that accept `(` immediately after them (no space required).
const ACCEPT_LEFT_PAREN: &[&[u8]] = &[
    b"break",
    b"defined?",
    b"next",
    b"not",
    b"rescue",
    b"super",
    b"yield",
];

/// Keywords that accept `[` immediately after them.
const ACCEPT_LEFT_SQUARE_BRACKET: &[&[u8]] = &[b"super", b"yield"];

/// Returns true if the character before position `i` means we should NOT
/// flag "space before missing". Mirrors RuboCop's `space_before_missing?`
/// which returns false for `[\s(|{\[;,*=]`.
/// Characters like `!`, `?`, `.`, `>` need context: they're accepted as
/// operators but not as method-name suffixes (e.g. `chop!until` needs a space).
fn accepted_before(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let ch = bytes[i - 1];
    if matches!(
        ch,
        b' ' | b'\t'
            | b'\n'
            | b'\r'
            | b'('
            | b'|'
            | b'{'
            | b'['
            | b';'
            | b','
            | b'*'
            | b'='
            | b'+'
            | b'-'
            | b'/'
            | b'<'
            | b'&'
    ) {
        return true;
    }
    // `.` is accepted (method call handled by is_method_call, range by Layout/SpaceInsideRangeLiteral)
    if ch == b'.' {
        return true;
    }
    // `>` is accepted only when it's an operator (not preceded by `-` which makes `->`)
    if ch == b'>' {
        // `->do` — lambda literal, NOT accepted
        if i >= 2 && bytes[i - 2] == b'-' {
            return false;
        }
        return true;
    }
    // `!` and `?` are accepted only as unary operators (not preceded by alphanumeric/underscore).
    // `!yield` → accepted (unary not). `chop!until` → not accepted (method suffix).
    if ch == b'!' || ch == b'?' {
        if i >= 2 && (bytes[i - 2].is_ascii_alphanumeric() || bytes[i - 2] == b'_') {
            return false;
        }
        return true;
    }
    false
}

/// Returns true if the char after a keyword means "no space required".
/// Mirrors RuboCop's `space_after_missing?` which returns false for `[\s;,#\\)}\].]`.
fn accepted_after(ch: u8) -> bool {
    matches!(
        ch,
        b' ' | b'\t' | b'\n' | b'\r' | b';' | b',' | b'#' | b'\\' | b')' | b'}' | b']' | b'.'
    )
}

/// Returns true if `kw` is a word boundary — the byte after the keyword is
/// not alphanumeric or underscore (so `ifdef` doesn't match `if`).
/// Keywords ending in `?` (only `defined?`) always have a word boundary
/// because `?` cannot appear mid-identifier in Ruby.
fn is_word_end(bytes: &[u8], kw: &[u8], kw_end: usize) -> bool {
    if kw_end >= bytes.len() {
        return true;
    }
    // `defined?` ends with `?` — always a word boundary regardless of what follows
    if kw.last() == Some(&b'?') {
        return true;
    }
    let ch = bytes[kw_end];
    !(ch.is_ascii_alphanumeric() || ch == b'_')
}

/// Returns true if the byte before position `i` is part of an identifier,
/// meaning this is NOT a keyword boundary.
/// `@case` is an instance variable, `$end` is a global variable, etc.
/// A bare digit before a keyword is allowed: `1and` is parsed as `1 and ...`.
/// But digits within an identifier (e.g. `ft2in`) mean the keyword-like suffix
/// is part of the method name, not a keyword.
fn is_word_before(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    let ch = bytes[i - 1];
    if ch.is_ascii_alphabetic() || ch == b'_' {
        return true;
    }
    // `@case`, `@@end`, `$next` — variable sigils make this a variable name
    if ch == b'@' || ch == b'$' {
        return true;
    }
    // Walk backwards past digits: if there's a letter/underscore before the digits,
    // the whole thing is an identifier (e.g. `ft2in` — `2` is preceded by `t`).
    // A bare digit (e.g. `1and`) is not an identifier boundary.
    if ch.is_ascii_digit() {
        let mut j = i - 1;
        while j > 0 && bytes[j - 1].is_ascii_digit() {
            j -= 1;
        }
        if j > 0 && (bytes[j - 1].is_ascii_alphabetic() || bytes[j - 1] == b'_') {
            return true;
        }
    }
    false
}

impl Cop for SpaceAroundKeyword {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundKeyword"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Collect source offsets where RuboCop's AST semantics differ from
        // raw keyword text scanning.
        let mut collector = KeywordSkipCollector {
            skip_keyword_positions: HashSet::new(),
        };
        collector.visit(&parse_result.node());
        let skip_keyword_positions = collector.skip_keyword_positions;

        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Quick dispatch on first byte to candidate keywords.
            let candidates: &[&[u8]] = match bytes[i] {
                b'a' => &[b"and"],
                b'b' => &[b"begin", b"break"],
                b'c' => &[b"case"],
                b'd' => &[b"defined?", b"do"],
                b'e' => &[b"else", b"elsif", b"end", b"ensure"],
                b'i' => &[b"if", b"in"],
                b'n' => &[b"next", b"not"],
                b'o' => &[b"or"],
                b'r' => &[b"rescue", b"return"],
                b's' => &[b"super"],
                b't' => &[b"then"],
                b'u' => &[b"unless", b"until"],
                b'w' => &[b"when", b"while"],
                b'y' => &[b"yield"],
                b'B' => &[b"BEGIN"],
                b'E' => &[b"END"],
                _ => {
                    i += 1;
                    continue;
                }
            };

            for &kw in candidates {
                let kw_len = kw.len();
                if i + kw_len > len {
                    continue;
                }
                if &bytes[i..i + kw_len] != kw {
                    continue;
                }
                if !is_word_end(bytes, kw, i + kw_len) {
                    continue;
                }
                if is_word_before(bytes, i) {
                    continue;
                }
                if !code_map.is_code(i)
                    && (!code_map.is_heredoc_interpolation(i)
                        || code_map.is_non_code_in_heredoc_interpolation(i))
                {
                    continue;
                }

                // Check if preceded by `.` or `&.` — that makes it a method call, not a keyword
                if is_method_call(bytes, i, code_map) {
                    continue;
                }

                // Check if preceded by `def ` — that's a method definition named after the keyword
                if preceded_by_def(bytes, i) {
                    continue;
                }

                // Check if preceded by `:` — that's a symbol literal (`:end`, `:rescue`)
                // but NOT `::` which is handled by `is_method_call` above
                if is_symbol_literal(bytes, i) {
                    continue;
                }

                // Check if followed by `!` or `?` — method name like `ensure!`, `next?`
                // (but not `defined?` which already includes `?` in the keyword).
                // Don't skip if `!` is followed by `=` (that's `!=` operator, e.g. `super!=`).
                if i + kw_len < len
                    && (bytes[i + kw_len] == b'!'
                        || (kw != b"defined?" && bytes[i + kw_len] == b'?'))
                {
                    let suffix_pos = i + kw_len;
                    let next_after_suffix = if suffix_pos + 1 < len {
                        Some(bytes[suffix_pos + 1])
                    } else {
                        None
                    };
                    if next_after_suffix != Some(b'=') {
                        continue;
                    }
                }

                // Prism gives exact source ranges for keyword-looking tokens that
                // RuboCop never checks as executable keywords.
                if skip_keyword_positions.contains(&i) {
                    continue;
                }

                let kw_str = std::str::from_utf8(kw).unwrap_or("");

                // --- Check "space before missing" ---
                if i > 0 && !accepted_before(bytes, i) {
                    let (line, column) = source.offset_to_line_col(i);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Space before keyword `{kw_str}` is missing."),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: i,
                            end: i,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }

                // --- Check "space after missing" ---
                // RuboCop only checks "space before" for `end` (not "space after"),
                // since chaining after `end` is common: `end.method`, `end[0]`, etc.
                if kw != b"end" && i + kw_len < len {
                    let after = bytes[i + kw_len];
                    let skip_after = accepted_after(after)
                        || (after == b'(' && is_accept_left_paren(kw))
                        || (after == b'[' && is_accept_left_bracket(kw))
                        || (after == b':'
                            && kw == b"super"
                            && i + kw_len + 1 < len
                            && bytes[i + kw_len + 1] == b':')
                        || (after == b'&' && i + kw_len + 1 < len && bytes[i + kw_len + 1] == b'.');

                    if !skip_after {
                        let (line, column) = source.offset_to_line_col(i);
                        let mut diag = self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Space after keyword `{kw_str}` is missing."),
                        );
                        if let Some(ref mut corr) = corrections {
                            corr.push(crate::correction::Correction {
                                start: i + kw_len,
                                end: i + kw_len,
                                replacement: " ".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }
            }
            i += 1;
        }
    }
}

/// Check if the keyword at position `i` is a method call (preceded by `.`, `&.`, or `::`).
/// The `code_map` is used to verify that the preceding `.` or `::` is actual code,
/// not inside a comment (e.g., a sentence ending with a period on the previous line).
fn is_method_call(bytes: &[u8], i: usize, code_map: &CodeMap) -> bool {
    if i == 0 {
        return false;
    }
    // Skip whitespace before the keyword to find the actual preceding token
    let mut j = i - 1;
    while j > 0 && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r')
    {
        j -= 1;
    }
    if bytes[j] == b'.' {
        // Only treat as method call if the `.` is in code (not inside a comment).
        // Comments ending with `.` (sentence periods) would otherwise cause FN.
        return code_map.is_code(j);
    }
    // `Foo::rescue`, `Bar::next` — constant path method calls
    if bytes[j] == b':' && j > 0 && bytes[j - 1] == b':' {
        return code_map.is_code(j);
    }
    false
}

/// Check if the keyword is preceded by `def ` (method definition).
fn preceded_by_def(bytes: &[u8], i: usize) -> bool {
    i >= 4 && &bytes[i - 4..i] == b"def "
}

/// Check if the keyword at position `i` is preceded by `:` making it a symbol literal.
/// Returns true for `:end`, `:rescue`, etc. but NOT for `::rescue` (constant path)
/// and NOT for ternary `:super` (where `:` is the else branch of `? :`).
fn is_symbol_literal(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    if bytes[i - 1] != b':' {
        return false;
    }
    // It's `::` (constant path), not a symbol — handled separately by is_method_call
    if i >= 2 && bytes[i - 2] == b':' {
        return false;
    }
    // Distinguish symbol `:keyword` from ternary `expr:keyword`.
    // When `)`, `]`, or `}` immediately precede `:`, it's a ternary else operator,
    // not a symbol. These closing delimiters always end expressions and never
    // appear before a symbol literal in valid Ruby.
    if i >= 2 && matches!(bytes[i - 2], b')' | b']' | b'}') {
        return false;
    }
    true
}

/// Returns true if this keyword accepts `(` immediately after it.
fn is_accept_left_paren(kw: &[u8]) -> bool {
    ACCEPT_LEFT_PAREN.contains(&kw)
}

/// Returns true if this keyword accepts `[` immediately after it.
fn is_accept_left_bracket(kw: &[u8]) -> bool {
    ACCEPT_LEFT_SQUARE_BRACKET.contains(&kw)
}

/// Collects byte offsets of `end` keywords that RuboCop does NOT check for
/// "space before". RuboCop only checks `end` for: begin..end, do..end blocks,
/// if/unless/case (with 'then' begin_keyword), while/until/for with `do`.
/// It does NOT check `end` for: def, class, module, singleton class.
struct KeywordSkipCollector {
    skip_keyword_positions: HashSet<usize>,
}

impl<'pr> Visit<'pr> for KeywordSkipCollector {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(end_loc) = node.end_keyword_loc() {
            self.skip_keyword_positions.insert(end_loc.start_offset());
        }
        // Continue visiting children (nested defs, etc.)
        ruby_prism::visit_def_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.skip_keyword_positions
            .insert(node.end_keyword_loc().start_offset());
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.skip_keyword_positions
            .insert(node.end_keyword_loc().start_offset());
        ruby_prism::visit_module_node(self, node);
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.skip_keyword_positions
            .insert(node.end_keyword_loc().start_offset());
        ruby_prism::visit_singleton_class_node(self, node);
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        // Label keys like `case:` / `end:` have no opening `:` in Prism, unlike
        // symbol literals after keywords (`when:new_ring`, `do:w`) which do.
        if node.operator_loc().is_none() {
            if let Some(symbol) = node.key().as_symbol_node() {
                if symbol.opening_loc().is_none() {
                    self.skip_keyword_positions
                        .insert(symbol.location().start_offset());
                }
            }
        }
        ruby_prism::visit_assoc_node(self, node);
    }

    fn visit_when_node(&mut self, node: &ruby_prism::WhenNode<'pr>) {
        if let Some(then_loc) = node.then_keyword_loc() {
            self.skip_keyword_positions.insert(then_loc.start_offset());
        }
        ruby_prism::visit_when_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(message_loc) = node.message_loc() {
            self.skip_keyword_positions
                .insert(message_loc.start_offset());
        }

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_optional_keyword_parameter_node(
        &mut self,
        node: &ruby_prism::OptionalKeywordParameterNode<'pr>,
    ) {
        self.skip_keyword_positions
            .insert(node.name_loc().start_offset());
        ruby_prism::visit_optional_keyword_parameter_node(self, node);
    }

    fn visit_required_keyword_parameter_node(
        &mut self,
        node: &ruby_prism::RequiredKeywordParameterNode<'pr>,
    ) {
        self.skip_keyword_positions
            .insert(node.name_loc().start_offset());
        ruby_prism::visit_required_keyword_parameter_node(self, node);
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        if node.is_begin_modifier() {
            self.skip_keyword_positions
                .insert(node.keyword_loc().start_offset());
        }
        ruby_prism::visit_until_node(self, node);
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        if node.is_begin_modifier() {
            self.skip_keyword_positions
                .insert(node.keyword_loc().start_offset());
        }
        ruby_prism::visit_while_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAroundKeyword, "cops/layout/space_around_keyword");
    crate::cop_autocorrect_fixture_tests!(SpaceAroundKeyword, "cops/layout/space_around_keyword");

    #[test]
    fn ternary_colon_super_no_space_detected() {
        // Ternary `]:super` — colon is ternary else, not a symbol prefix.
        // Camping-style minified Ruby: `a==[]?self[m.to_s]:super`
        let input = b"x = a==[]?self[m.to_s]:super\n";
        let diags = crate::testutil::run_cop_full(&SpaceAroundKeyword, input);
        let super_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("super"))
            .collect();
        assert_eq!(
            super_diags.len(),
            1,
            "Expected 1 super offense but got {}: {:?}",
            super_diags.len(),
            super_diags
        );
    }

    #[test]
    fn autocorrect_insert_space() {
        let input = b"if(x)\n  y\nend\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&SpaceAroundKeyword, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"if (x)\n  y\nend\n");
    }
}
