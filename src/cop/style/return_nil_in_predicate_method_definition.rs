use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ReturnNilInPredicateMethodDefinition;

impl Cop for ReturnNilInPredicateMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/ReturnNilInPredicateMethodDefinition"
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
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_default();
        let allowed_patterns = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default();

        let mut visitor = PredicateReturnVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_methods,
            allowed_patterns,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct PredicateReturnVisitor<'a> {
    cop: &'a ReturnNilInPredicateMethodDefinition,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_methods: Vec<String>,
    allowed_patterns: Vec<String>,
}

impl<'pr> Visit<'pr> for PredicateReturnVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice();
        // Only check predicate methods (ending with ?)
        if !name.ends_with(b"?") {
            ruby_prism::visit_def_node(self, node);
            return;
        }

        // Check AllowedMethods
        let name_str = std::str::from_utf8(name).unwrap_or("");
        if self.allowed_methods.iter().any(|m| m == name_str) {
            return;
        }

        // Check AllowedPatterns
        for pattern in &self.allowed_patterns {
            if name_str.contains(pattern.as_str()) {
                return;
            }
        }

        // Scan body for return/return nil statements
        if let Some(body) = node.body() {
            let mut finder = ReturnFinder {
                returns: Vec::new(),
            };
            finder.visit(&body);

            for ret_loc in finder.returns {
                let (line, column) = self.source.offset_to_line_col(ret_loc.0);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Avoid using `return nil` or `return` in predicate methods.".to_string(),
                ));
            }
        }

        // Don't recurse into nested defs
    }
}

struct ReturnFinder {
    returns: Vec<(usize, usize)>, // (start_offset, end_offset)
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // Check for `return` (bare) or `return nil`
        let is_bare = node.arguments().is_none();
        let is_nil = if let Some(args) = node.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            arg_list.len() == 1 && arg_list[0].as_nil_node().is_some()
        } else {
            false
        };

        if is_bare || is_nil {
            self.returns
                .push((node.location().start_offset(), node.location().end_offset()));
        }
    }

    // Don't recurse into nested defs
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ReturnNilInPredicateMethodDefinition,
        "cops/style/return_nil_in_predicate_method_definition"
    );
}
