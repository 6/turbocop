use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::path::Path;

pub struct RequireRelativeSelfPath;

impl Cop for RequireRelativeSelfPath {
    fn name(&self) -> &'static str {
        "Lint/RequireRelativeSelfPath"
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
        // Look for `require_relative 'self_filename'`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"require_relative" {
            return Vec::new();
        }

        // Must have no receiver
        if call.receiver().is_some() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return Vec::new();
        }

        let first_arg = args.iter().next().unwrap();
        let string_node = match first_arg.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let required_path = string_node.unescaped();
        let required_str = match std::str::from_utf8(&required_path) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Get the current file's basename without extension
        let file_path = Path::new(source.path_str());
        let file_stem = match file_path.file_stem() {
            Some(s) => s.to_str().unwrap_or(""),
            None => return Vec::new(),
        };

        // The required path's filename (last component)
        let required_path_obj = Path::new(required_str);
        let required_stem = match required_path_obj.file_stem() {
            Some(s) => s.to_str().unwrap_or(""),
            None => return Vec::new(),
        };

        // Check if the extension (if any) is `.rb` or absent
        let required_ext = required_path_obj
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if !required_ext.is_empty() && required_ext != "rb" {
            return Vec::new();
        }

        // Check if it's requiring itself (same directory, same name)
        // Only flag if the required path has no directory component or its directory
        // resolves to the same file
        let required_parent = required_path_obj.parent();
        let is_same_dir = match required_parent {
            None => true,
            Some(p) => p.as_os_str().is_empty() || p.as_os_str() == ".",
        };

        if is_same_dir && required_stem == file_stem {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Remove the `require_relative` that requires itself.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireRelativeSelfPath, "cops/lint/require_relative_self_path");
}
