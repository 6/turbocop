use crate::cop::node_type::HASH_NODE;
use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for trailing commas in hash literals.
///
/// ## Heredoc handling (2026-03)
///
/// Prism reports a hash pair's `end_offset()` at the heredoc opening token
/// (for example `<<~RUBY.chomp`), not at the closing heredoc terminator. A
/// previous FP fix tried to avoid scanning heredoc bodies by starting at the
/// closing `}` line whenever a hash contained a heredoc, but that skipped the
/// real trailing comma on the heredoc opening line:
///
/// `key: <<~RUBY,`
///
/// Fix: keep scanning from the last element end offset, but stop at the first
/// newline when a heredoc is present. This matches RuboCop's heredoc-specific
/// `/\A[^\S\n]*,/` check, so commas on the heredoc opening line are found
/// without treating commas inside heredoc bodies as trailing hash commas.
///
/// Nested hash values also need heredoc recursion. Without that, an outer hash
/// whose last value is another hash containing a heredoc still scans through
/// the nested heredoc body and can mistake commas in embedded Ruby for a
/// trailing comma on the outer hash.
pub struct TrailingCommaInHashLiteral;

impl Cop for TrailingCommaInHashLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInHashLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Note: keyword_hash_node (keyword args like `foo(a: 1)`) intentionally not
        // handled — this cop only applies to trailing commas in hash literals.
        let hash_node = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let closing_loc = hash_node.closing_loc();
        let elements: Vec<ruby_prism::Node<'_>> = hash_node.elements().iter().collect();
        let last_elem = match elements.last() {
            Some(e) => e,
            None => return,
        };

        let last_end = last_elem.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();

        // For heredoc elements, Prism's location.end_offset() is at the
        // heredoc opening tag, so the heredoc body sits between last_end and
        // closing_start. Match RuboCop here: only look for a comma before the
        // first newline so we can catch `<<~RUBY,` without scanning heredoc text.
        let has_heredoc = any_heredoc(&elements);
        let has_comma = if has_heredoc {
            has_trailing_comma_no_newline(bytes, last_end, closing_start)
        } else {
            has_trailing_comma(bytes, last_end, closing_start)
        };

        let style = config.get_str("EnforcedStyleForMultiline", "no_comma");
        let last_line = source.offset_to_line_col(last_end).0;
        let close_line = source.offset_to_line_col(closing_start).0;
        let is_multiline = close_line > last_line;

        // Helper: find the absolute offset of the trailing comma for diagnostics.
        let find_comma_offset = || -> Option<usize> {
            let search_range = &bytes[last_end..closing_start];
            for (offset, &byte) in search_range.iter().enumerate() {
                if has_heredoc && byte == b'\n' {
                    return None;
                }
                if byte == b',' {
                    return Some(last_end + offset);
                }
            }
            None
        };

        match style {
            "comma" | "consistent_comma" => {
                // Require trailing comma in multiline; no opinion on single-line
                if is_multiline && !has_comma {
                    let (line, column) = source.offset_to_line_col(last_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Put a comma after the last item of a multiline hash.".to_string(),
                    ));
                }
            }
            _ => {
                // no_comma: flag trailing commas
                if has_comma {
                    if let Some(abs_offset) = find_comma_offset() {
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last item of a hash.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

/// Like `has_trailing_comma` but stops at the first newline. This matches
/// RuboCop's heredoc-specific `/\A[^\S\n]*,/` behavior so only the heredoc
/// opening line is considered when looking for a trailing comma.
fn has_trailing_comma_no_newline(
    source_bytes: &[u8],
    last_element_end: usize,
    closing_start: usize,
) -> bool {
    if last_element_end >= closing_start || closing_start > source_bytes.len() {
        return false;
    }

    let region = &source_bytes[last_element_end..closing_start];
    for &byte in region {
        if byte == b'\n' {
            return false;
        }
        if byte == b',' {
            return true;
        }
        if byte != b' ' && byte != b'\t' {
            return false;
        }
    }

    false
}

/// Returns true if any element in the hash contains a heredoc.
fn any_heredoc(elements: &[ruby_prism::Node<'_>]) -> bool {
    elements.iter().any(|e| is_heredoc_element(e))
}

/// Returns true if a node is or contains a heredoc.
/// Handles pair nodes (hash elements) by checking both key and value.
fn is_heredoc_element(node: &ruby_prism::Node<'_>) -> bool {
    // Check pair nodes (hash key-value pairs)
    if let Some(pair) = node.as_assoc_node() {
        return is_heredoc_element(&pair.value());
    }
    if let Some(hash) = node.as_hash_node() {
        return hash
            .elements()
            .iter()
            .any(|element| is_heredoc_element(&element));
    }
    // Direct string heredoc
    if let Some(s) = node.as_interpolated_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    if let Some(s) = node.as_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    // Check method calls on heredocs (e.g., <<~SQL.strip.chomp)
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if is_heredoc_element(&recv) {
                return true;
            }
        }
        // Check arguments for heredocs (e.g., method(<<RUBY))
        if let Some(args) = call.arguments() {
            if args.arguments().iter().any(|a| is_heredoc_element(&a)) {
                return true;
            }
        }
    }
    // Check array values containing heredocs (e.g., key: [method(<<RUBY)])
    if let Some(arr) = node.as_array_node() {
        return arr.elements().iter().any(|e| is_heredoc_element(&e));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        TrailingCommaInHashLiteral,
        "cops/style/trailing_comma_in_hash_literal"
    );
}
