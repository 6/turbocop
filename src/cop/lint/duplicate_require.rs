use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct DuplicateRequire;

impl Cop for DuplicateRequire {
    fn name(&self) -> &'static str {
        "Lint/DuplicateRequire"
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
        let mut visitor = RequireVisitor {
            cop: self,
            source,
            // Per RuboCop: scoped by parent StatementsNode.
            // Each StatementsNode is a unique parent scope.
            scope_stack: vec![HashSet::new()],
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RequireVisitor<'a, 'src> {
    cop: &'a DuplicateRequire,
    source: &'src SourceFile,
    /// Stack of seen sets: one per StatementsNode scope level.
    scope_stack: Vec<HashSet<(Vec<u8>, Vec<u8>)>>,
    diagnostics: Vec<Diagnostic>,
}

impl RequireVisitor<'_, '_> {
    fn check_require_call(&mut self, node: &ruby_prism::CallNode<'_>) {
        let method_name = node.name().as_slice();

        if (method_name != b"require" && method_name != b"require_relative")
            || node.receiver().is_some()
        {
            return;
        }

        if let Some(args) = node.arguments() {
            let arg_list = args.arguments();
            if arg_list.len() == 1 {
                if let Some(first) = arg_list.iter().next() {
                    if let Some(s) = first.as_string_node() {
                        let key = (method_name.to_vec(), s.unescaped().to_vec());
                        let loc = node.location();
                        let current_scope = self.scope_stack.last_mut().unwrap();
                        if current_scope.contains(&key) {
                            let (line, column) =
                                self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Duplicate `require` detected.".to_string(),
                            ));
                        } else {
                            current_scope.insert(key);
                        }
                    }
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for RequireVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_require_call(node);
        ruby_prism::visit_call_node(self, node);
    }

    // Each StatementsNode creates a new scope. This matches the vendor's
    // `node.parent` approach since each StatementsNode is a unique parent.
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        self.scope_stack.push(HashSet::new());
        ruby_prism::visit_statements_node(self, node);
        self.scope_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateRequire, "cops/lint/duplicate_require");
}
