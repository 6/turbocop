use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-13)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// FP=2: both false positives came from `class << self` bodies that contain
/// multi-argument mixin macros such as `include Foo, Bar` in puppetlabs/puppet.
///
/// Root cause: RuboCop only defines `on_class` and `on_module` (aliased) for
/// this cop — there is no `on_sclass`. So `class << self` bodies are never
/// checked by RuboCop. nitrocop was incorrectly visiting `SingletonClassNode`.
///
/// Fix: removed `visit_singleton_class_node` entirely. The default Visit impl
/// still recurses into singleton class children, so any nested class/module
/// nodes inside `class << self` are still checked by `visit_class_node` /
/// `visit_module_node`.
///
/// Previous attempts that also landed at actual=181 likely had a different bug
/// (e.g., breaking the recursive visit into singleton class children).
pub struct MixinGrouping;

const MIXIN_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

impl Cop for MixinGrouping {
    fn name(&self) -> &'static str {
        "Style/MixinGrouping"
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
        let style = config.get_str("EnforcedStyle", "separated").to_string();
        let mut visitor = MixinGroupingVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            style,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MixinGroupingVisitor<'a> {
    cop: &'a MixinGrouping,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    style: String,
}

impl MixinGroupingVisitor<'_> {
    fn check_body_statements(&mut self, stmts: &ruby_prism::StatementsNode<'_>) {
        for stmt in stmts.body().iter() {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            let method_bytes = call.name().as_slice();

            if !MIXIN_METHODS.contains(&method_bytes) {
                continue;
            }

            // Must not have a receiver (bare include/extend/prepend)
            if call.receiver().is_some() {
                continue;
            }

            let args = match call.arguments() {
                Some(a) => a,
                None => continue,
            };

            let arg_list: Vec<_> = args.arguments().iter().collect();

            if self.style == "separated" && arg_list.len() > 1 {
                let method_str = std::str::from_utf8(method_bytes).unwrap_or("include");
                let loc = call.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("Put `{method_str}` mixins in separate statements."),
                ));
            }
        }
    }
}

impl<'pr> Visit<'pr> for MixinGroupingVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.check_body_statements(&stmts);
            }
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                self.check_body_statements(&stmts);
            }
        }
        ruby_prism::visit_module_node(self, node);
    }

    // Note: RuboCop's on_class/on_module do NOT handle sclass (class << self).
    // We intentionally skip visit_singleton_class_node. The default Visit impl
    // still recurses into singleton class children, so nested class/module nodes
    // inside class << self are still checked.
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MixinGrouping, "cops/style/mixin_grouping");
}
