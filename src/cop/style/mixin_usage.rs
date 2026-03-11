use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (round 1): 14+ FPs from `include T('default/layout/html')` in YARD
/// templates. Root cause: we checked `node.arguments().is_some()` which matches any argument
/// including method calls. RuboCop's node pattern requires arguments to be `const` nodes.
/// Fixed by verifying all arguments are ConstantReadNode or ConstantPathNode before flagging.
///
/// Corpus investigation (round 2): 6 FPs from `include M` inside `while`, `until`, `for`,
/// `case`, and lambda/proc blocks at the top level. Root cause: nitrocop only tracked
/// `in_class_or_module` and `in_block` (BlockNode) as scope barriers, but missed other
/// constructs. RuboCop's `in_top_level_scope?` pattern only considers `begin`, `kwbegin`,
/// `if`, and `def` as transparent wrappers — everything else (while, until, for, case,
/// lambda, etc.) creates an opaque scope. Fixed by replacing the opt-out approach with an
/// opt-in approach: only transparent nodes (if, def, begin) pass through the top-level flag.
///
/// Corpus investigation (round 3): a live 2026-03-11 rerun disproved two tempting
/// follow-up theories. Treating multi-argument calls like `include A, B` as
/// no-offense introduced 27 FN, so RuboCop clearly still counts many of those
/// calls despite the simplified node-pattern reading. Treating `begin ...
/// rescue/ensure` as opaque regressed even harder (32 FN). The remaining FPs
/// need a narrower explanation than argument count or exception-wrapper shape.
pub struct MixinUsage;

const MIXIN_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

impl Cop for MixinUsage {
    fn name(&self) -> &'static str {
        "Style/MixinUsage"
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
        let mut visitor = MixinUsageVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_opaque_scope: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MixinUsageVisitor<'a> {
    cop: &'a MixinUsage,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when we're inside a scope that is NOT considered "top level" by RuboCop.
    /// RuboCop's `in_top_level_scope?` only treats `begin`, `kwbegin`, `if`, and `def`
    /// as transparent wrappers. Everything else (class, module, block, while, until,
    /// for, case, lambda, etc.) creates an opaque scope where mixin calls are allowed.
    in_opaque_scope: bool,
}

impl<'pr> Visit<'pr> for MixinUsageVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_bytes = node.name().as_slice();

        if MIXIN_METHODS.contains(&method_bytes)
            && node.receiver().is_none()
            && !self.in_opaque_scope
        {
            // RuboCop's node pattern requires `const` args. Method call arguments like
            // `include T('...')` are not flagged.
            let is_const_mixin = node.arguments().is_some_and(|args| {
                args.arguments().iter().all(|arg| {
                    arg.as_constant_read_node().is_some() || arg.as_constant_path_node().is_some()
                })
            });

            if is_const_mixin {
                let method_str = std::str::from_utf8(method_bytes).unwrap_or("include");
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "`{method_str}` is used at the top level. Use inside `class` or `module`."
                    ),
                ));
            }
        }

        // Visit children
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    // === Transparent wrappers (RuboCop considers these still "top level") ===
    // `begin`/`kwbegin`, `if`, and `def` are transparent.
    // No need to override visit_begin_node, visit_if_node, or visit_def_node —
    // the default traversal descends into children without changing in_opaque_scope.

    // === Opaque scopes (mixin calls inside these are NOT top-level) ===

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_opaque_scope = prev;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_opaque_scope = prev;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_opaque_scope = prev;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_opaque_scope = prev;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_opaque_scope = prev;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        ruby_prism::visit_while_node(self, node);
        self.in_opaque_scope = prev;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        ruby_prism::visit_until_node(self, node);
        self.in_opaque_scope = prev;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        ruby_prism::visit_for_node(self, node);
        self.in_opaque_scope = prev;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        ruby_prism::visit_case_node(self, node);
        self.in_opaque_scope = prev;
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        ruby_prism::visit_case_match_node(self, node);
        self.in_opaque_scope = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MixinUsage, "cops/style/mixin_usage");
}
