use crate::cop::node_type::{CLASS_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyClass;

impl Cop for EmptyClass {
    fn name(&self) -> &'static str {
        "Lint/EmptyClass"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Handle both ClassNode and SingletonClassNode (metaclass)
        let (body_empty, kw_loc, start_line, end_line) =
            if let Some(class_node) = node.as_class_node() {
                // Per RuboCop: skip classes with a parent class (e.g. class Error < StandardError; end)
                if class_node.superclass().is_some() {
                    return;
                }
                let empty = match class_node.body() {
                    None => true,
                    Some(body) => {
                        if let Some(stmts) = body.as_statements_node() {
                            stmts.body().is_empty()
                        } else {
                            false
                        }
                    }
                };
                let loc = class_node.location();
                let (sl, _) = source.offset_to_line_col(loc.start_offset());
                let (el, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                (empty, class_node.class_keyword_loc(), sl, el)
            } else if let Some(sclass) = node.as_singleton_class_node() {
                let empty = match sclass.body() {
                    None => true,
                    Some(body) => {
                        if let Some(stmts) = body.as_statements_node() {
                            stmts.body().is_empty()
                        } else {
                            false
                        }
                    }
                };
                let loc = sclass.location();
                let (sl, _) = source.offset_to_line_col(loc.start_offset());
                let (el, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                (empty, sclass.class_keyword_loc(), sl, el)
            } else {
                return;
            };

        if !body_empty {
            return;
        }

        // AllowComments: default false per vendor config
        let allow_comments = config.get_bool("AllowComments", false);
        if allow_comments {
            let lines: Vec<&[u8]> = source.lines().collect();
            for line_num in start_line..=end_line {
                if let Some(line) = lines.get(line_num - 1) {
                    let trimmed = line
                        .iter()
                        .position(|&b| b != b' ' && b != b'\t')
                        .map(|start| &line[start..])
                        .unwrap_or(&[]);
                    if trimmed.starts_with(b"#") {
                        return;
                    }
                }
            }
        }

        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Empty class detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyClass, "cops/lint/empty_class");
}
