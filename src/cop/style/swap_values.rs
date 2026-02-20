use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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
            if let (Some(w1), Some(w2), Some(w3)) = (
                window[0].as_local_variable_write_node(),
                window[1].as_local_variable_write_node(),
                window[2].as_local_variable_write_node(),
            ) {
                let tmp_name = w1.name().as_slice();
                let val1 = w1.value();
                let a_value = get_lvar_name(&val1);
                let b_name = w2.name().as_slice();
                let val2 = w2.value();
                let b_value = get_lvar_name(&val2);
                let c_name = w3.name().as_slice();
                let val3 = w3.value();
                let c_value = get_lvar_name(&val3);

                if let (Some(a_val), Some(b_val), Some(c_val)) = (a_value, b_value, c_value) {
                    // Pattern: tmp = a; a = b; b = tmp
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
            }
        }

        ruby_prism::visit_statements_node(self, node);
    }
}

fn get_lvar_name<'a>(node: &'a ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(lv) = node.as_local_variable_read_node() {
        Some(lv.name().as_slice())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SwapValues, "cops/style/swap_values");
}
