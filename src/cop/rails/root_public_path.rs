use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct RootPublicPath;

impl Cop for RootPublicPath {
    fn name(&self) -> &'static str {
        "Rails/RootPublicPath"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"join" {
            return;
        }

        // Must have at least one argument, first must be a string starting with "public"
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }
        let first_str = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };
        let content = first_str.unescaped();
        // Match strings like "public", "public/file.pdf"
        if content != b"public" && !content.starts_with(b"public/") {
            return;
        }

        // Receiver should be a call to `root`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let root_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if root_call.name().as_slice() != b"root" {
            return;
        }

        // root's receiver should be constant `Rails` or `::Rails`
        let rails_recv = match root_call.receiver() {
            Some(r) => r,
            None => return,
        };
        if util::constant_name(&rails_recv) != Some(b"Rails") {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.public_path`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootPublicPath, "cops/rails/root_public_path");
}
