use std::cell::RefCell;
use std::collections::HashMap;

use ruby_prism::Visit;

use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/OperatorMethodCall — flags redundant dot before operator methods.
///
/// Investigation (2026-04-02): the remaining false negatives came from two places.
/// First, `OPERATOR_METHODS` was missing RuboCop operators like `=~`, so cases like
/// `@regexp.=~(@string)` were never considered. Second, the old source-text heuristic
/// over-modeled RuboCop's parenthesized-call exemption and skipped real offenses such as
/// `65.+(rand(25)).chr`, `self.==(other)`, and `array.-(other).length`.
///
/// Fix: cache each call node's Prism parent/grandparent once per file, then mirror the
/// real RuboCop boundary:
/// - skip parenthesized operator calls used as arguments to another call
/// - skip chained parenthesized calls only when the RHS has a Parser-style truthy first
///   child (for example `foo.+(@bar).to_s` or `scopes.-(%i[x]).any?`)
///
/// Bare no-receiver calls like `other` and `rand(25)` remain offenses.
pub struct OperatorMethodCall;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"|", b"^", b"&", b"<=>", b"==", b"===", b"=~", b">", b">=", b"<", b"<=", b"<<", b">>", b"+",
    b"-", b"*", b"/", b"%", b"**", b"~", b"!", b"!=", b"!~",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ParentCallRelation {
    #[default]
    None,
    Receiver,
    Argument,
    Other,
}

#[derive(Clone, Copy, Default)]
struct CallContext {
    parent_call_relation: ParentCallRelation,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct CallKey {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct CacheKey {
    parse_result_ptr: usize,
    source_ptr: usize,
    source_len: usize,
}

thread_local! {
    static CALL_CONTEXT_CACHE: RefCell<Option<(CacheKey, HashMap<CallKey, CallContext>)>> =
        const { RefCell::new(None) };
}

struct CallContextVisitor<'pr> {
    ancestors: Vec<ruby_prism::Node<'pr>>,
    contexts: HashMap<CallKey, CallContext>,
}

impl<'pr> Visit<'pr> for CallContextVisitor<'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.ancestors.push(node);
    }

    fn visit_branch_node_leave(&mut self) {
        self.ancestors.pop();
    }

    fn visit_leaf_node_enter(&mut self, _node: ruby_prism::Node<'pr>) {}

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let parent_call_relation = self
            .ancestors
            .get(self.ancestors.len().saturating_sub(2))
            .map(|parent| parent_call_relation(parent, node))
            .unwrap_or_default();

        self.contexts.insert(
            call_key(node),
            CallContext {
                parent_call_relation,
            },
        );

        ruby_prism::visit_call_node(self, node);
    }
}

fn parent_call_relation(
    parent: &ruby_prism::Node<'_>,
    node: &ruby_prism::CallNode<'_>,
) -> ParentCallRelation {
    let Some(parent_call) = parent.as_call_node() else {
        return ParentCallRelation::Other;
    };

    if parent_call
        .receiver()
        .is_some_and(|receiver| same_span(receiver.location(), node.location()))
    {
        return ParentCallRelation::Receiver;
    }

    if parent_call.arguments().is_some_and(|args| {
        args.arguments()
            .iter()
            .any(|arg| same_span(arg.location(), node.location()))
    }) {
        return ParentCallRelation::Argument;
    }

    ParentCallRelation::Other
}

fn same_span(left: ruby_prism::Location<'_>, right: ruby_prism::Location<'_>) -> bool {
    left.start_offset() == right.start_offset() && left.end_offset() == right.end_offset()
}

fn call_key(call: &ruby_prism::CallNode<'_>) -> CallKey {
    let loc = call.location();
    CallKey {
        start: loc.start_offset(),
        end: loc.end_offset(),
    }
}

fn call_context(
    parse_result: &ruby_prism::ParseResult<'_>,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> CallContext {
    let cache_key = CacheKey {
        parse_result_ptr: parse_result as *const _ as usize,
        source_ptr: source.as_bytes().as_ptr() as usize,
        source_len: source.as_bytes().len(),
    };

    CALL_CONTEXT_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let needs_rebuild = !matches!(cache.as_ref(), Some((key, _)) if *key == cache_key);

        if needs_rebuild {
            let mut visitor = CallContextVisitor {
                ancestors: Vec::new(),
                contexts: HashMap::new(),
            };
            visitor.visit(&parse_result.node());
            *cache = Some((cache_key, visitor.contexts));
        }

        cache
            .as_ref()
            .and_then(|(_, contexts)| contexts.get(&call_key(call)).copied())
            .unwrap_or_default()
    })
}

fn parser_like_first_child_truthy(arg: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = arg.as_call_node() {
        return call.receiver().is_some();
    }
    if let Some(array) = arg.as_array_node() {
        return array.elements().iter().next().is_some();
    }
    if let Some(hash) = arg.as_hash_node() {
        return hash.elements().iter().next().is_some();
    }
    if let Some(hash) = arg.as_keyword_hash_node() {
        return hash.elements().iter().next().is_some();
    }
    if let Some(paren) = arg.as_parentheses_node() {
        return paren.body().is_some();
    }

    !(arg.as_self_node().is_some()
        || arg.as_nil_node().is_some()
        || arg.as_true_node().is_some()
        || arg.as_false_node().is_some()
        || arg.as_constant_read_node().is_some())
}

impl Cop for OperatorMethodCall {
    fn name(&self) -> &'static str {
        "Style/OperatorMethodCall"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be an operator method
        if !OPERATOR_METHODS.contains(&method_bytes) {
            return;
        }

        // Must have a receiver, and receiver must not be a constant
        // RuboCop skips const_type? receivers (e.g., `Tree.<<(result)`)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if receiver.as_constant_read_node().is_some() || receiver.as_constant_path_node().is_some()
        {
            return;
        }

        // Must have a dot call operator (redundant dot before operator)
        let call_op = match call.call_operator_loc() {
            Some(op) => op,
            None => return,
        };

        if call_op.as_slice() != b"." {
            return;
        }

        // Must have exactly one argument (binary operator)
        let arg = if let Some(args) = call.arguments() {
            let mut arg_iter = args.arguments().iter();
            let Some(arg) = arg_iter.next() else {
                return;
            };
            if arg_iter.next().is_some() {
                return;
            }

            // Skip splat, kwsplat, forwarded args — removing dot would be
            // invalid syntax (RuboCop's INVALID_SYNTAX_ARG_TYPES)
            if arg.as_splat_node().is_some() || arg.as_assoc_splat_node().is_some() {
                return;
            }
            if let Some(kh) = arg.as_keyword_hash_node() {
                if kh
                    .elements()
                    .iter()
                    .any(|e| e.as_assoc_splat_node().is_some())
                {
                    return;
                }
            }

            arg
        } else {
            // Unary operator with dot is also wrong but less common
            // Only flag binary operators
            return;
        };

        if call.opening_loc().is_some() {
            let context = call_context(parse_result, source, &call);

            if context.parent_call_relation == ParentCallRelation::Argument {
                return;
            }

            if context.parent_call_relation == ParentCallRelation::Receiver
                && parser_like_first_child_truthy(&arg)
            {
                return;
            }
        }

        let (line, column) = source.offset_to_line_col(call_op.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Redundant dot detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OperatorMethodCall, "cops/style/operator_method_call");
}
