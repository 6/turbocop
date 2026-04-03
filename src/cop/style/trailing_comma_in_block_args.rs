use crate::cop::shared::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-29)
///
/// The missed corpus cases all share the same shape: a chained call where an
/// inner receiver block uses a single trailing-comma parameter (`|name,|`) and
/// a later outer block in the chain uses 2+ parameters (`|name, value|`).
///
/// RuboCop's `argument_tokens` logic effectively looks at the first receiver
/// block's pipes while still using the outer block's arity, so it reports the
/// receiver block's comma. To match RuboCop, this cop now keeps the normal
/// direct-block check and also, for multi-arg outer blocks, walks the receiver
/// chain to find the first single-arg trailing-comma block that RuboCop flags.
///
/// Plain single-argument trailing-comma blocks such as `items.each { |item,| }`
/// still remain non-offenses unless they appear in that chained outer-block
/// context.
///
/// FP fix (2026-03-31): when an earlier block in the receiver chain already
/// has piped parameters (even without a trailing comma), RuboCop's token-based
/// approach picks up those pipes first, masking any later single-param
/// trailing-comma block. We now stop the receiver-chain walk when an earlier
/// piped block is found, matching RuboCop's behavior.
pub struct TrailingCommaInBlockArgs;

fn block_param_count(block: &ruby_prism::BlockNode<'_>) -> Option<usize> {
    let params = block.parameters()?;
    let block_params = params.as_block_parameters_node()?;
    let inner_params = block_params.parameters()?;

    Some(
        inner_params
            .requireds()
            .iter()
            .filter(|param| param.as_required_parameter_node().is_some())
            .count()
            + inner_params
                .optionals()
                .iter()
                .filter(|param| param.as_optional_parameter_node().is_some())
                .count()
            + inner_params
                .posts()
                .iter()
                .filter(|param| param.as_required_parameter_node().is_some())
                .count()
            + inner_params
                .keywords()
                .iter()
                .filter(|param| param.as_optional_keyword_parameter_node().is_some())
                .count(),
    )
}

fn trailing_comma_offset(source: &SourceFile, block: &ruby_prism::BlockNode<'_>) -> Option<usize> {
    let params = block.parameters()?;
    let block_params = params.as_block_parameters_node()?;
    let close_loc = block_params.closing_loc()?;

    let bytes = source.as_bytes();
    let close_offset = close_loc.start_offset();
    if close_offset == 0 {
        return None;
    }

    let mut pos = close_offset - 1;
    while pos > 0 && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
        pos -= 1;
    }

    (bytes[pos] == b',').then_some(pos)
}

/// Check whether a call node (or any call in its receiver chain) has a block
/// with piped parameters (i.e. `block_parameters_node` is present).
fn receiver_chain_has_piped_block(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(call) => call,
        None => return false,
    };

    if let Some(receiver) = call.receiver() {
        if receiver_chain_has_piped_block(&receiver) {
            return true;
        }
    }

    call.block()
        .and_then(|block| block.as_block_node())
        .and_then(|block| block.parameters())
        .and_then(|params| params.as_block_parameters_node())
        .is_some()
}

fn receiver_chain_trailing_comma_offset(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<usize> {
    let call = node.as_call_node()?;

    if let Some(receiver) = call.receiver() {
        if let Some(offset) = receiver_chain_trailing_comma_offset(source, &receiver) {
            return Some(offset);
        }
        // If a deeper block already has piped parameters, stop searching.
        // RuboCop's token-based approach picks up the earliest pipe tokens
        // in the source range, so later blocks are masked.
        if receiver_chain_has_piped_block(&receiver) {
            return None;
        }
    }

    let block = call.block().and_then(|block| block.as_block_node())?;
    if block_param_count(&block) == Some(1) {
        return trailing_comma_offset(source, &block);
    }

    None
}

fn push_diagnostic(
    cop: &TrailingCommaInBlockArgs,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    offset: usize,
) {
    let (line, column) = source.offset_to_line_col(offset);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Useless trailing comma present in block arguments.".to_string(),
    ));
}

impl Cop for TrailingCommaInBlockArgs {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInBlockArgs"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
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
        if let Some(block) = node.as_block_node() {
            match block_param_count(&block) {
                Some(count) if count > 1 => {}
                _ => return,
            }

            if let Some(offset) = trailing_comma_offset(source, &block) {
                push_diagnostic(self, source, diagnostics, offset);
            }
            return;
        }

        let call = match node.as_call_node() {
            Some(call) => call,
            None => return,
        };

        let block = match call.block().and_then(|block| block.as_block_node()) {
            Some(block) => block,
            None => return,
        };
        match block_param_count(&block) {
            Some(count) if count > 1 => {}
            _ => return,
        }

        if let Some(receiver) = call.receiver() {
            if let Some(offset) = receiver_chain_trailing_comma_offset(source, &receiver) {
                push_diagnostic(self, source, diagnostics, offset);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TrailingCommaInBlockArgs,
        "cops/style/trailing_comma_in_block_args"
    );
}
