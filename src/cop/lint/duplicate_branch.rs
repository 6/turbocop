use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CASE_NODE, ELSE_NODE, IF_NODE, WHEN_NODE};

pub struct DuplicateBranch;

impl Cop for DuplicateBranch {
    fn name(&self) -> &'static str {
        "Lint/DuplicateBranch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_NODE, ELSE_NODE, IF_NODE, WHEN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let _ignore_literal = config.get_bool("IgnoreLiteralBranches", false);
        let _ignore_constant = config.get_bool("IgnoreConstantBranches", false);
        let _ignore_dup_else = config.get_bool("IgnoreDuplicateElseBranch", false);

        // Check if/elsif/else chains
        if let Some(if_node) = node.as_if_node() {
            diagnostics.extend(check_if_branches(self, source, &if_node));
            return;
        }

        // Check case/when statements
        if let Some(case_node) = node.as_case_node() {
            diagnostics.extend(check_case_branches(self, source, &case_node));
            return;
        }

    }
}

/// Extract a comparison key for branch body.
/// For heredocs, Prism's `location()` on the node only covers the opening
/// delimiter (`<<~RUBY`), not the heredoc content/closing. We use a
/// `MaxExtentFinder` visitor to discover the true end offset including
/// heredoc closing locations, then slice the full source range.
fn stmts_source(source: &SourceFile, stmts: &Option<ruby_prism::StatementsNode<'_>>) -> Vec<u8> {
    match stmts {
        Some(s) => {
            let loc = s.location();
            let start = loc.start_offset();
            let mut end = loc.end_offset();

            // Walk descendants to find heredoc closings that extend beyond loc
            let mut finder = MaxExtentFinder { max_end: end };
            finder.visit(&s.as_node());
            end = finder.max_end;

            let bytes = source.as_bytes();
            if end <= bytes.len() {
                bytes[start..end].to_vec()
            } else {
                loc.as_slice().to_vec()
            }
        }
        None => Vec::new(),
    }
}

/// Visitor that finds the maximum source offset among all descendant nodes,
/// including heredoc closing locations which extend beyond the parent node's range.
struct MaxExtentFinder {
    max_end: usize,
}

impl<'pr> Visit<'pr> for MaxExtentFinder {
    fn visit_interpolated_string_node(
        &mut self,
        node: &ruby_prism::InterpolatedStringNode<'pr>,
    ) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let Some(close) = node.closing_loc() {
            let end = close.end_offset();
            if end > self.max_end {
                self.max_end = end;
            }
        }
        ruby_prism::visit_string_node(self, node);
    }
}

fn check_if_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    if_node: &ruby_prism::IfNode<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = HashSet::new();

    // Collect branches as (body_source, location_for_reporting)
    let mut branches: Vec<(Vec<u8>, ruby_prism::Location<'_>)> = Vec::new();

    // The if branch
    let if_body = stmts_source(source, &if_node.statements());
    branches.push((if_body, if_node.location()));

    // Walk elsif/else chain
    let mut subsequent = if_node.subsequent();
    while let Some(sub) = subsequent {
        if let Some(elsif) = sub.as_if_node() {
            let body = stmts_source(source, &elsif.statements());
            branches.push((body, elsif.location()));
            subsequent = elsif.subsequent();
        } else if let Some(else_node) = sub.as_else_node() {
            let body = stmts_source(source, &else_node.statements());
            branches.push((body, else_node.location()));
            break;
        } else {
            break;
        }
    }

    if branches.len() < 2 {
        return diagnostics;
    }

    for (body, loc) in &branches {
        if !body.is_empty() && !seen.insert(body.clone()) {
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Duplicate branch body detected.".to_string(),
            ));
        }
    }

    diagnostics
}

fn check_case_branches(
    cop: &DuplicateBranch,
    source: &SourceFile,
    case_node: &ruby_prism::CaseNode<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = HashSet::new();

    for when_ref in case_node.conditions().iter() {
        if let Some(when_node) = when_ref.as_when_node() {
            let body = stmts_source(source, &when_node.statements());
            if !body.is_empty() && !seen.insert(body) {
                let loc = when_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Duplicate branch body detected.".to_string(),
                ));
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateBranch, "cops/lint/duplicate_branch");
}
