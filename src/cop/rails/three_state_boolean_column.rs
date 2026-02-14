use crate::cop::util::has_keyword_arg;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ThreeStateBooleanColumn;

impl Cop for ThreeStateBooleanColumn {
    fn name(&self) -> &'static str {
        "Rails/ThreeStateBooleanColumn"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();

        let is_boolean_column = if method == b"add_column" {
            // add_column :table, :col, :boolean
            let args = match call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() < 3 {
                return Vec::new();
            }
            // Third arg should be :boolean
            arg_list[2]
                .as_symbol_node()
                .is_some_and(|s| s.unescaped() == b"boolean")
        } else if method == b"boolean" {
            // t.boolean :col — receiver should be present (the block variable)
            call.receiver().is_some()
        } else {
            false
        };

        if !is_boolean_column {
            return Vec::new();
        }

        // Check if null: false is present
        if has_keyword_arg(&call, b"null") {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Add `null: false` to boolean columns to avoid three-state booleans.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ThreeStateBooleanColumn, "cops/rails/three_state_boolean_column");
}
