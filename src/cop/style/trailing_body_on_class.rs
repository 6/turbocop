use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_NODE, SINGLETON_CLASS_NODE};

pub struct TrailingBodyOnClass;

impl Cop for TrailingBodyOnClass {
    fn name(&self) -> &'static str {
        "Style/TrailingBodyOnClass"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SINGLETON_CLASS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check class ... ; body
        if let Some(class_node) = node.as_class_node() {
            let body = match class_node.body() {
                Some(b) => b,
                None => return Vec::new(),
            };

            // Check if class keyword line equals body start line
            let class_loc = class_node.constant_path().location();
            let (class_line, _) = source.offset_to_line_col(class_loc.start_offset());
            let body_loc = body.location();
            let (body_line, body_column) = source.offset_to_line_col(body_loc.start_offset());

            if class_line == body_line {
                return vec![self.diagnostic(
                    source,
                    body_line,
                    body_column,
                    "Place the first line of class body on its own line.".to_string(),
                )];
            }
        }

        // Check sclass (singleton class) `class << self; body`
        if let Some(sclass_node) = node.as_singleton_class_node() {
            let body = match sclass_node.body() {
                Some(b) => b,
                None => return Vec::new(),
            };

            let kw_loc = sclass_node.class_keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let body_loc = body.location();
            let (body_line, body_column) = source.offset_to_line_col(body_loc.start_offset());

            if kw_line == body_line {
                return vec![self.diagnostic(
                    source,
                    body_line,
                    body_column,
                    "Place the first line of class body on its own line.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingBodyOnClass, "cops/style/trailing_body_on_class");
}
