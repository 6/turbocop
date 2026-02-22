use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DirEmpty;

fn is_dir_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == "Dir";
    }
    if let Some(path) = node.as_constant_path_node() {
        let name = std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("");
        return name == "Dir" && path.parent().is_none();
    }
    false
}

impl Cop for DirEmpty {
    fn name(&self) -> &'static str {
        "Style/DirEmpty"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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

        // Pattern: Dir.children('path').empty?
        // Pattern: Dir.each_child('path').none?
        if method_name == "empty?" || method_name == "none?" {
            if let Some(receiver) = call.receiver() {
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method =
                        std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if matches!(recv_method, "children" | "each_child") {
                        if let Some(recv_recv) = recv_call.receiver() {
                            if is_dir_const(&recv_recv) {
                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Use `Dir.empty?('path/to/dir')` instead.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Pattern: Dir.entries('path').size == 2
        // Pattern: Dir.children('path').size == 0
        if method_name == "==" || method_name == "!=" || method_name == ">" {
            if let Some(receiver) = call.receiver() {
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method =
                        std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if recv_method == "size" || recv_method == "length" || recv_method == "count" {
                        if let Some(inner_recv) = recv_call.receiver() {
                            if let Some(inner_call) = inner_recv.as_call_node() {
                                let inner_method =
                                    std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                                if matches!(inner_method, "entries" | "children") {
                                    if let Some(dir_recv) = inner_call.receiver() {
                                        if is_dir_const(&dir_recv) {
                                            let loc = node.location();
                                            let (line, column) =
                                                source.offset_to_line_col(loc.start_offset());
                                            diagnostics.push(
                                                self.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    "Use `Dir.empty?('path/to/dir')` instead."
                                                        .to_string(),
                                                ),
                                            );
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
    crate::cop_fixture_tests!(DirEmpty, "cops/style/dir_empty");
}
