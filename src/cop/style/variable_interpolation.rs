use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct VariableInterpolation;

impl Cop for VariableInterpolation {
    fn name(&self) -> &'static str {
        "Style/VariableInterpolation"
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
        let mut visitor = VarInterpVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct VarInterpVisitor<'a> {
    cop: &'a VariableInterpolation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl VarInterpVisitor<'_> {
    fn check_parts(&mut self, parts: ruby_prism::NodeList<'_>) {
        for part in parts.iter() {
            // Embedded variable nodes represent #@var, #@@var, #$var without braces
            if let Some(ev) = part.as_embedded_variable_node() {
                let var = ev.variable();
                let var_bytes = &self.source.as_bytes()[var.location().start_offset()..var.location().end_offset()];
                let var_str = String::from_utf8_lossy(var_bytes);

                let loc = var.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Replace interpolated variable `{}` with expression `#{{{}}}`.",
                        var_str, var_str
                    ),
                ));
            }
        }
    }
}

impl<'pr> Visit<'pr> for VarInterpVisitor<'_> {
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        self.check_parts(node.parts());
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_interpolated_regular_expression_node(&mut self, node: &ruby_prism::InterpolatedRegularExpressionNode<'pr>) {
        self.check_parts(node.parts());
        ruby_prism::visit_interpolated_regular_expression_node(self, node);
    }

    fn visit_interpolated_symbol_node(&mut self, node: &ruby_prism::InterpolatedSymbolNode<'pr>) {
        self.check_parts(node.parts());
        ruby_prism::visit_interpolated_symbol_node(self, node);
    }

    fn visit_interpolated_x_string_node(&mut self, node: &ruby_prism::InterpolatedXStringNode<'pr>) {
        self.check_parts(node.parts());
        ruby_prism::visit_interpolated_x_string_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VariableInterpolation, "cops/style/variable_interpolation");
}
