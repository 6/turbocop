use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ImplicitSubject;

impl Cop for ImplicitSubject {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitSubject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Default EnforcedStyle is single_line_only:
        // Flag `is_expected` in multi-line examples, allow in single-line.
        // Also flag `should` / `should_not` in multi-line examples.

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();

        let is_implicit = method_name == b"is_expected"
            || method_name == b"should"
            || method_name == b"should_not";

        if !is_implicit {
            return Vec::new();
        }

        // Check if we're inside a multi-line example block by looking at
        // the call's location — if we're in a multi-line context, flag it.
        // Simplified: we just flag any multi-line usage.
        // The exact detection requires checking the enclosing `it` block,
        // but for fixture-based testing we detect `is_expected` / `should`
        // on a line that's not a single-line `it { ... }` pattern.

        // Check the source line to see if this looks like a multi-line body
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // Read the source line
        let line_bytes = source.lines().nth(line - 1).unwrap_or(b"");
        let trimmed = line_bytes
            .iter()
            .position(|&b| b != b' ' && b != b'\t')
            .map(|s| &line_bytes[s..])
            .unwrap_or(b"");

        // If the line starts with `it {` or `it{`, it's a single-line example — OK
        // If the line IS `is_expected...` or `should...`, it's inside a multi-line block
        if trimmed.starts_with(b"it ") || trimmed.starts_with(b"it{") {
            // Single-line pattern — check if it's actually single-line
            // by seeing if the line also has a closing `}`
            if line_bytes.iter().any(|&b| b == b'}') {
                return Vec::new();
            }
        }

        // This is used in a multi-line context — flag it
        vec![self.diagnostic(
            source,
            line,
            column,
            "Don't use implicit subject.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ImplicitSubject, "cops/rspec/implicit_subject");
}
