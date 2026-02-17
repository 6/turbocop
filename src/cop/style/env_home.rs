use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EnvHome;

impl Cop for EnvHome {
    fn name(&self) -> &'static str {
        "Style/EnvHome"
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

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be [] or fetch
        if method_bytes != b"[]" && method_bytes != b"fetch" {
            return Vec::new();
        }

        // Receiver must be ENV constant
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_env = receiver
            .as_constant_read_node()
            .is_some_and(|c| c.name().as_slice() == b"ENV")
            || receiver.as_constant_path_node().is_some_and(|cp| {
                cp.parent().is_none()
                    && cp.name().is_some_and(|n| n.as_slice() == b"ENV")
            });

        if !is_env {
            return Vec::new();
        }

        // First argument must be string "HOME"
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];
        let is_home = first_arg.as_string_node().is_some_and(|s| {
            s.unescaped() == b"HOME"
        });

        if !is_home {
            return Vec::new();
        }

        // For fetch, second arg must be nil or absent
        if method_bytes == b"fetch" && arg_list.len() == 2 {
            if arg_list[1].as_nil_node().is_none() {
                return Vec::new();
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Dir.home` instead.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnvHome, "cops/style/env_home");
}
