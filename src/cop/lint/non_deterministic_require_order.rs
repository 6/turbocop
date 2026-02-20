use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, ELSE_NODE, IF_NODE, STATEMENTS_NODE, SYMBOL_NODE};

pub struct NonDeterministicRequireOrder;

impl Cop for NonDeterministicRequireOrder {
    fn name(&self) -> &'static str {
        "Lint/NonDeterministicRequireOrder"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, ELSE_NODE, IF_NODE, STATEMENTS_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RuboCop: maximum_target_ruby_version 2.7
        // Dir.glob and Dir[] return sorted results in Ruby 3.0+, making this cop
        // unnecessary. Skip if the target Ruby version is 3.0 or later.
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version >= 3.0 {
            return;
        }
        // Pattern 1: Dir["..."].each { |f| require f }
        // Pattern 2: Dir.glob("...").each { |f| require f }
        // Pattern 3: Dir.glob("...") { |f| require f } (direct block)
        //
        // Offense if there's no `.sort` before `.each`.
        // On Ruby >= 3.0, Dir.glob and Dir[] return sorted results,
        // so this cop only fires for Ruby < 3.0. However, many projects
        // still target older Rubies, so we flag it.
        //
        // We check for CallNode where the method is `each` and the receiver
        // is Dir["..."] or Dir.glob("...") (without .sort in between).

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for `.each` pattern
        if method_name == b"each" {
            let recv = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            // The block must contain a require/require_relative
            if !block_contains_require(&call) {
                return;
            }

            // Check if receiver is Dir[...] or Dir.glob(...)
            if is_dir_glob_or_index(&recv) {
                // Check there's no .sort before .each
                // If the receiver is a CallNode to `sort`, it's fine
                if let Some(recv_call) = recv.as_call_node() {
                    if recv_call.name().as_slice() == b"sort" {
                        return;
                    }
                }

                let loc = recv.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Sort files before requiring them.".to_string(),
                ));
            }
        }

        // Check for Dir.glob("...") with direct block (no .each)
        if method_name == b"glob" {
            let recv = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            if !is_dir_constant(&recv) {
                return;
            }

            // Must have a block
            if call.block().is_none() {
                // Check arguments for &method(:require) pattern
                if let Some(args) = call.arguments() {
                    let has_require_block_arg = args.arguments().iter().any(|a| {
                        is_require_block_arg(&a)
                    });
                    if has_require_block_arg {
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Sort files before requiring them.".to_string(),
                        ));
                    }
                }
                return;
            }

            // Block must contain require
            if !block_node_contains_require(call.block().as_ref().unwrap()) {
                return;
            }

            let loc = call.location();
            let msg_loc = call.message_loc().unwrap();
            let report_end = msg_loc.end_offset();
            let _ = report_end; // We report on the full call location
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Sort files before requiring them.".to_string(),
            ));
        }

    }
}

fn is_dir_constant(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(name) = constant_name(node) {
        return name == b"Dir";
    }
    false
}

fn is_dir_glob_or_index(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let method = call.name().as_slice();
        if method == b"glob" || method == b"[]" {
            if let Some(recv) = call.receiver() {
                return is_dir_constant(&recv);
            }
        }
    }
    false
}

fn block_contains_require(call: &ruby_prism::CallNode<'_>) -> bool {
    // Check if the block argument is &method(:require)
    // In Prism, `&method(:require)` can be in call.arguments() or call.block()
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if is_require_block_arg(&arg) {
                return true;
            }
        }
    }

    // Check literal block or block argument
    if let Some(block) = call.block() {
        // Check if it's a BlockArgumentNode with &method(:require)
        if is_require_block_arg(&block) {
            return true;
        }
        return block_node_contains_require(&block);
    }

    false
}

fn block_node_contains_require(block: &ruby_prism::Node<'_>) -> bool {
    if let Some(block_node) = block.as_block_node() {
        if let Some(body) = block_node.body() {
            return statements_contain_require(&body);
        }
    }
    false
}

fn statements_contain_require(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            if is_require_call(&stmt) || contains_require_recursive(&stmt) {
                return true;
            }
        }
    }
    false
}

fn is_require_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"require" || name == b"require_relative") && call.receiver().is_none() {
            return true;
        }
    }
    false
}

fn contains_require_recursive(node: &ruby_prism::Node<'_>) -> bool {
    // Check if/else/elsif blocks for require calls
    if let Some(if_node) = node.as_if_node() {
        if let Some(body) = if_node.statements() {
            for stmt in body.body().iter() {
                if is_require_call(&stmt) || contains_require_recursive(&stmt) {
                    return true;
                }
            }
        }
        if let Some(else_clause) = if_node.subsequent() {
            if contains_require_recursive(&else_clause) {
                return true;
            }
        }
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for stmt in stmts.body().iter() {
                if is_require_call(&stmt) || contains_require_recursive(&stmt) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_require_block_arg(node: &ruby_prism::Node<'_>) -> bool {
    // Check for &method(:require) or &method(:require_relative)
    if let Some(block_arg) = node.as_block_argument_node() {
        if let Some(expr) = block_arg.expression() {
            if let Some(call) = expr.as_call_node() {
                if call.name().as_slice() == b"method" && call.receiver().is_none() {
                    if let Some(args) = call.arguments() {
                        let arg_list = args.arguments();
                        if arg_list.len() == 1 {
                            let first = arg_list.iter().next().unwrap();
                            if let Some(sym) = first.as_symbol_node() {
                                let val = sym.unescaped();
                                return val == b"require" || val == b"require_relative";
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NonDeterministicRequireOrder,
        "cops/lint/non_deterministic_require_order"
    );
}
