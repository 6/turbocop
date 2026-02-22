use crate::cop::node_type::{
    BLOCK_ARGUMENT_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AutoResourceCleanup;

fn is_resource_class(node: &ruby_prism::Node<'_>) -> bool {
    let name = if let Some(read) = node.as_constant_read_node() {
        std::str::from_utf8(read.name().as_slice()).unwrap_or("")
    } else if let Some(path) = node.as_constant_path_node() {
        std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("")
    } else {
        return false;
    };
    matches!(name, "File" | "Tempfile")
}

impl Cop for AutoResourceCleanup {
    fn name(&self) -> &'static str {
        "Style/AutoResourceCleanup"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_ARGUMENT_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "open" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_resource_class(&receiver) {
            return;
        }

        // Skip if it has a block
        if call.block().is_some() {
            return;
        }

        // Skip if it has a block argument (&:read etc)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if arg.as_block_argument_node().is_some() {
                    return;
                }
            }
        }

        // Skip if followed by .close (method chain)
        // We can't easily detect this from the node itself, so we check the
        // source bytes after the call
        let loc = node.location();
        let end_offset = loc.end_offset();
        let src_bytes = source.as_bytes();
        // Check if .close follows
        if end_offset < src_bytes.len() {
            let rest = &src_bytes[end_offset..];
            let rest_str = std::str::from_utf8(rest).unwrap_or("");
            let trimmed = rest_str.trim_start();
            if trimmed.starts_with(".close") {
                return;
            }
        }

        let recv_str = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("File");
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use the block version of `{}.open`.", recv_str),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AutoResourceCleanup, "cops/style/auto_resource_cleanup");
}
