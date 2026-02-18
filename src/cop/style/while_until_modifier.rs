use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct WhileUntilModifier;

/// Returns true if the node or any descendant contains a local variable assignment.
fn contains_lvar_assignment(node: &ruby_prism::Node<'_>) -> bool {
    struct LvarAssignChecker {
        found: bool,
    }
    impl<'pr> Visit<'pr> for LvarAssignChecker {
        fn visit_local_variable_write_node(&mut self, _node: &ruby_prism::LocalVariableWriteNode<'pr>) {
            self.found = true;
        }
        fn visit_local_variable_and_write_node(&mut self, _node: &ruby_prism::LocalVariableAndWriteNode<'pr>) {
            self.found = true;
        }
        fn visit_local_variable_or_write_node(&mut self, _node: &ruby_prism::LocalVariableOrWriteNode<'pr>) {
            self.found = true;
        }
        fn visit_local_variable_operator_write_node(&mut self, _node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>) {
            self.found = true;
        }
    }
    let mut checker = LvarAssignChecker { found: false };
    checker.visit(node);
    checker.found
}

impl Cop for WhileUntilModifier {
    fn name(&self) -> &'static str {
        "Style/WhileUntilModifier"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let (kw_loc, statements, keyword) = if let Some(while_node) = node.as_while_node() {
            (while_node.keyword_loc(), while_node.statements(), "while")
        } else if let Some(until_node) = node.as_until_node() {
            (until_node.keyword_loc(), until_node.statements(), "until")
        } else {
            return Vec::new();
        };

        // Skip modifier form — check if keyword comes before the closing
        let closing_loc = if let Some(while_node) = node.as_while_node() {
            while_node.closing_loc()
        } else if let Some(until_node) = node.as_until_node() {
            until_node.closing_loc()
        } else {
            return Vec::new();
        };

        // If no closing (end), it's already modifier form
        if closing_loc.is_none() {
            return Vec::new();
        }

        let body = match statements {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_stmts: Vec<_> = body.body().iter().collect();
        if body_stmts.len() != 1 {
            return Vec::new();
        }

        let body_node = &body_stmts[0];

        // Body must be on a single line
        let (body_start_line, _) = source.offset_to_line_col(body_node.location().start_offset());
        let body_end_off = body_node.location().end_offset().saturating_sub(1).max(body_node.location().start_offset());
        let (body_end_line, _) = source.offset_to_line_col(body_end_off);
        if body_start_line != body_end_line {
            return Vec::new();
        }

        // Check if the modifier form would fit within the max line length.
        // RuboCop considers Layout/LineLength Max (default 120).
        let max_line_length = _config
            .options
            .get("MaxLineLength")
            .and_then(|v| v.as_u64())
            .unwrap_or(120) as usize;

        // Estimate modifier form length: "body keyword condition"
        let body_src = &source.as_bytes()[body_node.location().start_offset()..body_node.location().end_offset()];
        let body_str = String::from_utf8_lossy(body_src);
        let body_trimmed = body_str.trim();

        let predicate = if let Some(while_node) = node.as_while_node() {
            while_node.predicate()
        } else if let Some(until_node) = node.as_until_node() {
            until_node.predicate()
        } else {
            return Vec::new();
        };

        // Skip if the condition contains a local variable assignment
        // (e.g., `while (chunk = file.read(1024))`)
        if contains_lvar_assignment(&predicate) {
            return Vec::new();
        }

        let pred_src = &source.as_bytes()[predicate.location().start_offset()..predicate.location().end_offset()];
        let pred_str = String::from_utf8_lossy(pred_src);

        // Calculate indentation of the original while/until keyword
        let kw_offset = kw_loc.start_offset();
        let src_bytes = source.as_bytes();
        // Walk back to find the start of the line
        let line_start = src_bytes[..kw_offset].iter().rposition(|&b| b == b'\n').map(|p| p + 1).unwrap_or(0);
        let indent = src_bytes[line_start..].iter().take_while(|&&b| b == b' ' || b == b'\t').count();

        // "  body keyword condition"
        let modifier_len = indent + body_trimmed.len() + 1 + keyword.len() + 1 + pred_str.len();
        if modifier_len > max_line_length {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Favor modifier `{}` usage when having a single-line body.", keyword),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhileUntilModifier, "cops/style/while_until_modifier");
}
