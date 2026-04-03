use crate::cop::shared::node_type::MULTI_WRITE_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for unnecessary trailing underscore variables in parallel assignment.
///
/// ## Investigation findings (2026-03-19)
///
/// Root causes of FP=8, FN=39 in the 1000-repo corpus oracle:
///
/// **FN root causes:**
/// - `posts()` not checked: For `a, *_, _, _ = foo`, Prism puts post-rest
///   variables in `posts()`, but the cop only checked `lefts()`.
/// - Splat-only rest: `a, *_ = foo` should flag since `*_` is an underscore
///   splat with no posts, but the rest-only check was too restrictive.
/// - Nested destructuring: `a, (b, _) = foo` has an inner `MultiTargetNode`
///   that wasn't recursed into.
/// - Splat-before exemption: `a, *_b, _ = foo` should flag when `AllowNamedUnderscoreVariables`
///   is false (the splat is a named underscore), but the code didn't distinguish
///   underscore splats from non-underscore splats.
///
/// **FP root causes:**
/// - Named splat underscore with AllowNamedUnderscoreVariables=true: `a, *_b = foo`
///   should NOT be flagged when AllowNamedUnderscoreVariables is true (default).
/// - Splat before trailing underscore: `a, *b, _ = foo` should NOT be flagged
///   because there's a non-underscore splat before the trailing underscore.
///   The cop was not checking this exemption.
/// - Bare splat (`*`) was incorrectly treated as an underscore variable (FP=167).
///   Patterns like `a, b, * = values` should not be flagged — RuboCop only treats
///   named underscore splats (`*_`, `*_var`) as trailing underscore variables.
///
/// **Fixes applied:**
/// - Added `posts()` iteration to detect trailing underscores after rest.
/// - Added splat-before exemption: if rest is a non-underscore splat, don't
///   flag trailing underscores in posts.
/// - Added nested `MultiTargetNode` recursion for inner destructuring.
/// - Combined lefts + rest + posts into a unified variable list for analysis.
pub struct TrailingUnderscoreVariable;

impl Cop for TrailingUnderscoreVariable {
    fn name(&self) -> &'static str {
        "Style/TrailingUnderscoreVariable"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[MULTI_WRITE_NODE]
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
        let allow_named = config.get_bool("AllowNamedUnderscoreVariables", true);

        // Only handle top-level MultiWriteNode; nested MultiTargetNode is handled
        // via children_offenses recursion from the parent MultiWriteNode.
        let multi = match node.as_multi_write_node() {
            Some(m) => m,
            None => return,
        };

        let lefts: Vec<_> = multi.lefts().iter().collect();
        let rest = multi.rest();
        let rights: Vec<_> = multi.rights().iter().collect();

        // Check the main node for trailing underscore offense
        check_multi_assignment(
            self,
            source,
            &lefts,
            rest.as_ref(),
            &rights,
            allow_named,
            diagnostics,
        );

        // Check nested MultiTargetNode children (e.g., `a, (b, _) = foo`)
        check_children_offenses(self, source, &lefts, allow_named, diagnostics);
        check_children_offenses(self, source, &rights, allow_named, diagnostics);
    }
}

/// Check a multi-assignment (lefts + optional rest + posts) for trailing underscore variables.
fn check_multi_assignment(
    cop: &TrailingUnderscoreVariable,
    source: &SourceFile,
    lefts: &[ruby_prism::Node<'_>],
    rest: Option<&ruby_prism::Node<'_>>,
    rights: &[ruby_prism::Node<'_>],
    allow_named: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Build a unified list of all variables in order: lefts, rest, rights
    // We need to find how many trailing ones are underscore variables.

    // RuboCop's splat_variable_before? check: if there's a non-underscore splat
    // BEFORE the first trailing underscore, the trailing underscores are exempt.
    // This applies to both lefts containing splats and the rest position.

    // First, count trailing underscores from the end.
    // Order: lefts..., rest, rights...
    let mut trailing_count = 0;

    // Count trailing underscores in rights (from the end)
    for target in rights.iter().rev() {
        if is_underscore_var(target, allow_named) {
            trailing_count += 1;
        } else {
            break;
        }
    }

    // If ALL rights are trailing underscores, check rest and lefts
    if trailing_count == rights.len() {
        if let Some(r) = rest {
            if r.as_implicit_rest_node().is_some() {
                // ImplicitRestNode (trailing comma like `a, _, = foo`) — skip it
                // and continue counting from lefts. It's not a variable.
                for target in lefts.iter().rev() {
                    if is_underscore_var(target, allow_named) {
                        trailing_count += 1;
                    } else {
                        break;
                    }
                }
            } else if is_underscore_var(r, allow_named) {
                trailing_count += 1;

                // If rest is also underscore, continue counting from lefts
                for target in lefts.iter().rev() {
                    if is_underscore_var(target, allow_named) {
                        trailing_count += 1;
                    } else {
                        break;
                    }
                }
            }
        } else {
            // No rest — count trailing underscores from lefts
            for target in lefts.iter().rev() {
                if is_underscore_var(target, allow_named) {
                    trailing_count += 1;
                } else {
                    break;
                }
            }
        }
    }

    if trailing_count == 0 {
        return;
    }

    // Check for splat_variable_before: if there's a non-underscore splat anywhere
    // before the first trailing underscore, the offense is exempt.
    // This handles cases like `*a, b, _ = foo` and `a, *b, _ = foo`.
    let first_trailing = find_first_trailing(lefts, rest, rights, trailing_count);
    if first_trailing.is_none() {
        return;
    }

    if has_non_underscore_splat_before(lefts, rest, trailing_count, allow_named) {
        return;
    }

    let first = first_trailing.unwrap();
    let loc = first.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Trailing underscore variable(s) in parallel assignment are unnecessary.".to_string(),
    ));
}

/// Check if there's a non-underscore splat variable before the first trailing underscore.
/// This implements RuboCop's `splat_variable_before?` check.
fn has_non_underscore_splat_before(
    lefts: &[ruby_prism::Node<'_>],
    rest: Option<&ruby_prism::Node<'_>>,
    trailing_count: usize,
    allow_named: bool,
) -> bool {
    // Check if any left variable is a non-underscore splat
    // (This handles cases like `*a, b, _ = foo` where the splat is in lefts)
    let non_trailing_lefts_count = if trailing_count > lefts.len() {
        0
    } else {
        lefts.len() - std::cmp::min(trailing_count, lefts.len())
    };
    for left in &lefts[..non_trailing_lefts_count] {
        if left.as_splat_node().is_some() && !is_underscore_var(left, allow_named) {
            return true;
        }
    }

    // Check if rest is a non-underscore splat (and is NOT part of the trailing underscores)
    // Skip ImplicitRestNode (trailing comma) — it's not a real splat variable.
    if let Some(r) = rest {
        if r.as_implicit_rest_node().is_none() && !is_underscore_var(r, allow_named) {
            return true;
        }
    }

    false
}

/// Find the node that is the first trailing underscore variable.
fn find_first_trailing<'a>(
    lefts: &'a [ruby_prism::Node<'a>],
    rest: Option<&'a ruby_prism::Node<'a>>,
    rights: &'a [ruby_prism::Node<'a>],
    trailing_count: usize,
) -> Option<&'a ruby_prism::Node<'a>> {
    // Build a flat list: lefts, rest (excluding ImplicitRestNode), rights
    let mut all: Vec<&ruby_prism::Node<'a>> = Vec::new();
    for l in lefts {
        all.push(l);
    }
    if let Some(r) = rest {
        // Skip ImplicitRestNode — it's not a real variable node
        if r.as_implicit_rest_node().is_none() {
            all.push(r);
        }
    }
    for p in rights {
        all.push(p);
    }

    if trailing_count > all.len() || trailing_count == 0 {
        return None;
    }

    let idx = all.len() - trailing_count;
    Some(all[idx])
}

/// Recursively check children for nested multi-target nodes (e.g., `(b, _)` in `a, (b, _) = foo`).
fn check_children_offenses(
    cop: &TrailingUnderscoreVariable,
    source: &SourceFile,
    variables: &[ruby_prism::Node<'_>],
    allow_named: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for var in variables {
        if let Some(mt) = var.as_multi_target_node() {
            let lefts: Vec<_> = mt.lefts().iter().collect();
            let rest = mt.rest();
            let rights: Vec<_> = mt.rights().iter().collect();

            check_multi_assignment(
                cop,
                source,
                &lefts,
                rest.as_ref(),
                &rights,
                allow_named,
                diagnostics,
            );

            // Recurse into nested multi-target nodes
            check_children_offenses(cop, source, &lefts, allow_named, diagnostics);
            check_children_offenses(cop, source, &rights, allow_named, diagnostics);
        }
    }
}

fn is_underscore_var(node: &ruby_prism::Node<'_>, allow_named: bool) -> bool {
    if let Some(target) = node.as_local_variable_target_node() {
        let name = target.name().as_slice();
        if name == b"_" {
            return true;
        }
        if !allow_named && name.starts_with(b"_") {
            return true;
        }
        return false;
    }
    // Splat node like *_
    if let Some(splat) = node.as_splat_node() {
        if let Some(expr) = splat.expression() {
            if let Some(target) = expr.as_local_variable_target_node() {
                let name = target.name().as_slice();
                if name == b"_" {
                    return true;
                }
                if !allow_named && name.starts_with(b"_") {
                    return true;
                }
            }
        } else {
            // bare * (implicit rest) — NOT an underscore variable.
            // RuboCop doesn't treat bare splat as a trailing underscore.
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TrailingUnderscoreVariable,
        "cops/style/trailing_underscore_variable"
    );
}
