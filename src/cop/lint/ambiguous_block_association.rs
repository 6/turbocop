use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AmbiguousBlockAssociation;

impl Cop for AmbiguousBlockAssociation {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousBlockAssociation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // We look for CallNode where:
        // 1. The outer call has no parentheses (opening_loc is None)
        // 2. It has arguments
        // 3. The last argument is a CallNode that has a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must not have parentheses on the outer call
        if call.opening_loc().is_some() {
            return Vec::new();
        }

        // Skip operator methods (==, !=, +, -, etc.) and assignment methods (x=)
        let outer_name = call.name().as_slice();
        if is_operator(outer_name) {
            return Vec::new();
        }
        if outer_name.ends_with(b"=") && outer_name != b"==" && outer_name != b"!=" {
            return Vec::new();
        }

        // Must have a message_loc (named method call, not just a block)
        if call.message_loc().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.is_empty() {
            return Vec::new();
        }

        // Check the last argument - it should be a CallNode with a block
        let last_arg = args.iter().last().unwrap();

        // Skip lambda/proc/Proc.new — these are block builders, not ambiguous
        if is_lambda_or_proc(&last_arg) {
            return Vec::new();
        }

        let inner_call = match last_arg.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // The inner call must have a real block (do...end or { }),
        // not just a block argument (&method). In Prism, block() returns
        // both BlockNode and BlockArgumentNode. We only care about BlockNode.
        let has_real_block = match inner_call.block() {
            Some(block) => block.as_block_node().is_some(),
            None => false,
        };
        if !has_real_block {
            return Vec::new();
        }

        // If the inner call has arguments (parenthesized or not), the block
        // clearly belongs to it — no ambiguity. RuboCop checks:
        // `!send_node.last_argument.send_node.arguments?`
        if inner_call.arguments().is_some() {
            return Vec::new();
        }

        // Check AllowedMethods
        let allowed_methods = config.get_string_array("AllowedMethods");
        let inner_name = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
        if let Some(ref methods) = allowed_methods {
            if methods.iter().any(|m| m == inner_name) {
                return Vec::new();
            }
        }

        // Check AllowedPatterns
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        if let Some(ref patterns) = allowed_patterns {
            // Get the full source text of the arguments for pattern matching
            let args_start = arguments.location().start_offset();
            let args_end = arguments.location().end_offset();
            let args_text =
                std::str::from_utf8(&source.as_bytes()[args_start..args_end]).unwrap_or("");
            for pattern in patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(args_text) {
                        return Vec::new();
                    }
                }
            }
        }

        // Build the param text from the inner call (method + block)
        let inner_start = inner_call.location().start_offset();
        let inner_end = inner_call.location().end_offset();
        let param_text =
            std::str::from_utf8(&source.as_bytes()[inner_start..inner_end]).unwrap_or("...");

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Parenthesize the param `{}` to make sure that the block will be associated with the `{}` method call.",
                param_text, inner_name
            ),
        )]
    }
}

/// Check if the node is a `lambda { }`, `proc { }`, or `Proc.new { }` call.
/// These are block builders and their block association is never ambiguous.
fn is_lambda_or_proc(node: &ruby_prism::Node<'_>) -> bool {
    // `-> { }` is a LambdaNode in Prism, not a CallNode
    if node.as_lambda_node().is_some() {
        return true;
    }

    if let Some(call) = node.as_call_node() {
        // Must have a block
        if call.block().is_none() {
            return false;
        }

        let name = call.name().as_slice();

        // `lambda { }` or `proc { }` — bare method call, no receiver
        if call.receiver().is_none() && (name == b"lambda" || name == b"proc") {
            return true;
        }

        // `Proc.new { }` — receiver is `Proc`, method is `new`
        if name == b"new" {
            if let Some(recv) = call.receiver() {
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() == b"Proc" {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn is_operator(name: &[u8]) -> bool {
    matches!(
        name,
        b"==" | b"!=" | b"<" | b">" | b"<=" | b">=" | b"<=>"
            | b"+" | b"-" | b"*" | b"/" | b"%" | b"**"
            | b"&" | b"|" | b"^" | b"~" | b"<<" | b">>"
            | b"[]" | b"[]=" | b"=~" | b"!~"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AmbiguousBlockAssociation, "cops/lint/ambiguous_block_association");
}
