use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantParentheses;

impl Cop for RedundantParentheses {
    fn name(&self) -> &'static str {
        "Style/RedundantParentheses"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut visitor = RedundantParensVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            parent_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

#[derive(Clone, Copy)]
enum ParentKind {
    And,
    Or,
    Call,
    Splat,
    KeywordSplat,
    Return,
    Next,
    Break,
    Ternary,
    Range,
    Other,
}

struct ParentInfo {
    kind: ParentKind,
    multiline: bool,
    call_parenthesized: bool,
    call_arg_count: usize,
    is_operator: bool,
}

struct RedundantParensVisitor<'a> {
    cop: &'a RedundantParentheses,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    parent_stack: Vec<ParentInfo>,
}

impl RedundantParensVisitor<'_> {
    fn check_parens(&mut self, node: &ruby_prism::ParenthesesNode<'_>) {
        let body = match node.body() {
            Some(b) => b,
            None => return,
        };

        let inner_nodes: Vec<ruby_prism::Node<'_>> =
            if let Some(stmts) = body.as_statements_node() {
                stmts.body().iter().collect()
            } else {
                vec![body]
            };

        if inner_nodes.len() != 1 {
            return;
        }

        let inner = &inner_nodes[0];
        // parent_stack.last() is the ParenthesesNode's own entry (pushed by
        // visit_branch_node_enter). The actual parent is one level up.
        let parent = if self.parent_stack.len() >= 2 {
            Some(&self.parent_stack[self.parent_stack.len() - 2])
        } else {
            None
        };

        // like_method_argument_parentheses?
        if let Some(p) = parent {
            if matches!(p.kind, ParentKind::Call)
                && !p.call_parenthesized
                && !p.is_operator
                && p.call_arg_count == 1
            {
                return;
            }
        }

        // multiline_control_flow_statements?
        if let Some(p) = parent {
            if matches!(p.kind, ParentKind::Return | ParentKind::Next | ParentKind::Break)
                && p.multiline
            {
                return;
            }
        }

        // allowed_ancestor? — don't flag `break(value)`, `return(value)`, `next(value)`
        // when the keyword is directly adjacent to the open paren (no space).
        // RuboCop's `parens_required?` checks if a letter precedes the `(`.
        if let Some(p) = parent {
            if matches!(p.kind, ParentKind::Return | ParentKind::Next | ParentKind::Break) {
                let open_offset = node.location().start_offset();
                if open_offset > 0 {
                    let before = self.source.as_bytes()[open_offset - 1];
                    if before.is_ascii_alphabetic() {
                        return;
                    }
                }
            }
        }

        // allowed_ternary? — look through wrapper nodes (StatementsNode, ElseNode)
        // because Prism wraps ternary branches in intermediate nodes
        if self.has_ternary_ancestor() {
            return;
        }

        // range parent
        if let Some(p) = parent {
            if matches!(p.kind, ParentKind::Range) {
                return;
            }
        }

        if let Some(msg) = classify_simple(inner) {
            self.add_offense(node, msg);
            return;
        }

        // Logical expression
        if inner.as_and_node().is_some() || inner.as_or_node().is_some() {
            if let Some(msg) = check_logical(self.source.as_bytes(), node, inner, parent) {
                self.add_offense(node, msg);
                return;
            }
        }

        // Method call
        if inner.as_call_node().is_some() {
            if let Some(msg) = check_method_call(self.source.as_bytes(), node, inner, parent) {
                self.add_offense(node, msg);
            }
        }
    }

    fn add_offense(&mut self, node: &ruby_prism::ParenthesesNode<'_>, msg: &str) {
        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Don't use parentheses around {}.", msg),
        ));
    }

    /// Check if a nearby ancestor is a ternary, looking through intermediate
    /// wrapper nodes (StatementsNode, ElseNode) that Prism inserts.
    fn has_ternary_ancestor(&self) -> bool {
        if self.parent_stack.len() < 2 {
            return false;
        }
        // Start at len-2 (skip the ParenthesesNode's own entry)
        for i in (0..self.parent_stack.len() - 1).rev() {
            match self.parent_stack[i].kind {
                ParentKind::Ternary => return true,
                ParentKind::Other => continue,
                _ => return false,
            }
        }
        false
    }

    fn push_parent(&mut self, kind: ParentKind) {
        self.parent_stack.push(ParentInfo {
            kind,
            multiline: false,
            call_parenthesized: false,
            call_arg_count: 0,
            is_operator: false,
        });
    }
}

fn check_logical<'a>(
    content: &[u8],
    paren_node: &ruby_prism::ParenthesesNode<'_>,
    inner: &ruby_prism::Node<'_>,
    parent: Option<&ParentInfo>,
) -> Option<&'a str> {
    if is_chained(content, paren_node) {
        return None;
    }

    let is_and = inner.as_and_node().is_some();

    // RuboCop: semantic_operator? means keyword form (and/or);
    // if keyword form and has parent, skip
    if uses_keyword_operator(inner) && parent.is_some() {
        return None;
    }

    // ALLOWED_NODE_TYPES: or, send (call), splat, kwsplat
    if let Some(p) = parent {
        if matches!(
            p.kind,
            ParentKind::Or | ParentKind::Call | ParentKind::Splat | ParentKind::KeywordSplat
        ) {
            return None;
        }
    }

    // inner is `or` and parent is `and` → skip
    if !is_and {
        if let Some(p) = parent {
            if matches!(p.kind, ParentKind::And) {
                return None;
            }
        }
    }

    // ternary parent → skip
    if let Some(p) = parent {
        if matches!(p.kind, ParentKind::Ternary) {
            return None;
        }
    }

    Some("a logical expression")
}

fn check_method_call<'a>(
    content: &[u8],
    paren_node: &ruby_prism::ParenthesesNode<'_>,
    inner: &ruby_prism::Node<'_>,
    parent: Option<&ParentInfo>,
) -> Option<&'a str> {
    if is_chained(content, paren_node) {
        return None;
    }

    let call = inner.as_call_node()?;

    // prefix_not: !expr
    if call.name().as_slice() == b"!"
        && call.receiver().is_some()
        && call.arguments().is_none()
    {
        return None;
    }

    // If the inner call has a do..end block and is an argument to an
    // unparenthesized method call, the parens are required to prevent
    // Ruby from binding the do..end block to the outer method call.
    // e.g. `scope :name, (lambda do |args| ... end)` — removing the parens
    // would make `do..end` attach to `scope` instead of `lambda`.
    if has_do_end_block(&call) {
        if let Some(p) = parent {
            if matches!(p.kind, ParentKind::Call) && !p.call_parenthesized {
                return None;
            }
        }
    }

    let has_args = call.arguments().is_some();
    let call_has_parens = call.opening_loc().is_some();

    // If call has unparenthesized args (like `1 + 2`), only flag if paren
    // is in a "singular parent" position (sole child of its parent).
    if has_args && !call_has_parens {
        let singular = match parent {
            None => true,
            Some(p) => matches!(
                p.kind,
                ParentKind::Return | ParentKind::Next | ParentKind::Break
            ),
        };
        if !singular {
            return None;
        }
    }

    Some("a method call")
}

impl<'pr> Visit<'pr> for RedundantParensVisitor<'_> {
    // visit_branch_node_enter/leave provide push/pop for ALL branch nodes.
    // Specific visit_* methods then MODIFY the top of stack to set the correct kind.
    fn visit_branch_node_enter(&mut self, _node: ruby_prism::Node<'pr>) {
        self.push_parent(ParentKind::Other);
    }

    fn visit_branch_node_leave(&mut self) {
        self.parent_stack.pop();
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        self.check_parens(node);
        // enter already pushed; leave will pop
        ruby_prism::visit_parentheses_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            let start_line = self.source.offset_to_line_col(node.location().start_offset()).0;
            let end_line = self.source.offset_to_line_col(node.location().end_offset().saturating_sub(1)).0;
            top.kind = ParentKind::Call;
            top.multiline = start_line != end_line;
            top.call_parenthesized = node.opening_loc().is_some();
            top.call_arg_count = node.arguments().map(|a| a.arguments().len()).unwrap_or(0);
            top.is_operator = is_operator_method(node);
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            top.kind = ParentKind::And;
        }
        ruby_prism::visit_and_node(self, node);
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            top.kind = ParentKind::Or;
        }
        ruby_prism::visit_or_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if node.if_keyword_loc().is_none() {
            if let Some(top) = self.parent_stack.last_mut() {
                top.kind = ParentKind::Ternary;
            }
        }
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            let start_line = self.source.offset_to_line_col(node.location().start_offset()).0;
            let end_line = self.source.offset_to_line_col(node.location().end_offset().saturating_sub(1)).0;
            top.kind = ParentKind::Return;
            top.multiline = start_line != end_line;
        }
        ruby_prism::visit_return_node(self, node);
    }

    fn visit_next_node(&mut self, node: &ruby_prism::NextNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            let start_line = self.source.offset_to_line_col(node.location().start_offset()).0;
            let end_line = self.source.offset_to_line_col(node.location().end_offset().saturating_sub(1)).0;
            top.kind = ParentKind::Next;
            top.multiline = start_line != end_line;
        }
        ruby_prism::visit_next_node(self, node);
    }

    fn visit_break_node(&mut self, node: &ruby_prism::BreakNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            let start_line = self.source.offset_to_line_col(node.location().start_offset()).0;
            let end_line = self.source.offset_to_line_col(node.location().end_offset().saturating_sub(1)).0;
            top.kind = ParentKind::Break;
            top.multiline = start_line != end_line;
        }
        ruby_prism::visit_break_node(self, node);
    }

    fn visit_splat_node(&mut self, node: &ruby_prism::SplatNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            top.kind = ParentKind::Splat;
        }
        ruby_prism::visit_splat_node(self, node);
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            top.kind = ParentKind::KeywordSplat;
        }
        ruby_prism::visit_assoc_splat_node(self, node);
    }

    fn visit_range_node(&mut self, node: &ruby_prism::RangeNode<'pr>) {
        if let Some(top) = self.parent_stack.last_mut() {
            top.kind = ParentKind::Range;
        }
        ruby_prism::visit_range_node(self, node);
    }
}

fn is_chained(content: &[u8], paren_node: &ruby_prism::ParenthesesNode<'_>) -> bool {
    let end_offset = paren_node.location().end_offset();
    if end_offset < content.len() {
        let next_byte = content[end_offset];
        if next_byte == b'.' || next_byte == b'&' {
            return true;
        }
    }
    false
}

/// Returns true if the call node has a do..end block attached to it.
fn has_do_end_block(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(block) = call.block() {
        if let Some(block_node) = block.as_block_node() {
            return block_node.opening_loc().as_slice() == b"do";
        }
    }
    false
}

fn uses_keyword_operator(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(and_node) = node.as_and_node() {
        and_node.operator_loc().as_slice() == b"and"
    } else if let Some(or_node) = node.as_or_node() {
        or_node.operator_loc().as_slice() == b"or"
    } else {
        false
    }
}

fn is_operator_method(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    matches!(
        name,
        b"+" | b"-"
            | b"*"
            | b"/"
            | b"%"
            | b"**"
            | b"=="
            | b"!="
            | b"<"
            | b">"
            | b"<="
            | b">="
            | b"<=>"
            | b"<<"
            | b">>"
            | b"&"
            | b"|"
            | b"^"
            | b"~"
            | b"[]"
            | b"[]="
    )
}

fn classify_simple(node: &ruby_prism::Node<'_>) -> Option<&'static str> {
    if is_literal(node) {
        Some("a literal")
    } else if is_variable(node) {
        Some("a variable")
    } else if is_keyword_value(node) {
        Some("a keyword")
    } else if is_constant(node) {
        Some("a constant")
    } else {
        None
    }
}

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_array_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
}

fn is_variable(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
}

fn is_keyword_value(node: &ruby_prism::Node<'_>) -> bool {
    node.as_self_node().is_some()
        || node.as_source_file_node().is_some()
        || node.as_source_line_node().is_some()
        || node.as_source_encoding_node().is_some()
}

fn is_constant(node: &ruby_prism::Node<'_>) -> bool {
    node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantParentheses, "cops/style/redundant_parentheses");
}
