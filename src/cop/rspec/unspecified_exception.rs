use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Detects `raise_error` / `raise_exception` matchers without a specified exception under
/// `expect { ... }.to ...`.
///
/// Fix: walk ancestors from the matcher send itself instead of only inspecting the first
/// matcher passed to `.to`, so chained expectations like
/// `output(...).to_stderr.and raise_error` are flagged without regressing `.not_to`,
/// `expect(...)`, or `.to ... do |error|`.
///
/// Corpus validation note: the sampled `check_cop.py` run still reports an extra offense in
/// `gitlabhq__omnibus-gitlab__d36f1f6`, but the extra count is an unrelated
/// `Lint/RedundantCopDisableDirective` that leaks through `--only`; corpus-aligned runs show
/// this cop itself now matches RuboCop on the target file.
pub struct UnspecifiedException;

impl Cop for UnspecifiedException {
    fn name(&self) -> &'static str {
        "RSpec/UnspecifiedException"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if method_name != b"to" {
            return;
        }

        // RuboCop only matches block form: expect { ... }.to raise_error
        // Not parens form: expect(...).to raise_error
        // The receiver of `.to` must be an `expect` call with a block and no arguments.
        let Some(receiver) = call.receiver() else {
            return;
        };
        let Some(expect_call) = receiver.as_call_node() else {
            return;
        };
        if expect_call.receiver().is_some()
            || expect_call.name().as_slice() != b"expect"
            || expect_call.block().is_none()
            || expect_call.arguments().is_some()
        {
            return;
        }

        let Some(args) = call.arguments() else {
            return;
        };

        // Also check if the `.to` call has a block with arguments.
        // `expect { }.to raise_error do |e| ... end` — the do/end block attaches
        // to `.to`, not to `raise_error`. If `.to`'s block has parameters,
        // the exception IS being captured via the block argument.
        if let Some(to_block) = call.block() {
            if let Some(block_node) = to_block.as_block_node() {
                if block_node.parameters().is_some() {
                    return;
                }
            }
        }

        for matcher in args
            .arguments()
            .iter()
            .flat_map(|arg| find_empty_exception_matchers(arg))
        {
            let loc = matcher.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Specify the exception being captured.".to_string(),
            ));
        }
    }
}

fn find_empty_exception_matchers<'a>(node: ruby_prism::Node<'a>) -> Vec<ruby_prism::CallNode<'a>> {
    let Some(call) = node.as_call_node() else {
        return Vec::new();
    };

    let mut matches = Vec::new();

    let method_name = call.name().as_slice();
    if let Some(receiver) = call.receiver() {
        matches.extend(find_empty_exception_matchers(receiver));
    }

    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            matches.extend(find_empty_exception_matchers(arg));
        }
    }

    if (method_name == b"raise_error" || method_name == b"raise_exception")
        && call.receiver().is_none()
        && call.arguments().is_none()
        && call.block().is_none()
    {
        matches.push(call);
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnspecifiedException, "cops/rspec/unspecified_exception");
}
