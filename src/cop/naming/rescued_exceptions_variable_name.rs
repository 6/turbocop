use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RescuedExceptionsVariableName;

fn check_rescue_clause(
    cop: &RescuedExceptionsVariableName,
    source: &SourceFile,
    rescue_node: &ruby_prism::RescueNode<'_>,
    preferred: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(reference) = rescue_node.reference() {
        if let Some(local_var) = reference.as_local_variable_target_node() {
            let var_name = local_var.name().as_slice();
            let var_str = std::str::from_utf8(var_name).unwrap_or("");
            if var_str != preferred {
                let loc = local_var.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Use `{preferred}` instead of `{var_str}` for rescued exceptions."
                    ),
                ));
            }
        }
    }

    // Check subsequent rescue clauses in the chain
    if let Some(subsequent) = rescue_node.subsequent() {
        check_rescue_clause(cop, source, &subsequent, preferred, diagnostics);
    }
}

impl Cop for RescuedExceptionsVariableName {
    fn name(&self) -> &'static str {
        "Naming/RescuedExceptionsVariableName"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let preferred = config.get_str("PreferredName", "e");

        let begin_node = match node.as_begin_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let rescue_clause = match begin_node.rescue_clause() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        check_rescue_clause(self, source, &rescue_clause, preferred, &mut diagnostics);
        diagnostics
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
