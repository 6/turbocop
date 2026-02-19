use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};

pub struct DateTime;

impl Cop for DateTime {
    fn name(&self) -> &'static str {
        "Style/DateTime"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let allow_coercion = config.get_bool("AllowCoercion", false);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        // Check for .to_datetime calls
        if method_name == "to_datetime" {
            if allow_coercion {
                return;
            }
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `#to_datetime`.".to_string(),
            ));
        }

        // Check for DateTime.something calls
        if let Some(receiver) = call.receiver() {
            let is_datetime = is_datetime_const(&receiver);
            if !is_datetime {
                return;
            }

            // DateTime.iso8601('date', Date::ENGLAND) has 2 args - historic date, skip
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() >= 2 {
                    return;
                }
            }

            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `Time` over `DateTime`.".to_string(),
            ));
        }

    }
}

fn is_datetime_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == "DateTime";
    }
    if let Some(path) = node.as_constant_path_node() {
        // Check ::DateTime
        let name = std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("");
        if name == "DateTime" {
            // Make sure it's ::DateTime (parent is None/root) not Something::DateTime
            if path.parent().is_none() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DateTime, "cops/style/date_time");
}
