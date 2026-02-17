use crate::cop::factory_bot::{is_factory_call, FACTORY_BOT_SPEC_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CreateList;

impl Cop for CreateList {
    fn name(&self) -> &'static str {
        "FactoryBot/CreateList"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "create_list");
        let explicit_only = config.get_bool("ExplicitOnly", false);

        if style == "create_list" {
            // Check blocks: n.times { create :user }, n.times.map { ... }, Array.new(n) { ... }
            if let Some(block) = node.as_block_node() {
                return self.check_block_for_create_list(source, &block, explicit_only);
            }
        }

        if style == "n_times" {
            // Check create_list calls
            if let Some(call) = node.as_call_node() {
                return self.check_call_for_n_times(source, &call, explicit_only);
            }
        }

        Vec::new()
    }
}

impl CreateList {
    fn check_block_for_create_list(
        &self,
        source: &SourceFile,
        block: &ruby_prism::BlockNode<'_>,
        explicit_only: bool,
    ) -> Vec<Diagnostic> {
        let send = match block.call().as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Try to extract repeat count from:
        // - n.times { ... }
        // - n.times.map { ... }
        // - Array.new(n) { ... }
        let count = get_repeat_count(&send);
        let count = match count {
            Some(c) if c > 1 => c,
            _ => return Vec::new(),
        };

        // Check if block arg is used (if so, skip)
        if block_arg_is_used(block) {
            return Vec::new();
        }

        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Body must be a single factory create call
        let body_call = get_single_create_call(&body);
        let body_call = match body_call {
            Some(c) => c,
            None => return Vec::new(),
        };

        if !is_factory_call(body_call.receiver(), explicit_only) {
            return Vec::new();
        }

        if body_call.name().as_slice() != b"create" {
            return Vec::new();
        }

        // First arg must be a symbol
        let args = match body_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() || arg_list[0].as_symbol_node().is_none() {
            return Vec::new();
        }

        // Check if arguments include a method call (rand, etc.)
        if arguments_include_method_call(&arg_list) {
            return Vec::new();
        }

        let send_loc = send.location();
        let (line, column) = source.offset_to_line_col(send_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer create_list.".to_string(),
        )]
    }

    fn check_call_for_n_times(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        explicit_only: bool,
    ) -> Vec<Diagnostic> {
        if call.name().as_slice() != b"create_list" {
            return Vec::new();
        }

        if !is_factory_call(call.receiver(), explicit_only) {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return Vec::new();
        }

        // First arg: symbol or string (factory name)
        if arg_list[0].as_symbol_node().is_none() && arg_list[0].as_string_node().is_none() {
            return Vec::new();
        }

        // Second arg: integer (count)
        let count = match arg_list[1].as_integer_node() {
            Some(int) => {
                let val = int.value();
                val.try_into().unwrap_or(0i64)
            }
            None => return Vec::new(),
        };

        if count < 2 {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Prefer {}.times.map.", count),
        )]
    }
}

/// Extract repeat count from n.times, n.times.map, or Array.new(n).
fn get_repeat_count(call: &ruby_prism::CallNode<'_>) -> Option<i64> {
    let method = call.name().as_slice();

    if method == b"times" {
        // n.times — receiver is integer literal
        if let Some(recv) = call.receiver() {
            if let Some(int) = recv.as_integer_node() {
                let val: i64 = int.value().try_into().unwrap_or(0);
                return Some(val);
            }
        }
    }

    if method == b"map" {
        // n.times.map — receiver is n.times
        if let Some(recv) = call.receiver() {
            if let Some(times_call) = recv.as_call_node() {
                if times_call.name().as_slice() == b"times" {
                    if let Some(int_recv) = times_call.receiver() {
                        if let Some(int) = int_recv.as_integer_node() {
                            let val: i64 = int.value().try_into().unwrap_or(0);
                            return Some(val);
                        }
                    }
                }
            }
        }
    }

    if method == b"new" {
        // Array.new(n)
        if let Some(recv) = call.receiver() {
            let is_array = if let Some(cr) = recv.as_constant_read_node() {
                cr.name().as_slice() == b"Array"
            } else if let Some(cp) = recv.as_constant_path_node() {
                cp.name().map_or(false, |n| n.as_slice() == b"Array")
            } else {
                false
            };

            if is_array {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if let Some(int) = arg_list.first().and_then(|a| a.as_integer_node()) {
                        let val: i64 = int.value().try_into().unwrap_or(0);
                        return Some(val);
                    }
                }
            }
        }
    }

    None
}

/// Check if a block has named parameters that are actually used in the body.
fn block_arg_is_used(block: &ruby_prism::BlockNode<'_>) -> bool {
    let params = match block.parameters() {
        Some(p) => p,
        None => return false,
    };

    let block_params = match params.as_block_parameters_node() {
        Some(bp) => bp,
        None => return false,
    };

    let inner_params = match block_params.parameters() {
        Some(p) => p,
        None => return false,
    };

    // Get parameter names
    let param_names: Vec<Vec<u8>> = inner_params
        .requireds()
        .iter()
        .filter_map(|p| p.as_required_parameter_node())
        .map(|p| p.name().as_slice().to_vec())
        .collect();

    if param_names.is_empty() {
        return false;
    }

    // Check if any param is used in arguments of the create call in the body
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };

    // Simple check: see if the param name appears as a local variable read in the body
    has_local_var_read(&body, &param_names)
}

fn has_local_var_read(node: &ruby_prism::Node<'_>, names: &[Vec<u8>]) -> bool {
    if let Some(lvar) = node.as_local_variable_read_node() {
        if names.iter().any(|n| lvar.name().as_slice() == n.as_slice()) {
            return true;
        }
    }

    // Recurse into children
    for child in node.child_nodes().iter().flatten() {
        if has_local_var_read(&child, names) {
            return true;
        }
    }

    false
}

/// Get the create call from a block body (single statement only).
fn get_single_create_call<'a>(
    body: &ruby_prism::Node<'a>,
) -> Option<ruby_prism::CallNode<'a>> {
    // If body is a statements node, it must have exactly one child
    if let Some(stmts) = body.as_statements_node() {
        let children: Vec<_> = stmts.body().iter().collect();
        if children.len() != 1 {
            return None;
        }
        // The child can be a call node or a block node wrapping a call
        let child = &children[0];
        if let Some(c) = child.as_call_node() {
            return Some(c);
        }
        if let Some(b) = child.as_block_node() {
            if let Some(c) = b.call().as_call_node() {
                return Some(c);
            }
        }
        return None;
    }

    // Single node body
    if let Some(c) = body.as_call_node() {
        return Some(c);
    }
    if let Some(b) = body.as_block_node() {
        if let Some(c) = b.call().as_call_node() {
            return Some(c);
        }
    }
    None
}

/// Check if arguments to create include a method call (like `rand`).
fn arguments_include_method_call(args: &[ruby_prism::Node<'_>]) -> bool {
    // Skip the first arg (factory name)
    for arg in args.iter().skip(1) {
        if contains_send_node(arg) {
            return true;
        }
    }
    false
}

fn contains_send_node(node: &ruby_prism::Node<'_>) -> bool {
    // Direct call node with arguments that's not a keyword hash
    if let Some(call) = node.as_call_node() {
        // Only flag if it's a real method call (not just a symbol/literal)
        if call.receiver().is_some() || call.arguments().is_some() || call.block().is_some() {
            return true;
        }
        // A bare method call like `rand`
        if call.receiver().is_none() && call.arguments().is_none() {
            return true;
        }
    }

    // Check keyword hash values
    if let Some(hash) = node.as_keyword_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(pair) = elem.as_assoc_node() {
                if contains_send_node(&pair.value()) {
                    return true;
                }
            }
        }
    }

    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(pair) = elem.as_assoc_node() {
                if contains_send_node(&pair.value()) {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CreateList, "cops/factory_bot/create_list");
}
