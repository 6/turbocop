use crate::cop::shared::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-17)
///
/// FP=2: `str.split(//u)` — regex with Unicode flag was treated same as `//`.
/// RuboCop's `(regexp (regopt))` only matches empty regex with no options.
/// Fix: check that closing_loc is exactly "/" (no trailing flags).
///
/// FN=2: `split(//)` without explicit receiver (implicit self) was not detected.
/// RuboCop's matcher works with or without receiver.
/// Fix: removed the `receiver().is_none()` early return.
pub struct StringChars;

impl Cop for StringChars {
    fn name(&self) -> &'static str {
        "Style/StringChars"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `split` method
        if call.name().as_slice() != b"split" {
            return;
        }

        // Note: receiver can be None (implicit self) — RuboCop flags both
        // `string.split(//)` and `split(//)`.

        // Must have exactly one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];

        // Check for split('') or split("")
        let is_empty_string = arg
            .as_string_node()
            .is_some_and(|s| s.unescaped().is_empty());

        // Check for split(//) — but NOT split(//u) or other flagged regexes.
        // In Parser gem, `//u` has a `regopt` child with options, so RuboCop's
        // `(regexp (regopt))` matcher only matches bare `//` (empty options).
        let is_empty_regexp = arg
            .as_regular_expression_node()
            .is_some_and(|r| r.unescaped().is_empty() && r.closing_loc().as_slice() == b"/");

        if !is_empty_string && !is_empty_regexp {
            return;
        }

        // Build the offense message using the source range from selector to end
        let msg_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        let offense_src = std::str::from_utf8(
            &source.content[msg_loc.start_offset()..node.location().end_offset()],
        )
        .unwrap_or("split(...)");

        let mut diag = self.diagnostic(
            source,
            line,
            column,
            format!("Use `chars` instead of `{}`.", offense_src),
        );
        // Autocorrect: replace `split(//)`, `split('')`, or `split("")` with `chars`
        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: msg_loc.start_offset(),
                end: node.location().end_offset(),
                replacement: "chars".to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringChars, "cops/style/string_chars");
    crate::cop_autocorrect_fixture_tests!(StringChars, "cops/style/string_chars");
}
