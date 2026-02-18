use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AccessorGrouping;

const ACCESSOR_METHODS: &[&str] = &["attr_reader", "attr_writer", "attr_accessor", "attr"];

impl Cop for AccessorGrouping {
    fn name(&self) -> &'static str {
        "Style/AccessorGrouping"
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
    // Track accessor method counts per type within current access modifier scope
    let mut accessor_counts: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, stmt) in stmts.body().iter().enumerate() {
        // Reset counters on access modifier boundaries
        if let Some(call) = stmt.as_call_node() {
            let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            if matches!(name, "private" | "protected" | "public") && call.arguments().is_none() {
                accessor_counts.clear();
                continue;
            }
        }

        if let Some(call) = stmt.as_call_node() {
            let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            if ACCESSOR_METHODS.contains(&name) && call.receiver().is_none() {
                accessor_counts
                    .entry(name.to_string())
                    .or_default()
                    .push(idx);
            }
        }
    }

    // Report offenses for accessor types that appear more than once
    for (accessor_type, indices) in &accessor_counts {
        if indices.len() > 1 {
            for &idx in indices {
                let stmt_list: Vec<_> = stmts.body().iter().collect();
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

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AccessorGrouping, "cops/style/accessor_grouping");
}
