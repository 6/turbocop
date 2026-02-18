use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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

        // Pattern: File.open(filename, 'w').write(content)
        if call.name().as_slice() != b"write" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let open_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if open_call.name().as_slice() != b"open" {
            return Vec::new();
        }

        let file_recv = match open_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if !Self::is_file_class(&file_recv) {
            return Vec::new();
        }

        // Check mode argument
        let open_args = match open_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let open_arg_list: Vec<_> = open_args.arguments().iter().collect();
        if open_arg_list.len() < 2 {
            return Vec::new();
        }

        if let Some(str_node) = open_arg_list[1].as_string_node() {
            let content = str_node.unescaped();
            if !Self::is_write_mode(content.as_slice()) {
                return Vec::new();
            }
            let is_binary = content.as_slice().contains(&b'b');
            let write_method = if is_binary { "File.binwrite" } else { "File.write" };

            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Use `{}`.", write_method),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileWrite, "cops/style/file_write");
}
