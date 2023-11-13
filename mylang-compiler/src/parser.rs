use std::{io::Read, fmt::Debug};
use std::collections::VecDeque;
use crate::ast::{Namespace, ScopeRef, Declaration, ScopedId, Expression, IdentifierType, Scope, StrRef, DeclType, DeclKeyword, FuncType, IdStr, self};
use crate::tokenizer::{Tokenizer, Location, Token, TokenType};

#[allow(dead_code)]
pub struct ParseError {
    message: StrRef,
    location: Location,
}

impl ParseError {
    fn unexpected<T>(token: Token, expected: &[&'static str]) -> ParseResult<T> {
        let mut message = format!("Unexpected token `{}` at {}. Expected ", token.token_type.str(), token.location.str());
        let mut first = true;
        for string in expected {
            if first {
                first = false;
                message.push_str(&format!("`{}`", string));
            } else {
                message.push_str(&format!(", `{}`", string));
            }
        }
        return Err(ParseError{
            message: message.into(),
            location: token.location,
        });
    }

    fn message<T>(token: Token, message: &'static str) -> ParseResult<T> {
        return Err(ParseError{
            message: message.into(),
            location: token.location,
        });
    }
}

impl Debug for ParseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return fmt.write_str(&self.message);
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

//type Stack<T> = Vec<T>;
type Queue<T> = VecDeque<T>;

pub struct Parser {
    ast: Namespace,
    unknown_ids: Queue<*mut ScopedId>,
}

impl Parser {
    pub fn new(name: &'static str) -> Parser {
        return Parser{
            ast: Namespace::new_root(name.into()),
            unknown_ids: Queue::new(),
        };
    }

    pub fn parse<R: Read>(&mut self, mut tokenizer: Tokenizer<R>) -> ParseResult<()> {
        use TokenType::*;
        loop {
            let token = tokenizer.next();
            match token.token_type {
                Var => {
                    let declaration = self.parse_declaration_keyword(DeclKeyword::Var, &self.ast, &mut tokenizer)?;
                    self.ast.declarations.push(declaration);
                },
                Def => {
                    let declaration = self.parse_declaration_keyword(DeclKeyword::Def, &self.ast, &mut tokenizer)?;
                    self.ast.declarations.push(declaration);
                }
                EOF => return Ok(()),
                _ => todo!(),
            }
        }
    }

    fn parse_declaration_keyword<R: Read, S: Scope>(&mut self, decl_keyword: DeclKeyword, module: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        use TokenType::*;
        let token = tokenizer.next();
        match token.token_type {
            Id(identifier) => return self.parse_declaration(decl_keyword, identifier, module, tokenizer),
            OpenBracket => todo!("Add destructuring"),
            _ => return ParseError::unexpected(token, &["identifier"]),
        }
    }

    fn parse_declaration<R: Read, S: Scope>(&mut self, decl_keyword: DeclKeyword, identifier: IdStr, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        use TokenType::*;
        let token = tokenizer.next();
        match token.token_type {
            Id(identifier) => {
                let token = tokenizer.next();
                use DeclType::*;
                match token.token_type {
                    Equal => {
                        let expression = self.parse_expression_semicolon(scope, tokenizer)?;
                        return Ok(Declaration{decl_keyword, identifier, decl_type: ValueDecl{value_type: None}, expression})
                    }
                    OpenParen => return self.parse_func_declaration(decl_keyword, identifier, scope, tokenizer),
                    _ => return ParseError::unexpected(token, &["=","("]),
                };
            }
            _ => return ParseError::unexpected(token, &["identifier"]),
        };
    }

    fn parse_func_declaration<R: Read, S: Scope>(&mut self, decl_keyword: DeclKeyword, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        let mut parameters = Vec::new();
        loop {
            use TokenType::*;
            let token = tokenizer.next();
            match token.token_type {
                Id(identifier) => {
                    let parameter = self.parse_declaration(DeclKeyword::Var, identifier, scope, tokenizer)?;
                    parameters.push(parameter);
                }
                Def => {
                    let parameter = self.parse_declaration_keyword(DeclKeyword::Def, scope, tokenizer);
                }
                CloseParen => {
                    let token = tokenizer.next();
                    match token.token_type {
                        Arrow => return self.parse_func_body(DeclKeyword::Def, parameters.into(), scope, tokenizer),
                        _ => return ParseError::unexpected(token, &["->"]),
                    }
                }
                _ => return ParseError::unexpected(token, &["identifier"]),
            }
        }
    }

    fn parse_func_body<R: Read, S: Scope>(&mut self, decl_keyword: DeclKeyword, parameters: Box<[Declaration]>, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        use TokenType::*;
        let token = tokenizer.next();
        match token.token_type {
            OpenBracket => {
                let block = self.parse_block(scope, tokenizer);
                let return_type = Box::new(ast::Type::Inferred);
                return Ok(Declaration{
                    decl_keyword,

                        FuncType{parameters, return_type}
                });
            }
            Do => {
                let expression = self.parse_expression_semicolon(scope, tokenizer);
                let return_type = Box::new(ast::Type::Inferred);
                return Ok(FuncType{parameters: parameters.into(), return_type});
            }
            _ => return ParseError::unexpected(token, &["{","do"]),
        }
    }

    fn parse_expression_semicolon<R: Read, S: Scope>(&mut self, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Expression> {
        use TokenType::*;
        let token = tokenizer.next();
        match token.token_type {
            Id(identifier) => {
                let token = tokenizer.next();
                match token.token_type {
                    Semicolon => {
                        use IdentifierType::*;
                        let mut u_id = Box::new(ScopedId{
                            id_type: Unresolved{
                                scope_used: ScopeRef::from_ptr(scope),
                                scope_described: Box::new([])
                            },
                            name: identifier.clone(),
                        });
                        self.unknown_ids.push_back(u_id.as_mut());
                        return Ok(Expression::Identifier(u_id));
                    }
                    _ => todo!(),
                }
            }
            Int(integer) => {
                let token = tokenizer.next();
                match token.token_type {
                    Semicolon => return Ok(Expression::Integer(integer)),
                    _ => todo!(),
                }
            }
            Dec(decimal) => {
                let token = tokenizer.next();
                match token.token_type {
                    Semicolon => return Ok(Expression::Decimal(decimal)),
                    _ => todo!(),
                }
            }
            _ => todo!(),
        }
    }

    fn parse_block<R: Read, S: Scope>(&mut self, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Expression> {
        todo!();
    }
}

impl Into<Namespace> for Parser {
    fn into(mut self) -> Namespace {
        'next_unresolved: while !self.unknown_ids.is_empty() {
            let identifier = unsafe {self.unknown_ids.pop_front().unwrap_unchecked()};
            use IdentifierType::*;
            if let Unresolved{scope_used, scope_described} = &mut unsafe {&mut *identifier}.id_type {
                if !scope_described.is_empty() {
                    todo!("Could not handle scope_described {:?}", scope_described);
                }
                if scope_used.resolve_identifier(unsafe {&mut *identifier}) {
                    continue 'next_unresolved;
                }
                let mut scope = scope_used;
                loop {
                    match *scope {
                        ScopeRef::Module(module_ptr) => {
                            let mut module = unsafe {&*module_ptr};
                            loop {
                                if ScopeRef::Module(module_ptr).resolve_identifier(unsafe {&mut *identifier}) {
                                    continue 'next_unresolved;
                                }
                                if module.parent.is_null() {
                                    panic!("Could not resolve identifier `{}`", unsafe {&mut *identifier}.name);
                                }
                                module = unsafe {&*module.parent};
                            }
                        }
                        ScopeRef::Block(block_ptr) => {
                            let block = unsafe {&*block_ptr};
                            if scope.resolve_identifier(unsafe {&mut *identifier}) {
                                continue 'next_unresolved;
                            }
                            if block.parent.is_null() {
                                panic!("Could not resolve identifier `{}`", unsafe {&mut *identifier}.name);
                            }
                            scope = unsafe {&mut *block.parent};
                        }
                    }
                }
            }
            else {
                panic!("Identifier `{}` is not unknown", unsafe {&*identifier}.name);
            }
        }
        return self.ast;
    }
}
