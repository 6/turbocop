use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CALL_OR_WRITE_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct RedundantReceiverInWithOptions;

impl Cop for RedundantReceiverInWithOptions {
    fn name(&self) -> &'static str {
        "Rails/RedundantReceiverInWithOptions"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CALL_OR_WRITE_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
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
                            if let Some(req) = requireds[0].as_required_parameter_node() {
                                Some(req.name().as_slice().to_vec())
                            } else {
                                None
                            }
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
        // Also skip if there are nested blocks (which would make scope analysis unsafe).

        // First pass: check that ALL statements are eligible and use the block param
        // as receiver. Also check for nested blocks.
        let body_stmts: Vec<_> = stmts.body().iter().collect();

        // Check every statement: must be a call with block param as receiver,
        // and no nested blocks allowed anywhere in the body.
        // Also check that no node anywhere uses a different receiver.
        if !self.all_sends_use_param(source, &body_stmts, &param_bytes) {
            return;
        }

        // Second pass: collect offenses for all statements with redundant receiver
        for stmt in &body_stmts {
            self.check_stmt_for_redundant_receiver(
                source,
                stmt,
                &param_bytes,
                diagnostics,
            );
        }

    }
}

impl RedundantReceiverInWithOptions {
    /// Check that ALL send nodes in the block body use the block param as receiver.
    /// Returns false if any send uses a different receiver, or if there are nested blocks.
    fn all_sends_use_param(
        &self,
        source: &SourceFile,
        stmts: &[ruby_prism::Node<'_>],
        param_name: &[u8],
    ) -> bool {
        for stmt in stmts {
            if !self.node_all_sends_use_param(source, stmt, param_name) {
                return false;
            }
        }
        true
    }

    /// Recursively check that all send nodes in a subtree use the block param as receiver.
    fn node_all_sends_use_param(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        param_name: &[u8],
    ) -> bool {
        if let Some(call) = node.as_call_node() {
            // Nested blocks make analysis unsafe
            if call.block().is_some() {
                return false;
            }
            // The call must have the block param as receiver
            if let Some(receiver) = call.receiver() {
                if !self.is_param_receiver(&receiver, param_name) {
                    return false;
                }
            }
            // Check arguments recursively
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if !self.node_all_sends_use_param(source, &arg, param_name) {
                        return false;
                    }
                }
            }
            return true;
        }
        // CallOrWriteNode: e.g. `self._named_contexts ||= {}`
        // The receiver must also be the block param, otherwise this block is mixed.
        if let Some(cor) = node.as_call_or_write_node() {
            if let Some(receiver) = cor.receiver() {
                if !self.is_param_receiver(&receiver, param_name) {
                    return false;
                }
            }
            return true;
        }
        // For non-call nodes, recurse into children that might contain sends
        if let Some(or_write) = node.as_instance_variable_or_write_node() {
            return self.node_all_sends_use_param(source, &or_write.value(), param_name);
        }
        if let Some(or_write) = node.as_local_variable_or_write_node() {
            return self.node_all_sends_use_param(source, &or_write.value(), param_name);
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
