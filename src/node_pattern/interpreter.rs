//! NodePattern interpreter — runtime evaluation of patterns against Prism AST nodes.
//!
//! Given a NodePattern string and a Prism AST node, determine whether the
//! pattern matches the node. This is used by the verifier to detect drift
//! between RuboCop's NodePattern definitions and our hand-written Rust cops.
//!
//! ## Supported patterns (Phase 1)
//!
//! NodeMatch, Wildcard, Rest, NilPredicate, SymbolLiteral, IntLiteral,
//! StringLiteral, TrueLiteral, FalseLiteral, NilLiteral, Alternatives,
//! Conjunction, Negation, Capture (transparent), TypePredicate, Ident.
//!
//! ## Deferred
//!
//! HelperCall (#method) and ParamRef (%1) always return true (optimistic).
//! ParentRef (^) and DescendRef (`) always return true.

use super::lexer::Lexer;
use super::parser::{Parser, PatternNode};

/// A child slot in the NodePattern positional matching.
///
/// Node children are heterogeneous: some are AST nodes, some are name/value
/// bytes, and some are absent (the `nil?` predicate).
#[derive(Debug)]
pub enum MatchChild<'pr> {
    /// A child AST node (receiver, body, condition, etc.)
    Node(ruby_prism::Node<'pr>),
    /// An absent child — `nil?` matches this.
    Absent,
    /// A name or value as raw bytes (method name, variable name, symbol value).
    Name(&'pr [u8]),
}

/// Evaluate a NodePattern string against a Prism AST node.
///
/// Returns `true` if the pattern matches the node, `false` otherwise.
/// Returns `false` on parse error.
pub fn interpret_pattern(pattern_str: &str, node: &ruby_prism::Node<'_>) -> bool {
    let mut lexer = Lexer::new(pattern_str);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let Some(ast) = parser.parse() else {
        return false;
    };
    matches_node(&ast, node)
}

/// Get the NodePattern type name for a Prism node.
///
/// Returns the Parser gem type name (e.g. "send", "block", "if") that
/// corresponds to this Prism node, or `None` if unmapped.
fn parser_type_for_node(node: &ruby_prism::Node<'_>) -> Option<&'static str> {
    // send vs csend: both are CallNode, distinguished by &. operator
    if let Some(call) = node.as_call_node() {
        return if call
            .call_operator_loc()
            .is_some_and(|loc| loc.as_slice() == b"&.")
        {
            Some("csend")
        } else {
            Some("send")
        };
    }

    // Use matches! for everything else
    match node {
        ruby_prism::Node::BlockNode { .. } => Some("block"),
        ruby_prism::Node::DefNode { .. } => {
            // def vs defs: defs has a receiver
            if let Some(def) = node.as_def_node() {
                if def.receiver().is_some() {
                    Some("defs")
                } else {
                    Some("def")
                }
            } else {
                Some("def")
            }
        }
        ruby_prism::Node::ConstantReadNode { .. } => Some("const"),
        ruby_prism::Node::ConstantPathNode { .. } => Some("const"),
        ruby_prism::Node::BeginNode { .. } => Some("begin"),
        ruby_prism::Node::AssocNode { .. } => Some("pair"),
        ruby_prism::Node::HashNode { .. } => Some("hash"),
        ruby_prism::Node::LocalVariableReadNode { .. } => Some("lvar"),
        ruby_prism::Node::InstanceVariableReadNode { .. } => Some("ivar"),
        ruby_prism::Node::ClassVariableReadNode { .. } => Some("cvar"),
        ruby_prism::Node::GlobalVariableReadNode { .. } => Some("gvar"),
        ruby_prism::Node::SymbolNode { .. } => Some("sym"),
        ruby_prism::Node::StringNode { .. } => Some("str"),
        ruby_prism::Node::IntegerNode { .. } => Some("int"),
        ruby_prism::Node::FloatNode { .. } => Some("float"),
        ruby_prism::Node::TrueNode { .. } => Some("true"),
        ruby_prism::Node::FalseNode { .. } => Some("false"),
        ruby_prism::Node::NilNode { .. } => Some("nil"),
        ruby_prism::Node::SelfNode { .. } => Some("self"),
        ruby_prism::Node::ArrayNode { .. } => Some("array"),
        ruby_prism::Node::IfNode { .. } => Some("if"),
        ruby_prism::Node::CaseNode { .. } => Some("case"),
        ruby_prism::Node::WhenNode { .. } => Some("when"),
        ruby_prism::Node::WhileNode { .. } => Some("while"),
        ruby_prism::Node::UntilNode { .. } => Some("until"),
        ruby_prism::Node::ForNode { .. } => Some("for"),
        ruby_prism::Node::ReturnNode { .. } => Some("return"),
        ruby_prism::Node::YieldNode { .. } => Some("yield"),
        ruby_prism::Node::AndNode { .. } => Some("and"),
        ruby_prism::Node::OrNode { .. } => Some("or"),
        ruby_prism::Node::RegularExpressionNode { .. } => Some("regexp"),
        ruby_prism::Node::ClassNode { .. } => Some("class"),
        ruby_prism::Node::ModuleNode { .. } => Some("module"),
        ruby_prism::Node::LocalVariableWriteNode { .. } => Some("lvasgn"),
        ruby_prism::Node::InstanceVariableWriteNode { .. } => Some("ivasgn"),
        ruby_prism::Node::ConstantWriteNode { .. } => Some("casgn"),
        ruby_prism::Node::SplatNode { .. } => Some("splat"),
        ruby_prism::Node::SuperNode { .. } => Some("super"),
        ruby_prism::Node::ForwardingSuperNode { .. } => Some("zsuper"),
        ruby_prism::Node::LambdaNode { .. } => Some("lambda"),
        ruby_prism::Node::InterpolatedStringNode { .. } => Some("dstr"),
        ruby_prism::Node::InterpolatedSymbolNode { .. } => Some("dsym"),
        ruby_prism::Node::ParametersNode { .. } => Some("args"),
        _ => None,
    }
}

/// Build the children list for a node given its Parser gem type.
///
/// The returned `Vec<MatchChild>` matches NodePattern positional semantics.
/// For `send`: `[receiver_or_Absent, Name(method_name), arg1, arg2, ...]`
fn get_children<'pr>(
    parser_type: &str,
    node: &ruby_prism::Node<'pr>,
) -> Option<Vec<MatchChild<'pr>>> {
    let mut children = Vec::new();

    match parser_type {
        "send" | "csend" => {
            let call = node.as_call_node()?;
            // Receiver
            match call.receiver() {
                Some(r) => children.push(MatchChild::Node(r)),
                None => children.push(MatchChild::Absent),
            }
            // Method name
            children.push(MatchChild::Name(call.name().as_slice()));
            // Arguments (flattened)
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    children.push(MatchChild::Node(arg));
                }
            }
        }
        "block" | "any_block" => {
            let block = node.as_block_node()?;
            // In Prism, the call is the parent of the BlockNode, not a child.
            // We push Absent here and let the pattern wildcard match it.
            // Most NodePattern block patterns use _ or a specific call pattern
            // which we handle permissively in Phase 1.
            children.push(MatchChild::Absent);
            match block.parameters() {
                Some(p) => children.push(MatchChild::Node(p)),
                None => children.push(MatchChild::Absent),
            }
            match block.body() {
                Some(b) => children.push(MatchChild::Node(b)),
                None => children.push(MatchChild::Absent),
            }
        }
        "def" => {
            let def = node.as_def_node()?;
            children.push(MatchChild::Name(def.name().as_slice()));
            match def.parameters() {
                Some(p) => children.push(MatchChild::Node(p.as_node())),
                None => children.push(MatchChild::Absent),
            }
            match def.body() {
                Some(b) => children.push(MatchChild::Node(b)),
                None => children.push(MatchChild::Absent),
            }
        }
        "defs" => {
            let def = node.as_def_node()?;
            match def.receiver() {
                Some(r) => children.push(MatchChild::Node(r)),
                None => children.push(MatchChild::Absent),
            }
            children.push(MatchChild::Name(def.name().as_slice()));
            match def.parameters() {
                Some(p) => children.push(MatchChild::Node(p.as_node())),
                None => children.push(MatchChild::Absent),
            }
            match def.body() {
                Some(b) => children.push(MatchChild::Node(b)),
                None => children.push(MatchChild::Absent),
            }
        }
        "const" => {
            // Parser gem: (const parent :Name) — parent is nil for bare constants.
            // Prism splits into ConstantReadNode (bare) and ConstantPathNode (qualified).
            if let Some(c) = node.as_constant_read_node() {
                children.push(MatchChild::Absent); // nil parent
                children.push(MatchChild::Name(c.name().as_slice()));
            } else if let Some(cp) = node.as_constant_path_node() {
                match cp.parent() {
                    Some(p) => children.push(MatchChild::Node(p)),
                    None => children.push(MatchChild::Absent), // :: prefix
                }
                match cp.name() {
                    Some(n) => children.push(MatchChild::Name(n.as_slice())),
                    None => children.push(MatchChild::Absent),
                }
            } else {
                return None;
            }
        }
        "begin" => {
            let b = node.as_begin_node()?;
            match b.statements() {
                Some(s) => children.push(MatchChild::Node(s.as_node())),
                None => children.push(MatchChild::Absent),
            }
        }
        "pair" => {
            let assoc = node.as_assoc_node()?;
            children.push(MatchChild::Node(assoc.key()));
            children.push(MatchChild::Node(assoc.value()));
        }
        "hash" => {
            let hash = node.as_hash_node()?;
            for elem in hash.elements().iter() {
                children.push(MatchChild::Node(elem));
            }
        }
        "lvar" => {
            let lv = node.as_local_variable_read_node()?;
            children.push(MatchChild::Name(lv.name().as_slice()));
        }
        "ivar" => {
            let iv = node.as_instance_variable_read_node()?;
            children.push(MatchChild::Name(iv.name().as_slice()));
        }
        "cvar" => {
            let cv = node.as_class_variable_read_node()?;
            children.push(MatchChild::Name(cv.name().as_slice()));
        }
        "gvar" => {
            let gv = node.as_global_variable_read_node()?;
            children.push(MatchChild::Name(gv.name().as_slice()));
        }
        "sym" => {
            // Symbol node — value matched via special-case in the interpreter
        }
        "str" => {
            // String node — content matched via special-case
        }
        "int" | "float" => {
            // Value-only nodes — matched via special-case
        }
        "true" | "false" | "nil" | "self" | "zsuper" => {
            // No children
        }
        "array" => {
            let arr = node.as_array_node()?;
            for elem in arr.elements().iter() {
                children.push(MatchChild::Node(elem));
            }
        }
        "if" => {
            let if_node = node.as_if_node()?;
            children.push(MatchChild::Node(if_node.predicate()));
            match if_node.statements() {
                Some(s) => children.push(MatchChild::Node(s.as_node())),
                None => children.push(MatchChild::Absent),
            }
            match if_node.subsequent() {
                Some(s) => children.push(MatchChild::Node(s)),
                None => children.push(MatchChild::Absent),
            }
        }
        "case" => {
            let case = node.as_case_node()?;
            match case.predicate() {
                Some(p) => children.push(MatchChild::Node(p)),
                None => children.push(MatchChild::Absent),
            }
            for cond in case.conditions().iter() {
                children.push(MatchChild::Node(cond));
            }
            match case.else_clause() {
                Some(e) => children.push(MatchChild::Node(e.as_node())),
                None => children.push(MatchChild::Absent),
            }
        }
        "when" => {
            let when = node.as_when_node()?;
            for cond in when.conditions().iter() {
                children.push(MatchChild::Node(cond));
            }
            match when.statements() {
                Some(s) => children.push(MatchChild::Node(s.as_node())),
                None => children.push(MatchChild::Absent),
            }
        }
        "while" => {
            let w = node.as_while_node()?;
            children.push(MatchChild::Node(w.predicate()));
            match w.statements() {
                Some(s) => children.push(MatchChild::Node(s.as_node())),
                None => children.push(MatchChild::Absent),
            }
        }
        "until" => {
            let u = node.as_until_node()?;
            children.push(MatchChild::Node(u.predicate()));
            match u.statements() {
                Some(s) => children.push(MatchChild::Node(s.as_node())),
                None => children.push(MatchChild::Absent),
            }
        }
        "for" => {
            let f = node.as_for_node()?;
            children.push(MatchChild::Node(f.index()));
            children.push(MatchChild::Node(f.collection()));
            match f.statements() {
                Some(s) => children.push(MatchChild::Node(s.as_node())),
                None => children.push(MatchChild::Absent),
            }
        }
        "return" => {
            let r = node.as_return_node()?;
            if let Some(args) = r.arguments() {
                for arg in args.arguments().iter() {
                    children.push(MatchChild::Node(arg));
                }
            }
        }
        "yield" => {
            let y = node.as_yield_node()?;
            if let Some(args) = y.arguments() {
                for arg in args.arguments().iter() {
                    children.push(MatchChild::Node(arg));
                }
            }
        }
        "and" => {
            let a = node.as_and_node()?;
            children.push(MatchChild::Node(a.left()));
            children.push(MatchChild::Node(a.right()));
        }
        "or" => {
            let o = node.as_or_node()?;
            children.push(MatchChild::Node(o.left()));
            children.push(MatchChild::Node(o.right()));
        }
        "regexp" => {
            // Content matched via special-case
        }
        "class" => {
            let c = node.as_class_node()?;
            children.push(MatchChild::Node(c.constant_path()));
            match c.superclass() {
                Some(s) => children.push(MatchChild::Node(s)),
                None => children.push(MatchChild::Absent),
            }
            match c.body() {
                Some(b) => children.push(MatchChild::Node(b)),
                None => children.push(MatchChild::Absent),
            }
        }
        "module" => {
            let m = node.as_module_node()?;
            children.push(MatchChild::Node(m.constant_path()));
            match m.body() {
                Some(b) => children.push(MatchChild::Node(b)),
                None => children.push(MatchChild::Absent),
            }
        }
        "lvasgn" => {
            let lv = node.as_local_variable_write_node()?;
            children.push(MatchChild::Name(lv.name().as_slice()));
            children.push(MatchChild::Node(lv.value()));
        }
        "ivasgn" => {
            let iv = node.as_instance_variable_write_node()?;
            children.push(MatchChild::Name(iv.name().as_slice()));
            children.push(MatchChild::Node(iv.value()));
        }
        "casgn" => {
            let cw = node.as_constant_write_node()?;
            children.push(MatchChild::Name(cw.name().as_slice()));
            children.push(MatchChild::Node(cw.value()));
        }
        "splat" => {
            let s = node.as_splat_node()?;
            match s.expression() {
                Some(e) => children.push(MatchChild::Node(e)),
                None => children.push(MatchChild::Absent),
            }
        }
        "super" => {
            let s = node.as_super_node()?;
            if let Some(args) = s.arguments() {
                for arg in args.arguments().iter() {
                    children.push(MatchChild::Node(arg));
                }
            }
        }
        "lambda" => {
            let l = node.as_lambda_node()?;
            match l.parameters() {
                Some(p) => children.push(MatchChild::Node(p)),
                None => children.push(MatchChild::Absent),
            }
            match l.body() {
                Some(b) => children.push(MatchChild::Node(b)),
                None => children.push(MatchChild::Absent),
            }
        }
        "dstr" => {
            let isn = node.as_interpolated_string_node()?;
            for part in isn.parts().iter() {
                children.push(MatchChild::Node(part));
            }
        }
        "dsym" => {
            let isn = node.as_interpolated_symbol_node()?;
            for part in isn.parts().iter() {
                children.push(MatchChild::Node(part));
            }
        }
        "args" => {
            // ParametersNode — no positional children in simple patterns
        }
        _ => return None,
    }

    Some(children)
}

/// Match a PatternNode against a MatchChild (dispatcher).
fn matches_child(pattern: &PatternNode, child: &MatchChild<'_>) -> bool {
    match child {
        MatchChild::Node(node) => matches_node(pattern, node),
        MatchChild::Absent => matches_absent(pattern),
        MatchChild::Name(bytes) => matches_name(pattern, bytes),
    }
}

/// Match a pattern against a Prism AST node.
fn matches_node(pattern: &PatternNode, node: &ruby_prism::Node<'_>) -> bool {
    match pattern {
        PatternNode::Wildcard => true,
        PatternNode::NilPredicate => false, // Node is present, not absent

        PatternNode::SymbolLiteral(name) => {
            if let Some(sym) = node.as_symbol_node() {
                sym.unescaped() == name.as_bytes()
            } else {
                false
            }
        }

        PatternNode::IntLiteral(n) => {
            if let Some(int_node) = node.as_integer_node() {
                let loc = int_node.location();
                let src = loc.as_slice();
                let src_str = std::str::from_utf8(src).unwrap_or("");
                let cleaned: String = src_str.chars().filter(|c| *c != '_').collect();
                cleaned.parse::<i64>().ok() == Some(*n)
            } else {
                false
            }
        }

        PatternNode::StringLiteral(s) => {
            if let Some(str_node) = node.as_string_node() {
                str_node.unescaped() == s.as_bytes()
            } else {
                false
            }
        }

        PatternNode::NilLiteral => node.as_nil_node().is_some(),
        PatternNode::TrueLiteral => node.as_true_node().is_some(),
        PatternNode::FalseLiteral => node.as_false_node().is_some(),

        PatternNode::TypePredicate(typ) => parser_type_for_node(node) == Some(typ.as_str()),

        PatternNode::Ident(name) => parser_type_for_node(node) == Some(name.as_str()),

        PatternNode::NodeMatch {
            node_type,
            children: pattern_children,
        } => {
            let actual_type = parser_type_for_node(node);
            let type_matches = actual_type == Some(node_type.as_str())
                || (node_type == "any_block" && actual_type == Some("block"));

            if !type_matches {
                return false;
            }

            // Value-only nodes: (int 42), (str "foo"), (sym :bar)
            if !pattern_children.is_empty() {
                match node_type.as_str() {
                    "int" | "str" => {
                        return matches_node(&pattern_children[0], node);
                    }
                    "sym" => {
                        if let Some(sym) = node.as_symbol_node() {
                            return matches_name(&pattern_children[0], sym.unescaped());
                        }
                        return false;
                    }
                    _ => {}
                }
            }

            let effective_type = if node_type == "any_block" {
                "block"
            } else {
                node_type.as_str()
            };
            let Some(actual_children) = get_children(effective_type, node) else {
                return pattern_children.is_empty();
            };

            matches_children_list(pattern_children, &actual_children)
        }

        PatternNode::Alternatives(alts) => alts.iter().any(|alt| matches_node(alt, node)),
        PatternNode::Conjunction(items) => items.iter().all(|item| matches_node(item, node)),
        PatternNode::Negation(inner) => !matches_node(inner, node),
        PatternNode::Capture(inner) => matches_node(inner, node),

        PatternNode::HelperCall(_) => true,
        PatternNode::ParamRef(_) => true,
        PatternNode::ParentRef(_) => true,
        PatternNode::DescendRef(_) => true,

        PatternNode::FloatLiteral(s) => {
            if let Some(float_node) = node.as_float_node() {
                let loc = float_node.location();
                let src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                src == s.as_str()
            } else {
                false
            }
        }

        PatternNode::Rest => true,
    }
}

/// Match a pattern against an absent child (`nil?` predicate target).
fn matches_absent(pattern: &PatternNode) -> bool {
    match pattern {
        PatternNode::Wildcard => true,
        PatternNode::NilPredicate => true,
        PatternNode::Alternatives(alts) => alts.iter().any(matches_absent),
        PatternNode::Conjunction(items) => items.iter().all(matches_absent),
        PatternNode::Negation(inner) => !matches_absent(inner),
        PatternNode::Capture(inner) => matches_absent(inner),
        PatternNode::HelperCall(_) => true,
        PatternNode::ParamRef(_) => true,
        PatternNode::ParentRef(_) => true,
        PatternNode::DescendRef(_) => true,
        PatternNode::Rest => true,
        _ => false,
    }
}

/// Match a pattern against a name/value byte slice (method name, variable name).
fn matches_name(pattern: &PatternNode, bytes: &[u8]) -> bool {
    match pattern {
        PatternNode::Wildcard => true,
        PatternNode::SymbolLiteral(name) => bytes == name.as_bytes(),
        PatternNode::Alternatives(alts) => alts.iter().any(|alt| matches_name(alt, bytes)),
        PatternNode::Conjunction(items) => items.iter().all(|item| matches_name(item, bytes)),
        PatternNode::Negation(inner) => !matches_name(inner, bytes),
        PatternNode::Capture(inner) => matches_name(inner, bytes),
        PatternNode::HelperCall(_) => true,
        PatternNode::ParamRef(_) => true,
        PatternNode::ParentRef(_) => true,
        PatternNode::DescendRef(_) => true,
        PatternNode::Rest => true,
        _ => false,
    }
}

/// Match a list of pattern children against a list of actual children.
///
/// Walks patterns and actuals in parallel. `Rest` consumes all remaining.
/// Without `Rest`, extra actuals after all patterns are consumed are tolerated
/// (permissive for Phase 1).
fn matches_children_list(patterns: &[PatternNode], actuals: &[MatchChild<'_>]) -> bool {
    let mut pi = 0; // pattern index
    let mut ai = 0; // actual index

    while pi < patterns.len() {
        let pattern = &patterns[pi];

        if matches!(pattern, PatternNode::Rest) {
            // Rest consumes all remaining actuals — match succeeds
            return true;
        }

        if ai >= actuals.len() {
            // No more actuals but still have patterns — fail
            return false;
        }

        if !matches_child(pattern, &actuals[ai]) {
            return false;
        }

        pi += 1;
        ai += 1;
    }

    // All patterns consumed. Extra actuals are tolerated (permissive).
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ruby(source: &str) -> ruby_prism::ParseResult<'_> {
        ruby_prism::parse(source.as_bytes())
    }

    /// Helper: get first statement from parsed Ruby source.
    fn first_stmt<'a>(result: &'a ruby_prism::ParseResult<'a>) -> ruby_prism::Node<'a> {
        let root = result.node();
        let program = root.as_program_node().unwrap();
        let stmts = program.statements();
        stmts.body().iter().next().unwrap()
    }

    #[test]
    fn test_send_nil_receiver() {
        // require 'foo' → (send nil? :require (str "foo"))
        let source = b"require 'foo'";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(send nil? :require ...)", &node));
        assert!(!interpret_pattern("(send nil? :include ...)", &node));
    }

    #[test]
    fn test_send_with_receiver() {
        let source = b"obj.foo";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(send _ :foo)", &node));
        assert!(!interpret_pattern("(send nil? :foo)", &node));
    }

    #[test]
    fn test_wildcard() {
        let source = b"x.bar(1)";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(send _ :bar _)", &node));
        assert!(interpret_pattern("(send _ _ ...)", &node));
    }

    #[test]
    fn test_rest() {
        let source = b"foo(1, 2, 3)";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(send nil? :foo ...)", &node));
    }

    #[test]
    fn test_if_no_else() {
        // if x; y; end → (if _ _ nil?)
        let source = b"if x; y; end";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(if _ _ nil?)", &node));
        assert!(interpret_pattern("(if _ _ _)", &node)); // wildcard also matches Absent
    }

    #[test]
    fn test_alternatives() {
        let source = b"obj.first";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(send _ {:first | :take})", &node));
        assert!(interpret_pattern("(send _ {:first :take})", &node));
    }

    #[test]
    fn test_alternatives_no_match() {
        let source = b"obj.last";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(!interpret_pattern("(send _ {:first :take})", &node));
    }

    #[test]
    fn test_negation() {
        let source = b"obj.foo";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        // Has a receiver (not nil?), so !nil? on receiver should match
        assert!(interpret_pattern("(send !nil? :foo)", &node));
        // nil? on receiver should NOT match
        assert!(!interpret_pattern("(send nil? :foo)", &node));
    }

    #[test]
    fn test_nested_send() {
        let source = b"obj.where.first";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(send (send _ :where) :first)", &node));
    }

    #[test]
    fn test_type_predicate() {
        let source = b"'hello'";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("str?", &node));
        assert!(!interpret_pattern("int?", &node));
    }

    #[test]
    fn test_true_false_nil_literals() {
        let source_true = b"true";
        let result_true = ruby_prism::parse(source_true);
        let node_true = first_stmt(&result_true);
        assert!(interpret_pattern("true", &node_true));
        assert!(!interpret_pattern("false", &node_true));

        let source_false = b"false";
        let result_false = ruby_prism::parse(source_false);
        let node_false = first_stmt(&result_false);
        assert!(interpret_pattern("false", &node_false));

        let source_nil = b"nil";
        let result_nil = ruby_prism::parse(source_nil);
        let node_nil = first_stmt(&result_nil);
        assert!(interpret_pattern("nil", &node_nil));
    }

    #[test]
    fn test_capture_transparent() {
        let source = b"obj.foo";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        // $_ should match like _
        assert!(interpret_pattern("(send $_ :foo)", &node));
        // $(send ...) should match like (send ...)
        assert!(interpret_pattern("$(send _ :foo)", &node));
    }

    #[test]
    fn test_conjunction() {
        let source = b"obj.foo";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        // [!nil? send_type?] — both must match
        assert!(interpret_pattern("[!nil? send_type?]", &node));
    }

    #[test]
    fn test_int_literal() {
        let source = b"42";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(int 42)", &node));
    }

    #[test]
    fn test_string_literal_match() {
        let source = b"'hello'";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(str 'hello')", &node));
        assert!(!interpret_pattern("(str 'world')", &node));
    }

    #[test]
    fn test_symbol_literal_match() {
        let source = b":foo";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(sym :foo)", &node));
        assert!(!interpret_pattern("(sym :bar)", &node));
    }

    #[test]
    fn test_block_pattern() {
        let source = b"items.each { |x| x }";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        // In Prism, `items.each { |x| x }` is a CallNode with a block.
        // The BlockNode is the block child, not the top-level statement.
        // The top-level node is the CallNode.
        assert!(interpret_pattern("(send _ :each)", &node));

        // To test BlockNode matching, get the block from the call
        let call = node.as_call_node().unwrap();
        let block = call.block().unwrap();
        // BlockNode: call child is Absent (Prism structure), params, body
        assert!(interpret_pattern("(block _ _ _)", &block));
    }

    #[test]
    fn test_def_pattern() {
        let source = b"def initialize; end";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(def :initialize ...)", &node));
        assert!(!interpret_pattern("(def :other ...)", &node));
    }

    #[test]
    fn test_and_or_patterns() {
        let source_and = b"a && b";
        let result_and = ruby_prism::parse(source_and);
        let node_and = first_stmt(&result_and);
        assert!(interpret_pattern("(and _ _)", &node_and));

        let source_or = b"a || b";
        let result_or = ruby_prism::parse(source_or);
        let node_or = first_stmt(&result_or);
        assert!(interpret_pattern("(or _ _)", &node_or));
    }

    #[test]
    fn test_helper_call_always_true() {
        let source = b"obj.foo";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        // HelperCall patterns are always-true in Phase 1
        assert!(interpret_pattern("(send #any_helper? :foo)", &node));
    }

    #[test]
    fn test_no_match_wrong_type() {
        let source = b"42";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(!interpret_pattern("(send _ :foo)", &node));
    }

    #[test]
    fn test_parse_error_returns_false() {
        let source = b"x";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        // Broken pattern
        assert!(!interpret_pattern("((( broken", &node));
    }

    #[test]
    fn test_lvasgn_pattern() {
        let source = b"x = 1";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(lvasgn :x _)", &node));
        assert!(!interpret_pattern("(lvasgn :y _)", &node));
    }

    #[test]
    fn test_array_pattern() {
        let source = b"[1, 2, 3]";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(array ...)", &node));
        assert!(interpret_pattern("(array _ _ _)", &node));
    }

    #[test]
    fn test_hash_pattern() {
        let source = b"{ a: 1 }";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(hash _)", &node));
    }

    #[test]
    fn test_class_pattern() {
        let source = b"class Foo < Bar; end";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(class _ _ _)", &node));
    }

    #[test]
    fn test_module_pattern() {
        let source = b"module Foo; end";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(module _ _)", &node));
    }

    #[test]
    fn test_return_pattern() {
        let source = b"return 42";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(return _)", &node));
    }

    #[test]
    fn test_csend_pattern() {
        let source = b"obj&.foo";
        let result = ruby_prism::parse(source);
        let node = first_stmt(&result);

        assert!(interpret_pattern("(csend _ :foo)", &node));
        assert!(!interpret_pattern("(send _ :foo)", &node));
    }

    #[test]
    fn test_send_csend_alternatives() {
        let source_send = b"obj.foo";
        let result_send = ruby_prism::parse(source_send);
        let node_send = first_stmt(&result_send);

        let source_csend = b"obj&.foo";
        let result_csend = ruby_prism::parse(source_csend);
        let node_csend = first_stmt(&result_csend);

        let pat = "{(send _ :foo) (csend _ :foo)}";
        assert!(interpret_pattern(pat, &node_send));
        assert!(interpret_pattern(pat, &node_csend));
    }
}
