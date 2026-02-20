use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, OR_NODE};

pub struct DoubleStartEndWith;

impl Cop for DoubleStartEndWith {
    fn name(&self) -> &'static str {
        "Performance/DoubleStartEndWith"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, OR_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let include_as_aliases = config.get_bool("IncludeActiveSupportAliases", false);

        let or_node = match node.as_or_node() {
            Some(n) => n,
            None => return,
        };

        let left_call = match or_node.left().as_call_node() {
            Some(c) => c,
            None => return,
        };

        let right_call = match or_node.right().as_call_node() {
            Some(c) => c,
            None => return,
        };

        let left_name = left_call.name().as_slice();
        let right_name = right_call.name().as_slice();

        // Both sides must use the same method: start_with? or end_with?
        if left_name != right_name {
            return;
        }

        let is_target = left_name == b"start_with?" || left_name == b"end_with?"
            || (include_as_aliases
                && (left_name == b"starts_with?" || left_name == b"ends_with?"));
        if !is_target {
            return;
        }

        let method_display = if left_name == b"start_with?" {
            "start_with?"
        } else {
            "end_with?"
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, format!(
            "Use `{method_display}` with multiple arguments instead of chaining `||`."
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleStartEndWith, "cops/performance/double_start_end_with");
}
