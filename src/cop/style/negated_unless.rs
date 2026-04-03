use crate::cop::shared::node_type::{CALL_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NegatedUnless;

impl Cop for NegatedUnless {
    fn name(&self) -> &'static str {
        "Style/NegatedUnless"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, UNLESS_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
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
        // Skip `!!expr` (double-bang truthiness cast) — not a simple negation
        let predicate = unless_node.predicate();
        if let Some(call) = predicate.as_call_node() {
            if call.name().as_slice() == b"!" {
                // If the receiver of `!` is itself another `!` call, this is `!!expr`
                if let Some(recv) = call.receiver() {
                    if let Some(inner_call) = recv.as_call_node() {
                        if inner_call.name().as_slice() == b"!" {
                            return;
                        }
                    }
                }
                let kw_loc = unless_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Favor `if` over `unless` for negative conditions.".to_string(),
                );
                // Autocorrect: replace `unless` with `if`, remove `!` from condition
                if let Some(ref mut corr) = corrections {
                    // 1. Replace `unless` with `if`
                    corr.push(crate::correction::Correction {
                        start: kw_loc.start_offset(),
                        end: kw_loc.end_offset(),
                        replacement: "if".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    // 2. Replace negated predicate with inner expression
                    if let Some(inner) = call.receiver() {
                        let inner_src = std::str::from_utf8(inner.location().as_slice())
                            .unwrap_or("")
                            .to_string();
                        corr.push(crate::correction::Correction {
                            start: predicate.location().start_offset(),
                            end: predicate.location().end_offset(),
                            replacement: inner_src,
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                    }
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegatedUnless, "cops/style/negated_unless");
    crate::cop_autocorrect_fixture_tests!(NegatedUnless, "cops/style/negated_unless");
}
