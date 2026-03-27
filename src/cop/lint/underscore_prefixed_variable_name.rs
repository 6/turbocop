use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for underscore-prefixed variables that are actually used.
///
/// RuboCop uses VariableForce to track variable scoping across all scope types
/// (def, block, lambda, top-level, class, module). This implementation replicates
/// that behavior by treating each scope type independently:
/// - Each scope (def, block, lambda, top-level, class/module body) checks variables
///   declared within it (params + direct local var writes at that level).
/// - Write collection stops at inner scope boundaries (blocks, lambdas, defs, classes,
///   modules) so each scope only handles its own variables.
/// - Read collection crosses into blocks and lambdas (since both can read outer-scope
///   variables) but stops at defs/classes/modules (which create new variable scopes).
///
/// This matches RuboCop's VariableForce model where blocks are "twisted scopes"
/// that create their own variable tables. Variables first assigned inside a block
/// belong to that block's scope, not the enclosing scope.
///
/// Key behaviors matching RuboCop:
/// - Flags underscore-prefixed method params, block params, and local variable
///   assignments that are subsequently read in the same scope.
/// - Includes bare `_` — if `_` is used (read), it's an offense.
/// - Respects block parameter shadowing: if a block redefines a param with the
///   same name, reads inside the block are attributed to the block param, not
///   the outer scope variable.
/// - Handles `AllowKeywordBlockArguments` config to skip keyword block params.
/// - Skips variables implicitly forwarded via bare `super` or `binding`.
/// - Handles top-level scope, class/module bodies, and nested blocks.
/// - Handles destructured block parameters (e.g., `|(a, _b)|`).
///
/// Supported variable declaration types (matching RuboCop's VariableForce):
/// - Required, optional, rest, keyword, keyword-rest, and block-pass parameters
/// - Local variable writes (`_x = 1`)
/// - Multi-assignment targets (`_a, _b = 1, 2`)
/// - Named capture regex (`/(?<_name>\w+)/ =~ str`)
/// - For-loop index variables (`for _x in items`)
/// - Operator writes (`_x += 1`, `_x ||= 1`, `_x &&= 1`) count as both
///   writes and reads (they read the variable before writing)
///
/// Scoping model changes (matching RuboCop VariableForce):
/// - Each block independently checks local variable writes within it.
/// - Def/lambda/top-level scopes check only direct local var writes (not crossing
///   into blocks). Reads still cross into blocks to catch outer-scope references.
/// - To prevent double-reporting when a variable is written at def level and
///   reassigned inside a block, blocks skip writes for names already declared
///   in an enclosing scope (tracked via `outer_var_names`).
/// - Class/module bodies are visited so blocks within them are checked.
///
/// Historical bugs fixed:
/// - `check_def` returned early when no underscore vars in def scope, skipping
///   visit of nested blocks/lambdas. Fixed to always visit body for nested scopes.
/// - Bare `_` was excluded from checks. RuboCop checks it.
/// - Destructured block params (MultiTargetNode) were not collected.
/// - Block-scope WriteCollector picked up reassignments of outer scope variables,
///   causing FP double-reporting.
/// - Blocks inside class/module bodies were never checked (FN). Fixed by adding
///   class/module visiting to ScopeFinder.
/// - Multiple blocks in the same def with the same variable name only reported
///   the first occurrence (FN). Fixed by making each block its own scope.
/// - Variables in different blocks at top-level sharing the same name caused
///   FP (reads from one block attributed to writes in another). Fixed by per-block
///   scoping.
/// - Lambda read collection: Initially, lambda bodies were traversed during read
///   collection, causing FPs. Then read collection was blocked at lambda boundaries,
///   fixing FPs but introducing FNs (corpus evidence shows RuboCop DOES flag
///   outer-scope underscore vars read inside lambdas). Final fix: cross lambda
///   boundaries during read collection (like blocks) with parameter shadowing,
///   matching RuboCop's actual VariableForce behavior. Also fixed check_lambda
///   to filter out reassignments of outer-scope variables (matching check_block
///   behavior).
/// - Class superclass expressions were skipped during read collection, causing
///   FNs for patterns like `_Base = Spark::Command::Base` followed by
///   `class Spark::Command::Map < _Base`. Fixed by visiting `ClassNode`
///   superclasses as twisted-scope reads while still skipping class bodies.
pub struct UnderscorePrefixedVariableName;

impl Cop for UnderscorePrefixedVariableName {
    fn name(&self) -> &'static str {
        "Lint/UnderscorePrefixedVariableName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_keyword_block_args = config.get_bool("AllowKeywordBlockArguments", false);
        let mut visitor = ScopeFinder {
            cop: self,
            source,
            allow_keyword_block_args,
            diagnostics: Vec::new(),
            outer_var_names: HashSet::new(),
        };
        // Check top-level scope first
        visitor.check_scope_body(&parse_result.node());
        // Then visit nested scopes
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ScopeFinder<'a, 'src> {
    cop: &'a UnderscorePrefixedVariableName,
    source: &'src SourceFile,
    allow_keyword_block_args: bool,
    diagnostics: Vec<Diagnostic>,
    /// Variable names declared in enclosing scopes. Used by check_block to
    /// skip reassignments of outer-scope variables (avoids double-reporting).
    outer_var_names: HashSet<String>,
}

impl<'pr> Visit<'pr> for ScopeFinder<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.check_def(node);
        // Don't recurse into nested defs — each is its own scope
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.check_block(node);
        // Don't recurse — nested blocks are checked when we visit the block's body
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.check_lambda(node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Enter class body so blocks inside it are visited.
        // Reset outer_var_names since class creates a new variable scope.
        let old_outer = std::mem::take(&mut self.outer_var_names);
        if let Some(body) = node.body() {
            self.check_scope_body(&body);
            self.visit(&body);
        }
        self.outer_var_names = old_outer;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        // Enter module body so blocks inside it are visited.
        let old_outer = std::mem::take(&mut self.outer_var_names);
        if let Some(body) = node.body() {
            self.check_scope_body(&body);
            self.visit(&body);
        }
        self.outer_var_names = old_outer;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        // Enter singleton class body so blocks inside it are visited.
        let old_outer = std::mem::take(&mut self.outer_var_names);
        if let Some(body) = node.body() {
            self.check_scope_body(&body);
            self.visit(&body);
        }
        self.outer_var_names = old_outer;
    }
}

impl ScopeFinder<'_, '_> {
    fn check_def(&mut self, def_node: &ruby_prism::DefNode<'_>) {
        let mut underscore_vars: Vec<UnderscoreVar> = Vec::new();

        if let Some(params) = def_node.parameters() {
            collect_underscore_params(&params, &mut underscore_vars, false);
        }

        // Collect underscore-prefixed local variable writes in the body.
        // WriteCollector stops at blocks — each block handles its own writes.
        // Still stops at lambdas and nested defs (which create new scopes).
        if let Some(body) = def_node.body() {
            let mut write_collector = WriteCollector { writes: Vec::new() };
            write_collector.visit(&body);
            underscore_vars.extend(write_collector.writes);
        }

        // Build the set of variable names declared at this def level
        let def_var_names: HashSet<String> =
            underscore_vars.iter().map(|v| v.name.clone()).collect();

        if !underscore_vars.is_empty() {
            // Collect all local variable reads in the body, respecting block scoping
            let mut reads = HashSet::new();
            if let Some(body) = def_node.body() {
                collect_reads_scope_aware(&body, &mut reads);
            }
            // Also collect reads from parameter default values (e.g., locale: _locale)
            if let Some(params) = def_node.parameters() {
                collect_reads_from_param_defaults(&params, &mut reads);
            }

            // Check for implicit forwarding (bare `super` or `binding`)
            let has_forwarding = if let Some(body) = def_node.body() {
                check_forwarding(&body)
            } else {
                false
            };

            self.emit_diagnostics(&underscore_vars, &reads, has_forwarding);
        }

        // Set outer_var_names for nested blocks within this def
        let old_outer = std::mem::replace(&mut self.outer_var_names, def_var_names);

        // Always visit body for nested scopes (blocks, lambdas, nested defs)
        if let Some(body) = def_node.body() {
            self.visit(&body);
        }

        self.outer_var_names = old_outer;
    }

    fn check_block(&mut self, block_node: &ruby_prism::BlockNode<'_>) {
        let mut underscore_vars: Vec<UnderscoreVar> = Vec::new();

        // Collect block parameters
        if let Some(params) = block_node.parameters() {
            if let Some(params_node) = params.as_block_parameters_node() {
                if let Some(inner_params) = params_node.parameters() {
                    collect_underscore_params(&inner_params, &mut underscore_vars, true);
                }
            }
        }

        // Also collect local variable writes directly in the block body.
        // WriteCollector stops at nested blocks, so this only gets writes at
        // this block level. Filter out reassignments of enclosing-scope vars.
        if let Some(body) = block_node.body() {
            let mut write_collector = WriteCollector { writes: Vec::new() };
            write_collector.visit(&body);
            for write in write_collector.writes {
                if !self.outer_var_names.contains(&write.name) {
                    underscore_vars.push(write);
                }
            }
        }

        // Build block-level var names for nested scope tracking
        let block_var_names: HashSet<String> =
            underscore_vars.iter().map(|v| v.name.clone()).collect();

        if !underscore_vars.is_empty() {
            // Collect reads in body, respecting nested block scoping
            let mut reads = HashSet::new();
            if let Some(body) = block_node.body() {
                collect_reads_scope_aware(&body, &mut reads);
            }

            // Filter out allowed keyword block arguments
            if self.allow_keyword_block_args {
                underscore_vars.retain(|v| !v.is_keyword_block_arg);
            }

            self.emit_diagnostics(&underscore_vars, &reads, false);
        }

        // Update outer_var_names for nested blocks: include both enclosing + this block's vars
        let old_outer = self.outer_var_names.clone();
        self.outer_var_names.extend(block_var_names);

        // Visit body for nested scopes
        if let Some(body) = block_node.body() {
            self.visit(&body);
        }

        self.outer_var_names = old_outer;
    }

    fn check_lambda(&mut self, lambda_node: &ruby_prism::LambdaNode<'_>) {
        let mut underscore_vars: Vec<UnderscoreVar> = Vec::new();

        if let Some(params) = lambda_node.parameters() {
            if let Some(params_node) = params.as_block_parameters_node() {
                if let Some(inner_params) = params_node.parameters() {
                    collect_underscore_params(&inner_params, &mut underscore_vars, true);
                }
            }
        }

        // Lambdas create new scopes, so collect local variable writes here.
        // Filter out reassignments of enclosing-scope vars (same as check_block).
        if let Some(body) = lambda_node.body() {
            let mut write_collector = WriteCollector { writes: Vec::new() };
            write_collector.visit(&body);
            for write in write_collector.writes {
                if !self.outer_var_names.contains(&write.name) {
                    underscore_vars.push(write);
                }
            }
        }

        let lambda_var_names: HashSet<String> =
            underscore_vars.iter().map(|v| v.name.clone()).collect();

        if !underscore_vars.is_empty() {
            // Collect reads in body
            let mut reads = HashSet::new();
            if let Some(body) = lambda_node.body() {
                collect_reads_scope_aware(&body, &mut reads);
            }

            // Filter out allowed keyword block arguments (lambdas are block-like)
            if self.allow_keyword_block_args {
                underscore_vars.retain(|v| !v.is_keyword_block_arg);
            }

            self.emit_diagnostics(&underscore_vars, &reads, false);
        }

        // Lambdas create new scopes — reset outer_var_names
        let old_outer = std::mem::replace(&mut self.outer_var_names, lambda_var_names);

        // Visit body for nested scopes
        if let Some(body) = lambda_node.body() {
            self.visit(&body);
        }

        self.outer_var_names = old_outer;
    }

    /// Check a scope body for local variable writes: top-level, class, module.
    fn check_scope_body(&mut self, node: &ruby_prism::Node<'_>) {
        // Collect local variable writes at this scope level (stops at blocks)
        let mut underscore_vars: Vec<UnderscoreVar> = Vec::new();
        let mut write_collector = WriteCollector { writes: Vec::new() };
        write_collector.visit(node);
        underscore_vars.extend(write_collector.writes);

        if underscore_vars.is_empty() {
            return;
        }

        // Set outer var names for nested blocks
        let scope_var_names: HashSet<String> =
            underscore_vars.iter().map(|v| v.name.clone()).collect();
        self.outer_var_names.extend(scope_var_names);

        // Collect reads at this scope level, respecting scoping
        let mut reads = HashSet::new();
        collect_reads_scope_aware(node, &mut reads);

        self.emit_diagnostics(&underscore_vars, &reads, false);
    }

    fn emit_diagnostics(
        &mut self,
        underscore_vars: &[UnderscoreVar],
        reads: &HashSet<String>,
        has_forwarding: bool,
    ) {
        // Deduplicate: only flag the first occurrence of each variable name
        let mut seen_names: HashSet<&str> = HashSet::new();

        for var in underscore_vars {
            if !seen_names.insert(&var.name) {
                continue;
            }

            // If there's bare super/binding and the var is NOT explicitly read,
            // don't flag it (it's implicitly forwarded)
            if has_forwarding && !reads.contains(var.name.as_str()) {
                continue;
            }

            if reads.contains(var.name.as_str()) {
                let (line, col) = self.source.offset_to_line_col(var.offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    col,
                    "Do not use prefix `_` for a variable that is used.".to_string(),
                ));
            }
        }
    }
}

struct UnderscoreVar {
    name: String,
    offset: usize,
    is_keyword_block_arg: bool,
}

/// Check if a name is an underscore-prefixed variable that should be unused.
/// Matches RuboCop's `should_be_unused?` which returns true for any name
/// starting with `_`, including bare `_`.
fn should_be_unused(name: &str) -> bool {
    name.starts_with('_')
}

fn collect_underscore_params(
    params: &ruby_prism::ParametersNode<'_>,
    out: &mut Vec<UnderscoreVar>,
    is_block: bool,
) {
    for param in params.requireds().iter() {
        if let Some(req) = param.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            if should_be_unused(name) {
                out.push(UnderscoreVar {
                    name: name.to_string(),
                    offset: req.location().start_offset(),
                    is_keyword_block_arg: false,
                });
            }
        }
        // Handle destructured parameters (MultiTargetNode)
        if let Some(mt) = param.as_multi_target_node() {
            collect_underscore_multi_target(&mt, out);
        }
    }

    for param in params.optionals().iter() {
        if let Some(opt) = param.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice()).unwrap_or("");
            if should_be_unused(name) {
                out.push(UnderscoreVar {
                    name: name.to_string(),
                    offset: opt.name_loc().start_offset(),
                    is_keyword_block_arg: false,
                });
            }
        }
    }

    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name_const) = rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice()).unwrap_or("");
                if should_be_unused(name) {
                    if let Some(name_loc) = rest_param.name_loc() {
                        out.push(UnderscoreVar {
                            name: name.to_string(),
                            offset: name_loc.start_offset(),
                            is_keyword_block_arg: false,
                        });
                    }
                }
            }
        }
    }

    // Keyword parameters (required and optional)
    for param in params.keywords().iter() {
        if let Some(req_kw) = param.as_required_keyword_parameter_node() {
            let name = std::str::from_utf8(req_kw.name().as_slice()).unwrap_or("");
            // Keyword param names include trailing colon in some representations
            let clean_name = name.trim_end_matches(':');
            if should_be_unused(clean_name) {
                out.push(UnderscoreVar {
                    name: clean_name.to_string(),
                    offset: req_kw.name_loc().start_offset(),
                    is_keyword_block_arg: is_block,
                });
            }
        }
        if let Some(opt_kw) = param.as_optional_keyword_parameter_node() {
            let name = std::str::from_utf8(opt_kw.name().as_slice()).unwrap_or("");
            let clean_name = name.trim_end_matches(':');
            if should_be_unused(clean_name) {
                out.push(UnderscoreVar {
                    name: clean_name.to_string(),
                    offset: opt_kw.name_loc().start_offset(),
                    is_keyword_block_arg: is_block,
                });
            }
        }
    }

    // Keyword rest parameter (**_opts)
    if let Some(kw_rest) = params.keyword_rest() {
        if let Some(kw_rest_param) = kw_rest.as_keyword_rest_parameter_node() {
            if let Some(name_const) = kw_rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice()).unwrap_or("");
                if should_be_unused(name) {
                    if let Some(name_loc) = kw_rest_param.name_loc() {
                        out.push(UnderscoreVar {
                            name: name.to_string(),
                            offset: name_loc.start_offset(),
                            is_keyword_block_arg: false,
                        });
                    }
                }
            }
        }
    }

    // Block parameter (&_block)
    if let Some(block_param) = params.block() {
        if let Some(name_const) = block_param.name() {
            let name = std::str::from_utf8(name_const.as_slice()).unwrap_or("");
            if should_be_unused(name) {
                if let Some(name_loc) = block_param.name_loc() {
                    out.push(UnderscoreVar {
                        name: name.to_string(),
                        offset: name_loc.start_offset(),
                        is_keyword_block_arg: false,
                    });
                }
            }
        }
    }
}

/// Collect underscore-prefixed names from a destructured parameter (MultiTargetNode).
fn collect_underscore_multi_target(
    mt: &ruby_prism::MultiTargetNode<'_>,
    out: &mut Vec<UnderscoreVar>,
) {
    for target in mt.lefts().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            if should_be_unused(name) {
                out.push(UnderscoreVar {
                    name: name.to_string(),
                    offset: req.location().start_offset(),
                    is_keyword_block_arg: false,
                });
            }
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_underscore_multi_target(&inner, out);
        }
    }
    if let Some(rest) = mt.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(req) = expr.as_required_parameter_node() {
                    let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
                    if should_be_unused(name) {
                        out.push(UnderscoreVar {
                            name: name.to_string(),
                            offset: req.location().start_offset(),
                            is_keyword_block_arg: false,
                        });
                    }
                }
            }
        }
    }
    for target in mt.rights().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            if should_be_unused(name) {
                out.push(UnderscoreVar {
                    name: name.to_string(),
                    offset: req.location().start_offset(),
                    is_keyword_block_arg: false,
                });
            }
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_underscore_multi_target(&inner, out);
        }
    }
}

/// Collects underscore-prefixed local variable writes.
/// Stops at blocks, defs, classes, modules, and lambdas (each handles its own).
struct WriteCollector {
    writes: Vec<UnderscoreVar>,
}

impl<'pr> Visit<'pr> for WriteCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if should_be_unused(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.name_loc().start_offset(),
                is_keyword_block_arg: false,
            });
        }
        // Visit the value expression
        self.visit(&node.value());
    }

    /// Handle LocalVariableTargetNode: used in multi-assignment, for-loops,
    /// pattern matching, and named capture regex.
    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if should_be_unused(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.location().start_offset(),
                is_keyword_block_arg: false,
            });
        }
    }

    /// Handle MatchWriteNode: named capture regex `/(?<_name>\w+)/ =~ str`.
    fn visit_match_write_node(&mut self, node: &ruby_prism::MatchWriteNode<'pr>) {
        for target in node.targets().iter() {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let name = std::str::from_utf8(target_node.name().as_slice()).unwrap_or("");
                if should_be_unused(name) {
                    // Point at the regex (first child of the call), matching RuboCop
                    let call = node.call();
                    let offset = if let Some(receiver) = call.receiver() {
                        receiver.location().start_offset()
                    } else {
                        target_node.location().start_offset()
                    };
                    self.writes.push(UnderscoreVar {
                        name: name.to_string(),
                        offset,
                        is_keyword_block_arg: false,
                    });
                }
            }
        }
        // Don't visit children (we already handled targets)
    }

    /// Handle operator writes: _x += 1, _x ||= 1, _x &&= 1
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if should_be_unused(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.name_loc().start_offset(),
                is_keyword_block_arg: false,
            });
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if should_be_unused(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.name_loc().start_offset(),
                is_keyword_block_arg: false,
            });
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if should_be_unused(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.name_loc().start_offset(),
                is_keyword_block_arg: false,
            });
        }
        self.visit(&node.value());
    }

    // Stop at all scope boundaries — each scope handles its own writes
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

/// Collects local variable reads while respecting block/lambda parameter scoping.
fn collect_reads_scope_aware(node: &ruby_prism::Node<'_>, reads: &mut HashSet<String>) {
    let mut collector = ScopeAwareReadCollector {
        reads,
        shadowed: HashSet::new(),
    };
    collector.visit(node);
}

/// Collect local variable reads from parameter default values.
/// E.g., `def foo(_locale = nil, locale: _locale)` — the `_locale` in the
/// keyword default is a read.
fn collect_reads_from_param_defaults(
    params: &ruby_prism::ParametersNode<'_>,
    reads: &mut HashSet<String>,
) {
    // Optional positional params: their default values may read other params
    for param in params.optionals().iter() {
        if let Some(opt) = param.as_optional_parameter_node() {
            collect_reads_scope_aware(&opt.value(), reads);
        }
    }
    // Optional keyword params: their default values may read other params
    for param in params.keywords().iter() {
        if let Some(opt_kw) = param.as_optional_keyword_parameter_node() {
            collect_reads_scope_aware(&opt_kw.value(), reads);
        }
    }
}

struct ScopeAwareReadCollector<'a> {
    reads: &'a mut HashSet<String>,
    shadowed: HashSet<String>,
}

impl<'pr> Visit<'pr> for ScopeAwareReadCollector<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        // Only record reads for names not shadowed by an inner block param
        if !self.shadowed.contains(name) {
            self.reads.insert(name.to_string());
        }
    }

    /// Operator writes (_x += 1, _x ||= 1, _x &&= 1) implicitly read the variable.
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if !self.shadowed.contains(name) {
            self.reads.insert(name.to_string());
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if !self.shadowed.contains(name) {
            self.reads.insert(name.to_string());
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if !self.shadowed.contains(name) {
            self.reads.insert(name.to_string());
        }
        self.visit(&node.value());
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Collect param names that shadow outer variables
        let block_params = collect_block_param_names(node);

        // Save old shadowed set, add block params
        let old_shadowed = self.shadowed.clone();
        self.shadowed.extend(block_params);

        // Visit the block body with updated shadow set
        if let Some(body) = node.body() {
            self.visit(&body);
        }

        // Restore old shadowed set
        self.shadowed = old_shadowed;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Cross into lambdas for read collection (like blocks), since RuboCop's
        // VariableForce DOES attribute reads inside lambdas to the enclosing scope.
        // Handle parameter shadowing just like blocks.
        let lambda_params = collect_lambda_param_names(node);

        let old_shadowed = self.shadowed.clone();
        self.shadowed.extend(lambda_params);

        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.shadowed = old_shadowed;
    }

    // Don't cross into nested defs/modules — they have their own scope.
    // Class superclasses are evaluated in the outer scope, so visit them.
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(superclass) = node.superclass() {
            self.visit(&superclass);
        }
    }
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

fn collect_block_param_names(block_node: &ruby_prism::BlockNode<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(params) = block_node.parameters() {
        if let Some(params_node) = params.as_block_parameters_node() {
            if let Some(inner) = params_node.parameters() {
                collect_all_param_names(&inner, &mut names);
            }
        }
    }
    names
}

fn collect_lambda_param_names(lambda_node: &ruby_prism::LambdaNode<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(params) = lambda_node.parameters() {
        if let Some(params_node) = params.as_block_parameters_node() {
            if let Some(inner) = params_node.parameters() {
                collect_all_param_names(&inner, &mut names);
            }
        }
    }
    names
}

fn collect_all_param_names(params: &ruby_prism::ParametersNode<'_>, names: &mut HashSet<String>) {
    for param in params.requireds().iter() {
        if let Some(req) = param.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            names.insert(name.to_string());
        }
        // Handle destructured parameters
        if let Some(mt) = param.as_multi_target_node() {
            collect_multi_target_names(&mt, names);
        }
    }
    for param in params.optionals().iter() {
        if let Some(opt) = param.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice()).unwrap_or("");
            names.insert(name.to_string());
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name_const) = rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice()).unwrap_or("");
                names.insert(name.to_string());
            }
        }
    }
    for param in params.keywords().iter() {
        if let Some(req_kw) = param.as_required_keyword_parameter_node() {
            let name = std::str::from_utf8(req_kw.name().as_slice()).unwrap_or("");
            names.insert(name.trim_end_matches(':').to_string());
        }
        if let Some(opt_kw) = param.as_optional_keyword_parameter_node() {
            let name = std::str::from_utf8(opt_kw.name().as_slice()).unwrap_or("");
            names.insert(name.trim_end_matches(':').to_string());
        }
    }
    if let Some(kw_rest) = params.keyword_rest() {
        if let Some(kw_rest_param) = kw_rest.as_keyword_rest_parameter_node() {
            if let Some(name_const) = kw_rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice()).unwrap_or("");
                names.insert(name.to_string());
            }
        }
    }
    if let Some(block_param) = params.block() {
        if let Some(name_const) = block_param.name() {
            let name = std::str::from_utf8(name_const.as_slice()).unwrap_or("");
            names.insert(name.to_string());
        }
    }
}

/// Collect all names from a destructured MultiTargetNode.
fn collect_multi_target_names(mt: &ruby_prism::MultiTargetNode<'_>, names: &mut HashSet<String>) {
    for target in mt.lefts().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            names.insert(name.to_string());
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_names(&inner, names);
        }
    }
    if let Some(rest) = mt.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(req) = expr.as_required_parameter_node() {
                    let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
                    names.insert(name.to_string());
                }
            }
        }
    }
    for target in mt.rights().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            names.insert(name.to_string());
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_names(&inner, names);
        }
    }
}

/// Checks for bare `super` (ForwardingSuperNode) or `binding` calls without args.
fn check_forwarding(node: &ruby_prism::Node<'_>) -> bool {
    let mut checker = ForwardingChecker {
        has_forwarding: false,
    };
    checker.visit(node);
    checker.has_forwarding
}

struct ForwardingChecker {
    has_forwarding: bool,
}

impl<'pr> Visit<'pr> for ForwardingChecker {
    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        self.has_forwarding = true;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"binding"
            && node.receiver().is_none()
            && node.arguments().is_none()
        {
            self.has_forwarding = true;
        }
        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnderscorePrefixedVariableName,
        "cops/lint/underscore_prefixed_variable_name"
    );

    #[test]
    fn test_block_param_used_in_method_call() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo\n  proxy = @proxies.detect do |_proxy|\n    _proxy.params.has_key?(param_key)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _proxy, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_local_var_in_block_used() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def foo\n  items.each do |item|\n    _val = item.process\n    puts _val\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _val, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_bare_underscore_used() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"items.each { |_| _ }\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for bare _, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_no_double_report_outer_reassignment() {
        let cop = UnderscorePrefixedVariableName;
        // _finder is first assigned outside block, then reassigned inside.
        // Should only report once (at first assignment), not twice.
        let source = b"def foo\n  _finder = Model.all\n  items.each do |col|\n    _finder = _finder.where(col => val)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (at first assignment only), got: {:?}",
            diags
        );
    }

    #[test]
    fn test_var_in_nested_block() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"def test_data\n  assert_raise(Error) do\n    _data = data.dup\n    _data[_data.size - 4] = 'X'\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _data, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_param_default_value_read() {
        let cop = UnderscorePrefixedVariableName;
        let source =
            b"def exists?(key, _locale = nil, locale: _locale)\n  locale || config.locale\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _locale, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_let_block_var_used() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"describe 'test' do\n  let(:record) do\n    _p = Record.last\n    _p.name = 'test'\n    _p.save\n    _p\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _p in let block, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_times_block_var_used() {
        let cop = UnderscorePrefixedVariableName;
        let source =
            b"3.times do |i|\n  _user = User.first\n  _user.name = 'test'\n  _user.save!\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _user in times block, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_included_block_var_used() {
        let cop = UnderscorePrefixedVariableName;
        // Module included block pattern (discourse)
        let source = b"module HasSearchData\n  included do\n    _name = self.name.sub('SearchData', '').underscore\n    self.primary_key = _name\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense for _name in included block, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_var_only_assigned_in_block_no_offense() {
        // FP test: variable assigned in a block but never read
        let cop = UnderscorePrefixedVariableName;
        let source = b"describe 'test' do\n  it 'does something' do\n    _unused = create(:record)\n    expect(1).to eq(1)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            0,
            "Expected 0 offenses for unused _unused, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_multiple_blocks_same_var_name() {
        // Each block should be flagged independently (RuboCop treats each block as a scope)
        let cop = UnderscorePrefixedVariableName;
        let source = b"def test_data\n  assert_raise(Error) do\n    _data = data.dup\n    _data[_data.size - 4] = 'X'\n  end\n\n  assert_raise(Error) do\n    _data = data.dup\n    _data[_data.size - 5] = 'X'\n  end\n\n  assert_raise(Error) do\n    _data = data.dup\n    _data = _data.slice!(0, _data.size - 1)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert_eq!(
            diags.len(),
            3,
            "Expected 3 offenses (one per block), got: {:?}",
            diags
        );
    }

    #[test]
    fn test_different_blocks_same_var_name_no_cross_leak() {
        // FP test: two different it blocks with same variable name, only one reads it
        let cop = UnderscorePrefixedVariableName;
        let source = b"describe 'test' do\n  it 'first' do\n    _x = create(:record)\n    expect(1).to eq(1)\n  end\n\n  it 'second' do\n    _x = create(:record)\n    puts _x\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        // Only the second block should have an offense
        assert_eq!(
            diags.len(),
            1,
            "Expected 1 offense (only in second block), got: {:?}",
            diags
        );
    }

    #[test]
    fn test_destructured_block_param() {
        let cop = UnderscorePrefixedVariableName;
        let source = b"children.each { |(_page, _children)| add(_page, _children) }\n";
        let diags = crate::testutil::run_cop_full(&cop, source);
        assert!(
            diags.len() >= 1,
            "Expected at least 1 offense for destructured params, got: {:?}",
            diags
        );
    }
}
