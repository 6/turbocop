use crate::cop::node_type::WHEN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct WhenThen;

impl Cop for WhenThen {
    fn name(&self) -> &'static str {
        "Style/WhenThen"
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

        // If there's a then_keyword_loc that says "then", it's already fine
        if let Some(then_loc) = when_node.then_keyword_loc() {
            let text = then_loc.as_slice();
            if text == b"then" || text == b";" {
                // If it's "then", it's OK. If Prism reports ";", flag it.
                if text == b";" {
                    diagnostics.extend(self.flag_semicolon(
                        source,
                        &when_node,
                        then_loc.start_offset(),
                    ));
                }
                return;
            }
        }

        // Prism may not set then_keyword_loc for semicolons. Look at source
        // between the last condition and the first statement.
        let conditions: Vec<_> = when_node.conditions().into_iter().collect();
        if conditions.is_empty() {
            return;
        }

        let stmts = match when_node.statements() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.is_empty() {
            return;
        }

        let last_condition = &conditions[conditions.len() - 1];
        let last_cond_end =
            last_condition.location().start_offset() + last_condition.location().as_slice().len();
        let first_body_start = body_nodes[0].location().start_offset();

        // Check source bytes between end of conditions and start of body for a semicolon
        let src = source.as_bytes();
        let between = &src[last_cond_end..first_body_start];

        if let Some(semi_offset) = between.iter().position(|&b| b == b';') {
            let abs_offset = last_cond_end + semi_offset;
            diagnostics.extend(self.flag_semicolon(source, &when_node, abs_offset));
        }
    }
}

impl WhenThen {
    fn flag_semicolon(
        &self,
        source: &SourceFile,
        when_node: &ruby_prism::WhenNode<'_>,
        semi_offset: usize,
    ) -> Vec<Diagnostic> {
        let conditions: Vec<_> = when_node.conditions().into_iter().collect();
        let conditions_text: Vec<String> = conditions
            .iter()
            .map(|c| {
                let loc = c.location();
                String::from_utf8_lossy(loc.as_slice()).to_string()
            })
            .collect();
        let when_text = conditions_text.join(", ");

        let (line, column) = source.offset_to_line_col(semi_offset);
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Do not use `when {};`. Use `when {} then` instead.",
                when_text, when_text
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;
    crate::cop_fixture_tests!(WhenThen, "cops/style/when_then");

    #[test]
    fn inline_test_semicolon() {
        let source = b"case a\nwhen b; c\nend\n";
        let diags = run_cop_full(&WhenThen, source);
        assert_eq!(diags.len(), 1, "Should flag when b; c. Got: {:?}", diags);
    }
}
