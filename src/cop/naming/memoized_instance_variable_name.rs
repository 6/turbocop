use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, DEF_NODE, IF_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, INSTANCE_VARIABLE_WRITE_NODE, STATEMENTS_NODE};

pub struct MemoizedInstanceVariableName;

impl MemoizedInstanceVariableName {
    fn check_or_write(
        &self,
        source: &SourceFile,
        or_write: ruby_prism::InstanceVariableOrWriteNode<'_>,
        base_name: &str,
        method_name_str: &str,
        leading_underscore_style: &str,
    ) -> Vec<Diagnostic> {
        let ivar_name = or_write.name().as_slice();
        let ivar_str = std::str::from_utf8(ivar_name).unwrap_or("");
        let ivar_base = ivar_str.strip_prefix('@').unwrap_or(ivar_str);

        let matches = match leading_underscore_style {
            "required" => {
                // @_method_name is the only valid form
                let expected = format!("_{base_name}");
                ivar_base == expected
            }
            "optional" => {
                // Both @method_name and @_method_name are valid
                let with_underscore = format!("_{base_name}");
                ivar_base == base_name || ivar_base == with_underscore
            }
            _ => {
                // "disallowed" (default): only @method_name is valid
                ivar_base == base_name
            }
        };

        if !matches {
            let loc = or_write.name_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Memoized variable `@{ivar_base}` does not match method name `{method_name_str}`."
                ),
            )];
        }

        Vec::new()
    }

    /// Check the `defined?` memoization pattern and emit offenses on each ivar reference.
    /// RuboCop emits one offense per ivar occurrence (defined? check, return, assignment).
    fn check_defined_memoized(
        &self,
        source: &SourceFile,
        body_nodes: &[ruby_prism::Node<'_>],
        ivar_base: &str,
        base_name: &str,
        method_name_str: &str,
        enforced_style: &str,
    ) -> Vec<Diagnostic> {
        let matches = match enforced_style {
            "required" => {
                let expected = format!("_{base_name}");
                ivar_base == expected
            }
            "optional" => {
                let with_underscore = format!("_{base_name}");
                ivar_base == base_name || ivar_base == with_underscore
            }
            _ => {
                // "disallowed" (default)
                ivar_base == base_name
            }
        };

        if matches {
            return Vec::new();
        }

        let suggested = match enforced_style {
            "required" => format!("_{base_name}"),
            _ => base_name.to_string(),
        };

        let msg = format!(
            "Memoized variable `@{ivar_base}` does not match method name `{method_name_str}`. Use `@{suggested}` instead."
        );

        // Collect all ivar locations from the defined? pattern:
        // 1. defined?(@ivar) — the ivar inside defined?
        // 2. return @ivar — the ivar in the return
        // 3. @ivar = ... — the assignment
        let mut diags = Vec::new();

        // The first node should be an if with defined?
        if let Some(if_node) = body_nodes[0].as_if_node() {
            // defined?(@ivar) — in the predicate
            if let Some(call) = if_node.predicate().as_call_node() {
                if call.name().as_slice() == b"defined?" {
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if arg.as_instance_variable_read_node().is_some() {
                                let loc = arg.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diags.push(self.diagnostic(source, line, column, msg.clone()));
                            }
                        }
                    }
                }
            }
            // Also check if the predicate is a DefinedNode
            if let Some(defined) = if_node.predicate().as_defined_node() {
                let value = defined.value();
                if value.as_instance_variable_read_node().is_some() {
                    let loc = value.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diags.push(self.diagnostic(source, line, column, msg.clone()));
                }
            }

            // return @ivar — in the then/statements
            if let Some(stmts) = if_node.statements() {
                for stmt in stmts.body().iter() {
                    if let Some(ret) = stmt.as_return_node() {
                        if let Some(args) = ret.arguments() {
                            for arg in args.arguments().iter() {
                                if arg.as_instance_variable_read_node().is_some() {
                                    let loc = arg.location();
                                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                                    diags.push(self.diagnostic(source, line, column, msg.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        // The last node should be @ivar = ...
        let last = &body_nodes[body_nodes.len() - 1];
        if let Some(ivar_write) = last.as_instance_variable_write_node() {
            let loc = ivar_write.name_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diags.push(self.diagnostic(source, line, column, msg));
        }

        diags
    }
}

/// Extract the ivar name from a `defined?` memoization pattern.
/// Pattern: first statement is `return @ivar if defined?(@ivar)` (modifier if)
/// and last statement is `@ivar = expression`.
/// Returns the ivar name (e.g. "@token") if the pattern matches.
fn extract_defined_memoized_ivar(body_nodes: &[ruby_prism::Node<'_>]) -> Option<String> {
    if body_nodes.len() < 2 {
        return None;
    }

    // First statement: `return @ivar if defined?(@ivar)`
    // In Prism, this is an IfNode with:
    //   predicate: DefinedNode or CallNode(`defined?`)
    //   statements: ReturnNode with ivar argument
    let first = &body_nodes[0];
    let if_node = first.as_if_node()?;

    // Check predicate is `defined?(@ivar)`
    // Note: Prism's name().as_slice() for ivar nodes includes the '@' prefix.
    // We strip it here to get the base name for comparison.
    let defined_ivar_base = if let Some(defined) = if_node.predicate().as_defined_node() {
        // DefinedNode has a .value() that returns the argument
        let value = defined.value();
        let ivar = value.as_instance_variable_read_node()?;
        let full = std::str::from_utf8(ivar.name().as_slice()).ok()?;
        full.strip_prefix('@').unwrap_or(full).to_string()
    } else if let Some(call) = if_node.predicate().as_call_node() {
        // Fallback: CallNode with name `defined?`
        if call.name().as_slice() != b"defined?" {
            return None;
        }
        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }
        let ivar = arg_list[0].as_instance_variable_read_node()?;
        let full = std::str::from_utf8(ivar.name().as_slice()).ok()?;
        full.strip_prefix('@').unwrap_or(full).to_string()
    } else {
        return None;
    };

    // Check then-body has `return @ivar`
    let stmts = if_node.statements()?;
    let stmt_nodes: Vec<_> = stmts.body().iter().collect();
    if stmt_nodes.len() != 1 {
        return None;
    }
    let ret = stmt_nodes[0].as_return_node()?;
    let ret_args = ret.arguments()?;
    let ret_arg_list: Vec<_> = ret_args.arguments().iter().collect();
    if ret_arg_list.len() != 1 {
        return None;
    }
    let ret_ivar = ret_arg_list[0].as_instance_variable_read_node()?;
    let ret_full = std::str::from_utf8(ret_ivar.name().as_slice()).ok()?;
    let ret_ivar_base = ret_full.strip_prefix('@').unwrap_or(ret_full);
    if ret_ivar_base != defined_ivar_base {
        return None;
    }

    // Last statement: `@ivar = expression`
    let last = &body_nodes[body_nodes.len() - 1];
    let ivar_write = last.as_instance_variable_write_node()?;
    let write_full = std::str::from_utf8(ivar_write.name().as_slice()).ok()?;
    let write_base = write_full.strip_prefix('@').unwrap_or(write_full);
    if write_base != defined_ivar_base {
        return None;
    }

    // Return the base name (without '@')
    Some(defined_ivar_base)
}

impl Cop for MemoizedInstanceVariableName {
    fn name(&self) -> &'static str {
        "Naming/MemoizedInstanceVariableName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE, IF_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, INSTANCE_VARIABLE_WRITE_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyleForLeadingUnderscores", "disallowed");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let method_name = def_node.name().as_slice();
        let method_name_str = std::str::from_utf8(method_name).unwrap_or("");

        // RuboCop skips initialize methods — `||=` there is default initialization, not memoization
        if matches!(
            method_name_str,
            "initialize" | "initialize_clone" | "initialize_copy" | "initialize_dup"
        ) {
            return;
        }

        // Strip trailing ? or ! from method name for matching
        let base_name = method_name_str.trim_end_matches(|c| c == '?' || c == '!');

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Look for @var ||= pattern — only when it's the entire body or the last statement.
        // This is a memoization pattern; a `||=` in the middle of a method is just assignment.

        // Body could be a bare InstanceVariableOrWriteNode (single statement)
        if let Some(or_write) = body.as_instance_variable_or_write_node() {
            diagnostics.extend(self.check_or_write(source, or_write, base_name, method_name_str, enforced_style));
        }

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.is_empty() {
            return;
        }

        // Only check the last statement — vendor requires ||= be the sole or last statement
        let last = &body_nodes[body_nodes.len() - 1];
        if let Some(or_write) = last.as_instance_variable_or_write_node() {
            diagnostics.extend(self.check_or_write(source, or_write, base_name, method_name_str, enforced_style));
            return;
        }

        // Also check the `defined?` memoization pattern:
        //   return @ivar if defined?(@ivar)
        //   @ivar = expression
        // The first statement must be `if defined?(@ivar) then return @ivar end`
        // and the last statement must be `@ivar = expression`.
        if body_nodes.len() >= 2 {
            if let Some(ivar_base) = extract_defined_memoized_ivar(&body_nodes) {
                diagnostics.extend(
                    self.check_defined_memoized(source, &body_nodes, &ivar_base, base_name, method_name_str, enforced_style),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MemoizedInstanceVariableName,
        "cops/naming/memoized_instance_variable_name"
    );

    #[test]
    fn required_style_allows_leading_underscore() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForLeadingUnderscores".to_string(),
                serde_yml::Value::String("required".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def js_modules\n  @_js_modules ||= compute_modules\nend\n";
        assert_cop_no_offenses_full_with_config(&MemoizedInstanceVariableName, source, config);
    }

    #[test]
    fn optional_style_allows_both_forms() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForLeadingUnderscores".to_string(),
                serde_yml::Value::String("optional".to_string()),
            )]),
            ..CopConfig::default()
        };
        // Both forms should be accepted
        let source = b"def js_modules\n  @_js_modules ||= compute_modules\nend\n";
        assert_cop_no_offenses_full_with_config(
            &MemoizedInstanceVariableName,
            source,
            config.clone(),
        );
        let source2 = b"def js_modules\n  @js_modules ||= compute_modules\nend\n";
        assert_cop_no_offenses_full_with_config(&MemoizedInstanceVariableName, source2, config);
    }

    #[test]
    fn required_style_flags_missing_underscore() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForLeadingUnderscores".to_string(),
                serde_yml::Value::String("required".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def js_modules\n  @js_modules ||= compute_modules\nend\n";
        let diags = run_cop_full_with_config(&MemoizedInstanceVariableName, source, config);
        assert!(
            !diags.is_empty(),
            "required style should flag @js_modules (missing underscore)"
        );
    }
}
