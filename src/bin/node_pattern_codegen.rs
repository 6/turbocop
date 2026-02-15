//! NodePattern codegen — experimental prototype.
//!
//! Status: NOT used in CI or the standard cop-writing workflow. Kept in-tree
//! as a reference implementation of the NodePattern DSL parser.
//!
//! What works:
//!   - Lexer and parser for the NodePattern DSL
//!   - Parser→Prism node type mapping table
//!   - Code generation for simple single-type patterns without captures
//!
//! What does NOT work:
//!   - Alternatives codegen (e.g. `{send | csend}`)
//!   - Capture variables (`$name`)
//!   - Literal value matching (`:symbol`, `"string"`, integers)
//!   - `nil?` / `cbase` handling
//!   - Verify mode (stub — always reports "not implemented")
//!
//! For writing cops, the mapping table in `docs/node_pattern_analysis.md` is
//! the more useful reference — it shows which Prism node types and accessors
//! correspond to each Parser gem node type.
//!
//! Usage:
//!   cargo run --bin node_pattern_codegen -- generate <ruby_file>
//!   cargo run --bin node_pattern_codegen -- verify <ruby_file> <rust_file>

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::{self, Write};
use std::process;

// ---------------------------------------------------------------------------
// 1. Ruby file reader — extract def_node_matcher / def_node_search patterns
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ExtractedPattern {
    kind: PatternKind,
    method_name: String,
    pattern: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternKind {
    Matcher,
    Search,
}

fn extract_patterns(source: &str) -> Vec<ExtractedPattern> {
    let mut results = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Match: def_node_matcher :name, '...'
        // Match: def_node_matcher :name, <<~PATTERN
        // Match: def_node_search :name, '...'
        // Match: def_node_search :name, <<~PATTERN
        for (prefix, kind) in [
            ("def_node_matcher", PatternKind::Matcher),
            ("def_node_search", PatternKind::Search),
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let rest = rest.trim();
                // Extract :method_name
                if let Some(rest) = rest.strip_prefix(':') {
                    if let Some(comma_pos) = rest.find(',') {
                        let method_name = rest[..comma_pos].trim().to_string();
                        let after_comma = rest[comma_pos + 1..].trim();

                        if after_comma.starts_with("<<~") {
                            // Heredoc form — read until the delimiter
                            let delimiter = after_comma
                                .trim_start_matches("<<~")
                                .trim()
                                .trim_matches('\'')
                                .trim_matches('"');
                            let mut pattern_lines = Vec::new();
                            i += 1;
                            while i < lines.len() {
                                let line = lines[i].trim();
                                if line == delimiter {
                                    break;
                                }
                                pattern_lines.push(line);
                                i += 1;
                            }
                            let pattern = pattern_lines.join("\n");
                            results.push(ExtractedPattern {
                                kind,
                                method_name,
                                pattern,
                            });
                        } else if after_comma.starts_with('\'') || after_comma.starts_with('"') {
                            // Inline string form
                            let quote = after_comma.as_bytes()[0] as char;
                            let inner = &after_comma[1..];
                            if let Some(end) = inner.rfind(quote) {
                                let pattern = inner[..end].to_string();
                                results.push(ExtractedPattern {
                                    kind,
                                    method_name,
                                    pattern,
                                });
                            }
                        }
                    }
                }
            }
        }

        i += 1;
    }

    results
}

// ---------------------------------------------------------------------------
// 2. Pattern Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    LParen,
    RParen,
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Capture,   // $
    Wildcard,  // _
    Rest,      // ...
    Negation,  // !
    Pipe,      // | inside alternatives
    HelperCall(String),  // #method_name or #method_name?
    SymbolLiteral(String), // :sym
    IntLiteral(i64),
    FloatLiteral(String),
    StringLiteral(String),
    NilPredicate,   // nil?
    TruePredicate,  // true?
    FalsePredicate,  // false?
    TypePredicate(String), // int?, str?, sym?, etc.
    Ident(String),  // node type names: send, block, def, etc.
    ParamRef(String), // %1, %param
    Caret,     // ^ (parent node ref)
    Backtick,  // ` (descend operator)
}

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn read_while(&mut self, pred: impl Fn(u8) -> bool) -> String {
        let start = self.pos;
        while self.pos < self.input.len() && pred(self.input[self.pos]) {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.input[start..self.pos]).into_owned()
    }

    fn is_ident_char(ch: u8) -> bool {
        ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-'
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();
            let Some(ch) = self.peek() else { break };

            match ch {
                b'(' => {
                    self.advance();
                    tokens.push(Token::LParen);
                }
                b')' => {
                    self.advance();
                    tokens.push(Token::RParen);
                }
                b'{' => {
                    self.advance();
                    tokens.push(Token::LBrace);
                }
                b'}' => {
                    self.advance();
                    tokens.push(Token::RBrace);
                }
                b'[' => {
                    self.advance();
                    tokens.push(Token::LBracket);
                }
                b']' => {
                    self.advance();
                    tokens.push(Token::RBracket);
                }
                b'$' => {
                    self.advance();
                    tokens.push(Token::Capture);
                }
                b'|' => {
                    self.advance();
                    tokens.push(Token::Pipe);
                }
                b'^' => {
                    self.advance();
                    tokens.push(Token::Caret);
                }
                b'`' => {
                    self.advance();
                    tokens.push(Token::Backtick);
                }
                b'!' => {
                    self.advance();
                    tokens.push(Token::Negation);
                }
                b'.' => {
                    // Check for ...
                    if self.pos + 2 < self.input.len()
                        && self.input[self.pos + 1] == b'.'
                        && self.input[self.pos + 2] == b'.'
                    {
                        self.pos += 3;
                        tokens.push(Token::Rest);
                    } else {
                        // Skip unknown
                        self.advance();
                    }
                }
                b'#' => {
                    self.advance();
                    let name = self.read_while(|c| Self::is_ident_char(c) || c == b'?');
                    tokens.push(Token::HelperCall(name));
                }
                b':' => {
                    self.advance();
                    // Could be :: (cbase) — for now treat as symbol
                    if self.peek() == Some(b':') {
                        self.advance();
                        tokens.push(Token::Ident("cbase".to_string()));
                    } else {
                        let name = self.read_while(|c| Self::is_ident_char(c) || c == b'?');
                        tokens.push(Token::SymbolLiteral(name));
                    }
                }
                b'%' => {
                    self.advance();
                    let param = self.read_while(|c| c.is_ascii_alphanumeric() || c == b'_');
                    tokens.push(Token::ParamRef(param));
                }
                b'\'' | b'"' => {
                    let quote = self.advance().unwrap();
                    let s = self.read_while(move |c| c != quote);
                    self.advance(); // closing quote
                    tokens.push(Token::StringLiteral(s));
                }
                b'_' => {
                    // Could be just _ (wildcard) or an identifier starting with _
                    let word = self.read_while(|c| Self::is_ident_char(c) || c == b'?');
                    if word == "_" {
                        tokens.push(Token::Wildcard);
                    } else {
                        tokens.push(Token::Ident(word));
                    }
                }
                _ if ch.is_ascii_digit() || (ch == b'-' && self.input.get(self.pos + 1).is_some_and(|c| c.is_ascii_digit())) => {
                    let num_str = self.read_while(|c| c.is_ascii_digit() || c == b'-' || c == b'.');
                    if num_str.contains('.') {
                        tokens.push(Token::FloatLiteral(num_str));
                    } else if let Ok(n) = num_str.parse::<i64>() {
                        tokens.push(Token::IntLiteral(n));
                    } else {
                        tokens.push(Token::Ident(num_str));
                    }
                }
                _ if ch.is_ascii_alphabetic() => {
                    let word = self.read_while(|c| Self::is_ident_char(c) || c == b'?');
                    match word.as_str() {
                        "nil?" => tokens.push(Token::NilPredicate),
                        "true?" => tokens.push(Token::TruePredicate),
                        "false?" => tokens.push(Token::FalsePredicate),
                        "int?" => tokens.push(Token::TypePredicate("int".to_string())),
                        "str?" => tokens.push(Token::TypePredicate("str".to_string())),
                        "sym?" => tokens.push(Token::TypePredicate("sym".to_string())),
                        "float?" => tokens.push(Token::TypePredicate("float".to_string())),
                        "array?" => tokens.push(Token::TypePredicate("array".to_string())),
                        "hash?" => tokens.push(Token::TypePredicate("hash".to_string())),
                        "regexp?" => tokens.push(Token::TypePredicate("regexp".to_string())),
                        "send_type?" => tokens.push(Token::TypePredicate("send".to_string())),
                        "block_type?" => tokens.push(Token::TypePredicate("block".to_string())),
                        _ => tokens.push(Token::Ident(word)),
                    }
                }
                _ => {
                    // Skip unknown characters
                    self.advance();
                }
            }
        }

        tokens
    }
}

// ---------------------------------------------------------------------------
// 3. Pattern Parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum PatternNode {
    /// (node_type child1 child2 ...)
    NodeMatch {
        node_type: String,
        children: Vec<PatternNode>,
    },
    /// {a | b | c}
    Alternatives(Vec<PatternNode>),
    /// [a b c]
    Conjunction(Vec<PatternNode>),
    /// $pattern
    Capture(Box<PatternNode>),
    /// _
    Wildcard,
    /// ...
    Rest,
    /// !pattern
    Negation(Box<PatternNode>),
    /// #helper_method
    HelperCall(String),
    /// :symbol
    SymbolLiteral(String),
    /// Integer literal
    IntLiteral(i64),
    /// Float literal
    FloatLiteral(String),
    /// String literal
    StringLiteral(String),
    /// nil? — receiver is nil / no receiver
    NilPredicate,
    /// true literal node
    TrueLiteral,
    /// false literal node
    FalseLiteral,
    /// nil literal node
    NilLiteral,
    /// %param
    ParamRef(String),
    /// Type predicate: int?, str?, sym?, etc.
    TypePredicate(String),
    /// ^pattern — parent node
    ParentRef(Box<PatternNode>),
    /// `pattern — descend
    DescendRef(Box<PatternNode>),
    /// An identifier used in certain contexts (e.g., inside alternatives for node types)
    Ident(String),
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(tok)
    }

    fn expect(&mut self, expected: &Token) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse(&mut self) -> Option<PatternNode> {
        self.parse_node()
    }

    fn parse_node(&mut self) -> Option<PatternNode> {
        let tok = self.peek()?.clone();

        match tok {
            Token::LParen => self.parse_sequence(),
            Token::LBrace => self.parse_alternatives(),
            Token::LBracket => self.parse_conjunction(),
            Token::Capture => {
                self.advance();
                let inner = self.parse_node()?;
                Some(PatternNode::Capture(Box::new(inner)))
            }
            Token::Negation => {
                self.advance();
                let inner = self.parse_node()?;
                Some(PatternNode::Negation(Box::new(inner)))
            }
            Token::Caret => {
                self.advance();
                let inner = self.parse_node()?;
                Some(PatternNode::ParentRef(Box::new(inner)))
            }
            Token::Backtick => {
                self.advance();
                let inner = self.parse_node()?;
                Some(PatternNode::DescendRef(Box::new(inner)))
            }
            Token::Wildcard => {
                self.advance();
                Some(PatternNode::Wildcard)
            }
            Token::Rest => {
                self.advance();
                Some(PatternNode::Rest)
            }
            Token::NilPredicate => {
                self.advance();
                Some(PatternNode::NilPredicate)
            }
            Token::TruePredicate => {
                self.advance();
                Some(PatternNode::TrueLiteral)
            }
            Token::FalsePredicate => {
                self.advance();
                Some(PatternNode::FalseLiteral)
            }
            Token::TypePredicate(ref name) => {
                let name = name.clone();
                self.advance();
                Some(PatternNode::TypePredicate(name))
            }
            Token::HelperCall(ref name) => {
                let name = name.clone();
                self.advance();
                Some(PatternNode::HelperCall(name))
            }
            Token::SymbolLiteral(ref name) => {
                let name = name.clone();
                self.advance();
                Some(PatternNode::SymbolLiteral(name))
            }
            Token::IntLiteral(n) => {
                self.advance();
                Some(PatternNode::IntLiteral(n))
            }
            Token::FloatLiteral(ref s) => {
                let s = s.clone();
                self.advance();
                Some(PatternNode::FloatLiteral(s))
            }
            Token::StringLiteral(ref s) => {
                let s = s.clone();
                self.advance();
                Some(PatternNode::StringLiteral(s))
            }
            Token::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                match name.as_str() {
                    "nil" => Some(PatternNode::NilLiteral),
                    "true" => Some(PatternNode::TrueLiteral),
                    "false" => Some(PatternNode::FalseLiteral),
                    _ => Some(PatternNode::Ident(name)),
                }
            }
            Token::ParamRef(ref s) => {
                let s = s.clone();
                self.advance();
                Some(PatternNode::ParamRef(s))
            }
            _ => None,
        }
    }

    fn parse_sequence(&mut self) -> Option<PatternNode> {
        self.expect(&Token::LParen);

        // First element is the node type (or could be a complex expression)
        let first = self.parse_node()?;

        // Determine if this is a node match or something else
        let node_type = match &first {
            PatternNode::Ident(name) => Some(name.clone()),
            _ => None,
        };

        let mut children = Vec::new();

        // Parse remaining children
        while self.peek().is_some() && self.peek() != Some(&Token::RParen) {
            if let Some(child) = self.parse_node() {
                children.push(child);
            } else {
                break;
            }
        }

        self.expect(&Token::RParen);

        if let Some(nt) = node_type {
            Some(PatternNode::NodeMatch {
                node_type: nt,
                children,
            })
        } else {
            // Non-identifier first element (e.g. alternatives) — wrap in _complex
            let mut all = vec![first];
            all.extend(children);
            Some(PatternNode::NodeMatch {
                node_type: "_complex".to_string(),
                children: all,
            })
        }
    }

    fn parse_alternatives(&mut self) -> Option<PatternNode> {
        self.expect(&Token::LBrace);
        let mut alts = Vec::new();

        while self.peek().is_some() && self.peek() != Some(&Token::RBrace) {
            // Skip pipe separators
            if self.peek() == Some(&Token::Pipe) {
                self.advance();
                continue;
            }
            if let Some(node) = self.parse_node() {
                alts.push(node);
            } else {
                break;
            }
        }

        self.expect(&Token::RBrace);
        Some(PatternNode::Alternatives(alts))
    }

    fn parse_conjunction(&mut self) -> Option<PatternNode> {
        self.expect(&Token::LBracket);
        let mut items = Vec::new();

        while self.peek().is_some() && self.peek() != Some(&Token::RBracket) {
            if let Some(node) = self.parse_node() {
                items.push(node);
            } else {
                break;
            }
        }

        self.expect(&Token::RBracket);
        Some(PatternNode::Conjunction(items))
    }
}

// ---------------------------------------------------------------------------
// 4. Parser gem → Prism Mapping Table
// ---------------------------------------------------------------------------

struct NodeMapping {
    parser_type: &'static str,
    prism_type: &'static str,
    cast_method: &'static str,
    child_accessors: &'static [(&'static str, &'static str)],
}

fn build_mapping_table() -> HashMap<&'static str, &'static NodeMapping> {
    let mappings: &[NodeMapping] = &[
        NodeMapping {
            parser_type: "send",
            prism_type: "CallNode",
            cast_method: "as_call_node",
            child_accessors: &[
                ("receiver", "receiver()"),
                ("method_name", "name()"),
                ("args", "arguments()"),
            ],
        },
        NodeMapping {
            parser_type: "csend",
            prism_type: "CallNode",
            cast_method: "as_call_node",
            child_accessors: &[
                ("receiver", "receiver()"),
                ("method_name", "name()"),
                ("args", "arguments()"),
            ],
        },
        NodeMapping {
            parser_type: "block",
            prism_type: "BlockNode",
            cast_method: "as_block_node",
            child_accessors: &[
                ("call", "call()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "def",
            prism_type: "DefNode",
            cast_method: "as_def_node",
            child_accessors: &[
                ("name", "name()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "defs",
            prism_type: "DefNode",
            cast_method: "as_def_node",
            child_accessors: &[
                ("recv", "receiver()"),
                ("name", "name()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "const",
            prism_type: "ConstantReadNode",
            cast_method: "as_constant_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "begin",
            prism_type: "BeginNode",
            cast_method: "as_begin_node",
            child_accessors: &[("body", "statements()")],
        },
        NodeMapping {
            parser_type: "pair",
            prism_type: "AssocNode",
            cast_method: "as_assoc_node",
            child_accessors: &[("key", "key()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "hash",
            prism_type: "HashNode",
            cast_method: "as_hash_node",
            child_accessors: &[("pairs", "elements()")],
        },
        NodeMapping {
            parser_type: "lvar",
            prism_type: "LocalVariableReadNode",
            cast_method: "as_local_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "ivar",
            prism_type: "InstanceVariableReadNode",
            cast_method: "as_instance_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "cvar",
            prism_type: "ClassVariableReadNode",
            cast_method: "as_class_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "gvar",
            prism_type: "GlobalVariableReadNode",
            cast_method: "as_global_variable_read_node",
            child_accessors: &[("name", "name()")],
        },
        NodeMapping {
            parser_type: "sym",
            prism_type: "SymbolNode",
            cast_method: "as_symbol_node",
            child_accessors: &[("value", "value()")],
        },
        NodeMapping {
            parser_type: "str",
            prism_type: "StringNode",
            cast_method: "as_string_node",
            child_accessors: &[("content", "content()")],
        },
        NodeMapping {
            parser_type: "int",
            prism_type: "IntegerNode",
            cast_method: "as_integer_node",
            child_accessors: &[("value", "value()")],
        },
        NodeMapping {
            parser_type: "float",
            prism_type: "FloatNode",
            cast_method: "as_float_node",
            child_accessors: &[("value", "value()")],
        },
        NodeMapping {
            parser_type: "true",
            prism_type: "TrueNode",
            cast_method: "as_true_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "false",
            prism_type: "FalseNode",
            cast_method: "as_false_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "nil",
            prism_type: "NilNode",
            cast_method: "as_nil_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "self",
            prism_type: "SelfNode",
            cast_method: "as_self_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "array",
            prism_type: "ArrayNode",
            cast_method: "as_array_node",
            child_accessors: &[("elements", "elements()")],
        },
        NodeMapping {
            parser_type: "if",
            prism_type: "IfNode",
            cast_method: "as_if_node",
            child_accessors: &[
                ("cond", "predicate()"),
                ("body", "statements()"),
                ("else", "subsequent()"),
            ],
        },
        NodeMapping {
            parser_type: "case",
            prism_type: "CaseNode",
            cast_method: "as_case_node",
            child_accessors: &[
                ("expr", "predicate()"),
                ("whens", "conditions()"),
                ("else", "else_clause()"),
            ],
        },
        NodeMapping {
            parser_type: "when",
            prism_type: "WhenNode",
            cast_method: "as_when_node",
            child_accessors: &[
                ("conds", "conditions()"),
                ("body", "statements()"),
            ],
        },
        NodeMapping {
            parser_type: "while",
            prism_type: "WhileNode",
            cast_method: "as_while_node",
            child_accessors: &[
                ("cond", "predicate()"),
                ("body", "statements()"),
            ],
        },
        NodeMapping {
            parser_type: "until",
            prism_type: "UntilNode",
            cast_method: "as_until_node",
            child_accessors: &[
                ("cond", "predicate()"),
                ("body", "statements()"),
            ],
        },
        NodeMapping {
            parser_type: "for",
            prism_type: "ForNode",
            cast_method: "as_for_node",
            child_accessors: &[
                ("var", "index()"),
                ("iter", "collection()"),
                ("body", "statements()"),
            ],
        },
        NodeMapping {
            parser_type: "return",
            prism_type: "ReturnNode",
            cast_method: "as_return_node",
            child_accessors: &[("args", "arguments()")],
        },
        NodeMapping {
            parser_type: "yield",
            prism_type: "YieldNode",
            cast_method: "as_yield_node",
            child_accessors: &[("args", "arguments()")],
        },
        NodeMapping {
            parser_type: "and",
            prism_type: "AndNode",
            cast_method: "as_and_node",
            child_accessors: &[("left", "left()"), ("right", "right()")],
        },
        NodeMapping {
            parser_type: "or",
            prism_type: "OrNode",
            cast_method: "as_or_node",
            child_accessors: &[("left", "left()"), ("right", "right()")],
        },
        NodeMapping {
            parser_type: "regexp",
            prism_type: "RegularExpressionNode",
            cast_method: "as_regular_expression_node",
            child_accessors: &[("content", "content()")],
        },
        NodeMapping {
            parser_type: "class",
            prism_type: "ClassNode",
            cast_method: "as_class_node",
            child_accessors: &[
                ("name", "constant_path()"),
                ("super", "superclass()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "module",
            prism_type: "ModuleNode",
            cast_method: "as_module_node",
            child_accessors: &[
                ("name", "constant_path()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "lvasgn",
            prism_type: "LocalVariableWriteNode",
            cast_method: "as_local_variable_write_node",
            child_accessors: &[("name", "name()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "ivasgn",
            prism_type: "InstanceVariableWriteNode",
            cast_method: "as_instance_variable_write_node",
            child_accessors: &[("name", "name()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "casgn",
            prism_type: "ConstantWriteNode",
            cast_method: "as_constant_write_node",
            child_accessors: &[("name", "name()"), ("value", "value()")],
        },
        NodeMapping {
            parser_type: "splat",
            prism_type: "SplatNode",
            cast_method: "as_splat_node",
            child_accessors: &[("expr", "expression()")],
        },
        NodeMapping {
            parser_type: "super",
            prism_type: "SuperNode",
            cast_method: "as_super_node",
            child_accessors: &[("args", "arguments()")],
        },
        NodeMapping {
            parser_type: "zsuper",
            prism_type: "ForwardingSuperNode",
            cast_method: "as_forwarding_super_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "lambda",
            prism_type: "LambdaNode",
            cast_method: "as_lambda_node",
            child_accessors: &[
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "dstr",
            prism_type: "InterpolatedStringNode",
            cast_method: "as_interpolated_string_node",
            child_accessors: &[("parts", "parts()")],
        },
        NodeMapping {
            parser_type: "dsym",
            prism_type: "InterpolatedSymbolNode",
            cast_method: "as_interpolated_symbol_node",
            child_accessors: &[("parts", "parts()")],
        },
        NodeMapping {
            parser_type: "args",
            prism_type: "ParametersNode",
            cast_method: "as_parameters_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "any_block",
            prism_type: "BlockNode",
            cast_method: "as_block_node",
            child_accessors: &[
                ("call", "call()"),
                ("params", "parameters()"),
                ("body", "body()"),
            ],
        },
        NodeMapping {
            parser_type: "cbase",
            prism_type: "ConstantPathNode",
            cast_method: "as_constant_path_node",
            child_accessors: &[],
        },
        NodeMapping {
            parser_type: "op-asgn",
            prism_type: "OperatorWriteNode",
            cast_method: "as_operator_write_node",
            child_accessors: &[
                ("target", "target()"),
                ("operator", "binary_operator()"),
                ("value", "value()"),
            ],
        },
    ];

    // SAFETY: We leak these mappings for 'static lifetime since the binary
    // runs once and exits. This avoids complex lifetime juggling for the table.
    let leaked: &'static [NodeMapping] = Box::leak(mappings.to_vec().into_boxed_slice());

    let mut table = HashMap::new();
    for mapping in leaked {
        table.insert(mapping.parser_type, mapping as &'static NodeMapping);
    }
    table
}

// Implement Clone for NodeMapping so we can put them in a Vec
impl Clone for NodeMapping {
    fn clone(&self) -> Self {
        Self {
            parser_type: self.parser_type,
            prism_type: self.prism_type,
            cast_method: self.cast_method,
            child_accessors: self.child_accessors,
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Rust Code Generator
// ---------------------------------------------------------------------------

struct CodeGenerator {
    mapping: HashMap<&'static str, &'static NodeMapping>,
    output: String,
    indent: usize,
    capture_count: usize,
    has_captures: bool,
    helper_stubs: Vec<String>,
}

impl CodeGenerator {
    fn new() -> Self {
        Self {
            mapping: build_mapping_table(),
            output: String::new(),
            indent: 0,
            capture_count: 0,
            has_captures: false,
            helper_stubs: Vec::new(),
        }
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn writeln(&mut self, s: &str) {
        let indent = self.indent_str();
        let _ = writeln!(self.output, "{indent}{s}");
    }

    /// Scan the pattern tree for captures to determine function signature.
    fn count_captures(node: &PatternNode) -> usize {
        match node {
            PatternNode::Capture(inner) => 1 + Self::count_captures(inner),
            PatternNode::NodeMatch { children, .. } => {
                children.iter().map(|c| Self::count_captures(c)).sum()
            }
            PatternNode::Alternatives(alts) => {
                // All branches must have same capture count; use max
                alts.iter().map(|a| Self::count_captures(a)).max().unwrap_or(0)
            }
            PatternNode::Conjunction(items) => {
                items.iter().map(|c| Self::count_captures(c)).sum()
            }
            PatternNode::Negation(inner) => Self::count_captures(inner),
            PatternNode::ParentRef(inner) => Self::count_captures(inner),
            PatternNode::DescendRef(inner) => Self::count_captures(inner),
            _ => 0,
        }
    }

    fn generate_pattern(
        &mut self,
        extracted: &ExtractedPattern,
        pattern: &PatternNode,
    ) -> String {
        self.output.clear();
        self.capture_count = 0;
        self.helper_stubs.clear();

        let num_captures = Self::count_captures(pattern);
        self.has_captures = num_captures > 0;

        let fn_name = extracted
            .method_name
            .trim_end_matches('?')
            .to_string();

        // Generate function signature
        if self.has_captures {
            self.writeln(&format!(
                "fn {fn_name}<'a>(node: &ruby_prism::Node<'a>) -> Option<MatchCapture<'a>> {{"
            ));
        } else {
            self.writeln(&format!(
                "fn {fn_name}(node: &ruby_prism::Node<'_>) -> bool {{"
            ));
        }
        self.indent += 1;

        // Generate the body
        self.generate_node_check(pattern, "node", true);

        // Return success
        if self.has_captures {
            // The captures are returned within the generation
        } else {
            self.writeln("true");
        }

        self.indent -= 1;
        self.writeln("}");

        let mut full_output = String::new();

        // Add capture type if needed
        if self.has_captures {
            let _ = writeln!(full_output, "// Capture result from {fn_name}");
            if num_captures == 1 {
                let _ = writeln!(
                    full_output,
                    "type MatchCapture<'a> = ruby_prism::Node<'a>;"
                );
            } else {
                let fields: Vec<String> = (0..num_captures)
                    .map(|_| "ruby_prism::Node<'a>".to_string())
                    .collect();
                let _ = writeln!(
                    full_output,
                    "type MatchCapture<'a> = ({});",
                    fields.join(", ")
                );
            }
            let _ = writeln!(full_output);
        }

        full_output.push_str(&self.output);

        // Add helper stubs
        for stub in &self.helper_stubs {
            let _ = writeln!(full_output);
            let _ = writeln!(full_output, "// Generated stub — cop must implement this");
            let _ = writeln!(
                full_output,
                "fn {stub}(node: &ruby_prism::Node<'_>) -> bool {{"
            );
            let _ = writeln!(
                full_output,
                "    todo!(\"implement #{stub} helper\")"
            );
            let _ = writeln!(full_output, "}}");
        }

        full_output
    }

    fn generate_node_check(&mut self, node: &PatternNode, var: &str, is_top: bool) {
        match node {
            PatternNode::NodeMatch {
                node_type,
                children,
            } => {
                self.generate_node_match(node_type, children, var, is_top);
            }
            PatternNode::Wildcard => {
                // No check needed — any value is OK
            }
            PatternNode::Rest => {
                // No check on remaining children
            }
            PatternNode::NilPredicate => {
                // nil? means receiver is None
                if self.has_captures {
                    self.writeln(&format!("if {var}.is_some() {{ return None; }}"));
                } else {
                    self.writeln(&format!(
                        "if {var}.is_some() {{ return false; }}"
                    ));
                }
            }
            PatternNode::NilLiteral => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var}.as_nil_node().is_none() {{ return {fail}; }}"
                ));
            }
            PatternNode::TrueLiteral => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var}.as_true_node().is_none() {{ return {fail}; }}"
                ));
            }
            PatternNode::FalseLiteral => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var}.as_false_node().is_none() {{ return {fail}; }}"
                ));
            }
            PatternNode::SymbolLiteral(name) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "// Check symbol :{name}"
                ));
                self.writeln(&format!(
                    "if {var} != b\"{name}\" {{ return {fail}; }}"
                ));
            }
            PatternNode::IntLiteral(n) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var} != {n} {{ return {fail}; }}"
                ));
            }
            PatternNode::StringLiteral(s) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "if {var} != b\"{s}\" {{ return {fail}; }}"
                ));
            }
            PatternNode::HelperCall(name) => {
                let fail = if self.has_captures { "None" } else { "false" };
                let fn_name = name.trim_end_matches('?');
                self.writeln(&format!(
                    "if !{fn_name}(&{var}) {{ return {fail}; }}"
                ));
                if !self.helper_stubs.contains(&fn_name.to_string()) {
                    self.helper_stubs.push(fn_name.to_string());
                }
            }
            PatternNode::Capture(inner) => {
                let cap_idx = self.capture_count;
                self.capture_count += 1;
                let cap_var = format!("capture_{cap_idx}");

                // First generate inner check, then capture the variable
                self.writeln(&format!("let {cap_var} = {var}.clone();"));
                self.generate_node_check(inner, var, false);

                // If this is the last statement before return, we use the capture
                if is_top {
                    self.writeln(&format!("return Some({cap_var});"));
                }
            }
            PatternNode::Alternatives(alts) => {
                self.generate_alternatives(alts, var);
            }
            PatternNode::Conjunction(items) => {
                for item in items {
                    self.generate_node_check(item, var, false);
                }
            }
            PatternNode::Negation(inner) => {
                self.generate_negation(inner, var);
            }
            PatternNode::TypePredicate(typ) => {
                let fail = if self.has_captures { "None" } else { "false" };
                let cast = self.mapping.get(typ.as_str()).map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!(
                        "if {var}.{cast}().is_none() {{ return {fail}; }}"
                    ));
                } else {
                    self.writeln(&format!("// Unknown type predicate: {typ}?"));
                }
            }
            PatternNode::ParamRef(param) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "// TODO: parameter reference %{param}"
                ));
                self.writeln(&format!(
                    "// if {var} != param_{param} {{ return {fail}; }}"
                ));
            }
            PatternNode::ParentRef(inner) => {
                self.writeln("// TODO: parent node reference (^)");
                self.writeln(&format!(
                    "// Check parent: {:?}",
                    pattern_summary(inner)
                ));
            }
            PatternNode::DescendRef(inner) => {
                self.writeln("// TODO: descend operator (`)");
                self.writeln(&format!(
                    "// Descend into children looking for: {:?}",
                    pattern_summary(inner)
                ));
            }
            PatternNode::Ident(name) => {
                // An identifier in child position — might be a node type check
                let fail = if self.has_captures { "None" } else { "false" };
                let cast = self.mapping.get(name.as_str()).map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!(
                        "if {var}.{cast}().is_none() {{ return {fail}; }}"
                    ));
                } else {
                    self.writeln(&format!("// Unknown identifier: {name}"));
                }
            }
            PatternNode::FloatLiteral(s) => {
                let fail = if self.has_captures { "None" } else { "false" };
                self.writeln(&format!(
                    "// Float check: {s}"
                ));
                self.writeln(&format!(
                    "// if {var}.value() != {s} {{ return {fail}; }}"
                ));
            }
        }
    }

    fn generate_node_match(
        &mut self,
        node_type: &str,
        children: &[PatternNode],
        var: &str,
        _is_top: bool,
    ) {
        let fail = if self.has_captures {
            "return None"
        } else {
            "return false"
        };
        if node_type == "_complex" {
            self.writeln(&format!("// Complex pattern — manual review needed"));
            for (i, child) in children.iter().enumerate() {
                self.writeln(&format!("// child[{i}]: {:?}", pattern_summary(child)));
            }
            return;
        }

        let Some(mapping) = self.mapping.get(node_type) else {
            self.writeln(&format!(
                "// Unmapped node type: {node_type}"
            ));
            self.writeln(&format!(
                "// TODO: add mapping for {node_type} to generate correct code"
            ));
            return;
        };

        // Copy mapping data to avoid borrow conflict with &mut self
        let cast = mapping.cast_method.to_string();
        let accessors: Vec<(&str, &str)> = mapping.child_accessors.to_vec();

        let typed_var = format!("{var}_{node_type}");

        // Cast the node
        if self.has_captures {
            self.writeln(&format!(
                "let {typed_var} = {var}.{cast}()?;"
            ));
        } else {
            self.writeln(&format!(
                "let Some({typed_var}) = {var}.{cast}() else {{ {fail}; }};"
            ));
        }

        // Special handling for csend — check call_operator
        if node_type == "csend" {
            self.writeln("// csend: verify safe navigation operator");
            self.writeln(&format!(
                "if {typed_var}.call_operator_loc().is_none() {{ {fail}; }}"
            ));
        }

        let mut accessor_idx = 0;

        for child in children {
            match child {
                PatternNode::Rest => {
                    // ... means skip remaining children
                    break;
                }
                PatternNode::Wildcard => {
                    // Skip this accessor — any value OK
                    accessor_idx += 1;
                }
                _ => {
                    if accessor_idx < accessors.len() {
                        let (_child_name, accessor) = accessors[accessor_idx];
                        let child_var = format!("{typed_var}_{accessor_idx}");

                        // Generate accessor call and child check
                        self.generate_child_access_and_check(
                            &typed_var,
                            accessor,
                            &child_var,
                            child,
                            node_type,
                            accessor_idx,
                        );
                        accessor_idx += 1;
                    } else {
                        self.writeln(&format!(
                            "// Extra child beyond known accessors: {:?}",
                            pattern_summary(child)
                        ));
                    }
                }
            }
        }
    }

    fn generate_child_access_and_check(
        &mut self,
        parent_var: &str,
        accessor: &str,
        child_var: &str,
        child: &PatternNode,
        parent_type: &str,
        child_idx: usize,
    ) {
        let fail = if self.has_captures { "None" } else { "false" };

        match child {
            PatternNode::NilPredicate => {
                // nil? on a child means that child should be None
                self.writeln(&format!(
                    "if {parent_var}.{accessor}.is_some() {{ return {fail}; }}"
                ));
            }
            PatternNode::SymbolLiteral(name) => {
                // Check method name or symbol value
                if accessor.contains("name") {
                    self.writeln(&format!(
                        "if {parent_var}.{accessor} != b\"{name}\" {{ return {fail}; }}"
                    ));
                } else {
                    // Might be a symbol node child — check value
                    self.writeln(&format!(
                        "// Check for symbol :{name} in {parent_type} child {child_idx}"
                    ));
                    self.writeln(&format!(
                        "let {child_var} = {parent_var}.{accessor};"
                    ));
                    self.writeln(&format!(
                        "// TODO: extract symbol value and compare to \"{name}\""
                    ));
                }
            }
            PatternNode::NodeMatch {
                node_type,
                children,
            } => {
                // Recurse into child node
                let is_optional = accessor.contains("receiver")
                    || accessor.contains("body")
                    || accessor.contains("subsequent")
                    || accessor.contains("superclass")
                    || accessor.contains("else_clause")
                    || accessor.contains("parameters");

                if is_optional {
                    if self.has_captures {
                        self.writeln(&format!(
                            "let {child_var} = {parent_var}.{accessor}.ok_or(())?;"
                        ));
                    } else {
                        self.writeln(&format!(
                            "let Some({child_var}) = {parent_var}.{accessor} else {{ return {fail}; }};"
                        ));
                    }
                } else {
                    self.writeln(&format!(
                        "let {child_var} = {parent_var}.{accessor};"
                    ));
                }
                self.generate_node_match(node_type, children, child_var, false);
            }
            PatternNode::Wildcard => {
                // No check needed
            }
            PatternNode::Rest => {
                // No check needed
            }
            PatternNode::Alternatives(alts) => {
                // For alternatives on a symbol/method name, generate OR checks
                if accessor.contains("name") {
                    self.generate_name_alternatives(parent_var, accessor, alts);
                } else {
                    self.writeln(&format!(
                        "let {child_var} = {parent_var}.{accessor};"
                    ));
                    self.generate_alternatives(alts, child_var);
                }
            }
            PatternNode::Capture(inner) => {
                let cap_idx = self.capture_count;
                self.capture_count += 1;
                self.writeln(&format!(
                    "let capture_{cap_idx} = {parent_var}.{accessor};"
                ));
                // Still validate the inner pattern
                let temp_var = format!("{child_var}_cap");
                self.writeln(&format!(
                    "let {temp_var} = {parent_var}.{accessor};"
                ));
                self.generate_node_check(inner, &temp_var, false);
            }
            PatternNode::HelperCall(name) => {
                let fn_name = name.trim_end_matches('?');
                self.writeln(&format!(
                    "let {child_var} = {parent_var}.{accessor};"
                ));
                self.writeln(&format!(
                    "if !{fn_name}(&{child_var}) {{ return {fail}; }}"
                ));
                if !self.helper_stubs.contains(&fn_name.to_string()) {
                    self.helper_stubs.push(fn_name.to_string());
                }
            }
            PatternNode::Negation(inner) => {
                self.writeln(&format!(
                    "let {child_var} = {parent_var}.{accessor};"
                ));
                self.generate_negation(inner, child_var);
            }
            PatternNode::Conjunction(items) => {
                self.writeln(&format!(
                    "let {child_var} = {parent_var}.{accessor};"
                ));
                for item in items {
                    self.generate_node_check(item, child_var, false);
                }
            }
            PatternNode::Ident(name) => {
                // An identifier as a child — probably a node type
                let cast = self.mapping.get(name.as_str()).map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!(
                        "if {parent_var}.{accessor}.{cast}().is_none() {{ return {fail}; }}"
                    ));
                } else {
                    self.writeln(&format!(
                        "// Identifier in child position: {name}"
                    ));
                }
            }
            PatternNode::IntLiteral(n) => {
                self.writeln(&format!(
                    "if {parent_var}.{accessor} != {n} {{ return {fail}; }}"
                ));
            }
            PatternNode::StringLiteral(s) => {
                self.writeln(&format!(
                    "if {parent_var}.{accessor} != b\"{s}\" {{ return {fail}; }}"
                ));
            }
            PatternNode::TypePredicate(typ) => {
                let cast = self.mapping.get(typ.as_str()).map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!(
                        "if {parent_var}.{accessor}.{cast}().is_none() {{ return {fail}; }}"
                    ));
                } else {
                    self.writeln(&format!(
                        "// Unknown type predicate: {typ}?"
                    ));
                }
            }
            _ => {
                self.writeln(&format!(
                    "// TODO: handle child pattern {:?}",
                    pattern_summary(child)
                ));
            }
        }
    }

    fn generate_alternatives(&mut self, alts: &[PatternNode], var: &str) {
        let fail = if self.has_captures { "None" } else { "false" };

        // Simple case: all alternatives are symbol literals (common for method name checks)
        let all_symbols = alts
            .iter()
            .all(|a| matches!(a, PatternNode::SymbolLiteral(_)));

        if all_symbols {
            let names: Vec<&str> = alts
                .iter()
                .filter_map(|a| match a {
                    PatternNode::SymbolLiteral(n) => Some(n.as_str()),
                    _ => None,
                })
                .collect();
            let conditions: Vec<String> = names
                .iter()
                .map(|n| format!("{var} == b\"{n}\""))
                .collect();
            self.writeln(&format!(
                "if !({})",
                conditions.join(" || ")
            ));
            self.writeln(&format!("{{ return {fail}; }}"));
            return;
        }

        // Simple case: all alternatives are identifiers (node types)
        let all_idents = alts
            .iter()
            .all(|a| matches!(a, PatternNode::Ident(_)));

        if all_idents {
            let mapping = &self.mapping;
            let checks: Vec<String> = alts
                .iter()
                .filter_map(|a| match a {
                    PatternNode::Ident(name) => {
                        mapping.get(name.as_str()).map(|m| {
                            format!("{var}.{}().is_some()", m.cast_method)
                        })
                    }
                    _ => None,
                })
                .collect();
            if !checks.is_empty() {
                self.writeln(&format!(
                    "if !({})",
                    checks.join(" || ")
                ));
                self.writeln(&format!("{{ return {fail}; }}"));
            }
            return;
        }

        // General case: use a closure or match block
        self.writeln(&format!("// Alternatives check on {var}"));
        self.writeln(&format!("let _matched = {{"));
        self.indent += 1;
        self.writeln("let mut matched = false;");

        for (i, alt) in alts.iter().enumerate() {
            self.writeln(&format!("// Alternative {i}"));
            match alt {
                PatternNode::NodeMatch { node_type, .. } => {
                    if let Some(mapping) = self.mapping.get(node_type.as_str()) {
                        let cast = mapping.cast_method.to_string();
                        self.writeln(&format!(
                            "if let Some(_alt) = {var}.{cast}() {{"
                        ));
                        self.indent += 1;
                        self.writeln("matched = true;");
                        self.indent -= 1;
                        self.writeln("}");
                    }
                }
                PatternNode::HelperCall(name) => {
                    let fn_name = name.trim_end_matches('?');
                    self.writeln(&format!("if {fn_name}(&{var}) {{ matched = true; }}"));
                    if !self.helper_stubs.contains(&fn_name.to_string()) {
                        self.helper_stubs.push(fn_name.to_string());
                    }
                }
                PatternNode::NilPredicate => {
                    self.writeln(&format!("if {var}.is_none() {{ matched = true; }}"));
                }
                PatternNode::Ident(name) => {
                    let cast = self.mapping.get(name.as_str()).map(|m| m.cast_method.to_string());
                    if let Some(cast) = cast {
                        self.writeln(&format!(
                            "if {var}.{cast}().is_some() {{ matched = true; }}"
                        ));
                    }
                }
                _ => {
                    self.writeln(&format!(
                        "// TODO: alternative {:?}",
                        pattern_summary(alt)
                    ));
                }
            }
        }

        self.writeln("matched");
        self.indent -= 1;
        self.writeln("};");
        self.writeln(&format!("if !_matched {{ return {fail}; }}"));
    }

    fn generate_name_alternatives(
        &mut self,
        parent_var: &str,
        accessor: &str,
        alts: &[PatternNode],
    ) {
        let fail = if self.has_captures { "None" } else { "false" };
        let names: Vec<String> = alts
            .iter()
            .filter_map(|a| match a {
                PatternNode::SymbolLiteral(n) => Some(n.clone()),
                PatternNode::Ident(n) => Some(n.clone()),
                _ => None,
            })
            .collect();

        if names.len() == alts.len() {
            let conditions: Vec<String> = names
                .iter()
                .map(|n| format!("{parent_var}.{accessor} == b\"{n}\""))
                .collect();
            self.writeln(&format!(
                "if !({})",
                conditions.join(" || ")
            ));
            self.writeln(&format!("{{ return {fail}; }}"));
        } else {
            self.writeln(&format!(
                "// Complex alternatives on {parent_var}.{accessor} — manual review needed"
            ));
        }
    }

    fn generate_negation(&mut self, inner: &PatternNode, var: &str) {
        let fail = if self.has_captures { "None" } else { "false" };

        match inner {
            PatternNode::NodeMatch { node_type, .. } => {
                let cast = self.mapping.get(node_type.as_str()).map(|m| m.cast_method.to_string());
                if let Some(cast) = cast {
                    self.writeln(&format!(
                        "if {var}.{cast}().is_some() {{ return {fail}; }}"
                    ));
                } else {
                    self.writeln(&format!("// Negation of unmapped type: {node_type}"));
                }
            }
            PatternNode::HelperCall(name) => {
                let fn_name = name.trim_end_matches('?');
                self.writeln(&format!(
                    "if {fn_name}(&{var}) {{ return {fail}; }}"
                ));
                if !self.helper_stubs.contains(&fn_name.to_string()) {
                    self.helper_stubs.push(fn_name.to_string());
                }
            }
            _ => {
                self.writeln(&format!(
                    "// TODO: negation of {:?}",
                    pattern_summary(inner)
                ));
            }
        }
    }
}

/// Produce a short summary string for a pattern node (for comments).
fn pattern_summary(node: &PatternNode) -> String {
    match node {
        PatternNode::NodeMatch { node_type, children } => {
            let child_summaries: Vec<String> =
                children.iter().map(pattern_summary).collect();
            format!("({node_type} {})", child_summaries.join(" "))
        }
        PatternNode::Wildcard => "_".to_string(),
        PatternNode::Rest => "...".to_string(),
        PatternNode::NilPredicate => "nil?".to_string(),
        PatternNode::NilLiteral => "nil".to_string(),
        PatternNode::TrueLiteral => "true".to_string(),
        PatternNode::FalseLiteral => "false".to_string(),
        PatternNode::SymbolLiteral(s) => format!(":{s}"),
        PatternNode::IntLiteral(n) => n.to_string(),
        PatternNode::FloatLiteral(s) => s.clone(),
        PatternNode::StringLiteral(s) => format!("\"{s}\""),
        PatternNode::Capture(inner) => format!("${}", pattern_summary(inner)),
        PatternNode::Alternatives(alts) => {
            let inner: Vec<String> = alts.iter().map(pattern_summary).collect();
            format!("{{{}}}", inner.join(" | "))
        }
        PatternNode::Conjunction(items) => {
            let inner: Vec<String> = items.iter().map(pattern_summary).collect();
            format!("[{}]", inner.join(" "))
        }
        PatternNode::Negation(inner) => format!("!{}", pattern_summary(inner)),
        PatternNode::HelperCall(name) => format!("#{name}"),
        PatternNode::TypePredicate(t) => format!("{t}?"),
        PatternNode::ParamRef(p) => format!("%{p}"),
        PatternNode::ParentRef(inner) => format!("^{}", pattern_summary(inner)),
        PatternNode::DescendRef(inner) => format!("`{}", pattern_summary(inner)),
        PatternNode::Ident(name) => name.clone(),
    }
}

// ---------------------------------------------------------------------------
// 6. CLI
// ---------------------------------------------------------------------------

fn print_usage() {
    eprintln!("Usage: node_pattern_codegen <generate|verify> <ruby_file> [rust_file]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  generate <ruby_file>              Parse Ruby cop file, output Rust matchers");
    eprintln!("  verify <ruby_file> <rust_file>     Compare generated vs existing Rust code");
}

fn cmd_generate(ruby_path: &str) -> io::Result<()> {
    let source = fs::read_to_string(ruby_path)?;
    let patterns = extract_patterns(&source);

    if patterns.is_empty() {
        eprintln!(
            "No def_node_matcher or def_node_search patterns found in {ruby_path}"
        );
        return Ok(());
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    writeln!(
        out,
        "// Auto-generated by node_pattern_codegen from {ruby_path}"
    )?;
    writeln!(
        out,
        "// Patterns extracted: {}",
        patterns.len()
    )?;
    writeln!(out, "//")?;
    writeln!(
        out,
        "// WARNING: This is generated scaffolding. Review and adapt before use."
    )?;
    writeln!(out)?;

    let mut codegen = CodeGenerator::new();

    for extracted in &patterns {
        let kind_label = match extracted.kind {
            PatternKind::Matcher => "def_node_matcher",
            PatternKind::Search => "def_node_search",
        };

        writeln!(out, "// --- {kind_label} :{} ---", extracted.method_name)?;
        writeln!(out, "// Pattern: {}", extracted.pattern.replace('\n', " "))?;

        // Lex
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();

        // Parse
        let mut parser = Parser::new(tokens);
        let Some(ast) = parser.parse() else {
            writeln!(
                out,
                "// ERROR: Failed to parse pattern for :{}",
                extracted.method_name
            )?;
            writeln!(out)?;
            continue;
        };

        // Generate
        let code = codegen.generate_pattern(extracted, &ast);
        writeln!(out, "{code}")?;

        // For search patterns, add a note
        if extracted.kind == PatternKind::Search {
            writeln!(
                out,
                "// NOTE: def_node_search yields all matching descendants."
            )?;
            writeln!(
                out,
                "// Wrap the above function in a tree-walk to search all nodes."
            )?;
            writeln!(out)?;
        }
    }

    Ok(())
}

fn cmd_verify(ruby_path: &str, rust_path: &str) {
    eprintln!("verify mode not yet implemented");
    eprintln!("  Ruby file: {ruby_path}");
    eprintln!("  Rust file: {rust_path}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        print_usage();
        process::exit(1);
    }

    let command = &args[1];
    match command.as_str() {
        "generate" => {
            let ruby_path = &args[2];
            if let Err(e) = cmd_generate(ruby_path) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        "verify" => {
            if args.len() < 4 {
                eprintln!("verify requires both <ruby_file> and <rust_file>");
                print_usage();
                process::exit(1);
            }
            cmd_verify(&args[2], &args[3]);
        }
        _ => {
            eprintln!("Unknown command: {command}");
            print_usage();
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_heredoc_pattern() {
        let source = r#"
        def_node_matcher :expect?, <<~PATTERN
          (send nil? :expect ...)
        PATTERN
        "#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "expect?");
        assert_eq!(patterns[0].kind, PatternKind::Matcher);
        assert!(patterns[0].pattern.contains("send nil? :expect"));
    }

    #[test]
    fn test_extract_inline_pattern() {
        let source =
            r#"def_node_search :gem_declarations, '(send nil? :gem str ...)'"#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "gem_declarations");
        assert_eq!(patterns[0].kind, PatternKind::Search);
    }

    #[test]
    fn test_lexer_basic() {
        let mut lexer = Lexer::new("(send nil? :expect ...)");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::LParen);
        assert_eq!(tokens[1], Token::Ident("send".to_string()));
        assert_eq!(tokens[2], Token::NilPredicate);
        assert_eq!(tokens[3], Token::SymbolLiteral("expect".to_string()));
        assert_eq!(tokens[4], Token::Rest);
        assert_eq!(tokens[5], Token::RParen);
    }

    #[test]
    fn test_lexer_alternatives() {
        let mut lexer = Lexer::new("{:first :take}");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::LBrace);
        assert_eq!(tokens[1], Token::SymbolLiteral("first".to_string()));
        assert_eq!(tokens[2], Token::SymbolLiteral("take".to_string()));
        assert_eq!(tokens[3], Token::RBrace);
    }

    #[test]
    fn test_lexer_capture() {
        let mut lexer = Lexer::new("$_");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::Capture);
        assert_eq!(tokens[1], Token::Wildcard);
    }

    #[test]
    fn test_lexer_helper_call() {
        let mut lexer = Lexer::new("#expect?");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::HelperCall("expect?".to_string()));
    }

    #[test]
    fn test_parser_simple_send() {
        let mut lexer = Lexer::new("(send nil? :expect ...)");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        match ast {
            PatternNode::NodeMatch { node_type, children } => {
                assert_eq!(node_type, "send");
                assert_eq!(children.len(), 3);
                assert!(matches!(children[0], PatternNode::NilPredicate));
                assert!(matches!(&children[1], PatternNode::SymbolLiteral(s) if s == "expect"));
                assert!(matches!(children[2], PatternNode::Rest));
            }
            _ => panic!("Expected NodeMatch"),
        }
    }

    #[test]
    fn test_parser_alternatives() {
        let mut lexer = Lexer::new("{:first | :take}");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        match ast {
            PatternNode::Alternatives(alts) => {
                assert_eq!(alts.len(), 2);
            }
            _ => panic!("Expected Alternatives"),
        }
    }

    #[test]
    fn test_parser_nested() {
        let mut lexer = Lexer::new("(send (send _ :where ...) :first)");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        match ast {
            PatternNode::NodeMatch { node_type, children } => {
                assert_eq!(node_type, "send");
                assert_eq!(children.len(), 2);
                match &children[0] {
                    PatternNode::NodeMatch { node_type, .. } => {
                        assert_eq!(node_type, "send");
                    }
                    _ => panic!("Expected inner NodeMatch"),
                }
            }
            _ => panic!("Expected NodeMatch"),
        }
    }

    #[test]
    fn test_codegen_simple_bool() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "where_method?".to_string(),
            pattern: "(send _ :where ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("fn where_method("));
        assert!(code.contains("-> bool"));
        assert!(code.contains("as_call_node"));
        assert!(code.contains("b\"where\""));
    }

    #[test]
    fn test_codegen_with_capture() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "find_method".to_string(),
            pattern: "(send _ ${:first :take})".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("fn find_method"));
        assert!(code.contains("Option"));
    }

    #[test]
    fn test_extract_multiple_patterns() {
        let source = r#"
        def_node_matcher :expect?, <<~PATTERN
          (send nil? :expect ...)
        PATTERN

        def_node_matcher :expect_block?, <<~PATTERN
          (block #expect? (args) _body)
        PATTERN
        "#;

        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].method_name, "expect?");
        assert_eq!(patterns[1].method_name, "expect_block?");
    }

    #[test]
    fn test_pattern_summary() {
        let node = PatternNode::NodeMatch {
            node_type: "send".to_string(),
            children: vec![
                PatternNode::Wildcard,
                PatternNode::SymbolLiteral("foo".to_string()),
                PatternNode::Rest,
            ],
        };
        let summary = pattern_summary(&node);
        assert_eq!(summary, "(send _ :foo ...)");
    }

    // ---------- Additional lexer tests ----------

    #[test]
    fn test_lexer_negation() {
        let mut lexer = Lexer::new("!nil?");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::Negation, Token::NilPredicate]);
    }

    #[test]
    fn test_lexer_conjunction() {
        let mut lexer = Lexer::new("[!nil? send_type?]");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0], Token::LBracket);
        assert_eq!(tokens[1], Token::Negation);
        assert_eq!(tokens[2], Token::NilPredicate);
        assert_eq!(tokens[3], Token::TypePredicate("send".to_string()));
        assert_eq!(tokens[4], Token::RBracket);
    }

    #[test]
    fn test_lexer_int_literal() {
        let mut lexer = Lexer::new("42");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::IntLiteral(42)]);
    }

    #[test]
    fn test_lexer_negative_int() {
        let mut lexer = Lexer::new("-1");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::IntLiteral(-1)]);
    }

    #[test]
    fn test_lexer_string_literal() {
        let mut lexer = Lexer::new("'hello'");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::StringLiteral("hello".to_string())]);
    }

    #[test]
    fn test_lexer_param_ref() {
        let mut lexer = Lexer::new("%1");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::ParamRef("1".to_string())]);
    }

    #[test]
    fn test_lexer_type_predicates() {
        for (input, expected_type) in [
            ("int?", "int"),
            ("str?", "str"),
            ("sym?", "sym"),
            ("float?", "float"),
            ("array?", "array"),
            ("hash?", "hash"),
            ("regexp?", "regexp"),
        ] {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(
                tokens,
                vec![Token::TypePredicate(expected_type.to_string())],
                "Failed for input: {input}"
            );
        }
    }

    #[test]
    fn test_lexer_cbase() {
        let mut lexer = Lexer::new("::");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::Ident("cbase".to_string())]);
    }

    #[test]
    fn test_lexer_complex_pattern() {
        let mut lexer = Lexer::new("(send (send nil? :expect ...) :to (send nil? :receive ...))");
        let tokens = lexer.tokenize();
        // Verify structure: ( send ( send nil? :expect ... ) :to ( send nil? :receive ... ) )
        assert_eq!(tokens[0], Token::LParen);
        assert_eq!(tokens[1], Token::Ident("send".to_string()));
        assert_eq!(tokens[2], Token::LParen);
        assert_eq!(tokens[3], Token::Ident("send".to_string()));
        assert_eq!(tokens[4], Token::NilPredicate);
        assert_eq!(tokens[5], Token::SymbolLiteral("expect".to_string()));
        assert_eq!(tokens[6], Token::Rest);
        assert_eq!(tokens[7], Token::RParen);
        assert_eq!(tokens[8], Token::SymbolLiteral("to".to_string()));
    }

    // ---------- Additional parser tests ----------

    #[test]
    fn test_parser_conjunction() {
        let mut lexer = Lexer::new("[!nil? send_type?]");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        match ast {
            PatternNode::Conjunction(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0], PatternNode::Negation(inner) if matches!(**inner, PatternNode::NilPredicate)));
                assert!(matches!(&items[1], PatternNode::TypePredicate(t) if t == "send"));
            }
            _ => panic!("Expected Conjunction, got {:?}", pattern_summary(&ast)),
        }
    }

    #[test]
    fn test_parser_capture_symbol() {
        let mut lexer = Lexer::new("${:first :take}");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        match ast {
            PatternNode::Capture(inner) => match *inner {
                PatternNode::Alternatives(alts) => {
                    assert_eq!(alts.len(), 2);
                    assert!(matches!(&alts[0], PatternNode::SymbolLiteral(s) if s == "first"));
                    assert!(matches!(&alts[1], PatternNode::SymbolLiteral(s) if s == "take"));
                }
                _ => panic!("Expected Alternatives inside Capture"),
            },
            _ => panic!("Expected Capture"),
        }
    }

    #[test]
    fn test_parser_nil_literal() {
        let mut lexer = Lexer::new("nil");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert!(matches!(ast, PatternNode::NilLiteral));
    }

    #[test]
    fn test_parser_helper_call() {
        let mut lexer = Lexer::new("(send #expect? _ ...)");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        match ast {
            PatternNode::NodeMatch { node_type, children } => {
                assert_eq!(node_type, "send");
                assert!(matches!(&children[0], PatternNode::HelperCall(n) if n == "expect?"));
                assert!(matches!(&children[1], PatternNode::Wildcard));
                assert!(matches!(&children[2], PatternNode::Rest));
            }
            _ => panic!("Expected NodeMatch"),
        }
    }

    #[test]
    fn test_parser_deeply_nested() {
        // (block (send (send nil? :described_class) :new ...) (args) _)
        let mut lexer = Lexer::new("(block (send (send nil? :described_class) :new ...) (args) _)");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        match ast {
            PatternNode::NodeMatch { node_type, children } => {
                assert_eq!(node_type, "block");
                assert_eq!(children.len(), 3);
                // First child is (send (send nil? :described_class) :new ...)
                match &children[0] {
                    PatternNode::NodeMatch { node_type, children } => {
                        assert_eq!(node_type, "send");
                        // It should have nested send, :new, ...
                        match &children[0] {
                            PatternNode::NodeMatch { node_type, .. } => {
                                assert_eq!(node_type, "send");
                            }
                            _ => panic!("Expected inner send node"),
                        }
                    }
                    _ => panic!("Expected NodeMatch for send"),
                }
            }
            _ => panic!("Expected NodeMatch for block"),
        }
    }

    // ---------- Additional codegen tests ----------

    #[test]
    fn test_codegen_nil_predicate() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "explicit_receiver?".to_string(),
            pattern: "(send nil? :foo)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("fn explicit_receiver("));
        assert!(code.contains("as_call_node"));
        assert!(code.contains("is_some()"));  // nil? -> receiver is_some check
        assert!(code.contains("b\"foo\""));
    }

    #[test]
    fn test_codegen_helper_generates_stub() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "match_with_helper?".to_string(),
            pattern: "(send #flow_command? _ ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("fn match_with_helper("), "Should generate main function");
        assert!(code.contains("flow_command"), "Should reference helper function");
        assert!(code.contains("Generated stub"), "Should generate helper stub");
        assert!(code.contains("todo!"), "Stub should have todo!()");
    }

    #[test]
    fn test_codegen_block_node() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "block_pass?".to_string(),
            pattern: "(block _ (args) _)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_block_node"), "Should cast to BlockNode");
    }

    #[test]
    fn test_codegen_def_node() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "method_def?".to_string(),
            pattern: "(def :initialize ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_def_node"), "Should cast to DefNode");
        assert!(code.contains("b\"initialize\""), "Should check method name");
    }

    #[test]
    fn test_codegen_alternatives_symbols() {
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "accessor?".to_string(),
            pattern: "(send nil? {:attr_reader :attr_writer :attr_accessor} ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("b\"attr_reader\""), "Should check attr_reader");
        assert!(code.contains("b\"attr_writer\""), "Should check attr_writer");
        assert!(code.contains("b\"attr_accessor\""), "Should check attr_accessor");
        assert!(code.contains("||"), "Should use OR for alternatives");
    }

    #[test]
    fn test_codegen_negation_helper() {
        // !#helper? in a pattern — negation of a helper call
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "not_lazy?".to_string(),
            pattern: "(send !#lazy? _ ...)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_call_node"), "Should cast to CallNode");
        assert!(code.contains("lazy"), "Should reference the helper in negation");
    }

    #[test]
    fn test_codegen_negation_node_match() {
        // !(send nil? :skip) — negation of a full node match
        let extracted = ExtractedPattern {
            kind: PatternKind::Matcher,
            method_name: "not_send?".to_string(),
            pattern: "(block !(send nil? :skip ...) _ _)".to_string(),
        };
        let mut lexer = Lexer::new(&extracted.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(&extracted, &ast);

        assert!(code.contains("as_block_node"), "Should cast to BlockNode");
        assert!(code.contains("as_call_node"), "Should reference CallNode for negated send");
    }

    // ---------- Ruby file reader edge cases ----------

    #[test]
    fn test_extract_with_double_quotes() {
        let source = "def_node_matcher :my_match?, \"(send _ :foo ...)\"";
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "my_match?");
        assert_eq!(patterns[0].pattern, "(send _ :foo ...)");
    }

    #[test]
    fn test_extract_no_patterns() {
        let source = "class MyCop < Base\n  def on_send(node)\n  end\nend";
        let patterns = extract_patterns(source);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_extract_search_pattern() {
        let source = r#"def_node_search :find_all_sends, '(send _ :puts ...)'"#;
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].kind, PatternKind::Search);
        assert_eq!(patterns[0].method_name, "find_all_sends");
    }

    #[test]
    fn test_extract_heredoc_with_single_quotes() {
        let source = "def_node_matcher :my_matcher?, <<~'PATTERN'\n  (send _ :bar)\nPATTERN";
        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].method_name, "my_matcher?");
        assert!(patterns[0].pattern.contains("send"));
    }

    // ---------- End-to-end tests ----------

    #[test]
    fn test_e2e_extract_lex_parse_generate() {
        let source = r#"
        def_node_matcher :where_take?, <<~PATTERN
          (send (send _ :where ...) {:first :take})
        PATTERN
        "#;

        let patterns = extract_patterns(source);
        assert_eq!(patterns.len(), 1);

        let pattern = &patterns[0];
        let mut lexer = Lexer::new(&pattern.pattern);
        let tokens = lexer.tokenize();
        assert!(!tokens.is_empty(), "Lexer should produce tokens");

        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parser should produce AST");

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(pattern, &ast);

        assert!(code.contains("fn where_take("), "Function name should be where_take");
        assert!(code.contains("as_call_node"), "Should cast to CallNode for send");
        assert!(code.contains("b\"where\""), "Should check for :where method name");
        assert!(code.contains("b\"first\""), "Should check for :first alternative");
        assert!(code.contains("b\"take\""), "Should check for :take alternative");
    }

    #[test]
    fn test_e2e_rspec_expect_pattern() {
        let source = "def_node_matcher :expect?, '(send nil? :expect ...)'";
        let patterns = extract_patterns(source);
        let pattern = &patterns[0];

        let mut lexer = Lexer::new(&pattern.pattern);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();

        let mut codegen = CodeGenerator::new();
        let code = codegen.generate_pattern(pattern, &ast);

        // Should generate a bool-returning function
        assert!(code.contains("-> bool"));
        assert!(code.contains("as_call_node"));
        // nil? should check receiver is None
        assert!(code.contains("is_some()"));
        assert!(code.contains("b\"expect\""));
    }

    // ---------- Mapping table tests ----------

    #[test]
    fn test_mapping_table_completeness() {
        let table = build_mapping_table();
        // Key node types that must be present
        for expected in &[
            "send", "csend", "block", "def", "defs", "const", "begin",
            "pair", "hash", "lvar", "ivar", "sym", "str", "int", "float",
            "true", "false", "nil", "self", "array", "if", "case", "when",
            "while", "until", "for", "return", "yield", "and", "or",
            "regexp", "class", "module", "lvasgn", "ivasgn", "casgn",
            "splat", "super", "zsuper", "lambda", "dstr", "dsym",
        ] {
            assert!(
                table.contains_key(expected),
                "Mapping table missing key node type: {expected}"
            );
        }
    }

    #[test]
    fn test_mapping_send_is_call_node() {
        let table = build_mapping_table();
        let send = table.get("send").unwrap();
        assert_eq!(send.prism_type, "CallNode");
        assert_eq!(send.cast_method, "as_call_node");
        assert!(!send.child_accessors.is_empty());
    }

    #[test]
    fn test_mapping_csend_same_as_send() {
        let table = build_mapping_table();
        let send = table.get("send").unwrap();
        let csend = table.get("csend").unwrap();
        assert_eq!(send.prism_type, csend.prism_type, "send and csend should map to same Prism type");
        assert_eq!(send.cast_method, csend.cast_method);
    }

    #[test]
    fn test_count_captures_simple() {
        let ast = PatternNode::NodeMatch {
            node_type: "send".to_string(),
            children: vec![
                PatternNode::Wildcard,
                PatternNode::Capture(Box::new(PatternNode::SymbolLiteral("foo".to_string()))),
                PatternNode::Rest,
            ],
        };
        assert_eq!(CodeGenerator::count_captures(&ast), 1);
    }

    #[test]
    fn test_count_captures_none() {
        let ast = PatternNode::NodeMatch {
            node_type: "send".to_string(),
            children: vec![PatternNode::Wildcard, PatternNode::Rest],
        };
        assert_eq!(CodeGenerator::count_captures(&ast), 0);
    }

    #[test]
    fn test_count_captures_nested() {
        let ast = PatternNode::NodeMatch {
            node_type: "send".to_string(),
            children: vec![
                PatternNode::Capture(Box::new(PatternNode::Wildcard)),
                PatternNode::Capture(Box::new(PatternNode::SymbolLiteral("x".to_string()))),
            ],
        };
        assert_eq!(CodeGenerator::count_captures(&ast), 2);
    }

    // ---------- Pattern summary round-trip tests ----------

    #[test]
    fn test_pattern_summary_alternatives() {
        let node = PatternNode::Alternatives(vec![
            PatternNode::SymbolLiteral("foo".to_string()),
            PatternNode::SymbolLiteral("bar".to_string()),
        ]);
        assert_eq!(pattern_summary(&node), "{:foo | :bar}");
    }

    #[test]
    fn test_pattern_summary_conjunction() {
        let node = PatternNode::Conjunction(vec![
            PatternNode::NilPredicate,
            PatternNode::TypePredicate("send".to_string()),
        ]);
        assert_eq!(pattern_summary(&node), "[nil? send?]");
    }

    #[test]
    fn test_pattern_summary_capture() {
        let node = PatternNode::Capture(Box::new(PatternNode::Wildcard));
        assert_eq!(pattern_summary(&node), "$_");
    }

    #[test]
    fn test_pattern_summary_negation() {
        let node = PatternNode::Negation(Box::new(PatternNode::NilPredicate));
        assert_eq!(pattern_summary(&node), "!nil?");
    }
}
