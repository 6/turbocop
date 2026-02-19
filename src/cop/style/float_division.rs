use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct FloatDivision;

impl FloatDivision {
    fn is_to_f_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"to_f" && call.receiver().is_some() {
                // Make sure it has no arguments (not an implicit receiver call)
                if call.arguments().is_none() {
                    return true;
                }
            }
        }
        false
    }
}

impl Cop for FloatDivision {
    fn name(&self) -> &'static str {
        "Style/FloatDivision"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"/" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let left_is_to_f = Self::is_to_f_call(&receiver);
        let right_is_to_f = Self::is_to_f_call(&arg_list[0]);

        if !left_is_to_f && !right_is_to_f {
            return Vec::new();
        }

        let style = config.get_str("EnforcedStyle", "single_coerce");

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        match style {
            "single_coerce" => {
                if left_is_to_f && right_is_to_f {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer using `.to_f` on one side only.".to_string(),
                    )];
                }
            }
            "left_coerce" => {
                if right_is_to_f {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer using `.to_f` on the left side.".to_string(),
                    )];
                }
            }
            "right_coerce" => {
                if left_is_to_f {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer using `.to_f` on the right side.".to_string(),
                    )];
                }
            }
            "fdiv" => {
                if left_is_to_f || right_is_to_f {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer using `fdiv` for float divisions.".to_string(),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FloatDivision, "cops/style/float_division");
}
