use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RedundantMerge;

impl Cop for RedundantMerge {
    fn name(&self) -> &'static str {
        "Performance/RedundantMerge"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let max_kv_pairs = config.get_usize("MaxKeyValuePairs", 2);
        let mut visitor = RedundantMergeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            max_kv_pairs,
            value_used: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantMergeVisitor<'a, 'src> {
    cop: &'a RedundantMerge,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    max_kv_pairs: usize,
    /// Whether the current expression's value is used by a parent.
    value_used: bool,
}

impl<'a, 'src> RedundantMergeVisitor<'a, 'src> {
    fn check_merge_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        if call.name().as_slice() != b"merge!" {
            return;
        }

        // Must have a receiver (hash.merge!)
        if call.receiver().is_none() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();

        // Count key-value pairs in the merge! argument
        let kv_count = if args.len() == 1 {
            let first = args.iter().next().unwrap();
            // Don't flag if argument contains a splat (**hash)
            if let Some(kw) = first.as_keyword_hash_node() {
                if kw
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
                kw.elements().len()
            } else if let Some(hash) = first.as_hash_node() {
                if hash
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
                hash.elements().len()
            } else {
                0
            }
        } else {
            0
        };

        if kv_count == 0 || kv_count > self.max_kv_pairs {
            return;
        }

        // RuboCop: when pairs > 1, only flag if receiver is "pure" (a simple
        // local variable). Method calls, indexing etc. could have side effects.
        let receiver = call.receiver().unwrap();
        if kv_count > 1 {
            let is_pure = receiver.as_local_variable_read_node().is_some();
            if !is_pure {
                return;
            }
        }

        // Don't flag if the return value of merge! is used. merge! returns
        // the hash, while []= returns the assigned value — they're not
        // interchangeable when the result is consumed.
        if self.value_used {
            return;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        let msg = if kv_count == 1 {
            "Use `[]=` instead of `merge!` with a single key-value pair.".to_string()
        } else {
            format!("Use `[]=` instead of `merge!` with {kv_count} key-value pairs.")
        };
        self.diagnostics
            .push(self.cop.diagnostic(self.source, line, column, msg));
    }
}

impl<'pr> Visit<'pr> for RedundantMergeVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check this call for merge! offense
        self.check_merge_call(node);

        // Visit receiver — its value is used (as the receiver)
        if let Some(recv) = node.receiver() {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&recv);
            self.value_used = prev;
        }

        // Visit arguments — their values are used
        if let Some(args) = node.arguments() {
            let prev = self.value_used;
            self.value_used = true;
            self.visit_arguments_node(&args);
            self.value_used = prev;
        }

        // Visit block — the block body's value may or may not be used
        // depending on the method, but we treat it conservatively
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        // RHS of assignment — value is used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_instance_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_class_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_global_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_constant_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_constant_path_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_operator_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_or_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_and_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        // Value part of a hash pair — value is used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_assoc_node(self, node);
        self.value_used = prev;
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_assoc_splat_node(self, node);
        self.value_used = prev;
    }

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_return_node(self, node);
        self.value_used = prev;
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        // Parenthesized expression passes through value_used context
        ruby_prism::visit_parentheses_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // The condition's value is used; the branches inherit parent's value_used
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        if let Some(subsequent) = node.subsequent() {
            self.visit(&subsequent);
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&node.predicate());
            self.value_used = prev;
        }
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        // In a statements list, only the last statement's value is potentially
        // used (as the implicit return). All other statements are side-effect only.
        let stmts: Vec<_> = node.body().iter().collect();
        let last_idx = stmts.len().saturating_sub(1);
        for (i, stmt) in stmts.iter().enumerate() {
            if i == last_idx {
                // Last statement inherits parent's value_used
                self.visit(stmt);
            } else {
                let prev = self.value_used;
                self.value_used = false;
                self.visit(stmt);
                self.value_used = prev;
            }
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Method body's last expression is the implicit return value —
        // treat it as value_used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_def_node(self, node);
        self.value_used = prev;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Block body's last expression becomes the block's return value —
        // conservatively treat as value_used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_block_node(self, node);
        self.value_used = prev;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_lambda_node(self, node);
        self.value_used = prev;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // begin..end passes through value_used to its statements
        ruby_prism::visit_begin_node(self, node);
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // rescue clauses inherit value_used from the begin block
        ruby_prism::visit_rescue_node(self, node);
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        ruby_prism::visit_ensure_node(self, node);
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // The predicate value is used; when branches inherit value_used
        if let Some(pred) = node.predicate() {
            let prev = self.value_used;
            self.value_used = true;
            self.visit(&pred);
            self.value_used = prev;
        }
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        // Interpolated parts have their value used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_interpolated_string_node(self, node);
        self.value_used = prev;
    }

    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_embedded_statements_node(self, node);
        self.value_used = prev;
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        // Array elements have their value used
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_array_node(self, node);
        self.value_used = prev;
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_hash_node(self, node);
        self.value_used = prev;
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_keyword_hash_node(self, node);
        self.value_used = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMerge, "cops/performance/redundant_merge");

    #[test]
    fn config_max_kv_pairs_flags_two() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // Default MaxKeyValuePairs:2 should flag merge! with 2 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag merge! with 2 pairs when MaxKeyValuePairs:2"
        );
    }

    #[test]
    fn config_max_kv_pairs_allows_three() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // MaxKeyValuePairs:2 should NOT flag merge! with 3 KV pairs
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag merge! with 3 pairs when MaxKeyValuePairs:2"
        );
    }

    #[test]
    fn config_max_kv_pairs_higher() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // MaxKeyValuePairs:5 should flag merge! with up to 5 KV pairs on a local var
        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(5.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"h = {}\nh.merge!(a: 1, b: 2, c: 3)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag merge! with 3 pairs when MaxKeyValuePairs:5"
        );
    }

    #[test]
    fn non_pure_receiver_multi_pair_not_flagged() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "MaxKeyValuePairs".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        // Method call receiver — not a local variable, should not be flagged with 2 pairs
        let source = b"obj.options.merge!(a: 1, b: 2)\n";
        let diags = run_cop_full_with_config(&RedundantMerge, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag non-pure receiver with multiple pairs"
        );
    }
}
