use crate::cop::node_type::{
    CLASS_VARIABLE_AND_WRITE_NODE, CLASS_VARIABLE_OPERATOR_WRITE_NODE,
    CLASS_VARIABLE_OR_WRITE_NODE, CLASS_VARIABLE_TARGET_NODE, CLASS_VARIABLE_WRITE_NODE, DEF_NODE,
    GLOBAL_VARIABLE_AND_WRITE_NODE, GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
    GLOBAL_VARIABLE_OR_WRITE_NODE, GLOBAL_VARIABLE_TARGET_NODE, GLOBAL_VARIABLE_WRITE_NODE,
    INSTANCE_VARIABLE_AND_WRITE_NODE, INSTANCE_VARIABLE_OPERATOR_WRITE_NODE,
    INSTANCE_VARIABLE_OR_WRITE_NODE, INSTANCE_VARIABLE_TARGET_NODE, INSTANCE_VARIABLE_WRITE_NODE,
    LOCAL_VARIABLE_AND_WRITE_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
    LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_TARGET_NODE, LOCAL_VARIABLE_WRITE_NODE,
    REQUIRED_PARAMETER_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FN=160 investigation: nitrocop only handled simple write nodes (e.g.
/// `LocalVariableWriteNode`) but missed compound assignment variants:
/// or-write (`||=`), and-write (`&&=`), operator-write (`+=`, `-=`, etc.),
/// and multi-assignment target nodes. All 16 missing node types have a
/// `.name()` method returning the variable name, same as the write nodes.
/// Fix: register all 16 additional node types and handle them identically.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=32. All 32 FPs from empty symbols like `:''"`,
/// `:""`  used as hash keys, symbol arguments, etc. Root cause: RuboCop's
/// Parser gem creates `dsym` (dynamic symbol) for empty symbols, not `sym`.
/// The VariableNumber cop only has `on_sym`, NOT `on_dsym`, so RuboCop
/// never checks empty symbols. In Prism, empty symbols are `SymbolNode`
/// with an empty `unescaped()` value, so nitrocop was processing them.
/// Fix: skip empty names early in `check_number_style` (`!has_digit || name.is_empty()`).
///
/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=0, FN=1. No example locations available.
/// The cop handles all variable write/compound-write/target node types,
/// RequiredParameterNode, DefNode (method names), and SymbolNode. RuboCop's
/// `on_arg` covers all parameter types, but optional/keyword/rest/block
/// parameters are not checked in RuboCop's VariableNumber cop (it only has
/// `on_arg`, not `on_optarg`/`on_kwarg`/`on_kwoptarg`/`on_restarg`/`on_blockarg`).
/// FN=1 is likely a corpus artifact (CI file discovery, encoding, or stale cache)
/// given 16,625 matches with 99.99% match rate. Local `check-cop.py --rerun`
/// needed to confirm.
///
/// ## Corpus fix (2026-03-13)
///
/// Corpus oracle reported FN=1 (confirmed fresh, not stale). Root cause:
/// the implicit-param exemption (`_1`, `_2`, etc.) was applied after sigil
/// stripping, so `@_1`, `@@_1`, `$_1` were incorrectly exempted. RuboCop's
/// `\A_\d+\z` implicit_param regex is applied to the FULL name including
/// sigils (`@_1` starts with `@`, not `_`, so it doesn't match). Fix: only
/// apply the implicit-param exemption to bare names (local variables and
/// parameters), not to sigiled variables.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=0, FN=1 on hexapdf test_serializer.rb:101.
/// The offense is on `"":` (empty string hash key). With TargetRubyVersion: 4.0
/// (the corpus baseline), Parser gem treats `"":` as `:sym` (not `:dsym`),
/// causing RuboCop's `on_sym` to fire. The normalcase regex doesn't match
/// empty strings, so it flags them.
///
/// Fix: stop skipping empty names in `check_number_style`, but only for
/// hash-key symbols (no colon-prefix opening in Prism). Standalone empty
/// symbols (`:""`, `:''`) still have `:dsym` in Parser gem and are not
/// checked by RuboCop, so we skip those by checking `opening_loc` for a
/// colon prefix.
///
/// ## Corpus investigation (2026-03-14) — batch 2
///
/// Corpus oracle reported FP=1 on opal/opal `$$` global variable.
/// Root cause: `trim_start_matches('$')` strips BOTH `$` chars from `$$`,
/// leaving empty bare name `""`. The empty name fails the normalcase regex.
/// RuboCop doesn't fire on `$$` because Parser gem handles it differently.
/// Fix: skip variables with empty bare names after sigil stripping.
pub struct VariableNumber;

const DEFAULT_ALLOWED: &[&str] = &[
    "TLS1_1",
    "TLS1_2",
    "capture3",
    "iso8601",
    "rfc1123_date",
    "rfc822",
    "rfc2822",
    "rfc3339",
    "x86_64",
];

impl Cop for VariableNumber {
    fn name(&self) -> &'static str {
        "Naming/VariableNumber"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CLASS_VARIABLE_AND_WRITE_NODE,
            CLASS_VARIABLE_OPERATOR_WRITE_NODE,
            CLASS_VARIABLE_OR_WRITE_NODE,
            CLASS_VARIABLE_TARGET_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            DEF_NODE,
            GLOBAL_VARIABLE_AND_WRITE_NODE,
            GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            GLOBAL_VARIABLE_TARGET_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            INSTANCE_VARIABLE_AND_WRITE_NODE,
            INSTANCE_VARIABLE_OPERATOR_WRITE_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            INSTANCE_VARIABLE_TARGET_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_AND_WRITE_NODE,
            LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_TARGET_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            REQUIRED_PARAMETER_NODE,
            SYMBOL_NODE,
        ]
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
        let enforced_style = config.get_str("EnforcedStyle", "normalcase");
        let check_method_names = config.get_bool("CheckMethodNames", true);
        let check_symbols = config.get_bool("CheckSymbols", true);
        let allowed = config.get_string_array("AllowedIdentifiers");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        let allowed_ids: Vec<String> =
            allowed.unwrap_or_else(|| DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect());

        let allowed_pats: Vec<String> = allowed_patterns.unwrap_or_default();

        // Extract (name_bytes, location) from any variable write/compound-write/target node
        let var_info: Option<(&[u8], ruby_prism::Location<'_>)> =
            // Local variables (no sigil to strip)
            if let Some(n) = node.as_local_variable_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_local_variable_or_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_local_variable_and_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_local_variable_operator_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_local_variable_target_node() {
                Some((n.name().as_slice(), n.location()))
            }
            // Instance variables (strip @)
            else if let Some(n) = node.as_instance_variable_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_instance_variable_or_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_instance_variable_and_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_instance_variable_operator_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_instance_variable_target_node() {
                Some((n.name().as_slice(), n.location()))
            }
            // Class variables (strip @@)
            else if let Some(n) = node.as_class_variable_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_class_variable_or_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_class_variable_and_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_class_variable_operator_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_class_variable_target_node() {
                Some((n.name().as_slice(), n.location()))
            }
            // Global variables (strip $)
            else if let Some(n) = node.as_global_variable_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_global_variable_or_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_global_variable_and_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_global_variable_operator_write_node() {
                Some((n.name().as_slice(), n.name_loc()))
            } else if let Some(n) = node.as_global_variable_target_node() {
                Some((n.name().as_slice(), n.location()))
            } else {
                None
            };

        if let Some((name_bytes, loc)) = var_info {
            let name_str = std::str::from_utf8(name_bytes).unwrap_or("");
            // Strip sigils: @@ for class vars, @ for instance vars, $ for globals
            let bare = name_str.trim_start_matches('@').trim_start_matches('$');
            let is_bare = bare.len() == name_str.len(); // no sigil stripped
            // Skip variables whose entire name IS the sigil (e.g., $$ → bare "").
            // RuboCop's Parser gem doesn't produce gvasgn for $$ in the same way,
            // so these are never checked.
            if bare.is_empty() {
                return;
            }
            if !is_allowed(bare, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(
                    self,
                    source,
                    bare,
                    &loc,
                    enforced_style,
                    "variable",
                    is_bare,
                ) {
                    diagnostics.push(diag);
                }
            }
            return;
        }

        // Check method names (def)
        if check_method_names {
            if let Some(def_node) = node.as_def_node() {
                let name = def_node.name().as_slice();
                let name_str = std::str::from_utf8(name).unwrap_or("");
                if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                    if let Some(diag) = check_number_style(
                        self,
                        source,
                        name_str,
                        &def_node.name_loc(),
                        enforced_style,
                        "method name",
                        true,
                    ) {
                        diagnostics.push(diag);
                    }
                }
            }
        }

        // Check symbols
        if check_symbols {
            if let Some(sym) = node.as_symbol_node() {
                let name = sym.unescaped();
                let name_str = std::str::from_utf8(name).unwrap_or("");
                // Skip standalone empty symbols (:'' and :""). In Parser gem
                // with TargetRubyVersion >= 4.0, these are :dsym (not :sym),
                // so RuboCop's on_sym never fires. Only hash-key empty symbols
                // ("": val) become :sym in Parser 4.0. In Prism, standalone
                // symbols have a colon-prefix opening (`:` or `:`), while
                // hash-key symbols don't.
                if name_str.is_empty() {
                    let is_standalone = sym
                        .opening_loc()
                        .is_some_and(|loc| loc.as_slice().starts_with(b":"));
                    if is_standalone {
                        return;
                    }
                }
                if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                    // For empty-value symbols like :"", value_loc() may return
                    // a zero-length range at an incorrect offset. Use the full
                    // symbol location instead when value_loc has zero length.
                    let loc = match sym.value_loc() {
                        Some(vloc) if !vloc.as_slice().is_empty() => vloc,
                        _ => sym.location(),
                    };
                    if let Some(diag) = check_number_style(
                        self,
                        source,
                        name_str,
                        &loc,
                        enforced_style,
                        "symbol",
                        true,
                    ) {
                        diagnostics.push(diag);
                    }
                }
            }
        }

        // Check method parameters
        if let Some(param) = node.as_required_parameter_node() {
            let name = param.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(
                    self,
                    source,
                    name_str,
                    &param.location(),
                    enforced_style,
                    "variable",
                    true,
                ) {
                    diagnostics.push(diag);
                }
            }
        }
    }
}

fn is_allowed(name: &str, allowed_ids: &[String], allowed_pats: &[String]) -> bool {
    if allowed_ids.iter().any(|a| a == name) {
        return true;
    }
    for pattern in allowed_pats {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(name) {
                return true;
            }
        }
    }
    false
}

fn check_number_style(
    cop: &VariableNumber,
    source: &SourceFile,
    name: &str,
    loc: &ruby_prism::Location<'_>,
    enforced_style: &str,
    identifier_type: &str,
    is_bare_name: bool,
) -> Option<Diagnostic> {
    // Skip names without digits — the style regex always matches non-empty
    // strings ending with a non-digit character. But empty names (e.g. `:""`
    // from `"":` hash key syntax) DON'T match any style regex. With
    // TargetRubyVersion >= 4.0, Parser gem creates :sym for `"":` (instead
    // of :dsym in older versions), so RuboCop's on_sym fires and the regex
    // check fails on the empty string → offense. Prism always creates
    // SymbolNode for these, so we match RuboCop 4.0 behavior by not skipping
    // empty names.
    let has_digit = name.bytes().any(|b| b.is_ascii_digit());
    if !has_digit && !name.is_empty() {
        return None;
    }

    // Implicit params like _1, _2 are always allowed, but only for bare names
    // (local variables, parameters). Instance/class/global variables like @_1
    // are NOT implicit params — RuboCop's regex checks the full name including
    // sigil, so \A_\d+\z won't match @_1.
    if is_bare_name && name.starts_with('_') && name[1..].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    // RuboCop checks the END of the identifier against a format regex.
    // The name is checked INCLUDING trailing `?` or `!` suffixes — these
    // count as non-digit characters that satisfy the \D alternative.
    //
    // normalcase:  /(?:\D|[^_\d]\d+|\A\d+)\z/ — trailing digits must NOT be preceded by _
    // snake_case:  /(?:\D|_\d+|\A\d+)\z/      — trailing digits MUST be preceded by _
    // non_integer: /(\D|\A\d+)\z/              — no trailing digits allowed
    let valid = match enforced_style {
        "normalcase" => is_valid_normalcase(name),
        "snake_case" => is_valid_snake_case(name),
        "non_integer" => is_valid_non_integer(name),
        _ => true,
    };

    if !valid {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        return Some(cop.diagnostic(
            source,
            line,
            column,
            format!("Use {enforced_style} for {identifier_type} numbers."),
        ));
    }

    None
}

/// normalcase: /(?:\D|[^_\d]\d+|\A\d+)\z/
/// Valid if: ends with non-digit, OR ends with digits NOT preceded by _, OR is all digits.
/// Empty names are invalid (regex doesn't match empty string).
fn is_valid_normalcase(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let last = bytes[bytes.len() - 1];
    // Ends with non-digit → OK
    if !last.is_ascii_digit() {
        return true;
    }
    // Ends with digits. Find where the trailing digit run starts.
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    // If trailing digits span the whole string → OK (all digits)
    if i == 0 {
        return true;
    }
    // The character before the trailing digits must NOT be underscore
    bytes[i - 1] != b'_'
}

/// snake_case: /(?:\D|_\d+|\A\d+)\z/
/// Valid if: ends with non-digit, OR ends with digits preceded by _, OR is all digits.
/// Empty names are invalid (regex doesn't match empty string).
fn is_valid_snake_case(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let last = bytes[bytes.len() - 1];
    if !last.is_ascii_digit() {
        return true;
    }
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i == 0 {
        return true;
    }
    // The character before the trailing digits MUST be underscore
    bytes[i - 1] == b'_'
}

/// non_integer: /(\D|\A\d+)\z/
/// Valid if: ends with non-digit, OR is all digits.
/// Empty names are invalid (regex doesn't match empty string).
fn is_valid_non_integer(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let last = bytes[bytes.len() - 1];
    if !last.is_ascii_digit() {
        return true;
    }
    // Only valid if ALL digits
    bytes.iter().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(VariableNumber, "cops/naming/variable_number");

    #[test]
    fn instance_var_implicit_param_name_is_offense() {
        // RuboCop's implicit_param regex (\A_\d+\z) only matches bare _1, not @_1.
        // So @_1 should be flagged as an offense in normalcase.
        let diags = crate::testutil::run_cop_full(&VariableNumber, b"@_1 = 1\n");
        assert_eq!(diags.len(), 1, "expected @_1 to be flagged");
    }

    #[test]
    fn class_var_implicit_param_name_is_offense() {
        let diags = crate::testutil::run_cop_full(&VariableNumber, b"@@_1 = 1\n");
        assert_eq!(diags.len(), 1, "expected @@_1 to be flagged");
    }

    #[test]
    fn global_var_implicit_param_name_is_offense() {
        let diags = crate::testutil::run_cop_full(&VariableNumber, b"$_1 = 1\n");
        assert_eq!(diags.len(), 1, "expected $_1 to be flagged");
    }

    #[test]
    fn local_var_implicit_param_is_no_offense() {
        // Bare _1 is an implicit param and should NOT be flagged
        let diags = crate::testutil::run_cop_full(&VariableNumber, b"_1 = 1\n");
        assert_eq!(diags.len(), 0, "expected _1 to NOT be flagged");
    }

    #[test]
    fn empty_hash_key_symbol_is_offense() {
        // With TargetRubyVersion >= 4.0, hash-key empty symbols ("": val)
        // are :sym in Parser gem, so RuboCop's on_sym fires and the normalcase
        // regex fails on empty strings. Prism creates SymbolNode without
        // colon opening for hash keys.
        let diags = crate::testutil::run_cop_full(&VariableNumber, b"{\"\":1}\n");
        assert_eq!(
            diags.len(),
            1,
            "expected hash-key empty symbol to be flagged"
        );
    }

    #[test]
    fn standalone_empty_symbol_is_no_offense() {
        // Standalone empty symbols (:'' and :"") are :dsym in Parser gem
        // (even with Ruby 4.0), so RuboCop's on_sym never fires.
        let diags = crate::testutil::run_cop_full(&VariableNumber, b":\"\"\n");
        assert_eq!(diags.len(), 0, "standalone :\"\" should NOT be flagged");
        let diags = crate::testutil::run_cop_full(&VariableNumber, b":''\n");
        assert_eq!(diags.len(), 0, "standalone :'' should NOT be flagged");
    }
}
