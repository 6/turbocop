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
                    // Skip if the preferred name is shadowed by a local variable in the body
                    let preferred_for_var = if var_str.starts_with('_') {
                        &underscore_preferred
                    } else {
                        self.preferred
                    };
                    if self.preferred_name_shadowed(rescue_node, preferred_for_var) {
                        // Don't flag — renaming would shadow an existing variable
                    } else {
                        let loc = local_var.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            format!(
                                "Use `{}` instead of `{}` for rescued exceptions.",
                                preferred_for_var, var_str,
                            ),
                        ));
                    }
                }
            }
        }

        // Check subsequent rescue clauses in the same chain (they're at the same depth)
        if let Some(subsequent) = rescue_node.subsequent() {
            self.check_rescue(&subsequent);
        }
    }

    /// Check if the preferred name appears as a local variable (read or write)
    /// anywhere in the rescue body. This matches RuboCop's `shadowed_variable_name?`.
    fn preferred_name_shadowed(
        &self,
        rescue_node: &ruby_prism::RescueNode<'_>,
        preferred: &str,
    ) -> bool {
        let preferred_bytes = preferred.as_bytes();
        if let Some(body) = rescue_node.statements() {
            let mut checker = ShadowChecker {
                preferred: preferred_bytes,
                found: false,
            };
            checker.visit_statements_node(&body);
            checker.found
        } else {
            false
        }
    }
}

/// Visitor that checks if a preferred variable name appears as a local variable
/// (read, write, or rescue target) in the body of a rescue clause.
struct ShadowChecker<'a> {
    preferred: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for ShadowChecker<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.preferred {
            self.found = true;
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.preferred {
            self.found = true;
        }
        // Continue visiting children (the value expression may contain reads)
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        if node.name().as_slice() == self.preferred {
            self.found = true;
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
