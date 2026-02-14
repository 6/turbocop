use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RootPathnameMethods;

const FILE_METHODS: &[&[u8]] = &[
    b"read", b"write", b"binread", b"binwrite", b"readlines",
    b"exist?", b"exists?", b"directory?", b"file?",
    b"empty?", b"size", b"delete", b"unlink",
];

impl Cop for RootPathnameMethods {
    fn name(&self) -> &'static str {
        "Rails/RootPathnameMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        let method_name = call.name().as_slice();
        if !FILE_METHODS.contains(&method_name) {
            return Vec::new();
        }

        // Receiver must be constant `File`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let const_read = match recv.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if const_read.name().as_slice() != b"File" {
            return Vec::new();
        }

        // First argument should contain a .join call (Rails.root.join(...))
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let first_arg: Vec<_> = args.arguments().iter().collect();
        if first_arg.is_empty() {
            return Vec::new();
        }

        let arg_call = match first_arg[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if arg_call.name().as_slice() != b"join" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.root.join(...).read` instead of `File.read(Rails.root.join(...))`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootPathnameMethods, "cops/rails/root_pathname_methods");
}
