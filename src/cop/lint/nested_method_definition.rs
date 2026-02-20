use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CLASS_NODE, DEF_NODE, MODULE_NODE, SINGLETON_CLASS_NODE};

pub struct NestedMethodDefinition;

struct NestedDefFinder {
    found: Vec<usize>,
    skip_depth: usize,
    // Stack of booleans: true if the branch node was a scope-creating node
    scope_stack: Vec<bool>,
}

impl<'pr> Visit<'pr> for NestedDefFinder {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let is_scope = node.as_class_node().is_some()
            || node.as_module_node().is_some()
            || node.as_singleton_class_node().is_some()
            || is_scope_creating_call(&node);
        self.scope_stack.push(is_scope);
        if is_scope {
            self.skip_depth += 1;
        }
        if self.skip_depth == 0 {
            if let Some(def_node) = node.as_def_node() {
                // Skip singleton method definitions (def obj.method) — they define
                // a method on a specific receiver, not on the enclosing scope.
                if def_node.receiver().is_none() {
                    self.found.push(node.location().start_offset());
                }
            }
        }
    }

    fn visit_branch_node_leave(&mut self) {
        if let Some(true) = self.scope_stack.pop() {
            self.skip_depth -= 1;
        }
    }
}

/// Check if a node is a scope-creating call like Module.new, Class.new,
/// define_method, class_eval, etc. that creates a new method scope.
fn is_scope_creating_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    // Must have a block for defs inside to be in a new scope
    if call.block().is_none() {
        return false;
    }
    let method_name = call.name().as_slice();
    // Metaprogramming methods that create new scopes
    if matches!(
        method_name,
        b"define_method"
            | b"class_eval"
            | b"module_eval"
            | b"instance_eval"
            | b"class_exec"
            | b"module_exec"
            | b"instance_exec"
    ) {
        return true;
    }
    // Module.new, Class.new, Struct.new (also handles qualified like ::Module.new via constant_path_node)
    if method_name == b"new" {
        if let Some(receiver) = call.receiver() {
            if let Some(name) = crate::cop::util::constant_name(&receiver) {
                if name == b"Module" || name == b"Class" || name == b"Struct" {
                    return true;
                }
            }
        }
    }
    false
}

impl Cop for NestedMethodDefinition {
    fn name(&self) -> &'static str {
        "Lint/NestedMethodDefinition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CLASS_NODE, DEF_NODE, MODULE_NODE, SINGLETON_CLASS_NODE]
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
        let def_node = match node.as_def_node() {
            Some(n) => n,
            None => return,
        };

        // AllowedMethods: skip offense if the enclosing method name is in the list
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let method_name = std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");
        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name) {
                return;
            }
        }
        if let Some(patterns) = &allowed_patterns {
            if patterns.iter().any(|p| method_name.contains(p.as_str())) {
                return;
            }
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        let mut finder = NestedDefFinder {
            found: vec![],
            skip_depth: 0,
            scope_stack: vec![],
        };
        finder.visit(&body);

        diagnostics.extend(finder
            .found
            .iter()
            .map(|&offset| {
                let (line, column) = source.offset_to_line_col(offset);
                self.diagnostic(
                    source,
                    line,
                    column,
                    "Method definitions must not be nested. Use `lambda` instead.".to_string(),
                )
            }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedMethodDefinition, "cops/lint/nested_method_definition");
}
