use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ActionFilter;

const FILTER_METHODS: &[(&[u8], &[u8])] = &[
    (b"after_filter", b"after_action"),
    (b"append_after_filter", b"append_after_action"),
    (b"append_around_filter", b"append_around_action"),
    (b"append_before_filter", b"append_before_action"),
    (b"around_filter", b"around_action"),
    (b"before_filter", b"before_action"),
    (b"prepend_after_filter", b"prepend_after_action"),
    (b"prepend_around_filter", b"prepend_around_action"),
    (b"prepend_before_filter", b"prepend_before_action"),
    (b"skip_after_filter", b"skip_after_action"),
    (b"skip_around_filter", b"skip_around_action"),
    (b"skip_before_filter", b"skip_before_action"),
    (b"skip_filter", b"skip_action_callback"),
];

impl Cop for ActionFilter {
    fn name(&self) -> &'static str {
        "Rails/ActionFilter"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/controllers/**/*.rb", "**/app/mailers/**/*.rb"]
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
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless
        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        let style = config.get_str("EnforcedStyle", "action");

        let (current, prefer) = if style == "action" {
            // Bad: filter methods; Good: action methods
            match FILTER_METHODS.iter().find(|(filter, _)| *filter == name) {
                Some((filter, action)) => {
                    (std::str::from_utf8(filter).unwrap(), std::str::from_utf8(action).unwrap())
                }
                None => return,
            }
        } else {
            // Bad: action methods; Good: filter methods
            match FILTER_METHODS.iter().find(|(_, action)| *action == name) {
                Some((filter, action)) => {
                    (std::str::from_utf8(action).unwrap(), std::str::from_utf8(filter).unwrap())
                }
                None => return,
            }
        };

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{prefer}` over `{current}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActionFilter, "cops/rails/action_filter");
}
