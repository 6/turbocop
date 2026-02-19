use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE, FLOAT_NODE, IMAGINARY_NODE, INTEGER_NODE, NIL_NODE, RATIONAL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct RedundantFetchBlock;

impl RedundantFetchBlock {
    fn is_simple_literal(node: &ruby_prism::Node<'_>) -> bool {
        node.as_integer_node().is_some()
            || node.as_float_node().is_some()
            || node.as_symbol_node().is_some()
            || node.as_string_node().is_some()
            || node.as_true_node().is_some()
            || node.as_false_node().is_some()
            || node.as_nil_node().is_some()
            || node.as_rational_node().is_some()
            || node.as_imaginary_node().is_some()
    }
}

impl Cop for RedundantFetchBlock {
    fn name(&self) -> &'static str {
        "Style/RedundantFetchBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE, FLOAT_NODE, IMAGINARY_NODE, INTEGER_NODE, NIL_NODE, RATIONAL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let safe_for_constants = config.get_bool("SafeForConstants", false);
        // Check if frozen_string_literal is enabled (needed for string body)
        let frozen_string_literal = source.lines().next().is_some_and(|line| {
            std::str::from_utf8(line).unwrap_or("").contains("frozen_string_literal: true")
        });

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"fetch" {
            return Vec::new();
        }

        // Must have exactly one argument (the key)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Block must have no parameters
        if block_node.parameters().is_some() {
            return Vec::new();
        }

        // Check block body
        let body = block_node.body();

        // Skip Rails.cache.fetch - those blocks do computation
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if recv_call.name().as_slice() == b"cache" {
                    if let Some(recv_recv) = recv_call.receiver() {
                        if let Some(const_node) = recv_recv.as_constant_read_node() {
                            if const_node.name().as_slice() == b"Rails" {
                                return Vec::new();
                            }
                        }
                        if let Some(const_path) = recv_recv.as_constant_path_node() {
                            if const_path.location().as_slice() == b"Rails" {
                                return Vec::new();
                            }
                        }
                    }
                }
            }
        }

        let is_redundant = if let Some(ref body) = body {
            if let Some(stmts) = body.as_statements_node() {
                let body_stmts: Vec<_> = stmts.body().iter().collect();
                if body_stmts.len() == 1 {
                    let expr = &body_stmts[0];
                    // String literals require frozen_string_literal: true
                    if expr.as_string_node().is_some() && !frozen_string_literal {
                        false
                    } else if Self::is_simple_literal(expr) {
                        true
                    } else if safe_for_constants {
                        expr.as_constant_read_node().is_some()
                            || expr.as_constant_path_node().is_some()
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            // Empty block: fetch(:key) {} => fetch(:key, nil)
            true
        };

        if !is_redundant {
            return Vec::new();
        }

        let key_src = std::str::from_utf8(arg_list[0].location().as_slice()).unwrap_or("");
        let value_src = if let Some(body) = body {
            if let Some(stmts) = body.as_statements_node() {
                let body_stmts: Vec<_> = stmts.body().iter().collect();
                if body_stmts.len() == 1 {
                    std::str::from_utf8(body_stmts[0].location().as_slice()).unwrap_or("nil").to_string()
                } else {
                    "nil".to_string()
                }
            } else {
                "nil".to_string()
            }
        } else {
            "nil".to_string()
        };

        let fetch_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(fetch_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `fetch({key_src}, {value_src})` instead of `fetch({key_src}) {{ {value_src} }}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFetchBlock, "cops/style/redundant_fetch_block");
}
