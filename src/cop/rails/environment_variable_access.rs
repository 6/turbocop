use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct EnvironmentVariableAccess;

impl Cop for EnvironmentVariableAccess {
    fn name(&self) -> &'static str {
        "Rails/EnvironmentVariableAccess"
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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_reads = config.get_bool("AllowReads", false);
        let allow_writes = config.get_bool("AllowWrites", false);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();

        // AllowWrites: skip ENV[]= assignments
        if allow_writes && method == b"[]=" {
            return Vec::new();
        }

        // AllowReads: skip ENV[] reads
        if allow_reads && method == b"[]" {
            return Vec::new();
        }

        if method != b"[]" {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Handle both ConstantReadNode (ENV) and ConstantPathNode (::ENV)
        if util::constant_name(&recv) != Some(b"ENV") {
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
