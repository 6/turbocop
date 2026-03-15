use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_hook,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-14)
///
/// FP=2, FN=8.
///
/// FP=2: asciidoctor__asciidoctor-pdf repo, spec/cli_spec.rb:93 and :102.
/// Both are `it '...', cli: true, visual: true, if: ..., &(proc do ... end)`.
/// Root cause: `&(proc do end)` stores a BlockArgumentNode in call.block(),
/// not a BlockNode. RuboCop's on_block pattern only fires for BlockNode.
/// Fix: require call.block().as_block_node().is_some() instead of is_some().
///
/// ## Corpus investigation (2026-03-15)
///
/// FN=10: All from hooks (`before`/`after`/`around`) called on config objects
/// with unsorted metadata, e.g. `config.after(:each, type: :system, js: true)`.
/// Root cause: cop only checked `is_rspec_example_group` and `is_rspec_example`,
/// missing hook methods entirely. Also, hooks are called on config variables
/// (not `RSpec.*` constants), so the receiver check needed relaxing for hooks.
/// Fix: added `is_rspec_hook` check; for hooks, accept any receiver.
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

        let is_hook = is_rspec_hook(method_name);

        // Must be an RSpec example/group method or a hook
        if !is_hook && !is_rspec_example_group(method_name) && !is_rspec_example(method_name) {
            return;
        }

        // Must have a BlockNode (do...end or { }), not BlockArgumentNode (&proc)
        if call.block().is_none_or(|b| b.as_block_node().is_none()) {
            return;
        }

        // For example/group methods: must be receiverless or RSpec.* / ::RSpec.*
        // For hooks: accept any receiver (e.g. config.before, c.after)
        if !is_hook {
            if let Some(recv) = call.receiver() {
                if util::constant_name(&recv).is_none_or(|n| n != b"RSpec") {
                    return;
                }
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let all_args: Vec<_> = args.arguments().iter().collect();

        // For hooks, skip the first argument (scope like :each, :all, :suite, :context).
        // RuboCop's Metadata mixin pattern uses `_ $...` to skip the first arg.
        // For example/group methods, skip nothing (the description is handled by
        // the trailing-symbol logic below which only collects trailing symbols).
        let arg_list = if is_hook && !all_args.is_empty() {
            &all_args[1..]
        } else {
            &all_args[..]
        };

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
