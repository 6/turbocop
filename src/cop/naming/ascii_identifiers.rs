use crate::cop::shared::node_type::{ALIAS_METHOD_NODE, DEF_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Checks for non-ASCII characters in identifier and constant names.
///
/// ## Investigation (2026-03-08)
/// FP=0, FN=156 in corpus. Root cause: original implementation only checked 3
/// AST node types (ConstantWriteNode, DefNode, LocalVariableWriteNode), missing
/// many identifier occurrences: local variable reads, method calls, parameters,
/// constant reads/paths, etc.
///
/// RuboCop's implementation iterates over lexer tokens (tIDENTIFIER and tCONSTANT),
/// not AST nodes. Switched to a `check_source` approach that scans raw bytes for
/// identifier tokens, skipping non-code regions (strings, comments, regexes) via
/// CodeMap. This matches RuboCop's token-level scanning without requiring Prism's
/// lexer API.
///
/// RuboCop reports the offense at the first contiguous run of non-ASCII characters
/// within the identifier, not at the identifier start.
///
/// ## Investigation (2026-03-08, round 2)
/// FP=29, FN=1. Two root causes:
///
/// FP=29 (BOM handling): All FPs from files starting with UTF-8 BOM (EF BB BF).
/// BOM's lead byte 0xEF satisfies is_ident_start(), causing the scanner to merge
/// the 3 BOM bytes with the following identifier (e.g., `require`), creating a
/// false non-ASCII match. The old exact-match check `ident == [EF,BB,BF]` failed
/// because the identifier was longer than just the BOM. Fix: skip BOM bytes before
/// entering the identifier scanner.
///
/// FN=1 (alias identifiers): `alias new old` in Prism produces AliasMethodNode
/// with SymbolNode children. The CodeMap marks SymbolNodes as non-code, so the
/// check_source scanner skips them. Fix: added check_node for AliasMethodNode
/// to inspect the bare method name symbols directly.
///
/// ## Corpus investigation (2026-03-23) — extended corpus
///
/// Extended corpus reported FP=10 from Pluvie/italian-ruby. All FPs from
/// `alias :non_è_nullo? :esiste?` — explicit `:` symbol notation in alias
/// statements. RuboCop only checks `tIDENTIFIER` and `tCONSTANT` tokens, not
/// `tSYMBOL`. In Prism, `alias :foo :bar` produces SymbolNodes with `:` in
/// `opening_loc`, while `alias foo bar` produces SymbolNodes without opening.
/// Fix: skip alias name nodes that have an `opening_loc` (explicit symbol
/// notation).
///
/// ## Corpus investigation (2026-03-23) — extended corpus, round 2
///
/// FP=2 remaining, both from Pluvie/italian-ruby: method calls `è_un_commento?`
/// and `è_una_stringa?`. Ruby's lexer produces `tFID` tokens for identifiers
/// ending in `?` or `!`, not `tIDENTIFIER`. RuboCop only checks `tIDENTIFIER`
/// and `tCONSTANT`, so these are never flagged. Fix: skip identifiers ending
/// with `?` or `!` in the byte scanner.
///
/// ## Corpus investigation (2026-03-25) — FN=3
///
/// FN=3 from Pluvie/italian-ruby: `def non_è_un?`, `def è_un_commento?`,
/// `def è_una_stringa?`. After `def`, Parser gem enters `expr_fname` state
/// where the method name is tokenized as `tIDENTIFIER` (not `tFID`), even
/// with `?`/`!` suffix. So RuboCop DOES flag method definitions but NOT
/// method calls with `?`/`!` endings. Fix: added DefNode to check_node to
/// inspect method definition names that end with `?`/`!`, which the byte
/// scanner skips.
pub struct AsciiIdentifiers;

impl Cop for AsciiIdentifiers {
    fn name(&self) -> &'static str {
        "Naming/AsciiIdentifiers"
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
        let ascii_constants = config.get_bool("AsciiConstants", true);
        let bytes = &source.content;
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Skip UTF-8 BOM (EF BB BF) wherever it appears. The BOM's lead
            // byte 0xEF satisfies is_ident_start(), so without this skip it
            // merges with the next identifier, creating false non-ASCII matches.
            if i + 2 < len && bytes[i] == 0xEF && bytes[i + 1] == 0xBB && bytes[i + 2] == 0xBF {
                i += 3;
                continue;
            }
            // Skip non-code regions (comments, strings, regexes, symbols)
            if !code_map.is_code(i) {
                i += 1;
                continue;
            }

            // Check if we're at the start of an identifier
            let b = bytes[i];
            if is_ident_start(b) {
                // Skip identifiers preceded by @ (ivar), @@ (cvar), $ (gvar),
                // or : (symbol). RuboCop only checks tIDENTIFIER and tCONSTANT
                // tokens, not tIVAR/tCVAR/tGVAR/tSYMBOL.
                // Check if preceded by @ (ivar/cvar) or $ (gvar).
                // Note: we intentionally don't skip :identifier here because
                // distinguishing symbol : from ternary : is complex. The CodeMap
                // already marks symbol literals as non-code, so :café in symbol
                // context will be skipped by the is_code() check above.
                let is_prefixed = if i > 0 {
                    let prev = bytes[i - 1];
                    prev == b'@' || prev == b'$'
                } else {
                    false
                };

                // Find the end of the identifier
                let start = i;
                i += utf8_char_len(b);
                while i < len && is_ident_continue(bytes[i]) {
                    i += utf8_char_len(bytes[i]);
                }
                // Allow trailing ? or ! on method names
                if i < len && (bytes[i] == b'?' || bytes[i] == b'!') {
                    // But not != (which is an operator)
                    if bytes[i] == b'!' && i + 1 < len && bytes[i + 1] == b'=' {
                        // Don't consume the !
                    } else {
                        i += 1;
                    }
                }

                let ident = &bytes[start..i];

                // Skip prefixed identifiers (ivars, cvars, gvars, symbols)
                if is_prefixed {
                    continue;
                }

                // Skip identifiers ending with ? or ! — these are tFID tokens
                // in Ruby's lexer. RuboCop only checks tIDENTIFIER and tCONSTANT
                // tokens, not tFID. This covers both method calls (è_un_commento?)
                // and method definitions (def è_un_commento?).
                if let Some(&last) = ident.last() {
                    if last == b'?' || last == b'!' {
                        continue;
                    }
                }

                // Check if identifier has non-ASCII characters
                if ident.iter().all(|&b| b.is_ascii()) {
                    continue;
                }

                // Determine if this is a constant (starts with uppercase A-Z)
                let is_constant = bytes[start].is_ascii_uppercase();

                if is_constant && !ascii_constants {
                    continue;
                }

                // Find the first non-ASCII character position (byte offset of
                // the first non-ASCII UTF-8 lead byte in the identifier)
                let first_non_ascii_offset = bytes[start..i]
                    .iter()
                    .enumerate()
                    .find(|&(_, &b)| !b.is_ascii() && (b & 0xC0) != 0x80)
                    .map(|(idx, _)| start + idx)
                    .unwrap_or(start);

                let (line, column) = source.offset_to_line_col(first_non_ascii_offset);
                let message = if is_constant {
                    "Use only ascii symbols in constants."
                } else {
                    "Use only ascii symbols in identifiers."
                };
                diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
            } else if !b.is_ascii() {
                // Skip non-ASCII bytes that aren't part of an identifier
                // (e.g., standalone Unicode operators)
                i += utf8_char_len(b);
            } else {
                i += 1;
            }
        }
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ALIAS_METHOD_NODE, DEF_NODE]
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
        // Handle `def method_with_non_ascii?` — after `def`, the Parser gem
        // tokenizes the method name as tIDENTIFIER (not tFID), even with ?/!
        // suffix. The byte scanner skips ?/! ending identifiers to avoid
        // flagging method calls (which are tFID), so we handle method
        // definitions here via check_node.
        if let Some(def_node) = node.as_def_node() {
            let name_bytes = def_node.name().as_slice();
            // Only handle names ending with ? or ! — names without these
            // suffixes are already caught by the byte scanner in check_source.
            let ends_with_fid_suffix = name_bytes.last().is_some_and(|&b| b == b'?' || b == b'!');
            if !ends_with_fid_suffix {
                return;
            }
            if name_bytes.iter().all(|&b| b.is_ascii()) {
                return;
            }
            let ascii_constants = config.get_bool("AsciiConstants", true);
            let is_constant = name_bytes.first().is_some_and(|b| b.is_ascii_uppercase());
            if is_constant && !ascii_constants {
                return;
            }
            // Find offset of first non-ASCII char in the name location
            let loc = def_node.name_loc();
            let src_bytes = &source.content[loc.start_offset()..loc.end_offset()];
            let first_non_ascii = src_bytes
                .iter()
                .enumerate()
                .find(|&(_, &b)| !b.is_ascii() && (b & 0xC0) != 0x80)
                .map(|(idx, _)| loc.start_offset() + idx)
                .unwrap_or(loc.start_offset());
            let (line, column) = source.offset_to_line_col(first_non_ascii);
            let message = if is_constant {
                "Use only ascii symbols in constants."
            } else {
                "Use only ascii symbols in identifiers."
            };
            diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
            return;
        }

        // Handle `alias new_name old_name` — Prism represents the bare method
        // names as SymbolNodes, which the CodeMap marks as non-code. The
        // check_source scanner skips them, so we catch non-ASCII identifiers
        // in alias arguments here via check_node.
        let alias_node = match node.as_alias_method_node() {
            Some(n) => n,
            None => return,
        };
        let ascii_constants = config.get_bool("AsciiConstants", true);
        for name_node in [alias_node.new_name(), alias_node.old_name()] {
            let sym = match name_node.as_symbol_node() {
                Some(s) => s,
                None => continue,
            };
            // Skip explicit symbol notation (alias :foo :bar). RuboCop only
            // checks tIDENTIFIER/tCONSTANT tokens, not tSYMBOL tokens.
            // In Prism, `alias foo bar` has no opening_loc on the SymbolNode,
            // while `alias :foo :bar` has `:` as opening_loc.
            if sym.opening_loc().is_some() {
                continue;
            }
            let name_bytes = sym.unescaped();
            if name_bytes.iter().all(|&b| b.is_ascii()) {
                continue;
            }
            let is_constant = name_bytes.first().is_some_and(|b| b.is_ascii_uppercase());
            if is_constant && !ascii_constants {
                continue;
            }
            // Find offset of first non-ASCII char in the source location
            let loc = sym.location();
            let src_bytes = &source.content[loc.start_offset()..loc.end_offset()];
            let first_non_ascii = src_bytes
                .iter()
                .enumerate()
                .find(|&(_, &b)| !b.is_ascii() && (b & 0xC0) != 0x80)
                .map(|(idx, _)| loc.start_offset() + idx)
                .unwrap_or(loc.start_offset());
            let (line, column) = source.offset_to_line_col(first_non_ascii);
            let message = if is_constant {
                "Use only ascii symbols in constants."
            } else {
                "Use only ascii symbols in identifiers."
            };
            diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
        }
    }
}

/// Check if a byte can start a Ruby identifier.
/// Ruby identifiers start with [a-zA-Z_] or non-ASCII (multi-byte UTF-8 lead byte).
/// UTF-8 continuation bytes (0x80..0xBF) are excluded.
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || (b >= 0xC0)
}

/// Check if a byte can continue a Ruby identifier.
/// Ruby identifiers continue with [a-zA-Z0-9_] or non-ASCII (including
/// UTF-8 continuation bytes which are part of multi-byte characters).
fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || !b.is_ascii()
}

/// Return the length of a UTF-8 character based on its first byte.
fn utf8_char_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AsciiIdentifiers, "cops/naming/ascii_identifiers");

    #[test]
    fn config_ascii_constants_true_flags_non_ascii_constant() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AsciiConstants".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = "Caf\u{00e9} = 1\n".as_bytes();
        let diags = run_cop_full_with_config(&AsciiIdentifiers, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag non-ASCII constant when AsciiConstants:true"
        );
    }

    #[test]
    fn config_ascii_constants_false_allows_non_ascii_constant() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AsciiConstants".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = "Caf\u{00e9} = 1\n".as_bytes();
        let diags = run_cop_full_with_config(&AsciiIdentifiers, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag non-ASCII constant when AsciiConstants:false"
        );
    }

    #[test]
    fn does_not_flag_non_ascii_in_strings() {
        use crate::testutil::run_cop_full;
        let source = b"x = \"caf\\xC3\\xA9\"\n";
        let diags = run_cop_full(&AsciiIdentifiers, source);
        assert!(diags.is_empty(), "Should not flag non-ASCII in strings");
    }

    #[test]
    fn does_not_flag_non_ascii_in_comments() {
        use crate::testutil::run_cop_full;
        let source = "# café comment\nx = 1\n".as_bytes();
        let diags = run_cop_full(&AsciiIdentifiers, source);
        assert!(diags.is_empty(), "Should not flag non-ASCII in comments");
    }
}
