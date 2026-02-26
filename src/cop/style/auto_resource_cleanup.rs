use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AutoResourceCleanup;

impl Cop for AutoResourceCleanup {
    fn name(&self) -> &'static str {
        "Style/AutoResourceCleanup"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = AutoResourceCleanupVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct AutoResourceCleanupVisitor<'a> {
    cop: &'a AutoResourceCleanup,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Check if a call node is `File.open(...)` or `Tempfile.open(...)` without a block.
fn is_resource_open_without_block(call: &ruby_prism::CallNode<'_>) -> Option<String> {
    let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
    if method_name != "open" {
        return None;
    }

    let receiver = call.receiver()?;

    let recv_name = if let Some(read) = receiver.as_constant_read_node() {
        std::str::from_utf8(read.name().as_slice()).unwrap_or("")
    } else if let Some(path) = receiver.as_constant_path_node() {
        std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("")
    } else {
        return None;
    };

    if !matches!(recv_name, "File" | "Tempfile") {
        return None;
    }

    // Skip if it has a block
    if call.block().is_some() {
        return None;
    }

    // Skip if it has a block argument (&:read etc)
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if arg.as_block_argument_node().is_some() {
                return None;
            }
        }
    }

    let recv_str = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("File");
    Some(recv_str.to_string())
}

impl<'pr> Visit<'pr> for AutoResourceCleanupVisitor<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // Only flag File.open/Tempfile.open when assigned to a local variable
        if let Some(call) = node.value().as_call_node() {
            if let Some(recv_str) = is_resource_open_without_block(&call) {
                let loc = call.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("Use the block version of `{}.open`.", recv_str),
                ));
            }
        }

        // Recurse into children
        ruby_prism::visit_local_variable_write_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AutoResourceCleanup, "cops/style/auto_resource_cleanup");
}
