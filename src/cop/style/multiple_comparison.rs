use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, LOCAL_VARIABLE_READ_NODE, OR_NODE};

pub struct MultipleComparison;

impl MultipleComparison {
    /// Recursively collect == comparisons joined by ||, returning the variable
    /// being compared if consistent, along with the comparison count.
    /// Handles OrNode (||) and CallNode (==).
    /// When AllowMethodComparison is true, comparisons where the value is a
    /// method call are skipped (returning count 0) but don't break the chain.
    fn collect_comparisons<'a>(
        node: &'a ruby_prism::Node<'a>,
        allow_method: bool,
    ) -> Option<(Vec<u8>, usize)> {
        // Handle OrNode: a == x || a == y
        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            let rhs = or_node.right();

            let lhs_result = Self::collect_comparisons(&lhs, allow_method);
            let rhs_result = Self::collect_comparisons(&rhs, allow_method);

            match (lhs_result, rhs_result) {
                (Some((lhs_var, lhs_count)), Some((rhs_var, rhs_count))) => {
                    if lhs_var == rhs_var {
                        return Some((lhs_var, lhs_count + rhs_count));
                    }
                    // Different variables but might share if one is empty (skipped method comparison)
                    if lhs_count == 0 {
                        return Some((rhs_var, rhs_count));
                    }
                    if rhs_count == 0 {
                        return Some((lhs_var, lhs_count));
                    }
                    return None;
                }
                (Some(result), None) | (None, Some(result)) => {
                    // One side might be a non-comparison node; that's still OK
                    // only if the other side is all comparisons
                    return None;
                }
                (None, None) => return None,
            }
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

                // Determine which side is the variable (lvar or method call)
                // and which is the value.
                // RuboCop treats both local variables and method calls as the
                // "variable" side of the comparison. The AllowMethodComparison
                // config controls whether comparisons where the VALUE is a
                // method call are counted.
                let (var_src, value_is_call) =
                    if lhs.as_local_variable_read_node().is_some() {
                        (lhs_src, rhs.as_call_node().is_some())
                    } else if rhs.as_local_variable_read_node().is_some() {
                        (rhs_src, lhs.as_call_node().is_some())
                    } else if lhs.as_call_node().is_some() && rhs.as_call_node().is_none() {
                        // lhs is a method call (the "variable"), rhs is a literal value
                        (lhs_src, false)
                    } else if rhs.as_call_node().is_some() && lhs.as_call_node().is_none() {
                        // rhs is a method call (the "variable"), lhs is a literal value
                        (rhs_src, false)
                    } else if lhs.as_call_node().is_some() && rhs.as_call_node().is_some() {
                        // Both sides are method calls — lhs is the variable, rhs is the value
                        if allow_method {
                            (lhs_src, true)
                        } else {
                            (lhs_src, true)
                        }
                    } else {
                        return None;
                    };

                // When AllowMethodComparison is true and the value is a method call,
                // skip this comparison (count = 0) but still return the variable
                // so the chain continues.
                if allow_method && value_is_call {
                    return Some((var_src.to_vec(), 0));
                }

                return Some((var_src.to_vec(), 1));
            }
        }
        None
    }
}

impl Cop for MultipleComparison {
    fn name(&self) -> &'static str {
        "Style/MultipleComparison"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LOCAL_VARIABLE_READ_NODE, OR_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_method = config.get_bool("AllowMethodComparison", true);
        let threshold = config.get_usize("ComparisonsThreshold", 2);

        // Must be an OrNode (||) — in Prism, `||` is OrNode, not CallNode
        let or_node = match node.as_or_node() {
            Some(n) => n,
            None => return,
        };

        if let Some((_, count)) = Self::collect_comparisons(node, allow_method) {
            if count >= threshold {
                // Deduplicate: if the left child is also a matching || chain that meets
                // the threshold, skip this node (the innermost matching chain reports).
                let left = or_node.left();
                if left.as_or_node().is_some() {
                    if let Some((_, inner_count)) = Self::collect_comparisons(&left, allow_method) {
                        if inner_count >= threshold {
                            return;
                        }
                    }
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleComparison, "cops/style/multiple_comparison");
}
