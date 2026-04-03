use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Rails/ReflectionClassName: Checks if the value of the option `class_name`
/// in the definition of a reflection is a string.
///
/// ## Investigation findings (2026-03-16)
///
/// Root cause of FN: nitrocop used a whitelist approach (only flag constants and
/// method calls on constants), while RuboCop uses a blacklist approach (flag
/// everything except str/sym/dstr, and method calls without a const receiver).
///
/// Key differences fixed:
/// - Removed `has_and_belongs_to_many` from checked methods (not in RuboCop's
///   `RESTRICT_ON_SEND`), which was causing false positives.
/// - Added local variable detection: when `class_name: var` where `var` is a
///   `LocalVariableReadNode`, search the AST for the assignment. Flag unless
///   the assigned value is a string/symbol/dstr.
/// - Switched to blacklist approach matching RuboCop's `reflection_class_value?`:
///   flag non-str/sym/dstr values, except method calls without const receiver.
/// - Added Ruby 3.1 shorthand syntax handling (`class_name:` with ImplicitNode).
pub struct ReflectionClassName;

const ASSOCIATION_METHODS: &[&[u8]] = &[b"has_many", b"has_one", b"belongs_to"];

/// Check if a node is a constant (ConstantReadNode or ConstantPathNode).
fn is_constant(node: &ruby_prism::Node<'_>) -> bool {
    node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some()
}

/// Check if a node is a string type (str, sym, or dstr) that RuboCop considers allowed.
fn is_allowed_type(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_string_node().is_some()
}

/// Visitor that searches for a LocalVariableWriteNode with a specific name
/// and checks whether the assigned value is a string type.
struct LvarAssignmentChecker<'a> {
    target_name: &'a [u8],
    before_offset: usize,
    found_string_assignment: bool,
}

impl<'pr> Visit<'pr> for LvarAssignmentChecker<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.target_name
            && node.location().start_offset() < self.before_offset
            && is_allowed_type(&node.value())
        {
            self.found_string_assignment = true;
        }
    }
}

/// Get the keyword argument pair (AssocNode) for a given key from a call's arguments.
/// Returns the AssocNode location (key + value) and the value node.
fn keyword_arg_pair_and_value<'a>(
    call: &ruby_prism::CallNode<'a>,
    key: &[u8],
) -> Option<(ruby_prism::Location<'a>, ruby_prism::Node<'a>)> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == key {
                            return Some((elem.location(), assoc.value()));
                        }
                    }
                }
            }
        }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == key {
                            return Some((elem.location(), assoc.value()));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Determine if a class_name value should be flagged.
/// Follows RuboCop's logic: flag everything except str/sym/dstr and method calls
/// without a constant receiver.
fn should_flag_value(value: &ruby_prism::Node<'_>, root: &ruby_prism::Node<'_>) -> bool {
    // String, symbol, or interpolated string — always allowed
    if is_allowed_type(value) {
        return false;
    }

    // Method call (send type) — only flag if receiver is a constant.
    // Bare method calls (no receiver) and calls on non-constant receivers are allowed.
    if let Some(method_call) = value.as_call_node() {
        return method_call
            .receiver()
            .is_some_and(|recv| is_constant(&recv));
    }

    // Local variable — flag unless assigned a string/symbol in scope
    if let Some(lvar) = value.as_local_variable_read_node() {
        let mut checker = LvarAssignmentChecker {
            target_name: lvar.name().as_slice(),
            before_offset: lvar.location().start_offset(),
            found_string_assignment: false,
        };
        checker.visit(root);
        return !checker.found_string_assignment;
    }

    // ImplicitNode (Ruby 3.1 shorthand `class_name:`) — unwrap and check the inner value
    if let Some(implicit) = value.as_implicit_node() {
        return should_flag_value(&implicit.value(), root);
    }

    // Constants — flag
    if is_constant(value) {
        return true;
    }

    // Everything else (rare) — flag to match RuboCop's blacklist approach
    true
}

impl Cop for ReflectionClassName {
    fn name(&self) -> &'static str {
        "Rails/ReflectionClassName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if call.receiver().is_some() {
            return;
        }
        if !ASSOCIATION_METHODS.contains(&call.name().as_slice()) {
            return;
        }

        if let Some((pair_loc, value)) = keyword_arg_pair_and_value(&call, b"class_name") {
            let root = parse_result.node();
            if should_flag_value(&value, &root) {
                let (line, column) = source.offset_to_line_col(pair_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use a string value for `class_name`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReflectionClassName, "cops/rails/reflection_class_name");
}
