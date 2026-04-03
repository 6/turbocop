use crate::cop::shared::node_type::{ASSOC_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for boolean-looking symbols such as `:true`, `:false`, `true:`, and `false:`.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=57.
///
/// FN:
/// - The implementation only handled standalone `SymbolNode`s with a `:` prefix and skipped
///   keyword-hash labels like `true:` / `false:`. In Prism those arrive through `AssocNode`
///   keys with no `=>` operator, so the label form was invisible to this cop.
/// - Rerunning the corpus gate after adding `AssocNode` handling matched RuboCop exactly:
///   expected 763, actual 763, with no potential FP/FN.
pub struct BooleanSymbol;

impl Cop for BooleanSymbol {
    fn name(&self) -> &'static str {
        "Lint/BooleanSymbol"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(symbol_node) = node.as_symbol_node() {
            // Skip %i[] / %I[] array entries, which have no normal symbol opening.
            let Some(opening) = symbol_node.opening_loc() else {
                return;
            };
            if !opening.as_slice().starts_with(b":") {
                return;
            }

            let Some(boolean_name) = boolean_symbol_name(symbol_node.unescaped()) else {
                return;
            };
            add_boolean_symbol_offense(
                self,
                source,
                symbol_node.location(),
                boolean_name,
                diagnostics,
            );
            return;
        }

        let Some(assoc) = node.as_assoc_node() else {
            return;
        };

        // `true:` / `false:` arrive as assoc keys without a rocket operator.
        if assoc.operator_loc().is_some() {
            return;
        }

        let Some(symbol_node) = assoc.key().as_symbol_node() else {
            return;
        };
        let Some(boolean_name) = boolean_symbol_name(symbol_node.unescaped()) else {
            return;
        };
        add_boolean_symbol_offense(
            self,
            source,
            symbol_node.location(),
            boolean_name,
            diagnostics,
        );
    }
}

fn boolean_symbol_name(value: &[u8]) -> Option<&'static str> {
    if value == b"true" {
        Some("true")
    } else if value == b"false" {
        Some("false")
    } else {
        None
    }
}

fn add_boolean_symbol_offense(
    cop: &BooleanSymbol,
    source: &SourceFile,
    loc: ruby_prism::Location<'_>,
    boolean_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!("Symbol with a boolean name - you probably meant to use `{boolean_name}`."),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BooleanSymbol, "cops/lint/boolean_symbol");
}
