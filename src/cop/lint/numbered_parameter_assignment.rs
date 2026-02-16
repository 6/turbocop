use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NumberedParameterAssignment;

impl Cop for NumberedParameterAssignment {
    fn name(&self) -> &'static str {
        "Lint/NumberedParameterAssignment"
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
        let write = match node.as_local_variable_write_node() {
            Some(w) => w,
            None => return Vec::new(),
        };

        let name = write.name();
        let name_bytes = name.as_slice();
        let name_str = match std::str::from_utf8(name_bytes) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Match pattern: _N where N is one or more digits
        if !name_str.starts_with('_') {
            return Vec::new();
        }

        let after_underscore = &name_str[1..];
        if after_underscore.is_empty() || !after_underscore.chars().all(|c| c.is_ascii_digit()) {
            return Vec::new();
        }

        let number: u64 = match after_underscore.parse() {
            Ok(n) => n,
            Err(_) => return Vec::new(),
        };

        let msg = if (1..=9).contains(&number) {
            format!("`_{number}` is reserved for numbered parameter; consider another name.")
        } else {
            format!("`_{number}` is similar to numbered parameter; consider another name.")
        };

        let loc = write.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NumberedParameterAssignment,
        "cops/lint/numbered_parameter_assignment"
    );
}
