use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/SuperArguments: flags `super(args)` when the arguments match the
/// enclosing method's parameters exactly, since bare `super` already forwards
/// them.
///
/// ## Root causes of prior FP/FN:
/// - FP: super inside a block (do...end / { }) was flagged. RuboCop skips
///   super calls inside blocks because the block may be evaluated in a
///   different method context (e.g. define_method, DSL blocks).
/// - FN: `super(&block)` — Prism puts the `&block` argument in SuperNode's
///   `block` field, not in `arguments`. The old code treated any `block()` as
///   a literal block and excluded the def's block param from matching, causing
///   block-only forwarding to be missed entirely.
/// - FN: anonymous forwarding (`*`, `**`, `&`) was skipped wholesale. RuboCop
///   still flags `super(*)`, `super(**)`, `super(*, **)`, and `super(&)` when
///   they exactly match the enclosing method signature.
/// - FN: `super(...)` forwarding — ForwardingArgumentsNode was not handled.
/// - FN: `super()` with no-arg def — early return on "both empty" skipped it.
/// - FN: Ruby 3.1 hash value omission `super(type:, hash:)` — ImplicitNode
///   wrapping LocalVariableReadNode was not unwrapped.
/// - FN: post arguments after a rest parameter (`def f(a, *rest, tail)`) must
///   stay in Ruby's source order when compared against `super a, *rest, tail`.
pub struct SuperArguments;

impl Cop for SuperArguments {
    fn name(&self) -> &'static str {
        "Style/SuperArguments"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = SuperArgumentsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct SuperArgumentsVisitor<'a> {
    cop: &'a SuperArguments,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Represents the kind of parameter in a method definition.
#[derive(Debug, PartialEq)]
enum DefParam {
    /// Required or optional positional param: `name` or `name = default`
    Positional(Vec<u8>),
    /// Rest param: `*args` or anonymous `*`
    Rest(Option<Vec<u8>>),
    /// Required or optional keyword param: `name:` or `name: default`
    Keyword(Vec<u8>),
    /// Keyword rest param: `**kwargs` or anonymous `**`
    KeywordRest(Option<Vec<u8>>),
    /// Block param: `&block` or anonymous `&`
    Block(Option<Vec<u8>>),
    /// Forwarding parameter: `...`
    Forwarding,
}

/// Extract the ordered list of def parameters with their kinds.
fn extract_def_params(params: &ruby_prism::ParametersNode<'_>) -> Vec<DefParam> {
    let mut result = Vec::new();

    for p in params.requireds().iter() {
        if let Some(rp) = p.as_required_parameter_node() {
            result.push(DefParam::Positional(rp.name().as_slice().to_vec()));
        }
    }
    for p in params.optionals().iter() {
        if let Some(op) = p.as_optional_parameter_node() {
            result.push(DefParam::Positional(op.name().as_slice().to_vec()));
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rp) = rest.as_rest_parameter_node() {
            result.push(DefParam::Rest(
                rp.name().map(|name| name.as_slice().to_vec()),
            ));
        }
    }
    // Post params come after a rest param in Ruby's source order.
    for p in params.posts().iter() {
        if let Some(rp) = p.as_required_parameter_node() {
            result.push(DefParam::Positional(rp.name().as_slice().to_vec()));
        }
    }
    for p in params.keywords().iter() {
        if let Some(kw) = p.as_required_keyword_parameter_node() {
            let name = kw.name().as_slice();
            let clean = if name.ends_with(b":") {
                &name[..name.len() - 1]
            } else {
                name
            };
            result.push(DefParam::Keyword(clean.to_vec()));
        }
        if let Some(kw) = p.as_optional_keyword_parameter_node() {
            let name = kw.name().as_slice();
            let clean = if name.ends_with(b":") {
                &name[..name.len() - 1]
            } else {
                name
            };
            result.push(DefParam::Keyword(clean.to_vec()));
        }
    }
    if let Some(kw_rest) = params.keyword_rest() {
        // `...` forwarding parameter is stored in keyword_rest
        if kw_rest.as_forwarding_parameter_node().is_some() {
            result.push(DefParam::Forwarding);
        } else if let Some(kwr) = kw_rest.as_keyword_rest_parameter_node() {
            result.push(DefParam::KeywordRest(
                kwr.name().map(|name| name.as_slice().to_vec()),
            ));
        }
    }
    if let Some(block) = params.block() {
        result.push(DefParam::Block(
            block.name().map(|name| name.as_slice().to_vec()),
        ));
    }
    result
}

/// Check if a super argument matches a def parameter.
fn super_arg_matches_def_param(arg: &ruby_prism::Node<'_>, def_param: &DefParam) -> bool {
    match def_param {
        DefParam::Positional(name) => {
            if let Some(lv) = arg.as_local_variable_read_node() {
                return lv.name().as_slice() == name.as_slice();
            }
            false
        }
        DefParam::Rest(name) => {
            if let Some(splat) = arg.as_splat_node() {
                if let Some(name) = name {
                    if let Some(expr) = splat.expression() {
                        if let Some(lv) = expr.as_local_variable_read_node() {
                            return lv.name().as_slice() == name.as_slice();
                        }
                    }
                    return false;
                }
                return splat.expression().is_none();
            }
            false
        }
        DefParam::Keyword(name) => {
            if let Some(assoc) = arg.as_assoc_node() {
                return keyword_pair_matches(&assoc, name);
            }
            false
        }
        DefParam::KeywordRest(name) => {
            if let Some(splat) = arg.as_assoc_splat_node() {
                if let Some(name) = name {
                    if let Some(value) = splat.value() {
                        if let Some(lv) = value.as_local_variable_read_node() {
                            return lv.name().as_slice() == name.as_slice();
                        }
                    }
                    return false;
                }
                return splat.value().is_none();
            }
            false
        }
        DefParam::Block(name) => {
            if let Some(block_arg) = arg.as_block_argument_node() {
                if let Some(name) = name {
                    if let Some(expr) = block_arg.expression() {
                        if let Some(lv) = expr.as_local_variable_read_node() {
                            return lv.name().as_slice() == name.as_slice();
                        }
                    }
                    return false;
                }
                return block_arg.expression().is_none();
            }
            false
        }
        DefParam::Forwarding => arg.as_forwarding_arguments_node().is_some(),
    }
}

/// Check if an AssocNode is `name: name` (symbol key matching a local variable value).
/// Also handles Ruby 3.1+ hash value omission (`name:`) where Prism wraps the
/// value in an ImplicitNode.
fn keyword_pair_matches(assoc: &ruby_prism::AssocNode<'_>, name: &[u8]) -> bool {
    let key = assoc.key();
    let value = assoc.value();
    if let Some(sym) = key.as_symbol_node() {
        let sym_name: &[u8] = sym.unescaped();
        if sym_name != name {
            return false;
        }
        // Direct: `name: name`
        if let Some(lv) = value.as_local_variable_read_node() {
            return lv.name().as_slice() == name;
        }
        // Ruby 3.1+ shorthand `name:` — value is an ImplicitNode wrapping LocalVariableReadNode
        if let Some(implicit) = value.as_implicit_node() {
            if let Some(lv) = implicit.value().as_local_variable_read_node() {
                return lv.name().as_slice() == name;
            }
        }
    }
    false
}

/// Flatten super arguments: expand bare keyword hashes into individual pairs.
fn flatten_super_args<'a>(
    args: impl Iterator<Item = ruby_prism::Node<'a>>,
) -> Vec<ruby_prism::Node<'a>> {
    let mut result = Vec::new();
    for arg in args {
        if let Some(kh) = arg.as_keyword_hash_node() {
            for elem in kh.elements().iter() {
                result.push(elem);
            }
        } else {
            result.push(arg);
        }
    }
    result
}

struct SuperChecker<'a> {
    def_params: &'a [DefParam],
    has_forwarding: bool,
    offsets: Vec<(usize, &'static str)>,
}

impl<'pr> Visit<'pr> for SuperChecker<'_> {
    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        let super_args_raw: Vec<ruby_prism::Node<'_>> = if let Some(arguments) = node.arguments() {
            arguments.arguments().iter().collect()
        } else {
            Vec::new()
        };

        // Determine block situation:
        // - BlockArgumentNode (&block) → forwarding block arg, should be matched
        // - BlockNode ({ } / do...end) → literal block, exclude def's block param
        // - None → no block at super call site
        let has_block_arg = node
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_some());
        let has_block_literal = node
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_none());

        // Build effective def params:
        // - If super has a block literal, exclude Block param (the block is overridden)
        // - Otherwise, include all params
        let effective_def_params: Vec<&DefParam> = if has_block_literal {
            self.def_params
                .iter()
                .filter(|p| !matches!(p, DefParam::Block(_)))
                .collect()
        } else {
            self.def_params.iter().collect()
        };

        let mut flat_args = flatten_super_args(super_args_raw.into_iter());

        // If the super call has a BlockArgumentNode (&block), add it to flat_args
        // so it can be matched against the def's Block param.
        if has_block_arg {
            flat_args.push(node.block().unwrap());
        }

        // Handle forwarding: def method(...) / super(...)
        if self.has_forwarding
            && flat_args.len() == 1
            && flat_args[0].as_forwarding_arguments_node().is_some()
            && effective_def_params.len() == 1
            && matches!(effective_def_params[0], DefParam::Forwarding)
        {
            self.offsets.push((
                node.location().start_offset(),
                "Call `super` without arguments and parentheses when the signature is identical.",
            ));
            return;
        }

        if flat_args.len() != effective_def_params.len() {
            // Special case: super has a block literal AND a non-forwarded block arg in def.
            // If positional/keyword args match (excluding block), RuboCop flags with a
            // different message: "when all positional and keyword arguments are forwarded."
            if has_block_literal {
                let non_block_params: Vec<&DefParam> = self
                    .def_params
                    .iter()
                    .filter(|p| !matches!(p, DefParam::Block(_)))
                    .collect();
                let has_block_param = self
                    .def_params
                    .iter()
                    .any(|p| matches!(p, DefParam::Block(_)));
                if has_block_param
                    && flat_args.len() == non_block_params.len()
                    && flat_args
                        .iter()
                        .zip(non_block_params.iter())
                        .all(|(arg, param)| super_arg_matches_def_param(arg, param))
                {
                    self.offsets.push((
                        node.location().start_offset(),
                        "Call `super` without arguments and parentheses when all positional and keyword arguments are forwarded.",
                    ));
                }
            }
            return;
        }

        let all_match = flat_args
            .iter()
            .zip(effective_def_params.iter())
            .all(|(arg, param)| super_arg_matches_def_param(arg, param));

        if all_match {
            // Use a different message when the def has a block param that's being
            // replaced by a block literal (super(a) { x } with def(a, &blk))
            let has_unreplaced_block = has_block_literal
                && self
                    .def_params
                    .iter()
                    .any(|p| matches!(p, DefParam::Block(_)));
            let message = if has_unreplaced_block {
                "Call `super` without arguments and parentheses when all positional and keyword arguments are forwarded."
            } else {
                "Call `super` without arguments and parentheses when the signature is identical."
            };
            self.offsets.push((node.location().start_offset(), message));
        }
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't recurse into nested defs
    }

    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {
        // Don't recurse into blocks — super inside a block refers to the
        // block's enclosing method context, not the def we're checking
    }

    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {
        // Don't recurse into lambdas
    }
}

/// Check if any parameter in the def body has been reassigned (LocalVariableWriteNode,
/// LocalVariableOrWriteNode, etc.) for a given name.
fn has_param_reassignment(body: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    struct ReassignChecker<'n> {
        name: &'n [u8],
        found: bool,
    }
    impl<'pr> Visit<'pr> for ReassignChecker<'_> {
        fn visit_local_variable_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.name {
                self.found = true;
            }
        }
        fn visit_local_variable_or_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.name {
                self.found = true;
            }
        }
        fn visit_local_variable_and_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.name {
                self.found = true;
            }
        }
        fn visit_local_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
        ) {
            if node.name().as_slice() == self.name {
                self.found = true;
            }
        }
        fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
            // Don't recurse into nested defs
        }
    }
    let mut checker = ReassignChecker { name, found: false };
    checker.visit(body);
    checker.found
}

impl<'pr> Visit<'pr> for SuperArgumentsVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let def_params = if let Some(params) = node.parameters() {
            extract_def_params(&params)
        } else {
            Vec::new()
        };

        let has_forwarding = def_params.iter().any(|p| matches!(p, DefParam::Forwarding));

        // Check for block param reassignment — if the block arg is reassigned,
        // super(&block) is not a trivial forwarding
        let has_block_reassignment = if let Some(body) = node.body() {
            def_params.iter().any(|p| {
                if let DefParam::Block(Some(name)) = p {
                    has_param_reassignment(&body, name)
                } else {
                    false
                }
            })
        } else {
            false
        };

        if let Some(body) = node.body() {
            // Filter out block param if it's been reassigned
            let effective_params: Vec<DefParam> = if has_block_reassignment {
                def_params
                    .into_iter()
                    .filter(|p| !matches!(p, DefParam::Block(_)))
                    .collect()
            } else {
                def_params
            };

            let mut checker = SuperChecker {
                def_params: &effective_params,
                has_forwarding,
                offsets: Vec::new(),
            };
            checker.visit(&body);

            for (offset, message) in checker.offsets {
                let (line, column) = self.source.offset_to_line_col(offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    message.to_string(),
                ));
            }
        }

        // Recurse into the body to find nested defs (which we process independently)
        if let Some(body) = node.body() {
            let mut finder = NestedDefFinder { parent: self };
            finder.visit(&body);
        }
    }
}

/// Traverses a subtree looking for nested DefNodes and delegates them to
/// SuperArgumentsVisitor for independent processing.
struct NestedDefFinder<'a, 'b> {
    parent: &'a mut SuperArgumentsVisitor<'b>,
}

impl<'pr> Visit<'pr> for NestedDefFinder<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Delegate to the main visitor
        self.parent.visit_def_node(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SuperArguments, "cops/style/super_arguments");
}
