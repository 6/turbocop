use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/Alias: enforces `alias` vs `alias_method` usage.
///
/// Investigation (2026-03-13): FP=428 caused by treating `class_eval` and `module_eval`
/// blocks as `Lexical` scope. RuboCop treats ALL blocks (except `instance_eval`) as
/// `:dynamic` scope, which means `alias_method` inside `class_eval`/`module_eval` is
/// NOT flagged (because `alias` keyword doesn't work in dynamic eval contexts).
/// Fix: removed special-casing of `class_eval`/`module_eval` as `Lexical` — they now
/// correctly use `Dynamic` scope like all other non-instance_eval blocks.
///
/// Investigation (2026-03-17): FP=154 from two root causes:
/// 1. `class << self` (SingletonClassNode) was treated as Lexical scope boundary,
///    but RuboCop's scope_type does NOT match `:sclass` — only `:class` and `:module`.
///    This caused `alias_method` inside `class << self` inside a block/def to appear
///    as Lexical scope, hiding the enclosing Dynamic scope. Fix: removed
///    `visit_singleton_class_node` so singleton class is transparent to scope.
/// 2. RuboCop's `alias_method_possible?` returns false when there is any `:def`
///    ancestor (but NOT `:defs`). This means `alias` keyword inside a `def` method
///    is never flagged for "use alias_method", even with blocks in between. Fix: added
///    `def_depth` counter incremented only for non-singleton DefNodes (`def foo`, not
///    `def self.foo`), and `alias_method_possible()` returns false when def_depth > 0.
///
/// Investigation (2026-03-31): FN=5054 from missing symbol-args detection.
/// RuboCop's `on_alias` flags `alias :foo :bar` in lexical scope with prefer_alias
/// style, suggesting `alias foo bar` instead (MSG_SYMBOL_ARGS). The check fires when
/// both arguments are symbol-prefixed (none are barewords). Fix: added detection in
/// `visit_alias_method_node` for the case where both new_name and old_name are
/// SymbolNodes with `opening_loc` (the `:` prefix), emitting the appropriate message.
/// Resolved all 5,054 FN with 0 regressions.
pub struct Alias;

/// Scope type for determining whether alias or alias_method should be used.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ScopeType {
    /// Top-level, class body, or module body
    Lexical,
    /// Inside a def, defs, or non-instance_eval block
    Dynamic,
    /// Inside an instance_eval block
    InstanceEval,
}

impl Cop for Alias {
    fn name(&self) -> &'static str {
        "Style/Alias"
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
        let enforced_style = config.get_str("EnforcedStyle", "prefer_alias");
        let mut visitor = AliasVisitor {
            cop: self,
            source,
            enforced_style,
            scope_stack: vec![ScopeType::Lexical],
            def_depth: 0,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct AliasVisitor<'a, 'src> {
    cop: &'a Alias,
    source: &'src SourceFile,
    enforced_style: &'a str,
    scope_stack: Vec<ScopeType>,
    /// Tracks nesting depth inside `def` (not `def self.foo`).
    /// RuboCop's `alias_method_possible?` returns false when any `:def` ancestor exists.
    def_depth: u32,
    diagnostics: Vec<Diagnostic>,
}

impl AliasVisitor<'_, '_> {
    fn current_scope(&self) -> ScopeType {
        *self.scope_stack.last().unwrap_or(&ScopeType::Lexical)
    }

    /// Check if alias_method can be replaced with alias keyword.
    /// Requires: not in dynamic scope, exactly 2 symbol arguments.
    fn alias_keyword_possible(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if self.current_scope() == ScopeType::Dynamic {
            return false;
        }
        // Must have exactly 2 symbol literal arguments
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 2 {
                return false;
            }
            for arg in &arg_list {
                if arg.as_symbol_node().is_none() {
                    return false;
                }
            }
        } else {
            return false;
        }
        true
    }

    /// Check if alias keyword can be replaced with alias_method.
    /// Returns false inside instance_eval (alias_method doesn't work there)
    /// or when inside a `def` (not `defs`) — matching RuboCop's
    /// `node.each_ancestor(:def).none?` check.
    fn alias_method_possible(&self) -> bool {
        self.current_scope() != ScopeType::InstanceEval && self.def_depth == 0
    }
}

impl Visit<'_> for AliasVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'_>) {
        self.scope_stack.push(ScopeType::Lexical);
        ruby_prism::visit_class_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'_>) {
        self.scope_stack.push(ScopeType::Lexical);
        ruby_prism::visit_module_node(self, node);
        self.scope_stack.pop();
    }

    // NOTE: No visit_singleton_class_node override — `class << self` is NOT a scope
    // boundary in RuboCop (`:sclass` is not matched in scope_type). The enclosing
    // scope passes through transparently.

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'_>) {
        // Only track def_depth for regular `def` (no receiver), not `def self.foo`.
        // This matches RuboCop's `node.each_ancestor(:def).none?` which only checks
        // `:def`, not `:defs`.
        let is_regular_def = node.receiver().is_none();
        if is_regular_def {
            self.def_depth += 1;
        }
        self.scope_stack.push(ScopeType::Dynamic);
        ruby_prism::visit_def_node(self, node);
        self.scope_stack.pop();
        if is_regular_def {
            self.def_depth -= 1;
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'_>) {
        self.scope_stack.push(ScopeType::Dynamic);
        ruby_prism::visit_lambda_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_alias_method_node(&mut self, node: &ruby_prism::AliasMethodNode<'_>) {
        let scope = self.current_scope();
        let loc = node.location();
        let kw_slice = &self.source.content[loc.start_offset()..];
        let is_alias_keyword = kw_slice.starts_with(b"alias ") || kw_slice.starts_with(b"alias\t");

        if !is_alias_keyword {
            return;
        }

        if self.enforced_style == "prefer_alias_method" {
            if self.alias_method_possible() {
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use `alias_method` instead of `alias`.".to_string(),
                ));
            }
        } else {
            // prefer_alias style
            if !self.alias_method_possible() {
                return;
            }

            if scope == ScopeType::Dynamic {
                // alias in dynamic scope should use alias_method
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use `alias_method` instead of `alias`.".to_string(),
                ));
            } else {
                // In lexical scope: flag alias :sym :sym to use barewords instead
                if let (Some(new_sym), Some(old_sym)) = (
                    node.new_name().as_symbol_node(),
                    node.old_name().as_symbol_node(),
                ) {
                    // Both must have : prefix (opening_loc present) — not barewords
                    if new_sym.opening_loc().is_some() && old_sym.opening_loc().is_some() {
                        let new_name = std::str::from_utf8(new_sym.unescaped()).unwrap_or("");
                        let old_name = std::str::from_utf8(old_sym.unescaped()).unwrap_or("");
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            format!(
                                "Use `alias {} {}` instead of `alias :{} :{}`.",
                                new_name, old_name, new_name, old_name
                            ),
                        ));
                    }
                }
            }
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Check for alias_method call (prefer_alias style)
        if self.enforced_style == "prefer_alias" {
            let name = node.name();
            if name.as_slice() == b"alias_method"
                && node.receiver().is_none()
                && self.alias_keyword_possible(node)
            {
                let msg_loc = node.message_loc().unwrap_or_else(|| node.location());
                let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use `alias` instead of `alias_method`.".to_string(),
                ));
            }
        }

        // If this call has a block, push appropriate scope for the block body
        if node.block().is_some() {
            let name = node.name().as_slice();
            let scope = if name == b"instance_eval" {
                ScopeType::InstanceEval
            } else {
                ScopeType::Dynamic
            };
            self.scope_stack.push(scope);
            ruby_prism::visit_call_node(self, node);
            self.scope_stack.pop();
        } else {
            ruby_prism::visit_call_node(self, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Alias, "cops/style/alias");
}
