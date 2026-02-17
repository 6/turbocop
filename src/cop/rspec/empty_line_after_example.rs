use crate::cop::util::{is_blank_line, is_rspec_example, line_at, node_on_single_line, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterExample;

impl Cop for EmptyLineAfterExample {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterExample"
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
        if call.receiver().is_some() || !is_rspec_example(method_name) {
            return Vec::new();
        }

        // RuboCop's EmptyLineAfterExample uses `on_block` — it only fires on example
        // calls that have a block (do..end or { }).  Bare calls like `skip('reason')`
        // inside a `before` block, or `scenario` used as a variable-like method from
        // `let(:scenario)`, are not example declarations and must be ignored.
        if call.block().is_none() {
            return Vec::new();
        }

        let allow_consecutive = config.get_bool("AllowConsecutiveOneLiners", true);

        // Determine the end line of this example
        let loc = node.location();
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);

        let is_one_liner = node_on_single_line(source, &loc);

        // Check if the next non-blank line is another node
        let next_line = end_line + 1;
        let next_content = line_at(source, next_line);
        match next_content {
            Some(line) => {
                if is_blank_line(line) {
                    return Vec::new(); // already has blank line
                }

                // Determine the effective "check line" — skip past comments to find
                // the first non-comment, non-blank line.  If a blank line or EOF is
                // encountered while scanning comments, the example is properly
                // separated and we return early.
                let check_line = if is_comment_line(line) {
                    let mut scan = next_line + 1;
                    loop {
                        match line_at(source, scan) {
                            Some(l) if is_blank_line(l) => return Vec::new(),
                            Some(l) if is_comment_line(l) => {}
                            Some(l) => break l,
                            None => return Vec::new(), // end of file
                        }
                        scan += 1;
                    }
                } else {
                    line
                };

                // If consecutive one-liners are allowed, check if the next
                // meaningful line is also a one-liner example.
                // Both the current AND next example must be one-liners.
                if allow_consecutive && is_one_liner {
                    let trimmed = check_line.iter().position(|&b| b != b' ' && b != b'\t');
                    if let Some(start) = trimmed {
                        let rest = &check_line[start..];
                        if starts_with_example_keyword(rest) && is_single_line_block(rest) {
                            return Vec::new();
                        }
                    }
                }

                // Check for terminator keywords (last example before closing
                // construct).  RuboCop uses `last_child?` on the AST; we
                // approximate by recognising `end`, `else`, `elsif`, `when`,
                // `rescue`, `ensure`, and `in` (pattern matching).
                if is_terminator_line(check_line) {
                    return Vec::new();
                }
            }
            None => return Vec::new(), // end of file
        }

        // Report on the end line of the example
        let method_str = std::str::from_utf8(method_name).unwrap_or("it");
        let report_col_actual = if is_one_liner {
            let (_, start_col) = source.offset_to_line_col(loc.start_offset());
            start_col
        } else {
            // For multi-line, report at the `end` keyword column
            if let Some(line_bytes) = line_at(source, end_line) {
                line_bytes.iter().take_while(|&&b| b == b' ').count()
            } else {
                0
            }
        };

        vec![self.diagnostic(
            source,
            end_line,
            report_col_actual,
            format!("Add an empty line after `{method_str}`."),
        )]
    }
}

/// Returns true if the trimmed line starts with `#`.
fn is_comment_line(line: &[u8]) -> bool {
    let trimmed_pos = line.iter().position(|&b| b != b' ' && b != b'\t');
    matches!(trimmed_pos, Some(start) if line[start] == b'#')
}

/// Check if a line is a block/construct terminator — i.e. the example is
/// the last child before the closing keyword.
fn is_terminator_line(line: &[u8]) -> bool {
    let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
    if let Some(start) = trimmed {
        let rest = &line[start..];
        for keyword in &[
            b"end" as &[u8], b"else", b"elsif", b"when", b"rescue", b"ensure", b"in ",
        ] {
            if rest.starts_with(keyword) {
                // Ensure keyword isn't part of a longer identifier
                if rest.len() == keyword.len()
                    || !rest[keyword.len()].is_ascii_alphanumeric() && rest[keyword.len()] != b'_'
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a line represents a single-line block (contains closing `end` or `}` on same line).
fn is_single_line_block(line: &[u8]) -> bool {
    // Single-line brace block: `it { something }`
    if line.contains(&b'{') && line.contains(&b'}') {
        return true;
    }
    // Single-line do..end: `it "foo" do something end` (very rare but possible)
    // Check if line contains both `do` and `end`
    if line.windows(2).any(|w| w == b"do") && line.windows(3).any(|w| w == b"end") {
        return true;
    }
    false
}

fn starts_with_example_keyword(line: &[u8]) -> bool {
    for keyword in &[b"it " as &[u8], b"it(", b"it{", b"it {",
                      b"specify ", b"specify(", b"specify{", b"specify {",
                      b"example ", b"example(", b"example{", b"example {",
                      b"scenario ", b"scenario("] {
        if line.starts_with(keyword) {
            return true;
        }
    }
    // Single-line `it { ... }`
    if line.starts_with(b"it ") || line == b"it" {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineAfterExample, "cops/rspec/empty_line_after_example");
}
