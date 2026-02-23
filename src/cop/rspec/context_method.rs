use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ContextMethod;

impl Cop for ContextMethod {
    fn name(&self) -> &'static str {
        "RSpec/ContextMethod"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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

        if call.name().as_slice() != b"context" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let string_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };

        let content = string_node.unescaped();
        let content_str = match std::str::from_utf8(content) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Flag if starts with '#' or '.'
        if !content_str.starts_with('#') && !content_str.starts_with('.') {
            return;
        }

        let loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `describe` for testing methods.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ContextMethod, "cops/rspec/context_method");
}
