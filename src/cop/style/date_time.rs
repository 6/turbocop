use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/DateTime: Prefer `Time` over `DateTime`.
///
/// ## Investigation findings (2026-03-23)
///
/// Root cause of FN (1018): The original `args.len() >= 2` historic-date check
/// was too broad — it skipped ANY call with 2+ args (e.g., `DateTime.new(2024, 1, 1)`).
/// The vendor RuboCop only skips calls where the last argument is a `Date::XXX`
/// constant (like `Date::ENGLAND`), matching the `historic_date?` pattern:
/// `(send _ _ _ (const (const {nil? (cbase)} :Date) _))`.
///
/// Root cause of FP (53): All 53 FPs were bare `to_datetime(args)` calls
/// without a receiver (e.g., `to_datetime(row["created_at"])` where
/// `to_datetime` is a locally-defined helper method). RuboCop only flags
/// `receiver.to_datetime`, not bare function calls. Fixed by checking
/// `call.receiver().is_some()` before flagging.
///
/// Fix applied: Replaced the `args.len() >= 2` check with proper `is_historic_date`
/// detection that only skips when the last arg is `Date::XXX` or `::Date::XXX`.
pub struct DateTime;

impl Cop for DateTime {
    fn name(&self) -> &'static str {
        "Style/DateTime"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        let allow_coercion = config.get_bool("AllowCoercion", false);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        // Check for receiver.to_datetime calls (not bare to_datetime calls)
        if method_name == "to_datetime" {
            // Only flag when there's a receiver (e.g., string.to_datetime).
            // Bare to_datetime(args) is a local/inherited method call, not a coercion.
            if call.receiver().is_none() {
                return;
            }
            if allow_coercion {
                return;
            }
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `#to_datetime`.".to_string(),
            ));
            return;
        }

        // Check for DateTime.something calls
        if let Some(receiver) = call.receiver() {
            let is_datetime = is_datetime_const(&receiver);
            if !is_datetime {
                return;
            }

            // Skip historic dates: last arg is Date::XXX or ::Date::XXX
            // Matches vendor pattern: (send _ _ _ (const (const {nil? (cbase)} :Date) _))
            if is_historic_date(&call) {
                return;
            }

            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer `Time` over `DateTime`.".to_string(),
            ));
        }
    }
}

fn is_datetime_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == "DateTime";
    }
    if let Some(path) = node.as_constant_path_node() {
        // Check ::DateTime
        let name = std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("");
        if name == "DateTime" {
            // Make sure it's ::DateTime (parent is None/root) not Something::DateTime
            if path.parent().is_none() {
                return true;
            }
        }
    }
    false
}

/// Check if a call has a historic date argument: last arg is Date::XXX or ::Date::XXX.
/// Matches vendor pattern: (send _ _ _ (const (const {nil? (cbase)} :Date) _))
fn is_historic_date(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() < 2 {
        return false;
    }

    // Check if the last argument is a constant path like Date::ENGLAND or ::Date::ITALY
    let last_arg = &arg_list[arg_list.len() - 1];
    if let Some(const_path) = last_arg.as_constant_path_node() {
        // The parent should be a constant named "Date" (with nil or cbase parent)
        if let Some(parent) = const_path.parent() {
            if let Some(parent_read) = parent.as_constant_read_node() {
                let name = std::str::from_utf8(parent_read.name().as_slice()).unwrap_or("");
                return name == "Date";
            }
            if let Some(parent_path) = parent.as_constant_path_node() {
                // ::Date::ITALY — parent_path is ::Date (ConstantPathNode with no parent)
                let name = std::str::from_utf8(parent_path.name_loc().as_slice()).unwrap_or("");
                if name == "Date" && parent_path.parent().is_none() {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DateTime, "cops/style/date_time");
}
