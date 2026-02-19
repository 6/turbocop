use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, GLOBAL_VARIABLE_READ_NODE, INTEGER_NODE};

pub struct NumericPredicate;

impl NumericPredicate {
    fn int_value(node: &ruby_prism::Node<'_>) -> Option<i64> {
        if let Some(int_node) = node.as_integer_node() {
            let src = int_node.location().as_slice();
            if let Ok(s) = std::str::from_utf8(src) {
                return s.parse::<i64>().ok();
            }
        }
        None
    }

    fn is_global_var(node: &ruby_prism::Node<'_>) -> bool {
        node.as_global_variable_read_node().is_some()
    }
}

impl Cop for NumericPredicate {
    fn name(&self) -> &'static str {
        "Style/NumericPredicate"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, GLOBAL_VARIABLE_READ_NODE, INTEGER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "predicate");
        let _allowed_methods = config.get_string_array("AllowedMethods");
        let _allowed_patterns = config.get_string_array("AllowedPatterns");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        if enforced_style == "predicate" {
            // Check for: x == 0, x > 0, x < 0, 0 == x, 0 > x, 0 < x
            if !matches!(method_bytes, b"==" | b">" | b"<") {
                return Vec::new();
            }

            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return Vec::new();
                }

                if let Some(receiver) = call.receiver() {
                    // x == 0, x > 0, x < 0
                    if Self::int_value(&arg_list[0]) == Some(0) && !Self::is_global_var(&receiver) {
                        let recv_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("x");
                        let replacement = match method_bytes {
                            b"==" => format!("{}.zero?", recv_src),
                            b">" => format!("{}.positive?", recv_src),
                            b"<" => format!("{}.negative?", recv_src),
                            _ => return Vec::new(),
                        };
                        let loc = node.location();
                        let current = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use `{}` instead of `{}`.", replacement, current),
                        )];
                    }

                    // 0 == x, 0 > x, 0 < x (inverted)
                    if Self::int_value(&receiver) == Some(0) && !Self::is_global_var(&arg_list[0]) {
                        let arg_src = std::str::from_utf8(arg_list[0].location().as_slice()).unwrap_or("x");
                        let replacement = match method_bytes {
                            b"==" => format!("{}.zero?", arg_src),
                            b">" => format!("{}.negative?", arg_src),  // 0 > x means x is negative
                            b"<" => format!("{}.positive?", arg_src),  // 0 < x means x is positive
                            _ => return Vec::new(),
                        };
                        let loc = node.location();
                        let current = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use `{}` instead of `{}`.", replacement, current),
                        )];
                    }
                }
            }
        } else if enforced_style == "comparison" {
            // Check for: x.zero?, x.positive?, x.negative?
            if !matches!(method_bytes, b"zero?" | b"positive?" | b"negative?") {
                return Vec::new();
            }
            if call.arguments().is_some() {
                return Vec::new();
            }
            if let Some(receiver) = call.receiver() {
                let recv_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("x");
                let replacement = match method_bytes {
                    b"zero?" => format!("{} == 0", recv_src),
                    b"positive?" => format!("{} > 0", recv_src),
                    b"negative?" => format!("{} < 0", recv_src),
                    _ => return Vec::new(),
                };
                let loc = node.location();
                let current = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of `{}`.", replacement, current),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NumericPredicate, "cops/style/numeric_predicate");
}
