use crate::cop::util::{self, is_rspec_example, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, FLOAT_NODE, INTEGER_NODE, INTERPOLATED_X_STRING_NODE, REGULAR_EXPRESSION_NODE, X_STRING_NODE};

pub struct UndescriptiveLiteralsDescription;

impl Cop for UndescriptiveLiteralsDescription {
    fn name(&self) -> &'static str {
        "RSpec/UndescriptiveLiteralsDescription"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FLOAT_NODE, INTEGER_NODE, INTERPOLATED_X_STRING_NODE, REGULAR_EXPRESSION_NODE, X_STRING_NODE]
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

        // Must be an example group or example method
        if !is_rspec_example_group(method_name)
            && !is_rspec_example(method_name)
        {
            return;
        }

        // Must be receiverless or RSpec.describe / ::RSpec.describe
        if let Some(recv) = call.receiver() {
            if util::constant_name(&recv).map_or(true, |n| n != b"RSpec") {
                return;
            }
        }

        // Get first positional argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];

        // Check if the first argument is a literal (integer, float, regex, xstring)
        // but not a string, constant, or method call
        let is_undescriptive = first_arg.as_integer_node().is_some()
            || first_arg.as_float_node().is_some()
            || first_arg.as_regular_expression_node().is_some()
            || first_arg.as_x_string_node().is_some()
            || first_arg.as_interpolated_x_string_node().is_some();

        if !is_undescriptive {
            return;
        }

        let loc = first_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Description should be descriptive.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UndescriptiveLiteralsDescription, "cops/rspec/undescriptive_literals_description");
}
