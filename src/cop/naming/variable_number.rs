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
/// ## Corpus rerun (2026-03-11)
///
/// Local acceptance gate `python3 scripts/check-cop.py Naming/VariableNumber
/// --verbose --rerun` matched RuboCop exactly (0 FP, 0 FN), confirming the
/// oracle's single FN row was stale artifact noise.
///
/// Investigation notes:
/// - The reported `hexapdf` FN points at a hash literal line with no numbered
///   identifiers, so there is no live cop repro to fix.
/// - The local `hexapdf` corpus checkout matches the manifest SHA
///   `f1176c6413df266cd07e27c17aa76e6264bd86dc`, so this row is stale oracle
///   data rather than local repo drift.
/// - The batch gate still prints a large raw delta because
///   `--corpus-check` includes 1,285 file-drop offenses from a `jruby` parser
///   crash repo; the RuboCop-aligned comparison is exact.
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
            if !is_allowed(bare, &allowed_ids, &allowed_pats) {
                if let Some(diag) =
                    check_number_style(self, source, bare, &loc, enforced_style, "variable")
                {
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
                if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                    // For empty-value symbols like :"", value_loc() may return
                    // a zero-length range at an incorrect offset. Use the full
                    // symbol location instead when value_loc has zero length.
                    let loc = match sym.value_loc() {
                        Some(vloc) if !vloc.as_slice().is_empty() => vloc,
                        _ => sym.location(),
                    };
                    if let Some(diag) =
                        check_number_style(self, source, name_str, &loc, enforced_style, "symbol")
                    {
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
) -> Option<Diagnostic> {
    // Find if name contains digits.
    // Empty names (e.g. `:""`  empty-string symbol) are skipped — RuboCop's
    // Parser gem creates dsym (not sym) for these, so on_sym never fires.
    let has_digit = name.bytes().any(|b| b.is_ascii_digit());
    if !has_digit || name.is_empty() {
        return None;
    }

    // Implicit params like _1, _2 are always allowed
    if name.starts_with('_') && name[1..].bytes().all(|b| b.is_ascii_digit()) {
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
    fn empty_string_symbol_is_not_offense() {
        // RuboCop's Parser gem creates dsym (not sym) for empty symbols,
        // so the cop never checks them.
        let diags = crate::testutil::run_cop_full(&VariableNumber, b":\"\"\n");
        assert_eq!(diags.len(), 0);
    }
}
