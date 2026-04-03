use ruby_prism::Visit;

use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP=115 investigation (2026-03-13): nitrocop's `NonForwardingRefFinder` had two bugs:
///
/// 1. **Missing write node types**: Only tracked `LocalVariableWriteNode` (simple `x = ...`),
///    missing `LocalVariableTargetNode` (multi-write `a, b, c = ...`), `LocalVariableOrWriteNode`
///    (`x ||= ...`), `LocalVariableAndWriteNode` (`x &&= ...`), and
///    `LocalVariableOperatorWriteNode` (`x += ...`). This caused params reassigned via
///    multi-assignment or `||=` to not be detected as referenced.
///
/// 2. **Overly broad forwarding context**: Marked the entire subtree of splat/kwsplat/block_pass
///    as "forwarding context", but RuboCop only checks the immediate parent. `*options[:cipher]`
///    should mark `options` as referenced since it's used as a hash, not forwarded.
///
/// Fix: Added visitors for all write node types. Changed splat/kwsplat/block_pass visitors to
/// only skip direct lvar children (matching RuboCop's immediate-parent check).
///
/// FP=10, FN=71 investigation (2026-03-24): Two issues found:
///
/// 1. **Missing yield handling**: `SendClassifier` only visited `CallNode` and `SuperNode`,
///    missing `YieldNode`. Forwarding via `yield(*args)` was not detected.
///    Fix: Added `visit_yield_node` to `SendClassifier`.
///
/// 2. **Missing block-ancestor check for anonymous forwarding**: For Ruby < 3.4, anonymous
///    forwarding (`*`, `**`, `&`) inside a block is a syntax error (Ruby 3.3.0 bug). RuboCop's
///    `all_forwarding_offenses_correctable?` suppresses all anonymous forwarding if ANY send
///    node is inside a block. Our cop didn't track block nesting depth.
///    Fix: Added `block_depth` tracking to `SendClassifier` and `inside_block` flag to
///    `SendClassification`. Skip anonymous forwarding when any send is inside a block
///    for Ruby < 3.4.
///
/// FP=39 investigation (2026-04-03): Two Prism-specific false-positive buckets remained:
///
/// 1. **Spacing-sensitive redundant names**: RuboCop compares the full parameter source (`*args`,
///    `&block`), so spaced forms like `* args` and `& block` are not treated as redundant.
///    nitrocop only compared bare local names and incorrectly flagged them.
///
/// 2. **Overly broad anonymous-to-`...` detection**: `try_report_all_anonymous_forwarding`
///    treated any send containing anonymous `*`, `**`, and `&` as `...`-eligible. Prism
///    RuboCop is narrower: explicit keyword params in the def, or extra/interleaved keyword
///    arguments in the call (for example `foo(*, extra:, **, &)`) must not be collapsed to
///    `...`.
pub struct ArgumentsForwarding;

const FORWARDING_MSG: &str = "Use shorthand syntax `...` for arguments forwarding.";
const ARGS_MSG: &str = "Use anonymous positional arguments forwarding (`*`).";
const KWARGS_MSG: &str = "Use anonymous keyword arguments forwarding (`**`).";
const BLOCK_MSG: &str = "Use anonymous block arguments forwarding (`&`).";

impl Cop for ArgumentsForwarding {
    fn name(&self) -> &'static str {
        "Style/ArgumentsForwarding"
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
        let allow_only_rest = config.get_bool("AllowOnlyRestArgument", true);
        let use_anonymous = config.get_bool("UseAnonymousForwarding", true);
        let redundant_rest = config
            .get_string_array("RedundantRestArgumentNames")
            .unwrap_or_else(|| vec!["args".to_string(), "arguments".to_string()]);
        let redundant_kw_rest = config
            .get_string_array("RedundantKeywordRestArgumentNames")
            .unwrap_or_else(|| {
                vec![
                    "kwargs".to_string(),
                    "options".to_string(),
                    "opts".to_string(),
                ]
            });
        let redundant_block = config
            .get_string_array("RedundantBlockArgumentNames")
            .unwrap_or_else(|| vec!["blk".to_string(), "block".to_string(), "proc".to_string()]);

        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(3.4);
        if ruby_version < 2.7 {
            return;
        }

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Already using ... forwarding
        if let Some(kw_rest) = params.keyword_rest() {
            if kw_rest.as_forwarding_parameter_node().is_some() {
                return;
            }
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Check for all-anonymous forwarding: def foo(*, **, &) → def foo(...)
        if self.try_report_all_anonymous_forwarding(
            source,
            &params,
            &body,
            &def_node,
            _parse_result,
            ruby_version,
            diagnostics,
        ) {
            return;
        }

        // Extract param names — these represent the method's actual parameters
        let rest_name = extract_rest_param_name(&params);
        let kwrest_name = extract_kwrest_param_name(&params);
        let block_name = extract_block_param_name(&params);

        // At least one forwardable param must exist
        if rest_name.is_none() && kwrest_name.is_none() && block_name.is_none() {
            return;
        }

        // Determine which param names are "redundant" (meaningless names)
        let rest_is_redundant = rest_name
            .as_ref()
            .is_some_and(|n| redundant_named_param_source(source, n, "*", &redundant_rest));
        let kwrest_is_redundant = kwrest_name
            .as_ref()
            .is_some_and(|n| redundant_named_param_source(source, n, "**", &redundant_kw_rest));
        let block_is_redundant = block_name
            .as_ref()
            .is_some_and(|n| redundant_named_param_source(source, n, "&", &redundant_block));

        // Collect non-forwarding references
        let referenced = non_forwarding_references(&body);

        let rest_referenced = rest_name.as_ref().is_some_and(|n| {
            referenced.contains(&String::from_utf8_lossy(n.as_slice()).to_string())
        });
        let kwrest_referenced = kwrest_name.as_ref().is_some_and(|n| {
            referenced.contains(&String::from_utf8_lossy(n.as_slice()).to_string())
        });
        let block_referenced = block_name.as_ref().is_some_and(|n| {
            referenced.contains(&String::from_utf8_lossy(n.as_slice()).to_string())
        });

        // The "forwardable" params are those that exist, are not referenced, and we can detect
        let fwd_rest = rest_name.as_ref().filter(|_| !rest_referenced);
        let fwd_kwrest = kwrest_name.as_ref().filter(|_| !kwrest_referenced);
        let fwd_block = block_name.as_ref().filter(|_| !block_referenced);

        // Find all forwarding call sites
        let send_classifications = classify_send_nodes(&body, fwd_rest, fwd_kwrest, fwd_block);

        if send_classifications.is_empty() {
            return;
        }

        // Determine if we have additional (non-forwardable) params
        let has_additional_params = !params.requireds().is_empty()
            || !params.optionals().is_empty()
            || !params.keywords().is_empty()
            || params.posts().iter().next().is_some();
        let has_optarg = !params.optionals().is_empty();
        let has_kwargs = !params.keywords().is_empty();

        // Try ... forwarding first
        let can_forward_all = can_use_forward_all(
            &send_classifications,
            // For ..., any referenced param blocks it entirely
            rest_referenced,
            kwrest_referenced,
            block_referenced,
            // Whether the method actually has these params
            rest_name.is_some(),
            kwrest_name.is_some(),
            block_name.is_some(),
            has_additional_params,
            has_optarg,
            has_kwargs,
            allow_only_rest,
            rest_is_redundant,
            kwrest_is_redundant,
            block_is_redundant,
            ruby_version,
        );

        if can_forward_all {
            // Report ... forwarding on the def's forwardable params
            let first_forwardable_offset = [
                fwd_rest.map(|n| n.start()),
                fwd_kwrest.map(|n| n.start()),
                fwd_block.map(|n| n.start()),
            ]
            .iter()
            .filter_map(|o| *o)
            .min();

            if let Some(offset) = first_forwardable_offset {
                let (line, column) = source.offset_to_line_col(offset);
                diagnostics.push(self.diagnostic(source, line, column, FORWARDING_MSG.to_string()));
            }

            for sc in &send_classifications {
                if let Some(offset) = sc.forwarding_start_offset() {
                    let (line, col) = source.offset_to_line_col(offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        FORWARDING_MSG.to_string(),
                    ));
                }
            }
        } else if ruby_version >= 3.2 && use_anonymous {
            // Ruby 3.3.0 had a bug where anonymous forwarding inside a block was a
            // syntax error. For Ruby < 3.4, if ANY classified send is inside a block,
            // skip all anonymous forwarding (matching RuboCop's
            // `all_forwarding_offenses_correctable?`).
            if ruby_version < 3.4 && send_classifications.iter().any(|sc| sc.inside_block) {
                return;
            }

            // Anonymous forwarding: report each forwardable arg with redundant name individually
            self.report_anonymous_forwarding(
                source,
                &send_classifications,
                fwd_rest.filter(|_| rest_is_redundant),
                fwd_kwrest.filter(|_| kwrest_is_redundant),
                fwd_block.filter(|_| block_is_redundant),
                diagnostics,
            );
        }
    }
}

impl ArgumentsForwarding {
    fn report_anonymous_forwarding(
        &self,
        source: &SourceFile,
        send_classifications: &[SendClassification],
        rest_name: Option<&ParamName>,
        kwrest_name: Option<&ParamName>,
        block_name: Option<&ParamName>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let any_forwards_rest = rest_name.is_some()
            && send_classifications
                .iter()
                .any(|sc| sc.forwards_rest.is_some());
        let any_forwards_kwrest = kwrest_name.is_some()
            && send_classifications
                .iter()
                .any(|sc| sc.forwards_kwrest.is_some());
        let any_forwards_block = block_name.is_some()
            && send_classifications
                .iter()
                .any(|sc| sc.forwards_block.is_some());

        // Report on def params
        if any_forwards_rest {
            if let Some(name) = rest_name {
                let (line, col) = source.offset_to_line_col(name.start());
                diagnostics.push(self.diagnostic(source, line, col, ARGS_MSG.to_string()));
            }
        }
        if any_forwards_kwrest {
            if let Some(name) = kwrest_name {
                let (line, col) = source.offset_to_line_col(name.start());
                diagnostics.push(self.diagnostic(source, line, col, KWARGS_MSG.to_string()));
            }
        }
        if any_forwards_block {
            if let Some(name) = block_name {
                let (line, col) = source.offset_to_line_col(name.start());
                diagnostics.push(self.diagnostic(source, line, col, BLOCK_MSG.to_string()));
            }
        }

        // Report on each call site
        for sc in send_classifications {
            if any_forwards_rest {
                if let Some(offset) = sc.forwards_rest {
                    let (line, col) = source.offset_to_line_col(offset);
                    diagnostics.push(self.diagnostic(source, line, col, ARGS_MSG.to_string()));
                }
            }
            if any_forwards_kwrest {
                if let Some(offset) = sc.forwards_kwrest {
                    let (line, col) = source.offset_to_line_col(offset);
                    diagnostics.push(self.diagnostic(source, line, col, KWARGS_MSG.to_string()));
                }
            }
            if any_forwards_block {
                if let Some(offset) = sc.forwards_block {
                    let (line, col) = source.offset_to_line_col(offset);
                    diagnostics.push(self.diagnostic(source, line, col, BLOCK_MSG.to_string()));
                }
            }
        }
    }
}

impl ArgumentsForwarding {
    /// Check if the def uses all-anonymous forwarding (*, **, &) and can be replaced with `...`.
    /// Returns true if offenses were reported (caller should return early).
    #[allow(clippy::too_many_arguments)]
    fn try_report_all_anonymous_forwarding(
        &self,
        source: &SourceFile,
        params: &ruby_prism::ParametersNode<'_>,
        body: &ruby_prism::Node<'_>,
        def_node: &ruby_prism::DefNode<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        ruby_version: f64,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> bool {
        // Anonymous forwarding requires Ruby >= 3.2
        if ruby_version < 3.2 {
            return false;
        }

        // Prism RuboCop does not collapse anonymous forwarding to `...` when the def
        // signature includes explicit keyword params alongside `*`, `**`, and `&`.
        if !params.keywords().is_empty() {
            return false;
        }

        // All three anonymous params must exist
        let anon_rest = match get_anonymous_rest_offset(params) {
            Some(o) => o,
            None => return false,
        };
        let anon_kwrest = match get_anonymous_kwrest_offset(params) {
            Some(o) => o,
            None => return false,
        };
        let _anon_block = match get_anonymous_block_offset(params) {
            Some(o) => o,
            None => return false,
        };

        // RuboCop's `ruby_32_only_anonymous_forwarding?` checks if the send node
        // has ANY block ancestor (including blocks enclosing the def). If a def is
        // inside a block (e.g., RSpec.describe do...end), the anonymous-to-...
        // suggestion is suppressed.
        if is_inside_block_node(&parse_result.node(), def_node.location().start_offset()) {
            return false;
        }

        // Find all call sites that use anonymous forwarding
        let classifications = classify_anonymous_sends(body, ruby_version);
        if classifications.is_empty() {
            return false;
        }

        // All classified sends must forward all three anonymous args
        if !classifications
            .iter()
            .all(|c| c.has_anon_rest && c.has_anon_kwrest && c.has_anon_block)
        {
            return false;
        }

        // Report offense on def params (first forwardable = anon_rest since * comes first)
        let first_offset = anon_rest.min(anon_kwrest);
        let (line, col) = source.offset_to_line_col(first_offset);
        diagnostics.push(self.diagnostic(source, line, col, FORWARDING_MSG.to_string()));

        // Report offense on each call site
        for c in &classifications {
            if let Some(offset) = c.first_anon_offset() {
                let (line, col) = source.offset_to_line_col(offset);
                diagnostics.push(self.diagnostic(source, line, col, FORWARDING_MSG.to_string()));
            }
        }

        true
    }
}

/// Classification of anonymous forwarding in a single call site
struct AnonSendClassification {
    has_anon_rest: bool,
    has_anon_kwrest: bool,
    has_anon_block: bool,
    anon_rest_offset: Option<usize>,
    anon_kwrest_offset: Option<usize>,
    anon_block_offset: Option<usize>,
}

impl AnonSendClassification {
    fn first_anon_offset(&self) -> Option<usize> {
        [
            self.anon_rest_offset,
            self.anon_kwrest_offset,
            self.anon_block_offset,
        ]
        .iter()
        .filter_map(|o| *o)
        .min()
    }
}

fn classify_anonymous_sends(
    body: &ruby_prism::Node<'_>,
    ruby_version: f64,
) -> Vec<AnonSendClassification> {
    let mut classifier = AnonSendClassifier {
        results: Vec::new(),
        block_depth: 0,
        ruby_version,
    };
    classifier.visit(body);
    classifier.results
}

struct AnonSendClassifier {
    results: Vec<AnonSendClassification>,
    block_depth: usize,
    ruby_version: f64,
}

impl AnonSendClassifier {
    fn classify_anon_call(
        &self,
        arguments: Option<ruby_prism::ArgumentsNode<'_>>,
        block: Option<ruby_prism::Node<'_>>,
    ) -> Option<AnonSendClassification> {
        // For Ruby < 3.4, anonymous forwarding inside a block is a syntax error
        if self.ruby_version < 3.4 && self.block_depth > 0 {
            return None;
        }

        let args = arguments?;
        let arg_nodes: Vec<_> = args.arguments().iter().collect();

        let rest_index = arg_nodes
            .iter()
            .position(|arg| anonymous_rest_offset(arg).is_some())?;

        let rest_offset = anonymous_rest_offset(&arg_nodes[rest_index])?;
        let kw_index = rest_index + 1;
        let kw_arg = arg_nodes.get(kw_index)?;
        let kw_offset = anonymous_kwrest_offset(kw_arg)?;

        let block_index = kw_index + 1;
        let inline_block_offset = arg_nodes
            .get(block_index)
            .and_then(anonymous_block_argument_offset);
        let separate_block_offset = anonymous_block_node_offset(block);

        let block_offset = if let Some(offset) = inline_block_offset {
            if block_index + 1 != arg_nodes.len() {
                return None;
            }
            offset
        } else {
            if block_index != arg_nodes.len() {
                return None;
            }
            separate_block_offset?
        };

        Some(AnonSendClassification {
            has_anon_rest: true,
            has_anon_kwrest: true,
            has_anon_block: true,
            anon_rest_offset: Some(rest_offset),
            anon_kwrest_offset: Some(kw_offset),
            anon_block_offset: Some(block_offset),
        })
    }
}

impl<'pr> Visit<'pr> for AnonSendClassifier {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(sc) = self.classify_anon_call(node.arguments(), node.block()) {
            self.results.push(sc);
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        if let Some(sc) = self.classify_anon_call(node.arguments(), node.block()) {
            self.results.push(sc);
        }
        ruby_prism::visit_super_node(self, node);
    }

    fn visit_yield_node(&mut self, node: &ruby_prism::YieldNode<'pr>) {
        if let Some(sc) = self.classify_anon_call(node.arguments(), None) {
            self.results.push(sc);
        }
        ruby_prism::visit_yield_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.block_depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.block_depth -= 1;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.block_depth += 1;
        ruby_prism::visit_lambda_node(self, node);
        self.block_depth -= 1;
    }

    // Don't recurse into nested defs
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

/// Check if a given byte offset falls inside any BlockNode or LambdaNode in the AST.
/// This mirrors RuboCop's `send_node.each_ancestor(:any_block).any?` check which
/// suppresses anonymous-to-... forwarding when the def is inside a block.
fn is_inside_block_node(root: &ruby_prism::Node<'_>, target_offset: usize) -> bool {
    let mut checker = BlockAncestorChecker {
        target_offset,
        found: false,
    };
    checker.visit(root);
    checker.found
}

struct BlockAncestorChecker {
    target_offset: usize,
    found: bool,
}

impl<'pr> Visit<'pr> for BlockAncestorChecker {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        if self.target_offset > start && self.target_offset < end {
            self.found = true;
            return;
        }
        if !self.found {
            ruby_prism::visit_block_node(self, node);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let start = node.location().start_offset();
        let end = node.location().end_offset();
        if self.target_offset > start && self.target_offset < end {
            self.found = true;
            return;
        }
        if !self.found {
            ruby_prism::visit_lambda_node(self, node);
        }
    }
}

/// A parameter name with its source location
struct ParamName {
    name: Vec<u8>,
    start_offset: usize,
    end_offset: usize,
}

impl ParamName {
    fn as_slice(&self) -> &[u8] {
        &self.name
    }

    fn start(&self) -> usize {
        self.start_offset
    }
}

fn redundant_named_param_source(
    source: &SourceFile,
    param: &ParamName,
    prefix: &str,
    redundant_names: &[String],
) -> bool {
    let Some(param_source) = source.try_byte_slice(param.start_offset, param.end_offset) else {
        return false;
    };

    redundant_names
        .iter()
        .any(|name| param_source == format!("{prefix}{name}"))
}

/// Get the start offset of an anonymous `*` rest param (no name).
fn get_anonymous_rest_offset(params: &ruby_prism::ParametersNode<'_>) -> Option<usize> {
    let rest = params.rest()?;
    let rest_param = rest.as_rest_parameter_node()?;
    if rest_param.name().is_some() {
        return None;
    }
    Some(rest.location().start_offset())
}

/// Get the start offset of an anonymous `**` kwrest param (no name).
fn get_anonymous_kwrest_offset(params: &ruby_prism::ParametersNode<'_>) -> Option<usize> {
    let kw_rest = params.keyword_rest()?;
    let kw_rest_param = kw_rest.as_keyword_rest_parameter_node()?;
    if kw_rest_param.name().is_some() {
        return None;
    }
    Some(kw_rest.location().start_offset())
}

/// Get the start offset of an anonymous `&` block param (no name).
fn get_anonymous_block_offset(params: &ruby_prism::ParametersNode<'_>) -> Option<usize> {
    let block = params.block()?;
    if block.name().is_some() {
        return None;
    }
    Some(block.location().start_offset())
}

fn extract_rest_param_name(params: &ruby_prism::ParametersNode<'_>) -> Option<ParamName> {
    let rest = params.rest()?;
    let rest_param = rest.as_rest_parameter_node()?;
    let name = rest_param.name()?;
    Some(ParamName {
        name: name.as_slice().to_vec(),
        start_offset: rest.location().start_offset(),
        end_offset: rest.location().end_offset(),
    })
}

fn extract_kwrest_param_name(params: &ruby_prism::ParametersNode<'_>) -> Option<ParamName> {
    let kw_rest = params.keyword_rest()?;
    let kw_rest_param = kw_rest.as_keyword_rest_parameter_node()?;
    let name = kw_rest_param.name()?;
    Some(ParamName {
        name: name.as_slice().to_vec(),
        start_offset: kw_rest.location().start_offset(),
        end_offset: kw_rest.location().end_offset(),
    })
}

fn extract_block_param_name(params: &ruby_prism::ParametersNode<'_>) -> Option<ParamName> {
    let block = params.block()?;
    let name = block.name()?;
    Some(ParamName {
        name: name.as_slice().to_vec(),
        start_offset: block.location().start_offset(),
        end_offset: block.location().end_offset(),
    })
}

fn anonymous_rest_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    let splat = node.as_splat_node()?;
    if splat.expression().is_some() {
        return None;
    }
    Some(splat.location().start_offset())
}

fn anonymous_kwrest_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    let elements = node
        .as_keyword_hash_node()
        .map(|h| h.elements())
        .or_else(|| node.as_hash_node().map(|h| h.elements()))?;

    let mut elements = elements.iter();
    let assoc_splat = elements.next()?.as_assoc_splat_node()?;
    if elements.next().is_some() || assoc_splat.value().is_some() {
        return None;
    }

    Some(assoc_splat.location().start_offset())
}

fn anonymous_block_argument_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    let block_arg = node.as_block_argument_node()?;
    if block_arg.expression().is_some() {
        return None;
    }
    Some(block_arg.location().start_offset())
}

fn anonymous_block_node_offset(node: Option<ruby_prism::Node<'_>>) -> Option<usize> {
    anonymous_block_argument_offset(&node?)
}

/// Classification of what a single call site forwards
struct SendClassification {
    forwards_rest: Option<usize>,
    forwards_kwrest: Option<usize>,
    forwards_block: Option<usize>,
    /// Whether this call is inside a block (relevant for Ruby < 3.4 anonymous forwarding)
    inside_block: bool,
}

impl SendClassification {
    fn forwarding_start_offset(&self) -> Option<usize> {
        [
            self.forwards_rest,
            self.forwards_kwrest,
            self.forwards_block,
        ]
        .iter()
        .filter_map(|o| *o)
        .min()
    }
}

/// Check if we can suggest `...` for all forwarding calls
#[allow(clippy::too_many_arguments)]
fn can_use_forward_all(
    send_classifications: &[SendClassification],
    rest_referenced: bool,
    kwrest_referenced: bool,
    block_referenced: bool,
    has_rest: bool,
    has_kwrest: bool,
    has_block: bool,
    has_additional_params: bool,
    has_optarg: bool,
    has_kwargs: bool,
    allow_only_rest: bool,
    rest_is_redundant: bool,
    kwrest_is_redundant: bool,
    block_is_redundant: bool,
    ruby_version: f64,
) -> bool {
    // ... forwarding replaces ALL of *rest, **kwrest, &block at once.
    // If ANY of them is referenced outside forwarding, ... is not possible.
    if rest_referenced || kwrest_referenced || block_referenced {
        return false;
    }

    // With keyword params (kwarg/kwoptarg), ... is not possible
    if has_kwargs {
        return false;
    }

    // Need at least one of *rest or **kwrest for ... forwarding
    // (block-only can't use ..., it should use anonymous & instead)
    if !has_rest && !has_kwrest {
        return false;
    }

    // All names must be redundant for ... (when AllowOnlyRestArgument is true)
    if allow_only_rest {
        if has_rest && !rest_is_redundant {
            return false;
        }
        if has_kwrest && !kwrest_is_redundant {
            return false;
        }
        if has_block && !block_is_redundant {
            return false;
        }
    }

    // ... also forwards blocks, so if block exists it must be forwarded
    if has_block {
        let all_forward_block = send_classifications
            .iter()
            .all(|sc| sc.forwards_block.is_some());
        if !all_forward_block && allow_only_rest {
            return false;
        }
    } else if allow_only_rest {
        // No block param — ... would also forward blocks which changes semantics
        return false;
    }

    // For Ruby >= 3.2, RuboCop prefers anonymous forwarding (*, **, &) over ...
    // unless BOTH rest and kwrest are present and forwarded together.
    // If only *rest is present (no **kwrest), prefer * over ...
    if ruby_version >= 3.2 {
        if has_rest && has_kwrest {
            let all_forward_both = send_classifications
                .iter()
                .all(|sc| sc.forwards_rest.is_some() && sc.forwards_kwrest.is_some());
            if !all_forward_both {
                return false;
            }
        } else {
            // Only one of rest/kwrest — prefer individual anonymous forwarding
            return false;
        }
    }

    // All sends must forward the rest args (if method has them)
    if has_rest {
        let all_forward_rest = send_classifications
            .iter()
            .all(|sc| sc.forwards_rest.is_some());
        if !all_forward_rest {
            return false;
        }
    }

    // All sends must forward kwrest args (if method has them)
    if has_kwrest {
        let all_forward_kwrest = send_classifications
            .iter()
            .all(|sc| sc.forwards_kwrest.is_some());
        if !all_forward_kwrest {
            return false;
        }
    }

    // Additional params compatibility
    if has_additional_params {
        if ruby_version < 3.0 {
            return false;
        }
        if has_optarg && ruby_version < 3.2 {
            return false;
        }
    }

    true
}

fn classify_send_nodes(
    body: &ruby_prism::Node<'_>,
    rest_name: Option<&ParamName>,
    kwrest_name: Option<&ParamName>,
    block_name: Option<&ParamName>,
) -> Vec<SendClassification> {
    let mut finder = SendClassifier {
        rest_name: rest_name.map(|n| n.as_slice().to_vec()),
        kwrest_name: kwrest_name.map(|n| n.as_slice().to_vec()),
        block_name: block_name.map(|n| n.as_slice().to_vec()),
        results: Vec::new(),
        block_depth: 0,
    };
    finder.visit(body);
    finder.results
}

struct SendClassifier {
    rest_name: Option<Vec<u8>>,
    kwrest_name: Option<Vec<u8>>,
    block_name: Option<Vec<u8>>,
    results: Vec<SendClassification>,
    /// Depth of block nesting (> 0 means inside a block)
    block_depth: usize,
}

impl SendClassifier {
    fn classify_call(
        &self,
        arguments: Option<ruby_prism::ArgumentsNode<'_>>,
        block: Option<ruby_prism::Node<'_>>,
    ) -> Option<SendClassification> {
        let mut forwards_rest = None;
        let mut forwards_kwrest = None;
        let mut forwards_block = None;

        if let Some(args) = &arguments {
            for arg in args.arguments().iter() {
                // Check for *rest forwarding
                if let Some(splat) = arg.as_splat_node() {
                    if let Some(ref rest_name) = self.rest_name {
                        if let Some(expr) = splat.expression() {
                            if let Some(lvar) = expr.as_local_variable_read_node() {
                                if lvar.name().as_slice() == rest_name.as_slice() {
                                    forwards_rest = Some(splat.location().start_offset());
                                }
                            }
                        }
                    }
                }
                // Check for **kwrest forwarding (inside a keyword hash node or explicit hash in args)
                let hash_elements = arg
                    .as_keyword_hash_node()
                    .map(|h| h.elements())
                    .or_else(|| arg.as_hash_node().map(|h| h.elements()));
                if let Some(elements) = hash_elements {
                    if let Some(ref kw_name) = self.kwrest_name {
                        for elem in elements.iter() {
                            if let Some(assoc_splat) = elem.as_assoc_splat_node() {
                                if let Some(expr) = assoc_splat.value() {
                                    if let Some(lvar) = expr.as_local_variable_read_node() {
                                        if lvar.name().as_slice() == kw_name.as_slice() {
                                            forwards_kwrest =
                                                Some(assoc_splat.location().start_offset());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Check for &block in arguments list
                if let Some(block_arg) = arg.as_block_argument_node() {
                    if let Some(ref blk_name) = self.block_name {
                        if let Some(expr) = block_arg.expression() {
                            if let Some(lvar) = expr.as_local_variable_read_node() {
                                if lvar.name().as_slice() == blk_name.as_slice() {
                                    forwards_block = Some(block_arg.location().start_offset());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check block node (separate from arguments)
        if forwards_block.is_none() {
            if let Some(block_node) = block {
                if let Some(block_arg) = block_node.as_block_argument_node() {
                    if let Some(ref blk_name) = self.block_name {
                        if let Some(expr) = block_arg.expression() {
                            if let Some(lvar) = expr.as_local_variable_read_node() {
                                if lvar.name().as_slice() == blk_name.as_slice() {
                                    forwards_block = Some(block_arg.location().start_offset());
                                }
                            }
                        }
                    }
                }
            }
        }

        if forwards_rest.is_some() || forwards_kwrest.is_some() || forwards_block.is_some() {
            Some(SendClassification {
                forwards_rest,
                forwards_kwrest,
                forwards_block,
                inside_block: self.block_depth > 0,
            })
        } else {
            None
        }
    }
}

impl<'pr> Visit<'pr> for SendClassifier {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(sc) = self.classify_call(node.arguments(), node.block()) {
            self.results.push(sc);
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        if let Some(sc) = self.classify_call(node.arguments(), node.block()) {
            self.results.push(sc);
        }
        ruby_prism::visit_super_node(self, node);
    }

    fn visit_yield_node(&mut self, node: &ruby_prism::YieldNode<'pr>) {
        // yield has arguments but no block
        if let Some(sc) = self.classify_call(node.arguments(), None) {
            self.results.push(sc);
        }
        ruby_prism::visit_yield_node(self, node);
    }

    // Track block nesting depth for the Ruby 3.3 anonymous-forwarding-in-block bug
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.block_depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.block_depth -= 1;
    }

    // Lambda blocks also count
    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.block_depth += 1;
        ruby_prism::visit_lambda_node(self, node);
        self.block_depth -= 1;
    }

    // Don't recurse into nested defs
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

/// Find local variable names that are referenced outside of forwarding contexts
fn non_forwarding_references(node: &ruby_prism::Node<'_>) -> std::collections::HashSet<String> {
    let mut finder = NonForwardingRefFinder {
        referenced: std::collections::HashSet::new(),
    };
    finder.visit(node);
    finder.referenced
}

struct NonForwardingRefFinder {
    referenced: std::collections::HashSet<String>,
}

impl<'pr> Visit<'pr> for NonForwardingRefFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
        self.referenced.insert(name);
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
        self.referenced.insert(name);
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    // Multi-write target: `a, b, c = ...` — each LHS var is a LocalVariableTargetNode
    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
        self.referenced.insert(name);
    }

    // `block ||= ...`
    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
        self.referenced.insert(name);
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    // `block &&= ...`
    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
        self.referenced.insert(name);
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    // `x += 1`
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
        self.referenced.insert(name);
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    // For splat/kwsplat/block_pass, only the direct expression (immediate child lvar)
    // is a forwarding reference. Deeper nested lvars (e.g., `*options[:key]`) are NOT
    // forwarding — they use the variable as a hash/object, not as a splat forward.
    // RuboCop checks only the immediate parent of the lvar node.
    fn visit_splat_node(&mut self, node: &ruby_prism::SplatNode<'pr>) {
        if let Some(expr) = node.expression() {
            if expr.as_local_variable_read_node().is_some() {
                // Direct lvar child: this IS a forwarding use, skip it
                return;
            }
        }
        // Not a direct lvar — recurse normally (lvars inside will be marked as referenced)
        ruby_prism::visit_splat_node(self, node);
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        if let Some(expr) = node.value() {
            if expr.as_local_variable_read_node().is_some() {
                return;
            }
        }
        ruby_prism::visit_assoc_splat_node(self, node);
    }

    fn visit_block_argument_node(&mut self, node: &ruby_prism::BlockArgumentNode<'pr>) {
        if let Some(expr) = node.expression() {
            if expr.as_local_variable_read_node().is_some() {
                return;
            }
        }
        ruby_prism::visit_block_argument_node(self, node);
    }

    // Don't recurse into nested defs
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArgumentsForwarding, "cops/style/arguments_forwarding");

    #[test]
    fn detects_triple_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(*args, **opts, &block)\n  bar(*args, **opts, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect triple forwarding (def + call): {:?}",
            diags
        );
    }

    #[test]
    fn detects_super_forwarding() {
        use crate::testutil::run_cop_full;
        // Ruby 3.2+ with *args, &block (no **kwrest) → anonymous * and &
        let source = b"def foo(*args, &block)\n  super(*args, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            4,
            "should detect anonymous forwarding for * and & (def + call × 2): {:?}",
            diags
        );
    }

    #[test]
    fn no_false_positive_different_calls_non_redundant_names() {
        use crate::testutil::run_cop_full;
        // *items and &handler are not redundant names — cannot suggest anonymous forwarding
        let source = b"def foo(*items, &handler)\n  bar(*items)\n  baz(&handler)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            0,
            "should not detect when args have non-redundant names and forwarded to different calls: {:?}",
            diags
        );
    }

    #[test]
    fn detects_self_class_method_forwarding() {
        use crate::testutil::run_cop_full;
        // Ruby 3.2+ with *args, &block (no **kwrest) → anonymous * and &
        let source = b"def self.foo(*args, &block)\n  bar(*args, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            4,
            "should detect anonymous forwarding for * and & (def + call × 2): {:?}",
            diags
        );
    }

    #[test]
    fn detects_forwarding_without_kwargs() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(*args, **options, &block)\n  bar(*args, **options, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect forwarding with options (def + call): {:?}",
            diags
        );
    }

    #[test]
    fn args_referenced_but_block_still_flagged() {
        use crate::testutil::run_cop_full;
        // args is used as a local variable (args.first), so can't use ... or *
        // But &block is NOT referenced, so &block -> & is still flagged (Ruby 3.2+)
        let source = b"def foo(*args, &block)\n  bar(*args, &block)\n  args.first\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should flag &block when only args is referenced: {:?}",
            diags
        );
        assert!(diags[0].message.contains("(`&`)"));
    }

    #[test]
    fn block_referenced_but_args_still_flagged() {
        use crate::testutil::run_cop_full;
        // block is called directly, so can't use ... or &
        // But *args is NOT referenced, so *args -> * is still flagged (Ruby 3.2+)
        let source = b"def foo(*args, &block)\n  bar(*args, &block)\n  block.call\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should flag *args when only block is referenced: {:?}",
            diags
        );
        assert!(diags[0].message.contains("(`*`)"));
    }

    #[test]
    fn detects_super_with_triple_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(*args, **opts, &block)\n  super(*args, **opts, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect super with triple forwarding (def + call): {:?}",
            diags
        );
    }

    #[test]
    fn detects_anonymous_block_forwarding() {
        use crate::testutil::run_cop_full;
        // &block forwarding only — should suggest &
        let source = b"def foo(&block)\n  bar(&block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect anonymous block forwarding (def + call): {:?}",
            diags
        );
        assert!(
            diags[0].message.contains("(`&`)"),
            "message should mention &: {}",
            diags[0].message
        );
    }

    #[test]
    fn detects_anonymous_block_with_extra_positional() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(name, &block)\n  run(name, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect &block forwarding with extra positional: {:?}",
            diags
        );
    }

    #[test]
    fn detects_anonymous_with_leading_call_args() {
        use crate::testutil::run_cop_full;
        // def post(*args, &block) with extra args in call site — Ruby 3.2+ uses * and &
        let source = b"def post(*args, &block)\n  future_on(executor, *args, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            4,
            "should detect anonymous * and & forwarding: {:?}",
            diags
        );
        assert!(
            diags[0].message.contains("(`*`)"),
            "first message should mention *: {}",
            diags[0].message
        );
    }

    #[test]
    fn detects_forward_all_with_leading_call_args_triple() {
        use crate::testutil::run_cop_full;
        // def post(*args, **opts, &block) — has both rest+kwrest → ... forwarding
        let source =
            b"def post(*args, **opts, &block)\n  future_on(executor, *args, **opts, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect ... forwarding with leading call args: {:?}",
            diags
        );
        assert!(
            diags[0].message.contains("`...`"),
            "message should mention ...: {}",
            diags[0].message
        );
    }

    #[test]
    fn detects_forward_all_with_leading_def_and_call_args() {
        use crate::testutil::run_cop_full;
        let source = b"def method_missing(m, *args, **kwargs, &block)\n  @template.send(m, *args, **kwargs, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect ... forwarding with leading def and call args: {:?}",
            diags
        );
    }

    #[test]
    fn non_redundant_block_name_only_flags_rest() {
        use crate::testutil::run_cop_full;
        // &task is not in RedundantBlockArgumentNames, so no & or ... suggestion
        // But *args IS redundant, so *args -> * is flagged (Ruby 3.2+)
        let source = b"def post(*args, &task)\n  @executor&.post(*args, &task)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should flag *args but not &task: {:?}",
            diags
        );
        assert!(diags[0].message.contains("(`*`)"));
    }

    #[test]
    fn no_false_positive_all_non_redundant_names() {
        use crate::testutil::run_cop_full;
        // Neither *items nor &handler are in redundant lists
        let source = b"def post(*items, &handler)\n  @executor&.post(*items, &handler)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            0,
            "should not flag when all names are non-redundant: {:?}",
            diags
        );
    }

    #[test]
    fn detects_yield_rest_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(*args)\n  yield(*args)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect anonymous * forwarding in yield: {:?}",
            diags
        );
        assert!(diags[0].message.contains("(`*`)"));
    }

    #[test]
    fn detects_yield_kwrest_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(**kwargs)\n  yield(**kwargs)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect anonymous ** forwarding in yield: {:?}",
            diags
        );
        assert!(diags[0].message.contains("(`**`)"));
    }

    #[test]
    fn no_anonymous_forwarding_inside_block_ruby33() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        // Ruby 3.3: anonymous block forwarding inside a block is a syntax error
        let mut options = std::collections::HashMap::new();
        options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.3_f64)),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let source = b"def foo(*args, &block)\n  wrapper do\n    bar(*args, &block)\n  end\nend\n";
        let diags = run_cop_full_with_config(&ArgumentsForwarding, source, config);
        assert_eq!(
            diags.len(),
            0,
            "Ruby 3.3: should not flag anonymous forwarding inside blocks: {:?}",
            diags
        );
    }

    #[test]
    fn anonymous_forwarding_inside_block_ruby34() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        // Ruby 3.4: anonymous forwarding inside blocks is fixed
        let mut options = std::collections::HashMap::new();
        options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(3.4_f64)),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let source = b"def foo(*args, &block)\n  wrapper do\n    bar(*args, &block)\n  end\nend\n";
        let diags = run_cop_full_with_config(&ArgumentsForwarding, source, config);
        assert_eq!(
            diags.len(),
            4,
            "Ruby 3.4: should flag anonymous forwarding inside blocks: {:?}",
            diags
        );
    }

    #[test]
    fn no_anonymous_to_dots_forwarding_inside_block() {
        use crate::testutil::run_cop_full;
        // def with anonymous (*, **, &) inside a block (e.g. RSpec.describe do...end)
        // should NOT suggest ... because RuboCop suppresses this via block ancestor check
        let source = b"RSpec.describe do\n  def render_component(*, **, &)\n    render(described_class.new(*, **, &))\n  end\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            0,
            "should not suggest ... for anonymous forwarding when def is inside a block: {:?}",
            diags
        );
    }
}
