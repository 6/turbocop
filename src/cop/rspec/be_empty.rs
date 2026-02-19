use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE};

pub struct BeEmpty;

impl Cop for BeEmpty {
    fn name(&self) -> &'static str {
        "RSpec/BeEmpty"
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
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Look for `.to contain_exactly` (no args) or `.to match_array([])`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];
        let matcher_call = match first_arg.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if matcher_call.receiver().is_some() {
            return;
        }

        let matcher_name = matcher_call.name().as_slice();

        let is_offense = if matcher_name == b"contain_exactly" {
            // `contain_exactly` with no arguments
            matcher_call.arguments().is_none()
        } else if matcher_name == b"match_array" {
            // `match_array([])` — match_array with empty array literal
            if let Some(matcher_args) = matcher_call.arguments() {
                let matcher_arg_list: Vec<_> = matcher_args.arguments().iter().collect();
                if matcher_arg_list.len() == 1 {
                    if let Some(array_node) = matcher_arg_list[0].as_array_node() {
                        array_node.elements().iter().count() == 0
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if is_offense {
            let loc = matcher_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `be_empty` matchers for checking an empty array.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BeEmpty, "cops/rspec/be_empty");
}
