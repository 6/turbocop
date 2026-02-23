use crate::cop::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SoleNestedConditional;

impl Cop for SoleNestedConditional {
    fn name(&self) -> &'static str {
        "Style/SoleNestedConditional"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
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
        let allow_modifier = config.get_bool("AllowModifier", false);

        // Check if this is an if/unless without else
        let (kw_loc, statements, has_else) = if let Some(if_node) = node.as_if_node() {
            let kw = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return, // ternary
            };
            if kw.as_slice() == b"elsif" {
                return;
            }
            (kw, if_node.statements(), if_node.subsequent().is_some())
        } else if let Some(unless_node) = node.as_unless_node() {
            (
                unless_node.keyword_loc(),
                unless_node.statements(),
                unless_node.else_clause().is_some(),
            )
        } else {
            return;
        };

        if has_else {
            return;
        }

        let stmts = match statements {
            Some(s) => s,
            None => return,
        };

        let body: Vec<_> = stmts.body().iter().collect();
        if body.len() != 1 {
            return;
        }

        // Check if the sole statement is another if/unless without else
        let is_nested_if = if let Some(inner_if) = body[0].as_if_node() {
            let inner_kw = match inner_if.if_keyword_loc() {
                Some(loc) => loc,
                None => return, // ternary
            };

            if allow_modifier {
                // Skip if inner is modifier form
                if inner_if.end_keyword_loc().is_none() {
                    return;
                }
            }

            // Inner if must not have else
            if inner_if.subsequent().is_some() {
                return;
            }

            inner_kw.as_slice() == b"if"
        } else if let Some(inner_unless) = body[0].as_unless_node() {
            if allow_modifier && inner_unless.end_keyword_loc().is_none() {
                return;
            }

            if inner_unless.else_clause().is_some() {
                return;
            }

            true
        } else {
            false
        };

        if !is_nested_if {
            return;
        }

        // RuboCop reports the offense on the inner conditional's keyword, not the outer
        let inner_kw_loc = if let Some(inner_if) = body[0].as_if_node() {
            inner_if.if_keyword_loc().unwrap_or(kw_loc)
        } else if let Some(inner_unless) = body[0].as_unless_node() {
            inner_unless.keyword_loc()
        } else {
            kw_loc
        };

        let (line, column) = source.offset_to_line_col(inner_kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Consider merging nested conditions into outer `if` conditions.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SoleNestedConditional, "cops/style/sole_nested_conditional");
}
