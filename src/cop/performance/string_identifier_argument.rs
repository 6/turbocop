use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct StringIdentifierArgument;

const METHODS: &[&[u8]] = &[
    b"send",
    b"public_send",
    b"__send__",
    b"respond_to?",
    b"method",
    b"instance_variable_get",
    b"instance_variable_set",
];

impl Cop for StringIdentifierArgument {
    fn name(&self) -> &'static str {
        "Performance/StringIdentifierArgument"
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
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if !METHODS.iter().any(|&m| m == method_name) {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.is_empty() {
            return;
        }

        // Check if first argument is a StringNode
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return,
        };
        if first_arg.as_string_node().is_none() {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use a symbol instead of a string for method identifier arguments.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringIdentifierArgument, "cops/performance/string_identifier_argument");
}
