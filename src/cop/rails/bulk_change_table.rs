use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BulkChangeTable;

/// Combinable alter methods (can be done in a single ALTER TABLE).
const COMBINABLE_ALTER_METHODS: &[&[u8]] = &[
    b"add_column",
    b"remove_column",
    b"add_timestamps",
    b"remove_timestamps",
    b"change_column",
    b"change_column_default",
    b"rename_column",
    b"add_index",
    b"remove_index",
    b"add_reference",
    b"remove_reference",
    b"add_belongs_to",
    b"remove_belongs_to",
];

/// Combinable transformations inside change_table block.
const COMBINABLE_TABLE_METHODS: &[&[u8]] = &[
    b"string",
    b"text",
    b"integer",
    b"bigint",
    b"float",
    b"decimal",
    b"numeric",
    b"datetime",
    b"timestamp",
    b"time",
    b"date",
    b"binary",
    b"boolean",
    b"json",
    b"virtual",
    b"column",
    b"remove",
    b"index",
    b"remove_index",
    b"timestamps",
    b"rename",
    b"change",
    b"change_default",
    b"references",
    b"belongs_to",
    b"remove_references",
    b"remove_belongs_to",
];

/// Extract the table name from the first argument of an alter method call.
fn extract_table_name(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let first = args.arguments().iter().next()?;

    if let Some(sym) = first.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = first.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

/// Check if a change_table call has `bulk: true` or `bulk: false`.
fn has_bulk_option(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            // Check KeywordHashNode (common in call args)
            if let Some(kw) = arg.as_keyword_hash_node() {
                for elem in kw.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        if let Some(sym) = assoc.key().as_symbol_node() {
                            if sym.unescaped() == b"bulk" {
                                return true;
                            }
                        }
                    }
                }
            }
            // Check HashNode (explicit hash literal)
            if let Some(hash) = arg.as_hash_node() {
                for elem in hash.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        if let Some(sym) = assoc.key().as_symbol_node() {
                            if sym.unescaped() == b"bulk" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Count combinable transformations inside a change_table block body.
/// Returns (count, has_conditional) -- conditional blocks like `if` make it not combinable.
fn count_combinable_in_block(block_body: &ruby_prism::Node<'_>) -> (usize, bool) {
    let stmts = match block_body.as_statements_node() {
        Some(s) => s,
        None => return (0, false),
    };

    let mut count = 0;
    let mut has_conditional = false;
    for stmt in stmts.body().iter() {
        // Check for conditionals that would make combining unsafe
        if stmt.as_if_node().is_some() || stmt.as_unless_node().is_some() {
            has_conditional = true;
        }
        // Check for nested blocks (reversible, etc.)
        if let Some(call) = stmt.as_call_node() {
            if call.block().is_some() && call.name().as_slice() != b"remove" {
                has_conditional = true;
            }
        }

        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if call.receiver().is_some() && COMBINABLE_TABLE_METHODS.contains(&name) {
                // For `t.remove`, count multi-column remove as multiple
                if name == b"remove" {
                    if let Some(args) = call.arguments() {
                        let arg_count = args
                            .arguments()
                            .iter()
                            .filter(|a| a.as_keyword_hash_node().is_none())
                            .count();
                        if arg_count > 1 {
                            count += arg_count;
                            continue;
                        }
                    }
                }
                count += 1;
            }
        }
    }
    (count, has_conditional)
}

impl Cop for BulkChangeTable {
    fn name(&self) -> &'static str {
        "Rails/BulkChangeTable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _database = config.get_str("Database", "");
        let _supported_databases = config.get_string_array("SupportedDatabases");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let def_name = def_node.name().as_slice();
        if def_name != b"change" && def_name != b"up" && def_name != b"down" {
            return Vec::new();
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        // Check for change_table without bulk: true that has multiple transformations
        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                if call.name().as_slice() == b"change_table" && call.receiver().is_none() {
                    if has_bulk_option(&call) {
                        continue;
                    }
                    if let Some(block) = call.block() {
                        if let Some(block_node) = block.as_block_node() {
                            if let Some(block_body) = block_node.body() {
                                let (count, has_conditional) =
                                    count_combinable_in_block(&block_body);
                                if count >= 2 && !has_conditional {
                                    let loc = call.location();
                                    let (line, column) =
                                        source.offset_to_line_col(loc.start_offset());
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        column,
                                        "You can combine alter queries using `bulk: true` options."
                                            .to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for multiple combinable alter methods on the same table
        // Group consecutive alter method calls by table name
        let mut table_runs: Vec<(Vec<u8>, Vec<usize>)> = Vec::new();

        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                let name = call.name().as_slice();
                if COMBINABLE_ALTER_METHODS.contains(&name) && call.receiver().is_none() {
                    if let Some(table) = extract_table_name(&call) {
                        // Try to append to existing run for this table
                        let appended = if let Some(last) = table_runs.last_mut() {
                            if last.0 == table {
                                last.1.push(call.location().start_offset());
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        if !appended {
                            table_runs
                                .push((table, vec![call.location().start_offset()]));
                        }
                        continue;
                    }
                }
                // Non-combinable method or different table breaks the run
                table_runs.push((Vec::new(), Vec::new()));
            } else {
                table_runs.push((Vec::new(), Vec::new()));
            }
        }

        // Report offenses for tables with multiple alter methods
        for (table, offsets) in &table_runs {
            if offsets.len() >= 2 && !table.is_empty() {
                let table_str = std::str::from_utf8(table).unwrap_or("table");
                let (line, column) = source.offset_to_line_col(offsets[0]);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "You can use `change_table :{table_str}, bulk: true` to combine alter queries."
                    ),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BulkChangeTable, "cops/rails/bulk_change_table");
}
