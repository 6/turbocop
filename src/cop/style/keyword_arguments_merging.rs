use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_SPLAT_NODE, CALL_NODE, KEYWORD_HASH_NODE};

pub struct KeywordArgumentsMerging;

impl KeywordArgumentsMerging {
    /// Check if a call node is a `.merge(...)` call with a receiver.
    fn is_merge_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"merge" && call.receiver().is_some() {
                return true;
            }
        }
        false
    }
}

impl Cop for KeywordArgumentsMerging {
    fn name(&self) -> &'static str {
        "Style/KeywordArgumentsMerging"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_SPLAT_NODE, CALL_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };


        for arg in args.arguments().iter() {
            if let Some(kw_hash) = arg.as_keyword_hash_node() {
                let elements: Vec<_> = kw_hash.elements().iter().collect();
                // RuboCop only flags when the kwsplat is the first element
                // of the keyword hash (no preceding keyword args like key: val).
                if let Some(first) = elements.first() {
                    if let Some(splat) = first.as_assoc_splat_node() {
                        if let Some(value) = splat.value() {
                            if Self::is_merge_call(&value) {
                                let merge_call = value.as_call_node().unwrap();
                                let receiver = merge_call.receiver().unwrap();
                                let (line, column) = source.offset_to_line_col(receiver.location().start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Provide additional arguments directly rather than using `merge`.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(KeywordArgumentsMerging, "cops/style/keyword_arguments_merging");
}
