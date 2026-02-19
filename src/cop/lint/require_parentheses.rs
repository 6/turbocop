use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};

pub struct RequireParentheses;

impl Cop for RequireParentheses {
    fn name(&self) -> &'static str {
        "Lint/RequireParentheses"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, OR_NODE]
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

        // Must be a predicate method (name ends with ?)
        let name = call.name();
        if !name.as_slice().ends_with(b"?") {
            return Vec::new();
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Must NOT have parentheses
        if call.opening_loc().is_some() {
            return Vec::new();
        }

        // Check if any argument is an AndNode or OrNode (but not `and`/`or` keywords,
        // which have lower precedence and wouldn't end up inside the args)
        let has_boolean_arg = args.arguments().iter().any(|arg| {
            arg.as_and_node().is_some() || arg.as_or_node().is_some()
        });

        if !has_boolean_arg {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use parentheses in the method call to avoid confusion about precedence."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireParentheses, "cops/lint/require_parentheses");
}
