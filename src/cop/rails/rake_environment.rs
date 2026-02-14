use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RakeEnvironment;

impl Cop for RakeEnvironment {
    fn name(&self) -> &'static str {
        "Rails/RakeEnvironment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Start from CallNode `task`, then check if it has a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be receiverless `task` call
        if call.receiver().is_some() {
            return Vec::new();
        }

        if call.name().as_slice() != b"task" {
            return Vec::new();
        }

        // Must have a block
        if call.block().is_none() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Check if first arg is a symbol (simple task definition)
        let has_symbol_arg = arg_list[0].as_symbol_node().is_some();
        if !has_symbol_arg {
            return Vec::new();
        }

        // Check if any argument is a hash containing :environment
        for arg in &arg_list {
            if let Some(kw) = arg.as_keyword_hash_node() {
                for elem in kw.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        if let Some(sym) = assoc.value().as_symbol_node() {
                            if sym.unescaped() == b"environment" {
                                return Vec::new();
                            }
                        }
                    }
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Add `:environment` dependency to the rake task.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RakeEnvironment, "cops/rails/rake_environment");
}
