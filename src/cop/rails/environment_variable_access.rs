use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnvironmentVariableAccess;

impl Cop for EnvironmentVariableAccess {
    fn name(&self) -> &'static str {
        "Rails/EnvironmentVariableAccess"
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

        if call.name().as_slice() != b"[]" {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let const_node = match recv.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if const_node.name().as_slice() != b"ENV" {
            return Vec::new();
        }

        // Get the key string for the message
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let key = if let Some(s) = arg_list[0].as_string_node() {
            String::from_utf8_lossy(s.unescaped()).to_string()
        } else {
            return Vec::new();
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `ENV.fetch('{key}')` instead of `ENV['{key}']` for safer access."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnvironmentVariableAccess, "cops/rails/environment_variable_access");
}
