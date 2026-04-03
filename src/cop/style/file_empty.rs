use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTEGER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/FileEmpty detects patterns that check whether a file is empty
/// and suggests using `File.empty?` instead.
///
/// Detected patterns:
/// - `File.zero?('path')` / `FileTest.zero?('path')`
/// - `File.size('path') == 0`
/// - `File.size('path').zero?`
/// - `File.read('path') == ""` / `File.binread('path') == ""`
/// - `File.read('path').empty?` / `File.binread('path').empty?`
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

    fn report_offense(
        &self,
        source: &SourceFile,
        call_loc: &ruby_prism::Location<'_>,
        file_recv: &ruby_prism::Node<'_>,
        arg: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let (line, column) = source.offset_to_line_col(call_loc.start_offset());
        let file_str = String::from_utf8_lossy(
            &source.as_bytes()
                [file_recv.location().start_offset()..file_recv.location().end_offset()],
        );
        let arg_str = String::from_utf8_lossy(
            &source.as_bytes()[arg.location().start_offset()..arg.location().end_offset()],
        );
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}.empty?({})` instead.", file_str, arg_str),
        ));
    }
}

impl Cop for FileEmpty {
    fn name(&self) -> &'static str {
        "Style/FileEmpty"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTEGER_NODE,
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

        let method_bytes = call.name().as_slice();

        // Pattern 1: File.zero?('path') / FileTest.zero?('path')
        // Pattern 2: File.size('path').zero? / FileTest.size('path').zero?
        if method_bytes == b"zero?" {
            if let Some(recv) = call.receiver() {
                // Pattern 1: File.zero?(arg)
                if Self::is_file_or_filetest(&recv) {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            self.report_offense(
                                source,
                                &call.location(),
                                &recv,
                                &arg_list[0],
                                diagnostics,
                            );
                        }
                    }
                }
                // Pattern 2: File.size(arg).zero?
                else if let Some(size_call) = recv.as_call_node() {
                    if size_call.name().as_slice() == b"size" {
                        if let Some(file_recv) = size_call.receiver() {
                            if Self::is_file_or_filetest(&file_recv) {
                                if let Some(size_args) = size_call.arguments() {
                                    let sa: Vec<_> = size_args.arguments().iter().collect();
                                    if sa.len() == 1 {
                                        self.report_offense(
                                            source,
                                            &call.location(),
                                            &file_recv,
                                            &sa[0],
                                            diagnostics,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Pattern 3: File.size('path') == 0
        // Pattern 4: File.read('path') == "" / File.binread('path') == ""
        if method_bytes == b"==" {
            if let Some(recv) = call.receiver() {
                if let Some(inner_call) = recv.as_call_node() {
                    let inner_method = inner_call.name().as_slice();
                    if let Some(file_recv) = inner_call.receiver() {
                        if Self::is_file_or_filetest(&file_recv) {
                            if let Some(inner_args) = inner_call.arguments() {
                                let ia: Vec<_> = inner_args.arguments().iter().collect();
                                if ia.len() == 1 {
                                    if let Some(args) = call.arguments() {
                                        let arg_list: Vec<_> = args.arguments().iter().collect();
                                        if arg_list.len() == 1 {
                                            // File.size(arg) == 0
                                            if inner_method == b"size" {
                                                if let Some(int_node) =
                                                    arg_list[0].as_integer_node()
                                                {
                                                    if int_node.location().as_slice() == b"0" {
                                                        self.report_offense(
                                                            source,
                                                            &call.location(),
                                                            &file_recv,
                                                            &ia[0],
                                                            diagnostics,
                                                        );
                                                    }
                                                }
                                            }
                                            // File.read(arg) == "" / File.binread(arg) == ""
                                            if inner_method == b"read" || inner_method == b"binread"
                                            {
                                                if let Some(str_node) = arg_list[0].as_string_node()
                                                {
                                                    if str_node.unescaped() == b"" {
                                                        self.report_offense(
                                                            source,
                                                            &call.location(),
                                                            &file_recv,
                                                            &ia[0],
                                                            diagnostics,
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
        }

        // Pattern 5: File.read('path').empty? / File.binread('path').empty?
        if method_bytes == b"empty?" {
            if let Some(recv) = call.receiver() {
                if let Some(read_call) = recv.as_call_node() {
                    let read_method = read_call.name().as_slice();
                    if read_method == b"read" || read_method == b"binread" {
                        if let Some(file_recv) = read_call.receiver() {
                            if Self::is_file_or_filetest(&file_recv) {
                                if let Some(read_args) = read_call.arguments() {
                                    let ra: Vec<_> = read_args.arguments().iter().collect();
                                    if ra.len() == 1 {
                                        self.report_offense(
                                            source,
                                            &call.location(),
                                            &file_recv,
                                            &ra[0],
                                            diagnostics,
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileEmpty, "cops/style/file_empty");
}
