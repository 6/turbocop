use ruby_prism::Visit;

use crate::cop::util::is_blank_or_whitespace_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/EmptyLinesAfterModuleInclusion
///
/// Checks for an empty line after a module inclusion method (`extend`,
/// `include` and `prepend`), or a group of them.
///
/// ## Investigation findings (2026-03-11)
///
/// **Root cause of 493 FPs (2026-03-11):** nitrocop was firing on include/extend/prepend
/// inside `if`/`unless` bodies. RuboCop's `next_line_node` method returns nil
/// when `node.parent.if_type?`, which blanket-skips all includes inside
/// conditional branches. nitrocop had no equivalent check.
///
/// **Fix:** Added `in_if_body` flag to the visitor, set when visiting
/// `IfNode` and `UnlessNode` bodies. The cop now skips include calls
/// when this flag is true, matching RuboCop's behavior.
///
/// **Also handles:** RuboCop's `allowed_method?` unwraps modifier forms
/// (`include Baz if condition`) — nitrocop's line-based check already
/// handles this because `is_module_inclusion_line` matches lines starting
/// with `include`/`extend`/`prepend` regardless of trailing modifiers.
///
/// ## Investigation findings (2026-03-14)
///
/// **FP root cause (447):** `is_blank_or_whitespace_line()` only matches truly empty lines,
/// not whitespace-only lines. RuboCop's `blank?` considers whitespace-only
/// lines as blank. Many real-world files have trailing spaces/tabs on
/// "empty" lines after includes. Fix: use `is_blank_or_whitespace_line()`.
///
/// **FN root cause (181):** `in_if_body` flag was not reset when entering
/// class/module/singleton class bodies. RuboCop only checks if the include's
/// DIRECT parent is `if_type?`. When `include` is inside a class nested in
/// an `if` block, the parent is the class body (begin/StatementsNode), not
/// the `if` node. Fix: reset `in_if_body` in class/module/singleton class
/// visitors, same as `in_block_or_send` was already reset.
pub struct EmptyLinesAfterModuleInclusion;

const MODULE_INCLUSION_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

/// Check if a trimmed line starts with a module inclusion method call.
fn is_module_inclusion_line(trimmed: &[u8]) -> bool {
    for &method in MODULE_INCLUSION_METHODS {
        if trimmed.starts_with(method) {
            let after = trimmed.get(method.len());
            if after.is_none() || matches!(after, Some(b' ') | Some(b'(')) {
                return true;
            }
        }
    }
    false
}

impl Cop for EmptyLinesAfterModuleInclusion {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAfterModuleInclusion"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = InclusionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            in_block_or_send: false,
            in_if_body: false,
        };
        visitor.visit(&parse_result.node());
        if let Some(ref mut corr) = corrections {
            for d in &mut visitor.diagnostics {
                d.corrected = true;
            }
            corr.extend(visitor.corrections);
        }
        diagnostics.extend(visitor.diagnostics);
    }
}

struct InclusionVisitor<'a> {
    cop: &'a EmptyLinesAfterModuleInclusion,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    /// True when inside a block, lambda, or array — contexts where
    /// include/extend/prepend are NOT module inclusions
    in_block_or_send: bool,
    /// True when inside an if/unless body — RuboCop's `next_line_node` returns
    /// nil when `node.parent.if_type?`, so the cop never fires in these contexts.
    in_if_body: bool,
}

impl InclusionVisitor<'_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must be a bare call (no receiver)
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !MODULE_INCLUSION_METHODS.contains(&method_name) {
            return;
        }

        // Must have arguments (e.g., `include Foo`)
        if call.arguments().is_none() {
            return;
        }

        // Skip if inside a block, array, or used as argument to another call
        // (matches RuboCop: `return if node.parent&.type?(:send, :any_block, :array)`)
        if self.in_block_or_send {
            return;
        }

        // Skip if inside an if/unless body
        // (matches RuboCop: `return if node.parent&.if_type?` in `next_line_node`)
        if self.in_if_body {
            return;
        }

        let loc = call.location();
        let (last_line, _) = self
            .source
            .offset_to_line_col(loc.end_offset().saturating_sub(1));
        let lines: Vec<&[u8]> = self.source.lines().collect();

        // Check if the next line exists
        if last_line >= lines.len() {
            return; // End of file
        }

        let next_line = lines[last_line]; // next line (0-indexed)

        // If next line is blank, no offense
        if is_blank_or_whitespace_line(next_line) {
            return;
        }

        // If next line is end of class/module, no offense
        let next_trimmed: Vec<u8> = next_line
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        if next_trimmed.starts_with(b"end") {
            let after_end = next_trimmed.get(3);
            if after_end.is_none()
                || matches!(
                    after_end,
                    Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'#')
                )
            {
                return;
            }
        }

        // If next line is another module inclusion, no offense
        if is_module_inclusion_line(&next_trimmed) {
            return;
        }

        // If next line is a comment, skip over comments and check if followed
        // by another module inclusion (RuboCop treats comments between includes
        // as part of the include block)
        if next_trimmed.starts_with(b"#") {
            let mut idx = last_line + 1;
            while idx < lines.len() {
                let line_trimmed: Vec<u8> = lines[idx]
                    .iter()
                    .copied()
                    .skip_while(|&b| b == b' ' || b == b'\t')
                    .collect();
                if is_blank_or_whitespace_line(lines[idx]) || line_trimmed.starts_with(b"#") {
                    idx += 1;
                    continue;
                }
                // Found a non-blank, non-comment line
                if is_module_inclusion_line(&line_trimmed) {
                    return;
                }
                break;
            }
        }

        // If next line is a rubocop:enable directive comment, check the line after
        if next_trimmed.starts_with(b"# rubocop:enable")
            || next_trimmed.starts_with(b"#rubocop:enable")
        {
            // Check the line after the enable directive
            if last_line + 1 < lines.len() {
                let line_after = lines[last_line + 1];
                if is_blank_or_whitespace_line(line_after) {
                    return;
                }
            } else {
                return; // enable directive at end of file
            }
        }

        let (line, col) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            col,
            "Add an empty line after module inclusion.".to_string(),
        ));
        // Insert a blank line after the inclusion line
        if let Some(offset) = self.source.line_col_to_offset(last_line + 1, 0) {
            self.corrections.push(crate::correction::Correction {
                start: offset,
                end: offset,
                replacement: "\n".to_string(),
                cop_name: self.cop.name(),
                cop_index: 0,
            });
        }
    }
}

impl<'pr> Visit<'pr> for InclusionVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call is an include/extend/prepend at the right level
        self.check_call(node);

        // When descending into arguments of a call node, mark that we're
        // inside a "send" context. This prevents include/extend/prepend
        // used as arguments (e.g., `.and include(Foo)`) from being flagged.
        if let Some(args) = node.arguments() {
            let was = self.in_block_or_send;
            self.in_block_or_send = true;
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
            self.in_block_or_send = was;
        }

        // Visit receiver normally
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }

        // Visit block argument if any (this IS a block context)
        if let Some(block) = node.block() {
            let was = self.in_block_or_send;
            self.in_block_or_send = true;
            self.visit(&block);
            self.in_block_or_send = was;
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let was = self.in_block_or_send;
        // In RuboCop, `return if node.parent&.type?(:send, :any_block, :array)`.
        // This only skips when the include's *direct parent* is a block.
        // In Prism, when a block body has multiple statements, they are children
        // of a StatementsNode, so the include's parent is NOT the block.
        // Only set in_block_or_send for single-statement block bodies.
        // For multi-statement bodies, RESET to false so nested includes are checked.
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                if stmts.body().len() <= 1 {
                    // Single statement: include's parent would be the block in RuboCop
                    self.in_block_or_send = true;
                } else {
                    // Multiple statements: include's parent is begin/StatementsNode
                    // Reset flag so nested includes at this level are checked
                    self.in_block_or_send = false;
                }
            }
            self.visit(&body);
        }
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        self.in_block_or_send = was;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let was = self.in_block_or_send;
        // Same logic as block_node: only set in_block_or_send for single-statement bodies
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.in_block_or_send = stmts.body().len() <= 1;
            }
            self.visit(&body);
        }
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        self.in_block_or_send = was;
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let was = self.in_block_or_send;
        self.in_block_or_send = true;
        for elem in node.elements().iter() {
            self.visit(&elem);
        }
        self.in_block_or_send = was;
    }

    // Class and module bodies reset the block context — include/extend/prepend
    // at the class/module body level SHOULD be flagged.
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let was_block = self.in_block_or_send;
        let was_if = self.in_if_body;
        self.in_block_or_send = false;
        self.in_if_body = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was_block;
        self.in_if_body = was_if;
        // Visit superclass expression
        if let Some(sup) = node.superclass() {
            self.visit(&sup);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let was_block = self.in_block_or_send;
        let was_if = self.in_if_body;
        self.in_block_or_send = false;
        self.in_if_body = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was_block;
        self.in_if_body = was_if;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let was_block = self.in_block_or_send;
        let was_if = self.in_if_body;
        self.in_block_or_send = false;
        self.in_if_body = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was_block;
        self.in_if_body = was_if;
    }

    // If/unless bodies: RuboCop's `next_line_node` returns nil when
    // `node.parent.if_type?`, so include/extend/prepend inside if/unless
    // bodies are never flagged.
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let was = self.in_if_body;
        self.in_if_body = true;
        ruby_prism::visit_if_node(self, node);
        self.in_if_body = was;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let was = self.in_if_body;
        self.in_if_body = true;
        ruby_prism::visit_unless_node(self, node);
        self.in_if_body = was;
    }

    // Method bodies should be treated as block context — include inside
    // a method is not a module inclusion
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let was = self.in_block_or_send;
        self.in_block_or_send = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAfterModuleInclusion,
        "cops/layout/empty_lines_after_module_inclusion"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAfterModuleInclusion,
        "cops/layout/empty_lines_after_module_inclusion"
    );
}
