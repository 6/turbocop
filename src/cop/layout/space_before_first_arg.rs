use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeFirstArg;

impl Cop for SpaceBeforeFirstArg {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeFirstArg"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_for_alignment = config.get_bool("AllowForAlignment", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Only check calls without parentheses
        if call.opening_loc().is_some() {
            return Vec::new();
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Get the method name location
        let msg_loc = call.message_loc();
        let msg_loc = match msg_loc {
            Some(l) => l,
            None => return Vec::new(),
        };

        let first_arg = match args.arguments().iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let method_end = msg_loc.end_offset();
        let arg_start = first_arg.location().start_offset();

        // Must be on the same line
        let (method_line, _) = source.offset_to_line_col(method_end);
        let (arg_line, _) = source.offset_to_line_col(arg_start);
        if method_line != arg_line {
            return Vec::new();
        }

        let gap = arg_start.saturating_sub(method_end);

        if gap == 0 {
            // No space at all between method name and first arg — always flag
            let (line, column) = source.offset_to_line_col(method_end);
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Put one space between the method name and the first argument.".to_string(),
            )];
        }

        if gap > 1 {
            // When AllowForAlignment is true (default), extra spaces are allowed
            // because they may be used for vertical alignment with adjacent lines.
            if allow_for_alignment {
                return Vec::new();
            }

            // More than one space between method name and first arg
            let bytes = source.as_bytes();
            let between = &bytes[method_end..arg_start];
            if between.iter().all(|&b| b == b' ') {
                let (line, column) = source.offset_to_line_col(method_end);
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Put one space between the method name and the first argument.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeFirstArg, "cops/layout/space_before_first_arg");
}
