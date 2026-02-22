use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RescueType;

impl Cop for RescueType {
    fn name(&self) -> &'static str {
        "Lint/RescueType"
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
        let mut visitor = RescueTypeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RescueTypeVisitor<'a, 'src> {
    cop: &'a RescueType,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RescueTypeVisitor<'_, '_> {
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        let exceptions = node.exceptions();
        let mut invalid = Vec::new();

        for exc in exceptions.iter() {
            if is_invalid_rescue_type(&exc) {
                let loc = exc.location();
                let src = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
                let src_str = std::str::from_utf8(src).unwrap_or("?").to_string();
                invalid.push(src_str);
            }
        }

        if !invalid.is_empty() {
            let loc = node.keyword_loc();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!(
                    "Rescuing from `{}` will raise a `TypeError` instead of catching the actual exception.",
                    invalid.join(", ")
                ),
            ));
        }

        // Continue visiting child rescue nodes (subsequent)
        ruby_prism::visit_rescue_node(self, node);
    }
}

fn is_invalid_rescue_type(node: &ruby_prism::Node<'_>) -> bool {
    node.as_nil_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_interpolated_string_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RescueType, "cops/lint/rescue_type");
}
