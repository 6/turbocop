use crate::cop::node_type::{AND_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, OR_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

const DEFAULT_ALLOWED_METHODS: &[&str] = &["present?", "blank?", "presence", "try", "try!"];

pub struct SafeNavigationConsistency;

impl Cop for SafeNavigationConsistency {
    fn name(&self) -> &'static str {
        "Lint/SafeNavigationConsistency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, OR_NODE]
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
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| {
                DEFAULT_ALLOWED_METHODS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        // Check for `&&` and `||` operators (AndNode and OrNode)
        if let Some(and_node) = node.as_and_node() {
            diagnostics.extend(check_logical_op(
                self,
                source,
                &and_node.left(),
                &and_node.right(),
                true,
                &allowed_methods,
            ));
            return;
        }

        if let Some(or_node) = node.as_or_node() {
            diagnostics.extend(check_logical_op(
                self,
                source,
                &or_node.left(),
                &or_node.right(),
                false,
                &allowed_methods,
            ));
        }
    }
}

fn check_logical_op(
    cop: &SafeNavigationConsistency,
    source: &SourceFile,
    left: &ruby_prism::Node<'_>,
    right: &ruby_prism::Node<'_>,
    is_and: bool,
    allowed_methods: &[String],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Get receiver name and safe-navigation status for both sides
    let left_info = extract_call_info(left);
    let right_info = extract_call_info(right);

    let (left_recv, left_is_safe, left_method) = match &left_info {
        Some(info) => (
            info.receiver_name.as_str(),
            info.is_safe_nav,
            info.method_name.as_str(),
        ),
        None => return diagnostics,
    };

    let (right_recv, right_is_safe, right_method) = match &right_info {
        Some(info) => (
            info.receiver_name.as_str(),
            info.is_safe_nav,
            info.method_name.as_str(),
        ),
        None => return diagnostics,
    };

    // Only compare if same receiver
    if left_recv != right_recv || left_recv.is_empty() {
        return diagnostics;
    }

    // Skip if nil? is involved
    if left_method == "nil?" || right_method == "nil?" {
        return diagnostics;
    }

    // Skip comparison operators on RHS
    if is_comparison_method(right_method) {
        return diagnostics;
    }

    // Skip allowed methods
    if allowed_methods.iter().any(|m| m == right_method) {
        return diagnostics;
    }

    if is_and {
        // For &&: if left uses safe nav, right doesn't need it (already guarded)
        // If left uses regular nav, right shouldn't use safe nav
        if !left_is_safe && right_is_safe {
            // `foo.bar && foo&.baz` -> right has unnecessary safe nav
            if let Some(info) = &right_info {
                let loc = info.call_operator_offset;
                let (line, column) = source.offset_to_line_col(loc);
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `.` instead of unnecessary `&.`.".to_string(),
                ));
            }
        } else if left_is_safe && right_is_safe {
            // `foo&.bar && foo&.baz` -> right has unnecessary safe nav
            if let Some(info) = &right_info {
                let loc = info.call_operator_offset;
                let (line, column) = source.offset_to_line_col(loc);
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `.` instead of unnecessary `&.`.".to_string(),
                ));
            }
        }
    } else {
        // For ||: if left uses safe nav, right should also use safe nav
        // If left uses regular nav, right shouldn't use safe nav
        if left_is_safe && !right_is_safe {
            // `foo&.bar || foo.baz` -> right should use safe nav
            if let Some(info) = &right_info {
                if is_comparison_method(right_method) {
                    return diagnostics;
                }
                let loc = info.dot_offset;
                let (line, column) = source.offset_to_line_col(loc);
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `&.` for consistency with safe navigation.".to_string(),
                ));
            }
        } else if !left_is_safe && right_is_safe {
            // `foo.bar || foo&.baz` -> right has unnecessary safe nav
            if let Some(info) = &right_info {
                let loc = info.call_operator_offset;
                let (line, column) = source.offset_to_line_col(loc);
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `.` instead of unnecessary `&.`.".to_string(),
                ));
            }
        }
    }

    diagnostics
}

struct CallInfo {
    receiver_name: String,
    method_name: String,
    is_safe_nav: bool,
    call_operator_offset: usize, // offset of `&.` or `.`
    dot_offset: usize,           // offset of `.` in regular call
}

fn extract_call_info(node: &ruby_prism::Node<'_>) -> Option<CallInfo> {
    let call = node.as_call_node()?;
    let recv = call.receiver()?;

    // Get receiver name (simple local variable or method call)
    let receiver_name = get_simple_receiver_name(&recv)?;

    let method_name = std::str::from_utf8(call.name().as_slice())
        .unwrap_or("")
        .to_string();

    let call_op = call.call_operator_loc();
    let is_safe_nav = call_op
        .as_ref()
        .map(|loc| {
            let bytes = &node.location().as_slice();
            let op_offset = loc.start_offset() - node.location().start_offset();
            op_offset + 1 < bytes.len() && bytes[op_offset] == b'&' && bytes[op_offset + 1] == b'.'
        })
        .unwrap_or(false);

    let call_operator_offset = call_op.as_ref().map(|loc| loc.start_offset()).unwrap_or(0);

    let dot_offset = call_op
        .as_ref()
        .map(|loc| {
            if is_safe_nav {
                loc.start_offset() // &. starts at this offset
            } else {
                loc.start_offset() // . is at this offset
            }
        })
        .unwrap_or(0);

    Some(CallInfo {
        receiver_name,
        method_name,
        is_safe_nav,
        call_operator_offset,
        dot_offset,
    })
}

fn get_simple_receiver_name(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(read) = node.as_local_variable_read_node() {
        return Some(
            std::str::from_utf8(read.name().as_slice())
                .unwrap_or("")
                .to_string(),
        );
    }
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() && call.arguments().is_none() {
            return Some(
                std::str::from_utf8(call.name().as_slice())
                    .unwrap_or("")
                    .to_string(),
            );
        }
    }
    None
}

fn is_comparison_method(method: &str) -> bool {
    matches!(
        method,
        "==" | "!=" | "===" | "<=>" | "<" | ">" | "<=" | ">="
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SafeNavigationConsistency,
        "cops/lint/safe_navigation_consistency"
    );
}
