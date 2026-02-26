use crate::cop::node_type::CASE_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CaseWhenSplat;

/// Returns true if a condition is considered "non-splat" for offense detection purposes.
/// A condition is non-splat if it is not a splat node at all, or if it is a splat on
/// an array literal (e.g., `*[1, 2]`). Array literal splats are treated as non-splat
/// because RuboCop suggests inlining them instead of reordering.
fn is_non_splat(condition: &ruby_prism::Node<'_>) -> bool {
    match condition.as_splat_node() {
        None => true, // Not a splat at all
        Some(splat) => {
            // Splat on an array literal (e.g., *[1, 2]) is treated as non-splat
            if let Some(expr) = splat.expression() {
                expr.as_array_node().is_some()
            } else {
                false
            }
        }
    }
}

impl Cop for CaseWhenSplat {
    fn name(&self) -> &'static str {
        "Performance/CaseWhenSplat"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_NODE]
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
        let case_node = match node.as_case_node() {
            Some(n) => n,
            None => return,
        };

        // Collect all conditions across all when branches, paired with their
        // parent when node's start offset (used for deduplication).
        let when_branches: Vec<_> = case_node.conditions().iter().collect();
        let mut condition_entries: Vec<(ruby_prism::Node<'_>, usize)> = Vec::new();

        for when_node_ref in &when_branches {
            let when_node = match when_node_ref.as_when_node() {
                Some(w) => w,
                None => continue,
            };
            let when_start = when_node.location().start_offset();
            for condition in when_node.conditions().iter() {
                condition_entries.push((condition, when_start));
            }
        }

        // Find offending splat conditions by iterating in reverse.
        // A splat on a variable/constant is an offense only if a "non-splat"
        // condition (including array-literal splats) appears after it.
        let mut found_non_splat = false;
        let mut offending_when_offsets: Vec<usize> = Vec::new();

        for (condition, when_start) in condition_entries.iter().rev() {
            if is_non_splat(condition) {
                found_non_splat = true;
                continue;
            }
            // This is a splat on a variable/constant
            if found_non_splat {
                offending_when_offsets.push(*when_start);
            }
        }

        // Deduplicate: report one diagnostic per when node
        offending_when_offsets.sort_unstable();
        offending_when_offsets.dedup();

        for when_start in offending_when_offsets {
            let (line, column) = source.offset_to_line_col(when_start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Reorder `when` conditions with a splat to the end.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseWhenSplat, "cops/performance/case_when_splat");
}
