use crate::cop::node_type::{RANGE_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for mixed-case character ranges that include unintended characters.
/// For example, `('A'..'z')` includes `[`, `\`, `]`, `^`, `_`, `` ` ``.
pub struct MixedCaseRange;

const MSG: &str = "Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.";

impl Cop for MixedCaseRange {
    fn name(&self) -> &'static str {
        "Lint/MixedCaseRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[RANGE_NODE, STRING_NODE]
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
        // Check inclusive range (..)
        if let Some(range) = node.as_range_node() {
            diagnostics.extend(self.check_range(source, range));
        }
    }
}

impl MixedCaseRange {
    fn check_range(
        &self,
        source: &SourceFile,
        range: ruby_prism::RangeNode<'_>,
    ) -> Vec<Diagnostic> {
        let left = match range.left() {
            Some(l) => l,
            None => return Vec::new(),
        };
        let right = match range.right() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Both must be string literals
        let left_str = match left.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let right_str = match right.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let left_val = left_str.unescaped();
        let right_val = right_str.unescaped();

        // Must be single characters
        if left_val.len() != 1 || right_val.len() != 1 {
            return Vec::new();
        }

        let left_char = left_val[0] as char;
        let right_char = right_val[0] as char;

        if is_unsafe_range(left_char, right_char) {
            let loc = range.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, MSG.to_string())];
        }

        Vec::new()
    }
}

fn char_range(c: char) -> Option<u8> {
    if c.is_ascii_lowercase() {
        Some(0) // a-z
    } else if c.is_ascii_uppercase() {
        Some(1) // A-Z
    } else {
        None
    }
}

fn is_unsafe_range(start: char, end: char) -> bool {
    let start_range = char_range(start);
    let end_range = char_range(end);

    match (start_range, end_range) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MixedCaseRange, "cops/lint/mixed_case_range");
}
