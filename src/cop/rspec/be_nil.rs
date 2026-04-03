/// RSpec/BeNil — ensures consistent style when matching `nil`.
///
/// Matches RuboCop's approach: triggers directly on `be`/`be_nil` send nodes
/// (RESTRICT_ON_SEND) rather than on the parent `to`/`should` call. This
/// correctly handles compound matchers like `all(be nil)` and `.or be(nil)`
/// where `be(nil)` is not a direct argument to `to`/`should`.
///
/// Pattern `(send nil? :be nil)` matches `be(nil)` and `be nil` (with or
/// without parentheses). Pattern `(send nil? :be_nil)` matches `be_nil`.
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BeNil;

impl Cop for BeNil {
    fn name(&self) -> &'static str {
        "RSpec/BeNil"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Match RuboCop's RESTRICT_ON_SEND = %i[be be_nil]
        // Trigger directly on `be` or `be_nil` calls, not on the parent `to`/`should`.
        let method_name = call.name().as_slice();

        // Both patterns require `nil?` receiver (no explicit receiver)
        if call.receiver().is_some() {
            return;
        }

        let enforced_style = config.get_str("EnforcedStyle", "be_nil");

        if enforced_style == "be_nil" {
            // Flag `be(nil)` / `be nil` — prefer `be_nil`
            if method_name != b"be" {
                return;
            }
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 || arg_list[0].as_nil_node().is_none() {
                return;
            }
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `be_nil` over `be(nil)`.".to_string(),
            ));
        } else {
            // Flag `be_nil` — prefer `be(nil)`
            if method_name != b"be_nil" {
                return;
            }
            if call.arguments().is_some() {
                return;
            }
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `be(nil)` over `be_nil`.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BeNil, "cops/rspec/be_nil");
}
