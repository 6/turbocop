use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, NUMBERED_REFERENCE_READ_NODE, STATEMENTS_NODE, SYMBOL_NODE};

pub struct RedundantAround;

/// Flags `around` hooks that just yield/run without doing anything else.
/// e.g. `around { |ex| ex.run }` or `around(&:run)`
impl Cop for RedundantAround {
    fn name(&self) -> &'static str {
        "RSpec/RedundantAround"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, NUMBERED_REFERENCE_READ_NODE, STATEMENTS_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Check for `around` method (with or without receiver like `config.around`)
        let method_name = call.name().as_slice();
        if method_name != b"around" {
            return;
        }

        // Check for block-pass `around(&:run)`
        if let Some(block_arg) = call.block() {
            if let Some(bp) = block_arg.as_block_argument_node() {
                // Check if it's &:run
                if let Some(expr) = bp.expression() {
                    if let Some(sym) = expr.as_symbol_node() {
                        if sym.unescaped() == b"run" {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Remove redundant `around` hook.".to_string(),
                            ));
                        }
                    }
                }
                return;
            }
        }

        // Check for block form `around do |ex| ex.run end`
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let stmt_list: Vec<_> = stmts.body().iter().collect();
        if stmt_list.len() != 1 {
            return;
        }

        // The single statement should be `param.run` or `_1.run`
        let stmt = &stmt_list[0];
        let stmt_call = match stmt.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if stmt_call.name().as_slice() != b"run" {
            return;
        }

        // The receiver should be the block parameter
        let recv = match stmt_call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if receiver is a local variable read (the block param)
        let is_block_param = recv.as_local_variable_read_node().is_some()
            || recv.as_numbered_reference_read_node().is_some();

        // Also check for `_1` pattern (call node to _1)
        let is_numbered_param = if let Some(c) = recv.as_call_node() {
            let n = c.name().as_slice();
            n == b"_1" && c.receiver().is_none()
        } else {
            false
        };

        if !is_block_param && !is_numbered_param {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Remove redundant `around` hook.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantAround, "cops/rspec/redundant_around");
}
