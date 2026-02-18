use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultipleComparison;

impl MultipleComparison {
    /// Recursively collect == comparisons joined by ||, returning the variable
    /// being compared if consistent, along with the comparison count.
    /// Handles OrNode (||) and CallNode (==).
    fn collect_comparisons<'a>(
        node: &'a ruby_prism::Node<'a>,
        allow_method: bool,
    ) -> Option<(Vec<u8>, usize)> {
        // Handle OrNode: a == x || a == y
        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            let rhs = or_node.right();

            let (lhs_var, lhs_count) = Self::collect_comparisons(&lhs, allow_method)?;
            let (rhs_var, rhs_count) = Self::collect_comparisons(&rhs, allow_method)?;

            if lhs_var == rhs_var {
                return Some((lhs_var, lhs_count + rhs_count));
            }
            return None;
        }

        // Handle CallNode with ==
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"==" {
                let lhs = call.receiver()?;
                let rhs_args = call.arguments()?;
                let rhs_list: Vec<_> = rhs_args.arguments().iter().collect();
                if rhs_list.len() != 1 {
                    return None;
                }
                let rhs = &rhs_list[0];

                let lhs_src = lhs.location().as_slice();
                let rhs_src = rhs.location().as_slice();

                // Try lhs as the variable, rhs as the value
                if lhs.as_local_variable_read_node().is_some()
                    || (!allow_method && lhs.as_call_node().is_some())
                {
                    // When AllowMethodComparison is true, skip if the value
                    // (other side) is a method call
                    if allow_method && rhs.as_call_node().is_some() {
                        return None;
                    }
                    return Some((lhs_src.to_vec(), 1));
                }
                // Try rhs as the variable, lhs as the value
                if rhs.as_local_variable_read_node().is_some()
                    || (!allow_method && rhs.as_call_node().is_some())
                {
                    if allow_method && lhs.as_call_node().is_some() {
                        return None;
                    }
                    return Some((rhs_src.to_vec(), 1));
                }
            }
        }
        None
    }
}

impl Cop for MultipleComparison {
    fn name(&self) -> &'static str {
        "Style/MultipleComparison"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_method = config.get_bool("AllowMethodComparison", true);
        let threshold = config.get_usize("ComparisonsThreshold", 2);

        // Must be an OrNode (||) — in Prism, `||` is OrNode, not CallNode
        let or_node = match node.as_or_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        if let Some((_, count)) = Self::collect_comparisons(node, allow_method) {
            if count >= threshold {
                // Deduplicate: if the left child is also a matching || chain that meets
                // the threshold, skip this node (the innermost matching chain reports).
                let left = or_node.left();
                if left.as_or_node().is_some() {
                    if let Some((_, inner_count)) = Self::collect_comparisons(&left, allow_method) {
                        if inner_count >= threshold {
                            return Vec::new();
                        }
                    }
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleComparison, "cops/style/multiple_comparison");
}
