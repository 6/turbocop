use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Enforces the use of shorthand-style swapping of 2 variables.
///
/// Supports local, instance, class, and global variable swaps (matching
/// RuboCop's `SIMPLE_ASSIGNMENT_TYPES`). FN fix: previously only detected
/// local variable swaps; instance variable swaps like
/// `tmp = @server; @server = @server2; @server2 = tmp` were missed.
pub struct SwapValues;

impl Cop for SwapValues {
    fn name(&self) -> &'static str {
        "Style/SwapValues"
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
        let mut visitor = SwapVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SwapVisitor<'a> {
    cop: &'a SwapValues,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for SwapVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let stmts: Vec<_> = node.body().iter().collect();

        for window in stmts.windows(3) {
            // Pattern: tmp = a; a = b; b = tmp
            // All three must be simple variable writes (local, instance, class, or global).
            let Some((tmp_name, val1)) = get_write_info(&window[0]) else {
                continue;
            };
            let Some((b_name, val2)) = get_write_info(&window[1]) else {
                continue;
            };
            let Some((c_name, val3)) = get_write_info(&window[2]) else {
                continue;
            };

            let Some(a_val) = get_var_name(&val1) else {
                continue;
            };
            let Some(b_val) = get_var_name(&val2) else {
                continue;
            };
            let Some(c_val) = get_var_name(&val3) else {
                continue;
            };

            // w1: tmp_name = a_val  (save a into tmp)
            // w2: b_name = b_val    (assign b's value to a)
            // w3: c_name = c_val    (restore tmp into b)
            // Conditions:
            //   b_name == a_val (second writes to the saved variable)
            //   c_name == b_val (third writes to the source variable)
            //   c_val == tmp_name (third reads from temp)
            //   b_val != tmp_name (second doesn't read temp)
            if b_name == a_val && c_name == b_val && c_val == tmp_name && b_val != tmp_name {
                let loc = window[0].location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Replace this swap with `{}, {} = {}, {}`.",
                        String::from_utf8_lossy(b_name),
                        String::from_utf8_lossy(c_name),
                        String::from_utf8_lossy(c_name),
                        String::from_utf8_lossy(b_name),
                    ),
                ));
            }
        }

        ruby_prism::visit_statements_node(self, node);
    }
}

/// Extract (name, value) from any simple variable write node.
fn get_write_info<'a>(node: &'a ruby_prism::Node<'a>) -> Option<(&'a [u8], ruby_prism::Node<'a>)> {
    if let Some(n) = node.as_local_variable_write_node() {
        Some((n.name().as_slice(), n.value()))
    } else if let Some(n) = node.as_instance_variable_write_node() {
        Some((n.name().as_slice(), n.value()))
    } else if let Some(n) = node.as_class_variable_write_node() {
        Some((n.name().as_slice(), n.value()))
    } else if let Some(n) = node.as_global_variable_write_node() {
        Some((n.name().as_slice(), n.value()))
    } else {
        None
    }
}

/// Extract the variable name from any simple variable read node.
fn get_var_name<'a>(node: &'a ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(lv) = node.as_local_variable_read_node() {
        Some(lv.name().as_slice())
    } else if let Some(iv) = node.as_instance_variable_read_node() {
        Some(iv.name().as_slice())
    } else if let Some(cv) = node.as_class_variable_read_node() {
        Some(cv.name().as_slice())
    } else if let Some(gv) = node.as_global_variable_read_node() {
        Some(gv.name().as_slice())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SwapValues, "cops/style/swap_values");
}
