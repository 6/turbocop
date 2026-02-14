use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ReversibleMigrationMethodDefinition;

impl Cop for ReversibleMigrationMethodDefinition {
    fn name(&self) -> &'static str {
        "Rails/ReversibleMigrationMethodDefinition"
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
        // Check if it inherits from a Migration class
        let superclass = match class_node.superclass() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let super_loc = superclass.location();
        let super_text = &source.as_bytes()[super_loc.start_offset()..super_loc.end_offset()];
        // Match ActiveRecord::Migration or ActiveRecord::Migration[x.y]
        if !super_text.starts_with(b"ActiveRecord::Migration") {
            return Vec::new();
        }

        let body = match class_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut has_up = false;
        let mut has_down = false;
        let mut has_change = false;

        for stmt in stmts.body().iter() {
            if let Some(def_node) = stmt.as_def_node() {
                let name = def_node.name().as_slice();
                match name {
                    b"up" => has_up = true,
                    b"down" => has_down = true,
                    b"change" => has_change = true,
                    _ => {}
                }
            }
        }

        // If has `change`, it's fine (reversible)
        if has_change {
            return Vec::new();
        }

        // If has `up` but not `down`, flag
        if has_up && !has_down {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Define both `up` and `down` methods, or use `change` for reversible migrations."
                    .to_string(),
            )];
        }

        // If has `down` but not `up`, also flag
        if has_down && !has_up {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Define both `up` and `down` methods, or use `change` for reversible migrations."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ReversibleMigrationMethodDefinition,
        "cops/rails/reversible_migration_method_definition"
    );
}
