use crate::cop::method_identifier_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=10, FN=81.
///
/// FP=10: All from loops containing `begin/rescue` blocks where both the main
/// body (raises) and rescue clause (break/return) are break statements. The cop
/// was treating these as "always breaks" but RuboCop does NOT — in Parser gem's
/// AST, `begin/rescue` has a rescue node as last child, and rescue is not a
/// `break_statement?` type. Fixed by returning false for begin nodes with rescue
/// clauses, matching RuboCop's behavior.
///
/// ## Corpus investigation (2026-03-17)
///
/// Remaining FP=2, FN=81.
///
/// FP=2: Two root causes:
/// 1. `return cached_files[path] || next` — the `|| next` provides a conditional
///    loop continuation, making the loop reachable for subsequent iterations.
///    Fixed by adding `conditional_continue_keyword` check matching RuboCop's
///    `conditional_continue_keyword?` method.
/// 2. `exactly(4).times { raise(...) }` — RSpec mock block matched by the default
///    AllowedPatterns config `(exactly|at_least|at_most)\(\d+\)\.times`.
///    Fixed by implementing AllowedPatterns regex matching against call source.
///
/// FN=81: Missing iterator method detection. The `is_loop_method_name` list was
/// incomplete — only had ~20 methods vs RuboCop's full `enumerable_method?` (60+
/// Enumerable instance methods) and `enumerator_method?` (20+ methods plus any
/// method starting with `each_`). Key missing methods: `grep`, `cycle`, `reject!`,
/// `select!`, `filter`, `filter_map`, `sort_by`, `find_all`, `each_entry`, etc.
/// Fixed by expanding to match RuboCop's full method sets. Also added `for` loop
/// detection (RuboCop's `on_for` handler).
///
/// ## Corpus investigation (2026-04-02)
///
/// Remaining FP=3, FN=0.
///
/// FP=3: Loops whose block body is `begin ... ensure ... end` were still being
/// flagged when the main body raised and the ensure body did `break`, `next`, or
/// `throw`. RuboCop does not treat `begin/ensure` as a direct break statement for
/// this cop, just like `begin/rescue`. Fixed by returning false for `BeginNode`
/// whenever `ensure_clause()` is present.
pub struct UnreachableLoop;

impl Cop for UnreachableLoop {
    fn name(&self) -> &'static str {
        "Lint/UnreachableLoop"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let compiled_patterns: Vec<regex::Regex> = allowed_patterns
            .as_ref()
            .map(|patterns| {
                patterns
                    .iter()
                    .filter_map(|p| {
                        // Handle Ruby regexp format: /pattern/ -> pattern
                        let pattern = p.trim();
                        let pattern = if pattern.starts_with('/')
                            && pattern.len() > 1
                            && pattern[1..].contains('/')
                        {
                            let end = pattern[1..].rfind('/').unwrap() + 1;
                            &pattern[1..end]
                        } else {
                            pattern
                        };
                        regex::Regex::new(pattern).ok()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut visitor = UnreachableLoopVisitor {
            cop: self,
            source,
            allowed_patterns: &compiled_patterns,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct UnreachableLoopVisitor<'a, 'src> {
    cop: &'a UnreachableLoop,
    source: &'src SourceFile,
    allowed_patterns: &'a [regex::Regex],
    diagnostics: Vec<Diagnostic>,
}

fn is_break_command(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_return_node().is_some() || node.as_break_node().is_some() {
        return true;
    }
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"raise"
            || name == b"fail"
            || name == b"throw"
            || name == b"exit"
            || name == b"exit!"
            || name == b"abort")
            && (call.receiver().is_none() || is_kernel_receiver(&call))
        {
            return true;
        }
    }
    false
}

fn is_kernel_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            return cr.name().as_slice() == b"Kernel";
        }
        if let Some(cp) = recv.as_constant_path_node() {
            return cp.name().is_some_and(|n| n.as_slice() == b"Kernel");
        }
    }
    false
}

fn is_loop_method_name(name: &[u8]) -> bool {
    method_identifier_predicates::is_enumerator_method(name)
        || method_identifier_predicates::is_enumerable_method(name)
}

/// Check if a sequence of statements has a break statement that isn't preceded
/// by a continue keyword and doesn't have a conditional continue (|| next/redo).
/// This is the core logic shared by begin blocks, if/unless branches, rescue clauses, etc.
fn stmts_break(body: &[ruby_prism::Node<'_>]) -> bool {
    if let Some(break_stmt) = body.iter().find(|s| is_break_statement(s)) {
        !preceded_by_continue(body, break_stmt) && !conditional_continue_keyword(break_stmt)
    } else {
        false
    }
}

/// Check if a node is a break statement (recursively checking if/case/begin blocks).
/// This matches RuboCop's `break_statement?` method.
fn is_break_statement(node: &ruby_prism::Node<'_>) -> bool {
    if is_break_command(node) {
        return true;
    }

    // If statement: both branches must be break statements
    if let Some(if_node) = node.as_if_node() {
        let if_breaks = if_node
            .statements()
            .map(|s| {
                let body: Vec<_> = s.body().iter().collect();
                stmts_break(&body)
            })
            .unwrap_or(false);
        let else_breaks = if let Some(subsequent) = if_node.subsequent() {
            is_break_statement(&subsequent)
        } else {
            false
        };
        return if_breaks && else_breaks;
    }

    // Unless: both branches must be break statements
    if let Some(unless_node) = node.as_unless_node() {
        let unless_breaks = unless_node
            .statements()
            .map(|s| {
                let body: Vec<_> = s.body().iter().collect();
                stmts_break(&body)
            })
            .unwrap_or(false);
        let else_breaks = unless_node
            .else_clause()
            .and_then(|e| e.statements())
            .map(|s| {
                let body: Vec<_> = s.body().iter().collect();
                stmts_break(&body)
            })
            .unwrap_or(false);
        return unless_breaks && else_breaks;
    }

    // Begin/kwbegin block
    if let Some(begin_node) = node.as_begin_node() {
        // RuboCop does NOT treat begin/rescue or begin/ensure as break statements.
        // These nodes create error-handling / cleanup context, so the loop should
        // only be flagged when a break statement exists outside that wrapper.
        if begin_node.rescue_clause().is_some() || begin_node.ensure_clause().is_some() {
            return false;
        }

        if let Some(stmts) = begin_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            return stmts_break(&body);
        }
        return false;
    }

    // ElseNode from if/unless
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            return stmts_break(&body);
        }
        return false;
    }

    // Case/when statement: all branches + else must be break statements
    if let Some(case_node) = node.as_case_node() {
        let else_breaks = case_node
            .else_clause()
            .map(|e| is_break_statement(&e.as_node()))
            .unwrap_or(false);
        if !else_breaks {
            return false;
        }
        return case_node.conditions().iter().all(|when_node| {
            if let Some(when) = when_node.as_when_node() {
                when.statements()
                    .map(|s| {
                        let body: Vec<_> = s.body().iter().collect();
                        stmts_break(&body)
                    })
                    .unwrap_or(false)
            } else {
                false
            }
        });
    }

    // Case/in (pattern matching) statement
    if let Some(case_match) = node.as_case_match_node() {
        let else_breaks = case_match
            .else_clause()
            .map(|e| is_break_statement(&e.as_node()))
            .unwrap_or(false);
        if !else_breaks {
            return false;
        }
        return case_match.conditions().iter().all(|in_node| {
            if let Some(in_clause) = in_node.as_in_node() {
                in_clause
                    .statements()
                    .map(|s| {
                        let body: Vec<_> = s.body().iter().collect();
                        stmts_break(&body)
                    })
                    .unwrap_or(false)
            } else {
                false
            }
        });
    }

    false
}

struct ContinueKeywordFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for ContinueKeywordFinder {
    fn visit_next_node(&mut self, _node: &ruby_prism::NextNode<'pr>) {
        self.found = true;
    }

    fn visit_redo_node(&mut self, _node: &ruby_prism::RedoNode<'pr>) {
        self.found = true;
    }

    fn visit_while_node(&mut self, _node: &ruby_prism::WhileNode<'pr>) {}
    fn visit_until_node(&mut self, _node: &ruby_prism::UntilNode<'pr>) {}
    fn visit_for_node(&mut self, _node: &ruby_prism::ForNode<'pr>) {}

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        let method_name = node.name().as_slice();
        if is_loop_method_name(method_name) && node.block().is_some() {
            if let Some(recv) = node.receiver() {
                self.visit(&recv);
            }
            if let Some(args) = node.arguments() {
                self.visit(&args.as_node());
            }
            return;
        }
        ruby_prism::visit_call_node(self, node);
    }
}

fn contains_continue_keyword(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = ContinueKeywordFinder { found: false };
    finder.visit(node);
    finder.found
}

/// Check if a break statement contains `|| next` or `|| redo` (conditional continue).
/// This matches RuboCop's `conditional_continue_keyword?` method.
/// e.g. `return do_something(value) || next`
fn conditional_continue_keyword(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = OrContinueFinder { found: false };
    finder.visit(node);
    finder.found
}

struct OrContinueFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for OrContinueFinder {
    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        let right = node.right();
        if right.as_next_node().is_some() || right.as_redo_node().is_some() {
            self.found = true;
        }
        if !self.found {
            ruby_prism::visit_or_node(self, node);
        }
    }

    // Don't descend into inner loops
    fn visit_while_node(&mut self, _node: &ruby_prism::WhileNode<'pr>) {}
    fn visit_until_node(&mut self, _node: &ruby_prism::UntilNode<'pr>) {}
    fn visit_for_node(&mut self, _node: &ruby_prism::ForNode<'pr>) {}

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        let method_name = node.name().as_slice();
        if is_loop_method_name(method_name) && node.block().is_some() {
            return;
        }
        ruby_prism::visit_call_node(self, node);
    }
}

fn preceded_by_continue(body: &[ruby_prism::Node<'_>], break_stmt: &ruby_prism::Node<'_>) -> bool {
    let break_offset = break_stmt.location().start_offset();
    for sibling in body {
        let sibling_offset = sibling.location().start_offset();
        if sibling_offset >= break_offset {
            break;
        }
        if is_loop_node(sibling) {
            continue;
        }
        if contains_continue_keyword(sibling) {
            return true;
        }
    }
    false
}

fn is_loop_node(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_while_node().is_some()
        || node.as_until_node().is_some()
        || node.as_for_node().is_some()
    {
        return true;
    }
    if let Some(call) = node.as_call_node() {
        if call.block().is_some() && is_loop_method_name(call.name().as_slice()) {
            return true;
        }
    }
    false
}

fn body_always_breaks(stmts: &ruby_prism::StatementsNode<'_>) -> bool {
    let body: Vec<_> = stmts.body().iter().collect();
    if body.is_empty() {
        return false;
    }
    stmts_break(&body)
}

impl UnreachableLoopVisitor<'_, '_> {
    /// Get the source text of a call node excluding the block portion.
    /// This matches RuboCop's `send_node.source` which is the call without the block.
    fn call_source_without_block(
        &self,
        call: &ruby_prism::CallNode<'_>,
        block: &ruby_prism::BlockNode<'_>,
    ) -> String {
        let call_start = call.location().start_offset();
        let block_start = block.location().start_offset();
        let src = self.source.as_bytes();
        let end = if block_start > call_start {
            block_start
        } else {
            call.location().end_offset()
        };
        String::from_utf8_lossy(&src[call_start..end])
            .trim()
            .to_string()
    }
}

impl<'pr> Visit<'pr> for UnreachableLoopVisitor<'_, '_> {
    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        if let Some(stmts) = node.statements() {
            if body_always_breaks(&stmts) {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "This loop will have at most one iteration.".to_string(),
                ));
            }
        }
        ruby_prism::visit_while_node(self, node);
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        if let Some(stmts) = node.statements() {
            if body_always_breaks(&stmts) {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "This loop will have at most one iteration.".to_string(),
                ));
            }
        }
        ruby_prism::visit_until_node(self, node);
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        if let Some(stmts) = node.statements() {
            if body_always_breaks(&stmts) {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "This loop will have at most one iteration.".to_string(),
                ));
            }
        }
        ruby_prism::visit_for_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        if is_loop_method_name(method_name) {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    // Check AllowedPatterns against the call source (without block)
                    if !self.allowed_patterns.is_empty() {
                        let call_source = self.call_source_without_block(node, &block_node);
                        if self
                            .allowed_patterns
                            .iter()
                            .any(|re| re.is_match(&call_source))
                        {
                            // Still visit children to check inner loops
                            if let Some(recv) = node.receiver() {
                                self.visit(&recv);
                            }
                            if let Some(args) = node.arguments() {
                                self.visit(&args.as_node());
                            }
                            self.visit(&block);
                            return;
                        }
                    }

                    if let Some(body) = block_node.body() {
                        let breaks = if let Some(stmts) = body.as_statements_node() {
                            body_always_breaks(&stmts)
                        } else if let Some(begin_node) = body.as_begin_node() {
                            is_break_statement(&begin_node.as_node())
                        } else {
                            false
                        };
                        if breaks {
                            let loc = node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "This loop will have at most one iteration.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnreachableLoop, "cops/lint/unreachable_loop");
}
