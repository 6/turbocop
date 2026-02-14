use crate::cop::util::class_body_calls;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AfterCommitOverride;

const AFTER_COMMIT_METHODS: &[&[u8]] = &[
    b"after_commit",
    b"after_create_commit",
    b"after_update_commit",
    b"after_destroy_commit",
    b"after_save_commit",
];

impl Cop for AfterCommitOverride {
    fn name(&self) -> &'static str {
        "Rails/AfterCommitOverride"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let calls = class_body_calls(&class_node);
        let after_commit_calls: Vec<_> = calls
            .iter()
            .filter(|c| {
                c.receiver().is_none()
                    && AFTER_COMMIT_METHODS.contains(&c.name().as_slice())
            })
            .collect();

        if after_commit_calls.len() >= 2 {
            // Flag the second and subsequent occurrences
            let mut diagnostics = Vec::new();
            for call in &after_commit_calls[1..] {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Multiple `after_commit` callbacks may override each other.".to_string(),
                ));
            }
            return diagnostics;
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AfterCommitOverride, "cops/rails/after_commit_override");
}
