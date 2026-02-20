use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE};

pub struct FileWrite;

impl FileWrite {
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

    fn is_write_mode(mode: &[u8]) -> bool {
        matches!(mode, b"w" | b"wb" | b"w+" | b"wb+" | b"w+b")
    }
}

impl Cop for FileWrite {
    fn name(&self) -> &'static str {
        "Style/FileWrite"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE]
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

        // Pattern: File.open(filename, 'w').write(content)
        if call.name().as_slice() != b"write" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let open_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if open_call.name().as_slice() != b"open" {
            return;
        }

        let file_recv = match open_call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !Self::is_file_class(&file_recv) {
            return;
        }

        // Check mode argument
        let open_args = match open_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let open_arg_list: Vec<_> = open_args.arguments().iter().collect();
        if open_arg_list.len() < 2 {
            return;
        }

        if let Some(str_node) = open_arg_list[1].as_string_node() {
            let content: &[u8] = &str_node.unescaped();
            if !Self::is_write_mode(content) {
                return;
            }
            let is_binary = content.contains(&b'b');
            let write_method = if is_binary { "File.binwrite" } else { "File.write" };

            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Use `{}`.", write_method),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileWrite, "cops/style/file_write");
}
