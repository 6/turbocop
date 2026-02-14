use crate::cop::util::{full_constant_path, parent_class_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ApplicationMailer;

impl Cop for ApplicationMailer {
    fn name(&self) -> &'static str {
        "Rails/ApplicationMailer"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["app/mailers/**/*.rb"]
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

        let class_name = full_constant_path(source, &class.constant_path());
        if class_name == b"ApplicationMailer" {
            return Vec::new();
        }

        let parent = match parent_class_name(source, &class) {
            Some(p) => p,
            None => return Vec::new(),
        };

        if parent == b"ActionMailer::Base" {
            let loc = class.class_keyword_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `ApplicationMailer` instead of `ActionMailer::Base`.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ApplicationMailer, "cops/rails/application_mailer");
}
