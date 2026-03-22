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
/// Corpus investigation (round 3): 6 FPs from `include`/`extend`/`prepend` inside
/// `begin...rescue` or `begin...ensure` blocks at the top level. In RuboCop's Parser AST,
/// `begin...rescue...end` wraps the body in a `rescue` node, making it opaque (not in the
/// transparent `{kwbegin begin if def}` list). In Prism, statements are direct children
/// of `BeginNode`. Fixed by overriding `visit_begin_node` to mark the scope as opaque when
/// `rescue_clause` or `ensure_clause` is present. Plain `begin...end` remains transparent.
///
/// Corpus investigation (round 4): 2 FPs from `include GravatarHelper, GravatarHelper::PublicMethods, ERB::Util`
/// in redmine forks. Root cause: RuboCop's node pattern `(send nil? ${:include :extend :prepend} const)`
/// matches exactly ONE `const` argument. Multi-argument mixin calls like `include A, B, C`
/// don't match the pattern and are not flagged. nitrocop was incorrectly accepting any number
/// of const arguments. Fixed by requiring exactly one argument in the const check.
///
/// Corpus investigation (round 5): 3 FPs from `include UtilityFunctions` inside `BEGIN { ... }`
/// blocks in the linguistics corpus. Prism exposes these bodies under `PreExecutionNode`, but
/// RuboCop does not consider `BEGIN` a transparent top-level wrapper. Fixed by treating
/// `PreExecutionNode` as an opaque scope so only real top-level / begin / if / def wrappers match.
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
            // RuboCop's node pattern `(send nil? ${:include :extend :prepend} const)`
            // matches exactly ONE `const` argument. Multi-argument calls like
            // `include A, B, C` don't match, nor do method call arguments like
            // `include T('...')`.
            let is_single_const_mixin = node.arguments().is_some_and(|args| {
                let arguments: Vec<_> = args.arguments().iter().collect();
                arguments.len() == 1
                    && (arguments[0].as_constant_read_node().is_some()
                        || arguments[0].as_constant_path_node().is_some())
            });

            if is_single_const_mixin {
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
    // No need to override visit_if_node or visit_def_node —
    // the default traversal descends into children without changing in_opaque_scope.
    //
    // However, `begin...rescue...end` and `begin...ensure...end` are special:
    // In RuboCop's Parser AST, the `rescue`/`ensure` node becomes the parent of
    // the body statements, and `rescue`/`ensure` is NOT in the transparent list.
    // So we must treat BeginNode with rescue/ensure as opaque.
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let has_rescue_or_ensure = node.rescue_clause().is_some() || node.ensure_clause().is_some();
        if has_rescue_or_ensure {
            let prev = self.in_opaque_scope;
            self.in_opaque_scope = true;
            ruby_prism::visit_begin_node(self, node);
            self.in_opaque_scope = prev;
        } else {
            // Plain `begin...end` without rescue/ensure is transparent
            ruby_prism::visit_begin_node(self, node);
        }
    }

    // === Opaque scopes (mixin calls inside these are NOT top-level) ===

    fn visit_pre_execution_node(&mut self, node: &ruby_prism::PreExecutionNode<'pr>) {
        let prev = self.in_opaque_scope;
        self.in_opaque_scope = true;
        ruby_prism::visit_pre_execution_node(self, node);
        self.in_opaque_scope = prev;
    }

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
