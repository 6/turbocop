use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct SpaceAroundOperators;

/// Collect byte offsets of `=` signs that are part of parameter defaults,
/// and byte ranges of operator method names in `def` statements.
struct ExclusionCollector {
    /// Byte offsets of `=` in default parameter positions.
    default_param_offsets: HashSet<usize>,
    /// Byte ranges (start..end) of operator method names in `def` statements.
    /// e.g., `def ==(other)` — the `==` is a method name, not an operator.
    def_method_name_ranges: Vec<std::ops::Range<usize>>,
}

impl<'pr> Visit<'pr> for ExclusionCollector {
    fn visit_optional_parameter_node(&mut self, node: &ruby_prism::OptionalParameterNode<'pr>) {
        let op_loc = node.operator_loc();
        self.default_param_offsets.insert(op_loc.start_offset());
    }

    fn visit_optional_keyword_parameter_node(
        &mut self,
        _node: &ruby_prism::OptionalKeywordParameterNode<'pr>,
    ) {
        // Keyword params use `:` not `=`, so nothing to exclude.
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice();
        // Check if the method name contains operator characters that this cop checks
        let is_operator_name = name.contains(&b'=') || name.contains(&b'!') || name.contains(&b'>');
        if is_operator_name {
            let loc = node.name_loc();
            self.def_method_name_ranges
                .push(loc.start_offset()..loc.end_offset());
        }
        // Recurse into the body to find nested defs and default params
        ruby_prism::visit_def_node(self, node);
    }
}

impl Cop for SpaceAroundOperators {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundOperators"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_for_alignment = config.get_bool("AllowForAlignment", true);
        let enforced_style_exponent =
            config.get_str("EnforcedStyleForExponentOperator", "no_space");
        let _enforced_style_rational =
            config.get_str("EnforcedStyleForRationalLiterals", "no_space");

        // Collect default parameter `=` offsets and operator method name ranges
        let mut collector = ExclusionCollector {
            default_param_offsets: HashSet::new(),
            def_method_name_ranges: Vec::new(),
        };
        collector.visit(&parse_result.node());
        let default_param_offsets = collector.default_param_offsets;
        let def_name_ranges = collector.def_method_name_ranges;

        // AST-based check for binary operators (+, -, *, /, %, &, |, ^, <<, >>,
        // <, >, <=, >=, <=>) and logical operators (&&, ||).
        let mut op_checker = BinaryOperatorChecker {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            has_corrections: corrections.is_some(),
            exponent_no_space: enforced_style_exponent == "no_space",
            allow_for_alignment,
        };
        op_checker.visit(&parse_result.node());
        diagnostics.extend(op_checker.diagnostics);
        if let Some(ref mut corr) = corrections {
            corr.extend(op_checker.corrections);
        }

        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        // Helper closure: check if offset `pos` falls within any operator method name range
        let in_def_name = |pos: usize| -> bool {
            def_name_ranges.iter().any(|r| r.contains(&pos))
        };

        while i < len {
            if !code_map.is_code(i) {
                i += 1;
                continue;
            }

            // Check for multi-char operators first: ==, !=, =>
            if i + 1 < len && code_map.is_code(i + 1) {
                let two = &bytes[i..i + 2];
                if two == b"==" || two == b"!=" || two == b"=>" {
                    // Skip ===
                    if two == b"==" && i + 2 < len && bytes[i + 2] == b'=' {
                        i += 3;
                        continue;
                    }

                    // Skip `=>` that is part of `<=>` (spaceship operator):
                    // if byte at i is `=` and i-1 is `<`, this is `<=>` not `=>`
                    if two == b"=>" && i > 0 && bytes[i - 1] == b'<' {
                        i += 2;
                        continue;
                    }

                    // Skip operator method names: `def ==(other)`, `def !=(other)`
                    if in_def_name(i) {
                        i += 2;
                        continue;
                    }

                    // Skip method calls via `.` or `&.`: e.g., `x&.!= y`, `x.== y`
                    if i > 0 && bytes[i - 1] == b'.' {
                        i += 2;
                        continue;
                    }

                    let op_str = std::str::from_utf8(two).unwrap_or("??");
                    let space_before = i > 0 && bytes[i - 1] == b' ';
                    let space_after = i + 2 < len && bytes[i + 2] == b' ';
                    let newline_after =
                        i + 2 >= len || bytes[i + 2] == b'\n' || bytes[i + 2] == b'\r';
                    if !space_before || (!space_after && !newline_after) {
                        let (line, column) = source.offset_to_line_col(i);
                        let mut diag = self.diagnostic(
                            source, line, column,
                            format!("Surrounding space missing for operator `{op_str}`."),
                        );
                        if let Some(ref mut corr) = corrections {
                            if !space_before {
                                corr.push(crate::correction::Correction {
                                    start: i, end: i, replacement: " ".to_string(),
                                    cop_name: self.name(), cop_index: 0,
                                });
                            }
                            if !space_after && !newline_after {
                                corr.push(crate::correction::Correction {
                                    start: i + 2, end: i + 2, replacement: " ".to_string(),
                                    cop_name: self.name(), cop_index: 0,
                                });
                            }
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                    i += 2;
                    continue;
                }
            }

            // Single = (not ==, !=, =>, =~, <=, >=, or part of +=/-=/etc.)
            if bytes[i] == b'=' {
                // Skip =~ and =>
                if i + 1 < len && (bytes[i + 1] == b'~' || bytes[i + 1] == b'>') {
                    i += 2;
                    continue;
                }
                // Skip ==
                if i + 1 < len && bytes[i + 1] == b'=' {
                    i += 2;
                    continue;
                }
                // Skip if preceded by !, <, >, =, +, -, *, /, %, &, |, ^, ~
                if i > 0 {
                    let prev = bytes[i - 1];
                    if matches!(
                        prev,
                        b'!' | b'<'
                            | b'>'
                            | b'='
                            | b'+'
                            | b'-'
                            | b'*'
                            | b'/'
                            | b'%'
                            | b'&'
                            | b'|'
                            | b'^'
                            | b'~'
                    ) {
                        i += 1;
                        continue;
                    }
                }

                // Skip default parameter `=` signs (handled by SpaceAroundEqualsInParameterDefault)
                if default_param_offsets.contains(&i) {
                    i += 1;
                    continue;
                }

                // Skip `=` that is part of an operator method name: `def []=`, `def ===`
                if in_def_name(i) {
                    i += 1;
                    continue;
                }

                let space_before = i > 0 && bytes[i - 1] == b' ';
                let space_after = i + 1 < len && bytes[i + 1] == b' ';
                let newline_after =
                    i + 1 >= len || bytes[i + 1] == b'\n' || bytes[i + 1] == b'\r';
                if !space_before || (!space_after && !newline_after) {
                    let (line, column) = source.offset_to_line_col(i);
                    let mut diag = self.diagnostic(
                        source, line, column,
                        "Surrounding space missing for operator `=`.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        if !space_before {
                            corr.push(crate::correction::Correction {
                                start: i, end: i, replacement: " ".to_string(),
                                cop_name: self.name(), cop_index: 0,
                            });
                        }
                        if !space_after && !newline_after {
                            corr.push(crate::correction::Correction {
                                start: i + 1, end: i + 1, replacement: " ".to_string(),
                                cop_name: self.name(), cop_index: 0,
                            });
                        }
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                i += 1;
                continue;
            }

            i += 1;
        }

    }
}

const BINARY_OPERATORS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"%", b"**",
    b"&", b"|", b"^", b"<<", b">>",
    b"<", b">", b"<=", b">=", b"<=>",
];

struct BinaryOperatorChecker<'a> {
    cop: &'a SpaceAroundOperators,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    has_corrections: bool,
    exponent_no_space: bool,
    allow_for_alignment: bool,
}

impl BinaryOperatorChecker<'_> {
    /// Check if the same operator text appears at the same byte column on an
    /// adjacent non-empty, non-comment line. Uses a two-pass approach matching
    /// RuboCop's `PrecedingFollowingAlignment`:
    /// - Pass 1: check the closest non-blank, non-comment line in each direction
    /// - Pass 2: search for a line with the same indentation as the operator line
    fn is_aligned_with_adjacent(&self, start: usize, op_bytes: &[u8]) -> bool {
        let bytes = self.source.as_bytes();

        // Compute byte column (distance from start of line)
        let mut ls = start;
        while ls > 0 && bytes[ls - 1] != b'\n' {
            ls -= 1;
        }
        let byte_col = start - ls;

        let lines: Vec<&[u8]> = self.source.lines().collect();
        let (line, _) = self.source.offset_to_line_col(start);
        let line_idx = line - 1; // 0-indexed

        // Pass 1: closest non-blank, non-comment line (no indentation filter)
        if self.check_alignment_any_direction(&lines, line_idx, byte_col, op_bytes, None) {
            return true;
        }

        // Pass 2: search for same-indentation lines further out
        let my_indent = lines[line_idx]
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .unwrap_or(0);
        self.check_alignment_any_direction(&lines, line_idx, byte_col, op_bytes, Some(my_indent))
    }

    /// Check both directions for alignment, optionally filtering by indentation.
    fn check_alignment_any_direction(
        &self,
        lines: &[&[u8]],
        line_idx: usize,
        byte_col: usize,
        op_bytes: &[u8],
        indent_filter: Option<usize>,
    ) -> bool {
        for up in [true, false] {
            let mut check_idx = if up {
                if line_idx == 0 {
                    continue;
                }
                line_idx - 1
            } else {
                line_idx + 1
            };

            loop {
                if check_idx >= lines.len() {
                    break;
                }

                let line_bytes = lines[check_idx];
                let first_non_ws =
                    line_bytes.iter().position(|&b| b != b' ' && b != b'\t');

                match first_non_ws {
                    None => {
                        // Empty line — skip
                    }
                    Some(fs) if line_bytes[fs] == b'#' => {
                        // Comment line — skip
                    }
                    Some(indent) => {
                        if let Some(required) = indent_filter {
                            if indent != required {
                                // Different indentation — skip in pass 2
                                // (RuboCop skips non-matching indent lines)
                                if up {
                                    if check_idx == 0 {
                                        break;
                                    }
                                    check_idx -= 1;
                                } else {
                                    check_idx += 1;
                                }
                                continue;
                            }
                        }
                        // Check if same operator at same byte column
                        if byte_col + op_bytes.len() <= line_bytes.len()
                            && &line_bytes[byte_col..byte_col + op_bytes.len()] == op_bytes
                        {
                            return true;
                        }
                        // In pass 1 (no indent filter), stop at first non-blank line
                        // In pass 2 (with indent filter), stop at first matching-indent line
                        break;
                    }
                }

                if up {
                    if check_idx == 0 {
                        break;
                    }
                    check_idx -= 1;
                } else {
                    check_idx += 1;
                }
            }
        }

        false
    }

    fn check_operator_spacing(&mut self, op_loc: &ruby_prism::Location<'_>) {
        let start = op_loc.start_offset();
        let end = op_loc.end_offset();
        let bytes = self.source.as_bytes();
        let op_str = std::str::from_utf8(op_loc.as_slice()).unwrap_or("??");

        // Skip ** when exponent style is no_space
        if op_str == "**" && self.exponent_no_space {
            return;
        }

        let space_before = start > 0 && bytes[start - 1] == b' ';
        let space_after = end < bytes.len() && bytes[end] == b' ';
        let newline_after = end >= bytes.len() || bytes[end] == b'\n' || bytes[end] == b'\r';

        // Check for multiple spaces (extra whitespace before or after operator)
        let multi_space_before = start >= 2 && bytes[start - 1] == b' ' && bytes[start - 2] == b' ';
        let multi_space_after = end + 1 < bytes.len() && bytes[end] == b' ' && bytes[end + 1] == b' ';

        if multi_space_before || multi_space_after {
            // Skip if operator is at start of line (spaces are indentation, not extra spacing)
            if multi_space_before {
                let mut ls = start;
                while ls > 0 && bytes[ls - 1] != b'\n' {
                    ls -= 1;
                }
                if bytes[ls..start].iter().all(|&b| b == b' ' || b == b'\t') {
                    return;
                }
            }

            // AllowForAlignment: skip if aligned with same operator on adjacent line
            if self.allow_for_alignment
                && self.is_aligned_with_adjacent(start, op_loc.as_slice())
            {
                return;
            }

            // Skip if trailing space extends to a comment on the same line
            // (RuboCop: `return if comment && with_space.last_column == comment.loc.column`)
            if multi_space_after {
                let mut p = end;
                while p < bytes.len() && bytes[p] == b' ' {
                    p += 1;
                }
                if p < bytes.len() && bytes[p] == b'#' {
                    return;
                }
            }

            // Find the extent of extra spaces before the operator
            let ws_start_before = if multi_space_before {
                let mut s = start - 1;
                while s > 0 && bytes[s - 1] == b' ' {
                    s -= 1;
                }
                s
            } else {
                start
            };
            // Find the extent of extra spaces after the operator
            let ws_end_after = if multi_space_after {
                let mut e = end;
                while e < bytes.len() && bytes[e] == b' ' {
                    e += 1;
                }
                e
            } else {
                end
            };
            let (line, column) = self.source.offset_to_line_col(start);
            let mut diag = self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Operator `{op_str}` should be surrounded by a single space."),
            );
            if self.has_corrections {
                // Replace multi-space before with single space
                if multi_space_before {
                    self.corrections.push(crate::correction::Correction {
                        start: ws_start_before, end: start, replacement: " ".to_string(),
                        cop_name: self.cop.name(), cop_index: 0,
                    });
                }
                // Replace multi-space after with single space
                if multi_space_after {
                    self.corrections.push(crate::correction::Correction {
                        start: end, end: ws_end_after, replacement: " ".to_string(),
                        cop_name: self.cop.name(), cop_index: 0,
                    });
                }
                diag.corrected = true;
            }
            self.diagnostics.push(diag);
        } else if !space_before || (!space_after && !newline_after) {
            let (line, column) = self.source.offset_to_line_col(start);
            let mut diag = self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Surrounding space missing for operator `{op_str}`."),
            );
            if self.has_corrections {
                if !space_before {
                    self.corrections.push(crate::correction::Correction {
                        start, end: start, replacement: " ".to_string(),
                        cop_name: self.cop.name(), cop_index: 0,
                    });
                }
                if !space_after && !newline_after {
                    self.corrections.push(crate::correction::Correction {
                        start: end, end, replacement: " ".to_string(),
                        cop_name: self.cop.name(), cop_index: 0,
                    });
                }
                diag.corrected = true;
            }
            self.diagnostics.push(diag);
        }
    }
}

impl<'pr> Visit<'pr> for BinaryOperatorChecker<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        if BINARY_OPERATORS.iter().any(|&op| op == name)
            && node.receiver().is_some()
            && node.arguments().is_some()
            && node.call_operator_loc().is_none() // skip x.+ y and x&.+ y
        {
            if let Some(msg_loc) = node.message_loc() {
                self.check_operator_spacing(&msg_loc);
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        let op_loc = node.operator_loc();
        // Skip keyword form `and`
        if op_loc.as_slice() != b"and" {
            self.check_operator_spacing(&op_loc);
        }
        ruby_prism::visit_and_node(self, node);
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        let op_loc = node.operator_loc();
        // Skip keyword form `or`
        if op_loc.as_slice() != b"or" {
            self.check_operator_spacing(&op_loc);
        }
        ruby_prism::visit_or_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAroundOperators, "cops/layout/space_around_operators");
    crate::cop_autocorrect_fixture_tests!(SpaceAroundOperators, "cops/layout/space_around_operators");
}
