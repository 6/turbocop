use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantReceiverInWithOptions;

impl Cop for RedundantReceiverInWithOptions {
    fn name(&self) -> &'static str {
        "Rails/RedundantReceiverInWithOptions"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"with_options" {
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
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // If no block parameter, check if any nested blocks exist (which would
        // make it unsafe to assume all receiver usages are the block param)
        if param_name.is_none() {
            // Check for numbered block parameter usage (_1)
            // or `it` usage (Ruby 3.4+)
            // For no block params, check if statements use _1/it as receiver
            return self.check_numbered_params(source, &stmts);
        }

        let param_bytes = param_name.unwrap();
        let mut diagnostics = Vec::new();

        // Check each statement in the block body
        for stmt in stmts.body().iter() {
            self.check_stmt_for_redundant_receiver(
                source,
                &stmt,
                &param_bytes,
                &mut diagnostics,
            );
        }

        diagnostics
    }
}

impl RedundantReceiverInWithOptions {
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
