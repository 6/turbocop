use crate::cop::shared::node_type::STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::{parse_source, source::SourceFile};

/// Mirrors RuboCop's `on_str` handling for `%q/%Q` literals.
///
/// Prism reports empty percent literals and some other `%q/%Q` shapes as
/// `StringNode`s, while the Parser gem that RuboCop uses treats empty and
/// most multiline percent literals as `dstr`, so RuboCop never inspects them.
///
/// Parser gem treats multiline `%Q` strings with exactly one trailing newline
/// (content on the opening line, closing delimiter alone on the next line) as
/// `str`, so RuboCop _does_ inspect those. Strings spanning 3+ lines or with
/// newlines mid-content are `dstr` in Parser and should be skipped.
///
/// The `swapcase_preserves_string_semantics` helper reparses the case-swapped
/// literal and compares `unescaped()` bytes before reporting an offense.
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

        // Parser gem reports empty percent literals as `dstr`, so RuboCop's
        // `on_str` never sees them. Most multiline `%Q` strings are also
        // treated as `dstr` by Parser. The exception is when content has
        // exactly one trailing newline — i.e., all content is on the opening
        // line and the closing delimiter sits alone on the next line
        // (e.g., `%Q{text\n}`). Those are `str` in Parser and should be checked.
        if raw_content.is_empty() {
            return;
        }
        let newline_count = raw_content.iter().filter(|&&b| b == b'\n').count();
        if newline_count > 1 || (newline_count == 1 && *raw_content.last().unwrap() != b'\n') {
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
