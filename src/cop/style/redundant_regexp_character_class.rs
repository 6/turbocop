use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::REGULAR_EXPRESSION_NODE;

pub struct RedundantRegexpCharacterClass;

impl Cop for RedundantRegexpCharacterClass {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpCharacterClass"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[REGULAR_EXPRESSION_NODE]
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
        let re = match node.as_regular_expression_node() {
            Some(re) => re,
            None => return,
        };

        let content_bytes: Vec<u8> = re.content_loc().as_slice().to_vec();
        let node_loc = node.location();
        let is_extended = re.is_extended();

        let full_bytes = &source.as_bytes()[node_loc.start_offset()..node_loc.end_offset()];

        // Find single-element character classes like [a], [\d], etc.
        let mut i = 0;
        while i < content_bytes.len() {
            if content_bytes[i] == b'[' && (i == 0 || content_bytes[i - 1] != b'\\') {
                // Check if it's a negated class [^...] — skip those
                let start = i;
                i += 1;
                if i < content_bytes.len() && content_bytes[i] == b'^' {
                    // Negated character class, skip
                    while i < content_bytes.len() && content_bytes[i] != b']' {
                        if content_bytes[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    i += 1;
                    continue;
                }

                // Count elements in the character class
                let mut elem_count = 0;
                let mut has_range = false;
                let class_start = i;
                while i < content_bytes.len() && content_bytes[i] != b']' {
                    if content_bytes[i] == b'\\' {
                        i += 1; // skip escaped char
                        elem_count += 1;
                    } else if content_bytes[i] == b'-' && elem_count > 0 {
                        has_range = true;
                    } else {
                        elem_count += 1;
                    }
                    i += 1;
                }

                if elem_count == 1 && !has_range && i < content_bytes.len() {
                    // Single element character class — redundant
                    // Find the position in the source
                    let inner = &content_bytes[class_start..i];
                    // Check it's not a special class like [\b] (word boundary in char class = backspace)
                    if inner == b"\\b" {
                        i += 1;
                        continue;
                    }

                    // In extended mode (/x), whitespace in a character class is NOT
                    // redundant because bare whitespace is ignored outside char classes
                    if is_extended && inner.len() == 1 && inner[0].is_ascii_whitespace() {
                        i += 1;
                        continue;
                    }

                    // Calculate offset relative to the node's opening
                    let open_len = if full_bytes.starts_with(b"%r") { 3 } else { 1 };
                    let abs_offset = node_loc.start_offset() + open_len + start;
                    let (line, column) = source.offset_to_line_col(abs_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Redundant single-element character class, `[x]` can be replaced with `x`.".to_string(),
                    ));
                }
                i += 1;
            } else {
                i += 1;
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRegexpCharacterClass, "cops/style/redundant_regexp_character_class");
}
