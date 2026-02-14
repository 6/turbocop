use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

    let const_node = match rails_recv.as_constant_read_node() {
        Some(c) => c,
        None => return false,
    };

    const_node.name().as_slice() == b"Rails"
}

impl Cop for EnvLocal {
    fn name(&self) -> &'static str {
        "Rails/EnvLocal"
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
        let or_node = match node.as_or_node() {
            Some(o) => o,
            None => return Vec::new(),
        };

        let left: ruby_prism::Node<'_> = or_node.left();
        let right: ruby_prism::Node<'_> = or_node.right();

        // Check both orderings: dev? || test? or test? || dev?
        let matches = (is_rails_env_check(&left, b"development?")
            && is_rails_env_check(&right, b"test?"))
            || (is_rails_env_check(&left, b"test?")
                && is_rails_env_check(&right, b"development?"));

        if !matches {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Rails.env.local?` instead of checking for development or test.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnvLocal, "cops/rails/env_local");
}
