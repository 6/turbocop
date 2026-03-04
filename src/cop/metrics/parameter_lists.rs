use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

// ## Investigation history
//
// Commit 0a021cc8 rewrote from check_node to check_source with Visit-based walker.
// Added block param checking, MaxOptionalParameters, Struct.new/Data.define exemption.
//
// 2026-03-03: Fixed two issues reducing FP=3 FN=8 → FP=2 FN=0:
// (1) Struct.new/Data.define initialize exemption was too broad — exempted ALL
//     initialize methods inside the block. RuboCop only exempts when initialize is the
//     sole statement (no begin wrapper in AST). When block has multiple methods,
//     parent.parent gives begin instead of block, so RuboCop's pattern doesn't match.
//     Fix: only exempt when block body has exactly one statement.
// (2) Block param offenses reported at ParametersNode location (first param after pipe)
//     instead of BlockParametersNode location (opening pipe). For multi-line blocks,
//     this put the offense on a different line than RuboCop, causing FP+FN pair.
//     Fix: report at block_params_node.location() start.
// (3) consuldemocracy FPs (2): rubocop:disable ParameterLists comment on line above.
//     This is a framework-level directive handling issue, not cop-level.
pub struct ParameterLists;

impl Cop for ParameterLists {
    fn name(&self) -> &'static str {
        "Metrics/ParameterLists"
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
        let max = config.get_usize("Max", 5);
        let count_keyword_args = config.get_bool("CountKeywordArgs", true);
        let max_optional = config.get_usize("MaxOptionalParameters", 3);

        let mut visitor = ParameterListsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            max,
            count_keyword_args,
            max_optional,
            in_struct_or_data_single_child: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ParameterListsVisitor<'a> {
    cop: &'a ParameterLists,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    max: usize,
    count_keyword_args: bool,
    max_optional: usize,
    /// True when inside a Struct.new/Data.define block whose body has exactly
    /// one statement (the initialize def). RuboCop's exemption only applies
    /// when `parent.parent` directly reaches the block node (no begin wrapper).
    in_struct_or_data_single_child: bool,
}

impl<'a> ParameterListsVisitor<'a> {
    fn count_params(&self, params: &ruby_prism::ParametersNode<'_>) -> usize {
        let mut count = 0usize;
        count += params.requireds().len();
        count += params.optionals().len();
        count += params.posts().len();

        if params.rest().is_some() {
            count += 1;
        }

        if self.count_keyword_args {
            count += params.keywords().len();
            if params.keyword_rest().is_some() {
                count += 1;
            }
        }

        count
    }

    /// Check if a CallNode is Struct.new or Data.define (or ::Struct.new / ::Data.define)
    fn is_struct_new_or_data_define(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name();
        let name_bytes = name.as_slice();

        if let Some(receiver) = call.receiver() {
            if name_bytes == b"new" {
                // Struct.new or ::Struct.new
                if let Some(cr) = receiver.as_constant_read_node() {
                    return cr.name().as_slice() == b"Struct";
                }
                if let Some(cp) = receiver.as_constant_path_node() {
                    // ::Struct (parent is None for cbase)
                    if cp.parent().is_none() {
                        if let Some(child) = cp.name() {
                            return child.as_slice() == b"Struct";
                        }
                    }
                }
            } else if name_bytes == b"define" {
                // Data.define or ::Data.define
                if let Some(cr) = receiver.as_constant_read_node() {
                    return cr.name().as_slice() == b"Data";
                }
                if let Some(cp) = receiver.as_constant_path_node() {
                    if cp.parent().is_none() {
                        if let Some(child) = cp.name() {
                            return child.as_slice() == b"Data";
                        }
                    }
                }
            }
        }
        false
    }
}

impl<'pr> Visit<'pr> for ParameterListsVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Skip initialize inside Struct.new/Data.define blocks when it's the sole child.
        // RuboCop's on_args checks `parent.parent` which only reaches the block node
        // (matching struct_new_or_data_define_block?) when there's no begin wrapper,
        // i.e., when initialize is the only statement in the block body.
        let is_initialize =
            self.in_struct_or_data_single_child && node.name().as_slice() == b"initialize";

        if !is_initialize {
            if let Some(params) = node.parameters() {
                let count = self.count_params(&params);
                if count > self.max {
                    let start_offset = node.def_keyword_loc().start_offset();
                    let (line, column) = self.source.offset_to_line_col(start_offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!(
                            "Avoid parameter lists longer than {} parameters. [{}/{}]",
                            self.max, count, self.max
                        ),
                    ));
                }
            }

            // Check optional parameter count (only for method defs, not blocks)
            if let Some(params) = node.parameters() {
                let optional_count = params.optionals().len();
                if optional_count > self.max_optional {
                    let start_offset = node.def_keyword_loc().start_offset();
                    let (line, column) = self.source.offset_to_line_col(start_offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!(
                            "Method has too many optional parameters. [{}/{}]",
                            optional_count, self.max_optional
                        ),
                    ));
                }
            }
        }

        // Continue visiting children (e.g., nested defs)
        ruby_prism::visit_def_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call has a block with parameters
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                // Skip proc/lambda blocks — their params are exempt
                let name = node.name();
                let is_proc_or_lambda = node.receiver().is_none()
                    && (name.as_slice() == b"proc" || name.as_slice() == b"lambda");

                if !is_proc_or_lambda {
                    self.check_block_params(&block_node);
                }

                if Self::is_struct_new_or_data_define(node) {
                    // Set context for children (def initialize exemption).
                    // Only exempt when block body has exactly one statement,
                    // matching RuboCop's parent.parent check behavior.
                    let single_child = Self::block_has_single_child(&block_node);
                    let prev = self.in_struct_or_data_single_child;
                    self.in_struct_or_data_single_child = single_child;
                    ruby_prism::visit_call_node(self, node);
                    self.in_struct_or_data_single_child = prev;
                    return;
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }

    // Lambda params are exempt — don't check, just visit children
    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        ruby_prism::visit_lambda_node(self, node);
    }
}

impl ParameterListsVisitor<'_> {
    /// Check if a block body has exactly one statement.
    /// In Prism, a block body is either None, a StatementsNode (with body list),
    /// or a single node. When there's a single statement, RuboCop's `parent.parent`
    /// from the def's args node reaches the block node directly (no begin wrapper).
    fn block_has_single_child(block_node: &ruby_prism::BlockNode<'_>) -> bool {
        if let Some(body) = block_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                return stmts.body().len() == 1;
            }
            // Non-statements body (e.g., a single node) counts as one child
            return true;
        }
        false // empty body
    }

    fn check_block_params(&mut self, block_node: &ruby_prism::BlockNode<'_>) {
        let block_params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params_node = match block_params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let params = match block_params_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let count = self.count_params(&params);
        if count > self.max {
            // Report at the BlockParametersNode location (opening pipe),
            // matching RuboCop's on_args reporting location.
            let start_offset = block_params_node.location().start_offset();
            let (line, column) = self.source.offset_to_line_col(start_offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!(
                    "Avoid parameter lists longer than {} parameters. [{}/{}]",
                    self.max, count, self.max
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ParameterLists, "cops/metrics/parameter_lists");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(2.into()))]),
            ..CopConfig::default()
        };
        // 3 params exceeds Max:2
        let source = b"def foo(a, b, c)\nend\n";
        let diags = run_cop_full_with_config(&ParameterLists, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire with Max:2 on 3-param method"
        );
        assert!(diags[0].message.contains("[3/2]"));
    }

    #[test]
    fn config_max_optional_parameters() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // 3 optional params with MaxOptionalParameters:2 should fire
        let config = CopConfig {
            options: HashMap::from([(
                "MaxOptionalParameters".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def foo(a = 1, b = 2, c = 3)\nend\n";
        let diags = run_cop_full_with_config(&ParameterLists, source, config);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("too many optional parameters")),
            "Should fire for too many optional parameters"
        );
    }

    #[test]
    fn config_max_optional_parameters_ok() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // 2 optional params with MaxOptionalParameters:3 should not fire
        let config = CopConfig {
            options: HashMap::from([(
                "MaxOptionalParameters".into(),
                serde_yml::Value::Number(3.into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def foo(a = 1, b = 2)\nend\n";
        let diags = run_cop_full_with_config(&ParameterLists, source, config);
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("optional parameters")),
            "Should not fire for optional parameters under limit"
        );
    }
}
