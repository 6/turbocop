use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BlockDelimiters;

impl Cop for BlockDelimiters {
    fn name(&self) -> &'static str {
        "Style/BlockDelimiters"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "line_count_based");
        let _procedural_methods = config.get_string_array("ProceduralMethods");
        let _functional_methods = config.get_string_array("FunctionalMethods");
        let _allowed_methods = config.get_string_array("AllowedMethods");
        let _allowed_patterns = config.get_string_array("AllowedPatterns");
        let _allow_braces_on_procedural = config.get_bool("AllowBracesOnProceduralOneLiners", false);
        let _braces_required_methods = config.get_string_array("BracesRequiredMethods");

        if enforced_style != "line_count_based" {
            // Only implement line_count_based for now
            return Vec::new();
        }

        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();
        let opening = opening_loc.as_slice();

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing_loc.start_offset());
        let is_single_line = open_line == close_line;

        if is_single_line && opening == b"do" {
            // Single-line block using do..end → should use {}
            let (line, column) = source.offset_to_line_col(opening_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer `{...}` over `do...end` for single-line blocks.".to_string(),
            )];
        }

        if !is_single_line && opening == b"{" {
            // Multi-line block using {} → should use do..end
            let (line, column) = source.offset_to_line_col(opening_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer `do...end` over `{...}` for multi-line blocks.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockDelimiters, "cops/style/block_delimiters");
}
