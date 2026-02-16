use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct DuplicateRequire;

impl Cop for DuplicateRequire {
    fn name(&self) -> &'static str {
        "Lint/DuplicateRequire"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut visitor = RequireVisitor {
            cop: self,
            source,
            seen: HashMap::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct RequireVisitor<'a, 'src> {
    cop: &'a DuplicateRequire,
    source: &'src SourceFile,
    /// Maps (method_name, argument_string) -> first occurrence offset
    seen: HashMap<(Vec<u8>, Vec<u8>), usize>,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RequireVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        if (method_name == b"require" || method_name == b"require_relative")
            && node.receiver().is_none()
        {
            if let Some(args) = node.arguments() {
                let arg_list = args.arguments();
                if arg_list.len() == 1 {
                    if let Some(first) = arg_list.iter().next() {
                        if let Some(s) = first.as_string_node() {
                            let key = (method_name.to_vec(), s.unescaped().to_vec());
                            let loc = node.location();
                            if let Some(_first_offset) = self.seen.get(&key) {
                                let (line, column) =
                                    self.source.offset_to_line_col(loc.start_offset());
                                self.diagnostics.push(self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    "Duplicate `require` detected.".to_string(),
                                ));
                            } else {
                                self.seen.insert(key, loc.start_offset());
                            }
                        }
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
    crate::cop_fixture_tests!(DuplicateRequire, "cops/lint/duplicate_require");
}
