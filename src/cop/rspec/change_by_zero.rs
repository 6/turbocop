use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ChangeByZero;

/// Detects `change { ... }.by(0)` or `change(X, :y).by(0)`.
impl Cop for ChangeByZero {
    fn name(&self) -> &'static str {
        "RSpec/ChangeByZero"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for `.by(0)` call whose receiver is `change(...)` or similar
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"by" {
            return Vec::new();
        }

        // Must have argument of 0
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let is_zero = if let Some(int_node) = arg_list[0].as_integer_node() {
            // Check the value is 0
            let loc = int_node.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            text == b"0"
        } else {
            false
        };

        if !is_zero {
            return Vec::new();
        }

        // Receiver must be change/a_block_changing/changing
        let change_call = match call.receiver() {
            Some(recv) => match recv.as_call_node() {
                Some(c) => c,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let change_name = change_call.name().as_slice();
        if change_name != b"change"
            && change_name != b"a_block_changing"
            && change_name != b"changing"
        {
            return Vec::new();
        }

        // No receiver on the change call (or it could be chained from expect)
        if change_call.receiver().is_some() {
            return Vec::new();
        }

        // Flag from the change call to the end of .by(0)
        let change_loc = change_call.location();
        let (line, column) = source.offset_to_line_col(change_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `not_to change` over `to change.by(0)`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ChangeByZero, "cops/rspec/change_by_zero");
}
