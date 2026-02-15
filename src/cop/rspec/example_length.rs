use crate::cop::util::{self, is_rspec_example, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExampleLength;

impl Cop for ExampleLength {
    fn name(&self) -> &'static str {
        "RSpec/ExampleLength"
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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if !is_rspec_example(method_name) {
            return Vec::new();
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 5);

        // Count body lines, skipping blank lines (RuboCop default behavior).
        // count_body_lines counts non-blank, non-comment lines between start and end.
        let block_loc = block.location();
        let count = util::count_body_lines(
            source,
            block_loc.start_offset(),
            block_loc.end_offset().saturating_sub(1).max(block_loc.start_offset()),
            true, // count_comments = true (comments count as lines)
        );

        // Adjust for CountAsOne: multi-line arrays/hashes/heredocs count as 1 line
        let count_as_one = config
            .get_string_array("CountAsOne")
            .unwrap_or_default();
        let adjusted = if !count_as_one.is_empty() {
            let reduction = count_multiline_reductions(source, &block, &count_as_one);
            count.saturating_sub(reduction)
        } else {
            count
        };

        if adjusted > max {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                format!("Example has too many lines. [{adjusted}/{max}]"),
            )]
        } else {
            Vec::new()
        }
    }
}

/// Count how many extra lines multi-line constructs add.
/// For each multi-line array/hash/heredoc, returns (span - 1) so they count as 1 line.
fn count_multiline_reductions(
    source: &SourceFile,
    block: &ruby_prism::BlockNode<'_>,
    count_as_one: &[String],
) -> usize {
    let body = match block.body() {
        Some(b) => b,
        None => return 0,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return 0,
    };

    let mut reduction = 0;
    for stmt in stmts.body().iter() {
        reduction += count_node_reduction(source, &stmt, count_as_one);
    }
    reduction
}

fn count_node_reduction(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    count_as_one: &[String],
) -> usize {
    let mut reduction = 0;

    if count_as_one.iter().any(|s| s == "array") {
        if let Some(arr) = node.as_array_node() {
            let span = node_line_span(source, &arr.location());
            if span > 1 {
                reduction += span - 1;
            }
            return reduction;
        }
    }

    if count_as_one.iter().any(|s| s == "hash") {
        if let Some(hash) = node.as_hash_node() {
            let span = node_line_span(source, &hash.location());
            if span > 1 {
                reduction += span - 1;
            }
            return reduction;
        }
    }

    if count_as_one.iter().any(|s| s == "heredoc") {
        if node.as_interpolated_string_node().is_some() || node.as_string_node().is_some() {
            let span = node_line_span(source, &node.location());
            if span > 1 {
                reduction += span - 1;
            }
            return reduction;
        }
    }

    reduction
}

fn node_line_span(source: &SourceFile, loc: &ruby_prism::Location<'_>) -> usize {
    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
    let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_off);
    end_line.saturating_sub(start_line) + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExampleLength, "cops/rspec/example_length");
}
