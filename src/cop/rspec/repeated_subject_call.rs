use crate::cop::util::{is_rspec_example, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/RepeatedSubjectCall: Flag calling `subject` multiple times in the same example
/// when at least one call is inside a block (expect { subject }).
pub struct RepeatedSubjectCall;

impl Cop for RepeatedSubjectCall {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedSubjectCall"
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
        // Look for example blocks (it/specify/etc.)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();
        if !is_rspec_example(name) {
            return Vec::new();
        }
        if call.receiver().is_some() {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Find all `subject` calls in the example body, tracking which are in blocks
        let mut subject_calls: Vec<(usize, usize, bool)> = Vec::new(); // (line, col, is_in_block)
        collect_subject_calls(source, &body, false, &mut subject_calls);

        if subject_calls.len() <= 1 {
            return Vec::new();
        }

        // Only flag if at least one call is inside a block
        let has_block_call = subject_calls.iter().any(|(_, _, in_block)| *in_block);
        if !has_block_call {
            return Vec::new();
        }

        // Flag all block calls after the first subject reference
        let mut diagnostics = Vec::new();
        let mut seen_first = false;
        for &(line, col, in_block) in &subject_calls {
            if !seen_first {
                seen_first = true;
                continue;
            }
            if in_block {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    "Calls to subject are memoized, this block is misleading".to_string(),
                ));
            }
        }

        diagnostics
    }
}

fn collect_subject_calls(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    in_block: bool,
    results: &mut Vec<(usize, usize, bool)>,
) {
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            collect_subject_calls(source, &stmt, in_block, results);
        }
        return;
    }

    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();

        // Check if this is a bare `subject` call (no receiver, no args)
        if name == b"subject" && call.receiver().is_none() {
            // Check if subject is used as a block argument (expect { subject })
            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            results.push((line, col, in_block));
        }

        // Check receiver for nested subject calls
        if let Some(recv) = call.receiver() {
            collect_subject_calls(source, &recv, in_block, results);
        }

        // Check arguments
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_subject_calls(source, &arg, in_block, results);
            }
        }

        // Check the call's own block
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    collect_subject_calls(source, &body, true, results);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedSubjectCall, "cops/rspec/repeated_subject_call");
}
