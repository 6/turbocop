use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct HashLikeCase;

impl Cop for HashLikeCase {
    fn name(&self) -> &'static str {
        "Style/HashLikeCase"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let min_branches = config.get_usize("MinBranchesCount", 3);
        let mut visitor = HashLikeCaseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            min_branches,
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct HashLikeCaseVisitor<'a, 'src> {
    cop: &'a HashLikeCase,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    min_branches: usize,
}

impl HashLikeCaseVisitor<'_, '_> {
    fn is_simple_when(when_node: &ruby_prism::WhenNode<'_>) -> bool {
        // Must have exactly one condition
        let conditions: Vec<_> = when_node.conditions().iter().collect();
        if conditions.len() != 1 {
            return false;
        }
        // Condition should be a literal (string, symbol, integer)
        let cond = &conditions[0];
        cond.as_string_node().is_some()
            || cond.as_symbol_node().is_some()
            || cond.as_integer_node().is_some()
    }

    fn when_body_is_simple_value(when_node: &ruby_prism::WhenNode<'_>) -> bool {
        if let Some(stmts) = when_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if body.len() == 1 {
                let expr = &body[0];
                return expr.as_string_node().is_some()
                    || expr.as_symbol_node().is_some()
                    || expr.as_integer_node().is_some()
                    || expr.as_float_node().is_some()
                    || expr.as_nil_node().is_some()
                    || expr.as_true_node().is_some()
                    || expr.as_false_node().is_some();
            }
        }
        false
    }
}

impl<'pr> Visit<'pr> for HashLikeCaseVisitor<'_, '_> {
    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let conditions: Vec<_> = node.conditions().iter().collect();
        let when_count = conditions.len();

        if when_count < self.min_branches {
            ruby_prism::visit_case_node(self, node);
            return;
        }

        // All when branches must be simple 1:1 mappings
        let all_simple = conditions.iter().all(|c| {
            if let Some(when_node) = c.as_when_node() {
                Self::is_simple_when(&when_node) && Self::when_body_is_simple_value(&when_node)
            } else {
                false
            }
        });

        if all_simple {
            let loc = node.case_keyword_loc();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Consider replacing `case-when` with a hash lookup.".to_string(),
            ));
        }

        ruby_prism::visit_case_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashLikeCase, "cops/style/hash_like_case");
}
