use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArrayFirstLast;

impl Cop for ArrayFirstLast {
    fn name(&self) -> &'static str {
        "Style/ArrayFirstLast"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = ArrayFirstLastVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            suppressed_offsets: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ArrayFirstLastVisitor<'a> {
    cop: &'a ArrayFirstLast,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Byte start offsets of `[]` call nodes that should NOT be flagged because
    /// they are a direct child (receiver or argument) of another `[]/[]=` call.
    /// This mirrors RuboCop's `innermost_braces_node` + `brace_method?(parent)` check.
    suppressed_offsets: Vec<usize>,
}

/// Check if a call node is a `[]` method call.
fn is_bracket_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.name().as_slice() == b"[]"
}

/// Walk down the receiver chain of `[]` calls, adding each intermediate
/// `[]` node's offset to the suppressed set. For `a[0][1][2]`, visiting
/// the outermost `[2]` adds `a[0][1]` and `a[0]` as suppressed.
fn suppress_bracket_receiver_chain(node: &ruby_prism::CallNode<'_>, suppressed: &mut Vec<usize>) {
    let mut current_recv = node.receiver();
    while let Some(recv) = current_recv {
        if let Some(recv_call) = recv.as_call_node() {
            if is_bracket_call(&recv_call) {
                suppressed.push(recv_call.location().start_offset());
                current_recv = recv_call.receiver();
                continue;
            }
        }
        break;
    }
}

/// Suppress a `[]` argument node and walk its receiver chain.
fn suppress_bracket_arg(arg_call: &ruby_prism::CallNode<'_>, suppressed: &mut Vec<usize>) {
    suppressed.push(arg_call.location().start_offset());
    suppress_bracket_receiver_chain(arg_call, suppressed);
}

impl<'pr> Visit<'pr> for ArrayFirstLastVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();
        let is_bracket = method_name == b"[]" || method_name == b"[]=";

        // When entering a []/[]= call, suppress [] calls that are direct
        // children (receiver or arguments). This matches RuboCop's behavior:
        // only suppress arr[0] when its immediate parent in the AST is []/[]=.
        if is_bracket {
            // Suppress receiver if it's a [] call (chained: arr[0][:key])
            // Also walk the chain deeper (arr[0][1][:key] → suppress arr[0][1] and arr[0])
            suppress_bracket_receiver_chain(node, &mut self.suppressed_offsets);

            // Suppress arguments that are [] calls (nested: hash[arr[0]])
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if let Some(arg_call) = arg.as_call_node() {
                        if is_bracket_call(&arg_call) {
                            suppress_bracket_arg(&arg_call, &mut self.suppressed_offsets);
                        }
                    }
                }
            }
        }

        // Check if this [] call should produce a diagnostic.
        if method_name == b"[]" {
            self.check_call(node);
        }

        // Recurse into children
        ruby_prism::visit_call_node(self, node);
    }
}

impl ArrayFirstLastVisitor<'_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Skip if receiver is itself a [] call (chained indexing like hash[:key][0])
        if let Some(recv_call) = receiver.as_call_node() {
            if recv_call.name().as_slice() == b"[]" {
                return;
            }
        }

        // Skip if this call is suppressed (it's a direct child of another []/[]= call)
        if self
            .suppressed_offsets
            .contains(&call.location().start_offset())
        {
            return;
        }

        // Must have exactly one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];

        // Check for integer literal 0 or -1
        if let Some(int_node) = arg.as_integer_node() {
            let src = std::str::from_utf8(int_node.location().as_slice()).unwrap_or("");
            if let Ok(v) = src.parse::<i64>() {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());

                if v == 0 {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `first`.".to_string(),
                    ));
                } else if v == -1 {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `last`.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayFirstLast, "cops/style/array_first_last");
}
