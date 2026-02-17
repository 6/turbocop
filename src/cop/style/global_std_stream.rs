use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct GlobalStdStream;

impl Cop for GlobalStdStream {
    fn name(&self) -> &'static str {
        "Style/GlobalStdStream"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check for bare STDOUT, STDERR, STDIN constants
        if let Some(const_read) = node.as_constant_read_node() {
            let name = const_read.name();
            let name_bytes = name.as_slice();
            if let Some(gvar) = std_stream_gvar(name_bytes) {
                // Check parent is NOT a gvar assignment like: $stdout = STDOUT
                // We can't easily check this, so we skip that edge case
                let loc = const_read.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let const_name = std::str::from_utf8(name_bytes).unwrap_or("");
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of `{}`.", gvar, const_name),
                )];
            }
        }

        // Check for ::STDOUT, ::STDERR, ::STDIN (ConstantPathNode with no parent = cbase)
        if let Some(const_path) = node.as_constant_path_node() {
            // Must be top-level (::STDOUT) — parent is None
            if const_path.parent().is_some() {
                return Vec::new();
            }
            if let Some(name_loc) = const_path.name() {
                let name_bytes = name_loc.as_slice();
                if let Some(gvar) = std_stream_gvar(name_bytes) {
                    let loc = const_path.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let const_name = std::str::from_utf8(name_bytes).unwrap_or("");
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `{}` instead of `{}`.", gvar, const_name),
                    )];
                }
            }
        }

        Vec::new()
    }
}

fn std_stream_gvar(name: &[u8]) -> Option<&'static str> {
    match name {
        b"STDOUT" => Some("$stdout"),
        b"STDERR" => Some("$stderr"),
        b"STDIN" => Some("$stdin"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GlobalStdStream, "cops/style/global_std_stream");
}
