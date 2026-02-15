use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UnfreezeString;

impl Cop for UnfreezeString {
    fn name(&self) -> &'static str {
        "Performance/UnfreezeString"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        if call.name().as_slice() != b"new" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Only match bare `String` or `::String`, not qualified paths like
        // `ActiveModel::Type::String` (which is a different class).
        let is_bare_string = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice() == b"String"
        } else if let Some(cp) = receiver.as_constant_path_node() {
            // ::String (rooted constant path with no parent)
            cp.parent().is_none()
                && cp.name().map(|n| n.as_slice()) == Some(b"String")
        } else {
            false
        };

        if !is_bare_string {
            return Vec::new();
        }

        // Allow String.new with no args, or String.new('') (empty string)
        match call.arguments() {
            None => {} // String.new — flag it
            Some(arguments) => {
                let args = arguments.arguments();
                if args.len() != 1 {
                    return Vec::new();
                }
                // Must be a string node with empty content
                let first_arg = match args.iter().next() {
                    Some(a) => a,
                    None => return Vec::new(),
                };
                match first_arg.as_string_node() {
                    Some(s) => {
                        if !s.unescaped().is_empty() {
                            return Vec::new();
                        }
                    }
                    None => return Vec::new(),
                }
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use unary plus to get an unfrozen string literal.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(UnfreezeString, "cops/performance/unfreeze_string");
}
