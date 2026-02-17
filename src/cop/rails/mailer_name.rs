use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MailerName;

const MAILER_BASES: &[&[u8]] = &[
    b"ActionMailer::Base",
    b"ApplicationMailer",
];

impl Cop for MailerName {
    fn name(&self) -> &'static str {
        "Rails/MailerName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/mailers/**/*.rb"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a superclass
        let superclass = match class_node.superclass() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Check superclass is a mailer base
        let superclass_name = util::full_constant_path(source, &superclass);
        if !MAILER_BASES.iter().any(|base| *base == superclass_name) {
            return Vec::new();
        }

        // Get class name and check if it ends with "Mailer"
        let class_name_node = class_node.constant_path();
        let class_name = util::full_constant_path(source, &class_name_node);
        let class_name_str = std::str::from_utf8(class_name).unwrap_or("");

        // Get the last segment of the class name
        let last_segment = class_name_str.rsplit("::").next().unwrap_or(class_name_str);
        if last_segment.ends_with("Mailer") {
            return Vec::new();
        }

        let loc = class_name_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Mailer should end with `Mailer` suffix.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MailerName, "cops/rails/mailer_name");
}
