use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantDirGlobSort;

impl Cop for RedundantDirGlobSort {
    fn name(&self) -> &'static str {
        "Lint/RedundantDirGlobSort"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        // Must be a .sort call
        if call.name().as_slice() != b"sort" {
            return Vec::new();
        }

        // Must have no arguments (bare .sort)
        if call.arguments().is_some() {
            return Vec::new();
        }

        // Receiver must be a Dir.glob or Dir[] call
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let recv_method = recv_call.name().as_slice();
        if recv_method != b"glob" && recv_method != b"[]" {
            return Vec::new();
        }

        // The receiver of glob/[] must be Dir
        let dir_recv = match recv_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_dir = if let Some(const_read) = dir_recv.as_constant_read_node() {
            const_read.name().as_slice() == b"Dir"
        } else if let Some(const_path) = dir_recv.as_constant_path_node() {
            const_path.name().is_some_and(|n| n.as_slice() == b"Dir")
        } else {
            false
        };

        if !is_dir {
            return Vec::new();
        }

        // Check for multiple arguments (not redundant if glob has multiple args)
        if let Some(args) = recv_call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() >= 2 {
                return Vec::new();
            }
            // Check for splat argument
            if !arg_list.is_empty() && arg_list[0].as_splat_node().is_some() {
                return Vec::new();
            }
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Remove redundant `sort`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantDirGlobSort, "cops/lint/redundant_dir_glob_sort");
}
