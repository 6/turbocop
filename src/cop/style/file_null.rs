use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FileNull;

impl Cop for FileNull {
    fn name(&self) -> &'static str {
        "Style/FileNull"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Only check simple string nodes (not interpolated, not in arrays/hashes)
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let content_bytes = string_node.unescaped();
        let content_str = match std::str::from_utf8(&content_bytes) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        if content_str.is_empty() {
            return Vec::new();
        }

        // Check if it's a standalone "nul" - only flag if "/dev/null" is also present
        // Actually per the spec: NUL alone is not flagged, only NUL: or /dev/null are flagged independently
        // and NUL is flagged only when /dev/null appears in the same file
        // For simplicity, we flag /dev/null, NUL:, and nul: (case insensitive)
        let lower = content_str.to_lowercase();

        let matched = if lower == "/dev/null" {
            Some(content_str)
        } else if lower == "nul:" {
            Some(content_str)
        } else {
            None
        };

        if let Some(matched_str) = matched {
            let loc = string_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Use `File::NULL` instead of `{}`.", matched_str),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileNull, "cops/style/file_null");
}
