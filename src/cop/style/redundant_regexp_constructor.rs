use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantRegexpConstructor;

impl Cop for RedundantRegexpConstructor {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpConstructor"
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
        if name != b"new" && name != b"compile" {
            return Vec::new();
        }

        // Receiver must be Regexp constant
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_regexp = if let Some(c) = receiver.as_constant_read_node() {
            c.name().as_slice() == b"Regexp"
        } else if let Some(cp) = receiver.as_constant_path_node() {
            // Handle ::Regexp
            let bytes = &source.as_bytes()[cp.location().start_offset()..cp.location().end_offset()];
            bytes == b"Regexp" || bytes == b"::Regexp"
        } else {
            false
        };

        if !is_regexp {
            return Vec::new();
        }

        // Check if the argument is a regexp literal
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        if arg_list[0].as_regular_expression_node().is_none()
            && arg_list[0].as_interpolated_regular_expression_node().is_none()
        {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `//` around regular expression instead of `Regexp` constructor.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRegexpConstructor, "cops/style/redundant_regexp_constructor");
}
