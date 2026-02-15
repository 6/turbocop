use ruby_prism::Visit;

use crate::cop::util::{is_rspec_example, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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
    ) -> Vec<Diagnostic> {
        let max = config.get_usize("Max", 1);
        let mut visitor = MultipleExpectationsVisitor {
            source,
            cop: self,
            max,
            ancestor_aggregate_failures: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
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
    fn check_example(&mut self, call: &ruby_prism::CallNode<'pr>, block: &ruby_prism::BlockNode<'pr>) {
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

        // Check if this is an example group (describe/context) with aggregate_failures
        let is_group = if let Some(recv) = node.receiver() {
            crate::cop::util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
                && method_name == b"describe"
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
                    return; // Don't recurse into example body from visitor
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
            if sym.unescaped()== b"aggregate_failures" {
                return Some(true);
            }
        }
        // Hash argument with aggregate_failures: true/false
        if let Some(hash) = arg.as_keyword_hash_node() {
            for element in hash.elements().iter() {
                if let Some(pair) = element.as_assoc_node() {
                    if let Some(key_sym) = pair.key().as_symbol_node() {
                        if key_sym.unescaped()== b"aggregate_failures" {
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
        if node.receiver().is_none()
            && (name == b"expect"
                || name == b"expect_any_instance_of"
                || name == b"is_expected")
        {
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
