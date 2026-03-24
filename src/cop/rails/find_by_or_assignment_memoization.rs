use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Investigation notes (2026-03-16):
///
/// Root cause of 8 FNs in `basecamp__fizzy__a02042b`: the visitor's
/// `visit_instance_variable_or_write_node` was returning early (without recursing into children)
/// when the value was not a direct `find_by` call. This caused nested `||=` inside
/// `@outer ||= begin ... Class.new do ... def method ... @inner ||= foo.find_by(...) end end end`
/// patterns to be silently skipped — the visitor never descended into the begin block or the
/// anonymous class body.
///
/// Fix: always call `ruby_prism::visit_instance_variable_or_write_node` at the end to recurse,
/// even when the current node doesn't match (value is not a direct find_by call, or we're
/// inside an if/unless). The if/unless skip logic (`in_if_depth`) correctly matches RuboCop's
/// `assignment_node.each_ancestor(:if).any?` check — a `||=` inside an if body is skipped
/// regardless of depth in other constructs.
///
/// After fix: FN=0, FP=0 on corpus.
///
/// Investigation notes (2026-03-23):
///
/// Extended corpus: FP=4, FN=2.
///
/// FP root cause: RuboCop's `instance_variable_assigned?` check skips offenses when the same
/// instance variable is assigned (via plain `=`) in any `initialize` method in the file. This
/// respects Ruby 3.2's "object shapes" optimization — ivars initialized in `initialize` are
/// expected to be set at construction time and don't need `defined?`-based memoization.
/// Examples: `@tmu` assigned in `initialize` in WikiEduDashboard, `@host` assigned in
/// `initialize` in foreman_salt, `@invoice`/`@previous_daily_usage` in lago-api.
///
/// FN root cause: RuboCop has two entry points — `on_def` and `on_send`. `on_def` fires when
/// the method body is exactly `||= find_by` and does NOT check for `if` ancestors. `on_send`
/// fires for the `find_by` call and DOES check `each_ancestor(:if)`. So a `def` inside an
/// `if` block is caught by `on_def` but skipped by `on_send`. In nitrocop, `in_if_depth` was
/// inherited across `def` boundaries, causing these to be incorrectly skipped.
///
/// Fix: (1) Pre-collect ivar names from `initialize` methods and skip matching ||= nodes.
/// (2) Reset `in_if_depth` at `def` boundaries so outer `if` blocks don't suppress detection
/// inside inner method definitions.
pub struct FindByOrAssignmentMemoization;

/// Check if a node is a `find_by` call (not `find_by!`) without safe navigation.
fn is_find_by_call_without_safe_nav(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() != b"find_by" {
            return false;
        }
        // RuboCop uses (send ...) not (csend ...), so &.find_by is excluded
        if call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
        {
            return false;
        }
        return true;
    }
    false
}

/// Pre-pass visitor to collect instance variable names assigned in `initialize` methods.
struct InitializeIvarCollector {
    /// Set of ivar names (e.g. b"@host") assigned via `=` in any `initialize` method.
    ivar_names: Vec<Vec<u8>>,
    in_initialize: bool,
}

impl<'pr> Visit<'pr> for InitializeIvarCollector {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if node.name().as_slice() == b"initialize" {
            self.in_initialize = true;
            ruby_prism::visit_def_node(self, node);
            self.in_initialize = false;
        } else {
            ruby_prism::visit_def_node(self, node);
        }
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        if self.in_initialize {
            let name = node.name().as_slice().to_vec();
            if !self.ivar_names.contains(&name) {
                self.ivar_names.push(name);
            }
        }
        ruby_prism::visit_instance_variable_write_node(self, node);
    }
}

impl Cop for FindByOrAssignmentMemoization {
    fn name(&self) -> &'static str {
        "Rails/FindByOrAssignmentMemoization"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // First pass: collect ivar names assigned in initialize methods.
        let mut collector = InitializeIvarCollector {
            ivar_names: Vec::new(),
            in_initialize: false,
        };
        collector.visit(&parse_result.node());

        // Second pass: detect ||= find_by offenses.
        let mut visitor = FindByVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_if_depth: 0,
            initialize_ivars: &collector.ivar_names,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct FindByVisitor<'a> {
    cop: &'a FindByOrAssignmentMemoization,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_if_depth: usize,
    /// Ivar names assigned in `initialize` — these are skipped per RuboCop's object shapes check.
    initialize_ivars: &'a [Vec<u8>],
}

impl<'pr> Visit<'pr> for FindByVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.in_if_depth += 1;
        ruby_prism::visit_if_node(self, node);
        self.in_if_depth -= 1;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.in_if_depth += 1;
        ruby_prism::visit_unless_node(self, node);
        self.in_if_depth -= 1;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Reset in_if_depth at def boundaries: a `def` inside an `if` block creates a new
        // method scope. RuboCop's `on_def` handler doesn't check for `if` ancestors, so
        // outer `if` blocks should not suppress detection inside the method.
        let saved_if_depth = self.in_if_depth;
        self.in_if_depth = 0;
        ruby_prism::visit_def_node(self, node);
        self.in_if_depth = saved_if_depth;
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        // When inside an if/unless, the ||= itself has an if ancestor — RuboCop skips these.
        // We still recurse into children because an inner def (e.g., inside a block passed
        // to Class.new) starts a fresh method scope and its own ||= nodes are independent.
        if self.in_if_depth == 0 {
            let value = node.value();

            // The value should be a direct find_by call (not part of || or ternary),
            // and not using safe navigation (&.find_by)
            if is_find_by_call_without_safe_nav(&value) {
                // Skip if the same ivar is assigned in an initialize method (object shapes).
                let ivar_name = node.name().as_slice();
                if !self
                    .initialize_ivars
                    .iter()
                    .any(|n| n.as_slice() == ivar_name)
                {
                    let loc = node.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Avoid memoizing `find_by` results with `||=`.".to_string(),
                    ));
                }
            }
        }

        // Always recurse into children so we catch ||= nodes nested inside begin blocks,
        // blocks passed to Class.new, inner method definitions, etc.
        ruby_prism::visit_instance_variable_or_write_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        FindByOrAssignmentMemoization,
        "cops/rails/find_by_or_assignment_memoization"
    );
}
