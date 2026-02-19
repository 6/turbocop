use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct EnvironmentComparison;

/// Check if a node is `Rails.env` (CallNode `env` on ConstantReadNode/ConstantPathNode `Rails`).
fn is_rails_env(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if call.name().as_slice() != b"env" {
        return false;
    }
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    // Handle both ConstantReadNode (Rails) and ConstantPathNode (::Rails)
    util::constant_name(&recv) == Some(b"Rails")
}

impl Cop for EnvironmentComparison {
    fn name(&self) -> &'static str {
        "Rails/EnvironmentComparison"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        let method = call.name().as_slice();
        if method != b"==" && method != b"!=" {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        // Check if either side is Rails.env
        let recv_node: ruby_prism::Node<'_> = recv;
        let arg_node = &arg_list[0];

        let is_comparison = is_rails_env(&recv_node) || is_rails_env(arg_node);

        if !is_comparison {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.env.production?` instead of comparing `Rails.env`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnvironmentComparison, "cops/rails/environment_comparison");
}
