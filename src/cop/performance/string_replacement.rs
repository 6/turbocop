use crate::cop::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/StringReplacement
///
/// Identifies places where `gsub`/`gsub!` can be replaced by `tr`/`delete`.
///
/// Investigation notes:
/// - Original implementation only handled `gsub` (not `gsub!`) and only single-byte chars.
/// - Root cause of 1054 FNs: byte length check (`len() != 1`) rejected multi-byte UTF-8
///   single characters (e.g., "Á" is 2 bytes but 1 char). Also missed empty replacement
///   pattern (→ `delete`) and `gsub!` (→ `tr!`/`delete!`).
/// - RuboCop only flags `gsub`/`gsub!`, NOT `sub`/`sub!`.
/// - Message format: "Use `tr` instead of `gsub`." / "Use `delete` instead of `gsub`."
///   with bang variants for `gsub!`.
pub struct StringReplacement;

impl Cop for StringReplacement {
    fn name(&self) -> &'static str {
        "Performance/StringReplacement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        let method_name = call.name().as_slice();
        let is_bang = match method_name {
            b"gsub" => false,
            b"gsub!" => true,
            _ => return,
        };

        // Must have a receiver (str.gsub)
        if call.receiver().is_none() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 2 {
            return;
        }

        let mut args_iter = args.iter();
        let first_node = match args_iter.next() {
            Some(a) => a,
            None => return,
        };
        let second_node = match args_iter.next() {
            Some(a) => a,
            None => return,
        };

        let first = match first_node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let second = match second_node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let first_str = first.unescaped();
        let second_str = second.unescaped();

        // First arg must be a single character (by char count, not byte count)
        let first_text = String::from_utf8_lossy(first_str);
        if first_text.chars().count() != 1 {
            return;
        }

        // Empty first arg is not flagged
        if first_text.is_empty() {
            return;
        }

        // Second arg must be empty or a single character
        let second_text = String::from_utf8_lossy(second_str);
        let second_char_count = second_text.chars().count();
        if second_char_count > 1 {
            return;
        }

        let (prefer, current) = if second_char_count == 0 {
            // Empty replacement → delete
            if is_bang {
                ("delete!", "gsub!")
            } else {
                ("delete", "gsub")
            }
        } else {
            // Single char replacement → tr
            if is_bang {
                ("tr!", "gsub!")
            } else {
                ("tr", "gsub")
            }
        };

        // RuboCop points at the method name through end of args (node.loc.selector → end)
        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{prefer}` instead of `{current}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringReplacement, "cops/performance/string_replacement");
}
