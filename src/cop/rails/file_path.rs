use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FilePath;

impl Cop for FilePath {
    fn name(&self) -> &'static str {
        "Rails/FilePath"
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

        if call.name().as_slice() != b"join" {
            return Vec::new();
        }

        // Receiver should be a call to `root` on `Rails`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let root_call = match recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if root_call.name().as_slice() != b"root" {
            return Vec::new();
        }
        let rails_recv = match root_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let const_read = match rails_recv.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if const_read.name().as_slice() != b"Rails" {
            return Vec::new();
        }

        // Must have 2+ string arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return Vec::new();
        }

        // All args should be strings
        let all_strings = arg_list.iter().all(|a| a.as_string_node().is_some());
        if !all_strings {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.root.join('app/models')` with a single path string.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FilePath, "cops/rails/file_path");
}
