use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE};

pub struct MatchArray;

impl Cop for MatchArray {
    fn name(&self) -> &'static str {
        "RSpec/MatchArray"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Detect `match_array([...])` with a non-empty array literal argument.
        // Suggest `contain_exactly` instead.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"match_array" || call.receiver().is_some() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let array_node = match arg_list[0].as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Don't flag empty arrays (that's BeEmpty's job)
        if array_node.elements().iter().count() == 0 {
            return Vec::new();
        }

        // Don't flag percent literals (%w, %i, %W, %I) — RuboCop skips these
        // because they can't be splatted into contain_exactly arguments.
        // Percent literals have opening_loc starting with '%'.
        if let Some(open) = array_node.opening_loc() {
            let open_bytes = open.as_slice();
            if open_bytes.starts_with(b"%") {
                return Vec::new();
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `contain_exactly` when matching an array literal.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MatchArray, "cops/rspec/match_array");
}
