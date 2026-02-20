use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, OR_NODE};

pub struct EnvLocal;

/// Check if a node is `Rails.env.development?` or `Rails.env.test?`.
fn is_rails_env_check(node: &ruby_prism::Node<'_>, env_method: &[u8]) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.name().as_slice() != env_method {
        return false;
    }

    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    let env_call = match recv.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if env_call.name().as_slice() != b"env" {
        return false;
    }

    let rails_recv = match env_call.receiver() {
        Some(r) => r,
        None => return false,
    };

    // Handle both ConstantReadNode (Rails) and ConstantPathNode (::Rails)
    util::constant_name(&rails_recv) == Some(b"Rails")
}

impl Cop for EnvLocal {
    fn name(&self) -> &'static str {
        "Rails/EnvLocal"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, OR_NODE]
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
        let or_node = match node.as_or_node() {
            Some(o) => o,
            None => return,
        };

        let left: ruby_prism::Node<'_> = or_node.left();
        let right: ruby_prism::Node<'_> = or_node.right();

        // Check both orderings: dev? || test? or test? || dev?
        let matches = (is_rails_env_check(&left, b"development?")
            && is_rails_env_check(&right, b"test?"))
            || (is_rails_env_check(&left, b"test?")
                && is_rails_env_check(&right, b"development?"));

        if !matches {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.env.local?` instead of checking for development or test.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnvLocal, "cops/rails/env_local");
}
