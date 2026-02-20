use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct IneffectiveAccessModifier;

impl Cop for IneffectiveAccessModifier {
    fn name(&self) -> &'static str {
        "Lint/IneffectiveAccessModifier"
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
        let mut visitor = IneffectiveVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct IneffectiveVisitor<'a, 'src> {
    cop: &'a IneffectiveAccessModifier,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy)]
struct ModifierInfo {
    kind: ModifierKind,
    line: usize,
}

#[derive(Debug, Clone, Copy)]
enum ModifierKind {
    Private,
    Protected,
    Public,
}

fn check_class_body(
    cop: &IneffectiveAccessModifier,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    stmts: &ruby_prism::StatementsNode<'_>,
) {
    let body: Vec<_> = stmts.body().iter().collect();
    let mut current_modifier: Option<ModifierInfo> = None;

    for stmt in &body {
        // Check for bare access modifiers
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if call.receiver().is_none() && call.arguments().is_none() {
                let kind = match name {
                    b"private" => Some(ModifierKind::Private),
                    b"protected" => Some(ModifierKind::Protected),
                    b"public" => Some(ModifierKind::Public),
                    _ => None,
                };
                if let Some(k) = kind {
                    let (line, _) = source.offset_to_line_col(call.location().start_offset());
                    current_modifier = Some(ModifierInfo { kind: k, line });
                }
            }
        }

        // Check for singleton method definitions (def self.method)
        if let Some(defs) = stmt.as_def_node() {
            if defs.receiver().is_some() {
                // This is a `def self.method` or `def obj.method`
                if let Some(modifier) = &current_modifier {
                    match modifier.kind {
                        ModifierKind::Public => {}
                        ModifierKind::Private => {
                            let loc = defs.def_keyword_loc();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(cop.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "`private` (on line {}) does not make singleton methods private. Use `private_class_method` or `private` inside a `class << self` block instead.",
                                    modifier.line
                                ),
                            ));
                        }
                        ModifierKind::Protected => {
                            let loc = defs.def_keyword_loc();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(cop.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "`protected` (on line {}) does not make singleton methods protected. Use `protected` inside a `class << self` block instead.",
                                    modifier.line
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for IneffectiveVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_class_body(self.cop, self.source, &mut self.diagnostics, &stmts);
            }
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_class_body(self.cop, self.source, &mut self.diagnostics, &stmts);
            }
        }
        ruby_prism::visit_module_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IneffectiveAccessModifier,
        "cops/lint/ineffective_access_modifier"
    );
}
