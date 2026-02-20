use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use crate::cop::node_type::{ELSE_NODE, IF_NODE};

pub struct IdenticalConditionalBranches;

/// Check if a node contains any heredoc string nodes.
fn contains_heredoc(node: &ruby_prism::Node<'_>) -> bool {
    struct HeredocChecker {
        found: bool,
    }
    impl<'pr> Visit<'pr> for HeredocChecker {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            // Simple heredocs without interpolation are StringNode with << opening
            if let Some(opening) = node.opening_loc() {
                if opening.as_slice().starts_with(b"<<") {
                    self.found = true;
                    return;
                }
            }
            ruby_prism::visit_string_node(self, node);
        }
        fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
            // Heredocs have opening_loc starting with "<<"
            if let Some(opening) = node.opening_loc() {
                if opening.as_slice().starts_with(b"<<") {
                    self.found = true;
                    return;
                }
            }
            ruby_prism::visit_interpolated_string_node(self, node);
        }
        fn visit_interpolated_x_string_node(&mut self, node: &ruby_prism::InterpolatedXStringNode<'pr>) {
            if node.opening_loc().as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
            ruby_prism::visit_interpolated_x_string_node(self, node);
        }
        fn visit_x_string_node(&mut self, node: &ruby_prism::XStringNode<'pr>) {
            if node.opening_loc().as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
            ruby_prism::visit_x_string_node(self, node);
        }
    }
    let mut checker = HeredocChecker { found: false };
    checker.visit(node);
    checker.found
}

impl IdenticalConditionalBranches {
    fn last_stmt_source(source: &SourceFile, stmts: &ruby_prism::StatementsNode<'_>) -> Option<(String, usize, usize, bool)> {
        let body: Vec<_> = stmts.body().iter().collect();
        if body.is_empty() {
            return None;
        }
        let last = &body[body.len() - 1];
        let loc = last.location();
        let src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        let has_heredoc = contains_heredoc(last);
        Some((String::from_utf8_lossy(src).trim().to_string(), line, col, has_heredoc))
    }
}

impl Cop for IdenticalConditionalBranches {
    fn name(&self) -> &'static str {
        "Style/IdenticalConditionalBranches"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ELSE_NODE, IF_NODE]
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
        if let Some(if_node) = node.as_if_node() {
            // Must have an if keyword (skip ternaries)
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            // Skip elsif nodes — RuboCop processes the full if/elsif/else chain
            // from the top-level if only
            if kw_loc.as_slice() == b"elsif" {
                return;
            }

            let if_stmts = match if_node.statements() {
                Some(s) => s,
                None => return,
            };

            let else_clause = match if_node.subsequent() {
                Some(e) => e,
                None => return,
            };

            // Must be a direct else, not an elsif
            let else_stmts = if let Some(ec) = else_clause.as_else_node() {
                match ec.statements() {
                    Some(s) => s,
                    None => return,
                }
            } else {
                return;
            };

            if let (Some((if_last, if_line, if_col, if_heredoc)), Some((else_last, _, _, else_heredoc))) = (
                Self::last_stmt_source(source, &if_stmts),
                Self::last_stmt_source(source, &else_stmts),
            ) {
                // Skip comparison when heredocs are involved — the node source
                // may not include the heredoc body, leading to false matches.
                if if_heredoc || else_heredoc {
                    return;
                }
                if if_last == else_last && !if_last.is_empty() {
                    diagnostics.push(self.diagnostic(
                        source,
                        if_line,
                        if_col,
                        format!("Move `{}` out of the conditional.", if_last),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IdenticalConditionalBranches, "cops/style/identical_conditional_branches");
}
