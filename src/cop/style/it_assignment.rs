use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ItAssignment;

impl Cop for ItAssignment {
    fn name(&self) -> &'static str {
        "Style/ItAssignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Detect assignment to `it` variable: it = something
        let write_node = match node.as_local_variable_write_node() {
            Some(w) => w,
            None => return Vec::new(),
        };

        if write_node.name().as_slice() != b"it" {
            return Vec::new();
        }

        let loc = write_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid assigning to local variable `it`, since `it` will be the default block parameter in Ruby 3.4+. Consider using a different variable name.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ItAssignment, "cops/style/it_assignment");
}
