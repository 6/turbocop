use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MigrationClassName;

impl Cop for MigrationClassName {
    fn name(&self) -> &'static str {
        "Rails/MigrationClassName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/migrate/**/*.rb"]
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

        // Check if class inherits from ActiveRecord::Migration
        let superclass = match class_node.superclass() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let super_loc = superclass.location();
        let super_bytes = &source.as_bytes()[super_loc.start_offset()..super_loc.end_offset()];

        // Match ActiveRecord::Migration or ActiveRecord::Migration[x.y]
        if !super_bytes.starts_with(b"ActiveRecord::Migration") {
            return Vec::new();
        }

        // Get the class name
        let class_name = class_node.name().as_slice();

        // Check if class name contains lowercase (i.e., not CamelCase)
        let has_lowercase = class_name.iter().any(|&b| b.is_ascii_lowercase());
        let starts_upper = class_name
            .first()
            .is_some_and(|&b| b.is_ascii_uppercase());

        if !starts_upper || !has_lowercase {
            // Doesn't look like CamelCase — flag it
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Migration class name should be CamelCase and match the migration filename."
                    .to_string(),
            )];
        }

        // Check for underscores in the name (not CamelCase)
        if class_name.contains(&b'_') {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Migration class name should be CamelCase and match the migration filename."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MigrationClassName, "cops/rails/migration_class_name");
}
