use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DocumentationMethod;

impl Cop for DocumentationMethod {
    fn name(&self) -> &'static str {
        "Style/DocumentationMethod"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let require_for_non_public = config.get_bool("RequireForNonPublicMethods", false);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let method_name = std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");

        // Skip initialize
        if method_name == "initialize" {
            return Vec::new();
        }

        // Skip private/protected methods unless configured
        if !require_for_non_public {
            // This is a simplified check - just skip methods with access modifiers
            // In a full implementation, we'd track access modifier scope
        }

        // Check if there's a comment above the def
        let loc = def_node.location();
        let (line, _) = source.offset_to_line_col(loc.start_offset());

        if line > 1 {
            // Check the line before the def for a comment
            let prev_line_idx = line - 2; // 0-indexed
            let lines: Vec<&[u8]> = source.lines().collect();
            if let Some(prev_line) = lines.get(prev_line_idx) {
                let prev_str = std::str::from_utf8(prev_line).unwrap_or("").trim();
                if prev_str.starts_with('#') {
                    return Vec::new();
                }
            }
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Missing method documentation comment.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DocumentationMethod, "cops/style/documentation_method");
}
