use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UselessMethodDefinition;

impl Cop for UselessMethodDefinition {
    fn name(&self) -> &'static str {
        "Lint/UselessMethodDefinition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip methods with rest args, optional args, or optional keyword args.
        // These change the calling convention so `super` is not equivalent to
        // removing the method entirely.
        if let Some(params) = def_node.parameters() {
            if !params.optionals().is_empty()
                || params.rest().is_some()
                || params
                    .keywords()
                    .iter()
                    .any(|k| k.as_optional_keyword_parameter_node().is_some())
            {
                return Vec::new();
            }
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        // Check if the single statement is a `super` call
        let first = &body_nodes[0];

        // ForwardingSuperNode is `super` with implicit forwarding (no parens).
        // But skip if it has a block — `super do ... end` adds behavior.
        if let Some(fwd_super) = first.as_forwarding_super_node() {
            if fwd_super.block().is_none() {
                let loc = def_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Useless method definition detected. The method just delegates to `super`."
                        .to_string(),
                )];
            }
        }

        // SuperNode is explicit `super(args)` — only flag if args match the def's params exactly
        if let Some(super_node) = first.as_super_node() {
            if super_args_match_params(def_node.parameters(), &super_node) {
                let loc = def_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Useless method definition detected. The method just delegates to `super`."
                        .to_string(),
                )];
            }
        }

        Vec::new()
    }
}

/// Check if the super call arguments match the method parameters exactly.
/// For `def foo(a, b)` + `super(a, b)` => true.
/// For `def foo(a, b)` + `super(b, a)` or `super(a)` => false.
/// For `def foo()` + `super()` => true.
fn super_args_match_params(
    params: Option<ruby_prism::ParametersNode<'_>>,
    super_node: &ruby_prism::SuperNode<'_>,
) -> bool {
    let super_args: Vec<_> = match super_node.arguments() {
        Some(a) => a.arguments().iter().collect(),
        None => Vec::new(),
    };

    let params = match params {
        Some(p) => p,
        None => return super_args.is_empty(),
    };

    // Collect all parameter names in order
    let mut param_names: Vec<Vec<u8>> = Vec::new();

    for req in params.requireds().iter() {
        if let Some(rp) = req.as_required_parameter_node() {
            param_names.push(rp.name().as_slice().to_vec());
        } else {
            // Destructured parameter — not a simple delegation
            return false;
        }
    }

    for kw in params.keywords().iter() {
        if let Some(kp) = kw.as_required_keyword_parameter_node() {
            param_names.push(kp.name().as_slice().to_vec());
        } else {
            // Optional keyword args are already filtered out above, but be safe
            return false;
        }
    }

    if super_args.len() != param_names.len() {
        return false;
    }

    // Check each super arg is a local variable read matching the param name in order
    for (arg, expected_name) in super_args.iter().zip(param_names.iter()) {
        if let Some(lvar) = arg.as_local_variable_read_node() {
            if lvar.name().as_slice() != expected_name.as_slice() {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessMethodDefinition, "cops/lint/useless_method_definition");
}
