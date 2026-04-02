use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for unused block arguments.
///
/// Root causes of historical FPs/FNs (corpus: 144 FP, 937 FN at 89.4% match):
///
/// FN root causes:
/// - Lambda nodes (`-> (x) { ... }`) were not handled; only BlockNode was checked.
/// - Rest/splat parameters (`*args`) were not collected.
/// - Block-local variables (`|x; local|`) were not handled.
/// - `LocalVariableTargetNode` (multi-assign target) was incorrectly treated as
///   "referenced", masking cases where a param was only written but never read.
///
/// FP root causes:
/// - Zero-argument `binding` calls in the block body should suppress all offenses
///   even when they have a receiver (RuboCop's VariableForce treats all accessible
///   args as referenced for any `binding` send without arguments, since it may
///   capture the current local scope).
///
/// Fix: Rewrote to use `check_source` with a visitor that handles both BlockNode
/// and LambdaNode, collects rest params and block-local variables, detects
/// scope-capturing `binding` calls, and only counts actual reads (not write
/// targets) as references.
///
/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=27, FN=5393.
///
/// FN=5393: The `VarRefFinder` used simple name-matching — it collected all
/// `LocalVariableReadNode` names in the block body without considering scope.
/// When a nested block redeclares a parameter with the same name (variable
/// shadowing), reads of that name inside the nested scope were incorrectly
/// counted as references to the outer parameter. Fixed by tracking shadowed
/// names in `VarRefFinder`: when entering a nested block/lambda, any params
/// that shadow outer names are pushed to a `shadowed` list, and reads of
/// those names inside the nested scope are excluded from collection.
///
/// FP=27→24: Operator-assign nodes (`x += 1`, `x ||= val`, `x &&= val`)
/// were not counted as references. Prism represents these as
/// `LocalVariableOperatorWriteNode`, `LocalVariableOrWriteNode`, and
/// `LocalVariableAndWriteNode` — none of which contain a child
/// `LocalVariableReadNode`, so the implicit read was missed. Fixed by
/// adding visit handlers for all three operator-write node types in
/// `VarRefFinder`. Remaining FPs may be from VariableForce sophistication
/// gaps (e.g., scope tracking differences).
///
/// ## Corpus investigation (2026-03-16)
///
/// Corpus oracle reported FP=13, FN=5320 (46.7% match).
///
/// FN=5320 root cause: `BlockVisitor` stopped at `def`/`class`/`module`
/// nodes (line `fn visit_def_node(&mut self, _node) {}`), meaning blocks
/// inside method bodies — which is essentially all real-world Ruby code —
/// were never visited. Fixed by making `BlockVisitor` recurse into
/// `def`/`class`/`module`/`singleton_class` bodies. The scope boundary
/// for variable references is correctly handled by `VarRefFinder`, not
/// `BlockVisitor`, so this is safe.
///
/// FP=13 root cause: `VarRefFinder` didn't recognize `def o.method` as a
/// use of block argument `o`. When a block argument is used as the receiver
/// of a singleton method definition (`def o.to_str; ...; end`), it IS a
/// reference to that variable, but `DefNode` was being skipped entirely by
/// `VarRefFinder`. Fixed by checking the receiver of `DefNode` — if it's a
/// `LocalVariableReadNode`, the name is counted as referenced.
///
/// After fix: FP=0, FN=114 (98.9% match). Remaining 114 FN were from three
/// missing parameter types:
///
/// ## Corpus investigation (2026-03-18)
///
/// FN=114 root causes:
/// - Destructured block params `|(a, b)|` represented as `MultiTargetNode`
///   in `requireds()` / `posts()` were not traversed (~42 FN).
/// - Block-pass params `&block` (`BlockParameterNode`) were not collected (~23 FN).
/// - Keyword rest params `**opts` (`KeywordRestParameterNode`) were not collected (~30 FN).
///   Fixed by adding `collect_multi_target_params` for destructured params, and
///   handling `params.keyword_rest()` and `params.block()` in both param collection
///   and shadowing name collection.
///
/// ## Corpus investigation (2026-03-19)
///
/// FN=3: Lambdas used as default parameter values in method definitions
/// (e.g., `def foo(scope: ->(row) { true })`) were not visited by
/// `BlockVisitor` because `visit_def_node` only recursed into the method
/// body, not the parameters. Lambda nodes in optional parameter defaults
/// are children of `OptionalKeywordParameterNode.value()` or
/// `OptionalParameterNode.value()` under `ParametersNode`. Fixed by
/// adding `self.visit_parameters_node(&params)` in `visit_def_node`.
///
/// ## Corpus investigation (2026-04-02)
///
/// FP=1 root cause: `VarRefFinder` only treated bare `binding` as scope-capturing,
/// but RuboCop's `VariableForce` suppresses unused-argument offenses for any
/// zero-argument `binding` send, including receiver-qualified calls like
/// `tp.binding.local_variable_get(name)`. Fixed by matching zero-arg `binding`
/// calls regardless of receiver, while still leaving `binding(:arg)` as an offense.
pub struct UnusedBlockArgument;

impl Cop for UnusedBlockArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedBlockArgument"
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
        let ignore_empty = config.get_bool("IgnoreEmptyBlocks", true);
        let allow_unused_keyword = config.get_bool("AllowUnusedKeywordArguments", false);

        let mut visitor = BlockVisitor {
            cop: self,
            source,
            ignore_empty,
            allow_unused_keyword,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct BlockVisitor<'a, 'src> {
    cop: &'a UnusedBlockArgument,
    source: &'src SourceFile,
    ignore_empty: bool,
    allow_unused_keyword: bool,
    diagnostics: Vec<Diagnostic>,
}

/// Info about a parameter that may be unused.
struct ParamInfo {
    name: Vec<u8>,
    offset: usize,
    is_keyword: bool,
    is_block_local: bool,
}

impl<'pr> Visit<'pr> for BlockVisitor<'_, '_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.check_block_or_lambda(node.body(), node.parameters(), false);

        // Recurse into the body for nested blocks/lambdas
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.check_block_or_lambda(node.body(), node.parameters(), true);

        // Recurse into the body for nested blocks/lambdas
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    // Recurse into def/class/module: BlockVisitor needs to find blocks
    // at all nesting levels. The scope boundary for variable references
    // is handled by VarRefFinder, not here.
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Visit parameters to find lambdas in default values
        // (e.g., `def foo(callback: ->(row) { true })`)
        if let Some(params) = node.parameters() {
            self.visit_parameters_node(&params);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

impl BlockVisitor<'_, '_> {
    fn check_block_or_lambda(
        &mut self,
        body: Option<ruby_prism::Node<'_>>,
        parameters: Option<ruby_prism::Node<'_>>,
        _is_lambda: bool,
    ) {
        // Check body emptiness
        let body = match body {
            Some(b) => b,
            None => {
                if self.ignore_empty {
                    return;
                }
                match parameters {
                    Some(_) => {
                        self.check_params_with_body(parameters, None);
                        return;
                    }
                    None => return,
                }
            }
        };

        self.check_params_with_body(parameters, Some(body));
    }

    fn check_params_with_body(
        &mut self,
        parameters: Option<ruby_prism::Node<'_>>,
        body: Option<ruby_prism::Node<'_>>,
    ) {
        let block_params_node = match parameters {
            Some(p) => match p.as_block_parameters_node() {
                Some(bp) => bp,
                None => return,
            },
            None => return,
        };

        let mut param_info: Vec<ParamInfo> = Vec::new();

        // Collect regular parameters
        if let Some(params) = block_params_node.parameters() {
            // Required params (including destructured MultiTargetNode)
            for req in params.requireds().iter() {
                if let Some(rp) = req.as_required_parameter_node() {
                    param_info.push(ParamInfo {
                        name: rp.name().as_slice().to_vec(),
                        offset: rp.location().start_offset(),
                        is_keyword: false,
                        is_block_local: false,
                    });
                } else if let Some(mt) = req.as_multi_target_node() {
                    collect_multi_target_params(&mt, &mut param_info);
                }
            }

            // Optional params
            for opt in params.optionals().iter() {
                if let Some(op) = opt.as_optional_parameter_node() {
                    param_info.push(ParamInfo {
                        name: op.name().as_slice().to_vec(),
                        offset: op.location().start_offset(),
                        is_keyword: false,
                        is_block_local: false,
                    });
                }
            }

            // Rest/splat params (*args)
            if let Some(rest) = params.rest() {
                if let Some(rp) = rest.as_rest_parameter_node() {
                    if let Some(name) = rp.name() {
                        param_info.push(ParamInfo {
                            name: name.as_slice().to_vec(),
                            offset: rp
                                .name_loc()
                                .map_or(rp.location().start_offset(), |loc| loc.start_offset()),
                            is_keyword: false,
                            is_block_local: false,
                        });
                    }
                }
            }

            // Post params (after rest, including destructured)
            for post in params.posts().iter() {
                if let Some(rp) = post.as_required_parameter_node() {
                    param_info.push(ParamInfo {
                        name: rp.name().as_slice().to_vec(),
                        offset: rp.location().start_offset(),
                        is_keyword: false,
                        is_block_local: false,
                    });
                } else if let Some(mt) = post.as_multi_target_node() {
                    collect_multi_target_params(&mt, &mut param_info);
                }
            }

            // Keyword params
            if !self.allow_unused_keyword {
                for kw in params.keywords().iter() {
                    if let Some(kp) = kw.as_required_keyword_parameter_node() {
                        param_info.push(ParamInfo {
                            name: kp.name().as_slice().to_vec(),
                            offset: kp.location().start_offset(),
                            is_keyword: true,
                            is_block_local: false,
                        });
                    } else if let Some(kp) = kw.as_optional_keyword_parameter_node() {
                        param_info.push(ParamInfo {
                            name: kp.name().as_slice().to_vec(),
                            offset: kp.location().start_offset(),
                            is_keyword: true,
                            is_block_local: false,
                        });
                    }
                }
            }

            // Keyword rest parameter (**opts)
            if let Some(kwrest) = params.keyword_rest() {
                if let Some(kp) = kwrest.as_keyword_rest_parameter_node() {
                    if let Some(name) = kp.name() {
                        if let Some(name_loc) = kp.name_loc() {
                            param_info.push(ParamInfo {
                                name: name.as_slice().to_vec(),
                                offset: name_loc.start_offset(),
                                is_keyword: false,
                                is_block_local: false,
                            });
                        }
                    }
                }
            }

            // Block parameter (&block)
            if let Some(block) = params.block() {
                if let Some(name) = block.name() {
                    if let Some(name_loc) = block.name_loc() {
                        param_info.push(ParamInfo {
                            name: name.as_slice().to_vec(),
                            offset: name_loc.start_offset(),
                            is_keyword: false,
                            is_block_local: false,
                        });
                    }
                }
            }
        }

        // Collect block-local variables (|x; local_var|)
        for local in block_params_node.locals().iter() {
            if let Some(blv) = local.as_block_local_variable_node() {
                param_info.push(ParamInfo {
                    name: blv.name().as_slice().to_vec(),
                    offset: blv.location().start_offset(),
                    is_keyword: false,
                    is_block_local: true,
                });
            }
        }

        if param_info.is_empty() {
            return;
        }

        // Find all local variable reads and check for zero-arg `binding` calls in the body
        let mut finder = VarRefFinder {
            names: Vec::new(),
            has_zero_arg_binding: false,
            shadowed: Vec::new(),
        };
        if let Some(ref body) = body {
            finder.visit(body);
        }

        // If the block body calls `binding` without arguments, all args are considered used.
        // RuboCop applies this even when the call has a receiver (e.g. `tp.binding`).
        if finder.has_zero_arg_binding {
            return;
        }

        for info in &param_info {
            // Skip arguments prefixed with _
            if info.name.starts_with(b"_") {
                continue;
            }

            // For block-local variables, check if they are assigned (used as lvalue)
            // RuboCop considers a block-local variable "used" if it has any assignments
            if info.is_block_local {
                let mut write_finder = VarWriteFinder {
                    name: &info.name,
                    found: false,
                };
                if let Some(ref body) = body {
                    write_finder.visit(body);
                }
                if write_finder.found {
                    continue;
                }
            }

            // Check if the variable is referenced (read) in the body
            if !finder.names.iter().any(|n| n == &info.name) {
                let (line, column) = self.source.offset_to_line_col(info.offset);
                let display_name = if info.is_keyword {
                    let s = String::from_utf8_lossy(&info.name);
                    s.trim_end_matches(':').to_string()
                } else {
                    String::from_utf8_lossy(&info.name).to_string()
                };
                let var_type = if info.is_block_local {
                    "block local variable"
                } else {
                    "block argument"
                };
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("Unused {var_type} - `{display_name}`."),
                ));
            }
        }
    }
}

/// Recursively collect ParamInfo from a destructured MultiTargetNode.
/// E.g., `|(a, b, c)|` creates a MultiTargetNode with lefts [a, b, c].
fn collect_multi_target_params(mt: &ruby_prism::MultiTargetNode<'_>, out: &mut Vec<ParamInfo>) {
    for target in mt.lefts().iter() {
        if let Some(rp) = target.as_required_parameter_node() {
            out.push(ParamInfo {
                name: rp.name().as_slice().to_vec(),
                offset: rp.location().start_offset(),
                is_keyword: false,
                is_block_local: false,
            });
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_params(&inner, out);
        }
    }
    if let Some(rest) = mt.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(rp) = expr.as_required_parameter_node() {
                    out.push(ParamInfo {
                        name: rp.name().as_slice().to_vec(),
                        offset: rp.location().start_offset(),
                        is_keyword: false,
                        is_block_local: false,
                    });
                }
            }
        }
    }
    for target in mt.rights().iter() {
        if let Some(rp) = target.as_required_parameter_node() {
            out.push(ParamInfo {
                name: rp.name().as_slice().to_vec(),
                offset: rp.location().start_offset(),
                is_keyword: false,
                is_block_local: false,
            });
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_params(&inner, out);
        }
    }
}

/// Recursively collect names from a destructured MultiTargetNode (for shadowing).
fn collect_multi_target_names(mt: &ruby_prism::MultiTargetNode<'_>, names: &mut Vec<Vec<u8>>) {
    for target in mt.lefts().iter() {
        if let Some(rp) = target.as_required_parameter_node() {
            names.push(rp.name().as_slice().to_vec());
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_names(&inner, names);
        }
    }
    if let Some(rest) = mt.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(rp) = expr.as_required_parameter_node() {
                    names.push(rp.name().as_slice().to_vec());
                }
            }
        }
    }
    for target in mt.rights().iter() {
        if let Some(rp) = target.as_required_parameter_node() {
            names.push(rp.name().as_slice().to_vec());
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_names(&inner, names);
        }
    }
}

/// Finds local variable reads in a block body. Only counts actual reads,
/// not write targets (multi-assign LHS). Also detects zero-arg `binding` calls.
/// Tracks variable shadowing: when a nested block/lambda redeclares a parameter
/// with the same name, reads of that name inside the nested scope are not counted
/// as references to the outer parameter.
struct VarRefFinder {
    names: Vec<Vec<u8>>,
    has_zero_arg_binding: bool,
    /// Names currently shadowed by nested block parameters — reads of these
    /// names are NOT collected because they refer to the inner scope's param.
    shadowed: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for VarRefFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let name = node.name().as_slice();
        // Don't count reads of names that are shadowed by a nested block param
        if !self.shadowed.iter().any(|s| s.as_slice() == name) {
            self.names.push(name.to_vec());
        }
    }

    // Operator-assign (x += 1, x -= 1, etc.) implicitly reads the variable
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = node.name().as_slice();
        if !self.shadowed.iter().any(|s| s.as_slice() == name) {
            self.names.push(name.to_vec());
        }
        // Also visit the value expression (RHS) for any variable reads
        self.visit(&node.value());
    }

    // Or-assign (x ||= val) implicitly reads the variable
    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let name = node.name().as_slice();
        if !self.shadowed.iter().any(|s| s.as_slice() == name) {
            self.names.push(name.to_vec());
        }
        self.visit(&node.value());
    }

    // And-assign (x &&= val) implicitly reads the variable
    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let name = node.name().as_slice();
        if !self.shadowed.iter().any(|s| s.as_slice() == name) {
            self.names.push(name.to_vec());
        }
        self.visit(&node.value());
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // RuboCop treats any zero-arg `binding` send as scope-capturing, even
        // when it has a receiver like `tp.binding`.
        if node.name().as_slice() == b"binding" {
            let has_no_args = match node.arguments() {
                None => true,
                Some(args) => args.arguments().iter().next().is_none(),
            };
            if has_no_args {
                self.has_zero_arg_binding = true;
            }
        }
        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.visit_nested_block_or_lambda(node.parameters(), node.body());
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.visit_nested_block_or_lambda(node.parameters(), node.body());
    }

    // Don't recurse into nested def/class/module (they create new scopes).
    // But for def nodes, check the receiver — `def o.method` uses `o`.
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if let Some(lv) = receiver.as_local_variable_read_node() {
                let name = lv.name().as_slice();
                if !self.shadowed.iter().any(|s| s.as_slice() == name) {
                    self.names.push(name.to_vec());
                }
            }
        }
        // Don't recurse into the def body — it's a different scope
    }
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

impl VarRefFinder {
    /// Visit a nested block/lambda body, temporarily shadowing any parameter
    /// names that the nested scope redeclares.
    fn visit_nested_block_or_lambda<'pr>(
        &mut self,
        parameters: Option<ruby_prism::Node<'pr>>,
        body: Option<ruby_prism::Node<'pr>>,
    ) {
        // Collect parameter names from the nested block
        let nested_params = Self::collect_param_names(parameters);

        // Push shadowed names
        let shadow_start = self.shadowed.len();
        self.shadowed.extend(nested_params);

        // Visit the body with shadowed names active
        if let Some(body) = body {
            self.visit(&body);
        }

        // Pop shadowed names
        self.shadowed.truncate(shadow_start);
    }

    /// Extract all parameter names from a block_parameters node.
    fn collect_param_names(parameters: Option<ruby_prism::Node<'_>>) -> Vec<Vec<u8>> {
        let mut names = Vec::new();
        let block_params = match parameters.and_then(|p| p.as_block_parameters_node()) {
            Some(bp) => bp,
            None => return names,
        };
        let Some(params) = block_params.parameters() else {
            return names;
        };

        for req in params.requireds().iter() {
            if let Some(rp) = req.as_required_parameter_node() {
                names.push(rp.name().as_slice().to_vec());
            } else if let Some(mt) = req.as_multi_target_node() {
                collect_multi_target_names(&mt, &mut names);
            }
        }
        for opt in params.optionals().iter() {
            if let Some(op) = opt.as_optional_parameter_node() {
                names.push(op.name().as_slice().to_vec());
            }
        }
        if let Some(rest) = params.rest() {
            if let Some(rp) = rest.as_rest_parameter_node() {
                if let Some(name) = rp.name() {
                    names.push(name.as_slice().to_vec());
                }
            }
        }
        for post in params.posts().iter() {
            if let Some(rp) = post.as_required_parameter_node() {
                names.push(rp.name().as_slice().to_vec());
            } else if let Some(mt) = post.as_multi_target_node() {
                collect_multi_target_names(&mt, &mut names);
            }
        }
        for kw in params.keywords().iter() {
            if let Some(kp) = kw.as_required_keyword_parameter_node() {
                names.push(kp.name().as_slice().to_vec());
            } else if let Some(kp) = kw.as_optional_keyword_parameter_node() {
                names.push(kp.name().as_slice().to_vec());
            }
        }
        if let Some(kwrest) = params.keyword_rest() {
            if let Some(kp) = kwrest.as_keyword_rest_parameter_node() {
                if let Some(name) = kp.name() {
                    names.push(name.as_slice().to_vec());
                }
            }
        }
        if let Some(block) = params.block() {
            if let Some(name) = block.name() {
                names.push(name.as_slice().to_vec());
            }
        }
        names
    }
}

/// Checks if a specific variable is written to (assigned) in the body.
/// Used for block-local variables, which are considered "used" if assigned.
struct VarWriteFinder<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for VarWriteFinder<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }

    // Don't recurse into nested scopes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedBlockArgument, "cops/lint/unused_block_argument");
}
