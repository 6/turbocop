use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SkipBlockInsideExample;

/// Flags `skip 'reason' do ... end` inside an example.
/// `skip` should not be passed a block.
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
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
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
        // Look for example blocks (it, specify, etc.) and then find `skip` with a block inside
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

        find_skip_with_block(source, block_node, diagnostics, self);
    }
}

fn find_skip_with_block(
    source: &SourceFile,
    block: ruby_prism::BlockNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SkipBlockInsideExample,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            if call.name().as_slice() == b"skip"
                && call.receiver().is_none()
                && call.block().is_some()
            {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Don't pass a block to `skip` inside examples.".to_string(),
                ));
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
