use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RaiseArgs;

impl Cop for RaiseArgs {
    fn name(&self) -> &'static str {
        "Style/RaiseArgs"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();
        if name != b"raise" && name != b"fail" {
            return Vec::new();
        }

        // Only bare raise/fail (no receiver)
        if call.receiver().is_some() {
            return Vec::new();
        }

        let enforced_style = config.get_str("EnforcedStyle", "explode");

        if enforced_style != "explode" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Check if the first argument is a call to `.new`
        if let Some(arg_call) = arg_list[0].as_call_node() {
            if arg_call.name().as_slice() == b"new" && arg_call.receiver().is_some() {
                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(source, line, column, "Provide an exception class and message as separate arguments.".to_string())];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(RaiseArgs, "cops/style/raise_args");

    #[test]
    fn bare_raise_is_ignored() {
        let source = b"raise\n";
        let diags = run_cop_full(&RaiseArgs, source);
        assert!(diags.is_empty());
    }
}
