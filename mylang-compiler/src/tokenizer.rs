use std::io::{BufReader, Read, BufRead};
use std::collections::VecDeque as Queue;

use crate::ast::{StrRef, IdStr};

#[derive(Clone,Debug,Eq,PartialEq)]
pub enum TokenType {
    Invalid(StrRef),
    Id(IdStr), Type(IdStr),
    Int(StrRef), Dec(StrRef), Str(StrRef), Bool(bool),
    Op(IdStr), Arrow, Equal,
    Dot, Comma,
    Colon, Semicolon,
    OpenParen, CloseParen,
    OpenSquare, CloseSquare,
    OpenBracket, CloseBracket,
    Pub,
    Alias, Newtype, Struct, Union, Enum, Trait,
    Impl, Of, For,
    Var, Def, Virt, Pure, Macro, Extern,
    Return, Break, Continue, Do,
    EOF,
}

impl TokenType {
    pub fn str(&self) -> StrRef {
        use TokenType::*;
        return match self {
            Invalid(s)  => s.clone(),
            Id(i)  => i.clone(),
            Type(i)  => i.clone(),
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

#[derive(Clone,Copy,Debug)]
pub struct Location {
    line: u16,
    char: u16,
}

impl Location {
    pub fn str(&self) -> StrRef {
        return format!("line: {}, char: {}", self.line, self.char).into();
    }

    fn inc_line(&mut self) {
        self.line += 1;
        self.char = 0;
    }

    fn inc_char(&mut self, count: u16) {
        self.char += count;
    }
}

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
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

    fn to_token(&self, token_type: TokenType) -> Token {
        return Token{token_type, location: self.location};
    }

    fn tokenize(&mut self) -> Token {
        fn is_delimiter(c: char) -> bool {
            return ",:;()[]{}".chars().any(|e| c == e);
        }
        fn from_delimiter(c: char) -> TokenType {
            use TokenType::*;
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
                    return self.to_token(TokenType::EOF);
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
                                token = self.to_token(TokenType::Arrow);
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
                                token = self.to_token(TokenType::Equal);
                                break 'build_token;
                            }
                            '.' => {
                                token = self.to_token(TokenType::Dot);
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
                                token = self.to_token(TokenType::Invalid(ch.to_string().into()));
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
                                token = self.to_token(TokenType::Invalid(self.word.to_owned().into()));
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
                                token = self.to_token(TokenType::Invalid(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsDecimal => {
                                token = self.to_token(TokenType::Dec(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsNumber => {
                                token = self.to_token(TokenType::Int(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsOperator => {
                                token = self.to_token(TokenType::Op(self.word.to_owned().into()));
                                break 'build_token;
                            }
                            IsComment{is_line: _, block_depth: _} => {
                                token = self.to_token(TokenType::EOF);
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
            use TokenType::*;
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
                    "def"       => Def,
                    "virt"      => Virt,
                    "pure"      => Pure,
                    "macro"     => Macro,
                    "extern"    => Extern,
                    "return"    => Return,
                    "break"     => Break,
                    "continue"  => Continue,
                    "do"        => Do,
                    _ => Id(w.into()),
                }
                IsType => Type(w.into()),
                IsString => Str(w.into()),
                IsNumber => Int(w.into()),
                IsDecimal => Dec(w.into()),
                IsOperator => Op(w.into()),
                IsComment{is_line: _, block_depth: _} => panic!("Comment state should not be reached"),
            });

            break 'build_token;
        }

        self.reader.consume(consumed);
        self.word.clear();

        return token;
    }

    pub fn next(&mut self) -> Token {
        return self.peeked.pop_front().unwrap_or_else(|| self.tokenize());
    }

    pub fn peek(&mut self, n: usize) -> &Token {
        while self.peeked.len() <= n {
            let token = self.tokenize();
            self.peeked.push_back(token);
        }
        return self.peeked.get(n).unwrap();
    }

    pub fn str(&mut self) -> StrRef {
        let mut string = String::new();
        let mut indent = 0;
        let mut new_line = true;
        let mut index = 0;

        loop {
            use TokenType::*;
            let token = &self.peek(index).token_type;
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
