use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STATEMENTS_NODE};

pub struct VoidExpect;

impl Cop for VoidExpect {
    fn name(&self) -> &'static str {
        "RSpec/VoidExpect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Detect `expect(...)` or `expect { ... }` without `.to` or `.not_to` chained.
        // This means the expect call node should NOT be the receiver of a `.to`/`.not_to`/`.to_not` call.
        // We detect this by checking that the expect call is a statement (its parent is a
        // StatementsNode/block body, not a receiver of another call).
        //
        // Since we visit every node, we look for CallNodes named `expect` that are NOT
        // receivers of another call. We can't check parent directly, so instead we detect
        // when `expect(...)` is the node itself (not accessed via receiver chain).
        //
        // Strategy: look for `expect` calls that are a direct child of a statements node.
        // Actually, since we can't check parents in this walker, we need a different approach:
        // We look for `expect` call nodes. If the expect call is the receiver of `.to`/`.not_to`,
        // then the OUTER call is what we'd visit. So a void expect is one where the expect
        // call node is visited directly AND is not a receiver of `.to`/`.not_to`.
        //
        // Simpler: we can't detect parent calls, but we CAN check:
        // We look at StatementsNode children for CallNodes named `expect`.

        // We handle this differently: look at any call node that is `expect`, receiverless.
        // Then check if this node is used as the receiver of a to/not_to/to_not call.
        // Since we can't check the parent, we instead search for `.to`/`.not_to`/`.to_not`
        // calls and see if their receiver is an expect call — those are NOT void.
        // But that doesn't let us find void expects.
        //
        // Alternative approach: look for StatementsNode and check direct children.
        let stmts = match node.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        for stmt in stmts.body().iter() {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.name().as_slice() != b"expect" {
                continue;
            }

            // Must be receiverless
            if call.receiver().is_some() {
                continue;
            }

            // This is a void expect — `expect(...)` not chained with `.to`
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it."
                    .to_string(),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VoidExpect, "cops/rspec/void_expect");
}
