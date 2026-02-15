use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Blank;

impl Cop for Blank {
    fn name(&self) -> &'static str {
        "Rails/Blank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Sub-cop toggles
        let nil_or_empty = config.get_bool("NilOrEmpty", true);
        let not_present = config.get_bool("NotPresent", true);
        let unless_present = config.get_bool("UnlessPresent", true);
        let _ = (nil_or_empty, unless_present); // reserved for future sub-cop checks

        if !not_present {
            return Vec::new();
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Looking for `!` operator (negation)
        if call.name().as_slice() != b"!" {
            return Vec::new();
        }

        // The receiver should be a `present?` call
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if inner_call.name().as_slice() != b"present?" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `blank?` instead of `!present?`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Blank, "cops/rails/blank");
}
