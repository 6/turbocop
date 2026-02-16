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
                // If next line is a comment, scan forward — if there's a blank line
                // before the next code line, the example is properly separated.
                // This matches RuboCop's AST-sibling approach where comments between
                // nodes don't count as missing separation.
                {
                    let trimmed_pos = line.iter().position(|&b| b != b' ' && b != b'\t');
                    if let Some(start) = trimmed_pos {
                        if line[start] == b'#' {
                            // Scan forward past comment lines
                            let mut scan = next_line + 1;
                            loop {
                                match line_at(source, scan) {
                                    Some(l) if is_blank_line(l) => return Vec::new(),
                                    Some(l) => {
                                        let t = l.iter().position(|&b| b != b' ' && b != b'\t');
                                        if let Some(s) = t {
                                            if l[s] != b'#' {
                                                break; // reached non-comment code
                                            }
                                        }
                                    }
                                    None => return Vec::new(), // end of file
                                }
                                scan += 1;
                            }
                            // No blank line found between example and next code — fall through to report
                        }
                    }
                }
                // If consecutive one-liners are allowed, check if the next line is also a one-liner example
                if allow_consecutive && is_one_liner {
                    // Check if next line looks like an example call
                    let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
                    if let Some(start) = trimmed {
                        let rest = &line[start..];
                        if starts_with_example_keyword(rest) {
                            return Vec::new();
                        }
                    }
                }

                // Check for `end` line (last example in block)
                let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
                if let Some(start) = trimmed {
                    let rest = &line[start..];
                    if rest.starts_with(b"end") && (rest.len() == 3 || !rest[3].is_ascii_alphanumeric()) {
                        return Vec::new(); // last item before end
                    }
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
