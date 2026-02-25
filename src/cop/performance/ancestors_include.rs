use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AncestorsInclude;

impl Cop for AncestorsInclude {
    fn name(&self) -> &'static str {
        "Performance/AncestorsInclude"
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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"ancestors" || chain.outer_method != b"include?" {
            return;
        }

        // ancestors should have no arguments
        if chain.inner_call.arguments().is_some() {
            return;
        }

        // Only flag when the receiver of `.ancestors` is a constant or absent (implicit self).
        // Non-constant receivers (e.g. `self.class.ancestors`, `obj.ancestors`) are not flagged,
        // matching RuboCop's `subclass.const_type?` guard.
        if let Some(receiver) = chain.inner_call.receiver() {
            if receiver.as_constant_read_node().is_none()
                && receiver.as_constant_path_node().is_none()
            {
                return;
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `is_a?` instead of `ancestors.include?`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AncestorsInclude, "cops/performance/ancestors_include");
}
