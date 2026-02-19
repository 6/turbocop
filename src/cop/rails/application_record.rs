use crate::cop::util::{full_constant_path, parent_class_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CLASS_NODE;

pub struct ApplicationRecord;

impl Cop for ApplicationRecord {
    fn name(&self) -> &'static str {
        "Rails/ApplicationRecord"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["db/**/*.rb"]
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
    ) {
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let class_name = full_constant_path(source, &class.constant_path());
        if class_name == b"ApplicationRecord" {
            return;
        }

        let parent = match parent_class_name(source, &class) {
            Some(p) => p,
            None => return,
        };

        // Handle both ActiveRecord::Base and ::ActiveRecord::Base
        let parent_trimmed = if parent.starts_with(b"::") {
            &parent[2..]
        } else {
            parent
        };
        if parent_trimmed == b"ActiveRecord::Base" {
            let loc = class.class_keyword_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Models should subclass `ApplicationRecord`.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ApplicationRecord, "cops/rails/application_record");
}
