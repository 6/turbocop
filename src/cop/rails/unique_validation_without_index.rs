use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/UniqueValidationWithoutIndex
///
/// This cop requires schema analysis (db/schema.rb parsing) to determine
/// whether a table has a unique index. RuboCop's implementation checks
/// `return unless schema` and only fires when it can verify the table
/// lacks a unique index. Since rblint does not perform schema analysis,
/// this cop is not registered and never fires.
pub struct UniqueValidationWithoutIndex;

impl Cop for UniqueValidationWithoutIndex {
    fn name(&self) -> &'static str {
        "Rails/UniqueValidationWithoutIndex"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        _source: &SourceFile,
        _node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
    }
}
