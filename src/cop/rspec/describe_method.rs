use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE};

pub struct DescribeMethod;

impl Cop for DescribeMethod {
    fn name(&self) -> &'static str {
        "RSpec/DescribeMethod"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"describe" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();

        // Need at least 2 args: a class and a string description
        if arg_list.len() < 2 {
            return;
        }

        // First argument should be a class/constant
        if arg_list[0].as_constant_read_node().is_none()
            && arg_list[0].as_constant_path_node().is_none()
        {
            return;
        }

        // Second argument should be a string
        let string_arg = if let Some(s) = arg_list[1].as_string_node() {
            s
        } else {
            return;
        };

        let content = string_arg.unescaped();
        let content_str = match std::str::from_utf8(&content) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Method descriptions must start with '#' or '.'
        if content_str.starts_with('#') || content_str.starts_with('.') {
            return;
        }

        let loc = arg_list[1].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "The second argument to describe should be the method being tested. '#instance' or '.class'.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DescribeMethod, "cops/rspec/describe_method");
}
