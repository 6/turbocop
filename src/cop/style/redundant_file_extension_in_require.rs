use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct RedundantFileExtensionInRequire;

impl Cop for RedundantFileExtensionInRequire {
    fn name(&self) -> &'static str {
        "Style/RedundantFileExtensionInRequire"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be require or require_relative without a receiver
        let method_name = call.name();
        let method_bytes = method_name.as_slice();
        if !matches!(method_bytes, b"require" | b"require_relative") {
            return;
        }
        if call.receiver().is_some() {
            return;
        }

        // Must have exactly one string argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let str_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };

        let content = str_node.content_loc().as_slice();
        if content.ends_with(b".rb") {
            // Point to the .rb extension
            let content_loc = str_node.content_loc();
            let ext_start = content_loc.start_offset() + content.len() - 3;
            let (line, column) = source.offset_to_line_col(ext_start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant `.rb` file extension detected.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFileExtensionInRequire, "cops/style/redundant_file_extension_in_require");
}
