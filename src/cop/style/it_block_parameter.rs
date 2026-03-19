use crate::cop::node_type::{CALL_NODE, LAMBDA_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/ItBlockParameter: checks for blocks where `it` block parameter can be used.
///
/// ## Investigation findings (2026-03-15)
///
/// The original implementation was fundamentally wrong — it only detected `|it|` named
/// block parameters, but RuboCop's cop handles implicit `it` blocks (Ruby 3.4+) and
/// numbered parameters (`_1`).
///
/// Root cause of FP=602: was flagging `|it|` named params which RuboCop never flags
/// in the default `allow_single_line` style.
///
/// Root cause of FN=1523: was not detecting multi-line implicit `it` blocks or `_1`
/// numbered parameters at all.
///
/// Rewritten to handle all 4 EnforcedStyle options:
/// - `allow_single_line` (default): flags multi-line `it` blocks + any `_1` usage
/// - `only_numbered_parameters`: flags only `_1` usage
/// - `always`: flags `_1` + single named params (should use `it` instead)
/// - `disallow`: flags all implicit `it` usage
///
/// Prism node mapping:
/// - `on_itblock` → `BlockNode` with `ItParametersNode` as parameters
/// - `on_numblock` → `BlockNode` with `NumberedParametersNode` (maximum == 1)
/// - `on_block` (always) → `BlockNode` with `BlockParametersNode` (single required param)
/// - `ItLocalVariableReadNode` = implicit `it` reference
/// - `LocalVariableReadNode` with name `_1` = numbered param reference
///
/// ## Investigation findings (2026-03-19)
///
/// FP=21, FN=43 remaining. Root causes:
///
/// 1. **Lambda blocks not handled (FN)** — `-> { _1 }` and `-> do it end` create
///    `LambdaNode` in Prism, not `CallNode`. RuboCop's `on_numblock`/`on_itblock`/`on_block`
///    fire for lambdas too. Fix: add `LAMBDA_NODE` to interested types and handle it.
///
/// 2. **Multi-line check used block_node range instead of full expression (FN)** —
///    For `allow_single_line` itblocks, nitrocop checked `block_node.location()` which
///    only covers `do..end` or `{..}`. RuboCop checks the full itblock node which
///    includes the receiver/method call. For multi-line method chains with single-line
///    brace blocks like `foo\n  .bar { it }`, RuboCop flags it (itblock is multi-line)
///    but nitrocop didn't (block is single-line). Fix: use call_node range for the
///    multi-line check.
pub struct ItBlockParameter;

impl Cop for ItBlockParameter {
    fn name(&self) -> &'static str {
        "Style/ItBlockParameter"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LAMBDA_NODE]
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
        // RuboCop: minimum_target_ruby_version 3.4
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version < 3.4 {
            return;
        }

        let style = config.get_str("EnforcedStyle", "allow_single_line");

        // Handle LambdaNode (-> { _1 }, -> { it }, -> { |x| x })
        if let Some(lambda) = node.as_lambda_node() {
            self.check_lambda(source, node, &lambda, style, diagnostics);
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
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

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Handle itblock (implicit `it` parameter)
        if params.as_it_parameters_node().is_some() {
            self.check_itblock(source, node, &block_node, style, diagnostics);
            return;
        }

        // Handle numblock (numbered parameters like _1)
        if let Some(numbered) = params.as_numbered_parameters_node() {
            self.check_numblock(source, &block_node, &numbered, style, diagnostics);
            return;
        }

        // Handle regular block with named params (only for `always` style)
        if let Some(block_params) = params.as_block_parameters_node() {
            self.check_named_block(source, &block_node, &block_params, style, diagnostics);
        }
    }
}

impl ItBlockParameter {
    /// Check implicit `it` blocks (ItParametersNode).
    /// - allow_single_line: flag multi-line blocks
    /// - disallow: flag all `it` references
    /// - only_numbered_parameters / always: no action on itblocks
    fn check_itblock(
        &self,
        source: &SourceFile,
        call_node: &ruby_prism::Node<'_>,
        block_node: &ruby_prism::BlockNode<'_>,
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match style {
            "allow_single_line" => {
                // Use the call_node (full expression) range for multi-line check,
                // matching RuboCop's `node.single_line?` on the itblock node which
                // includes the receiver/method chain. This correctly flags cases like:
                //   foo
                //     .bar { it }
                // where the block itself is single-line but the full expression is multi-line.
                let expr_loc = call_node.location();
                let (start_line, _) = source.offset_to_line_col(expr_loc.start_offset());
                let (end_line, _) =
                    source.offset_to_line_col(expr_loc.end_offset().saturating_sub(1));
                if start_line == end_line {
                    return; // single-line, OK
                }
                // Offense on the call node (covers `block do`)
                let loc = call_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid using `it` block parameter for multi-line blocks.".to_string(),
                ));
            }
            "disallow" => {
                // Flag each `it` reference in the block body
                if let Some(body) = block_node.body() {
                    let mut finder = ItReferenceFinder {
                        locations: Vec::new(),
                    };
                    finder.visit(&body);
                    for (start_offset, _end_offset) in finder.locations {
                        let (line, column) = source.offset_to_line_col(start_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid using `it` block parameter.".to_string(),
                        ));
                    }
                }
            }
            // only_numbered_parameters, always: no offense for itblocks
            _ => {}
        }
    }

    /// Check lambda nodes (`-> { _1 }`, `-> { it }`, `-> { |x| x }`).
    /// In Prism, lambdas are `LambdaNode` (not `CallNode`), but RuboCop's
    /// `on_numblock`/`on_itblock`/`on_block` callbacks fire for lambdas too.
    fn check_lambda(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        lambda: &ruby_prism::LambdaNode<'_>,
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let params = match lambda.parameters() {
            Some(p) => p,
            None => return,
        };

        // Handle itblock (implicit `it` parameter in lambda)
        if params.as_it_parameters_node().is_some() {
            // For allow_single_line, check if the lambda is multi-line
            match style {
                "allow_single_line" => {
                    let loc = node.location();
                    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
                    let (end_line, _) =
                        source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                    if start_line == end_line {
                        return;
                    }
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid using `it` block parameter for multi-line blocks.".to_string(),
                    ));
                }
                "disallow" => {
                    if let Some(body) = lambda.body() {
                        let mut finder = ItReferenceFinder {
                            locations: Vec::new(),
                        };
                        finder.visit(&body);
                        for (start_offset, _end_offset) in finder.locations {
                            let (line, column) = source.offset_to_line_col(start_offset);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Avoid using `it` block parameter.".to_string(),
                            ));
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Handle numblock (numbered parameters in lambda)
        if let Some(numbered) = params.as_numbered_parameters_node() {
            if style == "disallow" {
                return;
            }
            if numbered.maximum() != 1 {
                return;
            }
            if let Some(body) = lambda.body() {
                let mut finder = NumberedParamFinder {
                    locations: Vec::new(),
                };
                finder.visit(&body);
                for (start_offset, _end_offset) in finder.locations {
                    let (line, column) = source.offset_to_line_col(start_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `it` block parameter.".to_string(),
                    ));
                }
            }
            return;
        }

        // Handle regular lambda with named params (only for `always` style)
        if style != "always" {
            return;
        }
        if let Some(block_params) = params.as_block_parameters_node() {
            let parameters = match block_params.parameters() {
                Some(p) => p,
                None => return,
            };
            let requireds = parameters.requireds();
            if requireds.len() != 1 {
                return;
            }
            if !parameters.optionals().is_empty()
                || parameters.rest().is_some()
                || !parameters.posts().is_empty()
                || !parameters.keywords().is_empty()
                || parameters.keyword_rest().is_some()
                || parameters.block().is_some()
            {
                return;
            }
            let param = match requireds.iter().next() {
                Some(p) => p,
                None => return,
            };
            let req_param = match param.as_required_parameter_node() {
                Some(rp) => rp,
                None => return,
            };
            let param_name = req_param.name();
            let body = match lambda.body() {
                Some(b) => b,
                None => return,
            };
            let mut finder = NamedParamFinder {
                name: param_name.as_slice(),
                locations: Vec::new(),
            };
            finder.visit(&body);
            for (start_offset, _end_offset) in finder.locations {
                let (line, column) = source.offset_to_line_col(start_offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `it` block parameter.".to_string(),
                ));
            }
        }
    }

    /// Check numbered parameter blocks (NumberedParametersNode).
    /// Flag `_1` usage when style is allow_single_line, only_numbered_parameters, or always.
    fn check_numblock(
        &self,
        source: &SourceFile,
        block_node: &ruby_prism::BlockNode<'_>,
        numbered: &ruby_prism::NumberedParametersNode<'_>,
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // disallow style doesn't flag numbered params
        if style == "disallow" {
            return;
        }

        // Only flag when maximum == 1 (only _1 is used, no _2+)
        if numbered.maximum() != 1 {
            return;
        }

        if let Some(body) = block_node.body() {
            let mut finder = NumberedParamFinder {
                locations: Vec::new(),
            };
            finder.visit(&body);
            for (start_offset, _end_offset) in finder.locations {
                let (line, column) = source.offset_to_line_col(start_offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `it` block parameter.".to_string(),
                ));
            }
        }
    }

    /// Check regular blocks with named parameters (only for `always` style).
    /// Flags single-arg blocks where `it` could be used instead.
    fn check_named_block(
        &self,
        source: &SourceFile,
        block_node: &ruby_prism::BlockNode<'_>,
        block_params: &ruby_prism::BlockParametersNode<'_>,
        style: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if style != "always" {
            return;
        }

        let parameters = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        // Must have exactly one required parameter and no other params
        let requireds = parameters.requireds();
        if requireds.len() != 1 {
            return;
        }
        if !parameters.optionals().is_empty()
            || parameters.rest().is_some()
            || !parameters.posts().is_empty()
            || !parameters.keywords().is_empty()
            || parameters.keyword_rest().is_some()
            || parameters.block().is_some()
        {
            return;
        }

        let param = match requireds.iter().next() {
            Some(p) => p,
            None => return,
        };

        let req_param = match param.as_required_parameter_node() {
            Some(rp) => rp,
            None => return,
        };

        let param_name = req_param.name();

        // Need a body to find references
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        // Find all references to this parameter name in the body
        let mut finder = NamedParamFinder {
            name: param_name.as_slice(),
            locations: Vec::new(),
        };
        finder.visit(&body);

        for (start_offset, _end_offset) in finder.locations {
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `it` block parameter.".to_string(),
            ));
        }
    }
}

/// Finds `ItLocalVariableReadNode` references (implicit `it`) in a block body.
struct ItReferenceFinder {
    locations: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for ItReferenceFinder {
    fn visit_it_local_variable_read_node(
        &mut self,
        node: &ruby_prism::ItLocalVariableReadNode<'pr>,
    ) {
        let loc = node.location();
        self.locations.push((loc.start_offset(), loc.end_offset()));
    }

    // Don't descend into nested blocks
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

/// Finds `_1` references (LocalVariableReadNode with name "_1") in a block body.
struct NumberedParamFinder {
    locations: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for NumberedParamFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == b"_1" {
            let loc = node.location();
            self.locations.push((loc.start_offset(), loc.end_offset()));
        }
    }

    // Don't descend into nested blocks
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

/// Finds references to a named local variable in a block body.
struct NamedParamFinder<'a> {
    name: &'a [u8],
    locations: Vec<(usize, usize)>,
}

impl<'pr, 'a> Visit<'pr> for NamedParamFinder<'a> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.name {
            let loc = node.location();
            self.locations.push((loc.start_offset(), loc.end_offset()));
        }
    }

    // Don't descend into nested blocks
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;

    fn ruby34_config() -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.4)),
        );
        config
    }

    #[test]
    fn offense_with_ruby34() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &ItBlockParameter,
            include_bytes!("../../../tests/fixtures/cops/style/it_block_parameter/offense.rb"),
            ruby34_config(),
        );
    }

    #[test]
    fn no_offense() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &ItBlockParameter,
            include_bytes!("../../../tests/fixtures/cops/style/it_block_parameter/no_offense.rb"),
            ruby34_config(),
        );
    }

    #[test]
    fn no_offense_below_ruby34() {
        // Default Ruby version (2.7) — cop should be completely silent
        crate::testutil::assert_cop_no_offenses_full(
            &ItBlockParameter,
            include_bytes!("../../../tests/fixtures/cops/style/it_block_parameter/offense.rb"),
        );
    }
}
