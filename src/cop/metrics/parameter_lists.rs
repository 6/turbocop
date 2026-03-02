use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

// ## Known false positives (811 FP) in corpus as of 2026-03-02
//
// Commit 0a021cc8 rewrote this cop from check_node to check_source with a Visit-based
// walker. The rewrite added: (1) block parameter checking for non-lambda/proc blocks,
// (2) MaxOptionalParameters checking, (3) Struct.new/Data.define initialize exemption.
// This eliminated all 8 FNs but introduced 811 new FPs across the corpus.
//
// Investigation findings:
// - Side-by-side comparison on specific repos (e.g., twilio-ruby) shows they MATCH on
//   individual files. The 811 excess is aggregate across 1000 repos.
// - The excess does NOT come from obviously wrong counting — param counts match on
//   spot-checked files.
// - RuboCop's on_args fires on ALL args nodes (both def and block params), and on_def
//   fires separately for MaxOptionalParameters. nitrocop's visitor does the same but
//   reports both on def_keyword_loc rather than separate locations.
// - Possible cause (a): block param checking may fire on constructs RuboCop exempts via
//   argument_to_lambda_or_proc? which uses ^lambda_or_proc? (parent-node check).
//   nitrocop only checks for explicit proc/lambda method names as receiver-less calls.
//   Other proc-like constructs (e.g., Proc.new, method(:foo)) might be missed.
// - Possible cause (b): config inheritance edge cases — some repos may have
//   MaxOptionalParameters set in inherited configs that nitrocop doesn't resolve
//   identically to RuboCop.
//
// A correct fix needs to:
// - Write a script to diff RuboCop vs nitrocop offenses per-file on a high-divergence
//   repo to find the exact lines that differ
// - Check if Proc.new blocks are being incorrectly flagged
// - Validate block param counting against RuboCop's argument_to_lambda_or_proc? more
//   carefully — it may exempt more than just proc and lambda calls
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
            in_struct_or_data_block: false,
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
    in_struct_or_data_block: bool,
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
        // Skip initialize inside Struct.new/Data.define blocks
        let is_initialize = self.in_struct_or_data_block && node.name().as_slice() == b"initialize";

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
                    // Set context for children (def initialize exemption)
                    let prev = self.in_struct_or_data_block;
                    self.in_struct_or_data_block = true;
                    ruby_prism::visit_call_node(self, node);
                    self.in_struct_or_data_block = prev;
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
            // Report on the parameters (inside the pipes)
            let start_offset = params.location().start_offset();
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
