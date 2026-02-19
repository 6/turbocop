use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE};

pub struct Debugger;

/// Default debugger methods when no config is provided.
const DEFAULT_DEBUGGER_METHODS: &[&str] = &[
    "binding.irb",
    "Kernel.binding.irb",
    "byebug",
    "remote_byebug",
    "Kernel.byebug",
    "Kernel.remote_byebug",
    "page.save_and_open_page",
    "page.save_and_open_screenshot",
    "page.save_page",
    "page.save_screenshot",
    "save_and_open_page",
    "save_and_open_screenshot",
    "save_page",
    "save_screenshot",
    "binding.b",
    "binding.break",
    "Kernel.binding.b",
    "Kernel.binding.break",
    "binding.pry",
    "binding.remote_pry",
    "binding.pry_remote",
    "Kernel.binding.pry",
    "Kernel.binding.remote_pry",
    "Kernel.binding.pry_remote",
    "Pry.rescue",
    "pry",
    "debugger",
    "Kernel.debugger",
    "jard",
    "binding.console",
];

const DEFAULT_DEBUGGER_REQUIRES: &[&str] = &["debug/open", "debug/start"];

/// Check if a call node matches a dotted method spec like "binding.pry" or "Kernel.binding.irb".
fn matches_method_spec(call: &ruby_prism::CallNode<'_>, spec: &str) -> bool {
    let parts: Vec<&str> = spec.split('.').collect();
    if parts.is_empty() {
        return false;
    }
    matches_parts(call, &parts)
}

fn matches_parts(call: &ruby_prism::CallNode<'_>, parts: &[&str]) -> bool {
    if parts.is_empty() {
        return false;
    }
    let method = parts[parts.len() - 1];
    if call.name().as_slice() != method.as_bytes() {
        return false;
    }
    let receiver_parts = &parts[..parts.len() - 1];
    if receiver_parts.is_empty() {
        return call.receiver().is_none();
    }
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    if receiver_parts.len() == 1 {
        let name = receiver_parts[0];
        // Could be a bare method call or a constant
        if let Some(recv_call) = recv.as_call_node() {
            return recv_call.name().as_slice() == name.as_bytes()
                && recv_call.receiver().is_none();
        }
        if let Some(const_read) = recv.as_constant_read_node() {
            return const_read.name().as_slice() == name.as_bytes();
        }
        // Handle qualified constants (Foo::Bar) — extract the last segment
        if let Some(const_path) = recv.as_constant_path_node() {
            if let Some(child) = const_path.name() {
                return child.as_slice() == name.as_bytes();
            }
        }
        return false;
    }
    // Multi-part receiver: recurse
    if let Some(recv_call) = recv.as_call_node() {
        matches_parts(&recv_call, receiver_parts)
    } else {
        false
    }
}

impl Cop for Debugger {
    fn name(&self) -> &'static str {
        "Lint/Debugger"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // DebuggerRequires: check for `require 'debug_lib'` calls
        if method_name == b"require" && call.receiver().is_none() {
            let requires: Vec<String> = config
                .get_flat_string_values("DebuggerRequires")
                .unwrap_or_else(|| {
                    DEFAULT_DEBUGGER_REQUIRES.iter().map(|s| s.to_string()).collect()
                });
            if let Some(args) = call.arguments() {
                let arg_list = args.arguments();
                if arg_list.len() == 1 {
                    let first = arg_list.iter().next().unwrap();
                    if let Some(s) = first.as_string_node() {
                        let val = s.unescaped();
                        if requires.iter().any(|r| r.as_bytes() == &*val) {
                            let loc = call.location();
                            let source_text =
                                std::str::from_utf8(loc.as_slice()).unwrap_or("require");
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Remove debugger entry point `{source_text}`."),
                            ));
                        }
                    }
                }
            }
        }

        // DebuggerMethods: check against configured or default method list
        let methods: Vec<String> = config
            .get_flat_string_values("DebuggerMethods")
            .unwrap_or_else(|| {
                DEFAULT_DEBUGGER_METHODS.iter().map(|s| s.to_string()).collect()
            });

        for spec in &methods {
            if matches_method_spec(&call, spec) {
                let loc = call.location();
                let source_text = std::str::from_utf8(loc.as_slice()).unwrap_or("debugger");
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Remove debugger entry point `{source_text}`."),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Debugger, "cops/lint/debugger");

    #[test]
    fn config_debugger_requires() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "DebuggerRequires".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("debug/start".into()),
                    serde_yml::Value::String("pry".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"require 'debug/start'\n";
        let diags = run_cop_full_with_config(&Debugger, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("debugger entry point"));
    }

    #[test]
    fn config_debugger_requires_no_match() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "DebuggerRequires".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("pry".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"require 'json'\n";
        let diags = run_cop_full_with_config(&Debugger, source, config);
        assert!(diags.is_empty());
    }

    #[test]
    fn save_page_detected() {
        use crate::testutil::run_cop_full;
        let source = b"save_page(path)\n";
        let diags = run_cop_full(&Debugger, source);
        assert_eq!(diags.len(), 1, "save_page should be detected: {:?}", diags);
        assert!(diags[0].message.contains("save_page"));
    }

    #[test]
    fn binding_console_detected() {
        use crate::testutil::run_cop_full;
        let source = b"binding.console\n";
        let diags = run_cop_full(&Debugger, source);
        assert_eq!(diags.len(), 1, "binding.console should be detected: {:?}", diags);
    }

    #[test]
    fn page_save_page_detected() {
        use crate::testutil::run_cop_full;
        let source = b"page.save_page\n";
        let diags = run_cop_full(&Debugger, source);
        assert_eq!(diags.len(), 1, "page.save_page should be detected: {:?}", diags);
    }

    #[test]
    fn debugger_methods_hash_config() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // Simulate the nested hash format from rubocop config
        let config = CopConfig {
            options: HashMap::from([(
                "DebuggerMethods".into(),
                serde_yml::Value::Mapping(serde_yml::Mapping::from_iter([
                    (
                        serde_yml::Value::String("Custom".into()),
                        serde_yml::Value::Sequence(vec![
                            serde_yml::Value::String("my_debug".into()),
                        ]),
                    ),
                ])),
            )]),
            ..CopConfig::default()
        };
        let source = b"my_debug\n";
        let diags = run_cop_full_with_config(&Debugger, source, config);
        assert_eq!(diags.len(), 1, "custom debugger method should be detected");
    }
}
