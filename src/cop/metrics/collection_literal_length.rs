use crate::cop::shared::node_type::{ARRAY_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus (5592 repos) reported FP=1, FN=0. Standard corpus is 0/0.
///
/// FP=1 from noosfero (vendor/plugins/xss_terminate/lib/html5lib_sanitize.rb:154).
/// Same cross-cutting file-level issue: vendored file that RuboCop does not
/// process but nitrocop does. No cop-level fix needed.
///
/// ## Corpus verification (2026-03-25)
///
/// verify_cop_locations.py: FP 0 fixed / 1 remain, FN 7 fixed / 0 remain.
/// All FN verified fixed. Remaining FP=1: noosfero (vendored plugin,
/// same file as extended corpus investigation). No cop-level fix needed.
pub struct CollectionLiteralLength;

impl Cop for CollectionLiteralLength {
    fn name(&self) -> &'static str {
        "Metrics/CollectionLiteralLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, HASH_NODE, KEYWORD_HASH_NODE, CALL_NODE]
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
        let max = config.get_usize("LengthThreshold", 250);

        // Check ArrayNode
        if let Some(array) = node.as_array_node() {
            let count = array.elements().len();
            if count >= max {
                let loc = array.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Collection literal is too long. [{count}/{max}]"),
                ));
            }
        }

        // Check HashNode
        if let Some(hash) = node.as_hash_node() {
            let count = hash.elements().len();
            if count >= max {
                let loc = hash.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Collection literal is too long. [{count}/{max}]"),
                ));
            }
        }

        // Check KeywordHashNode
        if let Some(hash) = node.as_keyword_hash_node() {
            let count = hash.elements().len();
            if count >= max {
                let loc = hash.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Collection literal is too long. [{count}/{max}]"),
                ));
            }
        }

        // Check Set[...] literal (CallNode with name `[]` and receiver `Set`)
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"[]" {
                if let Some(recv) = call.receiver() {
                    let is_set = recv
                        .as_constant_read_node()
                        .is_some_and(|c| c.name().as_slice() == b"Set")
                        || recv
                            .as_constant_path_node()
                            .is_some_and(|cp| cp.name().is_some_and(|n| n.as_slice() == b"Set"));
                    if is_set {
                        if let Some(args) = call.arguments() {
                            let count = args.arguments().len();
                            if count >= max {
                                let loc = call.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Collection literal is too long. [{count}/{max}]"),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        CollectionLiteralLength,
        "cops/metrics/collection_literal_length",
        large_array = "large_array.rb",
        large_hash = "large_hash.rb",
        larger_array = "larger_array.rb",
        boundary_array = "boundary_array.rb",
        large_set = "large_set.rb",
    );
}
