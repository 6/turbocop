use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UselessSetterCall;

impl Cop for UselessSetterCall {
    fn name(&self) -> &'static str {
        "Lint/UselessSetterCall"
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

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
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
            None => return Vec::new(),
        };

        // Check if the last expression is a setter call on a local variable
        let call = match last_expr.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
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
            return Vec::new();
        }

        // Receiver must be a local variable
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let lv = match recv.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return Vec::new(),
        };

        let var_name = std::str::from_utf8(lv.name().as_slice()).unwrap_or("var");

        let loc = last_expr.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Useless setter call to local variable `{var_name}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessSetterCall, "cops/lint/useless_setter_call");
}
