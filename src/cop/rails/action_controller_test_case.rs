use crate::cop::util::parent_class_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ActionControllerTestCase;

impl Cop for ActionControllerTestCase {
    fn name(&self) -> &'static str {
        "Rails/ActionControllerTestCase"
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let parent = match parent_class_name(source, &class) {
            Some(p) => p,
            None => return Vec::new(),
        };

        if parent == b"ActionController::TestCase" {
            let loc = class.class_keyword_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `ActionDispatch::IntegrationTest` instead of `ActionController::TestCase`."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ActionControllerTestCase,
        "cops/rails/action_controller_test_case"
    );
}
