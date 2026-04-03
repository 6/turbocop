use crate::cop::shared::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for nested percent literals inside `%w`, `%W`, `%i`, `%I` arrays.
///
/// Root cause of FP: `%s_changed?` was flagged because the check treated `_` as a
/// non-alphanumeric delimiter. RuboCop uses `\W` (excludes `[a-zA-Z0-9_]`), so
/// `_` after `%s` means it's NOT a nested percent literal.
///
/// Root cause of FNs: the prefix list was missing bare `%`, so patterns like `%=`,
/// `%.`, `%:`, `%#`, `%[` inside percent arrays were not detected. RuboCop's
/// `PERCENT_LITERAL_TYPES` includes `%` as a standalone prefix.
///
/// Also fixed: the cop was emitting multiple diagnostics per array (one per matching
/// element/prefix pair). RuboCop emits at most one per array.
pub struct NestedPercentLiteral;

/// Percent literal prefixes that indicate a nested percent literal.
/// Matches RuboCop's PreferredDelimiters::PERCENT_LITERAL_TYPES.
const PERCENT_PREFIXES: &[&[u8]] = &[
    b"%w", b"%W", b"%i", b"%I", b"%q", b"%Q", b"%r", b"%s", b"%x", b"%",
];

/// Returns true if the byte is a Ruby word character (\w = [a-zA-Z0-9_]).
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl Cop for NestedPercentLiteral {
    fn name(&self) -> &'static str {
        "Lint/NestedPercentLiteral"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
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
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        // Check if this is a %w or %i literal
        let open_loc = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let open_src = open_loc.as_slice();
        let is_percent_literal = open_src.starts_with(b"%w")
            || open_src.starts_with(b"%W")
            || open_src.starts_with(b"%i")
            || open_src.starts_with(b"%I");

        if !is_percent_literal {
            return;
        }

        // Check if any element contains a percent literal prefix followed by a
        // non-word character (\W in Ruby regex). This matches RuboCop's approach:
        // REGEXES = PERCENT_LITERAL_TYPES.map { |pl| /\A#{pl}\W/ }
        let has_nested = array_node.elements().iter().any(|element| {
            let elem_loc = element.location();
            let elem_src = &source.as_bytes()[elem_loc.start_offset()..elem_loc.end_offset()];

            PERCENT_PREFIXES.iter().any(|prefix| {
                elem_src.len() > prefix.len()
                    && elem_src.starts_with(prefix)
                    && !is_word_char(elem_src[prefix.len()])
            })
        });

        if has_nested {
            let loc = array_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Within percent literals, nested percent literals do not function and may be unwanted in the result.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedPercentLiteral, "cops/lint/nested_percent_literal");
}
