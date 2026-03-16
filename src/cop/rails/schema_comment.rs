use crate::cop::node_type::CALL_NODE;
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Enforces the use of the `comment` option when adding a new table or column
/// to the database during a migration.
///
/// ## Corpus investigation (2026-03-16)
///
/// Corpus oracle reported FP=0, FN=21208.
///
/// FN=21208: The cop was only checking `create_table` without `comment`. It was
/// missing `add_column`, column type methods inside `create_table` blocks
/// (`t.string`, `t.integer`, `t.column`, `t.references`, etc.), and did not
/// treat `comment: nil` or `comment: ''` as missing. Implemented full coverage
/// matching RuboCop's behavior.
pub struct SchemaComment;

const TABLE_MSG: &str = "New database table without `comment`.";
const COLUMN_MSG: &str = "New database column without `comment`.";

/// All column type methods that RuboCop's SchemaComment cop checks inside
/// `create_table` blocks. Matches RuboCop's CREATE_TABLE_COLUMN_METHODS set.
const CREATE_TABLE_COLUMN_METHODS: &[&[u8]] = &[
    // RAILS_ABSTRACT_SCHEMA_DEFINITIONS
    b"bigint",
    b"binary",
    b"boolean",
    b"date",
    b"datetime",
    b"decimal",
    b"float",
    b"integer",
    b"json",
    b"string",
    b"text",
    b"time",
    b"timestamp",
    b"virtual",
    // RAILS_ABSTRACT_SCHEMA_DEFINITIONS_HELPERS
    b"column",
    b"references",
    b"belongs_to",
    b"primary_key",
    b"numeric",
    // POSTGRES_SCHEMA_DEFINITIONS
    b"bigserial",
    b"bit",
    b"bit_varying",
    b"cidr",
    b"citext",
    b"daterange",
    b"hstore",
    b"inet",
    b"interval",
    b"int4range",
    b"int8range",
    b"jsonb",
    b"ltree",
    b"macaddr",
    b"money",
    b"numrange",
    b"oid",
    b"point",
    b"line",
    b"lseg",
    b"box",
    b"path",
    b"polygon",
    b"circle",
    b"serial",
    b"tsrange",
    b"tstzrange",
    b"tsvector",
    b"uuid",
    b"xml",
    // MYSQL_SCHEMA_DEFINITIONS
    b"blob",
    b"tinyblob",
    b"mediumblob",
    b"longblob",
    b"tinytext",
    b"mediumtext",
    b"longtext",
    b"unsigned_integer",
    b"unsigned_bigint",
    b"unsigned_float",
    b"unsigned_decimal",
];

/// Check whether a call node has a `comment` keyword arg with a non-nil,
/// non-empty-string value. Returns `true` if the comment is present and valid.
fn has_valid_comment(call: &ruby_prism::CallNode<'_>) -> bool {
    match keyword_arg_value(call, b"comment") {
        None => false,
        Some(val) => {
            // comment: nil → offense
            if val.as_nil_node().is_some() {
                return false;
            }
            // comment: '' → offense
            if let Some(s) = val.as_string_node() {
                if s.unescaped().is_empty() {
                    return false;
                }
            }
            true
        }
    }
}

/// Check if a call is a column type method (one of CREATE_TABLE_COLUMN_METHODS)
fn is_column_method(name: &[u8]) -> bool {
    CREATE_TABLE_COLUMN_METHODS.contains(&name)
}

impl Cop for SchemaComment {
    fn name(&self) -> &'static str {
        "Rails/SchemaComment"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();

        match name {
            b"create_table" => {
                if !has_valid_comment(&call) {
                    // Table without comment — only report table-level offense
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, TABLE_MSG.to_string()));
                } else {
                    // Table has comment — check column definitions inside the block
                    if let Some(block) = call.block() {
                        if let Some(block_node) = block.as_block_node() {
                            if let Some(body) = block_node.body() {
                                self.check_block_columns(source, &body, diagnostics);
                            }
                        }
                    }
                }
            }
            b"add_column" => {
                if call.receiver().is_some() {
                    return;
                }
                if !has_valid_comment(&call) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, COLUMN_MSG.to_string()));
                }
            }
            _ => {}
        }
    }
}

impl SchemaComment {
    fn check_block_columns(
        &self,
        source: &SourceFile,
        body: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(stmts) = body.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.check_column_call(source, &stmt, diagnostics);
            }
        } else {
            // Single statement body
            self.check_column_call(source, body, diagnostics);
        }
    }

    fn check_column_call(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if !is_column_method(name) {
            return;
        }

        // Must have a receiver (e.g., `t.string`, not just `string`)
        if call.receiver().is_none() {
            return;
        }

        if !has_valid_comment(&call) {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(source, line, column, COLUMN_MSG.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SchemaComment, "cops/rails/schema_comment");
}
