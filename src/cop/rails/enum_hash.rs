use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnumHash;

impl Cop for EnumHash {
    fn name(&self) -> &'static str {
        "Rails/EnumHash"
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

        // Old syntax: enum status: [:active, :archived]
        // Parsed as: enum(KeywordHashNode { status: ArrayNode })
        for arg in &arg_list {
            if let Some(kw) = arg.as_keyword_hash_node() {
                for elem in kw.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        if assoc.value().as_array_node().is_some() {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Use hash syntax for `enum` values: `enum status: { active: 0, archived: 1 }`.".to_string(),
                            )];
                        }
                    }
                }
            }
        }

        // New syntax: enum :status, [:active, :archived]
        // Check if second arg is an array
        if arg_list.len() >= 2 && arg_list[0].as_symbol_node().is_some() {
            if arg_list[1].as_array_node().is_some() {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use hash syntax for `enum` values: `enum status: { active: 0, archived: 1 }`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnumHash, "cops/rails/enum_hash");
}
