use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Detects bare `next` (without accumulator argument) inside `reduce`/`inject` blocks.
///
/// ## Corpus investigation (2026-03-28)
///
/// Corpus oracle reported FP=0, FN=6.
///
/// **FN root cause 1:** the visitor overrode `visit_def_node`, `visit_class_node`,
/// and `visit_module_node` to skip recursion entirely. The fixture only covered
/// top-level reduce blocks, so the cop appeared to work there, but real-world
/// offenses inside method bodies were never visited.
///
/// **FN root cause 2:** the cop tracked reduce context with a single boolean.
/// That would have flagged bare `next` inside nested blocks after recursion was
/// fixed, while RuboCop only flags `next` whose nearest enclosing block is the
/// current `reduce`/`inject` block.
///
/// Fix: recurse normally through method/class/module bodies, and track actual
/// block depth plus the active reduce-block depth so nested blocks are ignored
/// unless they are themselves `reduce`/`inject` blocks.
///
/// ## Corpus investigation (2026-03-31)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// **FP root cause:** nitrocop treated any block attached to `reduce`/`inject`
/// as accumulator-bearing, including `collection.reduce { |item| ... }` and
/// `collection.reduce(:+) { ... }`. RuboCop only checks reduce/inject calls
/// with an explicit, non-symbol accumulator argument, so bare `next` inside a
/// no-argument reduction is accepted upstream and must be ignored here.
pub struct NextWithoutAccumulator;

impl Cop for NextWithoutAccumulator {
    fn name(&self) -> &'static str {
        "Lint/NextWithoutAccumulator"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut visitor = NextWithoutAccVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            block_depth: 0,
            reduce_block_depths: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NextWithoutAccVisitor<'a, 'src> {
    cop: &'a NextWithoutAccumulator,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    block_depth: usize,
    reduce_block_depths: Vec<usize>,
}

impl<'pr> Visit<'pr> for NextWithoutAccVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(block_node) = node.block().and_then(|block| block.as_block_node()) {
            if let Some(recv) = node.receiver() {
                self.visit(&recv);
            }
            if let Some(args) = node.arguments() {
                self.visit(&args.as_node());
            }

            self.block_depth += 1;

            let is_reduce = reduce_call_with_explicit_accumulator(node);
            if is_reduce {
                self.reduce_block_depths.push(self.block_depth);
            }

            if let Some(body) = block_node.body() {
                self.visit(&body);
            }

            if is_reduce {
                self.reduce_block_depths.pop();
            }
            self.block_depth -= 1;
            return;
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

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.block_depth += 1;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.block_depth -= 1;
    }

    fn visit_next_node(&mut self, node: &ruby_prism::NextNode<'pr>) {
        if node.arguments().is_none()
            && self
                .reduce_block_depths
                .last()
                .is_some_and(|depth| *depth == self.block_depth)
        {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `next` with an accumulator argument in a `reduce`.".to_string(),
            ));
        }
    }
}

fn reduce_call_with_explicit_accumulator(node: &ruby_prism::CallNode<'_>) -> bool {
    if node.receiver().is_none() {
        return false;
    }

    let method_name = node.name().as_slice();
    if method_name != b"reduce" && method_name != b"inject" {
        return false;
    }

    let Some(arguments) = node.arguments() else {
        return false;
    };
    let Some(first_argument) = arguments.arguments().iter().next() else {
        return false;
    };

    first_argument.as_symbol_node().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NextWithoutAccumulator, "cops/lint/next_without_accumulator");
}
