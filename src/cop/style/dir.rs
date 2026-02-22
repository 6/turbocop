use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, SOURCE_FILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Dir;

/// Check if a node is File or ::File constant
fn is_file_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == "File";
    }
    if let Some(path) = node.as_constant_path_node() {
        let name = std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("");
        if name == "File" && path.parent().is_none() {
            return true;
        }
    }
    false
}

/// Check if a node is __FILE__
fn is_file_keyword(node: &ruby_prism::Node<'_>) -> bool {
    node.as_source_file_node().is_some()
}

impl Cop for Dir {
    fn name(&self) -> &'static str {
        "Style/Dir"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            SOURCE_FILE_NODE,
        ]
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_file_const(&receiver) {
            return;
        }

        // Pattern 1: File.expand_path(File.dirname(__FILE__))
        if method_name == "expand_path" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    if let Some(inner_call) = arg_list[0].as_call_node() {
                        let inner_method =
                            std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                        if inner_method == "dirname" {
                            if let Some(inner_recv) = inner_call.receiver() {
                                if is_file_const(&inner_recv) {
                                    if let Some(inner_args) = inner_call.arguments() {
                                        let inner_arg_list: Vec<_> =
                                            inner_args.arguments().iter().collect();
                                        if inner_arg_list.len() == 1
                                            && is_file_keyword(&inner_arg_list[0])
                                        {
                                            let loc = node.location();
                                            let (line, column) =
                                                source.offset_to_line_col(loc.start_offset());
                                            diagnostics.push(self.diagnostic(
                                                source,
                                                line,
                                                column,
                                                "Use `__dir__` to get an absolute path to the current file's directory.".to_string(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Pattern 2: File.dirname(File.realpath(__FILE__))
        if method_name == "dirname" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 1 {
                    if let Some(inner_call) = arg_list[0].as_call_node() {
                        let inner_method =
                            std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                        if inner_method == "realpath" {
                            if let Some(inner_recv) = inner_call.receiver() {
                                if is_file_const(&inner_recv) {
                                    if let Some(inner_args) = inner_call.arguments() {
                                        let inner_arg_list: Vec<_> =
                                            inner_args.arguments().iter().collect();
                                        if inner_arg_list.len() == 1
                                            && is_file_keyword(&inner_arg_list[0])
                                        {
                                            let loc = node.location();
                                            let (line, column) =
                                                source.offset_to_line_col(loc.start_offset());
                                            diagnostics.push(self.diagnostic(
                                                source,
                                                line,
                                                column,
                                                "Use `__dir__` to get an absolute path to the current file's directory.".to_string(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Dir, "cops/style/dir");
}
