use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/UnusedIgnoredColumns
///
/// Checks that columns listed in `ignored_columns` actually exist in the schema.
/// Reports offense on each column string/symbol that doesn't exist in the table.
pub struct UnusedIgnoredColumns;

impl Cop for UnusedIgnoredColumns {
    fn name(&self) -> &'static str {
        "Rails/UnusedIgnoredColumns"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/models/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let schema = match crate::schema::get() {
            Some(s) => s,
            None => return,
        };

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_str = std::str::from_utf8(method_name.as_slice()).unwrap_or("");

        // Match `self.ignored_columns = [...]`
        if method_str != "ignored_columns=" {
            return;
        }

        // Get the array argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        let array_node = match arg_list.first().and_then(|n| n.as_array_node()) {
            Some(a) => a,
            None => return,
        };

        // Resolve table name
        let class_name = match crate::schema::find_enclosing_class_name(
            source.as_bytes(),
            call.location().start_offset(),
            parse_result,
        ) {
            Some(n) => n,
            None => return,
        };
        let table_name = crate::schema::table_name_from_source(source.as_bytes(), &class_name);

        let table = match schema.table_by(&table_name) {
            Some(t) => t,
            None => return,
        };

        // Check each column in the array
        for elem in array_node.elements().iter() {
            let col_name = if let Some(sym) = elem.as_symbol_node() {
                String::from_utf8_lossy(sym.unescaped()).to_string()
            } else if let Some(s) = elem.as_string_node() {
                String::from_utf8_lossy(s.unescaped()).to_string()
            } else {
                continue;
            };

            if !table.has_column(&col_name) {
                let loc = elem.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Remove `{col_name}` from `ignored_columns` because the column does not exist."
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;

    fn setup_schema() {
        let schema_bytes =
            include_bytes!("../../../testdata/cops/rails/unused_ignored_columns/schema.rb");
        let schema = Schema::parse(schema_bytes).unwrap();
        crate::schema::set_test_schema(Some(schema));
    }

    #[test]
    fn offense_fixture() {
        setup_schema();
        crate::testutil::assert_cop_offenses_full(
            &UnusedIgnoredColumns,
            include_bytes!("../../../testdata/cops/rails/unused_ignored_columns/offense.rb"),
        );
        crate::schema::set_test_schema(None);
    }

    #[test]
    fn no_offense_fixture() {
        setup_schema();
        crate::testutil::assert_cop_no_offenses_full(
            &UnusedIgnoredColumns,
            include_bytes!("../../../testdata/cops/rails/unused_ignored_columns/no_offense.rb"),
        );
        crate::schema::set_test_schema(None);
    }
}
