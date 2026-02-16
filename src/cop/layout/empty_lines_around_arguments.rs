use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundArguments;

impl Cop for EmptyLinesAroundArguments {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundArguments"
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

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if args_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &args_list[0];
        let last_arg = &args_list[args_list.len() - 1];

        let (first_line, _) = source.offset_to_line_col(first_arg.location().start_offset());
        let last_end = last_arg.location().end_offset().saturating_sub(1);
        let (last_line, _) = source.offset_to_line_col(last_end);

        // Only check multiline argument lists
        if first_line == last_line {
            return Vec::new();
        }

        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        // Check for blank lines within the argument range
        for line_num in first_line..=last_line {
            if line_num > 0 && line_num <= lines.len() {
                let line = lines[line_num - 1];
                if is_blank_line(line) {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        0,
                        "Extra empty line detected inside method arguments.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundArguments,
        "cops/layout/empty_lines_around_arguments"
    );
}
