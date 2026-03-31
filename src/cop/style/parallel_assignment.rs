use crate::cop::node_type::{ARRAY_NODE, MULTI_WRITE_NODE, SPLAT_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ParallelAssignment flags `a, b = 1, 2` style multi-assignment when
/// each target gets exactly one value and the assignment can be safely rewritten
/// as sequential assignments.
///
/// ## Swap/cycle exemption (FP+FN fix, ~1300 FNs + ~8 FPs resolved)
/// The old implementation treated ANY variable overlap between LHS and RHS as a
/// "swap" and skipped it. This was too broad: `old, @var = @var, true` has an
/// overlap but no cycle — it can safely be rewritten sequentially.
///
/// We now use proper cycle detection (Kahn's algorithm / topological sort),
/// matching RuboCop's `AssignmentSorter`. Dependencies are detected via
/// word-boundary substring matching on source text, which handles:
///   - Simple swaps: `a, b = b, a`
///   - Expression cycles: `x, y = y, x+y` (Fibonacci pattern)
///   - Indexed/method swaps: `arr[0], arr[1] = arr[1], arr[0]`
///   - Nested-call cycles: `a.x, a.y = f(a.y), f(a.x)`
///   - Rotations: `a, b, c = b, c, a`
///
/// One-directional dependencies (no cycle) are correctly flagged:
///   - `old, @var = @var, true` → `old = @var; @var = true`
///   - `state, opts = opts, nil` → sequential rewrite is safe
///
/// ## Nested group / flattened target count (FP fix)
/// RuboCop's `node.assignments` flattens nested groups: `(a, b), c` has 3
/// assignments, not 2. We replicate this by counting leaf targets recursively,
/// so `(a, b), c = [1, 2], 3` (3 targets, 2 RHS) is correctly skipped.
///
/// ## Trailing-comma / ImplicitRestNode
/// `@name, @config, @bulk, = name, config, bulk` has a trailing comma that
/// Prism represents as `ImplicitRestNode` in the `rest()` slot. We only
/// skip when `rest()` is a real `SplatNode` (i.e., `*var`).
///
/// ## Remaining limitations
/// - `self.a, self.b = b, a`: RuboCop's `add_self_to_getters` converts bare
///   getters to `self.x` for dependency analysis. We don't do this, so these
///   rare implicit-self swaps may be false positives.
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
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

        // Skip if a real splat rest assignment is present (a, *b = ...)
        // but NOT for ImplicitRestNode (trailing comma: a, b, = ...)
        if let Some(rest) = multi_write.rest() {
            if rest.as_implicit_rest_node().is_none() {
                // It's a real SplatNode or other rest — skip
                return;
            }
        }

        // The value is the RHS. In Prism, for `a, b = 1, 2`, the value is an ArrayNode
        // with the implicit array of values. For `a, b = foo`, it's just a single node.
        let value = multi_write.value();

        // Check if RHS is an array node (implicit or explicit) with matching count.
        // RuboCop flattens nested groups in the LHS count (e.g., `(a, b), c` counts
        // as 3 targets, not 2). We replicate this: if the LHS has nested
        // MultiTargetNodes, use the flattened leaf count for the size comparison.
        if let Some(arr) = value.as_array_node() {
            let elements: Vec<_> = arr.elements().iter().collect();
            let flat_target_count = count_flat_targets(&targets);
            if elements.len() != flat_target_count {
                return;
            }

            // Check no splat in elements
            if elements.iter().any(|e| e.as_splat_node().is_some()) {
                return;
            }

            // Check for swap pattern: if any RHS element references any LHS target,
            // the assignment has order-dependent semantics and should be allowed.
            if is_swap_assignment(source, &targets, &elements) {
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

/// Check if the parallel assignment has a cyclic dependency that prevents safe
/// rewriting as sequential assignments (i.e., it is a swap or rotation).
///
/// RuboCop uses topological sort (Kahn's algorithm) with cycle detection on the
/// AST. We replicate the cycle-detection approach using source-text analysis:
/// build a dependency graph where edge i→j means "RHS[i] references LHS[j]",
/// then check for cycles with Kahn's algorithm.
///
/// Dependency detection uses word-boundary substring matching: LHS target text
/// must appear in the RHS element text as a whole token (not inside a larger
/// identifier). This handles both exact swaps (`a, b = b, a`) and expression
/// cycles (`x, y = y, x+y`).
///
/// Simple overlaps WITHOUT cycles (e.g., `x, @y = @y, true`) are NOT swaps —
/// they can be rewritten as `x = @y; @y = true` and must be flagged.
fn is_swap_assignment(
    source: &SourceFile,
    targets: &[ruby_prism::Node<'_>],
    rhs_elements: &[ruby_prism::Node<'_>],
) -> bool {
    let n = targets.len();

    // Extract source text for each LHS target
    let lhs_texts: Vec<&str> = targets
        .iter()
        .filter_map(|t| {
            let loc = t.location();
            source.try_byte_slice(loc.start_offset(), loc.end_offset())
        })
        .collect();

    // Extract source text for each RHS element
    let rhs_texts: Vec<&str> = rhs_elements
        .iter()
        .filter_map(|e| {
            let loc = e.location();
            source.try_byte_slice(loc.start_offset(), loc.end_offset())
        })
        .collect();

    if lhs_texts.len() != n || rhs_texts.len() != n {
        return false;
    }

    // Build dependency graph: edge i→j means RHS[i] references LHS[j]
    // (assignment i reads the old value of target j, so i must execute before j).
    // Skip self-references (i == j) — `a, b = a, b` has no real dependency.
    let mut in_degree = vec![0u32; n];
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];

    for i in 0..n {
        for j in 0..n {
            if i != j
                && !lhs_texts[j].is_empty()
                && has_variable_reference(rhs_texts[i], lhs_texts[j])
            {
                adj[i].push(j);
                in_degree[j] += 1;
            }
        }
    }

    // Kahn's algorithm: if topological sort cannot process all nodes, there is a cycle.
    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut sorted_count = 0usize;

    while let Some(node) = queue.pop() {
        sorted_count += 1;
        for &neighbor in &adj[node] {
            in_degree[neighbor] -= 1;
            if in_degree[neighbor] == 0 {
                queue.push(neighbor);
            }
        }
    }

    // Cycle exists → swap/rotation → allow the assignment
    sorted_count < n
}

/// Check if `rhs_text` contains a reference to `target` using word-boundary
/// matching. Returns true if `target` appears in `rhs_text` as a whole token
/// (not inside a larger identifier).
///
/// For sigiled variables (`@foo`), also ensures the match isn't part of a
/// class variable (`@@foo`).
fn has_variable_reference(rhs_text: &str, target: &str) -> bool {
    if rhs_text == target {
        return true;
    }

    let rhs_bytes = rhs_text.as_bytes();
    let target_bytes = target.as_bytes();

    if target_bytes.is_empty() || target_bytes.len() > rhs_bytes.len() {
        return false;
    }

    let mut start = 0;
    while start + target_bytes.len() <= rhs_bytes.len() {
        match rhs_text[start..].find(target) {
            Some(rel_pos) => {
                let abs_pos = start + rel_pos;
                let end_pos = abs_pos + target_bytes.len();

                // Word boundary before: start of string or non-ident char
                let before_ok = abs_pos == 0 || !is_ruby_ident_char(rhs_bytes[abs_pos - 1]);

                // For @-prefixed targets, also reject if preceded by @ (would be @@var)
                let sigil_ok =
                    !(target_bytes[0] == b'@' && abs_pos > 0 && rhs_bytes[abs_pos - 1] == b'@');

                // Word boundary after: end of string or non-ident char
                let after_ok =
                    end_pos >= rhs_bytes.len() || !is_ruby_ident_char(rhs_bytes[end_pos]);

                if before_ok && sigil_ok && after_ok {
                    return true;
                }

                start = abs_pos + 1;
            }
            None => break,
        }
    }

    false
}

fn is_ruby_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Count the flattened number of leaf targets in the LHS, recursing into
/// nested `MultiTargetNode` groups. RuboCop's `node.assignments` returns a
/// flattened list (e.g., `(a, b), c` has 3 assignments), so we need to
/// match that count when comparing against the RHS element count.
fn count_flat_targets(targets: &[ruby_prism::Node<'_>]) -> usize {
    let mut count = 0;
    for t in targets {
        if let Some(multi) = t.as_multi_target_node() {
            // Recurse into nested group
            let children: Vec<_> = multi.lefts().iter().collect();
            count += count_flat_targets(&children);
            // Also count any rest target in the nested group
            if multi.rest().is_some() {
                count += 1;
            }
        } else {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ParallelAssignment, "cops/style/parallel_assignment");

    #[test]
    fn trailing_comma_lhs() {
        let diags = crate::testutil::run_cop_full_internal(
            &ParallelAssignment,
            b"@name, @config, @bulk, = name, config, bulk\n",
            CopConfig::default(),
            "test.rb",
        );
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for trailing-comma LHS, got {}",
            diags.len()
        );
    }

    #[test]
    fn swap_not_flagged() {
        let diags = crate::testutil::run_cop_full_internal(
            &ParallelAssignment,
            b"a, b = b, a\n",
            CopConfig::default(),
            "test.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "Swap should not be flagged, got {} offenses",
            diags.len()
        );
    }

    #[test]
    fn indexed_swap_not_flagged() {
        let diags = crate::testutil::run_cop_full_internal(
            &ParallelAssignment,
            b"arr[0], arr[1] = arr[1], arr[0]\n",
            CopConfig::default(),
            "test.rb",
        );
        assert_eq!(
            diags.len(),
            0,
            "Indexed swap should not be flagged, got {} offenses",
            diags.len()
        );
    }
}
