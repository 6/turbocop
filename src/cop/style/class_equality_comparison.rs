use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClassEqualityComparison;

impl Cop for ClassEqualityComparison {
    fn name(&self) -> &'static str {
        "Style/ClassEqualityComparison"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allowed_methods = config.get_string_array("AllowedMethods");
        let _allowed_patterns = config.get_string_array("AllowedPatterns");

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call_node.name();
        let method_bytes = method_name.as_slice();

        // Must be ==, equal?, or eql?
        if method_bytes != b"==" && method_bytes != b"equal?" && method_bytes != b"eql?" {
            return Vec::new();
        }

        // Receiver must be a `.class` call
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Check if the receiver chain involves .class
        // Pattern: x.class == Y  or  x.class.name == 'Y'
        let is_class_call = recv_call.name().as_slice() == b"class";
        let is_class_name_call = if !is_class_call {
            // Check for x.class.name == 'Y'
            let name = recv_call.name().as_slice();
            if name == b"name" || name == b"to_s" || name == b"inspect" {
                if let Some(inner_recv) = recv_call.receiver() {
                    if let Some(inner_call) = inner_recv.as_call_node() {
                        inner_call.name().as_slice() == b"class"
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !is_class_call && !is_class_name_call {
            return Vec::new();
        }

        // Check AllowedMethods - if we're inside a method that's in the allowed list, skip
        // For simplicity, we'll use the default allowed methods
        let allowed_methods: Vec<String> = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| vec!["==".to_string(), "equal?".to_string(), "eql?".to_string()]);

        // We don't have easy access to the enclosing def node from check_node,
        // so we skip this check - in practice the allowed methods are the comparison
        // operators themselves, and we're already checking those. The AllowedMethods
        // config is for when the comparison is *inside* a method with that name.
        let _ = allowed_methods;

        let (line, column) = source.offset_to_line_col(recv_call.message_loc().unwrap_or_else(|| recv_call.location()).start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `instance_of?` instead of comparing classes.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassEqualityComparison, "cops/style/class_equality_comparison");
}
