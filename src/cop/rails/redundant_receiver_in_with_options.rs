use crate::cop::shared::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CALL_OR_WRITE_NODE,
    INSTANCE_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RuboCop's `all_block_nodes_in` and `all_send_nodes_in` use recursive
/// `def_node_search` which descends into ALL child nodes including hash
/// arguments, conditionals (`if`/`unless`), and boolean operators (`||`/`&&`).
///
/// FP fixes:
/// - Lambdas inside hash arguments (e.g., `:if => -> { ... }`) must be found
///   by `contains_block_or_lambda` to correctly bail out.
/// - Send nodes inside `unless`/`if` blocks and boolean operators must be
///   checked by `node_all_sends_use_param` so that non-param receivers
///   (e.g., `reflect_on_association(:parent)`) correctly prevent flagging.
pub struct RedundantReceiverInWithOptions;

impl Cop for RedundantReceiverInWithOptions {
    fn name(&self) -> &'static str {
        "Rails/RedundantReceiverInWithOptions"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            CALL_OR_WRITE_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
        ]
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

        if call.name().as_slice() != b"with_options" {
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

        // Get the block parameter name (e.g., |assoc|)
        let param_name = match block_node.parameters() {
            Some(params) => {
                if let Some(bp) = params.as_block_parameters_node() {
                    if let Some(params_node) = bp.parameters() {
                        let requireds: Vec<_> = params_node.requireds().iter().collect();
                        if requireds.len() == 1 {
                            requireds[0]
                                .as_required_parameter_node()
                                .map(|req| req.name().as_slice().to_vec())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            None => None,
        };

        // If no block parameter, the block might use _1 or `it` (numbered parameters)
        // We need to check for local variable reads of _1 or `it` used as receivers
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // If no block parameter, check if any nested blocks exist (which would
        // make it unsafe to assume all receiver usages are the block param)
        if param_name.is_none() {
            // Check for numbered block parameter usage (_1)
            // or `it` usage (Ruby 3.4+)
            // For no block params, check if statements use _1/it as receiver
            diagnostics.extend(self.check_numbered_params(source, &stmts));
            return;
        }

        let param_bytes = param_name.unwrap();

        // RuboCop requires ALL send nodes in the block to use the block parameter
        // as receiver. If any statement uses a different receiver (e.g. `self`),
        // or is not a send node, the whole block is not flagged.
        // Also skip if there are any block/lambda nodes in the body.

        let body_stmts: Vec<_> = stmts.body().iter().collect();

        // RuboCop: `all_block_nodes_in(body).none?` — exit if ANY block/lambda in body
        for stmt in &body_stmts {
            if Self::contains_block_or_lambda(stmt) {
                return;
            }
        }

        // RuboCop: `all_send_nodes_in(body).all?(&proc)` — ALL sends must use param
        if !self.all_sends_use_param_deep(&body_stmts, &param_bytes) {
            return;
        }

        // Second pass: collect offenses for all statements with redundant receiver
        for stmt in &body_stmts {
            self.check_stmt_for_redundant_receiver(source, stmt, &param_bytes, diagnostics);
        }
    }
}

impl RedundantReceiverInWithOptions {
    /// Recursively check if any block or lambda node exists anywhere in a subtree.
    /// RuboCop exits early if `all_block_nodes_in(body).none?` is false.
    /// RuboCop's `def_node_search :all_block_nodes_in, '(block ...)'` matches
    /// `(block ...)` anywhere in the AST, including inside hash arguments and
    /// conditionals. We must recurse through all structural node types.
    fn contains_block_or_lambda(node: &ruby_prism::Node<'_>) -> bool {
        // Lambda nodes (-> { ... }) are block nodes in Parser AST
        if node.as_lambda_node().is_some() {
            return true;
        }
        if let Some(call) = node.as_call_node() {
            if call.block().is_some() {
                return true;
            }
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if Self::contains_block_or_lambda(&arg) {
                        return true;
                    }
                }
            }
        }
        // Recurse into hash/assoc/array nodes (lambdas can appear in hash values)
        if let Some(hash) = node.as_keyword_hash_node() {
            for elem in hash.elements().iter() {
                if Self::contains_block_or_lambda(&elem) {
                    return true;
                }
            }
        }
        if let Some(hash) = node.as_hash_node() {
            for elem in hash.elements().iter() {
                if Self::contains_block_or_lambda(&elem) {
                    return true;
                }
            }
        }
        if let Some(assoc) = node.as_assoc_node() {
            if Self::contains_block_or_lambda(&assoc.value()) {
                return true;
            }
        }
        if let Some(array) = node.as_array_node() {
            for elem in array.elements().iter() {
                if Self::contains_block_or_lambda(&elem) {
                    return true;
                }
            }
        }
        // Recurse into if/unless nodes
        if let Some(if_node) = node.as_if_node() {
            if Self::contains_block_or_lambda(&if_node.predicate()) {
                return true;
            }
            if let Some(stmts) = if_node.statements() {
                for stmt in stmts.body().iter() {
                    if Self::contains_block_or_lambda(&stmt) {
                        return true;
                    }
                }
            }
            if let Some(else_clause) = if_node.subsequent() {
                if Self::contains_block_or_lambda(&else_clause) {
                    return true;
                }
            }
        }
        if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                for stmt in stmts.body().iter() {
                    if Self::contains_block_or_lambda(&stmt) {
                        return true;
                    }
                }
            }
        }
        if let Some(unless_node) = node.as_unless_node() {
            if Self::contains_block_or_lambda(&unless_node.predicate()) {
                return true;
            }
            if let Some(stmts) = unless_node.statements() {
                for stmt in stmts.body().iter() {
                    if Self::contains_block_or_lambda(&stmt) {
                        return true;
                    }
                }
            }
            if let Some(else_clause) = unless_node.else_clause() {
                if let Some(stmts) = else_clause.statements() {
                    for stmt in stmts.body().iter() {
                        if Self::contains_block_or_lambda(&stmt) {
                            return true;
                        }
                    }
                }
            }
        }
        // Recurse into boolean operators
        if let Some(or_node) = node.as_or_node() {
            return Self::contains_block_or_lambda(&or_node.left())
                || Self::contains_block_or_lambda(&or_node.right());
        }
        if let Some(and_node) = node.as_and_node() {
            return Self::contains_block_or_lambda(&and_node.left())
                || Self::contains_block_or_lambda(&and_node.right());
        }
        // Recurse into assignment values
        if let Some(or_write) = node.as_instance_variable_or_write_node() {
            return Self::contains_block_or_lambda(&or_write.value());
        }
        if let Some(or_write) = node.as_local_variable_or_write_node() {
            return Self::contains_block_or_lambda(&or_write.value());
        }
        false
    }

    /// Recursively check that ALL call nodes in a subtree use the block param as receiver.
    /// Matches RuboCop's `all_send_nodes_in(body).all?(&proc)` — deep recursive search
    /// through all node types (hashes, arrays, assocs, etc.).
    fn node_all_sends_use_param(&self, node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
        if let Some(call) = node.as_call_node() {
            // The call must have the block param as receiver
            match call.receiver() {
                Some(receiver) => {
                    if !self.is_param_receiver(&receiver, param_name) {
                        return false;
                    }
                }
                None => return false,
            }
            // Recurse into arguments
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if !self.node_all_sends_use_param(&arg, param_name) {
                        return false;
                    }
                }
            }
            return true;
        }
        if let Some(cor) = node.as_call_or_write_node() {
            if let Some(receiver) = cor.receiver() {
                if !self.is_param_receiver(&receiver, param_name) {
                    return false;
                }
            }
            return true;
        }
        // Recurse into hash/assoc nodes to find nested calls
        if let Some(hash) = node.as_keyword_hash_node() {
            for elem in hash.elements().iter() {
                if !self.node_all_sends_use_param(&elem, param_name) {
                    return false;
                }
            }
            return true;
        }
        if let Some(hash) = node.as_hash_node() {
            for elem in hash.elements().iter() {
                if !self.node_all_sends_use_param(&elem, param_name) {
                    return false;
                }
            }
            return true;
        }
        if let Some(assoc) = node.as_assoc_node() {
            return self.node_all_sends_use_param(&assoc.key(), param_name)
                && self.node_all_sends_use_param(&assoc.value(), param_name);
        }
        if let Some(array) = node.as_array_node() {
            for elem in array.elements().iter() {
                if !self.node_all_sends_use_param(&elem, param_name) {
                    return false;
                }
            }
            return true;
        }
        if let Some(or_write) = node.as_instance_variable_or_write_node() {
            return self.node_all_sends_use_param(&or_write.value(), param_name);
        }
        if let Some(or_write) = node.as_local_variable_or_write_node() {
            return self.node_all_sends_use_param(&or_write.value(), param_name);
        }
        // Recurse into boolean operators (||, &&)
        if let Some(or_node) = node.as_or_node() {
            return self.node_all_sends_use_param(&or_node.left(), param_name)
                && self.node_all_sends_use_param(&or_node.right(), param_name);
        }
        if let Some(and_node) = node.as_and_node() {
            return self.node_all_sends_use_param(&and_node.left(), param_name)
                && self.node_all_sends_use_param(&and_node.right(), param_name);
        }
        // Recurse into parentheses
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                if let Some(stmts) = body.as_statements_node() {
                    for stmt in stmts.body().iter() {
                        if !self.node_all_sends_use_param(&stmt, param_name) {
                            return false;
                        }
                    }
                }
            }
        }
        // Recurse into if/unless nodes — RuboCop's all_send_nodes_in finds
        // sends inside conditionals too
        if let Some(if_node) = node.as_if_node() {
            if !self.node_all_sends_use_param(&if_node.predicate(), param_name) {
                return false;
            }
            if let Some(stmts) = if_node.statements() {
                for stmt in stmts.body().iter() {
                    if !self.node_all_sends_use_param(&stmt, param_name) {
                        return false;
                    }
                }
            }
            if let Some(else_clause) = if_node.subsequent() {
                if !self.node_all_sends_use_param(&else_clause, param_name) {
                    return false;
                }
            }
        }
        if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                for stmt in stmts.body().iter() {
                    if !self.node_all_sends_use_param(&stmt, param_name) {
                        return false;
                    }
                }
            }
        }
        if let Some(unless_node) = node.as_unless_node() {
            if !self.node_all_sends_use_param(&unless_node.predicate(), param_name) {
                return false;
            }
            if let Some(stmts) = unless_node.statements() {
                for stmt in stmts.body().iter() {
                    if !self.node_all_sends_use_param(&stmt, param_name) {
                        return false;
                    }
                }
            }
            if let Some(else_clause) = unless_node.else_clause() {
                if let Some(stmts) = else_clause.statements() {
                    for stmt in stmts.body().iter() {
                        if !self.node_all_sends_use_param(&stmt, param_name) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Check that ALL call nodes across all body statements use the block param as receiver.
    fn all_sends_use_param_deep(&self, stmts: &[ruby_prism::Node<'_>], param_name: &[u8]) -> bool {
        for stmt in stmts {
            if !self.node_all_sends_use_param(stmt, param_name) {
                return false;
            }
        }
        true
    }
    fn check_stmt_for_redundant_receiver(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        param_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Check if the receiver is the block parameter
        if let Some(receiver) = call.receiver() {
            if self.is_param_receiver(&receiver, param_name) {
                let recv_loc = receiver.location();
                let (line, column) = source.offset_to_line_col(recv_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Redundant receiver in `with_options`.".to_string(),
                ));
            }
        }

        // Also check arguments for nested receiver usage
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                self.check_nested_receiver(source, &arg, param_name, diagnostics);
            }
        }
    }

    fn check_nested_receiver(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        param_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(call) = node.as_call_node() {
            if let Some(receiver) = call.receiver() {
                if self.is_param_receiver(&receiver, param_name) {
                    let recv_loc = receiver.location();
                    let (line, column) = source.offset_to_line_col(recv_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Redundant receiver in `with_options`.".to_string(),
                    ));
                }
            }
            // Recurse into call arguments
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    self.check_nested_receiver(source, &arg, param_name, diagnostics);
                }
            }
        }
    }

    fn is_param_receiver(&self, node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
        if let Some(local) = node.as_local_variable_read_node() {
            return local.name().as_slice() == param_name;
        }
        // Check for CallNode with just the param name (no receiver, no args)
        if let Some(call) = node.as_call_node() {
            if call.receiver().is_none() && call.arguments().is_none() {
                return call.name().as_slice() == param_name;
            }
        }
        false
    }

    fn check_numbered_params(
        &self,
        source: &SourceFile,
        stmts: &ruby_prism::StatementsNode<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                if let Some(receiver) = call.receiver() {
                    // Check for _1 (numbered parameter reference) or `it`
                    let loc = receiver.location();
                    let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
                    if text == b"_1" || text == b"it" {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Redundant receiver in `with_options`.".to_string(),
                        ));
                    }
                }
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantReceiverInWithOptions,
        "cops/rails/redundant_receiver_in_with_options"
    );
}
