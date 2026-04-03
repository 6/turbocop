use crate::cop::shared::node_type::{
    ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::shared::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_hook,
    is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// FP=8, FN=71. Root cause of FN=71: nitrocop only checked example groups and
/// examples but RuboCop's `Metadata` mixin also covers shared groups
/// (`shared_examples`, `shared_context`, `shared_examples_for`) and hooks
/// (`before`, `after`, `around`, etc.). Added `is_rspec_shared_group` and
/// `is_rspec_hook` checks. Also added block requirement (RuboCop uses
/// `on_block` / `on_numblock`).
///
/// For hooks, the first arg is the scope (`:each`, `:all`, etc.), not a
/// description string. Metadata follows after the scope arg.
///
/// ## Corpus investigation (2026-03-11)
///
/// FP=21, FN=63. Root causes:
/// - FN: Explicit hash metadata `{ a: true }` parsed as `HashNode` (not
///   `KeywordHashNode`) was not handled. Added `HashNode` support.
/// - FN: Hooks inside `RSpec.configure do |config| ... end` blocks
///   (`config.before(:each, :foo)`) have a local variable receiver and no
///   block. RuboCop's `metadata_in_block` pattern handles these. Added support
///   by detecting `RSpec.configure` calls and walking their block body for
///   hook calls on the block parameter variable.
/// - FP: Block argument `&(proc do...end)` was treated as a real block.
///   Added `as_block_node()` check to distinguish from `BlockArgumentNode`.
/// - FP: Need to skip the first argument (description/scope) per RuboCop's
///   `_ $...` pattern — the first arg is not metadata.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=0, FN=3 (2 from thredded, 1 from discourse).
///
/// FN=3: Hook calls inside `if ENV['MIGRATION_SPEC']` conditional blocks within
/// `RSpec.configure`. `walk_for_config_hooks` only checked direct statements,
/// not branches. Fix: recurse into if/unless/else branches.
pub struct MetadataStyle;

/// Default enforces symbol style: `:foo` instead of `foo: true`.
impl Cop for MetadataStyle {
    fn name(&self) -> &'static str {
        "RSpec/MetadataStyle"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Handle RSpec.configure blocks: walk their body for hook calls
        if method_name == b"configure" {
            self.check_configure_call(source, &call, config, diagnostics);
            return;
        }

        if !is_rspec_example_group(method_name)
            && !is_rspec_example(method_name)
            && !is_rspec_shared_group(method_name)
            && !is_rspec_hook(method_name)
        {
            return;
        }

        // RuboCop uses on_block: requires a real block (do...end / {}) wrapping the call.
        // BlockArgumentNode (&block) does not count.
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        if block.as_block_node().is_none() {
            return;
        }

        // Must be receiverless or RSpec.describe / ::RSpec.describe
        if let Some(recv) = call.receiver() {
            if util::constant_name(&recv).is_none_or(|n| n != b"RSpec") {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // RuboCop's Metadata mixin pattern is `(send #rspec? {methods} _ $...)`:
        // skip the first argument (description string for groups/examples,
        // scope symbol for hooks), then process the rest as metadata.
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }
        let metadata_args = &arg_list[1..];

        let style = config.get_str("EnforcedStyle", "symbol");
        self.check_metadata_args(source, metadata_args, style, diagnostics);
    }
}

impl MetadataStyle {
    /// Check metadata arguments for style violations.
    /// `metadata_args` should already have the first argument (description/scope) removed.
    fn check_metadata_args(
        &self,
        source: &SourceFile,
        metadata_args: &[ruby_prism::Node<'_>],
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if style == "symbol" {
            // Flag `key: true` keyword args — should be `:key` symbol style
            for arg in metadata_args {
                self.check_hash_like_for_symbol_style(source, arg, diagnostics);
            }
        } else if style == "hash" {
            // Flag `:key` symbol args — should be `key: true` hash style
            for arg in metadata_args {
                if let Some(sym) = arg.as_symbol_node() {
                    let loc = sym.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use hash style for metadata.".to_string(),
                    ));
                }
            }
        }
    }

    /// Check a single argument for `key: true` pairs that should be `:key` symbols.
    /// Handles both `KeywordHashNode` (implicit hash) and `HashNode` (explicit `{ }` hash).
    fn check_hash_like_for_symbol_style(
        &self,
        source: &SourceFile,
        arg: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Collect elements from either KeywordHashNode or HashNode
        let elements: Option<ruby_prism::NodeList<'_>> =
            if let Some(kw) = arg.as_keyword_hash_node() {
                Some(kw.elements())
            } else {
                arg.as_hash_node().map(|h| h.elements())
            };

        let elements = match elements {
            Some(e) => e,
            None => return,
        };

        for elem in elements.iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                // Key must be a symbol
                if assoc.key().as_symbol_node().is_none() {
                    continue;
                }
                // Value must be `true`
                if assoc.value().as_true_node().is_some() {
                    let loc = elem.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use symbol style for metadata.".to_string(),
                    ));
                }
            }
        }
    }

    /// Handle `RSpec.configure do |config| ... end` calls.
    /// Inside these blocks, hook calls like `config.before(:each, :foo)` are checked
    /// for metadata style. These calls have a local variable receiver and no block.
    fn check_configure_call(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Receiver must be RSpec
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if util::constant_name(&recv).is_none_or(|n| n != b"RSpec") {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(bn) => bn,
            None => return,
        };

        // Get the block parameter name (e.g., `config` from `do |config|`)
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let param_list = match block_params.parameters() {
            Some(pl) => pl,
            None => return,
        };
        let requireds = param_list.requireds();
        let first_param = match requireds.iter().next() {
            Some(p) => p,
            None => return,
        };
        let param_name = match first_param.as_required_parameter_node() {
            Some(rp) => rp.name(),
            None => return,
        };

        // Walk the block body looking for hook calls on the config variable
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let style = config.get_str("EnforcedStyle", "symbol");
        self.walk_for_config_hooks(source, &body, param_name.as_slice(), style, diagnostics);
    }

    /// Walk statements looking for hook calls on the config variable.
    /// Recurses into if/unless branches to find hook calls inside conditionals.
    fn walk_for_config_hooks(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        param_name: &[u8],
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(stmts) = node.as_statements_node() {
            for stmt in stmts.body().iter() {
                self.walk_for_config_hooks_single(source, &stmt, param_name, style, diagnostics);
            }
        } else {
            self.walk_for_config_hooks_single(source, node, param_name, style, diagnostics);
        }
    }

    /// Process a single statement: check if it's a hook call or recurse into if/unless.
    fn walk_for_config_hooks_single(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        param_name: &[u8],
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Direct hook call
        if node.as_call_node().is_some() {
            self.check_config_hook_call(source, node, param_name, style, diagnostics);
            return;
        }
        // Recurse into if/unless branches
        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                self.walk_for_config_hooks(
                    source,
                    &stmts.as_node(),
                    param_name,
                    style,
                    diagnostics,
                );
            }
            if let Some(subsequent) = if_node.subsequent() {
                self.walk_for_config_hooks(source, &subsequent, param_name, style, diagnostics);
            }
            return;
        }
        if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                self.walk_for_config_hooks(
                    source,
                    &stmts.as_node(),
                    param_name,
                    style,
                    diagnostics,
                );
            }
            if let Some(else_clause) = unless_node.else_clause() {
                self.walk_for_config_hooks(
                    source,
                    &else_clause.as_node(),
                    param_name,
                    style,
                    diagnostics,
                );
            }
            return;
        }
        if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                self.walk_for_config_hooks(
                    source,
                    &stmts.as_node(),
                    param_name,
                    style,
                    diagnostics,
                );
            }
        }
    }

    /// Check if a single node is a hook call on the config variable.
    fn check_config_hook_call(
        &self,
        source: &SourceFile,
        stmt: &ruby_prism::Node<'_>,
        param_name: &[u8],
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match stmt.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let method_name = call.name().as_slice();
        if !is_rspec_hook(method_name) {
            return;
        }
        // Check receiver is the config variable
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
        // Found a hook call on the config variable
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }
        let metadata_args = &arg_list[1..];
        self.check_metadata_args(source, metadata_args, style, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MetadataStyle, "cops/rspec/metadata_style");
}
