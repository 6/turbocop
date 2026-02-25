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
            parent_is_bracket: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ArrayFirstLastVisitor<'a> {
    cop: &'a ArrayFirstLast,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Whether the nearest ancestor CallNode is a `[]` or `[]=` call.
    /// Used to suppress `arr[0]` when it appears as receiver or argument
    /// of another bracket call (e.g., `arr[0][:key]`, `hash[arr[0]]`).
    parent_is_bracket: bool,
}

impl<'pr> Visit<'pr> for ArrayFirstLastVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();
        let is_bracket = method_name == b"[]" || method_name == b"[]=";

        // Check if this [] call should produce a diagnostic.
        // Skip if:
        // 1. The receiver is itself a [] call (chained: hash[:key][0])
        // 2. The nearest ancestor CallNode is []/[]= (parent bracket:
        //    arr[0][:key], hash[arr[0]], positions[pair[0]] = val)
        if method_name == b"[]" && !self.parent_is_bracket {
            self.check_call(node);
        }

        // Recurse into children, marking whether this call is a bracket method
        // so descendant [] calls know their parent context.
        let old = self.parent_is_bracket;
        self.parent_is_bracket = is_bracket;
        ruby_prism::visit_call_node(self, node);
        self.parent_is_bracket = old;
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
