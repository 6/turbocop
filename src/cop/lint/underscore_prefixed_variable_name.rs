use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for underscore-prefixed variables that are actually used.
///
/// RuboCop uses VariableForce to track variable scoping across all scope types
/// (def, block, lambda, top-level). This implementation replicates that behavior
/// by visiting each scope type and checking parameters and local variable writes
/// within that scope for reads.
///
/// Key behaviors matching RuboCop:
/// - Flags underscore-prefixed method params, block params, and local variable
///   assignments that are subsequently read in the same scope.
/// - Respects block parameter shadowing: if a block redefines a param with the
///   same name, reads inside the block are attributed to the block param, not
///   the outer scope variable.
/// - Handles `AllowKeywordBlockArguments` config to skip keyword block params.
/// - Skips variables implicitly forwarded via bare `super` or `binding`.
/// - Handles top-level scope (variables outside any def/block).
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
/// Historical FP root cause: The ReadCollector traversed into blocks without
/// respecting scope boundaries, causing reads of block-local params to be
/// misattributed to outer scope variables with the same name. Also, block and
/// lambda scopes were not checked at all, causing FNs.
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
        };
        // Check top-level scope first
        visitor.check_scope_body(&parse_result.node(), None, false);
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
}

impl ScopeFinder<'_, '_> {
    fn check_def(&mut self, def_node: &ruby_prism::DefNode<'_>) {
        let mut underscore_vars: Vec<UnderscoreVar> = Vec::new();

        if let Some(params) = def_node.parameters() {
            collect_underscore_params(&params, &mut underscore_vars, false);
        }

        // Collect underscore-prefixed local variable writes in the body
        if let Some(body) = def_node.body() {
            let mut write_collector = WriteCollector { writes: Vec::new() };
            write_collector.visit(&body);
            underscore_vars.extend(write_collector.writes);
        }

        if underscore_vars.is_empty() {
            return;
        }

        // Collect all local variable reads in the body, respecting block scoping
        let mut reads = HashSet::new();
        if let Some(body) = def_node.body() {
            collect_reads_scope_aware(&body, &mut reads);
        }

        // Check for implicit forwarding (bare `super` or `binding`)
        let has_forwarding = if let Some(body) = def_node.body() {
            check_forwarding(&body)
        } else {
            false
        };

        self.emit_diagnostics(&underscore_vars, &reads, has_forwarding);

        // Visit body for nested scopes (blocks, lambdas, nested defs)
        if let Some(body) = def_node.body() {
            self.visit(&body);
        }
    }

    fn check_block(&mut self, block_node: &ruby_prism::BlockNode<'_>) {
        let mut underscore_vars: Vec<UnderscoreVar> = Vec::new();

        if let Some(params) = block_node.parameters() {
            if let Some(params_node) = params.as_block_parameters_node() {
                if let Some(inner_params) = params_node.parameters() {
                    collect_underscore_params(&inner_params, &mut underscore_vars, true);
                }
            }
        }

        // Collect local variable writes in the block body
        if let Some(body) = block_node.body() {
            let mut write_collector = WriteCollector { writes: Vec::new() };
            write_collector.visit(&body);
            underscore_vars.extend(write_collector.writes);
        }

        if underscore_vars.is_empty() {
            // Still need to visit body for nested scopes
            if let Some(body) = block_node.body() {
                self.visit(&body);
            }
            return;
        }

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

        // Visit body for nested scopes
        if let Some(body) = block_node.body() {
            self.visit(&body);
        }
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

        // Collect local variable writes in the lambda body
        if let Some(body) = lambda_node.body() {
            let mut write_collector = WriteCollector { writes: Vec::new() };
            write_collector.visit(&body);
            underscore_vars.extend(write_collector.writes);
        }

        if underscore_vars.is_empty() {
            if let Some(body) = lambda_node.body() {
                self.visit(&body);
            }
            return;
        }

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

        // Visit body for nested scopes
        if let Some(body) = lambda_node.body() {
            self.visit(&body);
        }
    }

    /// Check top-level scope: variables outside any def/block/lambda.
    fn check_scope_body(
        &mut self,
        node: &ruby_prism::Node<'_>,
        _params: Option<&ruby_prism::ParametersNode<'_>>,
        _is_block: bool,
    ) {
        // Collect top-level local variable writes
        let mut underscore_vars: Vec<UnderscoreVar> = Vec::new();
        let mut write_collector = WriteCollector { writes: Vec::new() };
        write_collector.visit(node);
        underscore_vars.extend(write_collector.writes);

        if underscore_vars.is_empty() {
            return;
        }

        // Collect reads at top level, respecting scoping
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
/// starting with `_`. We exclude bare `_` to avoid FPs on the common Ruby
/// convention of using `_` as a throwaway variable.
fn is_underscore_prefixed(name: &str) -> bool {
    name.starts_with('_') && name != "_"
}

fn collect_underscore_params(
    params: &ruby_prism::ParametersNode<'_>,
    out: &mut Vec<UnderscoreVar>,
    is_block: bool,
) {
    for param in params.requireds().iter() {
        if let Some(req) = param.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            if is_underscore_prefixed(name) {
                out.push(UnderscoreVar {
                    name: name.to_string(),
                    offset: req.location().start_offset(),
                    is_keyword_block_arg: false,
                });
            }
        }
    }

    for param in params.optionals().iter() {
        if let Some(opt) = param.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice()).unwrap_or("");
            if is_underscore_prefixed(name) {
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
                if is_underscore_prefixed(name) {
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
            if is_underscore_prefixed(clean_name) {
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
            if is_underscore_prefixed(clean_name) {
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
                if is_underscore_prefixed(name) {
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
            if is_underscore_prefixed(name) {
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

/// Collects underscore-prefixed local variable writes.
struct WriteCollector {
    writes: Vec<UnderscoreVar>,
}

impl<'pr> Visit<'pr> for WriteCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if is_underscore_prefixed(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.name_loc().start_offset(),
                is_keyword_block_arg: false,
            });
        }
        // Visit the value expression (but not for collecting writes in nested scopes)
        self.visit(&node.value());
    }

    /// Handle LocalVariableTargetNode: used in multi-assignment, for-loops,
    /// pattern matching, and named capture regex.
    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if is_underscore_prefixed(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.location().start_offset(),
                is_keyword_block_arg: false,
            });
        }
    }

    /// Handle MatchWriteNode: named capture regex `/(?<_name>\w+)/ =~ str`.
    /// The targets contain LocalVariableTargetNode instances which will be
    /// visited by visit_local_variable_target_node above. But we need to
    /// override the offset to point at the regex (matching RuboCop behavior
    /// which uses `node.children.first.source_range` for match_with_lvasgn).
    fn visit_match_write_node(&mut self, node: &ruby_prism::MatchWriteNode<'pr>) {
        // Check each target for underscore-prefixed names
        for target in node.targets().iter() {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let name = std::str::from_utf8(target_node.name().as_slice()).unwrap_or("");
                if is_underscore_prefixed(name) {
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
    /// These are writes but also implicitly read the variable.
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if is_underscore_prefixed(name) {
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
        if is_underscore_prefixed(name) {
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
        if is_underscore_prefixed(name) {
            self.writes.push(UnderscoreVar {
                name: name.to_string(),
                offset: node.name_loc().start_offset(),
                is_keyword_block_arg: false,
            });
        }
        self.visit(&node.value());
    }

    // Don't cross into nested defs/classes/modules/blocks/lambdas
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

/// Collects local variable reads while respecting block/lambda parameter scoping.
///
/// When a block or lambda declares a parameter with the same name as an outer
/// variable, reads of that name inside the block refer to the block parameter,
/// not the outer variable. This collector tracks such shadowed names and excludes
/// them from the outer scope's read set.
fn collect_reads_scope_aware(node: &ruby_prism::Node<'_>, reads: &mut HashSet<String>) {
    let mut collector = ScopeAwareReadCollector {
        reads,
        shadowed: HashSet::new(),
    };
    collector.visit(node);
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
        let lambda_params = collect_lambda_param_names(node);

        let old_shadowed = self.shadowed.clone();
        self.shadowed.extend(lambda_params);

        if let Some(body) = node.body() {
            self.visit(&body);
        }

        self.shadowed = old_shadowed;
    }

    // Don't cross into nested defs/classes/modules — they have their own scope
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
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
}
