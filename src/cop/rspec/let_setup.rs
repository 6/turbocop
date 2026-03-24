use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;

/// RSpec/LetSetup: Flag `let!` that is not referenced in tests (only used for side effects).
///
/// Investigation findings:
/// - The dominant false-positive pattern was inner `let!` overriding an outer `let!` with the
///   same name (e.g., `let!(:record) { nil }` inside a nested context that overrides a parent
///   `let!(:record) { create(...) }`). RuboCop skips these via `overrides_outer_let_bang?`.
/// - Implemented a recursive visitor that maintains a stack of ancestor `let!` names, so inner
///   overrides are correctly suppressed without needing parent node references.
/// - FN fix: Added `include_examples` and `include_context` to recognized group names.
///   RuboCop's `example_or_shared_group_or_including?` matches `Includes.all` which includes
///   `include_examples`, `include_context`, `it_behaves_like`, `it_should_behave_like`.
///   nitrocop was missing `include_examples` and `include_context`, causing 103 FNs.
/// - FN fix: Removed `LocalVariableReadNode` from `IdentifierCollector`. RuboCop's
///   `method_called?` uses `(send nil? %)` which only matches method sends, not local
///   variable reads. Multi-line `let!` bodies that assign to a local variable with the
///   same name as the `let!` (e.g., `let!(:order) do order = create(...); order end`)
///   had the local variable read falsely marking the name as "used".
/// - FN fix: `let!` declaration search now recurses through non-scope-change blocks
///   (e.g., `[].each do ... end`) to find `let!` calls, matching RuboCop's
///   `ExampleGroup#find_all_in_scope` behavior which stops only at scope changes
///   (other example groups) and examples.
/// - FN fix: Prism wraps `if ... else` bodies in `ElseNode`. The in-scope `let!`
///   declaration walk handled `IfNode` but not `ElseNode`, so unused `let!`
///   defined under `else` branches were skipped entirely.
pub struct LetSetup;

impl Cop for LetSetup {
    fn name(&self) -> &'static str {
        "RSpec/LetSetup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        let mut visitor = LetSetupVisitor {
            cop: self,
            source,
            diagnostics,
            ancestor_let_bang_names: Vec::new(),
        };
        visitor.visit(&parse_result.node());
    }
}

struct LetSetupVisitor<'a> {
    cop: &'a LetSetup,
    source: &'a SourceFile,
    diagnostics: &'a mut Vec<Diagnostic>,
    /// Stack of sets: each set contains the `let!` names defined at that ancestor scope level.
    ancestor_let_bang_names: Vec<HashSet<Vec<u8>>>,
}

impl<'pr> LetSetupVisitor<'_> {
    fn process_example_group(&mut self, block_node: &ruby_prism::BlockNode<'pr>) {
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect let! names (recursing through non-scope-change blocks)
        // and all method-call identifiers used in the same scope.
        let mut let_bang_decls: Vec<(Vec<u8>, usize, usize)> = Vec::new();
        let mut used_names: HashSet<Vec<u8>> = HashSet::new();
        let mut this_scope_let_bang_names: HashSet<Vec<u8>> = HashSet::new();

        for stmt in stmts.body().iter() {
            // Recursively find let! declarations through non-scope-change blocks,
            // matching RuboCop's ExampleGroup#find_all_in_scope behavior.
            self.collect_let_bangs_in_scope(
                &stmt,
                &mut let_bang_decls,
                &mut this_scope_let_bang_names,
            );
            // Walk ALL siblings (including let! bodies) for identifier
            // collection. This matches RuboCop behavior where method_called?
            // searches the entire example group block, so a let! name used
            // inside a sibling let! body is not flagged.
            let mut collector = IdentifierCollector {
                names: &mut used_names,
            };
            collector.visit(&stmt);
        }

        for (let_name, line, col) in &let_bang_decls {
            // Skip if this let! overrides an outer let! with the same name
            if self.overrides_outer_let_bang(let_name) {
                continue;
            }
            if !used_names.contains(let_name) {
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    *line,
                    *col,
                    "Do not use `let!` to setup objects not referenced in tests.".to_string(),
                ));
            }
        }

        // Push this scope's let! names onto the ancestor stack, then recurse into children
        self.ancestor_let_bang_names.push(this_scope_let_bang_names);
        for stmt in stmts.body().iter() {
            self.visit(&stmt);
        }
        self.ancestor_let_bang_names.pop();
    }

    /// Recursively search for `let!` calls within the current scope, stopping at
    /// scope changes (example groups, includes) and examples. This mirrors RuboCop's
    /// `ExampleGroup#find_all_in_scope` which recurses through non-scope-change blocks
    /// like iterators (`[].each do ... end`).
    fn collect_let_bangs_in_scope(
        &self,
        node: &ruby_prism::Node<'pr>,
        decls: &mut Vec<(Vec<u8>, usize, usize)>,
        scope_names: &mut HashSet<Vec<u8>>,
    ) {
        // Unwrap trailing if/unless modifiers: `let!(:foo) { } if cond` parses as
        // IfNode/UnlessNode wrapping the CallNode. Recurse into the body statement(s).
        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                for stmt in stmts.body().iter() {
                    self.collect_let_bangs_in_scope(&stmt, decls, scope_names);
                }
            }
            if let Some(subsequent) = if_node.subsequent() {
                self.collect_let_bangs_in_scope(&subsequent, decls, scope_names);
            }
            return;
        }
        if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                for stmt in stmts.body().iter() {
                    self.collect_let_bangs_in_scope(&stmt, decls, scope_names);
                }
            }
            return;
        }
        if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                for stmt in stmts.body().iter() {
                    self.collect_let_bangs_in_scope(&stmt, decls, scope_names);
                }
            }
            return;
        }
        if let Some(c) = node.as_call_node() {
            let m = c.name().as_slice();
            // If it's a let! call, record it
            if m == b"let!" && c.receiver().is_none() {
                if let Some(let_name) = extract_let_name(&c) {
                    let loc = c.location();
                    let (line, col) = self.source.offset_to_line_col(loc.start_offset());
                    scope_names.insert(let_name.clone());
                    decls.push((let_name, line, col));
                }
                return;
            }
            // If it's a scope change (example group or include) or an example,
            // stop recursing — let! declarations inside belong to that inner scope
            if is_example_group_or_include(m) || is_example(m) {
                return;
            }
            // For other calls with blocks (e.g., `[].each do ... end`),
            // recurse into the block body
            if let Some(block) = c.block() {
                if let Some(block_node) = block.as_block_node() {
                    if let Some(body) = block_node.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for stmt in stmts.body().iter() {
                                self.collect_let_bangs_in_scope(&stmt, decls, scope_names);
                            }
                        }
                    }
                }
            }
        }
    }

    fn overrides_outer_let_bang(&self, name: &[u8]) -> bool {
        self.ancestor_let_bang_names
            .iter()
            .any(|scope| scope.contains(name))
    }
}

impl<'pr> Visit<'pr> for LetSetupVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        if !is_example_group_or_include(name) {
            // Not an example group — continue default traversal
            ruby_prism::visit_call_node(self, node);
            return;
        }

        let block = match node.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Process this example group (handles let! detection + nested recursion)
        self.process_example_group(&block_node);
        // Don't call visit_call_node default — we already recursed into children
    }
}

fn extract_let_name(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first = args.arguments().iter().next()?;
    if let Some(sym) = first.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = first.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

/// Walks the entire AST subtree, collecting all receiverless call names.
/// This matches RuboCop's `method_called?` which uses `(send nil? %)` —
/// only method sends without a receiver, NOT local variable reads.
struct IdentifierCollector<'a> {
    names: &'a mut HashSet<Vec<u8>>,
}

impl<'pr> Visit<'pr> for IdentifierCollector<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.receiver().is_none() {
            self.names.insert(node.name().as_slice().to_vec());
        }
        ruby_prism::visit_call_node(self, node);
    }
}

/// Returns true for RSpec example methods (it, specify, example, etc.)
/// which define a new scope where let! declarations inside belong to
/// the enclosing example group, not nested further.
fn is_example(name: &[u8]) -> bool {
    matches!(
        name,
        b"it"
            | b"specify"
            | b"example"
            | b"its"
            | b"xit"
            | b"xspecify"
            | b"xexample"
            | b"fit"
            | b"fspecify"
            | b"fexample"
            | b"skip"
            | b"pending"
    )
}

fn is_example_group_or_include(name: &[u8]) -> bool {
    matches!(
        name,
        // ExampleGroups (regular, focused, skipped)
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
            // SharedGroups
            | b"shared_context"
            | b"shared_examples"
            | b"shared_examples_for"
            // Includes (Examples + Context)
            | b"it_behaves_like"
            | b"it_should_behave_like"
            | b"include_examples"
            | b"include_context"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(LetSetup, "cops/rspec/let_setup");
}
