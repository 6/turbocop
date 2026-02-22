use crate::cop::node_type::CLASS_NODE;
use crate::cop::util::{full_constant_path, parent_class_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ApplicationJob;

impl Cop for ApplicationJob {
    fn name(&self) -> &'static str {
        "Rails/ApplicationJob"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE]
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let class_name = full_constant_path(source, &class.constant_path());
        if class_name == b"ApplicationJob" {
            return;
        }

        let parent = match parent_class_name(source, &class) {
            Some(p) => p,
            None => return,
        };

        if parent == b"ActiveJob::Base" {
            let loc = class.class_keyword_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `ApplicationJob` instead of `ActiveJob::Base`.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ApplicationJob, "cops/rails/application_job");
}
