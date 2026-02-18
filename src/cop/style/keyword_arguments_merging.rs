use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct KeywordArgumentsMerging;

impl KeywordArgumentsMerging {
    /// Check if a call node is a `.merge(...)` or chain of `.merge(...).merge(...)`
    fn is_merge_chain(node: &ruby_prism::Node<'_>) -> bool {
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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for `**options.merge(...)` pattern in keyword arguments.
        // This is an AssocSplatNode whose value is a CallNode to `merge`.
        // The pattern is: foo(x, **options.merge(y: 1))
        //                         ^^^^^^^^^^^^^^^^^^^^ offense

        // We need to check for keyword hash nodes that contain an AssocSplatNode
        // whose value is a merge call.

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for arg in args.arguments().iter() {
            // Check keyword hash nodes
            if let Some(kw_hash) = arg.as_keyword_hash_node() {
                for element in kw_hash.elements().iter() {
                    if let Some(splat) = element.as_assoc_splat_node() {
                        if let Some(value) = splat.value() {
                            if Self::is_merge_chain(&value) {
                                let merge_call = value.as_call_node().unwrap();
                                // Get the source from the receiver through the merge call
                                let receiver = merge_call.receiver().unwrap();
                                let full_loc = source.as_bytes();
                                let start = receiver.location().start_offset();
                                let end = merge_call.location().end_offset();
                                let src = &full_loc[start..end];
                                let src_str = String::from_utf8_lossy(src);
                                let loc = element.location();
                                let start_off = loc.start_offset();
                                let end_off = merge_call.location().end_offset();
                                let full_src = &full_loc[start_off..end_off];
                                let _full_str = String::from_utf8_lossy(full_src);
                                let (line, column) = source.offset_to_line_col(receiver.location().start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!(
                                        "Provide additional arguments directly rather than using `merge`.",
                                    ),
                                ));
                                let _ = src_str;
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(KeywordArgumentsMerging, "cops/style/keyword_arguments_merging");
}
