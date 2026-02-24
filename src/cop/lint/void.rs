use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct Void;

impl Cop for Void {
    fn name(&self) -> &'static str {
        "Lint/Void"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _check_methods = config.get_bool("CheckForMethodsWithNoSideEffects", false);

        let mut visitor = VoidVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct VoidVisitor<'a, 'src> {
    cop: &'a Void,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

fn is_void_expression(node: &ruby_prism::Node<'_>) -> bool {
    // Simple literals
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_self_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        // Interpolated literals
        || node.as_interpolated_string_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        // Variable reads
        || node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
        // Constants
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
        // Containers
        || node.as_range_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        // Keywords
        || node.as_source_file_node().is_some()
        || node.as_source_line_node().is_some()
        || node.as_source_encoding_node().is_some()
        // defined?
        || node.as_defined_node().is_some()
        // Operators (binary/unary) via CallNode
        || is_void_operator(node)
}

fn is_void_operator(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        matches!(
            name,
            b"+" | b"-"
                | b"*"
                | b"/"
                | b"%"
                | b"**"
                | b"=="
                | b"!="
                | b"<"
                | b">"
                | b"<="
                | b">="
                | b"<=>"
                | b"<<"
                | b">>"
                | b"&"
                | b"|"
                | b"^"
                | b"!"
                | b"~"
                | b"-@"
                | b"+@"
        )
    } else {
        false
    }
}

impl<'pr> Visit<'pr> for VoidVisitor<'_, '_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let body: Vec<_> = node.body().iter().collect();
        if body.len() > 1 {
            // All statements except the last are in void context
            for stmt in &body[..body.len() - 1] {
                if is_void_expression(stmt) {
                    let loc = stmt.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Void value expression detected.".to_string(),
                    ));
                }
            }
        }
        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Void, "cops/lint/void");
}
