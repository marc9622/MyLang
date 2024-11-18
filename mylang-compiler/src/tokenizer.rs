use std::io::{BufReader, Read, BufRead};
use std::collections::VecDeque as Queue;

use crate::ast::{RcStr, Location};

#[derive(Clone,Debug,Eq,PartialEq)]
pub enum TokenKind {
    Invalid(RcStr),
    Id(RcStr), Type(RcStr),
    Int(RcStr), Dec(RcStr), Str(RcStr), Bool(bool), // NOTE: The builtin Bool type should at some point be replaced by a type from the standard library
    Op(RcStr), Arrow, Equal,
    Dot, Comma,
    Colon, Semicolon,
    OpenParen, CloseParen,
    OpenSquare, CloseSquare,
    OpenBracket, CloseBracket,
    Pub,
    Alias, Newtype, Struct, Union, Enum, Trait,
    Impl, Of, For,
    Var, Let, Def, Virt, Pure, Macro, Extern,
    Return, Break, Continue, Do,
    EOF,
}

impl TokenKind {
    pub fn str(&self) -> RcStr {
        use TokenKind::*;
        return match self {
            Invalid(s)  => s.clone(),
            Id(i)       => i.clone(),
            Type(i)     => i.clone(),
            Int(i)      => i.clone(),
            Dec(s)      => s.clone(),
            Str(s)      => s.clone(),
            Bool(true)  => "true".into(),
            Bool(false) => "false".into(),
            Op(i)       => i.clone(),
            Arrow       => "->".into(),
            Equal       => "=".into(),
            Dot         => ".".into(),
            Comma       => ",".into(),
            Colon       => ":".into(),
            Semicolon   => ";".into(),
            OpenParen   => "(".into(),
            CloseParen  => ")".into(),
            OpenSquare  => "[".into(),
            CloseSquare => "]".into(),
            OpenBracket => "{".into(),
            CloseBracket=> "}".into(),
            Pub         => "pub".into(),
            Alias       => "alias".into(),
            Newtype     => "newtype".into(),
            Struct      => "struct".into(),
            Union       => "union".into(),
            Enum        => "enum".into(),
            Trait       => "trait".into(),
            Impl        => "impl".into(),
            Of          => "of".into(),
            For         => "for".into(),
            Var         => "var".into(),
            Let         => "let".into(),
            Def         => "def".into(),
            Virt        => "virt".into(),
            Pure        => "pure".into(),
            Macro       => "macro".into(),
            Extern      => "extern".into(),
            Return      => "return".into(),
            Break       => "break".into(),
            Continue    => "continue".into(),
            Do          => "do".into(),
            EOF         => "<EOF>".into(),
        };
    }
}

#[derive(Debug)]
pub struct Token {
    pub token_kind: TokenKind,
    pub location: Location,
}

pub struct Tokenizer<R: Read> {
    reader: BufReader<R>,
    word: String,
    location: Location,
    peeked: Queue<Token>,
}

impl<R: Read> Tokenizer<R> {
    pub fn new(read: R) -> Tokenizer<R> {
        return Tokenizer{
            reader: BufReader::new(read),
            word: String::new(),
            location: Location{line: 1, char: 1},
            peeked: Queue::new(),
        };
    }

    fn to_token(&self, token_type: TokenKind) -> Token {
        return Token{token_kind: token_type, location: self.location};
    }

    fn tokenize(&mut self) -> Token {
        fn is_delimiter(c: char) -> bool {
            return ",:;()[]{}".chars().any(|e| c == e);
        }
        fn from_delimiter(c: char) -> TokenKind {
            use TokenKind::*;
            return match c {
                ',' => Comma,
                ':' => Colon,
                ';' => Semicolon,
                '(' => OpenParen,
                ')' => CloseParen,
                '[' => OpenSquare,
                ']' => CloseSquare,
                '{' => OpenBracket,
                '}' => CloseBracket,
                _ => panic!("Not a delimiter"),
            };
        }
        fn is_operator_symbol(c: char) -> bool {
            return "+-*/<>=".chars().any(|e| c == e);
        }
        #[derive(PartialEq,Eq)]
        enum TokenState {
            IsEmpty,
            IsId,
            IsType,
            IsString,
            IsNumber,
            IsDecimal,
            IsOperator,
            IsComment{
                is_line: bool,
                block_depth: u16
            },
        }

        let mut consumed = 0;
        let token: Token;

        'build_token: loop {
            use TokenState::*;
            let mut state = IsEmpty;

            'build_word: while self.word.is_empty() {
                let bytes = self.reader.fill_buf().unwrap(); // NOTE: Constantly refilling the buffer might be dumb

                if bytes.is_empty() {
                    return self.to_token(TokenKind::EOF);
                }

                let mut c = match bytes.get(consumed) {
                    Some(b) => *b as char,
                    None => break 'build_word,
                };

                let mut next = bytes.get(consumed + 1).map(|b| *b as char);

                loop {
                    match state {
                        IsEmpty => match c {
                            '\n' => {
                                consumed += 1;
                                self.location.inc_line();
                            }
                            ws if ws.is_ascii_whitespace() => {
                                consumed += 1;
                                self.location.inc_char(1);
                            }
                            '/' if next == Some('/') => {
                                consumed += 2;
                                self.location.inc_char(2);
                                state = IsComment{is_line: true, block_depth: 0};
                            }
                            '/' if next == Some('*') => {
                                consumed += 2;
                                self.location.inc_char(2);
                                state = IsComment{is_line: false, block_depth: 1};
                            }
                            '"' => {
                                consumed += 1;
                                self.location.inc_char(1);
                                state = IsString;
                            }
                            '-' if next == Some('>') => {
                                token = self.to_token(TokenKind::Arrow);
                                consumed += 2;
                                self.location.inc_char(2);
                                break 'build_token;
                            }
                            '=' => 'block: {
                                consumed += 1;
                                self.location.inc_char(1);
                                if let Some(op) = next { if is_operator_symbol(op) { // Waiting for better if-let expressions
                                    state = IsOperator;
                                    self.word.push(op);
                                    break 'block;
                                }}
                                token = self.to_token(TokenKind::Equal);
                                break 'build_token;
                            }
                            '.' => {
                                token = self.to_token(TokenKind::Dot);
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_token;
                            }
                            nu if nu.is_numeric() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                state = IsNumber;
                                self.word.push(nu);
                            }
                            op if is_operator_symbol(op) => {
                                consumed += 1;
                                self.location.inc_char(1);
                                state = IsOperator;
                                self.word.push(op);
                            }
                            de if is_delimiter(de) => {
                                token = self.to_token(from_delimiter(de));
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_token;
                            }
                            ch if ch.is_alphanumeric() && ch.is_uppercase() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                state = IsType;
                                self.word.push(ch);
                            }
                            ch if ch.is_alphanumeric() && ch.is_lowercase() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                state = IsId;
                                self.word.push(ch);
                            }
                            '_' => {
                                consumed += 1;
                                self.location.inc_char(1);
                                state = IsId;
                                self.word.push('_');
                            }
                            '#' => {
                                consumed += 1;
                                self.location.inc_char(1);
                                state = IsOperator;
                                self.word.push('#');
                            }
                            ch => {
                                token = self.to_token(TokenKind::Invalid(ch.to_string().into()));
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_token;
                            }
                        }
                        IsId => match c {
                            '\n' => {
                                consumed += 1;
                                self.location.inc_line();
                                break 'build_word;
                            }
                            ws if ws.is_ascii_whitespace() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_word;
                            }
                            ch if ch.is_alphanumeric() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                self.word.push(ch);
                            }
                            '_' => {
                                consumed += 1;
                                self.location.inc_char(1);
                                self.word.push('_');
                            }
                            _ => {
                                break 'build_word;
                            }
                        }
                        IsType => match c {
                            '\n' => {
                                consumed += 1;
                                self.location.inc_line();
                                break 'build_word;
                            }
                            ws if ws.is_ascii_whitespace() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_word;
                            }
                            ch if ch.is_alphanumeric() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                self.word.push(ch);
                            }
                            _ => {
                                break 'build_word;
                            }
                        }
                        IsString => match c {
                            '\n' => {
                                token = self.to_token(TokenKind::Invalid(self.word.to_owned().into()));
                                consumed += 1;
                                self.location.inc_line();
                                break 'build_token;
                            }
                            '"' => {
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_word;
                            }
                            ch => {
                                consumed += 1;
                                self.location.inc_char(1);
                                self.word.push(ch);
                            }
                        }
                        IsNumber => match c {
                            '\n' => {
                                consumed += 1;
                                self.location.inc_line();
                                break 'build_word;
                            }
                            ws if ws.is_ascii_whitespace() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_word;
                            }
                            '.' => 'block: {
                                if let Some(nu) = next { if nu.is_numeric() { // Waiting for better better if-let expressions
                                    consumed += 2;
                                    self.location.inc_char(2);
                                    state = IsDecimal;
                                    self.word.push('.');
                                    self.word.push(nu);
                                    break 'block;
                                }}
                                break 'build_word;
                            }
                            ch if ch.is_numeric() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                self.word.push(ch);
                            }
                            _ => {
                                break 'build_word;
                            }
                        }
                        IsDecimal => match c {
                            '\n' => {
                                consumed += 1;
                                self.location.inc_line();
                                break 'build_word;
                            }
                            ws if ws.is_ascii_whitespace() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_word;
                            }
                            ch if ch.is_numeric() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                self.word.push(ch);
                            }
                            _ => {
                                break 'build_word;
                            }
                        }
                        IsOperator => match c {
                            '\n' => {
                                consumed += 1;
                                self.location.inc_line();
                                break 'build_word;
                            }
                            ws if ws.is_ascii_whitespace() => {
                                consumed += 1;
                                self.location.inc_char(1);
                                break 'build_word;
                            }
                            '/' if next == Some('/') || next == Some('*') => {
                                break 'build_word;
                            }
                            '-' if next == Some('>') => {
                                break 'build_word;
                            }
                            nu if nu.is_numeric() => {
                                break 'build_word;
                            }
                            op if is_operator_symbol(op) => {
                                consumed += 1;
                                self.location.inc_char(1);
                                self.word.push(op);
                            }
                            de if is_delimiter(de) => {
                                break 'build_word;
                            }
                            _ => {
                                break 'build_word;
                            }
                        }
                        IsComment{is_line, block_depth} => {
                            if is_line { match c {
                                '\n' => {
                                    consumed += 1;
                                    if block_depth == 0 {
                                        state = IsEmpty;
                                    } else {
                                        state = IsComment{is_line: true, block_depth};
                                    };
                                    self.location.inc_line();
                                }
                                _ => {
                                    consumed += 1;
                                    self.location.inc_char(1);
                                }
                            }}
                            else if !is_line { match c {
                                '\n' => {
                                    consumed += 1;
                                    self.location.inc_line();
                                }
                                '/' if next == Some('/') => {
                                    consumed += 2;
                                    self.location.inc_char(2);
                                    state = IsComment{is_line: true, block_depth};
                                }
                                '/' if next == Some('*') => {
                                    consumed += 2;
                                    self.location.inc_char(2);
                                    state = IsComment{is_line: false, block_depth: block_depth + 1};
                                }
                                '*' if next == Some('/') => {
                                    consumed += 2;
                                    self.location.inc_char(2);
                                    if block_depth == 0 {
                                        state = IsEmpty;
                                    } else {
                                        state = IsComment{is_line: false, block_depth: block_depth - 1};
                                    }
                                }
                                _ => {
                                    consumed += 1;
                                    self.location.inc_char(1);
                                }
                            }}
                        }
                    }

                    c = match bytes.get(consumed) {
                        Some(b) => *b as char,
                        None => { match state {
                            IsString => {
                                token = self.to_token(TokenKind::Invalid(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsDecimal => {
                                token = self.to_token(TokenKind::Dec(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsNumber => {
                                token = self.to_token(TokenKind::Int(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsOperator => {
                                token = self.to_token(TokenKind::Op(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsComment{is_line: _, block_depth: _} => {
                                token = self.to_token(TokenKind::EOF);
                                break 'build_token;
                            }
                            _ => {
                                break 'build_word;
                            }
                        }}
                    };

                    next = bytes.get(consumed + 1).map(|b| *b as char);
                }
            }

            // Create token
            use TokenKind::*;
            let w = self.word.as_str();
            token = self.to_token(match state {
                IsEmpty => EOF,
                IsId => match w {
                    "true"      => Bool(true),
                    "false"     => Bool(false),
                    "pub"       => Pub,
                    "alias"     => Alias,
                    "newtype"   => Newtype,
                    "struct"    => Struct,
                    "union"     => Union,
                    "enum"      => Enum,
                    "trait"     => Trait,
                    "impl"      => Impl,
                    "of"        => Of,
                    "for"       => For,
                    "var"       => Var,
                    "let"       => Let,
                    "def"       => Def,
                    "virt"      => Virt,
                    "pure"      => Pure,
                    "macro"     => Macro,
                    "extern"    => Extern,
                    "return"    => Return,
                    "break"     => Break,
                    "continue"  => Continue,
                    "do"        => Do,
                    _ => Id(self.word.as_str().into()),
                }
                IsType => Type(self.word.as_str().into()),
                IsString => Str(RcStr::from(w)),
                IsNumber => Int(RcStr::from(w)),
                IsDecimal => Dec(RcStr::from(w)),
                IsOperator => Op(RcStr::from(w)),
                IsComment{is_line: _, block_depth: _} => panic!("Comment state should not be reached"),
            });

            break 'build_token;
        }

        self.reader.consume(consumed);
        self.word.clear();

        return token;
    }

    /// Returns the next token and consumes it.
    /// (Will reuse peeked tokens if possible)
    pub fn next(&mut self) -> Token {
        if self.peeked.is_empty() {
            return self.tokenize();
        }
        return unsafe {self.peeked.pop_front().unwrap_unchecked()};
    }

    /// Returns the n'th token after the last consumed token.
    /// It does not consume it, meaning that it will not affect the next call to next().
    /// 'n' is 0-indexed, meaning that peek(0) returns the next token.
    pub fn peek(&mut self, n: usize) -> &Token {
        while self.peeked.len() <= n {
            let token = self.tokenize();
            self.peeked.push_back(token);
        }
        return self.peeked.get(n).unwrap();
    }

    /// Consumes the next peeked token.
    /// Panics if there are no peeked tokens.
    pub fn consume_peeked(&mut self) {
        if self.peeked.is_empty() {
            panic!("No peeked tokens to consume");
        }
        self.peeked.pop_front();
    }

    /// Returns a string representation of alle tokens until an EOF.
    /// Does not consume any tokens.
    pub fn str(&mut self) -> RcStr {
        let mut string = String::new();
        let mut indent = 0;
        let mut new_line = true;
        let mut index = 0;

        loop {
            use TokenKind::*;
            let token = &self.peek(index).token_kind;
            match token {
                EOF => break,
                OpenBracket => {
                    if new_line { for _ in 0..indent {
                        string.push_str("    ");
                    }}
                    string.push_str("{\n");
                    indent += 1;
                    new_line = true;
                }
                CloseBracket => {
                    indent -= 1;
                    if new_line { for _ in 0..indent {
                        string.push_str("    ");
                    }}
                    string.push_str("}\n");
                    new_line = true;
                }
                Semicolon => {
                    if new_line { for _ in 0..indent {
                        string.push_str("    ");
                    }}
                    string.push_str(";\n");
                    new_line = true;
                }
                token => {
                    if new_line { for _ in 0..indent {
                        string.push_str("    ");
                    }}
                    string.push_str(&token.str());
                    string.push(' ');
                    new_line = false;
                }
            }
            index += 1;
        }

        return string.into();
    }
}
