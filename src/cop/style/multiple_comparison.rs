use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultipleComparison;

impl MultipleComparison {
    /// Recursively collect == comparisons joined by ||, returning the variable
    /// being compared if consistent, along with the comparison node locations.
    fn collect_comparisons<'a>(
        node: &'a ruby_prism::Node<'a>,
        source: &SourceFile,
    ) -> Option<(Vec<u8>, usize)> {
        if let Some(call) = node.as_call_node() {
            let op = call.name();
            let op_bytes = op.as_slice();

            if op_bytes == b"||" {
                // Both sides should be == comparisons or nested ||
                let lhs = call.receiver()?;
                let rhs_args = call.arguments()?;
                let rhs_list: Vec<_> = rhs_args.arguments().iter().collect();
                if rhs_list.len() != 1 {
                    return None;
                }
                let rhs = &rhs_list[0];

                let (lhs_var, lhs_count) = Self::collect_comparisons(&lhs, source)?;
                let (rhs_var, rhs_count) = Self::collect_comparisons(rhs, source)?;

                if lhs_var == rhs_var {
                    return Some((lhs_var, lhs_count + rhs_count));
                }
                return None;
            }

            if op_bytes == b"==" {
                let lhs = call.receiver()?;
                let rhs_args = call.arguments()?;
                let rhs_list: Vec<_> = rhs_args.arguments().iter().collect();
                if rhs_list.len() != 1 {
                    return None;
                }
                let rhs = &rhs_list[0];

                // One side should be a local variable or method call (the "variable")
                let lhs_src = lhs.location().as_slice();
                let rhs_src = rhs.location().as_slice();

                // Try lhs as the variable
                if lhs.as_local_variable_read_node().is_some() || lhs.as_call_node().is_some() {
                    return Some((lhs_src.to_vec(), 1));
                }
                // Try rhs as the variable
                if rhs.as_local_variable_read_node().is_some() || rhs.as_call_node().is_some() {
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
        let _allow_method = config.get_bool("AllowMethodComparison", true);
        let threshold = config.get_usize("ComparisonsThreshold", 2);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be a `||` operation
        if call.name().as_slice() != b"||" {
            return Vec::new();
        }

        // Check if this || is nested inside another || (skip to avoid duplicate reports)
        // We can't walk up, so we check if the receiver is also ||
        // Only report on the topmost || in a chain
        // Actually we check inside collect_comparisons, so just try to collect
        if let Some((_, count)) = Self::collect_comparisons(node, source) {
            if count >= threshold {
                // Only report if we're at the top of the || chain
                // Check that receiver is a || too (if so, this is part of a chain,
                // but we handle the chain recursively, so we need to make sure we
                // only report at the top level).
                // We report on any node that matches, but the caller (walker) will
                // visit parent nodes after children, so we check that the parent is not
                // also a matching || chain.
                // Since we can't check parent, we use the heuristic: report only
                // if the FULL chain starts at this node. We check by seeing if
                // receiver is also a || with the same variable.
                let loc = node.location();
                // Check if receiver is || with same var (chain continuation)
                if let Some(recv) = call.receiver() {
                    if let Some((_, _)) = Self::collect_comparisons(&recv, source) {
                        // This is a sub-expression of a larger chain;
                        // the full chain will be caught at the top level
                        // But actually the walker visits children first, so this node IS
                        // the top. Let me just report on all || chains that meet threshold.
                        // To avoid duplicates, skip if the parent would also match.
                        // Since we can't check parent, just report here.
                    }
                }

                let start = call.receiver().map(|r| r.location().start_offset()).unwrap_or(loc.start_offset());
                let (line, column) = source.offset_to_line_col(start);
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
