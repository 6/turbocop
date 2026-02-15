use crate::cop::util::{
    self, is_rspec_example, is_rspec_example_group, is_rspec_let, RSPEC_DEFAULT_INCLUDE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct LetBeforeExamples;

impl Cop for LetBeforeExamples {
    fn name(&self) -> &'static str {
        "RSpec/LetBeforeExamples"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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

        let method_name = call.name().as_slice();

        // Check for example group calls (including ::RSpec.describe)
        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut seen_example = false;
        let mut diagnostics = Vec::new();

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let name = c.name().as_slice();
                if c.receiver().is_none() {
                    // Only count actual examples and non-shared example groups
                    // (with blocks) as "seen example". Shared groups
                    // (shared_examples, shared_context) don't count.
                    let is_example_or_group_with_block = (is_rspec_example(name)
                        || is_non_shared_example_group(name))
                        && c.block().is_some();
                    if is_example_or_group_with_block {
                        seen_example = true;
                    } else if is_example_include(name) {
                        seen_example = true;
                    } else if seen_example && is_rspec_let(name) {
                        let let_name = std::str::from_utf8(name).unwrap_or("let");
                        let loc = stmt.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Move `{let_name}` before the examples in the group."),
                        ));
                    }
                }
            }
        }

        diagnostics
    }
}

/// Check if a method name is an RSpec example group but NOT a shared group.
/// Shared groups (shared_examples, shared_examples_for, shared_context) don't
/// count as "examples seen" for the LetBeforeExamples cop because they define
/// reusable code, not actual test groups.
fn is_non_shared_example_group(name: &[u8]) -> bool {
    matches!(
        name,
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
    )
}

fn is_example_include(name: &[u8]) -> bool {
    name == b"include_examples"
        || name == b"it_behaves_like"
        || name == b"it_should_behave_like"
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LetBeforeExamples, "cops/rspec/let_before_examples");
}
