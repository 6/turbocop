use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstArgumentIndentation;

impl Cop for FirstArgumentIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstArgumentIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let args_node = match call_node.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args: Vec<_> = args_node.arguments().iter().collect();
        if args.is_empty() {
            return Vec::new();
        }

        let first_arg = &args[0];

        // Determine the call start location
        let call_loc = call_node.location();
        let (call_line, _) = source.offset_to_line_col(call_loc.start_offset());
        let call_line_indent = {
            let line_bytes = source.lines().nth(call_line - 1).unwrap_or(b"");
            crate::cop::util::indentation_of(line_bytes)
        };

        let first_arg_loc = first_arg.location();
        let (arg_line, arg_col) = source.offset_to_line_col(first_arg_loc.start_offset());

        // Skip if first arg is on same line as method call
        if arg_line == call_line {
            return Vec::new();
        }

        let width = config.get_usize("IndentationWidth", 2);

        let expected = call_line_indent + width;

        if arg_col != expected {
            return vec![self.diagnostic(
                source,
                arg_line,
                arg_col,
                "Indent the first argument one step more than the start of the previous line.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        FirstArgumentIndentation,
        "cops/layout/first_argument_indentation"
    );

    #[test]
    fn args_on_same_line_ignored() {
        let source = b"foo(1, 2, 3)\n";
        let diags = run_cop_full(&FirstArgumentIndentation, source);
        assert!(diags.is_empty());
    }
}
