use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Debugger;

impl Cop for Debugger {
    fn name(&self) -> &'static str {
        "Lint/Debugger"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Config: DebuggerMethods overrides/extends the built-in list of
        // debugger entry-point methods.
        let debugger_requires = config.get_string_array("DebuggerRequires");
        let extra_methods = config.get_string_array("DebuggerMethods");

        let method_name = call.name().as_slice();

        // DebuggerRequires: check for `require 'debug_lib'` calls
        if method_name == b"require" && call.receiver().is_none() {
            if let Some(requires) = &debugger_requires {
                if let Some(args) = call.arguments() {
                    let arg_list = args.arguments();
                    if arg_list.len() == 1 {
                        let first = arg_list.iter().next().unwrap();
                        if let Some(s) = first.as_string_node() {
                            let val = s.unescaped();
                            if requires.iter().any(|r| r.as_bytes() == &*val) {
                                let loc = call.location();
                                let source_text = std::str::from_utf8(loc.as_slice()).unwrap_or("require");
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                return vec![self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Remove debugger entry point `{source_text}`."),
                                )];
                            }
                        }
                    }
                }
            }
        }

        // Check if method_name matches a user-configured extra debugger method
        if let Some(extras) = &extra_methods {
            if extras.iter().any(|m| m.as_bytes() == method_name) && call.receiver().is_none() {
                let loc = call.location();
                let source_text = std::str::from_utf8(loc.as_slice()).unwrap_or("debugger");
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Remove debugger entry point `{source_text}`."),
                )];
            }
        }

        let is_debugger = match method_name {
            b"pry" | b"remote_pry" | b"pry_remote" => {
                // binding.pry, binding.remote_pry, binding.pry_remote
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        recv_call.name().as_slice() == b"binding"
                            && recv_call.receiver().is_none()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            b"debugger" | b"byebug" => {
                // receiver-less debugger/byebug calls
                call.receiver().is_none()
            }
            b"irb" => {
                // binding.irb
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        recv_call.name().as_slice() == b"binding"
                            && recv_call.receiver().is_none()
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };

        if is_debugger {
            let loc = call.location();
            let source_text = std::str::from_utf8(loc.as_slice()).unwrap_or("debugger");
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                format!("Remove debugger entry point `{source_text}`."),
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Debugger, "cops/lint/debugger");

    #[test]
    fn config_debugger_requires() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

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
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([(
                "DebuggerRequires".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("pry".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"require 'json'\n";
        let diags = run_cop_full_with_config(&Debugger, source, config);
        assert!(diags.is_empty());
    }
}
