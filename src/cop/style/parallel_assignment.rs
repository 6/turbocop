use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, MULTI_WRITE_NODE, SPLAT_NODE};

pub struct ParallelAssignment;

impl Cop for ParallelAssignment {
    fn name(&self) -> &'static str {
        "Style/ParallelAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, MULTI_WRITE_NODE, SPLAT_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Look for multi-write nodes (parallel assignment: a, b = 1, 2)
        let multi_write = match node.as_multi_write_node() {
            Some(m) => m,
            None => return,
        };

        let targets: Vec<_> = multi_write.lefts().iter().collect();

        // Check if there are at least 2 targets
        if targets.len() < 2 {
            return;
        }

        // Skip if rest assignment is present (a, *b = ...)
        if multi_write.rest().is_some() {
            return;
        }

        // The value is the RHS. In Prism, for `a, b = 1, 2`, the value is an ArrayNode
        // with the implicit array of values. For `a, b = foo`, it's just a single node.
        let value = multi_write.value();

        // Check if RHS is an array node (implicit or explicit) with matching count
        if let Some(arr) = value.as_array_node() {
            let elements: Vec<_> = arr.elements().iter().collect();
            if elements.len() == targets.len() {
                // Check no splat in elements
                if elements.iter().any(|e| e.as_splat_node().is_some()) {
                    return;
                }

                let loc = multi_write.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use parallel assignment.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ParallelAssignment, "cops/style/parallel_assignment");
}
