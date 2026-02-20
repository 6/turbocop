use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, SOURCE_FILE_NODE, STRING_NODE};

pub struct ExpandPathArguments;

impl Cop for ExpandPathArguments {
    fn name(&self) -> &'static str {
        "Style/ExpandPathArguments"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, SOURCE_FILE_NODE, STRING_NODE]
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
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call_node.name();
        let method_bytes = method_name.as_slice();

        if method_bytes != b"expand_path" {
            return;
        }

        // Receiver must be `File` or `::File`
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_file_receiver = if let Some(const_read) = receiver.as_constant_read_node() {
            const_read.name().as_slice() == b"File"
        } else if let Some(const_path) = receiver.as_constant_path_node() {
            // ::File
            const_path.parent().is_none()
                && const_path
                    .name()
                    .is_some_and(|n| n.as_slice() == b"File")
        } else {
            false
        };

        if !is_file_receiver {
            return;
        }

        // Must have arguments
        let args = match call_node.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Pattern: File.expand_path('...', __FILE__) - needs exactly 2 args
        if arg_list.len() != 2 {
            return;
        }

        // Second argument must be __FILE__
        if arg_list[1].as_source_file_node().is_none() {
            return;
        }

        // First argument must be a string literal
        let first_arg = &arg_list[0];
        let path_str = match extract_string_value(first_arg) {
            Some(s) => s,
            None => return,
        };

        // Build the suggestion
        let suggestion = build_suggestion(&path_str);

        let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        let orig = format!("expand_path('{}', __FILE__)", path_str);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{}` instead of `{}`.",
                suggestion, orig,
            ),
        ));
    }
}

/// Extract a simple string value from a string node.
fn extract_string_value(node: &ruby_prism::Node<'_>) -> Option<String> {
    let string_node = node.as_string_node()?;
    let content = string_node.content_loc().as_slice();
    String::from_utf8(content.to_vec()).ok()
}

/// Build the suggested replacement for File.expand_path(path, __FILE__).
fn build_suggestion(path: &str) -> String {
    // Normalize the path: resolve `.` and count `..` components
    let normalized = normalize_path(path);

    if normalized == "." {
        // File.expand_path('.', __FILE__) -> expand_path(__FILE__)
        return "expand_path(__FILE__)".to_string();
    }

    if normalized == ".." {
        // File.expand_path('..', __FILE__) -> expand_path(__dir__)
        return "expand_path(__dir__)".to_string();
    }

    // Count leading `..` components
    let parts: Vec<&str> = normalized.split('/').collect();
    let mut parent_count = 0;
    for part in &parts {
        if *part == ".." {
            parent_count += 1;
        } else {
            break;
        }
    }

    if parent_count == 0 {
        // No parent traversal, just a relative path
        return format!("expand_path('{}', __FILE__)", normalized);
    }

    // Build the new path: strip one level of `..` (since __dir__ replaces __FILE__'s directory)
    let remaining_parts = &parts[1..]; // skip first `..` (replaced by __dir__)
    if remaining_parts.is_empty() {
        return "expand_path(__dir__)".to_string();
    }

    let new_path = remaining_parts.join("/");
    if new_path.is_empty() || new_path == "." {
        return "expand_path(__dir__)".to_string();
    }

    format!("expand_path('{}', __dir__)", new_path)
}

/// Normalize a relative path: resolve `.` components and simplify.
fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut result: Vec<&str> = Vec::new();

    for part in parts {
        if part == "." {
            // Skip current directory references
            continue;
        }
        result.push(part);
    }

    if result.is_empty() {
        ".".to_string()
    } else {
        result.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpandPathArguments, "cops/style/expand_path_arguments");
}
