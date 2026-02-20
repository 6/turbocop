use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RescuedExceptionsVariableName;

impl Cop for RescuedExceptionsVariableName {
    fn name(&self) -> &'static str {
        "Naming/RescuedExceptionsVariableName"
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
        let preferred = config.get_str("PreferredName", "e");
        let mut visitor = RescuedVarVisitor {
            cop: self,
            source,
            preferred,
            diagnostics: Vec::new(),
            rescue_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RescuedVarVisitor<'a, 'src> {
    cop: &'a RescuedExceptionsVariableName,
    source: &'src SourceFile,
    preferred: &'a str,
    diagnostics: Vec<Diagnostic>,
    rescue_depth: usize,
}

impl<'pr> Visit<'pr> for RescuedVarVisitor<'_, '_> {
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // Only check top-level rescues (not nested). RuboCop skips nested
        // rescues because renaming the inner variable could shadow the outer.
        if self.rescue_depth == 0 {
            self.check_rescue(node);
        }

        // Increment depth for body and descendant traversal
        self.rescue_depth += 1;
        ruby_prism::visit_rescue_node(self, node);
        self.rescue_depth -= 1;
    }
}

impl<'a, 'src> RescuedVarVisitor<'a, 'src> {
    fn check_rescue(&mut self, rescue_node: &ruby_prism::RescueNode<'_>) {
        if let Some(reference) = rescue_node.reference() {
            if let Some(local_var) = reference.as_local_variable_target_node() {
                let var_name = local_var.name().as_slice();
                let var_str = std::str::from_utf8(var_name).unwrap_or("");
                // Accept both "e" and "_e" (underscore-prefixed preferred name)
                let underscore_preferred = format!("_{}", self.preferred);
                if var_str != self.preferred && var_str != underscore_preferred {
                    let loc = local_var.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!(
                            "Use `{}` instead of `{}` for rescued exceptions.",
                            self.preferred, var_str,
                        ),
                    ));
                }
            }
        }

        // Check subsequent rescue clauses in the same chain (they're at the same depth)
        if let Some(subsequent) = rescue_node.subsequent() {
            self.check_rescue(&subsequent);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        RescuedExceptionsVariableName,
        "cops/naming/rescued_exceptions_variable_name"
    );
}
