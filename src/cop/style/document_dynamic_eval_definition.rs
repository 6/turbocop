use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DocumentDynamicEvalDefinition;

const EVAL_METHODS: &[&str] = &[
    "class_eval",
    "module_eval",
    "instance_eval",
    "class_exec",
    "module_exec",
    "instance_exec",
];

impl Cop for DocumentDynamicEvalDefinition {
    fn name(&self) -> &'static str {
        "Style/DocumentDynamicEvalDefinition"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !EVAL_METHODS.contains(&method_name) {
            return Vec::new();
        }

        // Check if the first argument is a string/heredoc with interpolation
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];

        // Check for interpolated string
        let has_interpolation = if let Some(interp) = first_arg.as_interpolated_string_node() {
            interp.parts().iter().any(|p| p.as_embedded_statements_node().is_some())
        } else {
            false
        };

        if !has_interpolation {
            return Vec::new();
        }

        // Check if there's a comment on the same line or within the heredoc
        let loc = first_arg.location();
        let start = loc.start_offset();
        let end = loc.end_offset();
        let content = &source.as_bytes()[start..end];
        let content_str = std::str::from_utf8(content).unwrap_or("");
        if content_str.contains('#') {
            // There might be inline comments documenting the eval
            let lines: Vec<&str> = content_str.lines().collect();
            for line in &lines {
                if let Some(comment_pos) = line.rfind(" # ") {
                    // Has an inline comment
                    let _ = comment_pos;
                    return Vec::new();
                }
            }
        }

        let loc = if call.receiver().is_some() {
            call.message_loc().unwrap_or(call.location())
        } else {
            call.location()
        };
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Add a comment block showing its appearance if interpolated.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DocumentDynamicEvalDefinition,
        "cops/style/document_dynamic_eval_definition"
    );
}
