use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MethodCallWithArgsParentheses;

const IGNORED_METHODS: &[&[u8]] = &[
    b"require",
    b"require_relative",
    b"include",
    b"extend",
    b"prepend",
    b"puts",
    b"print",
    b"p",
    b"pp",
    b"raise",
    b"fail",
    b"attr_reader",
    b"attr_writer",
    b"attr_accessor",
    b"private",
    b"protected",
    b"public",
    b"module_function",
    b"gem",
    b"source",
    b"yield",
    b"return",
    b"super",
];

fn is_operator(name: &[u8]) -> bool {
    matches!(
        name,
        b"+" | b"-" | b"*" | b"/" | b"%" | b"**" | b"==" | b"!=" | b"<" | b">" | b"<="
            | b">=" | b"<=>" | b"<<" | b">>" | b"&" | b"|" | b"^" | b"~" | b"!" | b"[]"
            | b"[]=" | b"=~" | b"!~" | b"+@" | b"-@"
    )
}

impl Cop for MethodCallWithArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/MethodCallWithArgsParentheses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        // Skip operators
        if is_operator(name) {
            return Vec::new();
        }

        // Skip ignored methods
        if IGNORED_METHODS.contains(&name) {
            return Vec::new();
        }

        // Must have arguments
        if call.arguments().is_none() {
            return Vec::new();
        }

        // Check if parens are present (opening_loc is Some when parens are used)
        if call.opening_loc().is_some() {
            return Vec::new();
        }

        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use parentheses for method calls with arguments.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(MethodCallWithArgsParentheses, "cops/style/method_call_with_args_parentheses");

    #[test]
    fn operators_are_ignored() {
        let source = b"x = 1 + 2\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn method_without_args_is_ok() {
        let source = b"foo.bar\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty());
    }
}
