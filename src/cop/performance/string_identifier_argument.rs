use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct StringIdentifierArgument;

// All methods that accept identifier arguments (matches RuboCop's RESTRICT_ON_SEND)
const ALL_METHODS: &[&[u8]] = &[
    // Standard methods (receiver required or optional)
    b"class_variable_defined?",
    b"const_set",
    b"define_method",
    b"instance_method",
    b"method_defined?",
    b"private_class_method?",
    b"private_method_defined?",
    b"protected_method_defined?",
    b"public_class_method",
    b"public_instance_method",
    b"public_method_defined?",
    b"remove_class_variable",
    b"remove_method",
    b"undef_method",
    b"class_variable_get",
    b"class_variable_set",
    b"deprecate_constant",
    b"remove_const",
    b"ruby2_keywords",
    b"define_singleton_method",
    b"instance_variable_defined?",
    b"instance_variable_get",
    b"instance_variable_set",
    b"method",
    b"public_method",
    b"public_send",
    b"remove_instance_variable",
    b"respond_to?",
    b"send",
    b"singleton_method",
    b"__send__",
    // Command methods (only when receiverless)
    b"alias_method",
    b"attr_accessor",
    b"attr_reader",
    b"attr_writer",
    b"autoload",
    b"autoload?",
    b"private",
    b"private_constant",
    b"protected",
    b"public",
    b"public_constant",
    b"module_function",
];

// Command methods: only flagged when receiverless
const COMMAND_METHODS: &[&[u8]] = &[
    b"alias_method",
    b"attr_accessor",
    b"attr_reader",
    b"attr_writer",
    b"autoload",
    b"autoload?",
    b"private",
    b"private_constant",
    b"protected",
    b"public",
    b"public_constant",
    b"module_function",
];

// alias_method checks both arguments
const TWO_ARGUMENTS_METHOD: &[u8] = b"alias_method";

// These methods check ALL arguments
const MULTIPLE_ARGUMENTS_METHODS: &[&[u8]] = &[
    b"attr_accessor",
    b"attr_reader",
    b"attr_writer",
    b"private",
    b"private_constant",
    b"protected",
    b"public",
    b"public_constant",
    b"module_function",
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if !ALL_METHODS.iter().any(|&m| m == method_name) {
            return;
        }

        // Command methods are only flagged when receiverless
        if COMMAND_METHODS.iter().any(|&m| m == method_name) && call.receiver().is_some() {
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

        // Determine which arguments to check
        let is_two_arg_method = method_name == TWO_ARGUMENTS_METHOD;
        let is_multi_arg_method = MULTIPLE_ARGUMENTS_METHODS.iter().any(|&m| m == method_name);

        let args_to_check: Vec<_> = if is_two_arg_method {
            // Check first two arguments
            args.iter().take(2).collect()
        } else if is_multi_arg_method {
            // Check all arguments
            args.iter().collect()
        } else {
            // Check only first argument
            args.iter().take(1).collect()
        };

        for arg in args_to_check {
            let string_node = match arg.as_string_node() {
                Some(s) => s,
                None => continue,
            };

            // Skip interpolated strings (InterpolatedStringNode would not be a StringNode)
            // StringNode in Prism is always a plain string literal

            // Get the string content
            let content = string_node.unescaped();

            // Skip strings containing spaces
            if content.contains(&b' ') {
                continue;
            }

            // Skip strings containing :: (namespace separator)
            if content.windows(2).any(|w| w == b"::") {
                continue;
            }

            // Build the symbol replacement for the message
            let content_str = std::str::from_utf8(content).unwrap_or("?");
            let symbol_str = format!(":{}", content_str);

            // Get the source text of the string argument for the message
            let arg_loc = arg.location();
            let arg_source = std::str::from_utf8(arg_loc.as_slice()).unwrap_or("'?'");

            let (line, column) = source.offset_to_line_col(arg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Use `{}` instead of `{}`.", symbol_str, arg_source),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringIdentifierArgument, "cops/performance/string_identifier_argument");
}
