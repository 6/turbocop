use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct UnusedMethodArgument;

impl Cop for UnusedMethodArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedMethodArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let ignore_empty = config.get_bool("IgnoreEmptyMethods", true);
        let ignore_not_implemented = config.get_bool("IgnoreNotImplementedMethods", true);
        let allow_unused_keyword = config.get_bool("AllowUnusedKeywordArguments", false);
        let _not_implemented_exceptions = config.get_string_array("NotImplementedExceptions");

        let body = match def_node.body() {
            Some(b) => b,
            None => {
                // Empty method
                if ignore_empty {
                    return Vec::new();
                }
                // Fall through to check params with no body
                return Vec::new();
            }
        };

        // Check for not-implemented methods
        if ignore_not_implemented && is_not_implemented(&body) {
            return Vec::new();
        }

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Collect parameter info: (name_bytes, offset, is_keyword)
        let mut param_info: Vec<(Vec<u8>, usize, bool)> = Vec::new();

        for req in params.requireds().iter() {
            if let Some(rp) = req.as_required_parameter_node() {
                param_info.push((
                    rp.name().as_slice().to_vec(),
                    rp.location().start_offset(),
                    false,
                ));
            }
        }

        for opt in params.optionals().iter() {
            if let Some(op) = opt.as_optional_parameter_node() {
                param_info.push((
                    op.name().as_slice().to_vec(),
                    op.location().start_offset(),
                    false,
                ));
            }
        }

        if !allow_unused_keyword {
            for kw in params.keywords().iter() {
                if let Some(kp) = kw.as_required_keyword_parameter_node() {
                    param_info.push((
                        kp.name().as_slice().to_vec(),
                        kp.location().start_offset(),
                        true,
                    ));
                } else if let Some(kp) = kw.as_optional_keyword_parameter_node() {
                    param_info.push((
                        kp.name().as_slice().to_vec(),
                        kp.location().start_offset(),
                        true,
                    ));
                }
            }
        }

        if param_info.is_empty() {
            return Vec::new();
        }

        // Find all local variable reads in the body AND in parameter defaults.
        // A parameter used as a default value for another parameter counts as used
        // (e.g., `def foo(node, start = node)` — `node` is used in default of `start`).
        let mut finder = VarReadFinder {
            names: Vec::new(),
            has_forwarding_super: false,
            has_binding_call: false,
        };
        finder.visit(&body);

        // Also scan parameter default values for variable reads
        for opt in params.optionals().iter() {
            if let Some(op) = opt.as_optional_parameter_node() {
                finder.visit(&op.value());
            }
        }
        for kw in params.keywords().iter() {
            if let Some(kp) = kw.as_optional_keyword_parameter_node() {
                finder.visit(&kp.value());
            }
        }

        // If the body contains bare `super` (ForwardingSuperNode), all args are
        // implicitly forwarded and therefore "used".
        if finder.has_forwarding_super {
            return Vec::new();
        }

        // If the body calls `binding`, all local variables are accessible via
        // `binding.local_variable_get`, so consider all args as used.
        if finder.has_binding_call {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        for (name, offset, _is_keyword) in &param_info {
            // Skip arguments prefixed with _
            if name.starts_with(b"_") {
                continue;
            }

            // Check if the variable is referenced in the body
            if !finder.names.iter().any(|n| n == name) {
                let (line, column) = source.offset_to_line_col(*offset);
                // For keyword args, strip trailing ':'
                let display_name = if *_is_keyword {
                    let s = String::from_utf8_lossy(name);
                    s.trim_end_matches(':').to_string()
                } else {
                    String::from_utf8_lossy(name).to_string()
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Unused method argument - `{display_name}`."),
                ));
            }
        }

        diagnostics
    }
}

fn is_not_implemented(body: &ruby_prism::Node<'_>) -> bool {
    // Check if body is a single `raise NotImplementedError` or `fail "..."` statement
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => {
            // Could be a direct call node
            return check_not_implemented_call(body);
        }
    };

    let body_nodes: Vec<_> = stmts.body().iter().collect();
    if body_nodes.len() != 1 {
        return false;
    }

    check_not_implemented_call(&body_nodes[0])
}

fn check_not_implemented_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let name = call.name().as_slice();
    if call.receiver().is_some() {
        return false;
    }

    if name == b"raise" {
        // Check if argument is NotImplementedError
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if !arg_list.is_empty() {
                if let Some(c) = arg_list[0].as_constant_read_node() {
                    return c.name().as_slice() == b"NotImplementedError";
                }
                // Also handle qualified constant like ::NotImplementedError
                if let Some(cp) = arg_list[0].as_constant_path_node() {
                    if let Some(name) = cp.name() {
                        return name.as_slice() == b"NotImplementedError";
                    }
                }
            }
        }
        false
    } else if name == b"fail" {
        true
    } else {
        false
    }
}

struct VarReadFinder {
    names: Vec<Vec<u8>>,
    has_forwarding_super: bool,
    has_binding_call: bool,
}

impl<'pr> Visit<'pr> for VarReadFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
    }

    // Also check for local variable usage as call receiver or in other contexts
    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
    }

    // Bare `super` (no args, no parens) implicitly forwards all method arguments
    fn visit_forwarding_super_node(
        &mut self,
        _node: &ruby_prism::ForwardingSuperNode<'pr>,
    ) {
        self.has_forwarding_super = true;
    }

    // Detect `binding` calls — accessing binding exposes all local variables
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.receiver().is_none()
            && node.name().as_slice() == b"binding"
            && node.arguments().is_none()
        {
            self.has_binding_call = true;
        }
        ruby_prism::visit_call_node(self, node);
    }

    // Don't recurse into nested def/class/module (they have their own scope)
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedMethodArgument, "cops/lint/unused_method_argument");
}
