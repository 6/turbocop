use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct BlockGivenWithExplicitBlock;

impl Cop for BlockGivenWithExplicitBlock {
    fn name(&self) -> &'static str {
        "Performance/BlockGivenWithExplicitBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Check if method has an explicit &block parameter
        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        if params.block().is_none() {
            return Vec::new();
        }

        // Walk the body looking for `block_given?` calls
        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut finder = BlockGivenFinder {
            offsets: Vec::new(),
        };
        finder.visit(&body);

        let mut diagnostics = Vec::new();
        for offset in finder.offsets {
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: self.default_severity(),
                cop_name: self.name().to_string(),
                message: "Check `block` instead of using `block_given?` with explicit `&block` parameter.".to_string(),
            });
        }

        diagnostics
    }
}

struct BlockGivenFinder {
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for BlockGivenFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"block_given?"
            && node.receiver().is_none()
            && node.arguments().is_none()
        {
            self.offsets.push(node.location().start_offset());
        }
        // Don't recurse into nested def nodes
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't recurse into nested method definitions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &BlockGivenWithExplicitBlock,
            include_bytes!(
                "../../../testdata/cops/performance/block_given_with_explicit_block/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &BlockGivenWithExplicitBlock,
            include_bytes!(
                "../../../testdata/cops/performance/block_given_with_explicit_block/no_offense.rb"
            ),
        );
    }
}
