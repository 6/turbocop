use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct RedundantInterpolation;

impl Cop for RedundantInterpolation {
    fn name(&self) -> &'static str {
        "Style/RedundantInterpolation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RedundantInterpVisitor {
            cop: self,
            source,
            in_implicit_concat: false,
            in_percent_array: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantInterpVisitor<'a, 'src> {
    cop: &'a RedundantInterpolation,
    source: &'src SourceFile,
    in_implicit_concat: bool,
    in_percent_array: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RedundantInterpVisitor<'_, '_> {
    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        // Check if this is a %w[] or %W[] or %i[] or %I[] percent array
        let was_in_percent_array = self.in_percent_array;
        if let Some(open_loc) = node.opening_loc() {
            let open_bytes =
                &self.source.as_bytes()[open_loc.start_offset()..open_loc.end_offset()];
            if open_bytes.starts_with(b"%w")
                || open_bytes.starts_with(b"%W")
                || open_bytes.starts_with(b"%i")
                || open_bytes.starts_with(b"%I")
            {
                self.in_percent_array = true;
            }
        }

        // Visit children manually
        for element in node.elements().iter() {
            self.visit(&element);
        }

        self.in_percent_array = was_in_percent_array;
    }

    fn visit_interpolated_string_node(
        &mut self,
        node: &ruby_prism::InterpolatedStringNode<'pr>,
    ) {
        let is_implicit_concat = node.opening_loc().is_none();

        if is_implicit_concat {
            // This is an implicit concatenation node — skip flagging, but visit children
            let was = self.in_implicit_concat;
            self.in_implicit_concat = true;
            for part in node.parts().iter() {
                self.visit(&part);
            }
            self.in_implicit_concat = was;
            return;
        }

        // Skip if inside implicit concatenation or percent array
        if !self.in_implicit_concat && !self.in_percent_array {
            self.check_redundant_interpolation(node);
        }

        // Visit children
        for part in node.parts().iter() {
            self.visit(&part);
        }
    }
}

impl RedundantInterpVisitor<'_, '_> {
    fn check_redundant_interpolation(&mut self, node: &ruby_prism::InterpolatedStringNode<'_>) {
        // Must have exactly one part that is an embedded statements node
        let parts: Vec<_> = node.parts().into_iter().collect();
        if parts.len() != 1 {
            return;
        }

        let embedded = match parts[0].as_embedded_statements_node() {
            Some(e) => e,
            None => return,
        };

        // Must have exactly one statement inside #{...}
        let statements = match embedded.statements() {
            Some(s) => s,
            None => return,
        };

        let body: Vec<_> = statements.body().into_iter().collect();
        if body.len() != 1 {
            return;
        }

        // Skip if the inner expression is a string literal (that would be double-interpolation)
        let inner = &body[0];
        if inner.as_string_node().is_some() || inner.as_interpolated_string_node().is_some() {
            return;
        }

        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Prefer `to_s` over string interpolation.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantInterpolation, "cops/style/redundant_interpolation");
}
