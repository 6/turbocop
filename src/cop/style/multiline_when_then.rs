use crate::cop::node_type::WHEN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineWhenThen;

impl Cop for MultilineWhenThen {
    fn name(&self) -> &'static str {
        "Style/MultilineWhenThen"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[WHEN_NODE]
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
        let when_node = match node.as_when_node() {
            Some(w) => w,
            None => return,
        };

        // Check for `then` keyword
        let then_loc = match when_node.then_keyword_loc() {
            Some(loc) => loc,
            None => {
                // Also check if `then` appears on a separate line in the body
                // (Prism may parse `when bar\n  then do_something` differently)
                return;
            }
        };

        if then_loc.as_slice() != b"then" {
            return;
        }

        let then_line = source.offset_to_line_col(then_loc.start_offset()).0;

        // If the body starts on the same line as `then`, it's single-line style (allowed).
        // e.g., `when bar then do_something`
        if let Some(stmts) = when_node.statements() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if !body_nodes.is_empty() {
                let first_body_line = source
                    .offset_to_line_col(body_nodes[0].location().start_offset())
                    .0;
                if first_body_line == then_line {
                    // Check if all body nodes are on the same line as `then`
                    // If a later body node wraps to the next line, it could be:
                    // `when foo then do_something(arg1,\n arg2)` which is allowed
                    return;
                }
            }
        }

        let (line, column) = source.offset_to_line_col(then_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use `then` for multiline `when` statement.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineWhenThen, "cops/style/multiline_when_then");
}
