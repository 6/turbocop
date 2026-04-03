use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::keyword_arg_value;
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
/// FN=21208 (original): The cop was only checking `create_table` without `comment`.
/// It was missing `add_column`, column type methods inside `create_table` blocks
/// (`t.string`, `t.integer`, `t.column`, `t.references`, etc.), and did not
/// treat `comment: nil` or `comment: ''` as missing. Implemented full coverage
/// matching RuboCop's behavior.
///
/// FN=21208 (still 0% after logic fix): The cop had `default_include` set to
/// `["db/migrate/**/*.rb"]` but RuboCop's SchemaComment has NO Include restriction
/// — it runs on ALL files. This caused three failures:
/// 1. Missed `db/schema.rb` (4,383 FN) — not under `db/migrate/`
/// 2. Missed test/spec/lib files (1,455 FN) — `create_table`/`add_column` in tests
/// 3. Missed ALL migrate files in corpus mode (15,621 FN) — corpus paths have a
///    `vendor/corpus/repo_id/` prefix, so the non-globbed `db/migrate/**/*.rb`
///    pattern never matched.
///
/// Fix: removed `default_include` override. Commit 2026-03-17.
///
/// ## Corpus investigation (2026-03-18) — FP=100 across 23 repos
///
/// Root cause: Sequel ORM migrations use `add_column :col, Type` inside
/// `alter_table` blocks. This call has a nil receiver (like ActiveRecord's
/// `add_column`), so it was incorrectly flagged. The key difference is the
/// argument count: ActiveRecord's `add_column` requires 3 positional args
/// (`table, column, type`), while Sequel's takes only 2 (`column, type`).
///
/// RuboCop's cop uses the node-pattern `(send nil? :add_column _table _column
/// _type _?)` which inherently requires 3+ positional args. nitrocop's check
/// lacked this constraint.
///
/// Fix: skip `add_column` calls with fewer than 3 positional arguments.
/// This correctly excludes Sequel's `add_column :col, Type` (2 args) while
/// still flagging ActiveRecord's `add_column :table, :col, :type` (3 args).
///
/// Other FP sources (state_machine gem, 9 FPs from pluginaweek/state_machine)
/// may use similar 2-arg patterns and should be covered by the same fix.
///
/// ## Corpus investigation (2026-03-18) — FP=17 across 10 repos, FN=184 across 5 repos
///
/// FP root cause: nitrocop flagged `create_table` calls regardless of argument
/// count. RuboCop's node pattern `(send nil? :create_table _table _?)` only
/// matches calls with 1-2 argument children in the parser gem AST (where keyword
/// hash and `&block` (block_pass) each count as a child). Calls with 0 args
/// (bare `create_table` method calls), 3 positional args, or 2 positional +
/// `&block` were incorrectly flagged.
///
/// FN root cause: nitrocop's `add_column` check used `positional_arg_count`
/// which excluded keyword hash nodes from the count. But RuboCop's pattern
/// `(send nil? :add_column _table _column _type _?)` counts ALL children
/// including keyword hash. So `add_column :col, :text, null: false` has 3
/// children in the parser AST (sym, sym, hash) and matches. nitrocop only saw
/// 2 positional args and skipped it.
///
/// Fix for FN: introduced `parser_arg_count()` that counts all argument nodes
/// including keyword hash, plus 1 for `&block` (BlockArgumentNode in Prism's
/// `call.block()` vs block_pass in parser gem's send children). Applied
/// arg-count gate for `add_column` (requires 3-4 parser-gem args).
///
/// ## Corpus fix (2026-03-19) — create_table arg-count gate
///
/// Applied `parser_arg_count` gate (1-2 args) to `create_table` calls, matching
/// RuboCop's node pattern `(send nil? :create_table _table _?)`. This filters:
/// - Bare `create_table` with 0 args (5 FPs: custom Schema classes, adapters)
/// - `create_table` with 3+ parser-gem args (12 FPs: custom wrappers, `&block` pass)
///
/// Previous attempts (3 reverts) attributed FN=~2,481 to the gate, but the gap
/// was actually an infrastructure regression (~2,498 fewer offenses even on fully
/// reverted code, caused by config resolution differences between `--rerun` mode
/// and CI corpus oracle). The `parser_arg_count` gate is correct: `do...end` blocks
/// are `BlockNode` (not `BlockArgumentNode`), so they don't increment the count,
/// and standard `create_table :users, force: true do |t| end` correctly gets argc=2.
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

/// Count argument children as the parser gem would see them.
///
/// In the parser gem, ALL children after the method name in a `send` node
/// are counted: positional args, keyword hash, splat, AND `block_pass`
/// (`&block`). In Prism, `&block` is stored in `call.block()` rather than
/// in `call.arguments()`, so we add 1 when the block is a BlockArgumentNode.
fn parser_arg_count(call: &ruby_prism::CallNode<'_>) -> usize {
    let explicit_args = call.arguments().map(|a| a.arguments().len()).unwrap_or(0);
    let block_pass = call
        .block()
        .is_some_and(|b| b.as_block_argument_node().is_some());
    explicit_args + block_pass as usize
}

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
            b"create_table" if call.receiver().is_none() => {
                // RuboCop pattern: (send nil? :create_table _table _?)
                // Matches 1-2 argument children in the parser gem AST.
                // This filters bare `create_table` (0 args) and custom wrappers
                // with 3+ args (e.g., `create_table name, cols, pk`).
                let argc = parser_arg_count(&call);
                if !(1..=2).contains(&argc) {
                    return;
                }
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
                // RuboCop pattern: (send nil? :add_column _table _column _type _?)
                // Matches 3-4 argument children in the parser gem AST.
                // This counts ALL children (positional, keyword hash, block_pass).
                let argc = parser_arg_count(&call);
                if !(3..=4).contains(&argc) {
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

        // RuboCop's t_column? pattern uses `_var` which matches any receiver
        // including nil. This allows both ActiveRecord (`t.string :name`) and
        // Sequel (`column :name, :text`) column calls inside create_table blocks.

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
