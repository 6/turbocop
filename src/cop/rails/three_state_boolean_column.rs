use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ThreeStateBooleanColumn;

impl Cop for ThreeStateBooleanColumn {
    fn name(&self) -> &'static str {
        "Rails/ThreeStateBooleanColumn"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = ThreeStateBooleanVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            def_body: None,
            current_table_name: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Visitor that tracks method def context and create_table/change_table block context.
struct ThreeStateBooleanVisitor<'a, 'pr> {
    cop: &'a ThreeStateBooleanColumn,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// The body node of the enclosing def, if any (for searching change_column_null).
    def_body: Option<ruby_prism::Node<'pr>>,
    /// The current table name from an enclosing create_table/change_table block.
    current_table_name: Option<Vec<u8>>,
}

impl<'pr> Visit<'pr> for ThreeStateBooleanVisitor<'_, 'pr> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old_def_body = self.def_body.take();
        self.def_body = node.body().map(ruby_prism::Node::from);
        // Visit children (body of the def)
        ruby_prism::visit_def_node(self, node);
        self.def_body = old_def_body;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // If this is a create_table/change_table call with a block, set table context
        // and visit the block body with that context.
        if (method == b"create_table" || method == b"change_table") && node.block().is_some() {
            let old_table_name = self.current_table_name.take();
            // Extract table name from first argument
            if let Some(args) = node.arguments() {
                let arg_list: Vec<ruby_prism::Node<'pr>> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    self.current_table_name = extract_name_value(&arg_list[0]);
                }
            }
            // Visit children (including the block body)
            ruby_prism::visit_call_node(self, node);
            self.current_table_name = old_table_name;
            return;
        }

        // Determine if this is a boolean column call and extract table/column info
        let boolean_info = if method == b"add_column" {
            check_add_column(node)
        } else if method == b"column" {
            check_column_method(node, &self.current_table_name)
        } else if method == b"boolean" {
            check_boolean_method(node, &self.current_table_name)
        } else {
            None
        };

        if let Some(info) = boolean_info {
            // Check if required options (default: non-nil AND null: false) are present
            let has_default = keyword_arg_value(node, b"default")
                .is_some_and(|v| v.as_nil_node().is_none());
            let has_null_false =
                keyword_arg_value(node, b"null").is_some_and(|v| v.as_false_node().is_some());

            if !(has_default && has_null_false) {
                // Check the vendor's skip condition:
                // If inside a def AND (table_name unknown OR change_column_null found), skip
                let should_skip = self.def_body.is_some()
                    && (info.table_name.is_none()
                        || self.has_change_column_null(
                            info.table_name.as_deref(),
                            info.column_name.as_deref(),
                        ));

                if !should_skip {
                    let loc = node.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Boolean columns should always have a default value and a `NOT NULL` constraint."
                            .to_string(),
                    ));
                }
            }
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }
}

/// Information about a boolean column call.
struct BooleanColumnInfo {
    table_name: Option<Vec<u8>>,
    column_name: Option<Vec<u8>>,
}

impl ThreeStateBooleanVisitor<'_, '_> {
    /// Search the current def body for a `change_column_null(table, column, false)` call
    /// that matches the given table and column names.
    fn has_change_column_null(
        &self,
        table_name: Option<&[u8]>,
        column_name: Option<&[u8]>,
    ) -> bool {
        let def_body = match &self.def_body {
            Some(body) => body,
            None => return false,
        };
        let table = match table_name {
            Some(t) => t,
            None => return false,
        };
        let column = match column_name {
            Some(c) => c,
            None => return false,
        };
        let mut finder = ChangeColumnNullFinder {
            table_name: table,
            column_name: column,
            found: false,
        };
        finder.visit(def_body);
        finder.found
    }
}

/// Visitor that searches for `change_column_null(:table, :column, false)`.
struct ChangeColumnNullFinder<'a> {
    table_name: &'a [u8],
    column_name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for ChangeColumnNullFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }
        if node.name().as_slice() == b"change_column_null" && node.receiver().is_none() {
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                // change_column_null :table, :column, false
                if arg_list.len() >= 3 {
                    let table_matches = extract_name_value(&arg_list[0])
                        .is_some_and(|v| v == self.table_name);
                    let column_matches = extract_name_value(&arg_list[1])
                        .is_some_and(|v| v == self.column_name);
                    let is_false = arg_list[2].as_false_node().is_some();
                    if table_matches && column_matches && is_false {
                        self.found = true;
                        return;
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

/// Extract a name value from a symbol or string node, returning the bytes.
fn extract_name_value(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    if let Some(sym) = node.as_symbol_node() {
        return Some(sym.unescaped().to_vec());
    }
    if let Some(s) = node.as_string_node() {
        return Some(s.unescaped().to_vec());
    }
    None
}

/// Check `add_column :table, :col, :boolean` -- returns BooleanColumnInfo if it's a boolean column.
fn check_add_column(call: &ruby_prism::CallNode<'_>) -> Option<BooleanColumnInfo> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() < 3 {
        return None;
    }
    // Third arg should be :boolean or "boolean"
    let is_boolean = arg_list[2]
        .as_symbol_node()
        .is_some_and(|s| s.unescaped() == b"boolean")
        || arg_list[2]
            .as_string_node()
            .is_some_and(|s| s.unescaped() == b"boolean");
    if !is_boolean {
        return None;
    }
    Some(BooleanColumnInfo {
        table_name: extract_name_value(&arg_list[0]),
        column_name: extract_name_value(&arg_list[1]),
    })
}

/// Check `t.column :col, :boolean` -- returns BooleanColumnInfo if it's a boolean column.
fn check_column_method(
    call: &ruby_prism::CallNode<'_>,
    current_table_name: &Option<Vec<u8>>,
) -> Option<BooleanColumnInfo> {
    // Must have a receiver (the block variable `t`)
    call.receiver()?;
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() < 2 {
        return None;
    }
    let is_boolean = arg_list[1]
        .as_symbol_node()
        .is_some_and(|s| s.unescaped() == b"boolean")
        || arg_list[1]
            .as_string_node()
            .is_some_and(|s| s.unescaped() == b"boolean");
    if !is_boolean {
        return None;
    }
    Some(BooleanColumnInfo {
        table_name: current_table_name.clone(),
        column_name: extract_name_value(&arg_list[0]),
    })
}

/// Check `t.boolean :col` -- returns BooleanColumnInfo if it's a boolean column call with a receiver.
fn check_boolean_method(
    call: &ruby_prism::CallNode<'_>,
    current_table_name: &Option<Vec<u8>>,
) -> Option<BooleanColumnInfo> {
    // Must have a receiver (the block variable `t`)
    call.receiver()?;
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None;
    }
    Some(BooleanColumnInfo {
        table_name: current_table_name.clone(),
        column_name: extract_name_value(&arg_list[0]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ThreeStateBooleanColumn, "cops/rails/three_state_boolean_column");
}
