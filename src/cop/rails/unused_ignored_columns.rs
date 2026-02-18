use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/UnusedIgnoredColumns
///
/// This cop requires schema analysis (db/schema.rb parsing) to determine
/// whether a column actually exists in the database. RuboCop's implementation
/// loads the schema and checks column existence. Since rblint does not perform
/// schema analysis, this cop is not registered and never fires.
pub struct UnusedIgnoredColumns;

impl Cop for UnusedIgnoredColumns {
    fn name(&self) -> &'static str {
        "Rails/UnusedIgnoredColumns"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/models/**/*.rb"]
    }

    fn check_node(
        &self,
        _source: &SourceFile,
        _node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // No-op: requires schema analysis
        Vec::new()
    }
}

// No fixture tests: this cop requires schema analysis and never fires in rblint.
// Test fixtures exist at testdata/cops/rails/unused_ignored_columns/ for future use
// when/if schema analysis is implemented.
