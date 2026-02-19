use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};

pub struct TransactionExitStatement;

struct ExitFinder {
    found: Vec<(usize, &'static str)>,
}

impl<'pr> Visit<'pr> for ExitFinder {
    // Skip nested method/class/module definitions to avoid false positives
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        self.found
            .push((node.location().start_offset(), "return"));
    }

    fn visit_break_node(&mut self, node: &ruby_prism::BreakNode<'pr>) {
        self.found
            .push((node.location().start_offset(), "break"));
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"throw" && node.receiver().is_none() {
            self.found
                .push((node.location().start_offset(), "throw"));
        }
        // Continue visiting children of this call node
        ruby_prism::visit_call_node(self, node);
    }
}

impl Cop for TransactionExitStatement {
    fn name(&self) -> &'static str {
        "Rails/TransactionExitStatement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
    ) -> Vec<Diagnostic> {
        let transaction_methods = config.get_string_array("TransactionMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        let method_name = call.name().as_slice();
        let is_transaction = if let Some(ref methods) = transaction_methods {
            let name_str = std::str::from_utf8(method_name).unwrap_or("");
            methods.iter().any(|m| m == name_str)
        } else {
            method_name == b"transaction"
        };
        if !is_transaction {
            return Vec::new();
        }
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut finder = ExitFinder { found: vec![] };
        finder.visit(&body);

        finder
            .found
            .iter()
            .map(|&(offset, statement)| {
                let (line, column) = source.offset_to_line_col(offset);
                self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Do not use `{statement}` inside a transaction block."),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TransactionExitStatement,
        "cops/rails/transaction_exit_statement"
    );
}
