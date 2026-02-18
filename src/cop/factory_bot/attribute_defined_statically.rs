use crate::cop::factory_bot::{ATTRIBUTE_DEFINING_METHODS, FACTORY_BOT_DEFAULT_INCLUDE, RESERVED_METHODS};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AttributeDefinedStatically;

fn is_attribute_defining_method(name: &[u8]) -> bool {
    ATTRIBUTE_DEFINING_METHODS.contains(&name)
}

fn is_reserved_method(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    RESERVED_METHODS.contains(&s)
}

/// Check if a call has a hash argument with a `factory:` key.
fn has_factory_option(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    for arg in args.arguments().iter() {
        if let Some(hash) = arg.as_keyword_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if let Some(sym) = pair.key().as_symbol_node() {
                        if sym.unescaped() == b"factory" {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if let Some(sym) = pair.key().as_symbol_node() {
                        if sym.unescaped() == b"factory" {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if all arguments are block_pass (e.g. `sequence :foo, &:bar`).
fn all_args_block_pass(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    args.arguments()
        .iter()
        .all(|arg| arg.as_block_argument_node().is_some())
}

impl Cop for AttributeDefinedStatically {
    fn name(&self) -> &'static str {
        "FactoryBot/AttributeDefinedStatically"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Match on CallNode for attribute-defining methods (factory, trait, etc.)
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = outer_call.name().as_slice();
        if !is_attribute_defining_method(method) {
            return Vec::new();
        }

        // Must have a block
        let block = match outer_call.block() {
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

        let children: Vec<_> = if let Some(stmts) = body.as_statements_node() {
            stmts.body().iter().collect()
        } else {
            vec![body]
        };

        let mut diagnostics = Vec::new();

        // Get the block's first parameter name (if any)
        let block_param_name = block_node
            .parameters()
            .and_then(|p| p.as_block_parameters_node())
            .and_then(|bp| {
                bp.parameters().and_then(|params| {
                    params
                        .requireds()
                        .iter()
                        .next()
                        .and_then(|r| r.as_required_parameter_node())
                        .map(|rp| rp.name().as_slice().to_vec())
                })
            });

        for child in &children {
            let call = match child.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            let method_name = call.name().as_slice();

            // Skip reserved methods
            if is_reserved_method(method_name) {
                continue;
            }

            // Must have arguments (the value to set)
            if call.arguments().is_none() {
                continue;
            }

            // Skip if it's a proc-like call (all block_pass args)
            if all_args_block_pass(&call) {
                continue;
            }

            // Skip if it's an association (has factory: key)
            if has_factory_option(&call) {
                continue;
            }

            // Must NOT have a block (that's the offense: should use blocks)
            if call.block().is_some() {
                continue;
            }

            // Check receiver: must be nil, self, or match the block's parameter
            let offensive_receiver = match call.receiver() {
                None => true,
                Some(recv) => {
                    if recv.as_self_node().is_some() {
                        true
                    } else if let Some(lvar) = recv.as_local_variable_read_node() {
                        if let Some(ref param) = block_param_name {
                            lvar.name().as_slice() == param.as_slice()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            };

            if !offensive_receiver {
                continue;
            }

            let loc = child.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use a block to declare attribute values.".to_string(),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AttributeDefinedStatically,
        "cops/factorybot/attribute_defined_statically"
    );
}
