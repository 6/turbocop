use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NumberedParametersLimit;

impl Cop for NumberedParametersLimit {
    fn name(&self) -> &'static str {
        "Style/NumberedParametersLimit"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let max = config.get_usize("Max", 1);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Must have no explicit parameters
        if block_node.parameters().is_some() {
            return Vec::new();
        }

        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let body_src = std::str::from_utf8(body.location().as_slice()).unwrap_or("");

        // Find the highest numbered parameter used
        let mut highest = 0;
        for i in 1..=9 {
            let param = format!("_{i}");
            if body_src.contains(&param) {
                highest = i;
            }
        }

        if highest > max {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Avoid using more than {max} numbered parameters; {highest} detected."),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NumberedParametersLimit, "cops/style/numbered_parameters_limit");
}
