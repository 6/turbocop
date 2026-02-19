use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct RedundantCurrentDirectoryInPath;

impl Cop for RedundantCurrentDirectoryInPath {
    fn name(&self) -> &'static str {
        "Style/RedundantCurrentDirectoryInPath"
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
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be `require_relative` with no receiver
        if call.name().as_slice() != b"require_relative" {
            return Vec::new();
        }
        if call.receiver().is_some() {
            return Vec::new();
        }

        // Must have exactly one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        // Argument must be a string starting with "./"
        let str_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let content_bytes = str_node.unescaped();
        if !content_bytes.starts_with(b"./") {
            return Vec::new();
        }

        let str_loc = str_node.location();
        let (line, column) = source.offset_to_line_col(str_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Remove the redundant current directory path.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCurrentDirectoryInPath, "cops/style/redundant_current_directory_in_path");
}
