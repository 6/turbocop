use crate::cop::shared::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Flags `is_expected`/`should`/`should_not` usage when the enclosing example
/// group's `subject` returns a lambda or proc.
///
/// Investigation notes (corpus FP=0, FN=31→0):
///
/// Root cause of 31 FNs: the original implementation only flagged `is_expected`
/// when the matcher argument was a known block-expecting matcher (change,
/// raise_error, raise_exception, throw_symbol, output). RuboCop's cop works
/// differently — it checks whether the nearest `subject` definition in scope
/// contains a lambda/proc literal, and if so flags ANY `is_expected`/`should`/
/// `should_not` call regardless of the matcher. This meant custom matchers
/// like `terminate` were missed entirely.
///
/// The fix rewrites the cop to use `check_source` with a Visitor instead of
/// `check_node`. The visitor walks example groups (describe/context), tracks
/// `subject` definitions, checks if the subject body is a direct lambda/proc
/// (`-> {}`, `lambda {}`, `proc {}`, `Proc.new {}`), and flags implicit
/// expectations in sibling example blocks when the nearest subject is a
/// lambda/proc. Subject inheritance follows RuboCop: child contexts inherit
/// the parent's subject unless they define their own.
///
/// Also fixed FPs: standalone `is_expected.to change { ... }` outside any
/// example group was incorrectly flagged by the old block-matcher heuristic.
/// RuboCop only flags when a lambda subject is in scope.
///
/// Additional FN fix (FN=12→0): the cop previously only checked blocks of
/// known RSpec example methods (it/specify/etc.) for is_expected. RuboCop's
/// implementation uses `on_send` for is_expected and walks UP through all
/// block ancestors, so it flags is_expected in ANY block within an example
/// group that has a lambda subject — including custom methods like `its_call`.
/// Fixed by checking all non-subject, non-example-group child blocks when a
/// lambda subject is in scope, matching RuboCop's ancestor-walking behavior.
///
/// Additional FN fix (FN=12→0, 2026-03-20): the visitor only entered
/// `process_example_group` for bare `describe`/`context` calls (no receiver).
/// `RSpec.describe` (with explicit `RSpec` receiver) was missed, falling
/// through to default recursion without lambda-subject tracking. Fixed by
/// also recognizing `RSpec.describe` as an example group entry point.
///
/// ## Corpus investigation (2026-04-01)
///
/// Corpus oracle reported FP=4, FN=0.
///
/// FP=4: All from lambda subjects defined inside `shared_examples` blocks
/// (ecko/mastodon fork). RuboCop's `multi_statement_example_group?` uses
/// `example_group_with_body?` which only matches regular example groups
/// (describe/context/feature), NOT shared groups (shared_examples,
/// shared_context). So `nearest_subject` skips shared group blocks when
/// walking up the ancestor chain, making subjects defined inside them
/// invisible. Fixed by passing `is_shared_group` flag to
/// `process_example_group` and skipping subject detection for shared groups.
/// Parent lambda subjects still propagate correctly through shared groups.
pub struct ImplicitBlockExpectation;

impl Cop for ImplicitBlockExpectation {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitBlockExpectation"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = ImplicitBlockVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ImplicitBlockVisitor<'a> {
    cop: &'a ImplicitBlockExpectation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for ImplicitBlockVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Look for top-level example groups (describe/context/etc.)
        let name = node.name().as_slice();
        let is_example_group = if node.receiver().is_none() {
            is_rspec_example_group(name)
        } else {
            // Handle RSpec.describe (explicit receiver)
            name == b"describe"
                && node
                    .receiver()
                    .and_then(|r| util::constant_name(&r))
                    .is_some_and(|n| n == b"RSpec")
        };
        if is_example_group {
            if let Some(block) = node.block().and_then(|b| b.as_block_node()) {
                let is_shared = node.receiver().is_none() && is_rspec_shared_group(name);
                self.process_example_group(&block, false, is_shared);
                return; // Don't recurse further — we handle children manually
            }
        }

        // Default recursion for non-example-group nodes
        ruby_prism::visit_call_node(self, node);
    }
}

impl ImplicitBlockVisitor<'_> {
    /// Process an example group block: find subject definitions, check examples,
    /// and recurse into nested groups.
    fn process_example_group(
        &mut self,
        block: &ruby_prism::BlockNode<'_>,
        parent_has_lambda_subject: bool,
        is_shared_group: bool,
    ) {
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let stmts_vec: Vec<_> = stmts.body().iter().collect();

        // RuboCop's multi_statement_example_group? requires ≥2 statements
        // to look for a subject definition in THIS group. But we still
        // process children to inherit the parent's subject status.
        let is_multi_statement = stmts_vec.len() >= 2;

        // Find subject definition in direct children (only if multi-statement)
        let mut has_lambda_subject = parent_has_lambda_subject;

        // RuboCop's multi_statement_example_group? uses example_group_with_body?
        // which excludes shared groups (shared_examples, shared_context). So
        // subjects defined inside shared groups are invisible to nearest_subject.
        // Skip subject detection for shared groups, but inherit from parent.
        if is_multi_statement && !is_shared_group {
            for stmt in &stmts_vec {
                if let Some(call) = stmt.as_call_node() {
                    let name = call.name().as_slice();
                    if call.receiver().is_none()
                        && (name == b"subject" || name == b"subject!")
                        && call.block().is_some()
                    {
                        has_lambda_subject = is_lambda_subject_block(&call);
                    }
                }
            }
        }

        // Check children: flag implicit expects in examples, recurse into nested groups
        for stmt in &stmts_vec {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();

                if call.receiver().is_none() {
                    // Nested example group (context/describe) — recurse
                    if is_rspec_example_group(name) {
                        if let Some(bn) = call.block().and_then(|b| b.as_block_node()) {
                            let nested_shared = is_rspec_shared_group(name);
                            self.process_example_group(&bn, has_lambda_subject, nested_shared);
                        }
                    } else if has_lambda_subject {
                        // Any block within a lambda-subject group: check for
                        // is_expected/should/should_not. RuboCop walks UP from
                        // is_expected through all block ancestors, so it flags
                        // is_expected in ANY block (it, specify, its_call, custom
                        // methods, etc.), not just known RSpec example methods.
                        if let Some(bn) = call.block().and_then(|b| b.as_block_node()) {
                            // Skip subject/subject! definitions
                            if name != b"subject" && name != b"subject!" {
                                self.check_example_body(&bn);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check an example block body for `is_expected`/`should`/`should_not` calls.
    fn check_example_body(&mut self, block: &ruby_prism::BlockNode<'_>) {
        if let Some(body) = block.body() {
            let mut finder = ImplicitExpectFinder {
                cop: self.cop,
                source: self.source,
                diagnostics: &mut self.diagnostics,
            };
            finder.visit(&body);
        }
    }
}

/// Visitor that finds `is_expected`/`should`/`should_not` calls within
/// an example body.
struct ImplicitExpectFinder<'a> {
    cop: &'a ImplicitBlockExpectation,
    source: &'a SourceFile,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for ImplicitExpectFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();

        if node.receiver().is_none()
            && (name == b"is_expected" || name == b"should" || name == b"should_not")
        {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid implicit block expectations.".to_string(),
            ));
            return; // Don't recurse into the call
        }

        // Recurse for other calls (e.g., `.to` wrapping `is_expected`)
        ruby_prism::visit_call_node(self, node);
    }
}

/// Check if a subject call's block body is a direct lambda/proc.
fn is_lambda_subject_block(call: &ruby_prism::CallNode<'_>) -> bool {
    let block = match call.block() {
        Some(b) => b,
        None => return false,
    };
    let bn = match block.as_block_node() {
        Some(bn) => bn,
        None => return false,
    };
    let body = match bn.body() {
        Some(b) => b,
        None => return false,
    };

    // The body should be a StatementsNode with a single statement
    if let Some(stmts) = body.as_statements_node() {
        let stmts_vec: Vec<_> = stmts.body().iter().collect();
        if stmts_vec.len() == 1 {
            return is_lambda_or_proc(&stmts_vec[0]);
        }
    }

    false
}

/// Check if a node is a lambda/proc literal:
/// - `-> { ... }` (LambdaNode)
/// - `lambda { ... }` (CallNode with name `lambda`, no receiver, with block)
/// - `proc { ... }` (CallNode with name `proc`, no receiver, with block)
/// - `Proc.new { ... }` (CallNode with name `new`, receiver is `Proc` constant)
fn is_lambda_or_proc(node: &ruby_prism::Node<'_>) -> bool {
    // -> { ... }
    if node.as_lambda_node().is_some() {
        return true;
    }

    // lambda { ... }, proc { ... }, Proc.new { ... }
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();

        // lambda { ... } or proc { ... } — no receiver
        if call.receiver().is_none()
            && (name == b"lambda" || name == b"proc")
            && call.block().is_some()
        {
            return true;
        }

        // Proc.new { ... } — receiver is Proc constant (simple or qualified)
        if name == b"new" {
            if let Some(recv) = call.receiver() {
                if let Some(const_read) = recv.as_constant_read_node() {
                    if const_read.name().as_slice() == b"Proc" && call.block().is_some() {
                        return true;
                    }
                }
                if let Some(const_path) = recv.as_constant_path_node() {
                    if let Some(path_name) = const_path.name() {
                        if path_name.as_slice() == b"Proc" && call.block().is_some() {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ImplicitBlockExpectation,
        "cops/rspec/implicit_block_expectation"
    );
}
