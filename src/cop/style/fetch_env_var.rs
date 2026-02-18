use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct FetchEnvVar;

impl FetchEnvVar {
    fn is_env_receiver(node: &ruby_prism::Node<'_>) -> bool {
        // Simple constant: ENV
        if node.as_constant_read_node()
            .map_or(false, |c| c.name().as_slice() == b"ENV")
        {
            return true;
        }
        // Qualified constant: ::ENV (constant_path_node with no parent)
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() && cp.name().map_or(false, |n| n.as_slice() == b"ENV") {
                return true;
            }
        }
        false
    }

    fn is_env_bracket_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"[]" {
                if let Some(receiver) = call.receiver() {
                    return Self::is_env_receiver(&receiver);
                }
            }
        }
        false
    }
}

impl Cop for FetchEnvVar {
    fn name(&self) -> &'static str {
        "Style/FetchEnvVar"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allowed_vars = config.get_string_array("AllowedVars");
        let default_to_nil = config.get_bool("DefaultToNil", true);

        let mut visitor = FetchEnvVarVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_vars,
            default_to_nil,
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct FetchEnvVarVisitor<'a> {
    cop: &'a FetchEnvVar,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_vars: Option<Vec<String>>,
    default_to_nil: bool,
}

impl<'pr> Visit<'pr> for FetchEnvVarVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name();
        let method_bytes = name.as_slice();

        // Check for ENV['X'] that is the receiver of another method call.
        // In that context, ENV['X'] becomes a receiver of the outer call,
        // so we check the outer call instead.
        if method_bytes != b"[]" {
            // This is an outer call. Check if receiver is ENV['X'] — if so, skip.
            // (ENV['X'].some_method, !ENV['X'], ENV['X'] == 1, etc.)
            // We handle these by NOT reporting ENV['X'] when it's used as a receiver.
            // Just recurse normally — the inner ENV['X'] will be visited but
            // we'll skip it in the check below if it's a receiver here.
        }

        if method_bytes == b"[]" {
            let receiver = match node.receiver() {
                Some(r) => r,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            };

            if !FetchEnvVar::is_env_receiver(&receiver) {
                ruby_prism::visit_call_node(self, node);
                return;
            }

            let args = match node.arguments() {
                Some(a) => a,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            };

            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                ruby_prism::visit_call_node(self, node);
                return;
            }

            let arg_loc = arg_list[0].location();
            let arg_src = &self.source.as_bytes()[arg_loc.start_offset()..arg_loc.end_offset()];
            let arg_str = String::from_utf8_lossy(arg_src);

            // Check AllowedVars
            if let Some(ref allowed) = self.allowed_vars {
                let var_name = arg_str.trim_matches('\'').trim_matches('"');
                if allowed.iter().any(|v| v == var_name) {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            }

            // This is ENV['X'] — mark it as a candidate.
            // We'll store its offset and message, but we might suppress it later.
            let loc = node.location();
            let call_src = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let call_str = String::from_utf8_lossy(call_src);

            let replacement = if self.default_to_nil {
                format!("ENV.fetch({}, nil)", arg_str)
            } else {
                format!("ENV.fetch({})", arg_str)
            };

            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Use `{}` instead of `{}`.", replacement, call_str),
            ));

            // Don't recurse into this node (we already processed it)
            return;
        }

        // For non-[] calls, check if their receiver is ENV['X'].
        // If so, the ENV['X'] should NOT be flagged (it receives a message).
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_bracket_call(&receiver) {
                // Skip visiting the receiver — we handled the suppression by
                // NOT recursing into it.
                // Visit arguments and block only.
                if let Some(args) = node.arguments() {
                    self.visit(&args.as_node());
                }
                if let Some(block) = node.block() {
                    self.visit(&block);
                }
                return;
            }
        }

        // For operator-assignment nodes like `ENV['X'] ||= y`, the ENV['X']
        // node is wrapped in special assignment nodes, not as a receiver.
        // We handle that via visit_or_write_node and visit_and_write_node.

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        // ENV['X'] ||= y creates a CallOperatorWriteNode
        // The receiver ENV is on the "target" side, not a CallNode we'd visit.
        // We skip these entirely — don't recurse.
        // Actually, just recurse normally but the ENV['X'] part is the target
        // which won't be a separate CallNode visited.
        ruby_prism::visit_call_operator_write_node(self, node);
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        // ENV['X'] ||= y  — this is the lhs; don't flag it.
        // The receiver inside is ENV, the method is []=, and this is a compound assignment.
        // The node's receiver is ENV, and we should not report it.
        // Don't recurse into this node's receiver to avoid generating a FP.
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_receiver(&receiver) {
                // This is ENV['X'] ||= y — skip it entirely
                // Only visit the value side
                self.visit(&node.value());
                return;
            }
        }
        ruby_prism::visit_call_or_write_node(self, node);
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        // ENV['X'] &&= y  — same as above
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_receiver(&receiver) {
                self.visit(&node.value());
                return;
            }
        }
        ruby_prism::visit_call_and_write_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FetchEnvVar, "cops/style/fetch_env_var");
}
