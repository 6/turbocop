use crate::cop::shared::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::shared::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example_group, is_rspec_let, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ScatteredLet checks for let/let! declarations scattered across an example group.
///
/// ## Corpus investigation (2026-03-18) — FN=5
///
/// Root cause: when receiver was `RSpec`, only `describe` was matched as an
/// example group. `RSpec.feature`, `RSpec.context`, etc. were not recognized.
/// 4 of 5 FNs were from avo-hq using `RSpec.feature`; 1 from rubocop-rspec's
/// smoke test file (excluded by AllCops.Exclude in that project).
/// Fix: use `is_rspec_example_group()` for receiver-qualified calls too,
/// matching the same pattern used in LetBeforeExamples (commit 7158fd2b).
///
/// FP root cause (43 FPs): The cop was running inside shared_examples/shared_examples_for/
/// shared_context blocks. RuboCop's `example_group_with_body?` matcher only matches
/// ExampleGroups (describe/context/feature), NOT SharedGroups. Fixed by skipping
/// shared group method names.
///
/// ## Corpus investigation (2026-03-14)
///
/// Previously excluded `let :name, &proc` (BlockArgumentNode) form, thinking RuboCop's
/// `let?` only matched BlockNode. But RuboCop's actual `let?` pattern matches both:
///   `(block (send nil? {:let :let!}) ...)` AND `(send nil? {:let :let!} _ block_pass)`.
/// Restored block_pass recognition (2026-03-20) to fix FN=2 on rubocop-rspec's
/// `weird_rspec_spec.rb` where `let(:foo, &bar)` was not counted as a let declaration.
///
/// ## Corpus investigation (2026-03-31) — FP=1
///
/// FP=1: a scattered bare `let :name, &PROC` is a corpus mismatch even under the
/// oracle's baseline bundle. RuboCop 1.84.2 + rubocop-rspec 3.9.0 crashes while
/// building the autocorrection for that node shape, so the observable result is
/// "no offense" for that bare block-pass let and anything later in that group.
///
/// Match RuboCop's current behavior by treating block-pass lets in the initial let
/// group as normal lets, but stop scanning the rest of the group once a bare
/// block-pass let appears after a non-let sibling. Parenthesized
/// `let(:name, &PROC)` still reports normally. Earlier regular offenses in the
/// same group must still be kept.
pub struct ScatteredLet;

impl Cop for ScatteredLet {
    fn name(&self) -> &'static str {
        "RSpec/ScatteredLet"
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

        let method_name = call.name().as_slice();

        // Check for example group calls (including ::RSpec.describe)
        // Skip shared groups (shared_examples, shared_examples_for, shared_context)
        // — RuboCop's example_group_with_body? only matches ExampleGroups, not SharedGroups.
        if is_rspec_shared_group(method_name) {
            return;
        }

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && is_rspec_example_group(method_name)
                && !is_rspec_shared_group(method_name)
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Track if we've seen a non-let statement after the initial let block
        let mut seen_non_let = false;
        let mut in_let_group = false;

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let name = c.name().as_slice();
                // RuboCop's `let?` pattern matches two forms:
                // 1. (block (send nil? {:let :let!}) ...) — regular block
                // 2. (send nil? {:let :let!} _ block_pass) — &proc argument
                let has_block_node = c.block().is_some_and(|b| b.as_block_node().is_some());
                // In Prism, `&proc` (block-pass) is stored in `call.block()` as BlockArgumentNode
                let has_block_pass = c
                    .block()
                    .is_some_and(|b| b.as_block_argument_node().is_some());
                let is_bare_call = c.opening_loc().is_none();
                if c.receiver().is_none()
                    && is_rspec_let(name)
                    && (has_block_node || has_block_pass)
                {
                    if seen_non_let {
                        if has_block_pass && is_bare_call {
                            // RuboCop's current implementation crashes when a scattered
                            // bare `let :name, &PROC` needs autocorrection, so no offense
                            // from this node or any later sibling is reported for the group.
                            break;
                        }

                        let loc = stmt.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Group all let/let! blocks in the example group together.".to_string(),
                        ));
                    } else {
                        in_let_group = true;
                    }
                    continue;
                }
            }

            if in_let_group {
                seen_non_let = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ScatteredLet, "cops/rspec/scattered_let");
}
