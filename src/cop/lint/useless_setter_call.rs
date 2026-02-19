use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, DEF_NODE, LOCAL_VARIABLE_READ_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct UselessSetterCall;

impl Cop for UselessSetterCall {
    fn name(&self) -> &'static str {
        "Lint/UselessSetterCall"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE, LOCAL_VARIABLE_READ_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Get the last expression in the method body
        // If body is a StatementsNode, get the last statement.
        // Otherwise the body itself is the expression.
        let stmts_opt = body.as_statements_node();
        let body_stmts: Vec<ruby_prism::Node<'_>> = if let Some(stmts) = stmts_opt {
            stmts.body().iter().collect()
        } else {
            vec![body]
        };
        let last_expr = match body_stmts.last() {
            Some(e) => e,
            None => return,
        };

        // Check if the last expression is a setter call on a local variable
        let call = match last_expr.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a setter method (name ends with `=`)
        let method_name = call.name().as_slice();
        if !method_name.ends_with(b"=")
            || method_name == b"=="
            || method_name == b"!="
            || method_name == b"<="
            || method_name == b">="
            || method_name == b"[]="
        {
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

        // Don't flag setter calls on method parameters — the object
        // persists after the method returns, so the setter has real effect.
        if let Some(params) = def_node.parameters() {
            let is_param = params.requireds().iter().any(|p| {
                p.as_required_parameter_node()
                    .is_some_and(|rp| rp.name().as_slice() == var_name_bytes)
            }) || params.optionals().iter().any(|p| {
                p.as_optional_parameter_node()
                    .is_some_and(|op| op.name().as_slice() == var_name_bytes)
            });
            if is_param {
                return;
            }
        }

        let loc = last_expr.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Useless setter call to local variable `{var_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessSetterCall, "cops/lint/useless_setter_call");
}
