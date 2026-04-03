use crate::cop::shared::method_dispatch_predicates;
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SkipBlockInsideExample;

/// Flags `skip 'reason' do ... end` inside an example.
/// `skip` should not be passed a block.
///
/// Uses check_node to find example blocks (it, specify, etc.), then
/// recursively searches all descendants for `skip` calls with a block.
/// This handles `skip` nested arbitrarily deep inside helper method blocks,
/// describe blocks, etc., as long as there's an example ancestor.
impl Cop for SkipBlockInsideExample {
    fn name(&self) -> &'static str {
        "RSpec/SkipBlockInsideExample"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        if !is_rspec_example(call.name().as_slice()) {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        if let Some(body) = block_node.body() {
            find_skip_with_block_recursive(source, &body, diagnostics, self);
        }
    }
}

/// Recursively search inside a node for `skip` calls with a block.
fn find_skip_with_block_recursive(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SkipBlockInsideExample,
) {
    if let Some(call) = node.as_call_node() {
        if method_dispatch_predicates::is_command(&call, b"skip") && call.block().is_some() {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Don't pass a block to `skip` inside examples.".to_string(),
            ));
            return; // Don't recurse into the skip block itself
        }
    }

    // Recurse into child nodes — handle common container types
    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            find_skip_with_block_recursive(source, &child, diagnostics, cop);
        }
        return;
    }
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            find_skip_with_block_recursive(source, &recv, diagnostics, cop);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_skip_with_block_recursive(source, &arg, diagnostics, cop);
            }
        }
        if let Some(block) = call.block() {
            find_skip_with_block_recursive(source, &block, diagnostics, cop);
        }
        return;
    }
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            find_skip_with_block_recursive(source, &body, diagnostics, cop);
        }
        return;
    }
    if let Some(begin) = node.as_begin_node() {
        if let Some(stmts) = begin.statements() {
            find_skip_with_block_recursive(source, &stmts.as_node(), diagnostics, cop);
        }
        return;
    }
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            find_skip_with_block_recursive(source, &stmts.as_node(), diagnostics, cop);
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_skip_with_block_recursive(source, &subsequent, diagnostics, cop);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            find_skip_with_block_recursive(source, &stmts.as_node(), diagnostics, cop);
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                find_skip_with_block_recursive(source, &stmts.as_node(), diagnostics, cop);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SkipBlockInsideExample,
        "cops/rspec/skip_block_inside_example"
    );
}
