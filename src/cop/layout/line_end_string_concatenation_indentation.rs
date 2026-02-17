use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct LineEndStringConcatenationIndentation;

impl Cop for LineEndStringConcatenationIndentation {
    fn name(&self) -> &'static str {
        "Layout/LineEndStringConcatenationIndentation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "aligned");
        let indent_width = config.get_usize("IndentationWidth", 2);

        let mut visitor = ConcatVisitor {
            cop: self,
            source,
            code_map,
            diagnostics: Vec::new(),
            style,
            indent_width,
            parent_is_indented: true, // top-level counts as always indented
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct ConcatVisitor<'a> {
    cop: &'a LineEndStringConcatenationIndentation,
    source: &'a SourceFile,
    code_map: &'a CodeMap,
    diagnostics: Vec<Diagnostic>,
    style: &'a str,
    indent_width: usize,
    parent_is_indented: bool,
}

impl ConcatVisitor<'_> {
    fn check_dstr(&mut self, node: &ruby_prism::InterpolatedStringNode<'_>) {
        let parts: Vec<_> = node.parts().iter().collect();
        if parts.len() < 2 {
            return;
        }

        // Check that this is a backslash-concatenated string (multiline dstr
        // where each child is a single-line string/dstr part)
        let bytes = self.source.as_bytes();
        let (first_line, _) = self.source.offset_to_line_col(parts[0].location().start_offset());
        let (last_line, _) = self.source.offset_to_line_col(parts.last().unwrap().location().start_offset());
        if first_line == last_line {
            return; // Not multiline
        }

        // Check that each part is single-line and separated by backslash
        for part in &parts {
            let loc = part.location();
            let (sl, _) = self.source.offset_to_line_col(loc.start_offset());
            let (el, _) = self.source.offset_to_line_col(loc.end_offset().saturating_sub(1).max(loc.start_offset()));
            if sl != el {
                return; // Multi-line part
            }
        }

        // Check backslash between parts
        for pair in parts.windows(2) {
            let end_offset = pair[0].location().end_offset();
            let start_offset = pair[1].location().start_offset();
            let between = &bytes[end_offset..start_offset];
            if !between.iter().any(|&b| b == b'\\') {
                return; // Not backslash continuation
            }
        }

        // Skip if inside a heredoc body
        if self.code_map.is_heredoc(parts[0].location().start_offset()) {
            return;
        }

        // Determine effective mode: aligned vs indented
        let use_indented = self.style == "indented" || self.parent_is_indented;

        // Get column positions of each part
        let columns: Vec<usize> = parts.iter().map(|p| {
            let (_, col) = self.source.offset_to_line_col(p.location().start_offset());
            col
        }).collect();

        if use_indented && columns.len() >= 2 {
            // First, check indentation of the second part
            // base_column = indentation of the first part's source line
            let (first_part_line, _) = self.source.offset_to_line_col(parts[0].location().start_offset());
            let first_line_indent = if first_part_line > 0 {
                let lines: Vec<&[u8]> = self.source.lines().collect();
                lines[first_part_line - 1].iter().take_while(|&&b| b == b' ').count()
            } else {
                0
            };

            // Check if the first part's grandparent is a pair (hash key-value)
            // In that case, base_column is the pair's column
            // For simplicity, use the line indentation as base
            let expected_indent = first_line_indent + self.indent_width;

            if columns[1] != expected_indent {
                let (line_num, _) = self.source.offset_to_line_col(parts[1].location().start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line_num,
                    columns[1],
                    "Indent the first part of a string concatenated with backslash.".to_string(),
                ));
            }

            // Check alignment of third+ parts with the second part
            // RuboCop updates base_column after each check (rolling base)
            if columns.len() >= 3 {
                let mut base = columns[1];
                for (idx, &col) in columns[2..].iter().enumerate() {
                    if col != base {
                        let part_idx = idx + 2;
                        let (line_num, _) = self.source.offset_to_line_col(parts[part_idx].location().start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line_num,
                            col,
                            "Align parts of a string concatenated with backslash.".to_string(),
                        ));
                    }
                    base = col; // Update rolling base like RuboCop
                }
            }
        } else if self.style == "aligned" {
            // check_aligned from index 1: parts should be aligned (rolling base)
            let mut base = columns[0];
            for (idx, &col) in columns[1..].iter().enumerate() {
                if col != base {
                    let part_idx = idx + 1;
                    let (line_num, _) = self.source.offset_to_line_col(parts[part_idx].location().start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line_num,
                        col,
                        "Align parts of a string concatenated with backslash.".to_string(),
                    ));
                }
                base = col; // Update rolling base like RuboCop
            }
        }
    }
}

impl<'pr> Visit<'pr> for ConcatVisitor<'_> {
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        self.check_dstr(node);
        // Don't recurse into children — we handle the whole dstr at once
    }

    // Track parent context for always_indented? logic
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = true;
        ruby_prism::visit_def_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = true;
        ruby_prism::visit_block_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = true;
        ruby_prism::visit_begin_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = true;
        ruby_prism::visit_if_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = true;
        ruby_prism::visit_unless_node(self, node);
        self.parent_is_indented = was;
    }

    // Assignment, method call, hash, array → NOT always indented
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = false;
        ruby_prism::visit_call_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = false;
        ruby_prism::visit_local_variable_write_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_instance_variable_write_node(&mut self, node: &ruby_prism::InstanceVariableWriteNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = false;
        ruby_prism::visit_instance_variable_write_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = false;
        ruby_prism::visit_constant_write_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = false;
        ruby_prism::visit_hash_node(self, node);
        self.parent_is_indented = was;
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        let was = self.parent_is_indented;
        self.parent_is_indented = false;
        ruby_prism::visit_keyword_hash_node(self, node);
        self.parent_is_indented = was;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        LineEndStringConcatenationIndentation,
        "cops/layout/line_end_string_concatenation_indentation"
    );
}
