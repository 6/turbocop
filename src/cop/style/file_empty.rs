use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTEGER_NODE};

pub struct FileEmpty;

impl FileEmpty {
    fn is_file_or_filetest(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(c) = node.as_constant_read_node() {
            let name = c.name().as_slice();
            return name == b"File" || name == b"FileTest";
        }
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    return name.as_slice() == b"File" || name.as_slice() == b"FileTest";
                }
            }
        }
        false
    }
}

impl Cop for FileEmpty {
    fn name(&self) -> &'static str {
        "Style/FileEmpty"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTEGER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_bytes = call.name().as_slice();

        // Pattern 1: File.zero?('path')
        if method_bytes == b"zero?" {
            if let Some(recv) = call.receiver() {
                if Self::is_file_or_filetest(&recv) {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let recv_src = &source.as_bytes()[recv.location().start_offset()..recv.location().end_offset()];
                    let recv_str = String::from_utf8_lossy(recv_src);
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            let arg_src = &source.as_bytes()[arg_list[0].location().start_offset()..arg_list[0].location().end_offset()];
                            let arg_str = String::from_utf8_lossy(arg_src);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Use `{}.empty?({})` instead.", recv_str, arg_str),
                            ));
                        }
                    }
                }
            }
        }

        // Pattern 2: File.size('path') == 0 or File.size('path').zero?
        // This is detected at the outer call level
        if method_bytes == b"==" {
            if let Some(recv) = call.receiver() {
                if let Some(size_call) = recv.as_call_node() {
                    if size_call.name().as_slice() == b"size" {
                        if let Some(file_recv) = size_call.receiver() {
                            if Self::is_file_or_filetest(&file_recv) {
                                // Check that the argument is 0
                                if let Some(args) = call.arguments() {
                                    let arg_list: Vec<_> = args.arguments().iter().collect();
                                    if arg_list.len() == 1 {
                                        if let Some(int_node) = arg_list[0].as_integer_node() {
                                            if int_node.location().as_slice() == b"0" {
                                                let loc = call.location();
                                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                                let file_src = &source.as_bytes()[file_recv.location().start_offset()..file_recv.location().end_offset()];
                                                let file_str = String::from_utf8_lossy(file_src);
                                                if let Some(size_args) = size_call.arguments() {
                                                    let sa: Vec<_> = size_args.arguments().iter().collect();
                                                    if sa.len() == 1 {
                                                        let arg_src = &source.as_bytes()[sa[0].location().start_offset()..sa[0].location().end_offset()];
                                                        let arg_str = String::from_utf8_lossy(arg_src);
                                                        diagnostics.push(self.diagnostic(
                                                            source,
                                                            line,
                                                            column,
                                                            format!("Use `{}.empty?({})` instead.", file_str, arg_str),
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
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileEmpty, "cops/style/file_empty");
}
