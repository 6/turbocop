use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct KeywordArgumentsMerging;

impl Cop for KeywordArgumentsMerging {
    fn name(&self) -> &'static str {
        "Style/KeywordArgumentsMerging"
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

        // Check for method(a: 1, **hash) pattern
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.is_empty() {
                return Vec::new();
            }

            // Look for a keyword hash that mixes explicit keywords with double-splat
            let last_arg = &arg_list[arg_list.len() - 1];
            if let Some(kw_hash) = last_arg.as_keyword_hash_node() {
                let elements: Vec<_> = kw_hash.elements().iter().collect();
                let has_assoc = elements.iter().any(|e| e.as_assoc_node().is_some());
                let has_splat = elements.iter().any(|e| e.as_assoc_splat_node().is_some());

                if has_assoc && has_splat {
                    let loc = kw_hash.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not mix keyword arguments and hash splat.".to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(KeywordArgumentsMerging, "cops/style/keyword_arguments_merging");
}
