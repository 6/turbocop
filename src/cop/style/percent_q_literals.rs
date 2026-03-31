use crate::cop::node_type::STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::{parse_source, source::SourceFile};

/// Mirrors RuboCop's `on_str` handling for `%q/%Q` literals.
///
/// Prism reports empty percent literals and some other `%q/%Q` shapes as
/// `StringNode`s, while the Parser gem that RuboCop uses treats empty and
/// multiline percent literals as `dstr`, so RuboCop never inspects them here.
/// The original nitrocop implementation also skipped every backslash, which
/// missed safe `%Q` -> `%q` conversions like `\\n` and LaTeX-heavy strings.
/// Fix: only inspect static `StringNode` percent literals, skip empty/multiline
/// cases to match Parser, and reparse the case-swapped literal to compare
/// `unescaped()` bytes before reporting an offense.
pub struct PercentQLiterals;

impl Cop for PercentQLiterals {
    fn name(&self) -> &'static str {
        "Style/PercentQLiterals"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
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
        let style = config.get_str("EnforcedStyle", "lower_case_q");

        let Some(string) = node.as_string_node() else {
            return;
        };
        let Some(opening) = string.opening_loc().map(|loc| loc.as_slice()) else {
            return;
        };
        let raw_content = string.content_loc().as_slice();

        // Parser gem reports empty and multiline percent literals as `dstr`,
        // so RuboCop's `on_str` never sees them.
        if raw_content.is_empty() || raw_content.contains(&b'\n') {
            return;
        }

        let (expected_opening, message) = match style {
            "lower_case_q" => (
                b"%Q".as_slice(),
                "Do not use `%Q` unless interpolation is needed. Use `%q`.",
            ),
            "upper_case_q" => (b"%q".as_slice(), "Use `%Q` instead of `%q`."),
            _ => return,
        };

        if !opening.starts_with(expected_opening) {
            return;
        }

        if !swapcase_preserves_string_semantics(string) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
    }
}

fn swapcase_preserves_string_semantics(string: ruby_prism::StringNode<'_>) -> bool {
    let literal = string.location().as_slice();
    if literal.len() < 2 || literal[0] != b'%' {
        return false;
    }
    let original_unescaped = string.unescaped().to_vec();

    let mut corrected = literal.to_vec();
    corrected[1] = match corrected[1] {
        b'Q' => b'q',
        b'q' => b'Q',
        _ => return false,
    };

    let parse_result = parse_source(&corrected);
    if parse_result.errors().next().is_some() {
        return false;
    }

    let root = parse_result.node();
    let Some(program) = root.as_program_node() else {
        return false;
    };
    let mut body = program.statements().body().iter();
    let Some(corrected_node) = body.next() else {
        return false;
    };
    if body.next().is_some() {
        return false;
    }

    let Some(corrected_string) = corrected_node.as_string_node() else {
        return false;
    };

    original_unescaped == corrected_string.unescaped()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PercentQLiterals, "cops/style/percent_q_literals");
}
