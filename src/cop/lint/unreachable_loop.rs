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
    ) -> Vec<Diagnostic> {
        let _allowed_patterns = config.get_string_array("AllowedPatterns");
        let mut visitor = UnreachableLoopVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
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

fn all_paths_break(node: &ruby_prism::Node<'_>) -> bool {
    if is_break_command(node) {
        return true;
    }

    // If statement: both branches must break
    if let Some(if_node) = node.as_if_node() {
        let if_breaks = if_node
            .statements()
            .and_then(|s| {
                let body: Vec<_> = s.body().iter().collect();
                body.last().map(|n| all_paths_break(n))
            })
            .unwrap_or(false);
        let else_breaks = if let Some(subsequent) = if_node.subsequent() {
            all_paths_break(&subsequent)
        } else {
            false
        };
        return if_breaks && else_breaks;
    }

    // Unless: both branches must break
    if let Some(unless_node) = node.as_unless_node() {
        let unless_breaks = unless_node
            .statements()
            .and_then(|s| {
                let body: Vec<_> = s.body().iter().collect();
                body.last().map(|n| all_paths_break(n))
            })
            .unwrap_or(false);
        let else_breaks = unless_node
            .else_clause()
            .and_then(|e| e.statements())
            .and_then(|s| {
                let body: Vec<_> = s.body().iter().collect();
                body.last().map(|n| all_paths_break(n))
            })
            .unwrap_or(false);
        return unless_breaks && else_breaks;
    }

    // Begin/statements block
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body.last() {
                return all_paths_break(last);
            }
        }
    }

    // ElseNode from if/unless
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body.last() {
                return all_paths_break(last);
            }
        }
    }

    false
}

fn has_continue_keyword(node: &ruby_prism::Node<'_>) -> bool {
    node.as_next_node().is_some() || node.as_redo_node().is_some()
}

fn body_always_breaks(stmts: &ruby_prism::StatementsNode<'_>) -> bool {
    let body: Vec<_> = stmts.body().iter().collect();
    if body.is_empty() {
        return false;
    }

    // Check if any statement before the last is a continue keyword
    for stmt in &body[..body.len().saturating_sub(1)] {
        if has_continue_keyword(stmt) {
            return false;
        }
    }

    // Check the last statement
    if let Some(last) = body.last() {
        return all_paths_break(last);
    }

    false
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

        // Check for loop methods (each, map, times, loop, etc.)
        let is_loop_method = method_name == b"each"
            || method_name == b"map"
            || method_name == b"select"
            || method_name == b"reject"
            || method_name == b"collect"
            || method_name == b"detect"
            || method_name == b"find"
            || method_name == b"times"
            || method_name == b"upto"
            || method_name == b"downto"
            || method_name == b"loop"
            || method_name == b"each_with_index"
            || method_name == b"each_with_object"
            || method_name == b"flat_map";

        if is_loop_method {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    if let Some(body) = block_node.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            if body_always_breaks(&stmts) {
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
        }

        // Visit children
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
