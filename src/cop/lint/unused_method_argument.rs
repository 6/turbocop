use std::collections::HashSet;
use std::sync::Mutex;

use crate::cop::variable_force::{self, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

use super::super::variable_force::scope::ScopeKind;
use super::super::variable_force::variable::DeclarationKind;

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
///
/// ## Additional fixes (corpus 99.7% → improved):
/// - **FP: twisted scope expressions not visited** — RuboCop's VariableForce has
///   `TWISTED_SCOPE_TYPES` which processes certain expressions belonging to the outer
///   scope before entering a new scope. nitrocop's VarReadFinder was entirely skipping
///   nested `DefNode`, `ClassNode`, `SingletonClassNode`, and `ModuleNode` — meaning
///   method arguments used as: (1) singleton method receivers (`def obj.method_name`),
///   (2) singleton class expressions (`class << obj`), (3) superclass expressions
///   (`class Foo < base`) were not detected as used, producing false positives.
///   Fixed to visit these "twisted" expressions while still skipping the body.
///
/// ## Migration to VariableForce
///
/// This cop was migrated from a 645-line standalone AST visitor to use the shared
/// VariableForce engine. A minimal `check_source` pass pre-computes which def
/// scopes should be skipped (empty methods, not-implemented stubs). The VF
/// `before_leaving_scope` hook then checks each argument variable for usage.
pub struct UnusedMethodArgument {
    /// Byte offsets of def nodes whose bodies are empty or raise NotImplementedError.
    /// Populated by `check_source`, consumed by `before_leaving_scope`.
    skip_offsets: Mutex<HashSet<usize>>,
}

impl UnusedMethodArgument {
    pub fn new() -> Self {
        Self {
            skip_offsets: Mutex::new(HashSet::new()),
        }
    }
}

impl Default for UnusedMethodArgument {
    fn default() -> Self {
        Self::new()
    }
}

impl Cop for UnusedMethodArgument {
    fn name(&self) -> &'static str {
        "Lint/UnusedMethodArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        _source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ignore_empty = config.get_bool("IgnoreEmptyMethods", true);
        let ignore_not_implemented = config.get_bool("IgnoreNotImplementedMethods", true);
        let not_implemented_exceptions = config.get_string_array("NotImplementedExceptions");

        let mut collector = SkipCollector {
            offsets: HashSet::new(),
            ignore_empty,
            ignore_not_implemented,
            not_implemented_exceptions,
        };
        collector.visit(&parse_result.node());
        *self.skip_offsets.lock().unwrap() = collector.offsets;
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for UnusedMethodArgument {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only process method scopes (Def and Defs)
        if !matches!(scope.kind, ScopeKind::Def | ScopeKind::Defs) {
            return;
        }

        // Check if this scope should be skipped (empty or not-implemented)
        if self
            .skip_offsets
            .lock()
            .unwrap()
            .contains(&scope.node_start_offset)
        {
            return;
        }

        let allow_unused_keyword = config.get_bool("AllowUnusedKeywordArguments", false);
        let ignore_implicit = config.get_bool("IgnoreImplicitReferences", false);

        for variable in scope.variables.values() {
            if !variable.is_argument() {
                continue;
            }

            if variable.should_be_unused() {
                continue;
            }

            // Skip keyword args when AllowUnusedKeywordArguments is true
            if allow_unused_keyword
                && matches!(
                    variable.declaration_kind,
                    DeclarationKind::KeywordArg | DeclarationKind::OptionalKeywordArg
                )
            {
                continue;
            }

            // Check if the variable is used
            let is_used = if ignore_implicit {
                // Only count explicit references
                variable.references.iter().any(|r| r.explicit) || variable.captured_by_block
            } else {
                variable.used()
            };

            if !is_used {
                let (line, column) = source.offset_to_line_col(variable.declaration_offset);
                let display_name = String::from_utf8_lossy(&variable.name);
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

/// AST visitor that collects byte offsets of def nodes to skip:
/// - Empty methods (when IgnoreEmptyMethods is true)
/// - Methods that only raise NotImplementedError (when IgnoreNotImplementedMethods is true)
struct SkipCollector {
    offsets: HashSet<usize>,
    ignore_empty: bool,
    ignore_not_implemented: bool,
    not_implemented_exceptions: Option<Vec<String>>,
}

impl<'pr> Visit<'pr> for SkipCollector {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let body = node.body();
        if body.is_none() && self.ignore_empty {
            self.offsets.insert(node.location().start_offset());
        } else if let Some(ref b) = body {
            if self.ignore_not_implemented
                && is_not_implemented(b, self.not_implemented_exceptions.as_deref())
            {
                self.offsets.insert(node.location().start_offset());
            }
        }
        // Recurse into the body for nested defs
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

fn is_not_implemented(body: &ruby_prism::Node<'_>, exceptions: Option<&[String]>) -> bool {
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => {
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
        false
    } else {
        method_name == b"fail"
    }
}

fn is_allowed_exception(node: &ruby_prism::Node<'_>, exceptions: Option<&[String]>) -> bool {
    let const_name = if let Some(c) = node.as_constant_read_node() {
        String::from_utf8_lossy(c.name().as_slice()).to_string()
    } else if let Some(cp) = node.as_constant_path_node() {
        extract_constant_path_name(&cp)
    } else {
        return false;
    };

    match exceptions {
        Some(allowed) => {
            if allowed.is_empty() {
                const_name == "NotImplementedError" || const_name == "::NotImplementedError"
            } else {
                allowed.iter().any(|exc| {
                    const_name == *exc
                        || const_name == format!("::{exc}")
                        || format!("::{const_name}") == *exc
                })
            }
        }
        None => const_name == "NotImplementedError" || const_name == "::NotImplementedError",
    }
}

fn extract_constant_path_name(cp: &ruby_prism::ConstantPathNode<'_>) -> String {
    let mut parts = Vec::new();
    let mut has_root = false;

    if let Some(name) = cp.name() {
        parts.push(String::from_utf8_lossy(name.as_slice()).to_string());
    }

    if let Some(parent) = cp.parent() {
        if let Some(parent_cp) = parent.as_constant_path_node() {
            let parent_name = extract_constant_path_name(&parent_cp);
            return format!("{parent_name}::{}", parts.first().unwrap_or(&String::new()));
        } else if let Some(cr) = parent.as_constant_read_node() {
            parts.insert(0, String::from_utf8_lossy(cr.name().as_slice()).to_string());
        }
    } else {
        has_root = true;
    }

    let path = parts.join("::");
    if has_root { format!("::{path}") } else { path }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnusedMethodArgument::new(),
        "cops/lint/unused_method_argument"
    );

    #[test]
    fn test_block_param_unused() {
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let mut config = CopConfig::default();
        config.options.insert(
            "IgnoreEmptyMethods".to_string(),
            serde_yml::Value::Bool(false),
        );
        let diags = crate::testutil::run_cop_full_with_config(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
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
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
            b"def foo(a, b)\n  a, b = 1, 2\nend\n",
        );
        assert!(
            diags.len() >= 2,
            "Expected 2 offenses for multi-assign only, got: {} ({:?})",
            diags.len(),
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_implemented_method_skipped() {
        // Methods that raise NotImplementedError should not flag unused args
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
            b"def create_server(cloud_server)\n  raise NotImplementedError\nend\n",
        );
        assert!(
            diags.is_empty(),
            "NotImplementedError method should be skipped, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_empty_method_skipped() {
        // Empty methods should not flag unused args (IgnoreEmptyMethods default: true)
        let diags =
            crate::testutil::run_cop_full(&UnusedMethodArgument::new(), b"def foo(x)\nend\n");
        assert!(
            diags.is_empty(),
            "Empty method should be skipped, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_not_implemented_nested_skipped() {
        // Nested defs with NotImplementedError should also be skipped
        let diags = crate::testutil::run_cop_full(
            &UnusedMethodArgument::new(),
            b"class Foo\n  def bar(x)\n    raise NotImplementedError\n  end\nend\n",
        );
        assert!(
            diags.is_empty(),
            "Nested NotImplementedError method should be skipped, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
