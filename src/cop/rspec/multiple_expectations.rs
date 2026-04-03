use ruby_prism::Visit;

use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks if examples contain too many `expect` calls.
///
/// ## Root causes of corpus divergence (fixed):
/// - **173 FNs**: ExpectCounter only matched `expect`, `expect_any_instance_of`, and
///   `is_expected`. RuboCop's `Expectations.all` also includes `should`, `should_not`,
///   `should_receive`, `should_not_receive`, and `are_expected` (all without receiver,
///   i.e. implicit subject style). Added all missing expectation methods.
/// - **27 FPs (zammad, openproject)**: When `RSpec.shared_examples` or `RSpec.shared_context`
///   had `:aggregate_failures` metadata, nitrocop failed to propagate it to nested examples.
///   Root cause: the receiver check for example groups only recognized `RSpec.describe`, not
///   other group methods like `shared_examples`, `shared_context`, `context`, `feature`, etc.
///   Fixed by accepting all `is_rspec_example_group()` methods with `RSpec.` prefix.
/// - **1 FN (pry)**: `focus` (a focused example alias) was missing from `RSPEC_EXAMPLES`.
///   Added to the shared constant in `util.rs`.
/// - **21 FNs (openfoodfoundation, moneta, validates_timeliness)**: When `pending` or `skip`
///   was used as a group wrapper (e.g., `pending "deferred" do it "test" do ... end end`),
///   the visitor treated the outer `pending`/`skip` as an example and returned early without
///   recursing into the block body. Nested `it` blocks inside these wrappers were never
///   visited, so their expectations were never counted. Fixed by recursing into the example
///   block body after `check_example`, so nested examples are still discovered and checked.
///   RuboCop fires `on_block` for every block node, so both the outer wrapper and inner
///   examples are independently checked.
///
/// ## Expectation methods matched (from rubocop-rspec config/default.yml):
/// `are_expected`, `expect`, `expect_any_instance_of`, `is_expected`, `should`,
/// `should_not`, `should_not_receive`, `should_receive` — all without receiver only.
pub struct MultipleExpectations;

impl Cop for MultipleExpectations {
    fn name(&self) -> &'static str {
        "RSpec/MultipleExpectations"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max = config.get_usize("Max", 1);
        let mut visitor = MultipleExpectationsVisitor {
            source,
            cop: self,
            max,
            ancestor_aggregate_failures: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MultipleExpectationsVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a MultipleExpectations,
    max: usize,
    ancestor_aggregate_failures: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'a, 'pr> MultipleExpectationsVisitor<'a> {
    fn check_example(
        &mut self,
        call: &ruby_prism::CallNode<'pr>,
        block: &ruby_prism::BlockNode<'pr>,
    ) {
        // Check if this example itself has :aggregate_failures metadata
        let example_af = has_aggregate_failures_metadata(call);
        match example_af {
            Some(true) => return, // Example has :aggregate_failures or aggregate_failures: true
            Some(false) => {} // Example has aggregate_failures: false — override ancestor, check it
            None => {
                // No metadata on example — inherit from ancestor
                if self.ancestor_aggregate_failures {
                    return;
                }
            }
        }

        // Count expectations, treating aggregate_failures blocks as single expectations
        let mut counter = ExpectCounter { count: 0 };
        if let Some(body) = block.body() {
            counter.visit(&body);
        }

        if counter.count > self.max {
            let loc = call.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!(
                    "Example has too many expectations [{}/{}].",
                    counter.count, self.max
                ),
            ));
        }
    }
}

impl<'a, 'pr> Visit<'pr> for MultipleExpectationsVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if this is an example group (describe/context/shared_examples etc.)
        // with aggregate_failures. When called with RSpec. prefix, accept all example
        // group methods (not just describe) to handle RSpec.shared_examples, RSpec.context, etc.
        let is_group = if let Some(recv) = node.receiver() {
            crate::cop::shared::constant_predicates::constant_short_name(&recv)
                .is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };

        if is_group {
            if let Some(block) = node.block() {
                if let Some(bn) = block.as_block_node() {
                    let group_af = has_aggregate_failures_metadata(node);
                    let old_af = self.ancestor_aggregate_failures;
                    match group_af {
                        Some(true) => self.ancestor_aggregate_failures = true,
                        Some(false) => self.ancestor_aggregate_failures = false,
                        None => {} // Keep inherited value
                    }
                    // Visit block body to find nested examples/groups
                    if let Some(body) = bn.body() {
                        self.visit(&body);
                    }
                    self.ancestor_aggregate_failures = old_af;
                    return;
                }
            }
        }

        // Check if this is an example (it/specify/etc.)
        if node.receiver().is_none() && is_rspec_example(method_name) {
            if let Some(block) = node.block() {
                if let Some(bn) = block.as_block_node() {
                    self.check_example(node, &bn);
                    // Recurse into example body to find nested examples
                    // (e.g., `pending "group" do it "test" do ... end end`
                    // or `skip "disabled" do it "test" do ... end end`).
                    // RuboCop fires on_block for every block, including nested
                    // examples inside pending/skip wrappers.
                    if let Some(body) = bn.body() {
                        self.visit(&body);
                    }
                    return;
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

/// Check if a call node (example or example group) has :aggregate_failures metadata.
/// Returns:
///   Some(true) — has :aggregate_failures symbol or aggregate_failures: true
///   Some(false) — has aggregate_failures: false
///   None — no aggregate_failures metadata
fn has_aggregate_failures_metadata(call: &ruby_prism::CallNode<'_>) -> Option<bool> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        // Symbol argument: :aggregate_failures
        if let Some(sym) = arg.as_symbol_node() {
            if sym.unescaped() == b"aggregate_failures" {
                return Some(true);
            }
        }
        // Hash argument with aggregate_failures: true/false
        if let Some(hash) = arg.as_keyword_hash_node() {
            for element in hash.elements().iter() {
                if let Some(pair) = element.as_assoc_node() {
                    if let Some(key_sym) = pair.key().as_symbol_node() {
                        if key_sym.unescaped() == b"aggregate_failures" {
                            let val = pair.value();
                            if val.as_true_node().is_some() {
                                return Some(true);
                            }
                            if val.as_false_node().is_some() {
                                return Some(false);
                            }
                            // Unknown value — treat as true
                            return Some(true);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check if a method name is an RSpec expectation method (called without receiver).
/// Matches rubocop-rspec's `Language::Expectations` config from `config/default.yml`.
fn is_rspec_expectation(name: &[u8]) -> bool {
    matches!(
        name,
        b"are_expected"
            | b"expect"
            | b"expect_any_instance_of"
            | b"is_expected"
            | b"should"
            | b"should_not"
            | b"should_not_receive"
            | b"should_receive"
    )
}

struct ExpectCounter {
    count: usize,
}

impl<'pr> Visit<'pr> for ExpectCounter {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        // aggregate_failures { ... } block counts as one expectation
        if node.receiver().is_none() && name == b"aggregate_failures" && node.block().is_some() {
            self.count += 1;
            return; // Don't recurse into aggregate_failures block
        }
        // All RSpec expectation methods (from rubocop-rspec Expectations config):
        // are_expected, expect, expect_any_instance_of, is_expected,
        // should, should_not, should_not_receive, should_receive
        if node.receiver().is_none() && is_rspec_expectation(name) {
            self.count += 1;
        }
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleExpectations, "cops/rspec/multiple_expectations");
}
