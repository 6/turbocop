use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RootPublicPath;

impl Cop for RootPublicPath {
    fn name(&self) -> &'static str {
        "Rails/RootPublicPath"
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
        // Pattern: Rails.root.join("public")
        // 3-chain: Rails (const) -> root -> join("public")
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"join" {
            return Vec::new();
        }

        // Must have exactly one string argument "public"
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }
        let string_val = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };
        if string_val.unescaped() != b"public" {
            return Vec::new();
        }

        // Receiver should be a call to `root`
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

        // root's receiver should be constant `Rails`
        let rails_recv = match root_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        if rails_recv.as_constant_read_node().is_none() {
            return Vec::new();
        }
        let const_name = rails_recv.as_constant_read_node().unwrap();
        if const_name.name().as_slice() != b"Rails" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.public_path` instead of `Rails.root.join('public')`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootPublicPath, "cops/rails/root_public_path");
}
