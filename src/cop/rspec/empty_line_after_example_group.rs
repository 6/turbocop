use crate::cop::util::{is_blank_line, is_rspec_example_group, line_at, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterExampleGroup;

impl Cop for EmptyLineAfterExampleGroup {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterExampleGroup"
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
        if call.receiver().is_some() || !is_rspec_example_group(method_name) {
            return Vec::new();
        }

        // Must have a block (multi-line group)
        if call.block().is_none() {
            return Vec::new();
        }

        let loc = node.location();
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);

        // Check if the next line is blank
        let next_line = end_line + 1;
        let next_content = line_at(source, next_line);
        match next_content {
            Some(line) => {
                if is_blank_line(line) {
                    return Vec::new();
                }

                // Check for `end` line (last item before end)
                let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
                if let Some(start) = trimmed {
                    let rest = &line[start..];
                    if rest.starts_with(b"end") && (rest.len() == 3 || !rest[3].is_ascii_alphanumeric()) {
                        return Vec::new();
                    }
                }
            }
            None => return Vec::new(),
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("describe");
        // Report at the `end` keyword line
        let report_col = if let Some(line_bytes) = line_at(source, end_line) {
            line_bytes.iter().take_while(|&&b| b == b' ').count()
        } else {
            0
        };

        vec![self.diagnostic(
            source,
            end_line,
            report_col,
            format!("Add an empty line after `{method_str}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineAfterExampleGroup, "cops/rspec/empty_line_after_example_group");
}
