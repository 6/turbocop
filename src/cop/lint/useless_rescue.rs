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
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RescueVisitor<'a, 'src> {
    cop: &'a UselessRescue,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RescueVisitor<'_, '_> {
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // Check if this is the last rescue clause (no subsequent rescue)
        // RescueNode has a `subsequent` which is the next rescue clause
        if node.subsequent().is_none() {
            // This is the last rescue clause
            if only_reraising(node, self.source) {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Useless `rescue` detected.".to_string(),
                ));
            }
        }

        // Continue visiting children
        ruby_prism::visit_rescue_node(self, node);
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
    if let Some(ref_node) = rescue_node.reference() {
        let ref_src = source.byte_slice(
            ref_node.location().start_offset(),
            ref_node.location().end_offset(),
            "",
        );
        // The reference includes the `=> ` prefix in some cases, extract the variable name
        let var_name = ref_src.trim_start_matches("=> ").trim();
        if !var_name.is_empty() && arg_src == var_name {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessRescue, "cops/lint/useless_rescue");
}
