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
        let interp = match first_arg.as_interpolated_string_node() {
            Some(i) => i,
            None => return Vec::new(),
        };

        let has_interpolation = interp.parts().iter().any(|p| p.as_embedded_statements_node().is_some());

        if !has_interpolation {
            return Vec::new();
        }

        // Check if there are inline comments documenting the eval.
        // RuboCop checks that every interpolation line has a comment (# not followed by {).
        // We check both the string parts and the full source span.

        // Check each string part for inline comment patterns
        for part in interp.parts().iter() {
            if let Some(str_part) = part.as_string_node() {
                let content = str_part.content_loc().as_slice();
                if let Ok(s) = std::str::from_utf8(content) {
                    // Look for comment pattern: # not followed by {
                    for line in s.lines() {
                        if let Some(pos) = line.find(" # ") {
                            // Verify the # is not part of an interpolation marker
                            let after_hash = &line[pos + 2..];
                            if !after_hash.starts_with('{') {
                                return Vec::new();
                            }
                        }
                    }
                }
            }
        }

        // Also check the full location span (heredocs where body spans lines)
        let loc = first_arg.location();
        let start = loc.start_offset();
        let end = loc.end_offset();
        let content = &source.as_bytes()[start..end];
        if let Ok(content_str) = std::str::from_utf8(content) {
            for line in content_str.lines() {
                if let Some(pos) = line.rfind(" # ") {
                    // Verify this is a real comment, not # followed by { (interpolation)
                    let after_hash = &line[pos + 2..];
                    if !after_hash.starts_with('{') {
                        return Vec::new();
                    }
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
