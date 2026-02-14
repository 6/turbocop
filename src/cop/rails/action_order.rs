use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActionOrder;

const STANDARD_ORDER: &[&[u8]] = &[
    b"index", b"show", b"new", b"edit", b"create", b"update", b"destroy",
];

fn action_order_index(name: &[u8]) -> Option<usize> {
    STANDARD_ORDER.iter().position(|&a| a == name)
}

impl Cop for ActionOrder {
    fn name(&self) -> &'static str {
        "Rails/ActionOrder"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["app/controllers/**/*.rb"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let body = match class.body() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Collect (method_name, order_index, offset) for standard actions
        let mut actions: Vec<(&[u8], usize, usize)> = Vec::new();

        for node in stmts.body().iter() {
            if let Some(def_node) = node.as_def_node() {
                let name = def_node.name().as_slice();
                if let Some(idx) = action_order_index(name) {
                    let offset = def_node.def_keyword_loc().start_offset();
                    actions.push((name, idx, offset));
                }
            }
        }

        let mut diagnostics = Vec::new();
        let mut max_seen_idx = 0;
        let mut max_seen_name: &[u8] = b"";

        for &(name, idx, offset) in &actions {
            if idx < max_seen_idx {
                let (line, column) = source.offset_to_line_col(offset);
                let name_str = String::from_utf8_lossy(name);
                let other_str = String::from_utf8_lossy(max_seen_name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Action `{name_str}` should appear before `{other_str}` in the controller."
                    ),
                ));
            }
            if idx >= max_seen_idx {
                max_seen_idx = idx;
                max_seen_name = name;
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActionOrder, "cops/rails/action_order");
}
