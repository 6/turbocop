use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE, RANGE_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RangeInclude;

impl Cop for RangeInclude {
    fn name(&self) -> &'static str {
        "Performance/RangeInclude"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE, RANGE_NODE, STATEMENTS_NODE]
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

        if call.name().as_slice() != b"include?" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if receiver is a RangeNode directly or wrapped in parentheses
        let is_range = receiver.as_range_node().is_some()
            || receiver
                .as_parentheses_node()
                .and_then(|p| p.body())
                .and_then(|b| {
                    // The body of parentheses is a StatementsNode
                    let stmts = b.as_statements_node()?;
                    let body = stmts.body();
                    if body.len() == 1 {
                        body.iter().next()?.as_range_node()
                    } else {
                        None
                    }
                })
                .is_some();

        if !is_range {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Range#cover?` instead of `Range#include?`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RangeInclude, "cops/performance/range_include");
}
