use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{UNTIL_NODE, WHILE_NODE};

pub struct Loop;

impl Cop for Loop {
    fn name(&self) -> &'static str {
        "Lint/Loop"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[UNTIL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check WhileNode for begin..end while form
        // Prism sets the PM_LOOP_FLAGS_BEGIN_MODIFIER flag for this pattern.
        if let Some(while_node) = node.as_while_node() {
            if while_node.is_begin_modifier() {
                let kw_loc = while_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Kernel#loop` with `break` rather than `begin/end/while(until)`.".to_string(),
                ));
            }
        }

        // Check UntilNode for begin..end until form
        if let Some(until_node) = node.as_until_node() {
            if until_node.is_begin_modifier() {
                let kw_loc = until_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Kernel#loop` with `break` rather than `begin/end/while(until)`.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Loop, "cops/lint/loop_cop");
}
