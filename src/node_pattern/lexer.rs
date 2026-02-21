//! NodePattern DSL lexer.
//!
//! Tokenizes RuboCop NodePattern strings like `(send nil? :expect ...)`.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    LBrace,           // {
    RBrace,           // }
    LBracket,         // [
    RBracket,         // ]
    Capture,          // $
    Wildcard,         // _
    Rest,             // ...
    Negation,         // !
    Pipe,             // | inside alternatives
    HelperCall(String),    // #method_name or #method_name?
    SymbolLiteral(String), // :sym
    IntLiteral(i64),
    FloatLiteral(String),
    StringLiteral(String),
    NilPredicate,          // nil?
    TruePredicate,         // true?
    FalsePredicate,        // false?
    TypePredicate(String), // int?, str?, sym?, etc.
    Ident(String),         // node type names: send, block, def, etc.
    ParamRef(String),      // %1, %param
    Caret,                 // ^ (parent node ref)
    Backtick,              // ` (descend operator)
}

pub struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
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

    pub fn tokenize(&mut self) -> Vec<Token> {
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
                        // Ruby symbols can be operator method names: :==, :===, :!=,
                        // :<=>, :<=, :>=, :<<, :>>, :+, :-, :*, :/, :%, :!, :[],
                        // :[]=, :!~, :=~, :&, :|, :^, :~, :**
                        let name = if self
                            .peek()
                            .is_some_and(|c| b"=<>!~+*&|^/%-.".contains(&c))
                        {
                            self.read_while(|c| b"=<>!~+*&|^/%-.[]".contains(&c))
                        } else {
                            self.read_while(|c| Self::is_ident_char(c) || c == b'?')
                        };
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
                _ if ch.is_ascii_digit()
                    || (ch == b'-'
                        && self
                            .input
                            .get(self.pos + 1)
                            .is_some_and(|c| c.is_ascii_digit())) =>
                {
                    let num_str =
                        self.read_while(|c| c.is_ascii_digit() || c == b'-' || c == b'.');
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
                        "send_type?" => {
                            tokens.push(Token::TypePredicate("send".to_string()));
                        }
                        "block_type?" => {
                            tokens.push(Token::TypePredicate("block".to_string()));
                        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut lexer =
            Lexer::new("(send (send nil? :expect ...) :to (send nil? :receive ...))");
        let tokens = lexer.tokenize();
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
}
