use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/SharedContext: Detect shared_context/shared_examples misuse.
///
/// - `shared_context` with only examples (no let/subject/hooks) -> use `shared_examples`
/// - `shared_examples` with only let/subject/hooks (no examples) -> use `shared_context`
pub struct SharedContext;

impl Cop for SharedContext {
    fn name(&self) -> &'static str {
        "RSpec/SharedContext"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
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

        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        let is_shared_context = name == b"shared_context";
        let is_shared_examples = name == b"shared_examples" || name == b"shared_examples_for";

        if !is_shared_context && !is_shared_examples {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return, // Empty body is OK
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let nodes: Vec<_> = stmts.body().iter().collect();
        if nodes.is_empty() {
            return;
        }

        let mut has_examples = false;
        let mut has_context_setup = false; // let, subject, hooks

        for stmt in &nodes {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if is_example_method(m) {
                    has_examples = true;
                } else if is_context_method(m) || is_context_inclusion(m) {
                    has_context_setup = true;
                }
            }
        }

        let loc = call.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());

        if is_shared_context && has_examples && !has_context_setup {
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Use `shared_examples` when you don't define context.".to_string(),
            ));
        }

        if is_shared_examples && has_context_setup && !has_examples {
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Use `shared_context` when you don't define examples.".to_string(),
            ));
        }
    }
}

fn is_example_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"it"
            | b"specify"
            | b"example"
            | b"scenario"
            | b"xit"
            | b"xspecify"
            | b"xexample"
            | b"xscenario"
            | b"fit"
            | b"fspecify"
            | b"fexample"
            | b"fscenario"
            | b"pending"
            | b"skip"
            | b"its"
            | b"describe"
            | b"context"
            // Example inclusions also count as examples
            | b"it_behaves_like"
            | b"it_should_behave_like"
            | b"include_examples"
    )
}

fn is_context_inclusion(name: &[u8]) -> bool {
    matches!(name, b"include_context")
}

fn is_context_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"let"
            | b"let!"
            | b"subject"
            | b"subject!"
            | b"before"
            | b"after"
            | b"around"
            | b"prepend_before"
            | b"prepend_after"
            | b"append_before"
            | b"append_after"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SharedContext, "cops/rspec/shared_context");
}
