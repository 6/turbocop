use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/SharedContext: Detect shared_context/shared_examples misuse.
///
/// - `shared_context` with only examples (no let/subject/hooks) -> use `shared_examples`
/// - `shared_examples` with only let/subject/hooks (no examples) -> use `shared_context`
///
/// ## Investigation notes (2026-03-04)
///
/// **FP=40 root cause:** The cop was only checking direct children of the shared block
/// for example/context methods. RuboCop uses `def_node_search` which searches
/// **recursively** through the entire block body. When a `shared_context` has a
/// `describe` block containing a `before` hook inside it, RuboCop sees the nested
/// `before` and counts it as context setup (so no offense). Our cop only saw `describe`
/// at the top level and classified it as examples-only, producing false positives.
///
/// **Fix:** Changed from direct-child iteration to recursive AST search using
/// `has_examples_recursive` and `has_context_recursive` to match RuboCop behavior.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=1 (puppetlabs/puppet): `shared_context` containing only `describe` blocks
/// (no actual `it`/`specify` examples). `is_example_method` previously included
/// `describe` and `context`, but RuboCop's `Examples.all` does NOT include them —
/// those are ExampleGroups, not examples. Fixed by removing `describe` and `context`
/// from `is_example_method`.
///
/// ## Corpus investigation (2026-03-19)
///
/// FN=5: All five false negatives had `RSpec.` receiver prefix
/// (e.g., `RSpec.shared_examples "a software" do`). The cop was returning early
/// when `call.receiver().is_some()`, skipping all receiver-qualified calls.
/// RuboCop's `#rspec?` predicate accepts both receiverless calls and `RSpec.`
/// prefixed calls. Fixed by allowing `RSpec` constant receiver.
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

        // Accept receiverless calls or RSpec./::RSpec. receiver (matches RuboCop's #rspec? predicate)
        if let Some(recv) = call.receiver() {
            let is_rspec = recv
                .as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"RSpec")
                || recv.as_constant_path_node().is_some_and(|cp| {
                    cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"RSpec")
                });
            if !is_rspec {
                return;
            }
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

        // RuboCop uses def_node_search which searches recursively through
        // the entire block body, not just direct children. We must do the same.
        let has_examples = has_examples_recursive(&body);
        let has_context_setup = has_context_recursive(&body);

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

/// Recursively search for example methods in the AST.
fn has_examples_recursive(node: &ruby_prism::Node<'_>) -> bool {
    use ruby_prism::Visit;
    struct F {
        found: bool,
    }
    impl<'pr> Visit<'pr> for F {
        fn visit_call_node(&mut self, n: &ruby_prism::CallNode<'pr>) {
            if self.found {
                return;
            }
            if n.receiver().is_none() && is_example_method(n.name().as_slice()) {
                self.found = true;
                return;
            }
            ruby_prism::visit_call_node(self, n);
        }
    }
    let mut f = F { found: false };
    f.visit(node);
    f.found
}

/// Recursively search for context/setup methods in the AST.
fn has_context_recursive(node: &ruby_prism::Node<'_>) -> bool {
    use ruby_prism::Visit;
    struct F {
        found: bool,
    }
    impl<'pr> Visit<'pr> for F {
        fn visit_call_node(&mut self, n: &ruby_prism::CallNode<'pr>) {
            if self.found {
                return;
            }
            if n.receiver().is_none() {
                let m = n.name().as_slice();
                if is_context_method(m) || is_context_inclusion(m) {
                    self.found = true;
                    return;
                }
            }
            ruby_prism::visit_call_node(self, n);
        }
    }
    let mut f = F { found: false };
    f.visit(node);
    f.found
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
            // Note: `describe` and `context` are ExampleGroups, NOT examples.
            // RuboCop's Examples.all does not include them; omitting them here
            // prevents false positives when shared_context only has describe/context
            // blocks (without actual it/specify examples inside).
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
