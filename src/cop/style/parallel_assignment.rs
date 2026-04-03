use crate::cop::shared::node_type::{ARRAY_NODE, MULTI_WRITE_NODE, SPLAT_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ParallelAssignment flags `a, b = 1, 2` style multi-assignment when
/// each target gets exactly one value and the assignment can be safely rewritten
/// as sequential assignments.
///
/// ## Swap/cycle exemption
/// Uses proper cycle detection (Kahn's algorithm / topological sort),
/// matching RuboCop's `AssignmentSorter`. Dependencies are detected via
/// word-boundary substring matching on source text, which handles:
///   - Simple swaps: `a, b = b, a`
///   - Expression cycles: `x, y = y, x+y` (Fibonacci pattern)
///   - Indexed/method swaps: `arr[0], arr[1] = arr[1], arr[0]`
///   - Rotations: `a, b, c = b, c, a`
///
/// One-directional dependencies (no cycle) are correctly flagged:
///   - `old, @var = @var, true` → `old = @var; @var = true`
///   - `state, opts = opts, nil` → sequential rewrite is safe
///
/// ## Implicit-self swap detection
/// RuboCop's `add_self_to_getters` converts bare method calls `foo` on the
/// RHS to `self.foo` for dependency analysis. This lets it detect swaps like
/// `self.a, self.b = b, a` where `b` is really `self.b`. We replicate this
/// by stripping `self.` from LHS targets and checking for bare-name matches.
///
/// ## Sigil-aware variable matching
/// Bare identifiers like `react` must not match inside sigiled variables
/// like `@react`. We reject matches where a bare target is preceded by
/// `@` or `$` in the RHS text, preventing false cycles.
///
/// ## Splat handling (matching RuboCop's MlhsNode#assignments)
/// RuboCop's `node.assignments` unwraps named splats (`*b` → `b`) and
/// flattens nested groups. Only truly anonymous splats (`*`) remain as
/// splat nodes, which trigger `allowed_lhs?`. We replicate this:
///   - Named splats are unwrapped and counted as regular targets
///   - Anonymous splats cause the whole assignment to be skipped
///   - `a, *b, c = 1, 2, 3` is flagged (3 targets, 3 RHS)
///   - `a, *b = 1, 2, 3` is not flagged (2 targets, 3 RHS — mismatch)
///
/// ## Rescue modifier on RHS
/// `a, b = x, y rescue nil` wraps the RHS in a RescueModifierNode.
/// We unwrap it (matching RuboCop's `rhs.body if rhs.rescue_type?`)
/// to get the underlying array for the size/swap check.
///
/// ## Trailing-comma / ImplicitRestNode
/// `@name, @config, @bulk, = name, config, bulk` has a trailing comma that
/// Prism represents as `ImplicitRestNode` in the `rest()` slot. We only
/// skip when `rest()` is a real anonymous `SplatNode` (i.e., `*` without
/// a variable name).
///
/// ## Nested group / flattened target count
/// RuboCop's `node.assignments` flattens nested groups: `(a, b), c` has 3
/// assignments, not 2. We replicate this by counting leaf targets recursively
/// (including lefts, rest, and rights of nested MultiTargetNodes).
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

        // Collect ALL targets: lefts + unwrapped rest + rights
        // This matches RuboCop's MlhsNode#assignments which unwraps named splats.
        let mut targets: Vec<_> = multi_write.lefts().iter().collect();

        // Handle rest target
        if let Some(rest) = multi_write.rest() {
            if rest.as_implicit_rest_node().is_some() {
                // ImplicitRestNode (trailing comma: a, b, = ...) — ignore it
            } else if let Some(splat) = rest.as_splat_node() {
                if let Some(inner) = splat.expression() {
                    // Named splat (*b) — unwrap and count as regular target
                    targets.push(inner);
                } else {
                    // Anonymous splat (*) — skip assignment entirely
                    return;
                }
            } else {
                // Unknown rest type — skip to be safe
                return;
            }
        }

        // Add rights (targets after the splat, e.g., c, d in `a, *b, c, d = ...`)
        for r in multi_write.rights().iter() {
            targets.push(r);
        }

        // Check if there are at least 2 targets
        if targets.len() < 2 {
            return;
        }

        // Check for anonymous splats inside nested MultiTargetNodes
        if has_nested_anonymous_splat(&targets) {
            return;
        }

        // The value is the RHS. In Prism, for `a, b = 1, 2`, the value is an ArrayNode
        // with the implicit array of values. For `a, b = foo`, it's just a single node.
        let value = multi_write.value();

        // Unwrap rescue modifier: `a, b = x, y rescue nil`
        // RuboCop: `rhs = rhs.body if rhs.rescue_type?`
        let unwrapped_value = if let Some(rescue_mod) = value.as_rescue_modifier_node() {
            rescue_mod.expression()
        } else {
            value
        };

        // Check if RHS is an array node (implicit or explicit) with matching count.
        // RuboCop flattens nested groups in the LHS count (e.g., `(a, b), c` counts
        // as 3 targets, not 2). We replicate this: if the LHS has nested
        // MultiTargetNodes, use the flattened leaf count for the size comparison.
        if let Some(arr) = unwrapped_value.as_array_node() {
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

/// Check if any nested MultiTargetNode contains an anonymous splat (*).
/// RuboCop's `allowed_lhs?` skips assignments where any flattened target
/// is still a splat (which only happens with anonymous splats).
fn has_nested_anonymous_splat(targets: &[ruby_prism::Node<'_>]) -> bool {
    for t in targets {
        if let Some(multi) = t.as_multi_target_node() {
            // Check rest in this nested group
            if let Some(rest) = multi.rest() {
                if let Some(splat) = rest.as_splat_node() {
                    if splat.expression().is_none() {
                        return true; // Anonymous splat * inside nested group
                    }
                }
            }
            // Recurse into lefts and rights
            let lefts: Vec<_> = multi.lefts().iter().collect();
            if has_nested_anonymous_splat(&lefts) {
                return true;
            }
            let rights: Vec<_> = multi.rights().iter().collect();
            if has_nested_anonymous_splat(&rights) {
                return true;
            }
        }
    }
    false
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
            if i != j && !lhs_texts[j].is_empty() {
                // Check direct reference
                let has_dep = has_variable_reference(rhs_texts[i], lhs_texts[j])
                    // Also check implicit-self: if LHS is `self.foo`, check if
                    // RHS references the bare name `foo` (which is implicitly
                    // `self.foo` in Ruby).
                    || has_implicit_self_reference(rhs_texts[i], lhs_texts[j]);

                if has_dep {
                    adj[i].push(j);
                    in_degree[j] += 1;
                }
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

/// Check if `rhs_text` references a bare method name from a `self.xxx` LHS target.
///
/// When LHS is `self.issue_from`, the bare name `issue_from` in the RHS is really
/// `self.issue_from` (implicit self). This replicates RuboCop's `add_self_to_getters`.
fn has_implicit_self_reference(rhs_text: &str, lhs_target: &str) -> bool {
    if let Some(bare_name) = lhs_target.strip_prefix("self.") {
        if !bare_name.is_empty() {
            return has_variable_reference(rhs_text, bare_name);
        }
    }
    false
}

/// Check if `rhs_text` contains a reference to `target` using word-boundary
/// matching. Returns true if `target` appears in `rhs_text` as a whole token
/// (not inside a larger identifier).
///
/// For sigiled variables (`@foo`), also ensures the match isn't part of a
/// class variable (`@@foo`).
///
/// For bare identifiers (no sigil), also ensures the match isn't preceded by
/// `@` or `$`, which would make it a different variable (e.g., `react` should
/// not match inside `@react`).
fn has_variable_reference(rhs_text: &str, target: &str) -> bool {
    if rhs_text == target {
        return true;
    }

    let rhs_bytes = rhs_text.as_bytes();
    let target_bytes = target.as_bytes();

    if target_bytes.is_empty() || target_bytes.len() > rhs_bytes.len() {
        return false;
    }

    // Determine if target is a bare identifier (no sigil prefix)
    let target_is_bare =
        !target.starts_with('@') && !target.starts_with('$') && !target.starts_with("@@");

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

                // For bare identifiers, reject if preceded by @, $, or : (would be
                // @var, $var, or :symbol — none of which are references to the
                // bare local variable)
                let bare_ok = !target_is_bare
                    || abs_pos == 0
                    || (rhs_bytes[abs_pos - 1] != b'@'
                        && rhs_bytes[abs_pos - 1] != b'$'
                        && rhs_bytes[abs_pos - 1] != b':');

                // Word boundary after: end of string or non-ident char
                let after_ok =
                    end_pos >= rhs_bytes.len() || !is_ruby_ident_char(rhs_bytes[end_pos]);

                if before_ok && sigil_ok && bare_ok && after_ok {
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
///
/// This counts lefts, rest (as 1 if present), and rights from nested groups.
fn count_flat_targets(targets: &[ruby_prism::Node<'_>]) -> usize {
    let mut count = 0;
    for t in targets {
        if let Some(multi) = t.as_multi_target_node() {
            // Count lefts recursively
            let lefts: Vec<_> = multi.lefts().iter().collect();
            count += count_flat_targets(&lefts);
            // Count rest target (named splat counts as 1)
            if multi.rest().is_some() {
                count += 1;
            }
            // Count rights recursively
            let rights: Vec<_> = multi.rights().iter().collect();
            count += count_flat_targets(&rights);
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
