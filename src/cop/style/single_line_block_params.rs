use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks whether the block parameters of a single-line method accepting a block
/// match the names specified via configuration.
///
/// ## Root causes of historical FPs (138 FP, 4 FN in corpus):
/// - Leading underscores were not stripped before comparison (`_acc` should match `acc`)
/// - No receiver check (bare `reduce {}` without receiver should not be flagged)
/// - Destructured params (e.g. `|acc, (id, _)|`) were not properly excluded
/// - Partial param lists (e.g. `|acc|` alone) were not handled per RuboCop logic
/// - `Methods` config was read but ignored; method names and expected params were hardcoded
/// - Message did not preserve underscore prefix from actual params
/// - Blocks with keyword params (e.g. `|src, from:, to:|`) were not excluded;
///   RuboCop's `eligible_arguments?` requires all params to be `arg_type?`
pub struct SingleLineBlockParams;

/// Default methods config: reduce/inject with params [acc, elem]
const DEFAULT_METHODS: &[(&str, &[&str])] =
    &[("reduce", &["acc", "elem"]), ("inject", &["acc", "elem"])];

/// Parse the Methods config from YAML.
/// Format: [{reduce: [acc, elem]}, {inject: [acc, elem]}]
/// Returns a Vec of (method_name, [param_names]).
fn parse_methods_config(config: &CopConfig) -> Vec<(String, Vec<String>)> {
    if let Some(val) = config.options.get("Methods") {
        if let Some(seq) = val.as_sequence() {
            let mut result = Vec::new();
            for item in seq {
                if let Some(mapping) = item.as_mapping() {
                    for (k, v) in mapping.iter() {
                        if let Some(method_name) = k.as_str() {
                            let params: Vec<String> = v
                                .as_sequence()
                                .map(|s| {
                                    s.iter()
                                        .filter_map(|p| p.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            result.push((method_name.to_string(), params));
                        }
                    }
                }
            }
            if !result.is_empty() {
                return result;
            }
        }
    }
    // Fallback to defaults
    DEFAULT_METHODS
        .iter()
        .map(|(name, params)| {
            (
                name.to_string(),
                params.iter().map(|p| p.to_string()).collect(),
            )
        })
        .collect()
}

/// Strip leading underscores from a parameter name for comparison.
fn strip_underscores(name: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < name.len() && name[i] == b'_' {
        i += 1;
    }
    &name[i..]
}

impl Cop for SingleLineBlockParams {
    fn name(&self) -> &'static str {
        "Style/SingleLineBlockParams"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // RuboCop requires a receiver (eligible_method? checks node.receiver)
        if call.receiver().is_none() {
            return;
        }

        let method_name = call.name().as_slice();
        let method_name_str = std::str::from_utf8(method_name).unwrap_or("");

        // Look up expected params for this method from config
        let methods = parse_methods_config(config);
        let expected_params = match methods.iter().find(|(name, _)| name == method_name_str) {
            Some((_, params)) => params,
            None => return,
        };

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check if block is on a single line
        let (start_line, _) = source.offset_to_line_col(block_node.location().start_offset());
        let (end_line, _) = source.offset_to_line_col(block_node.location().end_offset());
        if start_line != end_line {
            return;
        }

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return, // no block arguments = no offense
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };

        let param_node = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        // RuboCop: eligible_arguments? checks node.arguments.to_a.all?(&:arg_type?)
        // If there are any non-required params (keywords, optionals, rest, etc.),
        // the block is not eligible.
        if !param_node.optionals().is_empty()
            || param_node.rest().is_some()
            || !param_node.posts().is_empty()
            || !param_node.keywords().is_empty()
            || param_node.keyword_rest().is_some()
            || param_node.block().is_some()
        {
            return;
        }

        let requireds: Vec<_> = param_node.requireds().iter().collect();

        // All params must be simple arg type (no destructuring)
        // RuboCop: node.arguments.to_a.all?(&:arg_type?)
        for req in &requireds {
            if req.as_required_parameter_node().is_none() {
                return;
            }
        }

        // RuboCop allows partial args: only compare first N expected params
        // where N = number of actual args
        if requireds.len() > expected_params.len() {
            return;
        }
        let expected_subset = &expected_params[..requireds.len()];

        // Check if parameter names match (stripping leading underscores)
        // RuboCop: actual_args.map { |arg| arg.to_s.sub(/^_+/, '') } == expected_args
        let mut all_match = true;
        for (i, req) in requireds.iter().enumerate() {
            if let Some(rp) = req.as_required_parameter_node() {
                let actual_stripped = strip_underscores(rp.name().as_slice());
                if actual_stripped != expected_subset[i].as_bytes() {
                    all_match = false;
                    break;
                }
            }
        }

        if all_match {
            return;
        }

        // Build the preferred param names, preserving underscore prefix from actuals
        let preferred: Vec<String> = requireds
            .iter()
            .enumerate()
            .map(|(i, req)| {
                let actual_name = req
                    .as_required_parameter_node()
                    .map(|rp| rp.name().as_slice())
                    .unwrap_or(b"");
                let starts_with_underscore = actual_name.first() == Some(&b'_');
                if starts_with_underscore {
                    format!("_{}", expected_subset[i])
                } else {
                    expected_subset[i].clone()
                }
            })
            .collect();

        let joined = preferred.join(", ");

        let loc = block_params.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Name `{}` block params `|{}|`.", method_name_str, joined,),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SingleLineBlockParams, "cops/style/single_line_block_params");
}
