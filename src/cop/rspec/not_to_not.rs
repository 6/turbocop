use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/NotToNot - Checks for consistent method usage for negating expectations.
///
/// RuboCop's matcher is `(send _ % ...)` where `_` matches ANY node
/// including nil (no receiver). This means `not_to`/`to_not` is flagged
/// regardless of receiver: `expect(x).to_not`, `expect { ... }.to_not`,
/// chained calls, and receiverless calls like `expect_it { to_not ... }`
/// (implicit self inside a block).
///
/// Corpus investigation: 10 FN in Arachni__arachni were all receiverless
/// `to_not` inside `expect_it { ... }` blocks. Fixed by removing the
/// receiver-presence check.
pub struct NotToNot;

impl Cop for NotToNot {
    fn name(&self) -> &'static str {
        "RSpec/NotToNot"
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
        let enforced_style = config.get_str("EnforcedStyle", "not_to");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if enforced_style == "not_to" {
            // Flag `to_not`
            if method_name != b"to_not" {
                return;
            }
        } else {
            // Flag `not_to`
            if method_name != b"not_to" {
                return;
            }
        }

        // RuboCop's matcher is `(send _ % ...)` — `_` matches any node
        // including nil (no receiver). This handles both `expect(x).to_not`
        // (with receiver) and bare `to_not` inside blocks like
        // `expect_it { to_not be_buffered }` (implicit self, no receiver).
        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let (preferred, flagged) = if enforced_style == "not_to" {
            ("not_to", "to_not")
        } else {
            ("to_not", "not_to")
        };

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{preferred}` over `{flagged}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NotToNot, "cops/rspec/not_to_not");
}
