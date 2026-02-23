use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for useless `rescue` blocks that only re-raise the exception.
pub struct UselessRescue;

impl Cop for UselessRescue {
    fn name(&self) -> &'static str {
        "Lint/UselessRescue"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut visitor = RescueVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            ensure_var_names: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RescueVisitor<'a, 'src> {
    cop: &'a UselessRescue,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Local variable names referenced in the current ensure context.
    /// When visiting inside a begin/def node with an ensure clause,
    /// this contains variable names used in the ensure body.
    ensure_var_names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for RescueVisitor<'_, '_> {
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // If this begin has an ensure clause, collect local variable names used in it
        let prev_len = self.ensure_var_names.len();
        if let Some(ensure_clause) = node.ensure_clause() {
            collect_ensure_lvar_names(ensure_clause, &mut self.ensure_var_names);
        }

        ruby_prism::visit_begin_node(self, node);

        self.ensure_var_names.truncate(prev_len);
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // Check if this is the last rescue clause (no subsequent rescue)
        // RescueNode has a `subsequent` which is the next rescue clause
        if node.subsequent().is_none()
            && only_reraising(node, self.source)
            && !self.exception_var_used_in_ensure(node)
        {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Useless `rescue` detected.".to_string(),
            ));
        }

        // Continue visiting children
        ruby_prism::visit_rescue_node(self, node);
    }
}

impl RescueVisitor<'_, '_> {
    fn exception_var_used_in_ensure(&self, rescue_node: &ruby_prism::RescueNode<'_>) -> bool {
        if self.ensure_var_names.is_empty() {
            return false;
        }

        if let Some(reference) = rescue_node.reference() {
            if let Some(local_var) = reference.as_local_variable_target_node() {
                let var_name = local_var.name().as_slice();
                return self.ensure_var_names.iter().any(|n| n == var_name);
            }
        }

        false
    }
}

/// Collect all local variable read names from an ensure clause's body.
fn collect_ensure_lvar_names(ensure_clause: ruby_prism::EnsureNode<'_>, names: &mut Vec<Vec<u8>>) {
    struct LvarCollector<'a> {
        names: &'a mut Vec<Vec<u8>>,
    }
    impl<'pr> Visit<'pr> for LvarCollector<'_> {
        fn visit_local_variable_read_node(
            &mut self,
            node: &ruby_prism::LocalVariableReadNode<'pr>,
        ) {
            self.names.push(node.name().as_slice().to_vec());
        }
    }
    let mut collector = LvarCollector { names };
    if let Some(statements) = ensure_clause.statements() {
        for stmt in statements.body().iter() {
            collector.visit(&stmt);
        }
    }
}

fn only_reraising(rescue_node: &ruby_prism::RescueNode<'_>, source: &SourceFile) -> bool {
    let statements = match rescue_node.statements() {
        Some(s) => s,
        None => return false,
    };

    let body: Vec<_> = statements.body().iter().collect();
    if body.len() != 1 {
        return false;
    }

    let stmt = &body[0];

    // Check if it's a `raise` call
    let call = match stmt.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.name().as_slice() != b"raise" {
        return false;
    }

    // Must not have a receiver
    if call.receiver().is_some() {
        return false;
    }

    // `raise` with no args => re-raises current exception
    let args = match call.arguments() {
        Some(a) => a,
        None => return true, // bare `raise`
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return false;
    }

    let first_arg = &arg_list[0];
    let arg_src = source.byte_slice(
        first_arg.location().start_offset(),
        first_arg.location().end_offset(),
        "",
    );

    // Check if it's re-raising the same exception variable
    if arg_src == "$!" || arg_src == "$ERROR_INFO" {
        return true;
    }

    // Check if it matches the rescue variable name
    if let Some(reference) = rescue_node.reference() {
        if let Some(local_var) = reference.as_local_variable_target_node() {
            let var_name = std::str::from_utf8(local_var.name().as_slice()).unwrap_or("");
            if !var_name.is_empty() && arg_src == var_name {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessRescue, "cops/lint/useless_rescue");
}
