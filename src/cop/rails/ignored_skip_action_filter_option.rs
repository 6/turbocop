use crate::cop::node_type::CALL_NODE;
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IgnoredSkipActionFilterOption;

const SKIP_METHODS: &[&[u8]] = &[
    b"skip_after_action",
    b"skip_around_action",
    b"skip_before_action",
    b"skip_action_callback",
];

impl Cop for IgnoredSkipActionFilterOption {
    fn name(&self) -> &'static str {
        "Rails/IgnoredSkipActionFilterOption"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless skip_*_action call
        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        if !SKIP_METHODS.contains(&name) {
            return;
        }

        // Check for keyword arguments
        let has_if = keyword_arg_value(&call, b"if").is_some();
        let has_only = keyword_arg_value(&call, b"only").is_some();
        let has_except = keyword_arg_value(&call, b"except").is_some();

        if has_if && has_only {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "`if` option will be ignored when `only` and `if` are used together.".to_string(),
            ));
        }

        if has_if && has_except {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(
                self.diagnostic(
                    source,
                    line,
                    column,
                    "`except` option will be ignored when `if` and `except` are used together."
                        .to_string(),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IgnoredSkipActionFilterOption,
        "cops/rails/ignored_skip_action_filter_option"
    );
}
