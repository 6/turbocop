use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::STRING_NODE;

pub struct RedundantPercentQ;

impl Cop for RedundantPercentQ {
    fn name(&self) -> &'static str {
        "Style/RedundantPercentQ"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let opening_loc = match string_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let opening = opening_loc.as_slice();

        if opening.starts_with(b"%q") {
            // %q string — check if it contains both single and double quotes
            let raw_content = string_node.content_loc().as_slice();
            let has_single = raw_content.contains(&b'\'');
            let has_double = raw_content.contains(&b'"');
            // Check for escape sequences other than \\ — if present, %q is justified
            let has_escape = has_non_backslash_escape(raw_content);
            // Check for string interpolation pattern #{...} — user likely chose %q
            // to avoid interpolation; this matches vendor behavior
            let has_interpolation_pattern = contains_interpolation_pattern(raw_content);

            if has_escape || has_interpolation_pattern || (has_single && has_double) {
                return Vec::new();
            }

            let loc = string_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `%q` only for strings that contain both single quotes and double quotes."
                    .to_string(),
            )];
        }

        if opening.starts_with(b"%Q") {
            // %Q string — acceptable if it contains double quotes (would need escaping in "")
            let raw_content = string_node.content_loc().as_slice();
            let has_double = raw_content.contains(&b'"');

            if has_double {
                return Vec::new();
            }

            let loc = string_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

/// Check if raw content contains escape sequences other than just \\
fn has_non_backslash_escape(raw: &[u8]) -> bool {
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' && i + 1 < raw.len() {
            if raw[i + 1] != b'\\' {
                return true;
            }
            i += 2; // skip \\
        } else {
            i += 1;
        }
    }
    false
}

/// Check if content contains a string interpolation pattern `#{...}`
fn contains_interpolation_pattern(raw: &[u8]) -> bool {
    raw.windows(2).any(|w| w == b"#{")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantPercentQ, "cops/style/redundant_percent_q");
}
