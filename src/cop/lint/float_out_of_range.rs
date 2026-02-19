use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::FLOAT_NODE;

pub struct FloatOutOfRange;

impl Cop for FloatOutOfRange {
    fn name(&self) -> &'static str {
        "Lint/FloatOutOfRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[FLOAT_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let float_node = match node.as_float_node() {
            Some(n) => n,
            None => return,
        };

        let loc = float_node.location();
        let src = loc.as_slice();

        // Remove underscores and parse as f64
        let cleaned: Vec<u8> = src.iter().copied().filter(|&b| b != b'_').collect();
        let text = match std::str::from_utf8(&cleaned) {
            Ok(t) => t,
            Err(_) => return,
        };

        match text.parse::<f64>() {
            Ok(val) if val.is_infinite() => {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Float out of range.".to_string(),
                ));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FloatOutOfRange, "cops/lint/float_out_of_range");
}
