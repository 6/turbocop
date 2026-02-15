use ruby_prism::Visit;

/// A sorted list of non-code byte ranges (comments, strings, regexps, symbols).
///
/// Used by `check_source` cops to skip non-code regions when scanning raw bytes,
/// avoiding false positives on commas/semicolons/etc inside strings or comments.
pub struct CodeMap {
    /// Sorted, non-overlapping (start, end) byte ranges of non-code regions.
    ranges: Vec<(usize, usize)>,
}

impl CodeMap {
    /// Build a CodeMap from a parse result, collecting non-code regions from
    /// comments, string literals, regular expressions, symbols, and heredocs.
    pub fn from_parse_result(
        _source: &[u8],
        parse_result: &ruby_prism::ParseResult<'_>,
    ) -> Self {
        let mut ranges = Vec::new();

        // Collect comment ranges
        for comment in parse_result.comments() {
            let loc = comment.location();
            ranges.push((loc.start_offset(), loc.end_offset()));
        }

        // Walk AST to collect string/regex/symbol ranges
        let mut collector = NonCodeCollector {
            ranges: &mut ranges,
        };
        collector.visit(&parse_result.node());

        // Sort and merge overlapping ranges
        ranges.sort_unstable();
        let merged = merge_ranges(ranges);

        CodeMap { ranges: merged }
    }

    /// Returns true if the given byte offset is in "code" (not inside a
    /// comment, string, regexp, or symbol literal).
    pub fn is_code(&self, offset: usize) -> bool {
        // Binary search for a range that contains this offset
        self.ranges
            .binary_search_by(|&(start, end)| {
                if offset < start {
                    std::cmp::Ordering::Greater
                } else if offset >= end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_err()
    }
}

struct NonCodeCollector<'a> {
    ranges: &'a mut Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for NonCodeCollector<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.collect_from_node(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.collect_from_node(&node);
    }
}

impl NonCodeCollector<'_> {
    fn collect_from_node(&mut self, node: &ruby_prism::Node<'_>) {
        // Collect the full range of string/regex/symbol nodes.
        // This marks the entire literal (including delimiters) as non-code.
        match node {
            ruby_prism::Node::StringNode { .. } => {
                let sn = node.as_string_node().unwrap();
                let loc = node.location();
                self.ranges.push((loc.start_offset(), loc.end_offset()));
                // For heredocs, the location only covers the opening delimiter (<<~FOO).
                // We need to also cover the content and closing terminator.
                if let Some(open) = sn.opening_loc() {
                    if open.as_slice().starts_with(b"<<") {
                        let content_loc = sn.content_loc();
                        if let Some(close) = sn.closing_loc() {
                            self.ranges.push((content_loc.start_offset(), close.end_offset()));
                        } else {
                            self.ranges.push((content_loc.start_offset(), content_loc.end_offset()));
                        }
                    }
                }
            }
            ruby_prism::Node::InterpolatedStringNode { .. } => {
                let isn = node.as_interpolated_string_node().unwrap();
                let loc = node.location();
                self.ranges.push((loc.start_offset(), loc.end_offset()));
                // For heredocs, also cover the content parts and closing terminator.
                if let Some(open) = isn.opening_loc() {
                    if open.as_slice().starts_with(b"<<") {
                        let parts = isn.parts();
                        if !parts.is_empty() {
                            let first_start = parts.iter().next().unwrap().location().start_offset();
                            if let Some(close) = isn.closing_loc() {
                                self.ranges.push((first_start, close.end_offset()));
                            } else {
                                let last = parts.iter().last().unwrap();
                                self.ranges.push((first_start, last.location().end_offset()));
                            }
                        }
                    }
                }
            }
            ruby_prism::Node::RegularExpressionNode { .. }
            | ruby_prism::Node::InterpolatedRegularExpressionNode { .. }
            | ruby_prism::Node::XStringNode { .. }
            | ruby_prism::Node::InterpolatedXStringNode { .. }
            | ruby_prism::Node::SymbolNode { .. }
            | ruby_prism::Node::InterpolatedSymbolNode { .. } => {
                let loc = node.location();
                self.ranges.push((loc.start_offset(), loc.end_offset()));
            }
            _ => {}
        }
    }
}

/// Merge sorted, possibly overlapping ranges into non-overlapping ranges.
fn merge_ranges(sorted: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in sorted {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_source;

    #[test]
    fn empty_source() {
        let source = b"";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);
        assert!(cm.ranges.is_empty());
    }

    #[test]
    fn comments_are_non_code() {
        let source = b"x = 1 # comment\ny = 2\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        // "x" at offset 0 is code
        assert!(cm.is_code(0));
        // "#" starts at offset 6
        assert!(!cm.is_code(6));
        // "c" in comment
        assert!(!cm.is_code(8));
        // "y" at start of next line is code
        let y_offset = source.iter().position(|&b| b == b'y').unwrap();
        assert!(cm.is_code(y_offset));
    }

    #[test]
    fn strings_are_non_code() {
        let source = b"x = \"hello, world\"\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        // "x" is code
        assert!(cm.is_code(0));
        // The comma inside the string is NOT code
        let comma_offset = source.iter().position(|&b| b == b',').unwrap();
        assert!(!cm.is_code(comma_offset));
    }

    #[test]
    fn regex_is_non_code() {
        let source = b"x = /a,b/\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        let comma_offset = source.iter().position(|&b| b == b',').unwrap();
        assert!(!cm.is_code(comma_offset));
    }

    #[test]
    fn code_between_strings() {
        let source = b"a = \"x\", \"y\"\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        // The comma between the two strings IS code
        // Find the comma that's between the strings
        let comma_offset = source
            .windows(2)
            .position(|w| w == b"\",")
            .unwrap()
            + 1;
        assert!(cm.is_code(comma_offset));
    }

    #[test]
    fn is_code_at_boundaries() {
        let source = b"# comment\nx = 1\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        // Offset 0 = '#', start of comment — non-code
        assert!(!cm.is_code(0));
        // 'x' on the next line is code
        assert!(cm.is_code(10));
    }

    #[test]
    fn merge_overlapping_ranges() {
        let merged = merge_ranges(vec![(0, 5), (3, 8), (10, 15)]);
        assert_eq!(merged, vec![(0, 8), (10, 15)]);
    }

    #[test]
    fn merge_adjacent_ranges() {
        let merged = merge_ranges(vec![(0, 5), (5, 10)]);
        assert_eq!(merged, vec![(0, 10)]);
    }

    #[test]
    fn merge_no_overlap() {
        let merged = merge_ranges(vec![(0, 3), (5, 8)]);
        assert_eq!(merged, vec![(0, 3), (5, 8)]);
    }

    #[test]
    fn heredoc_content_is_non_code() {
        let source = b"x = <<~FOO\n  font-weight: 500;\nFOO\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        // The semicolon inside the heredoc is NOT code
        let semi_offset = source.iter().position(|&b| b == b';').unwrap();
        assert!(!cm.is_code(semi_offset), "Semicolon inside heredoc should be non-code, offset={semi_offset}");
    }

    #[test]
    fn heredoc_with_method_is_non_code() {
        let source = b"x = <<~FOO.squish\n  font-weight: 500;\nFOO\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        let semi_offset = source.iter().position(|&b| b == b';').unwrap();
        assert!(!cm.is_code(semi_offset), "Semicolon inside heredoc.squish should be non-code, offset={semi_offset}");
    }

    #[test]
    fn symbol_is_non_code() {
        let source = b"x = :\"hello, world\"\n";
        let pr = parse_source(source);
        let cm = CodeMap::from_parse_result(source, &pr);

        let comma_offset = source.iter().position(|&b| b == b',').unwrap();
        assert!(!cm.is_code(comma_offset));
    }
}
