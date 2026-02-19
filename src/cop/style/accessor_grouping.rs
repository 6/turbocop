use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CLASS_NODE, MODULE_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE};

pub struct AccessorGrouping;

const ACCESSOR_METHODS: &[&str] = &["attr_reader", "attr_writer", "attr_accessor", "attr"];

impl Cop for AccessorGrouping {
    fn name(&self) -> &'static str {
        "Style/AccessorGrouping"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CLASS_NODE, MODULE_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "grouped");

        // Only check class and module bodies
        let body = if let Some(class_node) = node.as_class_node() {
            class_node.body()
        } else if let Some(module_node) = node.as_module_node() {
            module_node.body()
        } else if let Some(sclass) = node.as_singleton_class_node() {
            sclass.body()
        } else {
            return Vec::new();
        };

        let body = match body {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        if enforced_style == "grouped" {
            return check_grouped(self, source, &stmts);
        }

        Vec::new()
    }
}

fn check_grouped(
    cop: &AccessorGrouping,
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
) -> Vec<Diagnostic> {
    use std::collections::HashMap;

    let mut diagnostics = Vec::new();
    // Track accessor method counts per type within current "group"
    // A group is a contiguous sequence of accessor declarations.
    // Any non-accessor statement (def, non-visibility method call, etc.) breaks the group.
    let mut accessor_counts: HashMap<String, Vec<usize>> = HashMap::new();
    let stmt_list: Vec<_> = stmts.body().iter().collect();

    // We need to detect consecutive accessor declarations.
    // Accessors separated by comments, other method calls, or def nodes form separate groups.
    let mut last_was_accessor = false;

    for (idx, stmt) in stmt_list.iter().enumerate() {
        if let Some(call) = stmt.as_call_node() {
            let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

            // Bare access modifier resets the group
            if matches!(name, "private" | "protected" | "public") && call.arguments().is_none() && call.block().is_none() {
                report_grouped_offenses(cop, source, &accessor_counts, &stmt_list, &mut diagnostics);
                accessor_counts.clear();
                last_was_accessor = false;
                continue;
            }

            if ACCESSOR_METHODS.contains(&name) && call.receiver().is_none() {
                // Check if there's a gap (non-accessor statement or comment)
                // since the last accessor
                let has_gap = if !last_was_accessor {
                    true
                } else if idx > 0 {
                    // Check if there's a comment between the previous statement
                    // and this one by looking at the source text between them
                    let prev_end = stmt_list[idx - 1].location().end_offset();
                    let curr_start = stmt.location().start_offset();
                    let between = &source.as_bytes()[prev_end..curr_start];
                    let between_str = std::str::from_utf8(between).unwrap_or("");
                    between_str.lines().any(|l| l.trim().starts_with('#'))
                } else {
                    false
                };

                if has_gap && !accessor_counts.is_empty() {
                    report_grouped_offenses(cop, source, &accessor_counts, &stmt_list, &mut diagnostics);
                    accessor_counts.clear();
                }
                accessor_counts
                    .entry(name.to_string())
                    .or_default()
                    .push(idx);
                last_was_accessor = true;
                continue;
            }
        }
        // Non-accessor statement
        last_was_accessor = false;
    }

    // Report any remaining group
    report_grouped_offenses(cop, source, &accessor_counts, &stmt_list, &mut diagnostics);

    diagnostics
}

fn report_grouped_offenses(
    cop: &AccessorGrouping,
    source: &SourceFile,
    accessor_counts: &std::collections::HashMap<String, Vec<usize>>,
    stmt_list: &[ruby_prism::Node<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (accessor_type, indices) in accessor_counts {
        if indices.len() > 1 {
            for &idx in indices {
                if let Some(stmt) = stmt_list.get(idx) {
                    let loc = stmt.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        format!("Group together all `{}` attributes.", accessor_type),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AccessorGrouping, "cops/style/accessor_grouping");
}
