use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for unused method arguments.
///
/// ## Root causes of historical FP/FN (corpus 87.3% → 99.6% match rate):
/// - **FN: block params (`&block`)** were not collected — now handled via `params.block()`
/// - **FN: keyword rest (`**opts`)** were not collected — now handled via `params.keyword_rest()`
/// - **FN: post params** (after rest, e.g. `def foo(*a, b)`) were not collected — now handled via `params.posts()`
/// - **FN: `LocalVariableTargetNode` treated as use** — multi-assignment LHS (`a, b = 1, 2`)
///   incorrectly prevented flagging parameters that were only assigned to, never read.
///   Removed from VarReadFinder; only actual reads count.
/// - **FN: `NotImplementedExceptions` config ignored** — hardcoded `NotImplementedError` instead
///   of reading from config. Now uses the configured exception list.
/// - **FN: `LocalVariableOperatorWriteNode`/`AndWriteNode`/`OrWriteNode`** (`a += 1`, `a ||= x`)
///   implicitly read the variable but weren't detected. Now handled.
/// - **FP: `binding` with receiver** — RuboCop's VariableForce treats ANY call to a method
///   named `binding` (regardless of receiver) as making all local variables referenced.
///   nitrocop only handled receiverless `binding`. Fixed to match RuboCop: `obj.binding`
///   now also suppresses unused argument warnings.
/// - **FN: empty methods with `IgnoreEmptyMethods: false`** — a double-return bug in the
///   `body.is_none()` branch caused empty methods to always be skipped, even when config
///   set `IgnoreEmptyMethods: false`. Fixed to properly check params when body is absent.
///
/// ## Additional fixes (corpus 99.6% → improved):
/// - **FN: block/lambda parameter shadowing** — when a block or lambda declares a parameter
///   with the same name as a method parameter (e.g., `def foo(x); items.each { |x| x }`),
///   the read inside the block refers to the block's variable, NOT the method's. VarReadFinder
///   now tracks `block_depth` and uses Prism's `depth()` field on read/write nodes to only
///   count references that reach back to the method scope (`depth >= block_depth`).
/// - **FP: `binding(&block)` incorrectly suppressed warnings** — in RuboCop's Parser AST,
///   a block-pass `&block` is a child of the send node, making it look like `binding` has
///   arguments. Prism separates block arguments from regular arguments. Fixed to also check
///   that the call's `block()` is not a `BlockArgumentNode`.
pub struct UnusedMethodArgument;

impl Cop for UnusedMethodArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedMethodArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let ignore_empty = config.get_bool("IgnoreEmptyMethods", true);
        let ignore_not_implemented = config.get_bool("IgnoreNotImplementedMethods", true);
        let allow_unused_keyword = config.get_bool("AllowUnusedKeywordArguments", false);
        let not_implemented_exceptions = config.get_string_array("NotImplementedExceptions");

        let body = def_node.body();

        // Check for empty methods
        if body.is_none() && ignore_empty {
            return;
        }

        // Check for not-implemented methods
        if let Some(ref b) = body {
            if ignore_not_implemented
                && is_not_implemented(b, not_implemented_exceptions.as_deref())
            {
                return;
            }
        }

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Collect parameter info: (name_bytes, offset, is_keyword)
        let mut param_info: Vec<(Vec<u8>, usize, bool)> = Vec::new();

        for req in params.requireds().iter() {
            if let Some(rp) = req.as_required_parameter_node() {
                param_info.push((
                    rp.name().as_slice().to_vec(),
                    rp.location().start_offset(),
                    false,
                ));
            }
        }

        for opt in params.optionals().iter() {
            if let Some(op) = opt.as_optional_parameter_node() {
                param_info.push((
                    op.name().as_slice().to_vec(),
                    op.location().start_offset(),
                    false,
                ));
            }
        }

        // Rest parameter (*args)
        if let Some(rest) = params.rest() {
            if let Some(rp) = rest.as_rest_parameter_node() {
                if let Some(name_loc) = rp.name_loc() {
                    param_info.push((
                        rp.name().map(|n| n.as_slice().to_vec()).unwrap_or_default(),
                        name_loc.start_offset(),
                        false,
                    ));
                }
            }
        }

        // Post parameters (required params after rest, e.g. `def foo(*args, last)`)
        for post in params.posts().iter() {
            if let Some(rp) = post.as_required_parameter_node() {
                param_info.push((
                    rp.name().as_slice().to_vec(),
                    rp.location().start_offset(),
                    false,
                ));
            }
        }

        if !allow_unused_keyword {
            for kw in params.keywords().iter() {
                if let Some(kp) = kw.as_required_keyword_parameter_node() {
                    param_info.push((
                        kp.name().as_slice().to_vec(),
                        kp.location().start_offset(),
                        true,
                    ));
                } else if let Some(kp) = kw.as_optional_keyword_parameter_node() {
                    param_info.push((
                        kp.name().as_slice().to_vec(),
                        kp.location().start_offset(),
                        true,
                    ));
                }
            }
        }

        // Keyword rest parameter (**opts)
        if let Some(kwrest) = params.keyword_rest() {
            if let Some(kp) = kwrest.as_keyword_rest_parameter_node() {
                if let Some(name_loc) = kp.name_loc() {
                    let is_keyword = false; // **opts is not a keyword arg for display purposes
                    param_info.push((
                        kp.name().map(|n| n.as_slice().to_vec()).unwrap_or_default(),
                        name_loc.start_offset(),
                        is_keyword,
                    ));
                }
            }
        }

        // Block parameter (&block)
        if let Some(block) = params.block() {
            if let Some(name_loc) = block.name_loc() {
                param_info.push((
                    block
                        .name()
                        .map(|n| n.as_slice().to_vec())
                        .unwrap_or_default(),
                    name_loc.start_offset(),
                    false,
                ));
            }
        }

        if param_info.is_empty() {
            return;
        }

        // Find all local variable reads in the body AND in parameter defaults.
        // A parameter used as a default value for another parameter counts as used
        // (e.g., `def foo(node, start = node)` — `node` is used in default of `start`).
        let mut finder = VarReadFinder {
            names: Vec::new(),
            has_forwarding_super: false,
            has_binding_call: false,
            block_depth: 0,
        };
        if let Some(ref b) = body {
            finder.visit(b);
        }

        // Also scan parameter default values for variable reads
        for opt in params.optionals().iter() {
            if let Some(op) = opt.as_optional_parameter_node() {
                finder.visit(&op.value());
            }
        }
        for kw in params.keywords().iter() {
            if let Some(kp) = kw.as_optional_keyword_parameter_node() {
                finder.visit(&kp.value());
            }
        }

        // If the body contains bare `super` (ForwardingSuperNode), all args are
        // implicitly forwarded and therefore "used".
        if finder.has_forwarding_super {
            return;
        }

        // If the body calls `binding`, all local variables are accessible via
        // `binding.local_variable_get`, so consider all args as used.
        if finder.has_binding_call {
            return;
        }

        for (name, offset, is_keyword) in &param_info {
            // Skip arguments prefixed with _
            if name.starts_with(b"_") {
                continue;
            }

            // Check if the variable is referenced in the body
            if !finder.names.iter().any(|n| n == name) {
                let (line, column) = source.offset_to_line_col(*offset);
                // For keyword args, strip trailing ':'
                let display_name = if *is_keyword {
                    let s = String::from_utf8_lossy(name);
                    s.trim_end_matches(':').to_string()
                } else {
                    String::from_utf8_lossy(name).to_string()
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Unused method argument - `{display_name}`."),
                ));
            }
        }
    }
}

fn is_not_implemented(body: &ruby_prism::Node<'_>, exceptions: Option<&[String]>) -> bool {
    // Check if body is a single `raise NotImplementedError` or `fail "..."` statement
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => {
            // Could be a direct call node
            return check_not_implemented_call(body, exceptions);
        }
    };

    let body_nodes: Vec<_> = stmts.body().iter().collect();
    if body_nodes.len() != 1 {
        return false;
    }

    check_not_implemented_call(&body_nodes[0], exceptions)
}

fn check_not_implemented_call(node: &ruby_prism::Node<'_>, exceptions: Option<&[String]>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let method_name = call.name().as_slice();
    if call.receiver().is_some() {
        return false;
    }

    if method_name == b"raise" {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if !arg_list.is_empty() {
                return is_allowed_exception(&arg_list[0], exceptions);
            }
        }
        // `raise` with no arguments is also a "not implemented" pattern
        false
    } else {
        method_name == b"fail"
    }
}

/// Check if a node is an allowed exception class for NotImplementedExceptions config.
/// Default allowed classes: ["NotImplementedError"].
fn is_allowed_exception(node: &ruby_prism::Node<'_>, exceptions: Option<&[String]>) -> bool {
    let const_name = if let Some(c) = node.as_constant_read_node() {
        String::from_utf8_lossy(c.name().as_slice()).to_string()
    } else if let Some(cp) = node.as_constant_path_node() {
        // Handle qualified constants like ::NotImplementedError or Library::AbstractMethodError
        // Reconstruct the full constant path name
        extract_constant_path_name(&cp)
    } else {
        return false;
    };

    match exceptions {
        Some(allowed) => {
            if allowed.is_empty() {
                // Empty config: only default NotImplementedError
                const_name == "NotImplementedError" || const_name == "::NotImplementedError"
            } else {
                // Check against configured exceptions, allowing :: prefix
                allowed.iter().any(|exc| {
                    const_name == *exc
                        || const_name == format!("::{exc}")
                        || format!("::{const_name}") == *exc
                })
            }
        }
        None => {
            // No config: default to NotImplementedError
            const_name == "NotImplementedError" || const_name == "::NotImplementedError"
        }
    }
}

/// Extract the full constant path name, e.g., "Foo::Bar" or "::Foo::Bar"
fn extract_constant_path_name(cp: &ruby_prism::ConstantPathNode<'_>) -> String {
    let mut parts = Vec::new();
    let mut has_root = false;

    // Get the child name
    if let Some(name) = cp.name() {
        parts.push(String::from_utf8_lossy(name.as_slice()).to_string());
    }

    // Walk up the parent chain
    if let Some(parent) = cp.parent() {
        if let Some(parent_cp) = parent.as_constant_path_node() {
            let parent_name = extract_constant_path_name(&parent_cp);
            return format!("{parent_name}::{}", parts.first().unwrap_or(&String::new()));
        } else if let Some(cr) = parent.as_constant_read_node() {
            parts.insert(0, String::from_utf8_lossy(cr.name().as_slice()).to_string());
        }
    } else {
        // No parent means root-level (::Foo)
        has_root = true;
    }

    let path = parts.join("::");
    if has_root { format!("::{path}") } else { path }
}

struct VarReadFinder {
    names: Vec<Vec<u8>>,
    has_forwarding_super: bool,
    has_binding_call: bool,
    /// Number of block/lambda scopes we've entered. Used to correctly scope
    /// variable reads: only reads at depth >= block_depth reference the
    /// method's parameters. Reads at depth < block_depth reference a
    /// block/lambda's own parameter (which may shadow the method param).
    block_depth: u32,
}

impl<'pr> Visit<'pr> for VarReadFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        // Only count this read as a method-param reference if its depth
        // reaches back to the method scope. `depth` is how many scopes
        // up from the innermost enclosing scope the variable resolves to.
        // A read inside a block with depth 0 refers to the block's own
        // variable (which may shadow a method param with the same name).
        if node.depth() >= self.block_depth {
            self.names.push(node.name().as_slice().to_vec());
        }
    }

    // Compound assignment operators (+=, -=, etc.) implicitly read the variable
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        if node.depth() >= self.block_depth {
            self.names.push(node.name().as_slice().to_vec());
        }
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    // `a &&= b` implicitly reads `a`
    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        if node.depth() >= self.block_depth {
            self.names.push(node.name().as_slice().to_vec());
        }
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    // `a ||= b` implicitly reads `a`
    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        if node.depth() >= self.block_depth {
            self.names.push(node.name().as_slice().to_vec());
        }
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    // Bare `super` (no args, no parens) implicitly forwards all method arguments
    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        self.has_forwarding_super = true;
    }

    // Detect `binding` calls — accessing binding exposes all local variables.
    // RuboCop's VariableForce treats `binding` with ANY receiver (including
    // `obj.binding`) as making all variables referenced, so we match that
    // behavior. Only `binding` with arguments (e.g. `binding(:something)`)
    // is excluded — that's not Kernel#binding.
    // Also exclude `binding(&block)` — in RuboCop's Parser AST, a block_pass
    // is an argument that makes `args.children.empty?` false, so RuboCop
    // does NOT treat `binding(&block)` as Kernel#binding.
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"binding"
            && node.arguments().is_none()
            && node
                .block()
                .is_none_or(|b| b.as_block_argument_node().is_none())
        {
            self.has_binding_call = true;
        }
        ruby_prism::visit_call_node(self, node);
    }

    // Block/lambda scopes increment block_depth so that variable reads
    // inside them are correctly scoped. Only reads with depth >= block_depth
    // reference the method's parameters.
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.block_depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.block_depth -= 1;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.block_depth += 1;
        ruby_prism::visit_lambda_node(self, node);
        self.block_depth -= 1;
    }

    // Don't recurse into nested def/class/module/sclass (they have their own scope)
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_singleton_class_node(&mut self, _node: &ruby_prism::SingletonClassNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnusedMethodArgument, "cops/lint/unused_method_argument");

    #[test]
    fn test_block_param_unused() {
        // &block parameter that is unused should be flagged
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(a, &block)\n  puts a\nend\n",
        );
        let names: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(
            names.iter().any(|m| m.contains("block")),
            "Expected offense for unused &block, got: {:?}",
            names
        );
    }

    #[test]
    fn test_kwrest_param_unused() {
        // **opts parameter that is unused should be flagged
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(a, **opts)\n  puts a\nend\n",
        );
        let names: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(
            names.iter().any(|m| m.contains("opts")),
            "Expected offense for unused **opts, got: {:?}",
            names
        );
    }

    #[test]
    fn test_post_param_unused() {
        // post parameter (after rest) that is unused should be flagged
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(*args, last)\n  args.first\nend\n",
        );
        let names: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(
            names.iter().any(|m| m.contains("last")),
            "Expected offense for unused post param 'last', got: {:?}",
            names
        );
    }

    #[test]
    fn test_keyword_arg_used_no_offense() {
        // keyword arg that IS used should NOT be flagged
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(bar:)\n  puts bar\nend\n",
        );
        assert!(
            diags.is_empty(),
            "Expected no offense for used keyword arg, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_binding_with_receiver_no_offense() {
        // obj.binding should suppress unused arg warnings (matches RuboCop)
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(bar)\n  some_object.binding\nend\n",
        );
        assert!(
            diags.is_empty(),
            "Expected no offense when obj.binding is called, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_empty_method_ignore_false() {
        // When IgnoreEmptyMethods is false, empty methods should still flag unused args
        let mut config = CopConfig::default();
        config.options.insert(
            "IgnoreEmptyMethods".to_string(),
            serde_yml::Value::Bool(false),
        );
        let diags = crate::testutil::run_cop_full_with_config(
            &UnusedMethodArgument,
            b"def foo(bar)\nend\n",
            config,
        );
        assert!(
            !diags.is_empty(),
            "Expected offense for unused arg in empty method when IgnoreEmptyMethods=false"
        );
    }

    #[test]
    fn test_block_param_shadows_method_param_fn() {
        // When a block parameter shadows a method parameter, the method param
        // is unused even though a variable with the same name is read inside
        // the block. RuboCop's VariableForce correctly scopes this.
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(x)\n  items.each { |x| puts x }\nend\n",
        );
        assert!(
            diags.iter().any(|d| d.message.contains("x")),
            "Expected offense for method param 'x' shadowed by block param, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_lambda_param_shadows_method_param_fn() {
        // Lambda parameter shadows method parameter
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(x)\n  ->(x) { puts x }\nend\n",
        );
        assert!(
            diags.iter().any(|d| d.message.contains("x")),
            "Expected offense for method param 'x' shadowed by lambda param, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_binding_with_block_pass_still_flags() {
        // binding(&block) is NOT Kernel#binding — should still flag unused args
        // RuboCop's Parser treats &block as an argument, so VariableForce
        // does not suppress warnings.
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(bar, &blk)\n  binding(&blk)\nend\n",
        );
        assert!(
            diags.iter().any(|d| d.message.contains("bar")),
            "Expected offense for unused 'bar' when binding(&blk), got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_multi_assign_target_not_used() {
        // Multi-assignment target should NOT count as a use of the parameter
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument,
            b"def foo(a, b)\n  a, b = 1, 2\nend\n",
        );
        assert!(
            diags.len() >= 2,
            "Expected 2 offenses for multi-assign only, got: {} ({:?})",
            diags.len(),
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
