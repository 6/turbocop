use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Enforces safe navigation chains length to not exceed the configured maximum.
///
/// ## Investigation findings (2026-03-30)
///
/// **Root cause of 40 FNs:** nitrocop only counted `&.` through receiver links, so it
/// missed RuboCop chains that continue through Parser send children such as
/// `session['view_token']&.[](record&.id&.to_s)`. It also treated any `CallNode.block()`
/// as a chain boundary, but Prism uses `block()` for both real `BlockNode`s and block-pass
/// arguments like `&:inspect`, which RuboCop still counts through.
///
/// **Fix:** Switched to a RuboCop-style ancestor walk over safe-navigation call ancestors,
/// skipping Prism-only `ArgumentsNode` wrappers and treating only real `BlockNode`s as
/// chain boundaries. This preserves the existing no-offense behavior for real blocks while
/// detecting chains with block-pass arguments and nested safe-navigation calls inside
/// arguments.
///
/// ## Investigation findings (2026-03-15)
///
/// **Root cause of 28 FPs:** nitrocop counted `&.` operators across block boundaries,
/// but RuboCop's Parser gem AST wraps `a&.method { block }` in a `block` node that
/// breaks the csend ancestor chain. In Prism, blocks are children of CallNode, so
/// naive receiver-chain walking doesn't see the boundary.
///
/// **Fix:** When counting the safe navigation chain downward through receivers,
/// if a CallNode has a block, count that `&.` but stop recursing into its receiver.
/// This matches RuboCop's behavior where block nodes break `each_ancestor` traversal.
///
/// **Message fix:** Changed from "Do not chain more than N safe navigation operators.
/// (found M)" to "Avoid safe navigation chains longer than N calls." to match RuboCop.
pub struct SafeNavigationChainLength;

impl Cop for SafeNavigationChainLength {
    fn name(&self) -> &'static str {
        "Style/SafeNavigationChainLength"
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
        let mut visitor = SafeNavigationChainLengthVisitor {
            cop: self,
            source,
            max: config.get_usize("Max", 2),
            diagnostics: Vec::new(),
            // Stored outermost-first so index 0 is the offense location RuboCop reports.
            safe_nav_ancestors: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

fn is_safe_nav(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(op) = call.call_operator_loc() {
        op.as_slice() == b"&."
    } else {
        false
    }
}

fn has_real_block(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block().and_then(|block| block.as_block_node()).is_some()
}

struct SafeNavigationChainLengthVisitor<'a, 'src> {
    cop: &'a SafeNavigationChainLength,
    source: &'src SourceFile,
    max: usize,
    diagnostics: Vec<Diagnostic>,
    /// Active safe-navigation ancestors visible from the current traversal path.
    ///
    /// This mirrors RuboCop's `each_ancestor` behavior:
    /// - safe-nav parents stay visible through receivers and arguments
    /// - regular call parents reset the chain
    /// - real blocks (`BlockNode`) hide ancestors above the blocked call
    /// - block-pass args (`&:sym`, `&method`) do NOT break the chain
    safe_nav_ancestors: Vec<usize>,
}

impl SafeNavigationChainLengthVisitor<'_, '_> {
    fn maybe_add_offense(&mut self) {
        if self.safe_nav_ancestors.len() < self.max {
            return;
        }

        let outermost_start = self.safe_nav_ancestors[0];
        let message = format!(
            "Avoid safe navigation chains longer than {} calls.",
            self.max
        );
        let (line, column) = self.source.offset_to_line_col(outermost_start);
        if self.diagnostics.iter().any(|diagnostic| {
            diagnostic.location.line == line
                && diagnostic.location.column == column
                && diagnostic.cop_name == self.cop.name()
                && diagnostic.message == message
        }) {
            return;
        }

        self.diagnostics
            .push(self.cop.diagnostic(self.source, line, column, message));
    }

    fn visit_chain_child<'pr>(&mut self, node: &ruby_prism::Node<'pr>) {
        let saved_ancestors = self.safe_nav_ancestors.clone();
        if node.as_call_node().is_none() {
            self.safe_nav_ancestors.clear();
        }
        self.visit(node);
        self.safe_nav_ancestors = saved_ancestors;
    }
}

impl<'pr> Visit<'pr> for SafeNavigationChainLengthVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let is_safe = is_safe_nav(node);
        let real_block = has_real_block(node);
        let saved_ancestors = self.safe_nav_ancestors.clone();

        if is_safe && !real_block {
            self.maybe_add_offense();
        }

        self.safe_nav_ancestors = if is_safe {
            let mut ancestors = if real_block {
                Vec::new()
            } else {
                saved_ancestors.clone()
            };
            ancestors.push(node.location().start_offset());
            ancestors
        } else {
            Vec::new()
        };

        if let Some(receiver) = node.receiver() {
            self.visit_chain_child(&receiver);
        }
        if let Some(arguments) = node.arguments() {
            for argument in arguments.arguments().iter() {
                self.visit_chain_child(&argument);
            }
        }
        if let Some(block) = node.block() {
            if real_block || !is_safe {
                self.safe_nav_ancestors.clear();
            }
            self.visit_chain_child(&block);
        }

        self.safe_nav_ancestors = saved_ancestors;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SafeNavigationChainLength,
        "cops/style/safe_navigation_chain_length"
    );
}
