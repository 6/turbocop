use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantMatch;

impl Cop for RedundantMatch {
    fn name(&self) -> &'static str {
        "Performance/RedundantMatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"match" {
            return Vec::new();
        }

        // Must have a receiver (x.match)
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Must have arguments (x.match(y))
        if call.arguments().is_none() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `match?` instead of `match` when `MatchData` is not used.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMatch, "cops/performance/redundant_match");
}
