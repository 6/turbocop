use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for non-local exits from iterators without a return value.
///
/// ## Corpus investigation (2026-03-10)
///
/// Earlier fixes addressed two large issues:
/// - returns inside method bodies were skipped entirely because recursion stopped
///   at `DefNode`
/// - `lambda { }` call blocks were not treated as scope boundaries
///
/// Remaining FPs on the March 10, 2026 corpus run came from safe-navigation
/// chains such as `items&.keys&.each do |item| ... return ... end`. RuboCop's
/// matcher only treats ordinary `send` nodes with receivers as chained iterator
/// sends; `csend` / `&.` chains are excluded. The Prism port incorrectly treated
/// any receiver-bearing `CallNode` as chained.
///
/// Fix: only mark a block call as chained when its operator is not `&.`.
pub struct NonLocalExitFromIterator;

impl Cop for NonLocalExitFromIterator {
    fn name(&self) -> &'static str {
        "Lint/NonLocalExitFromIterator"
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
        let mut visitor = NonLocalExitVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            block_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Tracks block context for determining whether a `return` is a non-local exit.
///
/// ## Root cause of FNs (236 in corpus)
/// The original implementation stopped recursion at `def` nodes, meaning returns
/// inside method bodies were never analyzed. Since virtually all real-world Ruby
/// code has `return` inside methods, this caused massive false negatives.
///
/// ## Root cause of FPs
/// - Earlier FP bucket: `lambda { }` (Kernel#lambda method call) was not
///   recognized as a scope boundary.
/// - Current FP bucket: safe-navigation block sends (`&.each`, `&.map`, etc.)
///   were treated as chained iterator sends even though RuboCop excludes `csend`.
///
/// ## Fix
/// - Recurse into def/class/module nodes but save/restore the block stack so
///   returns inside them don't see blocks from an outer scope.
/// - Treat `lambda { }` calls (and `-> { }` stabby lambdas) as scope boundaries
///   that prevent return-from-iterator detection.
/// - Exclude safe-navigation block sends (`&.`) from chained-send detection.
/// - Match RuboCop's ancestor behavior for chained calls with blocks: for
///   `recv_call { ... return ... }.each { |x| ... }`, the `return` in the
///   receiver block still has the outer `.each` block as an ancestor. To mirror
///   this, visit receiver/arguments while the current call's block context is
///   active, not before pushing it.
#[derive(Clone)]
enum StackEntry {
    /// A block attached to a method call.
    Block {
        has_args: bool,
        is_chained_send: bool,
        is_define_method: bool,
    },
    /// Scope boundary (def, class, module, lambda) — stops ancestor walk.
    Scope,
}

struct NonLocalExitVisitor<'a, 'src> {
    cop: &'a NonLocalExitFromIterator,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    block_stack: Vec<StackEntry>,
}

impl NonLocalExitVisitor<'_, '_> {
    /// Visit a node inside a new scope (def, class, module).
    /// Saves and restores the block stack so returns inside the scope
    /// cannot see blocks from an outer scope.
    fn visit_in_new_scope<'pr>(&mut self, node: &ruby_prism::Node<'pr>) {
        let saved = std::mem::take(&mut self.block_stack);
        self.visit(node);
        self.block_stack = saved;
    }
}

impl<'pr> Visit<'pr> for NonLocalExitVisitor<'_, '_> {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // Per RuboCop: only flag `return` with NO return value
        if node.arguments().is_some() {
            return;
        }

        // Walk block stack from innermost to outermost (matching RuboCop's each_ancestor)
        for entry in self.block_stack.iter().rev() {
            match entry {
                StackEntry::Scope => break,
                StackEntry::Block {
                    has_args,
                    is_chained_send,
                    is_define_method,
                } => {
                    if *is_define_method {
                        break; // define_method creates its own scope
                    }
                    if !has_args {
                        continue; // Skip blocks without arguments, keep looking outward
                    }
                    if *is_chained_send {
                        // This is a non-local exit from an iterator
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(
                            self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Non-local exit from iterator, without return value. \
                                 `next`, `break`, `Array#find`, `Array#any?`, etc. is preferred."
                                    .to_string(),
                            ),
                        );
                        break;
                    }
                    // Block has args but no receiver — not an iterator, continue looking outward
                }
            }
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let has_args = node.parameters().is_some();
        // Default: standalone block (no call context known)
        self.block_stack.push(StackEntry::Block {
            has_args,
            is_chained_send: false,
            is_define_method: false,
        });
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.block_stack.pop();
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // RuboCop ancestor walking treats the block attached to this call as an
        // ancestor even when `return` appears in the receiver subtree
        // (e.g. `foo { return }.each { |x| ... }`).
        //
        // To mirror that behavior, when a call has a concrete block node we
        // push this block/scope entry first, then visit receiver/arguments/body
        // while that context is active.
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                let method_name = node.name().as_slice();

                // `lambda { }` (Kernel#lambda) creates its own scope for return,
                // just like `-> { }` (stabby lambda / LambdaNode).
                let is_lambda = method_name == b"lambda" && node.receiver().is_none();

                if is_lambda {
                    self.block_stack.push(StackEntry::Scope);
                    if let Some(recv) = node.receiver() {
                        self.visit(&recv);
                    }
                    if let Some(args) = node.arguments() {
                        self.visit(&args.as_node());
                    }
                    if let Some(body) = block_node.body() {
                        self.visit(&body);
                    }
                    self.block_stack.pop();
                } else {
                    let has_args = block_node.parameters().is_some();
                    let is_chained_send = node.receiver().is_some()
                        && node
                            .call_operator_loc()
                            .is_none_or(|op: ruby_prism::Location<'_>| op.as_slice() != b"&.");
                    let is_define_method = method_name == b"define_method"
                        || method_name == b"define_singleton_method";

                    self.block_stack.push(StackEntry::Block {
                        has_args,
                        is_chained_send,
                        is_define_method,
                    });
                    if let Some(recv) = node.receiver() {
                        self.visit(&recv);
                    }
                    if let Some(args) = node.arguments() {
                        self.visit(&args.as_node());
                    }
                    if let Some(body) = block_node.body() {
                        self.visit(&body);
                    }
                    self.block_stack.pop();
                }
                return;
            } else {
                // BlockArgumentNode (&block) - visit it normally
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

        // No block on this call.
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
    }

    // Recurse into def/class/module but in a new scope so outer blocks are invisible.
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit_in_new_scope(&body);
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit_in_new_scope(&body);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit_in_new_scope(&body);
        }
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit_in_new_scope(&body);
        }
    }

    // Stabby lambda `-> { }` creates its own scope — push Scope marker and recurse.
    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.block_stack.push(StackEntry::Scope);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.block_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;
    crate::cop_fixture_tests!(
        NonLocalExitFromIterator,
        "cops/lint/non_local_exit_from_iterator"
    );

    #[test]
    fn test_return_inside_lambda_call_block() {
        // lambda { ... } creates its own scope - return should not be flagged
        // This is different from LambdaNode (-> { }) - it's a CallNode for Kernel#lambda
        let source =
            b"items.each do |item|\n  callback = lambda do\n    return if item.nil?\n  end\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            0,
            "return inside lambda block should NOT be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn test_return_inside_proc_new_block() {
        // Proc.new { ... } does NOT create its own scope for return
        // but return inside blocks passed to Proc.new could be tricky
        let source =
            b"items.each do |item|\n  items.map do |x|\n    return if x.nil?\n  end\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "return in nested iterator should be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn test_return_in_iterator_inside_lambda() {
        // return is directly inside count.times (iterator) which is inside lambda.
        // The lambda scopes the outer times, but the inner times is still an iterator.
        // RuboCop flags this because the closest block ancestor is the iterator, not lambda.
        let source = b"count.times do |i|\n  callback = lambda do\n    count.times do |ix|\n      return if something\n    end\n  end\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "return in iterator inside lambda should be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn test_return_directly_in_lambda_not_in_iterator() {
        // return directly in lambda (not inside any inner iterator) should NOT be flagged
        let source =
            b"count.times do |i|\n  callback = lambda do\n    return if something\n  end\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            0,
            "return directly in lambda should NOT be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn test_return_in_ivar_each() {
        // Pattern from corpus: @ivar.each do |x| ... return ... end
        let source =
            b"def process\n  @items.each do |item|\n    return if item.blank?\n  end\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "return in ivar.each should be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn test_return_in_rspec_it_inside_each() {
        // Pattern from corpus: each iterator wrapping RSpec `it` block with bare return
        let source = b"ITEMS.each do |val, expected|\n  it \"works\" do\n    return if RUBY_VERSION < '1.9'\n    assert_equal expected, parse(val)\n  end\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "return in it block inside each should be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn test_chained_method_call_receiver() {
        // Method chain as receiver: Pathname.new(...).ascend do |x| return end
        let source = b"Pathname.new(path).ascend do |parent|\n  return if parent == target\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "should flag return in chained method block: {:?}",
            diags
        );
    }

    #[test]
    fn test_constant_receiver() {
        // Constant as receiver: ITEMS.each do |x| return end
        let source = b"ITEMS.each do |type, _|\n  return if type == target\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "should flag return with constant receiver: {:?}",
            diags
        );
    }

    #[test]
    fn test_numblock_each() {
        // numblock: items.each do ... _1 ... end
        let source = b"items.each do\n  return if baz?(_1)\n  _1.update!(foobar: true)\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "should flag return in numblock each: {:?}",
            diags
        );
    }

    #[test]
    fn test_basic_each() {
        let source = b"items.each do |item|\n  return if item.nil?\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "should flag return in each block: {:?}",
            diags
        );
    }

    #[test]
    fn test_curly_brace_block() {
        let source = b"items.select { |item| return if item.nil? }\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(
            diags.len(),
            1,
            "should flag return in curly brace block: {:?}",
            diags
        );
    }

    #[test]
    fn test_nested_no_args_block() {
        // return in argless block with receiver, inside block with args
        let source = b"transaction do\n  items.each do |item|\n    return if item.nil?\n    item.with_lock do\n      return if item.stock == 0\n    end\n  end\nend\n";
        let diags = run_cop_full(&NonLocalExitFromIterator, source);
        assert_eq!(diags.len(), 2, "should flag both returns: {:?}", diags);
    }
}
