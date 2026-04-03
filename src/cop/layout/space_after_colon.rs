use crate::cop::shared::node_type::{
    ASSOC_NODE, IMPLICIT_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10, updated 2026-03-14)
///
/// CI baseline reported FP=0, FN=33. 20 of 33 FNs were from BubbleWrap
/// (a RubyMotion project) using Objective-C-style method signatures like
/// `def locationManager(manager, didUpdateLocations:locations)`. The remaining
/// 13 FNs were similar missing-space-after-colon in keyword parameter defaults.
///
/// Root cause: the cop only handled `AssocNode` (hash pairs via `on_pair`)
/// but not `OptionalKeywordParameterNode` (RuboCop's `on_kwoptarg`).
///
/// Fix: added `OptionalKeywordParameterNode` to `interested_node_types` and
/// check for missing space after the colon in the `name_loc` (which includes
/// the trailing colon, e.g. `"b:"`). The previous attempt (2026-03-10) was
/// reverted due to FPs, but that was caused by incorrect offset calculation,
/// not by the approach itself.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=4. All 4 FNs from quoted-symbol hash keys
/// (`"return_to":"/"`, `"message":"ok"`). Prism parses these as `SymbolNode`
/// with `closing_loc` of `":` (quote + colon, 2 bytes) rather than just `:`
/// (1 byte) as with bare symbol keys. The original check `loc.as_slice() == b":"`
/// missed the quoted form. Fixed by using `ends_with(b":")` and computing the
/// colon offset as `end_offset() - 1` for accurate diagnostic column reporting.
pub struct SpaceAfterColon;

impl Cop for SpaceAfterColon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterColon"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            IMPLICIT_NODE,
            OPTIONAL_KEYWORD_PARAMETER_NODE,
            SYMBOL_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Handle optional keyword parameter nodes: def f(b:2)
        // RuboCop's on_kwoptarg checks for space after the colon in keyword args.
        if let Some(kwopt) = node.as_optional_keyword_parameter_node() {
            let name_loc = kwopt.name_loc();
            // name_loc covers the name including trailing colon (e.g. "b:")
            let colon_offset = name_loc.end_offset() - 1;
            let after_colon = name_loc.end_offset();
            let bytes = source.as_bytes();
            match bytes.get(after_colon) {
                Some(b) if b.is_ascii_whitespace() => {}
                _ => {
                    let (line, column) = source.offset_to_line_col(colon_offset);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing after colon.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: after_colon,
                            end: after_colon,
                            replacement: " ".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            return;
        }

        let assoc = match node.as_assoc_node() {
            Some(a) => a,
            None => return,
        };

        // Skip value-omission shorthand hash syntax (Ruby 3.1+): { url:, driver: }
        // In Prism, when value is omitted, the value node is an ImplicitNode.
        if assoc.value().as_implicit_node().is_some() {
            return;
        }

        let key = assoc.key();

        // Determine the colon location. Two cases:
        // 1. Symbol shorthand key (`foo: value`): colon is the symbol's closing_loc
        // 2. String key with colon syntax (`"foo": value`): colon is the assoc's operator_loc
        let colon_loc = if let Some(sym) = key.as_symbol_node() {
            match sym.closing_loc() {
                // Plain symbol key: closing_loc is ":" for `foo: value`.
                // Quoted symbol key: closing_loc is "\":" for `"return_to": value`.
                // In both cases the colon is the last byte of closing_loc.
                Some(loc) if loc.as_slice().ends_with(b":") => loc,
                _ => return,
            }
        } else if let Some(op_loc) = assoc.operator_loc() {
            if op_loc.as_slice() == b":" {
                op_loc
            } else {
                return;
            }
        } else {
            return;
        };

        let bytes = source.as_bytes();
        let after_colon = colon_loc.end_offset();
        // The colon is always the last byte of colon_loc. For quoted keys like
        // `"foo":`, the loc is `"\":` so the colon is at end_offset - 1.
        let colon_byte_offset = colon_loc.end_offset() - 1;
        // RuboCop accepts any whitespace after colon (space, newline, tab)
        match bytes.get(after_colon) {
            Some(b) if b.is_ascii_whitespace() => {}
            _ => {
                let (line, column) = source.offset_to_line_col(colon_byte_offset);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Space missing after colon.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: after_colon,
                        end: after_colon,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterColon, "cops/layout/space_after_colon");
    crate::cop_autocorrect_fixture_tests!(SpaceAfterColon, "cops/layout/space_after_colon");
}
