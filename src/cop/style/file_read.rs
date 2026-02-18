use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FileRead;

impl FileRead {
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

impl Cop for FileRead {
    fn name(&self) -> &'static str {
        "Style/FileRead"
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

        // Pattern: File.open(filename).read
        if call.name().as_slice() != b"read" {
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

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `File.read`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileRead, "cops/style/file_read");
}
