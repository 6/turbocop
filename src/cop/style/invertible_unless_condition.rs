use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InvertibleUnlessCondition;

impl Cop for InvertibleUnlessCondition {
    fn name(&self) -> &'static str {
        "Style/InvertibleUnlessCondition"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let unless_node = match node.as_unless_node() {
            Some(u) => u,
            None => return Vec::new(),
        };

        let _inverse_methods = config.get_string_hash("InverseMethods");

        let predicate = unless_node.predicate();

        // Check if the predicate uses a method with an inverse
        if let Some(call) = predicate.as_call_node() {
            let method_bytes = call.name().as_slice();

            let inverse = match method_bytes {
                b"==" => Some("!="),
                b"!=" => Some("=="),
                b">" => Some("<="),
                b"<" => Some(">="),
                b">=" => Some("<"),
                b"<=" => Some(">"),
                b"any?" => Some("none?"),
                b"none?" => Some("any?"),
                b"include?" => Some("exclude?"),
                b"exclude?" => Some("include?"),
                b"even?" => Some("odd?"),
                b"odd?" => Some("even?"),
                b"present?" => Some("blank?"),
                b"blank?" => Some("present?"),
                b"empty?" => Some("any?"),
                _ => None,
            };

            if let Some(inv) = inverse {
                let method_str = std::str::from_utf8(method_bytes).unwrap_or("method");
                let loc = unless_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `if` with `{}` instead of `unless` with `{}`.", inv, method_str),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InvertibleUnlessCondition, "cops/style/invertible_unless_condition");
}
