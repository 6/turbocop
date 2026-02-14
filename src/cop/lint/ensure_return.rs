use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EnsureReturn;

struct ReturnFinder {
    found: Vec<usize>,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if node.as_return_node().is_some() {
            self.found.push(node.location().start_offset());
        }
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if node.as_return_node().is_some() {
            self.found.push(node.location().start_offset());
        }
    }
}

impl Cop for EnsureReturn {
    fn name(&self) -> &'static str {
        "Lint/EnsureReturn"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // EnsureNode is visited via visit_begin_node's specific method,
        // not via the generic visit() dispatch. So we match BeginNode
        // and check its ensure_clause.
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let ensure_node = match begin_node.ensure_clause() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let statements = match ensure_node.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut finder = ReturnFinder { found: vec![] };
        for stmt in statements.body().iter() {
            finder.visit(&stmt);
        }

        finder
            .found
            .iter()
            .map(|&offset| {
                let (line, column) = source.offset_to_line_col(offset);
                Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: self.default_severity(),
                    cop_name: self.name().to_string(),
                    message: "Do not return from an `ensure` block.".to_string(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &EnsureReturn,
            include_bytes!("../../../testdata/cops/lint/ensure_return/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EnsureReturn,
            include_bytes!("../../../testdata/cops/lint/ensure_return/no_offense.rb"),
        );
    }
}
