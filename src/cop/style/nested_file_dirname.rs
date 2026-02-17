use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NestedFileDirname;

impl Cop for NestedFileDirname {
    fn name(&self) -> &'static str {
        "Style/NestedFileDirname"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be `dirname` method
        if call.name().as_slice() != b"dirname" {
            return Vec::new();
        }

        // Receiver must be File constant
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if !is_file_const(&receiver) {
            return Vec::new();
        }

        // First argument must be another File.dirname call
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];

        // Check if first arg is itself File.dirname (nesting starts)
        if !is_file_dirname_call(first_arg) {
            return Vec::new();
        }

        let level = count_dirname_nesting(first_arg, 1) + 1;

        // Get the innermost path arg source
        let inner_path_src = get_innermost_path_source(first_arg, source);

        let msg_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `dirname({}, {})` instead.", inner_path_src, level),
        )]
    }
}

fn is_file_dirname_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"dirname" {
            if let Some(recv) = call.receiver() {
                return is_file_const(&recv);
            }
        }
    }
    false
}

fn count_dirname_nesting(node: &ruby_prism::Node<'_>, level: usize) -> usize {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"dirname" {
            if let Some(recv) = call.receiver() {
                if is_file_const(&recv) {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() && is_file_dirname_call(&arg_list[0]) {
                            return count_dirname_nesting(&arg_list[0], level + 1);
                        }
                    }
                }
            }
        }
    }
    level
}

fn get_innermost_path_source(node: &ruby_prism::Node<'_>, source: &SourceFile) -> String {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"dirname" {
            if let Some(recv) = call.receiver() {
                if is_file_const(&recv) {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if !arg_list.is_empty() {
                            return get_innermost_path_source(&arg_list[0], source);
                        }
                    }
                }
            }
        }
    }
    let loc = node.location();
    std::str::from_utf8(&source.content[loc.start_offset()..loc.end_offset()])
        .unwrap_or("path")
        .to_string()
}

fn is_file_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        return c.name().as_slice() == b"File";
    }
    if let Some(cp) = node.as_constant_path_node() {
        return cp.parent().is_none()
            && cp.name().is_some_and(|n| n.as_slice() == b"File");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedFileDirname, "cops/style/nested_file_dirname");
}
