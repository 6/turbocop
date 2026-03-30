/// Style/SingleLineDoEndBlock
///
/// Checks for single-line `do`...`end` blocks and suggests converting them
/// to multiline form.
///
/// ## Investigation (2026-03-15)
///
/// Root cause of ~382 FP and ~384 FN: nitrocop was reporting the offense at the
/// `do` keyword location (column of `do`), but RuboCop reports at the start of
/// the entire expression (the CallNode, column 0 for `foo do end`). Since corpus
/// comparison matches on line:column, same-line offenses at different columns
/// appeared as both FP (nitrocop-only at `do` column) and FN (RuboCop-only at
/// call column). Also, the message was wrong ("Prefer braces" vs "Prefer multiline").
///
/// Fix: dispatch on CALL_NODE (for `foo do...end`, `lambda do...end`) and
/// LAMBDA_NODE (for `-> do...end`) to get the full expression location.
/// Report at the CallNode/LambdaNode start, matching RuboCop's `add_offense(node)`.
///
/// ## Investigation (2026-03-30)
///
/// Remaining FN clusters were multiline receiver/argument chains whose final
/// `do`...`end` stayed on one physical line, plus `super do...end` inside `{}`.
/// RuboCop still reports at the full invocation start, but its single-line
/// check is based on the block delimiters, not the whole enclosing expression
/// span. The previous CallNode-based check used the full call range for both,
/// so earlier line breaks suppressed legitimate offenses, and it never handled
/// `super`/`zsuper` block forms.
///
/// Fix: keep reporting from the invocation node (`CallNode`, `SuperNode`,
/// `ForwardingSuperNode`, `LambdaNode`) while deciding single-line status from
/// the attached block's `do` and `end` delimiter lines.
use crate::cop::node_type::{CALL_NODE, FORWARDING_SUPER_NODE, LAMBDA_NODE, SUPER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SingleLineDoEndBlock;

impl SingleLineDoEndBlock {
    fn check_do_end_block(
        &self,
        source: &SourceFile,
        expr_start: usize,
        opening_loc: ruby_prism::Location<'_>,
        closing_loc: ruby_prism::Location<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if opening_loc.as_slice() != b"do" {
            return;
        }

        let (opening_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (closing_line, _) = source.offset_to_line_col(closing_loc.start_offset());
        if opening_line != closing_line {
            return;
        }

        let (line, column) = source.offset_to_line_col(expr_start);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer multiline `do`...`end` block.".to_string(),
        ));
    }
}

impl Cop for SingleLineDoEndBlock {
    fn name(&self) -> &'static str {
        "Style/SingleLineDoEndBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SUPER_NODE, FORWARDING_SUPER_NODE, LAMBDA_NODE]
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
        if let Some(call) = node.as_call_node() {
            let block = match call.block().and_then(|block| block.as_block_node()) {
                Some(block) => block,
                None => return,
            };
            self.check_do_end_block(
                source,
                call.location().start_offset(),
                block.opening_loc(),
                block.closing_loc(),
                diagnostics,
            );
            return;
        }

        if let Some(super_node) = node.as_super_node() {
            let block = match super_node.block().and_then(|block| block.as_block_node()) {
                Some(block) => block,
                None => return,
            };
            self.check_do_end_block(
                source,
                super_node.location().start_offset(),
                block.opening_loc(),
                block.closing_loc(),
                diagnostics,
            );
            return;
        }

        if let Some(forwarding_super_node) = node.as_forwarding_super_node() {
            let block = match forwarding_super_node.block() {
                Some(block) => block,
                None => return,
            };
            self.check_do_end_block(
                source,
                forwarding_super_node.location().start_offset(),
                block.opening_loc(),
                block.closing_loc(),
                diagnostics,
            );
            return;
        }

        if let Some(lambda) = node.as_lambda_node() {
            self.check_do_end_block(
                source,
                lambda.location().start_offset(),
                lambda.opening_loc(),
                lambda.closing_loc(),
                diagnostics,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SingleLineDoEndBlock, "cops/style/single_line_do_end_block");
}
