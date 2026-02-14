use crate::cop::util::{class_body_calls, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActiveRecordCallbacksOrder;

const CALLBACK_ORDER: &[&[u8]] = &[
    b"before_validation",
    b"after_validation",
    b"before_save",
    b"around_save",
    b"before_create",
    b"before_update",
    b"around_create",
    b"around_update",
    b"after_create",
    b"after_update",
    b"after_save",
    b"before_commit",
    b"after_commit",
    b"after_rollback",
    b"before_destroy",
    b"around_destroy",
    b"after_destroy",
];

fn callback_order_index(name: &[u8]) -> Option<usize> {
    CALLBACK_ORDER.iter().position(|&c| c == name)
}

impl Cop for ActiveRecordCallbacksOrder {
    fn name(&self) -> &'static str {
        "Rails/ActiveRecordCallbacksOrder"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        let calls = class_body_calls(&class);

        // Collect (callback_name, order_index, offset) for known callbacks
        let mut callbacks: Vec<(&[u8], usize, usize)> = Vec::new();

        for call in &calls {
            for &cb_name in CALLBACK_ORDER {
                if is_dsl_call(call, cb_name) {
                    if let Some(idx) = callback_order_index(cb_name) {
                        let loc = call.message_loc().unwrap_or(call.location());
                        callbacks.push((cb_name, idx, loc.start_offset()));
                    }
                    break;
                }
            }
        }

        let mut diagnostics = Vec::new();
        let mut max_seen_idx = 0;
        let mut max_seen_name: &[u8] = b"";

        for &(name, idx, offset) in &callbacks {
            if idx < max_seen_idx {
                let (line, column) = source.offset_to_line_col(offset);
                let name_str = String::from_utf8_lossy(name);
                let other_str = String::from_utf8_lossy(max_seen_name);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Callback `{name_str}` should appear before `{other_str}`."),
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
    crate::cop_fixture_tests!(
        ActiveRecordCallbacksOrder,
        "cops/rails/active_record_callbacks_order"
    );
}
