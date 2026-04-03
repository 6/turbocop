use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_hook,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

struct MetadataSymbol {
    sort_key: Vec<u8>,
    start_offset: usize,
}

struct MetadataPair {
    sort_key: Vec<u8>,
    start_offset: usize,
}

/// ## Corpus investigation (2026-03-29)
///
/// FP=9, FN=3.
///
/// FP roots:
/// - RuboCop's `Metadata` mixin always skips the first positional argument for
///   examples and groups. Descriptionless groups like
///   `RSpec.describe type: :model, swars_spec: true` therefore have no trailing
///   metadata to sort. Nitrocop incorrectly treated that first hash as metadata.
/// - RuboCop sorts hash pairs by `pair.key.source.downcase`, not by normalized
///   symbol names. Mixed styles like
///   `:transactions => false, read_transaction: true` are therefore already in
///   order and must not be flagged.
///
/// FN root:
/// - Mixed metadata hashes such as `js: true, :retry => 3` are parsed as
///   `HashNode`, not `KeywordHashNode`. Nitrocop only handled keyword hashes and
///   missed those offenses.
///
/// Fix: mirror RuboCop's metadata boundary rules by skipping the first
/// positional arg, walking `RSpec.configure` block variables for config hooks,
/// handling both `HashNode` and `KeywordHashNode`, collecting only trailing
/// symbols, and sorting hash keys by source text.
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
        &[CALL_NODE]
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

        if method_name == b"configure" {
            self.check_configure_call(source, &call, diagnostics);
            return;
        }

        if !is_rspec_example_group(method_name)
            && !is_rspec_example(method_name)
            && !is_rspec_hook(method_name)
        {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        if block.as_block_node().is_none() {
            return;
        }

        if let Some(recv) = call.receiver() {
            if constant_predicates::constant_short_name(&recv).is_none_or(|n| n != b"RSpec") {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        self.check_metadata_args(source, &arg_list[1..], diagnostics);
    }
}

impl SortMetadata {
    fn check_metadata_args(
        &self,
        source: &SourceFile,
        metadata_args: &[ruby_prism::Node<'_>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if metadata_args.is_empty() {
            return;
        }

        let (args_without_hash, trailing_hash) =
            if Self::hash_like_elements(metadata_args.last().unwrap()).is_some() {
                (
                    &metadata_args[..metadata_args.len() - 1],
                    metadata_args.last(),
                )
            } else {
                (metadata_args, None)
            };

        let symbol_args = if Self::last_arg_could_be_hash(args_without_hash) {
            &args_without_hash[..args_without_hash.len() - 1]
        } else {
            args_without_hash
        };

        let symbols = Self::collect_trailing_symbols(symbol_args);
        let pairs = trailing_hash
            .map(|arg| Self::collect_hash_pairs(source, arg))
            .unwrap_or_default();

        let symbols_sorted = symbols.windows(2).all(|w| w[0].sort_key <= w[1].sort_key);
        let pairs_sorted = pairs.windows(2).all(|w| w[0].sort_key <= w[1].sort_key);

        if symbols_sorted && pairs_sorted {
            return;
        }

        let flag_offset = symbols
            .first()
            .map(|sym| sym.start_offset)
            .or_else(|| pairs.first().map(|pair| pair.start_offset))
            .unwrap_or(0);

        let (line, column) = source.offset_to_line_col(flag_offset);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Sort metadata alphabetically.".to_string(),
        ));
    }

    fn collect_trailing_symbols(args: &[ruby_prism::Node<'_>]) -> Vec<MetadataSymbol> {
        let mut symbols = Vec::new();

        for arg in args.iter().rev() {
            let Some(sym) = arg.as_symbol_node() else {
                break;
            };

            symbols.push(MetadataSymbol {
                sort_key: sym
                    .unescaped()
                    .iter()
                    .map(|b| b.to_ascii_lowercase())
                    .collect(),
                start_offset: sym.location().start_offset(),
            });
        }

        symbols.reverse();
        symbols
    }

    fn collect_hash_pairs(source: &SourceFile, arg: &ruby_prism::Node<'_>) -> Vec<MetadataPair> {
        let Some(elements) = Self::hash_like_elements(arg) else {
            return Vec::new();
        };

        let mut pairs = Vec::new();
        for elem in elements.iter() {
            let Some(assoc) = elem.as_assoc_node() else {
                continue;
            };

            let key_loc = assoc.key().location();
            let sort_key = source.as_bytes()[key_loc.start_offset()..key_loc.end_offset()]
                .iter()
                .map(|b| b.to_ascii_lowercase())
                .collect();

            pairs.push(MetadataPair {
                sort_key,
                start_offset: elem.location().start_offset(),
            });
        }

        pairs
    }

    fn hash_like_elements<'a>(arg: &'a ruby_prism::Node<'a>) -> Option<ruby_prism::NodeList<'a>> {
        if let Some(kw) = arg.as_keyword_hash_node() {
            Some(kw.elements())
        } else {
            arg.as_hash_node().map(|hash| hash.elements())
        }
    }

    fn last_arg_could_be_hash(args: &[ruby_prism::Node<'_>]) -> bool {
        let Some(last) = args.last() else {
            return false;
        };

        last.as_hash_node().is_none()
            && last.as_keyword_hash_node().is_none()
            && last.as_symbol_node().is_none()
            && last.as_string_node().is_none()
            && last.as_interpolated_string_node().is_none()
    }

    fn check_configure_call(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if constant_predicates::constant_short_name(&recv).is_none_or(|n| n != b"RSpec") {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(node) => node,
            None => return,
        };

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let param_list = match block_params.parameters() {
            Some(list) => list,
            None => return,
        };
        let first_param = match param_list.requireds().iter().next() {
            Some(param) => param,
            None => return,
        };
        let param_name = match first_param.as_required_parameter_node() {
            Some(param) => param.name(),
            None => return,
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        self.walk_for_config_hooks(source, &body, param_name.as_slice(), diagnostics);
    }

    fn walk_for_config_hooks(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        param_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(stmts) = node.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.walk_for_config_hooks_single(source, &stmt, param_name, diagnostics);
            }
        } else {
            self.walk_for_config_hooks_single(source, node, param_name, diagnostics);
        }
    }

    fn walk_for_config_hooks_single(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        param_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if node.as_call_node().is_some() {
            self.check_config_hook_call(source, node, param_name, diagnostics);
            return;
        }

        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                self.walk_for_config_hooks(source, &stmts.as_node(), param_name, diagnostics);
            }
            if let Some(subsequent) = if_node.subsequent() {
                self.walk_for_config_hooks(source, &subsequent, param_name, diagnostics);
            }
            return;
        }

        if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                self.walk_for_config_hooks(source, &stmts.as_node(), param_name, diagnostics);
            }
            if let Some(else_clause) = unless_node.else_clause() {
                self.walk_for_config_hooks(source, &else_clause.as_node(), param_name, diagnostics);
            }
            return;
        }

        if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                self.walk_for_config_hooks(source, &stmts.as_node(), param_name, diagnostics);
            }
        }
    }

    fn check_config_hook_call(
        &self,
        source: &SourceFile,
        stmt: &ruby_prism::Node<'_>,
        param_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match stmt.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if !is_rspec_hook(call.name().as_slice()) {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let lvar = match recv.as_local_variable_read_node() {
            Some(l) => l,
            None => return,
        };
        if lvar.name().as_slice() != param_name {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        self.check_metadata_args(source, &arg_list[1..], diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SortMetadata, "cops/rspec/sort_metadata");
}
