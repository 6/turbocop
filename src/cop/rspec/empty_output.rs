use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyOutput;

impl Cop for EmptyOutput {
    fn name(&self) -> &'static str {
        "RSpec/EmptyOutput"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        // Match:
        //   expect { ... }.to output('').to_stdout
        //   expect { ... }.not_to output('').to_stderr
        // and ignore bare matcher DSL calls like `output ''` in helper contexts.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let runner = call.name().as_slice();
        if runner != b"to" && runner != b"not_to" && runner != b"to_not" {
            return;
        }

        // Receiver must be a block expectation: expect { ... }
        let expect_call = match call.receiver().and_then(|r| r.as_call_node()) {
            Some(c) => c,
            None => return,
        };
        if expect_call.name().as_slice() != b"expect"
            || expect_call.receiver().is_some()
            || expect_call.block().is_none()
        {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // RuboCop pattern matches only when the first argument is a matcher chain
        // directly rooted at `output('')`, e.g. `output('').to_stdout`.
        let matcher_chain = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        let output_call = match matcher_chain.receiver().and_then(|r| r.as_call_node()) {
            Some(c) => c,
            None => return,
        };
        if output_call.name().as_slice() != b"output" || output_call.receiver().is_some() {
            return;
        }
        let output_args = match output_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let output_arg_list: Vec<_> = output_args.arguments().iter().collect();
        if output_arg_list.len() != 1 {
            return;
        }
        if !output_arg_list[0]
            .as_string_node()
            .is_some_and(|s| s.unescaped().is_empty())
        {
            return;
        }

        let preferred_runner = if runner == b"to" { "not_to" } else { "to" };
        let loc = output_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{preferred_runner}` instead of matching on an empty output."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyOutput, "cops/rspec/empty_output");
}
