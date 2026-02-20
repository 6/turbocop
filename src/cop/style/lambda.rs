use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};

pub struct Lambda;

impl Cop for Lambda {
    fn name(&self) -> &'static str {
        "Style/Lambda"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Only bare `lambda` calls (no receiver)
        if call.receiver().is_some() {
            return;
        }

        if call.name().as_slice() != b"lambda" {
            return;
        }

        let style = config.get_str("EnforcedStyle", "line_count_dependent");

        match style {
            "literal" => {
                // Always flag `lambda` — use `->` instead
                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, "Use the `-> {}` lambda literal syntax for all lambdas.".to_string()));
            }
            "lambda" => {
                // Never flag `lambda` — it's preferred

            }
            _ => {
                // "line_count_dependent" (default): only flag single-line `lambda`
                // Multi-line `lambda do...end` is the preferred style for multi-line.
                // Single-line `lambda { }` should use `-> { }` instead.
                let block = match call.block() {
                    Some(b) => b,
                    None => return,
                };
                let block_node = match block.as_block_node() {
                    Some(bn) => bn,
                    None => return,
                };

                let (start_line, _) = source.offset_to_line_col(block_node.location().start_offset());
                let (end_line, _) = source.offset_to_line_col(block_node.location().end_offset().saturating_sub(1).max(block_node.location().start_offset()));

                if start_line == end_line {
                    // Single-line lambda — flag it
                    let loc = call.message_loc().unwrap_or_else(|| call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, "Use the `-> {}` lambda literal syntax for single-line lambdas.".to_string()));
                } else {
                    // Multi-line lambda — this is correct for `line_count_dependent`

                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(Lambda, "cops/style/lambda");

    #[test]
    fn lambda_with_receiver_is_ignored() {
        let source = b"obj.lambda { |x| x }\n";
        let diags = run_cop_full(&Lambda, source);
        assert!(diags.is_empty());
    }
}
