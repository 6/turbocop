use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for cases where exceptions from numeric constructors like `Integer()`,
/// `Float()`, etc. may be unintentionally swallowed using `rescue nil`.
pub struct SuppressedExceptionInNumberConversion;

const NUMERIC_METHODS: &[&[u8]] = &[b"Integer", b"Float", b"BigDecimal", b"Complex", b"Rational"];

impl Cop for SuppressedExceptionInNumberConversion {
    fn name(&self) -> &'static str {
        "Lint/SuppressedExceptionInNumberConversion"
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
    ) -> Vec<Diagnostic> {
        let mut visitor = NumConvVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct NumConvVisitor<'a, 'src> {
    cop: &'a SuppressedExceptionInNumberConversion,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for NumConvVisitor<'_, '_> {
    fn visit_rescue_modifier_node(&mut self, node: &ruby_prism::RescueModifierNode<'pr>) {
        // This handles: Integer(arg) rescue nil
        let expression = node.expression();

        if is_numeric_constructor(&expression) {
            // Check if rescue value is nil
            let rescue_expr = node.rescue_expression();
            if rescue_expr.as_nil_node().is_some() {
                let call = expression.as_call_node().unwrap();
                let prefer = build_preferred(&call, self.source);
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("Use `{}` instead.", prefer),
                ));
            }
        }

        ruby_prism::visit_rescue_modifier_node(self, node);
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // Handle: begin; Integer(arg); rescue; nil; end
        if let Some(rescue_node) = node.rescue_clause() {
            if let Some(stmts) = node.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() == 1 && is_numeric_constructor(&body[0]) {
                    // Check if rescue body is nil or empty
                    let is_nil_rescue = is_rescue_nil_or_empty(&rescue_node);
                    if is_nil_rescue {
                        let call = body[0].as_call_node().unwrap();
                        let prefer = build_preferred(&call, self.source);
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            format!("Use `{}` instead.", prefer),
                        ));
                    }
                }
            }
        }

        ruby_prism::visit_begin_node(self, node);
    }
}

fn is_numeric_constructor(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let method_name = call.name().as_slice();
    if !NUMERIC_METHODS.iter().any(|m| *m == method_name) {
        return false;
    }

    // Must be receiverless or Kernel.Method
    if let Some(recv) = call.receiver() {
        if let Some(name) = crate::cop::util::constant_name(&recv) {
            if name != b"Kernel" {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn build_preferred(call: &ruby_prism::CallNode<'_>, source: &SourceFile) -> String {
    let method_name =
        std::str::from_utf8(call.name().as_slice()).unwrap_or("Integer");
    let mut args_parts: Vec<String> = Vec::new();

    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            let src = &source.as_bytes()[arg.location().start_offset()..arg.location().end_offset()];
            args_parts.push(std::str::from_utf8(src).unwrap_or("arg").to_string());
        }
    }
    args_parts.push("exception: false".to_string());

    format!("{}({})", method_name, args_parts.join(", "))
}

fn is_rescue_nil_or_empty(rescue_node: &ruby_prism::RescueNode<'_>) -> bool {
    match rescue_node.statements() {
        None => true, // empty rescue
        Some(stmts) => {
            let body: Vec<_> = stmts.body().iter().collect();
            body.len() == 1 && body[0].as_nil_node().is_some()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SuppressedExceptionInNumberConversion,
        "cops/lint/suppressed_exception_in_number_conversion"
    );
}
