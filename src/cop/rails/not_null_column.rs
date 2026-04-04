use crate::cop::shared::method_dispatch_predicates;
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{keyword_arg_pair_start_offset, keyword_arg_value};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks `add_column`, `add_reference`, and `change_table` column additions
/// for `null: false` without a real default value.
///
/// Corpus investigation (2026-04-04):
/// - Fixed FNs for `add_reference` and `change_table` body calls like
///   `t.string`, `t.references`, and `t.integer`, including legacy hash-rocket
///   option hashes.
/// - Fixed multiline `add_column`/`change_table` location mismatches by
///   reporting on the `null` pair instead of the call start, which removes the
///   paired Coursemology FP/FN cases.
/// - Fixed the malformed `add_column :table, :column, :null => false` FP by
///   requiring RuboCop's 3 positional arguments plus an options hash.
/// - Matched RuboCop's `default: nil` behavior: it still counts as missing a
///   default and remains an offense.
pub struct NotNullColumn;

const MSG: &str = "Do not add a NOT NULL column without a default value.";

impl Cop for NotNullColumn {
    fn name(&self) -> &'static str {
        "Rails/NotNullColumn"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/migrate/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let database_is_mysql = config.get_str("Database", "") == "mysql";

        let offense_offset = if method_dispatch_predicates::is_command(&call, b"add_column") {
            add_column_offense_offset(&call, database_is_mysql)
        } else if method_dispatch_predicates::is_command(&call, b"add_reference") {
            add_reference_offense_offset(&call)
        } else {
            None
        };

        if let Some(offset) = offense_offset {
            push_diagnostic(self, source, diagnostics, offset);
        }

        if method_dispatch_predicates::is_command(&call, b"change_table") {
            check_change_table(self, source, &call, database_is_mysql, diagnostics);
        }
    }
}

fn push_diagnostic(
    cop: &NotNullColumn,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    offset: usize,
) {
    let (line, column) = source.offset_to_line_col(offset);
    diagnostics.push(cop.diagnostic(source, line, column, MSG.to_string()));
}

fn add_column_offense_offset(
    call: &ruby_prism::CallNode<'_>,
    database_is_mysql: bool,
) -> Option<usize> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();

    // RuboCop requires `add_column(table, column, type, options)`.
    if arg_list.len() < 4 {
        return None;
    }
    if skip_typed_call(&arg_list[2], database_is_mysql) {
        return None;
    }

    null_offense_offset(call)
}

fn add_reference_offense_offset(call: &ruby_prism::CallNode<'_>) -> Option<usize> {
    let args = call.arguments()?;
    if args.arguments().len() < 3 {
        return None;
    }

    null_offense_offset(call)
}

fn check_change_table(
    cop: &NotNullColumn,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    database_is_mysql: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((table_var, body)) = change_table_block_parts(call) else {
        return;
    };

    if let Some(statements) = body.as_statements_node() {
        for stmt in statements.body().iter() {
            push_change_table_child_if_offense(
                cop,
                source,
                diagnostics,
                &table_var,
                database_is_mysql,
                &stmt,
            );
        }
        return;
    }

    push_change_table_child_if_offense(
        cop,
        source,
        diagnostics,
        &table_var,
        database_is_mysql,
        &body,
    );
}

fn push_change_table_child_if_offense(
    cop: &NotNullColumn,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    table_var: &[u8],
    database_is_mysql: bool,
    node: &ruby_prism::Node<'_>,
) {
    let Some(call) = node.as_call_node() else {
        return;
    };
    let Some(offset) = change_table_child_offense_offset(&call, table_var, database_is_mysql)
    else {
        return;
    };

    push_diagnostic(cop, source, diagnostics, offset);
}

fn change_table_block_parts<'a>(
    call: &ruby_prism::CallNode<'a>,
) -> Option<(Vec<u8>, ruby_prism::Node<'a>)> {
    let block = call.block()?.as_block_node()?;
    let body = block.body()?;

    let block_params = block.parameters()?.as_block_parameters_node()?;
    let parameters = block_params.parameters()?;
    let requireds = parameters.requireds();

    if requireds.len() != 1
        || !parameters.optionals().is_empty()
        || parameters.rest().is_some()
        || !parameters.posts().is_empty()
        || !parameters.keywords().is_empty()
        || parameters.keyword_rest().is_some()
        || parameters.block().is_some()
    {
        return None;
    }

    let param = requireds.iter().next()?.as_required_parameter_node()?;
    Some((param.name().as_slice().to_vec(), body))
}

fn change_table_child_offense_offset(
    call: &ruby_prism::CallNode<'_>,
    table_var: &[u8],
    database_is_mysql: bool,
) -> Option<usize> {
    if !receiver_matches_local(call, table_var) {
        return None;
    }

    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();

    match call.name().as_slice() {
        b"column" => {
            if arg_list.len() < 3 {
                return None;
            }
            if skip_typed_call(&arg_list[1], database_is_mysql) {
                return None;
            }
            null_offense_offset(call)
        }
        b"add_reference" => {
            if arg_list.len() < 3 {
                return None;
            }
            null_offense_offset(call)
        }
        method_name => {
            // RuboCop's shortcut matcher only covers `t.string :name, ...`-style
            // calls with a single positional column arg plus the options hash.
            if arg_list.len() != 2 {
                return None;
            }
            if skip_shortcut_type(method_name, database_is_mysql) {
                return None;
            }
            null_offense_offset(call)
        }
    }
}

fn receiver_matches_local(call: &ruby_prism::CallNode<'_>, table_var: &[u8]) -> bool {
    call.receiver()
        .and_then(|receiver| receiver.as_local_variable_read_node())
        .is_some_and(|local| local.name().as_slice() == table_var)
}

fn skip_typed_call(type_node: &ruby_prism::Node<'_>, database_is_mysql: bool) -> bool {
    matches_type_name(type_node, b"virtual")
        || (database_is_mysql && matches_type_name(type_node, b"text"))
}

fn matches_type_name(type_node: &ruby_prism::Node<'_>, expected: &[u8]) -> bool {
    type_node
        .as_symbol_node()
        .is_some_and(|symbol| symbol.unescaped() == expected)
        || type_node
            .as_string_node()
            .is_some_and(|string| string.unescaped() == expected)
}

fn skip_shortcut_type(method_name: &[u8], database_is_mysql: bool) -> bool {
    method_name == b"virtual" || (database_is_mysql && method_name == b"text")
}

fn null_offense_offset(call: &ruby_prism::CallNode<'_>) -> Option<usize> {
    if has_non_nil_default(call) {
        return None;
    }

    keyword_arg_value(call, b"null")?.as_false_node()?;
    keyword_arg_pair_start_offset(call, b"null")
}

fn has_non_nil_default(call: &ruby_prism::CallNode<'_>) -> bool {
    keyword_arg_value(call, b"default").is_some_and(|value| value.as_nil_node().is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NotNullColumn, "cops/rails/not_null_column");

    #[test]
    fn mysql_database_skips_text_columns() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Database".to_string(),
                serde_yml::Value::String("mysql".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"add_column :users, :bio, :text, null: false\n";
        let diags = run_cop_full_with_config(&NotNullColumn, source, config);
        assert!(diags.is_empty(), "MySQL should skip TEXT columns");
    }

    #[test]
    fn mysql_database_still_flags_string_columns() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "Database".to_string(),
                serde_yml::Value::String("mysql".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"add_column :users, :name, :string, null: false\n";
        let diags = run_cop_full_with_config(&NotNullColumn, source, config);
        assert!(
            !diags.is_empty(),
            "MySQL should still flag non-text columns"
        );
    }
}
