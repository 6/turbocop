use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SortMetadata;

impl Cop for SortMetadata {
    fn name(&self) -> &'static str {
        "RSpec/SortMetadata"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Must be an RSpec method
        if !is_rspec_example_group(method_name) && !is_rspec_example(method_name) {
            return;
        }

        // Must have a block
        if call.block().is_none() {
            return;
        }

        // Must be receiverless or RSpec.* / ::RSpec.*
        if let Some(recv) = call.receiver() {
            if util::constant_name(&recv).map_or(true, |n| n != b"RSpec") {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        // Collect trailing symbol arguments (metadata)
        // Find the first symbol argument after the description
        let mut symbol_names: Vec<(String, usize)> = Vec::new(); // (name, start_offset)
        let mut first_symbol_offset: Option<usize> = None;

        // Also collect keyword hash keys
        let mut hash_keys: Vec<(String, usize)> = Vec::new();

        for arg in arg_list.iter() {
            if let Some(sym) = arg.as_symbol_node() {
                let name = std::str::from_utf8(sym.unescaped())
                    .unwrap_or("")
                    .to_string();
                let offset = sym.location().start_offset();
                if first_symbol_offset.is_none() {
                    first_symbol_offset = Some(offset);
                }
                symbol_names.push((name, offset));
            } else if let Some(kw) = arg.as_keyword_hash_node() {
                for elem in kw.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        if let Some(key_sym) = assoc.key().as_symbol_node() {
                            let name = std::str::from_utf8(key_sym.unescaped())
                                .unwrap_or("")
                                .to_string();
                            let offset = elem.location().start_offset();
                            hash_keys.push((name, offset));
                        }
                    }
                }
            }
        }

        // Check if symbols are sorted
        let symbols_sorted = symbol_names.windows(2).all(|w| w[0].0 <= w[1].0);

        // Check if hash keys are sorted
        let hash_sorted = hash_keys.windows(2).all(|w| w[0].0 <= w[1].0);

        if !symbols_sorted || !hash_sorted {
            // Flag from first metadata to last
            let flag_offset = if !symbols_sorted {
                first_symbol_offset.unwrap_or(0)
            } else {
                hash_keys.first().map(|(_, o)| *o).unwrap_or(0)
            };

            let (line, column) = source.offset_to_line_col(flag_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Sort metadata alphabetically.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SortMetadata, "cops/rspec/sort_metadata");
}
