use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantRegexpQuantifiers;

impl Cop for RedundantRegexpQuantifiers {
    fn name(&self) -> &'static str {
        "Lint/RedundantRegexpQuantifiers"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let regexp = match node.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Check for interpolation — skip
        let raw_src = &source.as_bytes()
            [regexp.location().start_offset()..regexp.location().end_offset()];
        if raw_src.windows(2).any(|w| w == b"#{") {
            return Vec::new();
        }

        let content = regexp.unescaped();
        let content_str = match std::str::from_utf8(&content) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        // Find redundant quantifiers: (?:...Q1)Q2 where both are greedy quantifiers
        // and the group contains only a single element with quantifier Q1
        check_redundant_quantifiers(self, source, content_str, regexp, &mut diagnostics);

        diagnostics
    }
}

fn check_redundant_quantifiers(
    cop: &RedundantRegexpQuantifiers,
    source: &SourceFile,
    pattern: &str,
    regexp: &ruby_prism::RegularExpressionNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }

        // Skip character classes
        if bytes[i] == b'[' {
            i += 1;
            if i < len && bytes[i] == b'^' {
                i += 1;
            }
            if i < len && bytes[i] == b']' {
                i += 1;
            }
            while i < len && bytes[i] != b']' {
                if bytes[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Look for non-capturing groups: (?:...)
        if bytes[i] == b'(' && i + 2 < len && bytes[i + 1] == b'?' && bytes[i + 2] == b':' {
            let group_start = i;
            // Find matching close paren
            let group_end = find_matching_paren(bytes, i);
            if let Some(end) = group_end {
                // Check if the group is followed by a quantifier
                let after_group = end + 1;
                if let Some((outer_q, outer_q_end)) = parse_quantifier(bytes, after_group) {
                    // Check if the group content is a single element with a quantifier
                    let inner = &bytes[i + 3..end]; // content inside (?:...)
                    if let Some((inner_q, _)) = find_single_element_quantifier(inner) {
                        // Check if redundant (both greedy, no possessive/reluctant)
                        if is_greedy(&outer_q) && is_greedy(&inner_q) {
                            // Check that the group doesn't contain captures
                            let inner_str = std::str::from_utf8(inner).unwrap_or("");
                            if !contains_capture_group(inner_str) {
                                let combined = combine_quantifiers(&inner_q, &outer_q);
                                let inner_q_display = quantifier_display(&inner_q);
                                let outer_q_display = quantifier_display(&outer_q);
                                let combined_display = quantifier_display(&combined);

                                // Report at the position of the inner quantifier end through the outer quantifier
                                let regexp_start = regexp.location().start_offset() + 1; // skip '/'
                                let offset = regexp_start + (end - inner_q_display.len() + 1 - (i + 3) + (i + 3));
                                // Calculate more carefully
                                let inner_q_start_in_pattern = end - inner_q_display.len() + 1 - 3; // approximate
                                // Actually, let's find the column of the quantifiers in the source
                                // The pattern starts at regexp_start offset in the source
                                let q_start = regexp.location().start_offset() + 1 + group_start + 3;
                                // The redundant range is from the inner quantifier through the outer
                                let _ = q_start;

                                // Simpler approach: report at the regexp node location
                                let loc = regexp.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(cop.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!(
                                        "Replace redundant quantifiers `{}` and `{}` with a single `{}`.",
                                        inner_q_display, outer_q_display, combined_display
                                    ),
                                ));
                            }
                        }
                    }
                }

                i = group_end.map(|e| e + 1).unwrap_or(i + 1);
                continue;
            }
        }

        i += 1;
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Quantifier {
    Plus,       // +
    Star,       // *
    Question,   // ?
    Interval(Option<usize>, Option<usize>), // {n,m}
}

fn quantifier_display(q: &Quantifier) -> String {
    match q {
        Quantifier::Plus => "+".to_string(),
        Quantifier::Star => "*".to_string(),
        Quantifier::Question => "?".to_string(),
        Quantifier::Interval(min, max) => {
            match (min, max) {
                (Some(n), Some(m)) => format!("{{{},{}}}", n, m),
                (Some(n), None) => format!("{{{},}}", n),
                (None, Some(m)) => format!("{{,{}}}", m),
                (None, None) => "{,}".to_string(),
            }
        }
    }
}

fn is_greedy(q: &Quantifier) -> bool {
    // All our quantifiers are greedy by default
    true
}

fn normalize_quantifier(q: &Quantifier) -> Quantifier {
    match q {
        Quantifier::Plus => Quantifier::Plus,
        Quantifier::Star => Quantifier::Star,
        Quantifier::Question => Quantifier::Question,
        Quantifier::Interval(min, max) => {
            let min = min.unwrap_or(0);
            let max_val = *max;
            // {0,} = *, {1,} = +, {0,1} = ?
            match (min, max_val) {
                (0, None) => Quantifier::Star,
                (1, None) => Quantifier::Plus,
                (0, Some(1)) => Quantifier::Question,
                _ => Quantifier::Interval(Some(min), max_val),
            }
        }
    }
}

fn combine_quantifiers(inner: &Quantifier, outer: &Quantifier) -> Quantifier {
    let inner = normalize_quantifier(inner);
    let outer = normalize_quantifier(outer);

    // Both + -> +
    // Both * -> *
    // Both ? -> ?
    // + and ? (or ? and +) -> *
    // + and * (or * and +) -> *
    // * and ? (or ? and *) -> *
    match (&inner, &outer) {
        (Quantifier::Plus, Quantifier::Plus) => Quantifier::Plus,
        (Quantifier::Star, Quantifier::Star) => Quantifier::Star,
        (Quantifier::Question, Quantifier::Question) => Quantifier::Question,
        _ => Quantifier::Star, // Any other combination = *
    }
}

fn parse_quantifier(bytes: &[u8], pos: usize) -> Option<(Quantifier, usize)> {
    if pos >= bytes.len() {
        return None;
    }
    match bytes[pos] {
        b'+' => Some((Quantifier::Plus, pos + 1)),
        b'*' => Some((Quantifier::Star, pos + 1)),
        b'?' => Some((Quantifier::Question, pos + 1)),
        b'{' => {
            let mut end = pos + 1;
            let mut min = None;
            let mut max = None;
            let mut num_buf = String::new();
            let mut seen_comma = false;

            while end < bytes.len() && bytes[end] != b'}' {
                if bytes[end] == b',' {
                    if !num_buf.is_empty() {
                        min = num_buf.parse().ok();
                    }
                    num_buf.clear();
                    seen_comma = true;
                } else if bytes[end].is_ascii_digit() {
                    num_buf.push(bytes[end] as char);
                }
                end += 1;
            }

            if end < bytes.len() {
                if seen_comma {
                    if !num_buf.is_empty() {
                        max = num_buf.parse().ok();
                    }
                } else if !num_buf.is_empty() {
                    let n: Option<usize> = num_buf.parse().ok();
                    min = n;
                    max = n;
                }
                Some((Quantifier::Interval(min, max), end + 1))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn find_matching_paren(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'[' {
            i += 1;
            if i < bytes.len() && bytes[i] == b'^' {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b']' {
                i += 1;
            }
            while i < bytes.len() && bytes[i] != b']' {
                if bytes[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'(' {
            depth += 1;
        } else if bytes[i] == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Check if the inner content of a non-capturing group is a single element with a quantifier.
/// Returns the quantifier if found.
fn find_single_element_quantifier(inner: &[u8]) -> Option<(Quantifier, usize)> {
    let len = inner.len();
    if len == 0 {
        return None;
    }

    // Check if this is a single element followed by a quantifier
    // A single element is: a literal char, an escaped char, a character class, or a nested group
    let mut i = 0;

    // Skip the element
    if inner[i] == b'\\' {
        i += 2; // escaped char
    } else if inner[i] == b'[' {
        // character class
        i += 1;
        if i < len && inner[i] == b'^' {
            i += 1;
        }
        if i < len && inner[i] == b']' {
            i += 1;
        }
        while i < len && inner[i] != b']' {
            if inner[i] == b'\\' {
                i += 2;
            } else {
                i += 1;
            }
        }
        if i < len {
            i += 1;
        }
    } else if inner[i] == b'(' {
        // nested group
        if let Some(end) = find_matching_paren(inner, i) {
            i = end + 1;
        } else {
            return None;
        }
    } else if inner[i] == b'.' || inner[i].is_ascii_alphanumeric() || inner[i] == b'^' || inner[i] == b'$' {
        i += 1;
    } else {
        // Other special chars
        i += 1;
    }

    // Now check for quantifier
    if i >= len {
        return None;
    }

    let (q, q_end) = parse_quantifier(inner, i)?;

    // Check that the quantifier is followed by nothing (or a possessive/reluctant marker)
    if q_end < len {
        // Could be possessive (+) or reluctant (?)
        if inner[q_end] == b'+' || inner[q_end] == b'?' {
            // Not a plain greedy quantifier — not redundant
            return None;
        }
        // More content after the quantifier — not a single-element group
        return None;
    }

    Some((q, q_end))
}

fn contains_capture_group(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'[' {
            i += 1;
            if i < len && bytes[i] == b'^' {
                i += 1;
            }
            if i < len && bytes[i] == b']' {
                i += 1;
            }
            while i < len && bytes[i] != b']' {
                if bytes[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'(' && i + 1 < len && bytes[i + 1] != b'?' {
            return true;
        }
        if bytes[i] == b'(' && i + 2 < len && bytes[i + 1] == b'?' {
            match bytes[i + 2] {
                b'<' => {
                    if i + 3 < len && bytes[i + 3] != b'=' && bytes[i + 3] != b'!' {
                        return true;
                    }
                }
                b'\'' => return true,
                _ => {}
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRegexpQuantifiers, "cops/lint/redundant_regexp_quantifiers");
}
