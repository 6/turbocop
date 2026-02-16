use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundAccessModifier;

const ACCESS_MODIFIERS: &[&[u8]] = &[b"private", b"protected", b"public"];

impl Cop for EmptyLinesAroundAccessModifier {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundAccessModifier"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _enforced_style = config.get_str("EnforcedStyle", "around");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Check if it's a bare access modifier (no receiver, no args, no block)
        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if !ACCESS_MODIFIERS.iter().any(|&m| m == method_name) {
            return Vec::new();
        }

        // Must have no arguments to be an access modifier (not `private :foo`)
        if call.arguments().is_some() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        let lines: Vec<&[u8]> = source.lines().collect();

        let modifier_str = std::str::from_utf8(method_name).unwrap_or("");

        // Check blank line before (unless it's the first line in the body)
        let need_blank_before = line > 1;
        let has_blank_before = if line >= 2 {
            is_blank_line(lines[line - 2])
        } else {
            true
        };

        // Check blank line after
        let has_blank_after = if line < lines.len() {
            is_blank_line(lines[line])
        } else {
            true
        };

        if need_blank_before && (!has_blank_before || !has_blank_after) {
            return vec![self.diagnostic(
                source,
                line,
                col,
                format!("Keep a blank line before and after `{modifier_str}`."),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundAccessModifier,
        "cops/layout/empty_lines_around_access_modifier"
    );
}
