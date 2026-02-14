use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct WhereNot;

impl Cop for WhereNot {
    fn name(&self) -> &'static str {
        "Rails/WhereNot"
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

        if call.name().as_slice() != b"where" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        for arg in args.arguments().iter() {
            if let Some(str_node) = arg.as_string_node() {
                let content = str_node.unescaped();
                if content.windows(2).any(|w| w == b"!=" || w == b"<>")
                    || content.windows(3).any(|w| w == b"NOT" || w == b"not")
                {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `where.not(...)` instead of manually constructing negated SQL."
                            .to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereNot, "cops/rails/where_not");
}
