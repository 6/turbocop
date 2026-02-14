use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ScopeArgs;

impl Cop for ScopeArgs {
    fn name(&self) -> &'static str {
        "Rails/ScopeArgs"
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
        if call.receiver().is_some() {
            return Vec::new();
        }
        if call.name().as_slice() != b"scope" {
            return Vec::new();
        }
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return Vec::new();
        }
        let second = &arg_list[1];
        // Lambda is fine
        if second.as_lambda_node().is_some() {
            return Vec::new();
        }
        // proc { } or lambda { } calls are fine
        if let Some(call_arg) = second.as_call_node() {
            let name = call_arg.name().as_slice();
            if name == b"proc" || name == b"lambda" {
                return Vec::new();
            }
        }
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a lambda for the scope body: `scope :name, -> { ... }`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ScopeArgs, "cops/rails/scope_args");
}
