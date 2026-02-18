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

        // In Prism, blocks with numbered params have parameters() set to a
        // NumberedParametersNode which has a maximum() method returning the
        // highest numbered parameter used. This avoids false positives from
        // string matching _1.._9 in comments, strings, or variable names.
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let numbered = match params.as_numbered_parameters_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let highest = numbered.maximum() as usize;

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
