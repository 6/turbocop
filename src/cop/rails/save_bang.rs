use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct SaveBang;

/// Methods that should use the bang (!) version if the return value is not checked.
const PERSIST_METHODS: &[&[u8]] = &[b"save", b"create", b"update", b"destroy"];

/// Methods that should use the bang version (create_or_find_by, etc.)
const FIND_OR_CREATE_METHODS: &[&[u8]] = &[b"first_or_create", b"find_or_create_by"];

impl Cop for SaveBang {
    fn name(&self) -> &'static str {
        "Rails/SaveBang"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_implicit_return = config.get_bool("AllowImplicitReturn", true);
        let allowed_receivers = config
            .get_string_array("AllowedReceivers")
            .unwrap_or_default();

        let mut visitor = SaveBangVisitor {
            cop: self,
            source,
            allow_implicit_return,
            allowed_receivers,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SaveBangVisitor<'a, 'src> {
    cop: &'a SaveBang,
    source: &'src SourceFile,
    allow_implicit_return: bool,
    allowed_receivers: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

impl SaveBangVisitor<'_, '_> {
    /// Check if a node is a persist call that should be flagged.
    fn is_persist_call(&self, node: &ruby_prism::Node<'_>) -> bool {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return false,
        };

        let method_name = call.name().as_slice();

        let is_persist = PERSIST_METHODS.contains(&method_name);
        let is_find_or_create = FIND_OR_CREATE_METHODS.contains(&method_name);

        if !is_persist && !is_find_or_create {
            return false;
        }

        // `destroy` with arguments is not a persistence method
        if method_name == b"destroy" && call.arguments().is_some() {
            return false;
        }

        // `save` or `update` with a string argument (not a hash) is not a persistence call
        if method_name == b"save" || method_name == b"create" || method_name == b"update" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                // If has 2+ positional args (like Model.save(1, name: 'Tom')), skip
                if method_name != b"create" && arg_list.len() >= 2 {
                    return false;
                }
                // If single arg is a plain string, skip
                if arg_list.len() == 1 && arg_list[0].as_string_node().is_some() {
                    return false;
                }
            }
        }

        // Check allowed receivers
        if !self.allowed_receivers.is_empty() {
            if let Some(receiver) = call.receiver() {
                let recv_src = &self.source.as_bytes()
                    [receiver.location().start_offset()..receiver.location().end_offset()];
                let recv_str = std::str::from_utf8(recv_src).unwrap_or("");
                if self.allowed_receivers.iter().any(|r| r == recv_str) {
                    return false;
                }
            }
        }

        true
    }

    fn flag_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("save");
        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!(
                "Use `{method_name}!` instead of `{method_name}` if the return value is not checked."
            ),
        ));
    }
}

impl<'pr> Visit<'pr> for SaveBangVisitor<'_, '_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let body: Vec<_> = node.body().iter().collect();
        let len = body.len();

        for (i, stmt) in body.iter().enumerate() {
            let is_last = i == len - 1;

            // Skip last statement if AllowImplicitReturn — its value may be used
            // as the return value of the enclosing method/block/lambda.
            if is_last && self.allow_implicit_return {
                continue;
            }

            // A bare persist call as a direct statement is in void context —
            // the return value is discarded. If it's wrapped in an assignment,
            // condition, return, argument list, boolean expression, etc., then
            // the call won't be a direct child of StatementsNode.
            if self.is_persist_call(stmt) {
                if let Some(call) = stmt.as_call_node() {
                    self.flag_call(&call);
                }
            }
        }

        // Continue walking into children for nested scopes
        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SaveBang, "cops/rails/save_bang");
}
