use ruby_prism::Visit;

use crate::cop::shared::node_type::{CALL_NODE, CONSTANT_READ_NODE, MODULE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03):
/// 12 FPs all from TracksApp/tracks, all the pattern `raise Exception.new, "message"`.
/// Root cause: RuboCop's NodePattern `exception_new_with_message?` only matches when
/// `Exception.new(...)` is the sole argument to raise. When `.new` has no args and the
/// message is a separate second arg to raise (`raise Exception.new, "msg"`), neither
/// RuboCop pattern matches. Fixed by rejecting Exception.new as first arg when there
/// are additional args to raise/fail.
pub struct RaiseException;

/// Collect all module names that enclose a given byte offset.
struct EnclosingModuleFinder {
    target_offset: usize,
    module_names: Vec<String>,
}

impl<'pr> Visit<'pr> for EnclosingModuleFinder {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if let Some(module_node) = node.as_module_node() {
            let loc = module_node.location();
            if self.target_offset >= loc.start_offset() && self.target_offset < loc.end_offset() {
                // Extract the module name from its constant_path
                let name_loc = module_node.constant_path().location();
                if let Ok(name) = std::str::from_utf8(name_loc.as_slice()) {
                    self.module_names.push(name.to_string());
                }
            }
        }
    }
}

/// Check if a node is a bare `Exception` or root-scoped `::Exception`.
/// Returns false for namespaced constants like `Foreman::Exception` or `::Foreman::Exception`.
fn is_bare_exception(node: &ruby_prism::Node<'_>) -> bool {
    // Bare `Exception` → ConstantReadNode
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == b"Exception";
    }
    // `::Exception` → ConstantPathNode with no parent (cbase) and name "Exception"
    if let Some(cp) = node.as_constant_path_node() {
        if cp.parent().is_none() {
            if let Some(name) = cp.name() {
                return name.as_slice() == b"Exception";
            }
        }
    }
    false
}

fn is_exception_reference(node: &ruby_prism::Node<'_>) -> bool {
    // Direct constant: bare Exception or ::Exception (root-scoped)
    if is_bare_exception(node) {
        return true;
    }
    // Exception.new(...) or ::Exception.new(...)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"new" {
            if let Some(recv) = call.receiver() {
                return is_bare_exception(&recv);
            }
        }
    }
    false
}

impl Cop for RaiseException {
    fn name(&self) -> &'static str {
        "Lint/RaiseException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_READ_NODE, MODULE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_namespaces = config.get_string_array("AllowedImplicitNamespaces");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a receiverless raise or fail
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if method_name != b"raise" && method_name != b"fail" {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        let first_arg = match args.first() {
            Some(a) => a,
            None => return,
        };

        // RuboCop has two NodePattern matchers:
        //   exception?:                (send nil? {:raise :fail} $(const ...) ...)
        //   exception_new_with_message?: (send nil? {:raise :fail} (send $(const ...) :new ...))
        //
        // The first matches `raise Exception` or `raise Exception, "msg"` (const as first arg, any extra args).
        // The second matches `raise Exception.new(...)` (Exception.new as the ONLY arg to raise).
        //
        // `raise Exception.new, "msg"` does NOT match either pattern:
        // - Not exception? because first arg is a send node, not a const
        // - Not exception_new_with_message? because Exception.new is not the only arg to raise
        //
        // So we must reject the case where first_arg is Exception.new AND there are extra args to raise.
        let is_new_call = first_arg.as_call_node().is_some();
        if is_new_call {
            // Exception.new is only flagged when it's the sole argument to raise/fail
            if args.len() > 1 {
                return;
            }
        }

        if !is_exception_reference(&first_arg) {
            return;
        }

        // AllowedImplicitNamespaces: only apply to bare `Exception` (not `::Exception`)
        // When `raise Exception` is inside a module in the allowed list, the bare
        // `Exception` implicitly refers to that module's own Exception class.
        let is_unqualified = first_arg.as_constant_read_node().is_some()
            || first_arg
                .as_call_node()
                .and_then(|c| c.receiver())
                .and_then(|r| r.as_constant_read_node())
                .is_some();
        if is_unqualified {
            if let Some(allowed) = &allowed_namespaces {
                if !allowed.is_empty() {
                    let call_offset = call.location().start_offset();
                    let mut finder = EnclosingModuleFinder {
                        target_offset: call_offset,
                        module_names: Vec::new(),
                    };
                    finder.visit(&parse_result.node());
                    if finder
                        .module_names
                        .iter()
                        .any(|name| allowed.iter().any(|a| a == name))
                    {
                        return;
                    }
                }
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use a subclass of `Exception` instead of raising `Exception` directly.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RaiseException, "cops/lint/raise_exception");

    #[test]
    fn config_allowed_implicit_namespaces() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedImplicitNamespaces".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("Gem".into())]),
            )]),
            ..CopConfig::default()
        };
        // raise Exception inside module Gem should be allowed
        let source = b"module Gem\n  def foo\n    raise Exception\n  end\nend\n";
        let diags = run_cop_full_with_config(&RaiseException, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag raise Exception inside allowed namespace Gem"
        );
    }

    #[test]
    fn config_allowed_implicit_namespaces_not_matched() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedImplicitNamespaces".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("Gem".into())]),
            )]),
            ..CopConfig::default()
        };
        // raise Exception inside module Foo should still be flagged
        let source = b"module Foo\n  def bar\n    raise Exception\n  end\nend\n";
        let diags = run_cop_full_with_config(&RaiseException, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag raise Exception in non-allowed namespace"
        );
    }
}
