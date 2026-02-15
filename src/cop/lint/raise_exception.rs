use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

fn is_exception_reference(node: &ruby_prism::Node<'_>) -> bool {
    // Direct constant: Exception or Module::Exception (via constant_path_node)
    if let Some(name) = crate::cop::util::constant_name(node) {
        return name == b"Exception";
    }
    // Exception.new(...)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"new" {
            if let Some(recv) = call.receiver() {
                if let Some(name) = crate::cop::util::constant_name(&recv) {
                    return name == b"Exception";
                }
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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allowed_namespaces = config.get_string_array("AllowedImplicitNamespaces");
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be a receiverless raise or fail
        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if method_name != b"raise" && method_name != b"fail" {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        let first_arg = match args.first() {
            Some(a) => a,
            None => return Vec::new(),
        };

        if !is_exception_reference(&first_arg) {
            return Vec::new();
        }

        // AllowedImplicitNamespaces: only apply to bare `Exception` (not `::Exception`)
        // When `raise Exception` is inside a module in the allowed list, the bare
        // `Exception` implicitly refers to that module's own Exception class.
        let is_bare_exception = first_arg.as_constant_read_node().is_some();
        if is_bare_exception {
            if let Some(allowed) = &allowed_namespaces {
                if !allowed.is_empty() {
                    let call_offset = call.location().start_offset();
                    let mut finder = EnclosingModuleFinder {
                        target_offset: call_offset,
                        module_names: Vec::new(),
                    };
                    finder.visit(&parse_result.node());
                    if finder.module_names.iter().any(|name| allowed.iter().any(|a| a == name)) {
                        return Vec::new();
                    }
                }
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a subclass of `Exception` instead of raising `Exception` directly.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RaiseException, "cops/lint/raise_exception");

    #[test]
    fn config_allowed_implicit_namespaces() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedImplicitNamespaces".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("Gem".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        // raise Exception inside module Gem should be allowed
        let source = b"module Gem\n  def foo\n    raise Exception\n  end\nend\n";
        let diags = run_cop_full_with_config(&RaiseException, source, config);
        assert!(diags.is_empty(), "Should not flag raise Exception inside allowed namespace Gem");
    }

    #[test]
    fn config_allowed_implicit_namespaces_not_matched() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedImplicitNamespaces".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("Gem".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        // raise Exception inside module Foo should still be flagged
        let source = b"module Foo\n  def bar\n    raise Exception\n  end\nend\n";
        let diags = run_cop_full_with_config(&RaiseException, source, config);
        assert_eq!(diags.len(), 1, "Should flag raise Exception in non-allowed namespace");
    }
}
