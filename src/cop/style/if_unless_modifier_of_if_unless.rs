use crate::cop::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IfUnlessModifierOfIfUnless;

impl Cop for IfUnlessModifierOfIfUnless {
    fn name(&self) -> &'static str {
        "Style/IfUnlessModifierOfIfUnless"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Check modifier `if`
        if let Some(if_node) = node.as_if_node() {
            // Must be modifier form (no end keyword)
            if if_node.end_keyword_loc().is_some() {
                return;
            }

            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return, // ternary
            };

            let kw_bytes = kw_loc.as_slice();
            if kw_bytes != b"if" {
                return;
            }

            // Check if the body is a conditional
            if let Some(stmts) = if_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() == 1 && is_conditional(&body[0]) {
                    let keyword = "if";
                    let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Avoid modifier `{}` after another conditional.", keyword),
                    ));
                }
            }
        }

        // Check modifier `unless`
        if let Some(unless_node) = node.as_unless_node() {
            // Must be modifier form (no end keyword)
            if unless_node.end_keyword_loc().is_some() {
                return;
            }

            let kw_loc = unless_node.keyword_loc();

            if kw_loc.as_slice() != b"unless" {
                return;
            }

            if let Some(stmts) = unless_node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() == 1 && is_conditional(&body[0]) {
                    let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid modifier `unless` after another conditional.".to_string(),
                    ));
                }
            }
        }
    }
}

fn is_conditional(node: &ruby_prism::Node<'_>) -> bool {
    node.as_if_node().is_some() || node.as_unless_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IfUnlessModifierOfIfUnless,
        "cops/style/if_unless_modifier_of_if_unless"
    );
}
