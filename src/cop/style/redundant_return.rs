use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantReturn;

impl Cop for RedundantReturn {
    fn name(&self) -> &'static str {
        "Style/RedundantReturn"
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
        let allow_multiple = config.get_bool("AllowMultipleReturnValues", false);
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        check_terminal(self, source, &body, allow_multiple, diagnostics);
    }
}

/// Recursively check terminal positions for redundant `return` statements.
/// A terminal position is the last expression that would be implicitly returned.
fn check_terminal(
    cop: &RedundantReturn,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    allow_multiple: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // StatementsNode: check the last statement
    if let Some(stmts) = node.as_statements_node() {
        if let Some(last) = stmts.body().last() {
            check_terminal(cop, source, &last, allow_multiple, diagnostics);
        }
        return;
    }

    // ReturnNode: this is a redundant return in terminal position
    if let Some(ret_node) = node.as_return_node() {
        if allow_multiple {
            let arg_count = ret_node.arguments().map_or(0, |a| a.arguments().len());
            if arg_count > 1 {
                return;
            }
        }
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Redundant `return` detected.".to_string(),
        ));
        return;
    }

    // IfNode: check terminal position in each branch
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
        }
        if let Some(subsequent) = if_node.subsequent() {
            if let Some(elsif) = subsequent.as_if_node() {
                check_terminal(cop, source, &elsif.as_node(), allow_multiple, diagnostics);
            } else if let Some(else_node) = subsequent.as_else_node() {
                if let Some(stmts) = else_node.statements() {
                    check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
                }
            }
        }
        return;
    }

    // UnlessNode: check terminal position in each branch
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
            }
        }
        return;
    }

    // CaseNode: check terminal position in each when/else branch
    if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
            }
        }
        return;
    }

    // BeginNode: check statements body and rescue clauses
    if let Some(begin_node) = node.as_begin_node() {
        // Check main body statements
        if let Some(stmts) = begin_node.statements() {
            check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
        }
        // Check rescue clauses
        if let Some(rescue) = begin_node.rescue_clause() {
            check_rescue_terminal(cop, source, &rescue, allow_multiple, diagnostics);
        }
        // Check else clause on begin/rescue/else
        if let Some(else_clause) = begin_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
            }
        }
        return;
    }

    // RescueNode (implicit rescue on def body): check each rescue clause
    if let Some(rescue_node) = node.as_rescue_node() {
        // The rescue node's own statements
        if let Some(stmts) = rescue_node.statements() {
            check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
        }
        // Subsequent rescue clauses
        if let Some(subsequent) = rescue_node.subsequent() {
            check_rescue_terminal(cop, source, &subsequent, allow_multiple, diagnostics);
        }
    }
}

/// Check the last statement in a StatementsNode as a terminal position.
fn check_terminal_stmts(
    cop: &RedundantReturn,
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    allow_multiple: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(last) = stmts.body().last() {
        check_terminal(cop, source, &last, allow_multiple, diagnostics);
    }
}

/// Recursively check rescue clause chains for redundant returns.
fn check_rescue_terminal(
    cop: &RedundantReturn,
    source: &SourceFile,
    rescue: &ruby_prism::RescueNode<'_>,
    allow_multiple: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(stmts) = rescue.statements() {
        check_terminal_stmts(cop, source, &stmts, allow_multiple, diagnostics);
    }
    if let Some(subsequent) = rescue.subsequent() {
        check_rescue_terminal(cop, source, &subsequent, allow_multiple, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(RedundantReturn, "cops/style/redundant_return");

    #[test]
    fn allow_multiple_return_values() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultipleReturnValues".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // `return x, y` should be allowed when AllowMultipleReturnValues is true
        let source = b"def foo\n  return x, y\nend\n";
        let diags = run_cop_full_with_config(&RedundantReturn, source, config);
        assert!(
            diags.is_empty(),
            "Should allow multiple return values when configured"
        );
    }

    #[test]
    fn disallow_multiple_return_values_by_default() {
        // `return x, y` should be flagged by default
        let source = b"def foo\n  return x, y\nend\n";
        let diags = run_cop_full(&RedundantReturn, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multiple return values by default"
        );
    }

    #[test]
    fn allow_multiple_still_flags_single_return() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowMultipleReturnValues".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // `return x` should still be flagged even with AllowMultipleReturnValues
        let source = b"def foo\n  return x\nend\n";
        let diags = run_cop_full_with_config(&RedundantReturn, source, config);
        assert_eq!(diags.len(), 1, "Single return should still be flagged");
    }
}
