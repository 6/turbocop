use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `Hash` creation with a mutable default value.
/// `Hash.new([])` or `Hash.new({})` shares the default across all keys.
pub struct SharedMutableDefault;

impl Cop for SharedMutableDefault {
    fn name(&self) -> &'static str {
        "Lint/SharedMutableDefault"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        if call.name().as_slice() != b"new" {
            return Vec::new();
        }

        // Must not have a block (Hash.new { ... } is fine)
        if call.block().is_some() {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_name = match constant_name(&receiver) {
            Some(n) => n,
            None => return Vec::new(),
        };

        if recv_name != b"Hash" {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args: Vec<_> = arguments.arguments().iter().collect();
        if args.is_empty() {
            return Vec::new();
        }

        let first_arg = &args[0];

        // Check for mutable defaults: [], {}, Array.new, Hash.new
        let is_mutable = is_mutable_value(first_arg);

        if !is_mutable {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not create a Hash with a mutable default value as the default value can accidentally be changed.".to_string(),
        )]
    }
}

fn is_mutable_value(node: &ruby_prism::Node<'_>) -> bool {
    // Array literal []
    if node.as_array_node().is_some() {
        return true;
    }
    // Hash literal {}
    // Note: as_keyword_hash_node() is not checked here because keyword hash
    // nodes (keyword args like `foo(a: 1)`) cannot appear as Hash.new arguments.
    if node.as_hash_node().is_some() {
        return true;
    }
    // Array.new or Hash.new
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"new" {
            if let Some(recv) = call.receiver() {
                if let Some(name) = constant_name(&recv) {
                    if name == b"Array" || name == b"Hash" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SharedMutableDefault, "cops/lint/shared_mutable_default");
}
