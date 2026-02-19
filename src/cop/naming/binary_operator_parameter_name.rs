use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEF_NODE, REQUIRED_PARAMETER_NODE};

pub struct BinaryOperatorParameterName;

const BINARY_OPERATORS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**",
    b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>",
    b"&", b"|", b"^", b">>",
    b"eql?", b"equal?",
];

// Operators excluded from this cop per RuboCop: +@ -@ [] []= << === ` =~
const EXCLUDED_OPERATORS: &[&[u8]] = &[
    b"+@", b"-@", b"[]", b"[]=", b"<<", b"===", b"`", b"=~",
];

impl Cop for BinaryOperatorParameterName {
    fn name(&self) -> &'static str {
        "Naming/BinaryOperatorParameterName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, REQUIRED_PARAMETER_NODE]
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

        // Skip singleton methods (def self.foo, def obj.foo) — RuboCop only
        // handles :def, not :defs
        if def_node.receiver().is_some() {
            return Vec::new();
        }

        let method_name = def_node.name().as_slice();

        // Skip excluded operators
        if EXCLUDED_OPERATORS.iter().any(|&op| op == method_name) {
            return Vec::new();
        }

        // Check if this is a binary operator or operator-like method
        if !BINARY_OPERATORS.iter().any(|&op| op == method_name) {
            // Also accept non-word methods (operators) that aren't excluded
            let name_str = std::str::from_utf8(method_name).unwrap_or("");
            let is_op = !name_str.is_empty() && !name_str.starts_with(|c: char| c.is_alphanumeric() || c == '_');
            if !is_op {
                return Vec::new();
            }
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
            // Accept both `other` and `_other` as valid names
            if param_name != b"other" && param_name != b"_other" {
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
