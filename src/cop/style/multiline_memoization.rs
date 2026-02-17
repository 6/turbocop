use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMemoization;

impl Cop for MultilineMemoization {
    fn name(&self) -> &'static str {
        "Style/MultilineMemoization"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "keyword");

        // Look for `||=` (OrAssign nodes)
        let or_assign = match node.as_or_write_node()
            .map(|n| (n.location(), n.value()))
            .or_else(|| {
                node.as_instance_variable_or_write_node()
                    .map(|n| (n.location(), n.value()))
            })
            .or_else(|| {
                node.as_class_variable_or_write_node()
                    .map(|n| (n.location(), n.value()))
            })
            .or_else(|| {
                node.as_local_variable_or_write_node()
                    .map(|n| (n.location(), n.value()))
            })
            .or_else(|| {
                node.as_global_variable_or_write_node()
                    .map(|n| (n.location(), n.value()))
            })
        {
            Some(info) => info,
            None => return Vec::new(),
        };

        let (assign_loc, value) = or_assign;

        // Check if the value spans multiple lines
        let assign_line = source.offset_to_line_col(assign_loc.start_offset()).0;
        let value_end_offset = value.location().start_offset() + value.location().as_slice().len();
        let value_end_line = source.offset_to_line_col(value_end_offset.saturating_sub(1)).0;

        if assign_line == value_end_line {
            // Single line - not a multiline memoization
            return Vec::new();
        }

        // It's multiline. Check the wrapping style.
        if enforced_style == "keyword" {
            // keyword style: should use begin..end, not parentheses
            if value.as_parentheses_node().is_some() {
                let (line, column) = source.offset_to_line_col(assign_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap multiline memoization blocks in `begin` and `end`.".to_string(),
                )];
            }
        } else if enforced_style == "braces" {
            // braces style: should use parentheses, not begin..end
            if value.as_begin_node().is_some() {
                let (line, column) = source.offset_to_line_col(assign_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap multiline memoization blocks in `(` and `)`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineMemoization, "cops/style/multiline_memoization");
}
