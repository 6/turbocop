use crate::cop::variable_force::{self, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for every useless assignment to local variable in every scope.
///
/// ## Implementation
///
/// This cop is a thin consumer of the shared VariableForce engine. The engine
/// walks the AST once, building per-scope variable models with per-assignment
/// liveness tracking, branch analysis, loop back-edge marking, and
/// captured-by-block detection. This consumer simply iterates each scope's
/// variables at `before_leaving_scope` and reports assignments whose values
/// are never read.
///
/// ## Migration from standalone implementation
///
/// Previously this was a 1,900-line standalone AST walker with its own
/// control-flow approximation. The VariableForce engine provides more accurate
/// analysis (branch-aware reassignment, branch-aware referencing, loop
/// back-edges, rescue/retry loop detection) in ~90 lines of consumer code.
pub struct UselessAssignment;

impl Cop for UselessAssignment {
    fn name(&self) -> &'static str {
        "Lint/UselessAssignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for UselessAssignment {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for variable in scope.variables.values() {
            // Underscore-prefixed variables are intentionally unused.
            if variable.should_be_unused() {
                continue;
            }

            for assignment in &variable.assignments {
                if assignment.used(variable.captured_by_block) {
                    continue;
                }

                let (line, column) = source.offset_to_line_col(assignment.node_offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Useless assignment to variable - `{}`.",
                        String::from_utf8_lossy(&variable.name)
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAssignment, "cops/lint/useless_assignment");
}
