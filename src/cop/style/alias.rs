use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "prefer_alias");
        let mut visitor = AliasVisitor {
            cop: self,
            source,
            enforced_style,
            scope_stack: vec![ScopeType::Lexical],
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct AliasVisitor<'a, 'src> {
    cop: &'a Alias,
    source: &'src SourceFile,
    enforced_style: &'a str,
    scope_stack: Vec<ScopeType>,
    diagnostics: Vec<Diagnostic>,
}

impl AliasVisitor<'_, '_> {
    fn current_scope(&self) -> ScopeType {
        *self.scope_stack.last().unwrap_or(&ScopeType::Lexical)
    }

    /// Check if alias_method can be replaced with alias keyword.
    /// Requires: not in dynamic scope, and all arguments are symbols.
    fn alias_keyword_possible(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if self.current_scope() == ScopeType::Dynamic {
            return false;
        }
        // Check that arguments are symbol literals (not interpolated symbols or other types)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
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
    fn alias_method_possible(&self) -> bool {
        self.current_scope() != ScopeType::InstanceEval
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

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'_>) {
        self.scope_stack.push(ScopeType::Lexical);
        ruby_prism::visit_singleton_class_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'_>) {
        self.scope_stack.push(ScopeType::Dynamic);
        ruby_prism::visit_def_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'_>) {
        self.scope_stack.push(ScopeType::Dynamic);
        ruby_prism::visit_lambda_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_alias_method_node(&mut self, node: &ruby_prism::AliasMethodNode<'_>) {
        let scope = self.current_scope();

        if self.enforced_style == "prefer_alias_method" {
            if self.alias_method_possible() {
                let loc = node.location();
                let kw_slice = &self.source.as_bytes()[loc.start_offset()..];
                if kw_slice.starts_with(b"alias ") || kw_slice.starts_with(b"alias\t") {
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `alias_method` instead of `alias`.".to_string(),
                    ));
                }
            }
        } else {
            // prefer_alias style: if inside dynamic scope (def or block),
            // flag alias to use alias_method instead
            if scope == ScopeType::Dynamic {
                if self.alias_method_possible() {
                    let loc = node.location();
                    let kw_slice = &self.source.as_bytes()[loc.start_offset()..];
                    if kw_slice.starts_with(b"alias ") || kw_slice.starts_with(b"alias\t") {
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Use `alias_method` instead of `alias`.".to_string(),
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
            if name.as_slice() == b"alias_method" && node.receiver().is_none() {
                if self.alias_keyword_possible(node) {
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
        }

        // If this call has a block, push appropriate scope for the block body
        if node.block().is_some() {
            let scope = if node.name().as_slice() == b"instance_eval" {
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
