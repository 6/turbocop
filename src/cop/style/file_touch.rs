use crate::cop::node_type::{
    BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FileTouch;

impl FileTouch {
    fn is_file_class(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(c) = node.as_constant_read_node() {
            return c.name().as_slice() == b"File";
        }
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() {
                return cp.name().map_or(false, |n| n.as_slice() == b"File");
            }
        }
        false
    }
}

impl Cop for FileTouch {
    fn name(&self) -> &'static str {
        "Style/FileTouch"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            STRING_NODE,
        ]
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

        // File.open(filename, 'a') {}
        if call.name().as_slice() != b"open" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !Self::is_file_class(&receiver) {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        // Block must be empty (no body)
        if let Some(block_node) = block.as_block_node() {
            if block_node.body().is_some() {
                return;
            }
        } else {
            return;
        }

        // Must have 'a' mode argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return;
        }

        // Second arg should be 'a'
        if let Some(str_node) = arg_list[1].as_string_node() {
            let content: &[u8] = &str_node.unescaped();
            if content != b"a" {
                return;
            }
        } else {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // Get filename argument source
        let fname_src = &source.as_bytes()
            [arg_list[0].location().start_offset()..arg_list[0].location().end_offset()];
        let fname_str = String::from_utf8_lossy(fname_src);

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `FileUtils.touch({})` instead of `File.open` in append mode with empty block.",
                fname_str
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileTouch, "cops/style/file_touch");
}
