use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, UNLESS_NODE};

pub struct NegatedUnless;

impl Cop for NegatedUnless {
    fn name(&self) -> &'static str {
        "Style/NegatedUnless"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "both");
        let unless_node = match node.as_unless_node() {
            Some(n) => n,
            None => return,
        };

        // Must not have an else clause
        if unless_node.else_clause().is_some() {
            return;
        }

        // Detect modifier (postfix) form: no end keyword
        let is_modifier = unless_node.end_keyword_loc().is_none();

        match enforced_style {
            "prefix" if is_modifier => return,
            "postfix" if !is_modifier => return,
            _ => {} // "both" checks all forms
        }

        // Check if predicate is a `!` call (negation)
        let predicate = unless_node.predicate();
        if let Some(call) = predicate.as_call_node() {
            if call.name().as_slice() == b"!" {
                let kw_loc = unless_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Favor `if` over `unless` for negative conditions.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegatedUnless, "cops/style/negated_unless");
}
