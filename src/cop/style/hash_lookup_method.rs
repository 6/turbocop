use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashLookupMethod;

impl Cop for HashLookupMethod {
    fn name(&self) -> &'static str {
        "Style/HashLookupMethod"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let style = config.get_str("EnforcedStyle", "brackets");
        let method_bytes = call.name().as_slice();

        match style {
            "brackets" => {
                // Flag fetch calls, suggest []
                if method_bytes == b"fetch" {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        // Only flag fetch with exactly 1 argument (no default)
                        if arg_list.len() == 1 && call.block().is_none() {
                            if call.receiver().is_some() {
                                let loc = call.message_loc().unwrap_or_else(|| call.location());
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                return vec![self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Use `[]` instead of `fetch`.".to_string(),
                                )];
                            }
                        }
                    }
                }
            }
            "fetch" => {
                // Flag [] calls, suggest fetch
                if method_bytes == b"[]" {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 && call.receiver().is_some() {
                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Use `fetch` instead of `[]`.".to_string(),
                            )];
                        }
                    }
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashLookupMethod, "cops/style/hash_lookup_method");
}
