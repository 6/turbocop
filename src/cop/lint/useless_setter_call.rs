use std::collections::HashMap;

use ruby_prism::Visit;

use crate::cop::shared::method_identifier_predicates;
use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for setter calls to local variables as the final expression of a method.
///
/// ## Investigation findings (2026-03-08)
///
/// **Root cause of 82 FPs:** The original implementation only checked whether
/// the receiver was a local variable (not a parameter), but did NOT track what
/// the variable was assigned to. RuboCop's `MethodVariableTracker` checks whether
/// the local variable holds a "local object" — one created via `.new` or a literal.
/// Variables assigned from method calls (e.g., `Record.find(1)`, `Config.current`),
/// parameters, ivars, or other non-constructor sources are NOT local objects, so
/// setter calls on them are NOT useless (the object may be referenced elsewhere).
///
/// **Root cause of 3 FNs:** The cop excluded `[]=` (square bracket setter), but
/// RuboCop correctly flags `x[:key] = val` when `x` holds a local object.
///
/// **Fix:** Implemented `MethodVariableTracker` equivalent that scans all
/// assignments in the method body to determine whether each local variable
/// contains a locally-created object. Only flags setter calls on variables
/// that hold local objects (created via `.new` or literals).
///
/// ## Investigation findings (2026-03-10)
///
/// **Root cause of remaining 3 FPs:** When a method body has implicit
/// `rescue`/`ensure`/`else` clauses (e.g., `def foo; x.attr = 5; rescue; end`),
/// Prism wraps the body in a `BeginNode`. The code was unwrapping ALL BeginNodes
/// to find the last statement inside, but RuboCop's `last_expression` only unwraps
/// bare `begin` nodes (statement sequences), NOT rescue/ensure bodies. The fix:
/// only unwrap `BeginNode` when it has no rescue/ensure/else clauses.
///
/// **Root cause of the last FP:** `===` was still treated as a setter because the
/// helper only excluded `==`, `!=`, `<=`, and `>=`. RuboCop's `setter_method?`
/// predicate does not match case equality, so `===` must be excluded too.
///
/// ## Investigation findings (2026-03-24)
///
/// **Root cause of 12 FPs:** `.new` calls with a block (e.g., `Thread.new { ... }`,
/// `Class.new(Base) { ... }`) were incorrectly treated as local constructors. In
/// RuboCop's Parser AST, a block call is wrapped in a separate `(block ...)` node,
/// so `constructor?` (which checks `send_type?`) naturally returns `false` for them.
/// In Prism, the block is part of the `CallNode` itself, so we must explicitly check
/// `call.block().is_none()` before treating a `.new` call as a local constructor.
/// Objects created via `.new` with a block may be externally referenced (threads run
/// in the background, classes registered globally via `const_set`, etc.).
pub struct UselessSetterCall;

impl Cop for UselessSetterCall {
    fn name(&self) -> &'static str {
        "Lint/UselessSetterCall"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Find the last expression in the method body.
        // RuboCop's last_expression only unwraps `begin` type (bare statement sequence),
        // NOT rescue/ensure nodes. In Prism, BeginNode covers all of these, so we must
        // check: only unwrap BeginNode if it has NO rescue/ensure/else clauses (i.e., it's
        // just a bare `begin..end` or implicit statement sequence). If it has rescue/ensure,
        // the BeginNode itself is the last expression (not a setter call), so we skip.
        let last_expr_opt = if let Some(stmts) = body.as_statements_node() {
            stmts.body().iter().last()
        } else if let Some(begin) = body.as_begin_node() {
            // Only unwrap if this is a plain begin block (no rescue/ensure/else)
            if begin.rescue_clause().is_none()
                && begin.ensure_clause().is_none()
                && begin.else_clause().is_none()
            {
                begin.statements().and_then(|s| s.body().iter().last())
            } else {
                // Method body is begin+rescue/ensure — the last expression is the
                // BeginNode itself, which is not a setter call, so no offense.
                None
            }
        } else {
            None
        };

        // If body is a single expression (not StatementsNode/BeginNode), use it directly
        let call = if let Some(last_expr) = &last_expr_opt {
            match last_expr.as_call_node() {
                Some(c) => c,
                None => return,
            }
        } else if last_expr_opt.is_none()
            && body.as_statements_node().is_none()
            && body.as_begin_node().is_none()
        {
            match body.as_call_node() {
                Some(c) => c,
                None => return,
            }
        } else {
            return;
        };

        // Must be a setter method (name ends with `=`, but not ==, !=, <=, >=)
        if !is_setter_method(call.name().as_slice()) {
            return;
        }

        // Receiver must be a local variable
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let lv = match recv.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };

        let var_name_bytes = lv.name().as_slice();
        let var_name = std::str::from_utf8(var_name_bytes).unwrap_or("var");

        // Track variable assignments in the method body to determine if
        // the variable holds a locally-created object
        let mut tracker = VariableTracker::new();

        // Collect parameter names — these are non-local by default
        if let Some(params) = def_node.parameters() {
            for p in params.requireds().iter() {
                if let Some(rp) = p.as_required_parameter_node() {
                    tracker.mark_non_local(rp.name().as_slice());
                }
            }
            for p in params.optionals().iter() {
                if let Some(op) = p.as_optional_parameter_node() {
                    tracker.mark_non_local(op.name().as_slice());
                }
            }
            for p in params.keywords().iter() {
                if let Some(kp) = p.as_required_keyword_parameter_node() {
                    tracker.mark_non_local(kp.name().as_slice());
                } else if let Some(kp) = p.as_optional_keyword_parameter_node() {
                    tracker.mark_non_local(kp.name().as_slice());
                }
            }
            if let Some(rest) = params.rest() {
                if let Some(rp) = rest.as_rest_parameter_node() {
                    if let Some(name) = rp.name() {
                        tracker.mark_non_local(name.as_slice());
                    }
                }
            }
            if let Some(block) = params.block() {
                if let Some(name) = block.name() {
                    tracker.mark_non_local(name.as_slice());
                }
            }
            if let Some(krest) = params.keyword_rest() {
                if let Some(krp) = krest.as_keyword_rest_parameter_node() {
                    if let Some(name) = krp.name() {
                        tracker.mark_non_local(name.as_slice());
                    }
                }
            }
        }

        // Scan the method body for assignments using the Visit trait
        tracker.visit(&body);

        // Only flag if the variable contains a local object
        if !tracker.is_local(var_name_bytes) {
            return;
        }

        // Use the call node's location (which IS the last expression)
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Useless setter call to local variable `{var_name}`."),
        ));
    }
}

/// Check if a method name is a setter (ends with `=` but not comparison operators).
/// Includes `[]=` which RuboCop also flags.
fn is_setter_method(name: &[u8]) -> bool {
    method_identifier_predicates::is_setter_method(name)
}

/// Check if a node is a constructor call (`.new`) or a literal.
/// These create local objects that don't exist outside the method.
fn is_constructor(node: &ruby_prism::Node<'_>) -> bool {
    // Literals
    if node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_array_node().is_some()
        || node.as_string_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_range_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
    {
        return true;
    }

    // `.new` call WITHOUT a block — creates a new local object.
    // When `.new` has a block (e.g., `Thread.new { ... }`, `Class.new(Base) { ... }`),
    // the object may be externally referenced (thread runs in background, class registered
    // globally, etc.), so it's not purely local. RuboCop's Parser AST wraps block calls in
    // a separate `block` node, so `constructor?` naturally returns false for them. In Prism,
    // the block is part of the CallNode, so we must check explicitly.
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"new" && call.block().is_none() {
            return true;
        }
    }

    false
}

/// Get the name bytes from a variable read node (lvar, ivar, cvar, gvar).
fn variable_read_name<'a>(node: &ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(lv) = node.as_local_variable_read_node() {
        Some(lv.name().as_slice())
    } else if let Some(iv) = node.as_instance_variable_read_node() {
        Some(iv.name().as_slice())
    } else if let Some(cv) = node.as_class_variable_read_node() {
        Some(cv.name().as_slice())
    } else if let Some(gv) = node.as_global_variable_read_node() {
        Some(gv.name().as_slice())
    } else {
        None
    }
}

/// Tracks whether local variables hold locally-created objects.
/// Uses the Visit trait to recursively scan assignments in the method body.
struct VariableTracker {
    /// true = local object (created via .new or literal), false = non-local
    locals: HashMap<Vec<u8>, bool>,
}

impl VariableTracker {
    fn new() -> Self {
        Self {
            locals: HashMap::new(),
        }
    }

    fn mark_non_local(&mut self, name: &[u8]) {
        self.locals.insert(name.to_vec(), false);
    }

    fn mark_local(&mut self, name: &[u8]) {
        self.locals.insert(name.to_vec(), true);
    }

    fn is_local(&self, name: &[u8]) -> bool {
        self.locals.get(name).copied().unwrap_or(false)
    }

    /// Process an assignment: determine if the RHS is a local object.
    fn process_assignment(&mut self, name: &[u8], rhs: &ruby_prism::Node<'_>) {
        if variable_read_name(rhs).is_some() {
            // If RHS is a variable read, inherit its locality
            let rhs_name = variable_read_name(rhs);
            let is_local = rhs_name
                .and_then(|n| self.locals.get(n).copied())
                .unwrap_or(false);
            self.locals.insert(name.to_vec(), is_local);
        } else if is_constructor(rhs) {
            self.mark_local(name);
        } else {
            // Method calls, etc. — non-local (could return shared object)
            self.mark_non_local(name);
        }
    }
}

impl<'pr> Visit<'pr> for VariableTracker {
    // Local variable write: `x = expr`
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.process_assignment(node.name().as_slice(), &node.value());
        // Continue visiting children (RHS might contain nested assignments)
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    // Local variable operator write: `x += expr` — binary op creates new object
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.mark_local(node.name().as_slice());
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    // Local variable or-write: `x ||= expr`
    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let rhs = node.value();
        if let Some(rhs_name) = variable_read_name(&rhs) {
            let rhs_is_local = self.locals.get(rhs_name).copied().unwrap_or(false);
            if !rhs_is_local {
                self.mark_non_local(node.name().as_slice());
            }
        } else if is_constructor(&rhs) {
            // Only mark local if not already assigned from a non-local source
            if !self.locals.contains_key(node.name().as_slice()) {
                self.mark_local(node.name().as_slice());
            }
        } else {
            self.mark_non_local(node.name().as_slice());
        }
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    // Local variable and-write: `x &&= expr`
    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let rhs = node.value();
        if let Some(rhs_name) = variable_read_name(&rhs) {
            let rhs_is_local = self.locals.get(rhs_name).copied().unwrap_or(false);
            if !rhs_is_local {
                self.mark_non_local(node.name().as_slice());
            }
        } else if !is_constructor(&rhs) {
            self.mark_non_local(node.name().as_slice());
        }
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    // Multi-write: `a, b, c = expr` or `a, b, c = 1, 2, 3`
    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        let rhs = node.value();

        if let Some(arr) = rhs.as_array_node() {
            // `a, b = x, y` — each target gets corresponding RHS element
            let rhs_elements: Vec<_> = arr.elements().iter().collect();
            for (i, target) in node.lefts().iter().enumerate() {
                if let Some(lw) = target.as_local_variable_target_node() {
                    if let Some(rhs_elem) = rhs_elements.get(i) {
                        self.process_assignment(lw.name().as_slice(), rhs_elem);
                    } else {
                        self.mark_local(lw.name().as_slice());
                    }
                }
            }
        } else {
            // `a, b = some_method` — all targets get unknown objects
            // RuboCop marks them as local=true in this case
            for target in node.lefts().iter() {
                if let Some(lw) = target.as_local_variable_target_node() {
                    self.mark_local(lw.name().as_slice());
                }
            }
        }
        // Don't recurse into children (avoid double-processing)
    }

    // Don't descend into nested method definitions
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Skip nested defs entirely
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessSetterCall, "cops/lint/useless_setter_call");
}
