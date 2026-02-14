use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnumSyntax;

impl Cop for EnumSyntax {
    fn name(&self) -> &'static str {
        "Rails/EnumSyntax"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_some() {
            return Vec::new();
        }

        if call.name().as_slice() != b"enum" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Old syntax: enum status: { active: 0 }
        // The first argument is a KeywordHashNode containing status: { ... }
        // New syntax: enum :status, { active: 0 }
        // The first argument is a SymbolNode
        if arg_list[0].as_symbol_node().is_some() {
            // Already using new syntax
            return Vec::new();
        }

        // Check if first arg is a keyword hash with a symbol key mapped to a hash value
        if let Some(kw) = arg_list[0].as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if assoc.key().as_symbol_node().is_some() {
                        // This is old syntax: enum status: { ... } or enum status: [...]
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Use Rails 7+ enum syntax: `enum :status, { active: 0 }`.".to_string(),
                        )];
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnumSyntax, "cops/rails/enum_syntax");
}
