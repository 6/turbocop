use ruby_prism::Visit;

use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAfterModuleInclusion;

const MODULE_INCLUSION_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

impl Cop for EmptyLinesAfterModuleInclusion {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAfterModuleInclusion"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut visitor = InclusionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            // Track whether we're in a context where include/extend/prepend
            // should be treated as module inclusion (class/module body level)
            in_block_or_send: false,
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct InclusionVisitor<'a> {
    cop: &'a EmptyLinesAfterModuleInclusion,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when inside a block, lambda, or array — contexts where
    /// include/extend/prepend are NOT module inclusions
    in_block_or_send: bool,
}

impl InclusionVisitor<'_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must be a bare call (no receiver)
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !MODULE_INCLUSION_METHODS.iter().any(|&m| m == method_name) {
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

        let loc = call.location();
        let (last_line, _) = self.source.offset_to_line_col(loc.end_offset().saturating_sub(1));
        let lines: Vec<&[u8]> = self.source.lines().collect();

        // Check if the next line exists
        if last_line >= lines.len() {
            return; // End of file
        }

        let next_line = lines[last_line]; // next line (0-indexed)

        // If next line is blank, no offense
        if is_blank_line(next_line) {
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
        for &method in MODULE_INCLUSION_METHODS {
            if next_trimmed.starts_with(method) {
                let after = next_trimmed.get(method.len());
                if after.is_none() || matches!(after, Some(b' ') | Some(b'(')) {
                    return;
                }
            }
        }

        // If next line is a rubocop:enable directive comment, check the line after
        if next_trimmed.starts_with(b"# rubocop:enable") || next_trimmed.starts_with(b"#rubocop:enable") {
            // Check the line after the enable directive
            if last_line + 1 < lines.len() {
                let line_after = lines[last_line + 1];
                if is_blank_line(line_after) {
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
        self.in_block_or_send = true;
        // Visit all children within the block
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        self.in_block_or_send = was;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let was = self.in_block_or_send;
        self.in_block_or_send = true;
        if let Some(body) = node.body() {
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
        let was = self.in_block_or_send;
        self.in_block_or_send = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was;
        // Visit superclass expression
        if let Some(sup) = node.superclass() {
            self.visit(&sup);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let was = self.in_block_or_send;
        self.in_block_or_send = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let was = self.in_block_or_send;
        self.in_block_or_send = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_block_or_send = was;
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
}
