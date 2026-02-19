use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

/// RSpec/ItBehavesLike: Enforce `it_behaves_like` vs `it_should_behave_like` style.
/// Default prefers `it_behaves_like`.
pub struct ItBehavesLike;

impl Cop for ItBehavesLike {
    fn name(&self) -> &'static str {
        "RSpec/ItBehavesLike"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        let style = config.get_str("EnforcedStyle", "it_behaves_like");

        let (bad_method, good_method) = if style == "it_should_behave_like" {
            (b"it_behaves_like" as &[u8], "it_should_behave_like")
        } else {
            (b"it_should_behave_like" as &[u8], "it_behaves_like")
        };

        if name != bad_method {
            return;
        }

        let bad_name = std::str::from_utf8(bad_method).unwrap_or("?");
        let loc = call.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            format!(
                "Prefer `{}` over `{}` when including examples in a nested context.",
                good_method, bad_name
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ItBehavesLike, "cops/rspec/it_behaves_like");
}
