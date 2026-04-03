use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-20)
///
/// FP=1, FN=1 in extended corpus, both from `openai__openai-ruby/openai.gemspec`.
/// Root cause: `s.metadata["rubygems_mfa_required"] = false.to_s` — a dynamic
/// expression, not a string literal. The Phase 2 bracket-style check treated any
/// non-'true' value as `Some(false)` (reporting at the value line = FP), when it
/// should return `None` for non-literal values so the offense is reported at the
/// `Gem::Specification.new` line (= correct FN fix).
pub struct RequireMfa;

const MSG: &str = "`metadata['rubygems_mfa_required']` must be set to `'true'`.";

impl Cop for RequireMfa {
    fn name(&self) -> &'static str {
        "Gemspec/RequireMFA"
    }

    fn supports_autocorrect(&self) -> bool {
        true
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
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = GemSpecVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            correction_info: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        if let Some(ref mut corr) = corrections {
            for (diag, info) in visitor
                .diagnostics
                .iter()
                .zip(visitor.correction_info.iter())
            {
                let mut d = diag.clone();
                if let Some(correction) = info.to_correction(source, self.name()) {
                    corr.push(correction);
                    d.corrected = true;
                }
                diagnostics.push(d);
            }
        } else {
            diagnostics.extend(visitor.diagnostics);
        }
    }
}

/// Describes the correction needed for a RequireMfa offense.
enum CorrectionInfo {
    /// Replace 'false' with 'true' at the given byte offset range.
    ReplaceFalse { start: usize, end: usize },
    /// Insert `spec.metadata['rubygems_mfa_required'] = 'true'\n` before `end` of block.
    InsertBeforeEnd {
        block_end_offset: usize,
        block_param: Vec<u8>,
    },
}

impl CorrectionInfo {
    fn to_correction(
        &self,
        source: &SourceFile,
        cop_name: &'static str,
    ) -> Option<crate::correction::Correction> {
        match self {
            CorrectionInfo::ReplaceFalse { start, end } => Some(crate::correction::Correction {
                start: *start,
                end: *end,
                replacement: "'true'".to_string(),
                cop_name,
                cop_index: 0,
            }),
            CorrectionInfo::InsertBeforeEnd {
                block_end_offset,
                block_param,
            } => {
                let bytes = source.as_bytes();
                // Find the start of the `end` keyword line to insert before it.
                let mut insert_pos = *block_end_offset;
                while insert_pos > 0 && bytes[insert_pos - 1] != b'\n' {
                    insert_pos -= 1;
                }
                // Determine indentation from the `end` line
                let end_line = &bytes[insert_pos..*block_end_offset];
                let indent: String = end_line
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .map(|&b| b as char)
                    .collect();
                let param = std::str::from_utf8(block_param).unwrap_or("spec");
                let line =
                    format!("{indent}  {param}.metadata['rubygems_mfa_required'] = 'true'\n");
                Some(crate::correction::Correction {
                    start: insert_pos,
                    end: insert_pos,
                    replacement: line,
                    cop_name,
                    cop_index: 0,
                })
            }
        }
    }
}

struct GemSpecVisitor<'a> {
    cop: &'a RequireMfa,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    correction_info: Vec<CorrectionInfo>,
}

impl GemSpecVisitor<'_> {
    /// Check whether the receiver of a CallNode is `Gem::Specification`.
    fn is_gem_specification(receiver: &ruby_prism::Node<'_>) -> bool {
        if let Some(cp) = receiver.as_constant_path_node() {
            if let Some(name) = cp.name() {
                if name.as_slice() == b"Specification" {
                    if let Some(parent) = cp.parent() {
                        return crate::cop::shared::util::constant_name(&parent) == Some(b"Gem");
                    }
                }
            }
        }
        false
    }

    /// Check if a line is a `metadata=` setter (e.g. `spec.metadata = ...`).
    /// Returns true for lines like `.metadata =` or `.metadata=`.
    fn is_metadata_setter(trimmed: &str) -> bool {
        // Match patterns like `s.metadata = {`, `spec.metadata = Foo.new`, etc.
        // But NOT `s.metadata['key'] = ...` (that's bracket assignment).
        if let Some(pos) = trimmed.find(".metadata") {
            let after = &trimmed[pos + ".metadata".len()..];
            let after_trimmed = after.trim_start();
            // Must be followed by `=` (setter) but NOT `[` (bracket access)
            after_trimmed.starts_with('=') && !after_trimmed.starts_with("==")
        } else {
            false
        }
    }

    /// Scan lines within the given byte range for MFA metadata.
    ///
    /// Follows RuboCop's semantics:
    /// 1. If a `metadata=` setter exists, check its value for `rubygems_mfa_required`.
    ///    Bracket-style `metadata['rubygems_mfa_required'] = 'true'` is ignored when
    ///    a `metadata=` setter is present (RuboCop's NodePattern captures `metadata=` first).
    /// 2. If no `metadata=` setter, check bracket-style assignments.
    ///
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

        // Phase 1: Check for `metadata=` setter.
        let mut has_metadata_setter = false;
        let mut metadata_setter_has_mfa = None;

        for line_str in block_str.lines() {
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            if Self::is_metadata_setter(trimmed) {
                has_metadata_setter = true;
                // Check if this is a hash literal containing rubygems_mfa_required
                // (the value could be on subsequent lines inside the hash)
            }
        }

        if has_metadata_setter {
            // Look for 'rubygems_mfa_required' => value WITHIN the hash of metadata=
            // In RuboCop, `metadata(node)` captures the RHS of `metadata=`.
            // If the RHS is a hash, it looks for `rubygems_mfa_required` pair inside.
            // If the RHS is not a hash (dynamic value), mfa_value returns nil → offense.
            for line_str in block_str.lines() {
                let trimmed = line_str.trim();
                if trimmed.starts_with('#') {
                    continue;
                }
                let has_hash_key = trimmed.contains("'rubygems_mfa_required'")
                    || trimmed.contains("\"rubygems_mfa_required\"");
                if has_hash_key && trimmed.contains("=>") {
                    if trimmed.contains("'true'") || trimmed.contains("\"true\"") {
                        metadata_setter_has_mfa = Some(true);
                    } else {
                        metadata_setter_has_mfa = Some(false);
                    }
                    break;
                }
            }
            // If metadata= exists but MFA key not found in hash, that's None → offense
            return metadata_setter_has_mfa;
        }

        // Phase 2: No metadata= setter. Check bracket-style assignments.
        for line_str in block_str.lines() {
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            let has_mfa_key = trimmed.contains("metadata['rubygems_mfa_required']")
                || trimmed.contains("metadata[\"rubygems_mfa_required\"]");

            if has_mfa_key && trimmed.contains("= ") {
                if trimmed.contains("'true'") || trimmed.contains("\"true\"") {
                    return Some(true);
                }
                if trimmed.contains("'false'") || trimmed.contains("\"false\"") {
                    return Some(false);
                }
                // Dynamic expression (e.g. `false.to_s`) — treat as MFA not set
                return None;
            }
        }

        None
    }

    /// Find the byte range of the `'false'` (or `"false"`) value for replacement.
    fn find_false_value_byte_range(
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
                for pattern in &["'false'", "\"false\""] {
                    if let Some(pos) = line_str.find(pattern) {
                        let abs_start = current_offset + pos;
                        return Some((abs_start, abs_start + pattern.len()));
                    }
                }
            }
            current_offset += line_str.len();
            if current_offset < end_offset && bytes.get(current_offset) == Some(&b'\n') {
                current_offset += 1;
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
                    // RuboCop's NodePattern requires .new() with no positional args.
                    // Skip when positional args are present (e.g. `Gem::Specification.new "name", ver`)
                    if node.arguments().is_some() {
                        ruby_prism::visit_call_node(self, node);
                        return;
                    }
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
                                        // Correction: replace 'false' with 'true'
                                        if let Some((start, end)) =
                                            self.find_false_value_byte_range(block_start, block_end)
                                        {
                                            self.correction_info
                                                .push(CorrectionInfo::ReplaceFalse { start, end });
                                        }
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
                                    // Correction: insert metadata line before block end
                                    let block_param = block_node
                                        .parameters()
                                        .and_then(|p| p.as_block_parameters_node())
                                        .and_then(|bp| bp.parameters())
                                        .and_then(|params| params.requireds().iter().next())
                                        .and_then(|r| r.as_required_parameter_node())
                                        .map(|r| r.name().as_slice().to_vec())
                                        .unwrap_or_else(|| b"spec".to_vec());
                                    self.correction_info.push(CorrectionInfo::InsertBeforeEnd {
                                        block_end_offset: block_end,
                                        block_param,
                                    });
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
        metadata_hash_then_bracket = "metadata_hash_then_bracket.rb",
        dynamic_metadata_then_bracket = "dynamic_metadata_then_bracket.rb",
        dynamic_mfa_value = "dynamic_mfa_value.rb",
    );

    #[test]
    fn autocorrect_false_to_true() {
        let input = b"Gem::Specification.new do |spec|\n  spec.metadata['rubygems_mfa_required'] = 'false'\nend\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&RequireMfa, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |spec|\n  spec.metadata['rubygems_mfa_required'] = 'true'\nend\n"
        );
    }

    #[test]
    fn autocorrect_insert_missing_mfa() {
        let input = b"Gem::Specification.new do |spec|\n  spec.name = 'example'\nend\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&RequireMfa, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |spec|\n  spec.name = 'example'\n  spec.metadata['rubygems_mfa_required'] = 'true'\nend\n"
        );
    }

    #[test]
    fn autocorrect_insert_with_different_param_name() {
        let input = b"Gem::Specification.new do |s|\n  s.name = 'example'\nend\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&RequireMfa, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"Gem::Specification.new do |s|\n  s.name = 'example'\n  s.metadata['rubygems_mfa_required'] = 'true'\nend\n"
        );
    }
}
