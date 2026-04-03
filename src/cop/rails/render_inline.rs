use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-07)
///
/// FP=42, FN=0. All FPs from `renderer.render(inline: ...)` or `@view.render(inline: ...)`
/// where the render call has an explicit receiver. RuboCop's pattern uses `(send nil? :render ...)`
/// which only matches bare render calls (no receiver). Fixed by adding receiver check.
pub struct RenderInline;

impl Cop for RenderInline {
    fn name(&self) -> &'static str {
        "Rails/RenderInline"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        if call.name().as_slice() != b"render" {
            return;
        }
        // RuboCop only flags bare render calls (no receiver)
        if call.receiver().is_some() {
            return;
        }
        if keyword_arg_value(&call, b"inline").is_none() {
            return;
        }
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid `render inline:`. Use templates instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RenderInline, "cops/rails/render_inline");
}
