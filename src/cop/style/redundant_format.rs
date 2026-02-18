use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantFormat;

impl Cop for RedundantFormat {
    fn name(&self) -> &'static str {
        "Style/RedundantFormat"
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

        let method_bytes = call.name().as_slice();
        if method_bytes != b"format" && method_bytes != b"sprintf" {
            return Vec::new();
        }

        // Must be called without a receiver, or on Kernel/::Kernel
        if let Some(receiver) = call.receiver() {
            let is_kernel = if let Some(cr) = receiver.as_constant_read_node() {
                cr.name().as_slice() == b"Kernel"
            } else if let Some(cp) = receiver.as_constant_path_node() {
                cp.parent().is_none() && cp.name().map(|n| n.as_slice() == b"Kernel").unwrap_or(false)
            } else {
                false
            };
            if !is_kernel {
                return Vec::new();
            }
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

        let arg = &arg_list[0];

        // The argument must be a string (not needing formatting)
        if arg.as_string_node().is_some() || arg.as_interpolated_string_node().is_some() {
            // Check it's not a splat
            if arg.as_splat_node().is_some() {
                return Vec::new();
            }

            let method_str = std::str::from_utf8(method_bytes).unwrap_or("format");
            let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or("");
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Use `{arg_src}` directly instead of `{method_str}`."),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFormat, "cops/style/redundant_format");
}
