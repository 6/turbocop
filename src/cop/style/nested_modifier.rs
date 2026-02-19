use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{IF_NODE, UNLESS_NODE};

pub struct NestedModifier;

impl Cop for NestedModifier {
    fn name(&self) -> &'static str {
        "Style/NestedModifier"
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
    ) {
        // Get the body of a modifier conditional (if/unless)
        let body_node = if let Some(if_node) = node.as_if_node() {
            // Must be modifier form (no end keyword, has if keyword, not ternary)
            if if_node.end_keyword_loc().is_some() {
                return;
            }
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return, // ternary
            };
            let kw_bytes = kw_loc.as_slice();
            if kw_bytes != b"if" && kw_bytes != b"unless" {
                return;
            }
            if_node.statements()
        } else if let Some(unless_node) = node.as_unless_node() {
            if unless_node.end_keyword_loc().is_some() {
                return;
            }
            unless_node.statements()
        } else {
            return;
        };

        let stmts = match body_node {
            Some(s) => s,
            None => return,
        };

        let body: Vec<_> = stmts.body().iter().collect();
        if body.len() != 1 {
            return;
        }

        // Check if the body is another modifier if/unless
        if let Some(inner_if) = body[0].as_if_node() {
            if inner_if.end_keyword_loc().is_some() {
                return;
            }
            if let Some(inner_kw) = inner_if.if_keyword_loc() {
                let inner_bytes = inner_kw.as_slice();
                if inner_bytes == b"if" || inner_bytes == b"unless" {
                    let (line, column) = source.offset_to_line_col(inner_kw.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid using nested modifiers.".to_string(),
                    ));
                }
            }
        }

        if let Some(inner_unless) = body[0].as_unless_node() {
            if inner_unless.end_keyword_loc().is_some() {
                return;
            }
            let inner_kw = inner_unless.keyword_loc();
            if inner_kw.as_slice() == b"unless" {
                let (line, column) = source.offset_to_line_col(inner_kw.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid using nested modifiers.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedModifier, "cops/style/nested_modifier");
}
