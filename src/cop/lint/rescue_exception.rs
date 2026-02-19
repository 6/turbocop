use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RescueException;

impl Cop for RescueException {
    fn name(&self) -> &'static str {
        "Lint/RescueException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Match BeginNode to get rescue_clause
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
        let mut rescue_opt = begin_node.rescue_clause();

        while let Some(rescue_node) = rescue_opt {
            for exception in rescue_node.exceptions().iter() {
                let is_exception = if let Some(const_read) = exception.as_constant_read_node() {
                    // Bare `Exception`
                    const_read.name().as_slice() == b"Exception"
                } else if let Some(const_path) = exception.as_constant_path_node() {
                    // Only match `::Exception` (top-level), not `Gem::Exception` etc.
                    const_path.parent().is_none()
                        && const_path
                            .name()
                            .is_some_and(|n| n.as_slice() == b"Exception")
                } else {
                    false
                };

                if is_exception {
                    let loc = exception.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid rescuing the `Exception` class. Perhaps you meant `StandardError`?"
                            .to_string(),
                    ));
                }
            }
            rescue_opt = rescue_node.subsequent();
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RescueException, "cops/lint/rescue_exception");
}
