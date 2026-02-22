use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BigDecimalWithNumericArgument;

impl Cop for BigDecimalWithNumericArgument {
    fn name(&self) -> &'static str {
        "Performance/BigDecimalWithNumericArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if method_name == b"BigDecimal" {
            // BigDecimal() is a Kernel method, so no receiver
            if call.receiver().is_some() {
                return;
            }

            let arguments = match call.arguments() {
                Some(a) => a,
                None => return,
            };

            let args = arguments.arguments();
            if args.is_empty() {
                return;
            }

            // RuboCop only flags:
            // - Float arguments: BigDecimal(1.2) → BigDecimal('1.2')
            // - String arguments containing only digits: BigDecimal('1') → BigDecimal(1)
            // Integer arguments like BigDecimal(0) are NOT flagged.
            let first_arg = match args.iter().next() {
                Some(a) => a,
                None => return,
            };

            if let ruby_prism::Node::FloatNode { .. } = first_arg {
                let loc = first_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Convert float literal to string and pass it to `BigDecimal`.".to_string(),
                ));
            } else if let Some(str_node) = first_arg.as_string_node() {
                let content = str_node.unescaped();
                // Only flag string integers like BigDecimal('1'), not BigDecimal('1.5')
                if !content.is_empty() && content.iter().all(|b| b.is_ascii_digit()) {
                    let loc = first_arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            line,
                            column,
                            "Convert string literal to integer and pass it to `BigDecimal`."
                                .to_string(),
                        ),
                    );
                }
            }
        } else if method_name == b"to_d" {
            // receiver.to_d — flag float receivers and string receivers with digit-only content
            let receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            if let ruby_prism::Node::FloatNode { .. } = receiver {
                let loc = receiver.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Convert float literal to string and pass it to `BigDecimal`.".to_string(),
                ));
            } else if let Some(str_node) = receiver.as_string_node() {
                let content = str_node.unescaped();
                if !content.is_empty() && content.iter().all(|b| b.is_ascii_digit()) {
                    let loc = receiver.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            line,
                            column,
                            "Convert string literal to integer and pass it to `BigDecimal`."
                                .to_string(),
                        ),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        BigDecimalWithNumericArgument,
        "cops/performance/big_decimal_with_numeric_argument"
    );
}
