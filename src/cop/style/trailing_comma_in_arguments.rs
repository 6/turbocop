use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingCommaInArguments;

impl Cop for TrailingCommaInArguments {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInArguments"
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

        let closing_loc = match call_node.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let arguments = match call_node.arguments() {
            Some(args) => args,
            None => return Vec::new(),
        };

        let arg_list = arguments.arguments();
        let last_arg = match arg_list.last() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let last_end = last_arg.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();
        let has_comma = has_trailing_comma(bytes, last_end, closing_start);

        let style = config.get_str("EnforcedStyleForMultiline", "no_comma");
        let last_line = source.offset_to_line_col(last_end).0;
        let close_line = source.offset_to_line_col(closing_start).0;
        let is_multiline = close_line > last_line;

        match style {
            "comma" | "consistent_comma" => {
                if is_multiline && !has_comma {
                    let (line, column) = source.offset_to_line_col(last_end);
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Put a comma after the last parameter of a multiline method call."
                            .to_string(),
                    )];
                }
            }
            _ => {
                if has_comma {
                    let search_range = &bytes[last_end..closing_start];
                    if let Some(comma_offset) =
                        search_range.iter().position(|&b| b == b',')
                    {
                        let abs_offset = last_end + comma_offset;
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last parameter of a method call.".to_string(),
                        )];
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        TrailingCommaInArguments,
        "cops/style/trailing_comma_in_arguments"
    );
}
