use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BinaryOperatorParameterName;

const BINARY_OPERATORS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**",
    b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>",
    b"&", b"|", b"^", b"<<", b">>",
    b"===", b"=~",
];

impl Cop for BinaryOperatorParameterName {
    fn name(&self) -> &'static str {
        "Naming/BinaryOperatorParameterName"
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

        let method_name = def_node.name().as_slice();
        if !BINARY_OPERATORS.iter().any(|&op| op == method_name) {
            return Vec::new();
        }

        // Skip [] and []= (indexer methods)
        if method_name == b"[]" || method_name == b"[]=" {
            return Vec::new();
        }

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let requireds = params.requireds();
        if requireds.is_empty() {
            return Vec::new();
        }

        let first_param = &requireds.iter().next().unwrap();
        if let Some(req) = first_param.as_required_parameter_node() {
            let param_name = req.name().as_slice();
            if param_name != b"other" {
                let loc = req.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let op_str = std::str::from_utf8(method_name).unwrap_or("");
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "When defining the `{op_str}` operator, name its argument `other`."
                    ),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        BinaryOperatorParameterName,
        "cops/naming/binary_operator_parameter_name"
    );
}
