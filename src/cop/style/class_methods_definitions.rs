use crate::cop::node_type::{DEF_NODE, SELF_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ClassMethodsDefinitions cop.
///
/// ## Investigation findings (2026-03-31)
///
/// RuboCop only inspects direct child plain `def` nodes inside `class << self`.
/// Visibility-wrapped forms like `private def helper`, `protected def helper`,
/// and `public def helper` are wrapped in a call node, so they do not count as
/// candidate methods for this cop at all.
///
/// nitrocop previously walked all statements in the singleton class body and
/// treated inline visibility-wrapped defs as blockers (or additional public
/// defs). That caused false negatives when a `class << self` block mixed inline
/// helpers with direct public defs.
///
/// Fix: mirror RuboCop's `def_nodes` + `node_visibility` behavior more closely.
/// Only direct child plain `def` nodes are counted, compact same-line forms are
/// still skipped, bare left-sibling visibility sections (`private`, `protected`,
/// `public`) still apply, and right-sibling `private/protected/public :name`
/// overrides are resolved per method name.
pub struct ClassMethodsDefinitions;

impl Cop for ClassMethodsDefinitions {
    fn name(&self) -> &'static str {
        "Style/ClassMethodsDefinitions"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, SELF_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "def_self");

        if enforced_style == "def_self" {
            // Check for `class << self` with public methods
            if let Some(sclass) = node.as_singleton_class_node() {
                let expr = sclass.expression();
                if expr.as_self_node().is_some() {
                    // Check if body has defs and ALL are public
                    if let Some(body) = sclass.body() {
                        let sclass_line = source
                            .offset_to_line_col(sclass.location().start_offset())
                            .0;
                        if all_defs_public(source, &body, sclass_line) {
                            let loc = sclass.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Do not define public methods within class << self.".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Returns true if the sclass body contains at least one plain `def` node
/// (no receiver) and ALL such `def` nodes are public. This matches RuboCop's
/// `all_methods_public?` which only flags `class << self` when every method
/// can be trivially converted to `def self.method_name`.
///
/// Also returns false (skip) if any plain `def` starts on the same line as
/// the `class << self` keyword — RuboCop does not flag compact single-line forms.
fn all_defs_public(source: &SourceFile, body: &ruby_prism::Node<'_>, sclass_line: usize) -> bool {
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => {
            // Single-statement body: check if it's a plain def node (no receiver)
            if let Some(def_node) = body.as_def_node() {
                if def_node.receiver().is_some() {
                    return false; // `def self.x` — not a plain def
                }
                // Check single-line: if def is on same line as class << self, skip
                let def_line = source
                    .offset_to_line_col(def_node.location().start_offset())
                    .0;
                return def_line != sclass_line;
            }
            return false;
        }
    };

    let stmts_vec: Vec<_> = stmts.body().iter().collect();
    let mut found_direct_plain_def = false;

    for (idx, stmt) in stmts_vec.iter().enumerate() {
        let Some(def_node) = stmt.as_def_node() else {
            continue;
        };

        // Only consider plain defs (no receiver like `def self.x`)
        if def_node.receiver().is_some() {
            continue;
        }

        let def_line = source
            .offset_to_line_col(def_node.location().start_offset())
            .0;
        if def_line == sclass_line {
            return false; // Single-line form — RuboCop does not flag
        }

        found_direct_plain_def = true;

        if direct_def_visibility(&stmts_vec, idx, def_node.name().as_slice())
            != MethodVisibility::Public
        {
            return false;
        }
    }

    found_direct_plain_def
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MethodVisibility {
    Public,
    Protected,
    Private,
}

fn direct_def_visibility(
    stmts: &[ruby_prism::Node<'_>],
    idx: usize,
    method_name: &[u8],
) -> MethodVisibility {
    inline_method_visibility(stmts, idx, method_name)
        .or_else(|| enclosing_visibility(stmts, idx))
        .unwrap_or(MethodVisibility::Public)
}

fn inline_method_visibility(
    stmts: &[ruby_prism::Node<'_>],
    idx: usize,
    method_name: &[u8],
) -> Option<MethodVisibility> {
    for stmt in stmts[idx + 1..].iter().rev() {
        let Some(call) = stmt.as_call_node() else {
            continue;
        };
        if call.receiver().is_some() {
            continue;
        }

        let visibility = visibility_name(call.name().as_slice())?;
        let Some(args) = call.arguments() else {
            continue;
        };

        if args.arguments().iter().any(|arg| {
            arg.as_symbol_node()
                .is_some_and(|symbol| symbol.unescaped() == method_name)
        }) {
            return Some(visibility);
        }
    }

    None
}

fn enclosing_visibility(stmts: &[ruby_prism::Node<'_>], idx: usize) -> Option<MethodVisibility> {
    for stmt in stmts[..idx].iter().rev() {
        let Some(call) = stmt.as_call_node() else {
            continue;
        };

        if call.receiver().is_none() && call.arguments().is_none() {
            if let Some(visibility) = visibility_name(call.name().as_slice()) {
                return Some(visibility);
            }
        }
    }

    None
}

fn visibility_name(name: &[u8]) -> Option<MethodVisibility> {
    match name {
        b"public" => Some(MethodVisibility::Public),
        b"protected" => Some(MethodVisibility::Protected),
        b"private" => Some(MethodVisibility::Private),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ClassMethodsDefinitions,
        "cops/style/class_methods_definitions"
    );
}
