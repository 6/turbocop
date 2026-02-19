use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct SaveBang;

/// Methods that should use the bang (!) version if the return value is not checked.
const PERSIST_METHODS: &[&[u8]] = &[
    b"save",
    b"create",
    b"update",
    b"destroy",
];

/// Methods that should use the bang version (create_or_find_by, etc.)
const FIND_OR_CREATE_METHODS: &[&[u8]] = &[
    b"first_or_create",
    b"find_or_create_by",
];

impl Cop for SaveBang {
    fn name(&self) -> &'static str {
        "Rails/SaveBang"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let _allow_implicit_return = config.get_bool("AllowImplicitReturn", true);
        let _allowed_receivers = config.get_string_array("AllowedReceivers");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        let is_persist = PERSIST_METHODS.contains(&method_name);
        let is_find_or_create = FIND_OR_CREATE_METHODS.contains(&method_name);

        if !is_persist && !is_find_or_create {
            return;
        }

        // `destroy` with arguments is not a persistence method
        if method_name == b"destroy" && call.arguments().is_some() {
            return;
        }

        // `save` or `update` with a string argument (not a hash) is not a persistence call
        if method_name == b"save" || method_name == b"create" || method_name == b"update" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                // If has 2+ positional args (like Model.save(1, name: 'Tom')), skip
                if method_name != b"create" && arg_list.len() >= 2 {
                    return;
                }
                // If single arg is a plain string, skip
                if arg_list.len() == 1 && arg_list[0].as_string_node().is_some() {
                    return;
                }
            }
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("save");

        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{method_str}!` instead of `{method_str}` if the return value is not checked."
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SaveBang, "cops/rails/save_bang");
}
