use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RequireMfa;

const MSG: &str = "`metadata['rubygems_mfa_required']` must be set to `'true'`.";

impl Cop for RequireMfa {
    fn name(&self) -> &'static str {
        "Gemspec/RequireMFA"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = GemSpecVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct GemSpecVisitor<'a> {
    cop: &'a RequireMfa,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl GemSpecVisitor<'_> {
    /// Check whether the receiver of a CallNode is `Gem::Specification`.
    fn is_gem_specification(receiver: &ruby_prism::Node<'_>) -> bool {
        if let Some(cp) = receiver.as_constant_path_node() {
            if let Some(name) = cp.name() {
                if name.as_slice() == b"Specification" {
                    if let Some(parent) = cp.parent() {
                        return crate::cop::util::constant_name(&parent) == Some(b"Gem");
                    }
                }
            }
        }
        false
    }

    /// Scan lines within the given byte range for MFA metadata.
    /// Returns:
    ///   - `Some(true)` if MFA is set to 'true'
    ///   - `Some(false)` if MFA is set to a non-'true' value (e.g. 'false')
    ///   - `None` if MFA is not mentioned at all
    fn find_mfa_in_range(&self, start_offset: usize, end_offset: usize) -> Option<bool> {
        let bytes = self.source.as_bytes();
        let block_bytes = &bytes[start_offset..end_offset.min(bytes.len())];
        let block_str = match std::str::from_utf8(block_bytes) {
            Ok(s) => s,
            Err(_) => return None,
        };

        for line_str in block_str.lines() {
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            // Check for metadata['rubygems_mfa_required'] or metadata["rubygems_mfa_required"]
            // in assignment form: metadata['key'] = 'value'
            let has_mfa_key = trimmed.contains("metadata['rubygems_mfa_required']")
                || trimmed.contains("metadata[\"rubygems_mfa_required\"]");

            if has_mfa_key && trimmed.contains("= ") {
                if trimmed.contains("'true'") || trimmed.contains("\"true\"") {
                    return Some(true);
                }
                return Some(false);
            }

            // Also check for hash-style metadata:
            // 'rubygems_mfa_required' => 'true' (inside .metadata = { ... })
            let has_hash_key = trimmed.contains("'rubygems_mfa_required'")
                || trimmed.contains("\"rubygems_mfa_required\"");

            if has_hash_key && trimmed.contains("=>") {
                if trimmed.contains("'true'") || trimmed.contains("\"true\"") {
                    return Some(true);
                }
                return Some(false);
            }
        }

        None
    }

    /// Find the byte offset of the `'false'` (or `"false"`) value on the line
    /// containing `rubygems_mfa_required` within the given range.
    fn find_false_value_location(
        &self,
        start_offset: usize,
        end_offset: usize,
    ) -> Option<(usize, usize)> {
        let bytes = self.source.as_bytes();
        let block_bytes = &bytes[start_offset..end_offset.min(bytes.len())];
        let block_str = match std::str::from_utf8(block_bytes) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let mut current_offset = start_offset;
        for line_str in block_str.lines() {
            let trimmed = line_str.trim();
            if (trimmed.contains("metadata['rubygems_mfa_required']")
                || trimmed.contains("metadata[\"rubygems_mfa_required\"]")
                || trimmed.contains("'rubygems_mfa_required'")
                || trimmed.contains("\"rubygems_mfa_required\""))
                && !trimmed.contains("'true'")
                && !trimmed.contains("\"true\"")
            {
                // Find the false value on this line
                for pattern in &["'false'", "\"false\""] {
                    if let Some(pos) = line_str.find(pattern) {
                        let abs_offset = current_offset + pos;
                        let (line, col) = self.source.offset_to_line_col(abs_offset);
                        return Some((line, col));
                    }
                }
                // Fallback: find any quoted value that isn't 'true'
                // Report at the start of the line's content
                let (line, col) = self.source.offset_to_line_col(current_offset);
                return Some((line, col));
            }
            // Advance past the line (line_str length + newline)
            current_offset += line_str.len();
            // Skip the newline character if present
            if current_offset < end_offset && bytes.get(current_offset) == Some(&b'\n') {
                current_offset += 1;
            }
        }
        None
    }
}

impl<'pr> Visit<'pr> for GemSpecVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Look for Gem::Specification.new do |spec| ... end
        if node.name().as_slice() == b"new" {
            if let Some(receiver) = node.receiver() {
                if Self::is_gem_specification(&receiver) {
                    if let Some(block) = node.block() {
                        if let Some(block_node) = block.as_block_node() {
                            let block_start = block_node.location().start_offset();
                            let block_end = block_node.location().end_offset();

                            match self.find_mfa_in_range(block_start, block_end) {
                                Some(true) => {
                                    // MFA is correctly set to 'true', no offense
                                }
                                Some(false) => {
                                    // MFA is set to a wrong value (e.g., 'false')
                                    // Report at the false value's location
                                    if let Some((line, col)) =
                                        self.find_false_value_location(block_start, block_end)
                                    {
                                        self.diagnostics.push(self.cop.diagnostic(
                                            self.source,
                                            line,
                                            col,
                                            MSG.to_string(),
                                        ));
                                    }
                                }
                                None => {
                                    // MFA not mentioned at all — report at the
                                    // Gem::Specification.new call location
                                    let call_start = node.location().start_offset();
                                    let (line, col) = self.source.offset_to_line_col(call_start);
                                    // RuboCop reports at column 0 of the call line
                                    let _ = col;
                                    self.diagnostics.push(self.cop.diagnostic(
                                        self.source,
                                        line,
                                        0,
                                        MSG.to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Continue walking into children
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        RequireMfa,
        "cops/gemspec/require_mfa",
        missing_metadata = "missing_metadata.rb",
        wrong_value = "wrong_value.rb",
        no_metadata_at_all = "no_metadata_at_all.rb",
        preamble = "preamble.rb",
    );
}
