use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for array literals interpolated inside regexps.
/// When interpolating an array literal, it is converted to a string,
/// which is likely not the intended behavior inside a regexp.
pub struct ArrayLiteralInRegexp;

impl Cop for ArrayLiteralInRegexp {
    fn name(&self) -> &'static str {
        "Lint/ArrayLiteralInRegexp"
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
        let mut visitor = RegexpArrayVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_regexp: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RegexpArrayVisitor<'a, 'src> {
    cop: &'a ArrayLiteralInRegexp,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_regexp: bool,
}

impl<'pr> Visit<'pr> for RegexpArrayVisitor<'_, '_> {
    fn visit_interpolated_regular_expression_node(
        &mut self,
        node: &ruby_prism::InterpolatedRegularExpressionNode<'pr>,
    ) {
        // Check if any parts contain array literals
        for part in node.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if let Some(last) = body.last() {
                        if last.as_array_node().is_some() {
                            // Report at the regex location, not the embedded node
                            let loc = node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Use alternation or a character class instead of interpolating an array in a regexp.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        ruby_prism::visit_interpolated_regular_expression_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayLiteralInRegexp, "cops/lint/array_literal_in_regexp");
}
