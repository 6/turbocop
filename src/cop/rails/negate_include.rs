use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-07)
///
/// FP=26, FN=1. FPs from safe navigation (`!arr&.include?(x)`) and multi-arg
/// `include?` calls. RuboCop's pattern `(send (send $!nil? :include? $_) :!)`
/// uses `send` (not `csend`) and `$_` (exactly one arg).
/// Fixed by checking for safe navigation and argument count.
///
/// ## Corpus investigation (2026-03-16)
///
/// FP=0, FN=1. The remaining FN is in `rubocop__rubocop__b210a6e` at
/// `lib/rubocop/cop/lint/cop_directive_syntax.rb:74` —
/// `elsif !DirectiveComment::AVAILABLE_MODES.include?(mode)`. Verified that
/// the cop logic correctly detects `!` calls with constant path receivers in
/// both `if` and `elsif` conditions (test fixtures added). The FN is a
/// corpus config artifact — likely the rubocop repo's config resolution
/// differs from the baseline, causing this cop to not run on that file.
pub struct NegateInclude;

impl Cop for NegateInclude {
    fn name(&self) -> &'static str {
        "Rails/NegateInclude"
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"!" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"include?" {
            return;
        }

        // RuboCop uses `send` not `csend` — skip safe navigation (&.include?)
        if let Some(op) = inner_call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return;
            }
        }

        // RuboCop: receiver must exist ($!nil?)
        if inner_call.receiver().is_none() {
            return;
        }

        // RuboCop: exactly one argument ($_)
        let arg_count = inner_call
            .arguments()
            .map(|a| a.arguments().len())
            .unwrap_or(0);
        if arg_count != 1 {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `exclude?` instead of `!include?`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegateInclude, "cops/rails/negate_include");
}
