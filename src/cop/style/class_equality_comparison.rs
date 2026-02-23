use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ClassEqualityComparison;

impl Cop for ClassEqualityComparison {
    fn name(&self) -> &'static str {
        "Style/ClassEqualityComparison"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_methods: Vec<String> = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| vec!["==".to_string(), "equal?".to_string(), "eql?".to_string()]);
        let allowed_patterns: Vec<regex::Regex> = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default()
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect();

        let mut visitor = ClassEqVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_methods,
            allowed_patterns,
            enclosing_def_name: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ClassEqVisitor<'a> {
    cop: &'a ClassEqualityComparison,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_methods: Vec<String>,
    allowed_patterns: Vec<regex::Regex>,
    enclosing_def_name: Option<Vec<u8>>,
}

impl<'a, 'pr> Visit<'pr> for ClassEqVisitor<'a> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let prev = self.enclosing_def_name.take();
        self.enclosing_def_name = Some(node.name().as_slice().to_vec());
        ruby_prism::visit_def_node(self, node);
        self.enclosing_def_name = prev;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_bytes = node.name().as_slice();

        // Must be ==, equal?, or eql?
        if method_bytes == b"==" || method_bytes == b"equal?" || method_bytes == b"eql?" {
            // Check if we're inside an allowed method
            if let Some(ref def_name) = self.enclosing_def_name {
                let def_str = std::str::from_utf8(def_name).unwrap_or("");
                if self.allowed_methods.iter().any(|m| m == def_str) {
                    // Inside an allowed method, skip
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            }

            // Receiver must be a `.class` call or `.class.name` call
            if let Some(receiver) = node.receiver() {
                if let Some(recv_call) = receiver.as_call_node() {
                    let is_class_call = recv_call.name().as_slice() == b"class";
                    let is_class_name_call = if !is_class_call {
                        let name = recv_call.name().as_slice();
                        if name == b"name" || name == b"to_s" || name == b"inspect" {
                            recv_call
                                .receiver()
                                .and_then(|ir| ir.as_call_node())
                                .is_some_and(|ic| ic.name().as_slice() == b"class")
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if is_class_call || is_class_name_call {
                        // Check AllowedPatterns against the source line
                        let loc = recv_call
                            .message_loc()
                            .unwrap_or_else(|| recv_call.location());
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        if !self.allowed_patterns.is_empty() {
                            if let Some(line_bytes) = self.source.lines().nth(line - 1) {
                                if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                                    if self.allowed_patterns.iter().any(|p| p.is_match(line_str)) {
                                        ruby_prism::visit_call_node(self, node);
                                        return;
                                    }
                                }
                            }
                        }
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Use `instance_of?` instead of comparing classes.".to_string(),
                        ));
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ClassEqualityComparison,
        "cops/style/class_equality_comparison"
    );
}
