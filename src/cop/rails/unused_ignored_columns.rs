use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::cop::shared::node_type::{CALL_NODE, CALL_OPERATOR_WRITE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/UnusedIgnoredColumns
///
/// Checks that columns listed in `ignored_columns` actually exist in the schema.
/// Reports offense on each column string/symbol that doesn't exist in the table.
///
/// Prism parses `self.ignored_columns += %w[...]` as `CallOperatorWriteNode`, not
/// `CallNode`. That meant appended literal word arrays in model class bodies were
/// missed entirely, including multi-line `%w(...)` lists from Mastodon and Whitehall.
/// This cop now visits `CALL_OPERATOR_WRITE_NODE` and checks only
/// `self.ignored_columns += <literal array>` to match RuboCop's `on_op_asgn`.
///
/// Corpus runs also invoke nitrocop with overlay configs that can place
/// `config_dir()` outside the target repo. When that happens, the global schema
/// singleton is unset because `db/schema.rb` is looked up in the wrong directory.
/// This cop now falls back to loading `db/schema.rb` relative to the current source
/// file's repo root when the global schema is unavailable.
///
/// ## Synthetic corpus note
/// RuboCop's SchemaLoader crashes on `t.timestamps` (no arguments) in
/// db/schema.rb because `Column.new` calls `node.first_argument.str_content`
/// which raises NoMethodError on nil. When schema loading fails, both RuboCop
/// and nitrocop silently skip schema-dependent cops. The synthetic schema was
/// fixed to use explicit `t.datetime "created_at"` columns instead.
pub struct UnusedIgnoredColumns;

fn schema_for_source(source: &SourceFile) -> Option<&'static crate::schema::Schema> {
    if let Some(schema) = crate::schema::get() {
        return Some(schema);
    }

    static FALLBACK_SCHEMAS: OnceLock<
        Mutex<HashMap<PathBuf, Option<&'static crate::schema::Schema>>>,
    > = OnceLock::new();

    let repo_root = source
        .path
        .ancestors()
        .find(|path| path.join("db/schema.rb").is_file())?
        .to_path_buf();

    let mut cache = FALLBACK_SCHEMAS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()?;

    if let Some(schema) = cache.get(&repo_root).copied() {
        return schema;
    }

    let schema = std::fs::read(repo_root.join("db/schema.rb"))
        .ok()
        .and_then(|bytes| crate::schema::Schema::parse(&bytes))
        .map(|schema| Box::leak(Box::new(schema)) as &'static crate::schema::Schema);

    cache.insert(repo_root, schema);
    schema
}

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
        &[CALL_NODE, CALL_OPERATOR_WRITE_NODE]
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
        let schema = match schema_for_source(source) {
            Some(s) => s,
            None => return,
        };

        let array_node = if let Some(call) = node.as_call_node() {
            if call.name().as_slice() != b"ignored_columns=" {
                return;
            }

            if call
                .receiver()
                .is_none_or(|receiver| receiver.as_self_node().is_none())
            {
                return;
            }

            let args = match call.arguments() {
                Some(arguments) => arguments,
                None => return,
            };

            match args
                .arguments()
                .iter()
                .next()
                .and_then(|arg| arg.as_array_node())
            {
                Some(array) => array,
                None => return,
            }
        } else if let Some(op_write) = node.as_call_operator_write_node() {
            if op_write.read_name().as_slice() != b"ignored_columns"
                || op_write.binary_operator().as_slice() != b"+"
            {
                return;
            }

            if op_write
                .receiver()
                .is_none_or(|receiver| receiver.as_self_node().is_none())
            {
                return;
            }

            match op_write.value().as_array_node() {
                Some(array) => array,
                None => return,
            }
        } else {
            return;
        };

        // Resolve table name
        let class_name = match crate::schema::find_enclosing_class_name(
            source.as_bytes(),
            node.location().start_offset(),
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
            include_bytes!("../../../tests/fixtures/cops/rails/unused_ignored_columns/schema.rb");
        let schema = Schema::parse(schema_bytes).unwrap();
        crate::schema::set_test_schema(Some(schema));
    }

    #[test]
    fn offense_fixture() {
        setup_schema();
        crate::testutil::assert_cop_offenses_full(
            &UnusedIgnoredColumns,
            include_bytes!("../../../tests/fixtures/cops/rails/unused_ignored_columns/offense.rb"),
        );
        crate::schema::set_test_schema(None);
    }

    #[test]
    fn no_offense_fixture() {
        setup_schema();
        crate::testutil::assert_cop_no_offenses_full(
            &UnusedIgnoredColumns,
            include_bytes!(
                "../../../tests/fixtures/cops/rails/unused_ignored_columns/no_offense.rb"
            ),
        );
        crate::schema::set_test_schema(None);
    }
}
