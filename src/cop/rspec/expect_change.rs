use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExpectChange;

impl Cop for ExpectChange {
    fn name(&self) -> &'static str {
        "RSpec/ExpectChange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Config: EnforcedStyle — "method_call" (default) or "block"
        let enforced_style = config.get_str("EnforcedStyle", "method_call");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_some() {
            return Vec::new();
        }

        if call.name().as_slice() != b"change" {
            return Vec::new();
        }

        if enforced_style == "block" {
            // "block" style: flag `change(Obj, :attr)` — prefer block form
            let args = match call.arguments() {
                Some(a) => a,
                None => return Vec::new(),
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 2 {
                return Vec::new();
            }
            // First arg should be a constant, second a symbol
            let first = &arg_list[0];
            if first.as_constant_read_node().is_none() && first.as_constant_path_node().is_none() {
                return Vec::new();
            }
            if arg_list[1].as_symbol_node().is_none() {
                return Vec::new();
            }
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Prefer `change { }` over `change(obj, :attr)`.".to_string(),
            )];
        }

        // Default: "method_call" style — flag `change { User.count }`
        // and suggest `change(User, :count)`.
        let block_node_raw = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block = match block_node_raw.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // If it already has positional arguments, it's method_call style — fine
        if call.arguments().is_some() {
            return Vec::new();
        }

        // Check if the block body is a simple method call: Receiver.method (no args)
        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let stmt_list: Vec<_> = stmts.body().iter().collect();
        if stmt_list.len() != 1 {
            return Vec::new();
        }

        let inner_call = match stmt_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be a method call on a receiver with no arguments
        if inner_call.receiver().is_none() {
            return Vec::new();
        }

        if inner_call.arguments().is_some() {
            return Vec::new();
        }

        // The receiver must be a constant (User, Admin::Base, etc.) — not a method call
        let recv = inner_call.receiver().unwrap();
        if recv.as_constant_read_node().is_none() && recv.as_constant_path_node().is_none() {
            return Vec::new();
        }

        let recv_loc = recv.location();
        let recv_text = std::str::from_utf8(
            &source.as_bytes()[recv_loc.start_offset()..recv_loc.end_offset()],
        )
        .unwrap_or("");
        let method = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `change({recv_text}, :{method})`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectChange, "cops/rspec/expect_change");

    #[test]
    fn block_style_flags_method_call_form() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("block".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect { x }.to change(User, :count)\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExpectChange, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("change { }"));
    }

    #[test]
    fn block_style_does_not_flag_block_form() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("block".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect { x }.to change { User.count }\n";
        let diags = crate::testutil::run_cop_full_with_config(&ExpectChange, source, config);
        assert!(diags.is_empty());
    }
}
