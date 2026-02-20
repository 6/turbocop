use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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
        let _allowed_patterns = config.get_string_array("AllowedPatterns");
        let mut visitor = UnreachableLoopVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct UnreachableLoopVisitor<'a, 'src> {
    cop: &'a UnreachableLoop,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

fn is_break_command(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_return_node().is_some() || node.as_break_node().is_some() {
        return true;
    }
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"raise" || name == b"fail" || name == b"throw"
            || name == b"exit" || name == b"exit!" || name == b"abort")
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
    matches!(
        name,
        b"each"
            | b"map"
            | b"select"
            | b"reject"
            | b"collect"
            | b"detect"
            | b"find"
            | b"times"
            | b"upto"
            | b"downto"
            | b"loop"
            | b"each_with_index"
            | b"each_with_object"
            | b"flat_map"
    )
}

/// Check if a sequence of statements has a break statement that isn't preceded
/// by a continue keyword. This is the core logic shared by begin blocks,
/// if/unless branches, rescue clauses, etc.
fn stmts_break(body: &[ruby_prism::Node<'_>]) -> bool {
    if let Some(break_stmt) = body.iter().find(|s| is_break_statement(s)) {
        !preceded_by_continue(body, break_stmt)
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
        if begin_node.rescue_clause().is_some() {
            let main_breaks = begin_node
                .statements()
                .map(|stmts| {
                    let body: Vec<_> = stmts.body().iter().collect();
                    stmts_break(&body)
                })
                .unwrap_or(false);

            let rescues_break = all_rescue_clauses_break(begin_node.rescue_clause());

            return main_breaks && rescues_break;
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

/// Check if all rescue clauses in a chain break.
fn all_rescue_clauses_break(rescue_clause: Option<ruby_prism::RescueNode<'_>>) -> bool {
    let mut current = rescue_clause;
    while let Some(rescue_node) = current {
        let rescue_breaks = rescue_node
            .statements()
            .map(|stmts| {
                let body: Vec<_> = stmts.body().iter().collect();
                stmts_break(&body)
            })
            .unwrap_or(false);
        if !rescue_breaks {
            return false;
        }
        current = rescue_node.subsequent();
    }
    true
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

fn preceded_by_continue(
    body: &[ruby_prism::Node<'_>],
    break_stmt: &ruby_prism::Node<'_>,
) -> bool {
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

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        if is_loop_method_name(method_name) {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
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
                            let (line, column) =
                                self.source.offset_to_line_col(loc.start_offset());
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
