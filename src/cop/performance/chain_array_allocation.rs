use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ChainArrayAllocation;

const INNER_METHODS: &[&[u8]] = &[b"compact", b"flatten", b"sort", b"uniq"];
const OUTER_METHODS: &[&[u8]] = &[b"map", b"flat_map"];

impl Cop for ChainArrayAllocation {
    fn name(&self) -> &'static str {
        "Performance/ChainArrayAllocation"
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if !INNER_METHODS.contains(&chain.inner_method) {
            return Vec::new();
        }

        if !OUTER_METHODS.contains(&chain.outer_method) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Avoid chaining array methods that allocate intermediate arrays.".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &ChainArrayAllocation,
            include_bytes!(
                "../../../testdata/cops/performance/chain_array_allocation/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ChainArrayAllocation,
            include_bytes!(
                "../../../testdata/cops/performance/chain_array_allocation/no_offense.rb"
            ),
        );
    }
}
