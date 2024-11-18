use std::{io::Read, fmt::Debug};
use std::collections::VecDeque;
use crate::ast::{*, self};
use crate::tokenizer::{Tokenizer, Token, TokenKind};

#[allow(dead_code)]
pub struct ParseError {
    message: RcStr,
    location: Location,
}

impl ParseError {
    fn unexpected<T>(token: &Token, expected: &[&'static str]) -> ParseResult<T> {
        let mut message = format!("Unexpected token `{}` at {}. Expected ", token.token_kind.str(), token.location.str());
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

    fn not_implemented<T>(token: &Token) -> ParseResult<T> {
        return Err(ParseError{
            message: format!("Not implemented: `{}` at {}", token.token_kind.str(), token.location.str()).into(),
            location: token.location,
        });
    }

    fn message<T>(token: &Token, message: &'static str) -> ParseResult<T> {
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
    ast: GlobalNamespace,
    unresolved_identifiers: Queue<*mut ScopedId>,
}

impl Parser {
    pub fn new() -> Parser {
        return Parser {
            ast: GlobalNamespace::new(),
            unresolved_identifiers: Queue::new(),
        };
    }

    pub fn parse<R: Read>(&mut self, mut tokenizer: Tokenizer<R>) -> ParseResult<()> {
        use TokenKind::*;
        loop {
            let token = tokenizer.next();
            match token.token_kind {
                Pub => {
                    let declaration = self.parse_top_declaration_public(&self.ast, &mut tokenizer)?;
                    self.ast.declarations.push(declaration);
                }
                Var => {
                    let declaration = self.parse_top_declaration(false, DeclKeyword::Var, &self.ast, &mut tokenizer)?;
                    self.ast.declarations.push(declaration);
                }
                Let => {
                    let declaration = self.parse_top_declaration(false, DeclKeyword::Let, &self.ast, &mut tokenizer)?;
                    self.ast.declarations.push(declaration);
                }
                Def => {
                    let declaration = self.parse_top_declaration(false, DeclKeyword::Def, &self.ast, &mut tokenizer)?;
                    self.ast.declarations.push(declaration);
                }
                EOF => return Ok(()),
                _ => return ParseError::not_implemented(&token),
            }
        }
    }

    fn parse_top_declaration_public<R: Read, S: Scope>(&mut self, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        use TokenKind::*;
        let token = tokenizer.next();
        match token.token_kind {
            Var => return self.parse_top_declaration(true, DeclKeyword::Var, scope, tokenizer),
            Let => return self.parse_top_declaration(true, DeclKeyword::Let, scope, tokenizer),
            Def => return self.parse_top_declaration(true, DeclKeyword::Def, scope, tokenizer),
            _ => return ParseError::unexpected(&token, &["var","let","def"]),
        };
    }

    fn parse_top_declaration<R: Read, S: Scope>(&mut self, public: bool, decl_keyword: DeclKeyword, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        use TokenKind::*;
        let token = tokenizer.next();
        match token.token_kind {
            Id(identifier) => {
                let mut type_kind = TypeKind::Inferred;
                let mut token = tokenizer.next();
                if let Colon = token.token_kind {
                    type_kind = self.parse_type(scope, tokenizer)?;
                    token = tokenizer.next();
                }
                match token.token_kind {
                    Equal => {
                        let expression = self.parse_expression_semicolon(scope, tokenizer)?;
                        return Ok(Declaration::ValueDecl(Box::new(ValueDecl{
                            public,
                            decl_keyword,
                            identifier,
                            type_kind,
                            decl_kind: DeclKind::AssignDecl(Box::new(expression)),
                        })));
                    }
                    OpenParen => return self.parse_func_decl(public, decl_keyword, identifier, scope, tokenizer),
                    _ => return ParseError::unexpected(&token, &["=","("]),
                };
            }
            OpenBracket => return ParseError::not_implemented(&token),
            _ => return ParseError::unexpected(&token, &["identifier"]),
        };
    }

    fn parse_func_decl<R: Read, S: Scope>(&mut self, public: bool, decl_keyword: DeclKeyword, identifier: RcStr, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        let mut arguments = Vec::new();
        loop {
            use TokenKind::*;
            let token = tokenizer.next();
            match token.token_kind {
                Id(arg_id) => {
                    let parameter = self.parse_argument(arg_id, scope, tokenizer)?;
                    arguments.push(parameter);
                }
                // Def => {
                //     let parameter = self.parse_declaration_keyword(DeclKeyword::Def, scope, tokenizer);
                // }
                CloseParen => {
                    let token = tokenizer.next();
                    match token.token_kind {
                        Arrow => {
                            let return_type = self.parse_type(scope, tokenizer)?;
                            return self.parse_func_body(public, decl_keyword, identifier, arguments.into(), return_type, scope, tokenizer);
                        }
                        _ => return ParseError::unexpected(&token, &["->"]),
                    }
                }
                _ => return ParseError::unexpected(&token, &["identifier"]),
            };
        }
    }

    fn parse_argument<R: Read, S: Scope>(&mut self, identifier: RcStr, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Argument> {
        let token = tokenizer.next();
        match token.token_kind {
            TokenKind::Colon => {
                let mut value_decl = Box::new(ValueDecl{
                    public: true,
                    decl_keyword: DeclKeyword::Let,
                    type_kind: TypeKind::Inferred,
                    identifier,
                    decl_kind: DeclKind::EmptyDecl{},
                });
                value_decl.type_kind = self.parse_type(&*value_decl, tokenizer)?;
                return Ok(Argument{decl: Declaration::ValueDecl(value_decl)});
            }
            _ => return ParseError::unexpected(&token, &[":"]),
        };
    }

    fn parse_type<R: Read, S: Scope>(&mut self, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<TypeKind> {
        let token = tokenizer.peek(0);
        use TokenKind::*;
        use TypeKind::*;
        use ast::Primitive::*;
        let kind = match &token.token_kind {
            Type(identifier) => match identifier.as_ref() {
                "U1"    => Some(Primitive(U1)),
                "U8"    => Some(Primitive(U8)),
                "U16"   => Some(Primitive(U16)),
                "U32"   => Some(Primitive(U32)),
                "U64"   => Some(Primitive(U64)),
                "U128"  => Some(Primitive(U128)),
                "I8"    => Some(Primitive(I8)),
                "I16"   => Some(Primitive(I16)),
                "I32"   => Some(Primitive(I32)),
                "I64"   => Some(Primitive(I64)),
                "I128"  => Some(Primitive(I128)),
                "Bool"  => Some(Primitive(ast::Primitive::Bool)),
                "F16"   => Some(Primitive(F16)),
                "F32"   => Some(Primitive(F32)),
                "F64"   => Some(Primitive(F64)),
                "F128"  => Some(Primitive(F128)),
                _ => Some(TypeKind::Identifier(Box::new(ScopedId{
                    name: identifier.clone(),
                    id_kind: IdKind::Unresolved{
                        scope_used: Scope::into_scopekind(scope),
                        scope_described: Box::new([]),
                    }
                }))),
            }
            Comma =>        None,
            CloseParen =>   None,
            OpenBracket =>  None,
            CloseBracket => None,
            Semicolon =>    None,
            Do =>           None,
            _ => return ParseError::unexpected(token, &["type"]),
        };
        match kind {
            Option::Some(kind) => {
                tokenizer.consume_peeked();
                return Ok(kind);
            }
            Option::None => {
                return Ok(Inferred);
            }
        };
    }

    fn parse_func_body<R: Read, S: Scope>(&mut self, public: bool, decl_keyword: DeclKeyword, identifier: RcStr, arguments: Box<[Argument]>, return_type: TypeKind, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Declaration> {
        let mut value_decl = Box::new(ValueDecl{
            public,
            decl_keyword,
            identifier,
            type_kind: TypeKind::FuncType(Box::new(FuncType{
                arguments,
                return_type: Box::new(return_type),
            })),
            decl_kind: DeclKind::EmptyDecl{},
        });
        use TokenKind::*;
        let token = tokenizer.next();
        value_decl.decl_kind = DeclKind::FuncDecl(Box::new(match token.token_kind {
            OpenBracket => self.parse_block(&*value_decl, tokenizer)?,
            Do => self.parse_expression_semicolon(&*value_decl, tokenizer)?,
            _ => return ParseError::unexpected(&token, &["{","do"]),
        }));
        return Ok(Declaration::ValueDecl(value_decl));
    }

    fn parse_expression_semicolon<R: Read, S: Scope>(&mut self, scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Expression> {
        use TokenKind::*;
        let token = tokenizer.next();
        match token.token_kind {
            Id(identifier) => {
                let token = tokenizer.next();
                match token.token_kind {
                    Semicolon => {
                        let mut u_id = Box::new(ScopedId{
                            id_kind: IdKind::Unresolved{
                                scope_used: ScopeKind::from_ptr(scope),
                                scope_described: Box::new([])
                            },
                            name: *Box::new(identifier.into()),
                        });
                        self.unresolved_identifiers.push_back(u_id.as_mut());
                        return Ok(Expression::Identifier(u_id));
                    }
                    _ => ParseError::not_implemented(&token),
                }
            }
            Int(integer) => {
                let token = tokenizer.next();
                match token.token_kind {
                    Semicolon => return Ok(Expression::Integer(integer)),
                    _ => ParseError::not_implemented(&token)
                }
            }
            Dec(decimal) => {
                let token = tokenizer.next();
                match token.token_kind {
                    Semicolon => return Ok(Expression::Decimal(decimal)),
                    _ => ParseError::not_implemented(&token)
                }
            }
            Bool(boolean) => {
                let token = tokenizer.next();
                match token.token_kind {
                    Semicolon => return Ok(Expression::Bool(boolean)),
                    _ => ParseError::not_implemented(&token)
                }
            }
            _ => ParseError::not_implemented(&token),
        }
    }

    fn parse_block<R: Read, S: Scope>(&mut self, _scope: *const S, tokenizer: &mut Tokenizer<R>) -> ParseResult<Expression> {
        return ParseError::not_implemented(&tokenizer.next());
    }
}

impl Parser {
    fn resolve_identifiers(&mut self) {
        'next_unresolved: while !self.unresolved_identifiers.is_empty() {
            let identifier = unsafe {self.unresolved_identifiers.pop_front().unwrap_unchecked()};

            if let IdKind::Unresolved{scope_used, scope_described} = &mut unsafe {&mut *identifier}.id_kind {
                if !scope_described.is_empty() {
                    todo!("Could not handle scope_described {:?}", scope_described);
                }
                if scope_used.resolve_identifier(unsafe {&mut *identifier}) {
                    continue 'next_unresolved;
                }

                let scope = scope_used;
                loop {
                    match *scope {
                        ScopeKind::GlobalNamespace(namespace_ptr) => {
                            let current_namespace = unsafe {&* namespace_ptr};
                            if current_namespace.resolve_identifier(unsafe {&mut *identifier}) {
                                continue 'next_unresolved;
                            }
                            else {
                                panic!("Could not resolve identifier `{}`", unsafe {&mut *identifier}.name);
                            }
                        }
                        ScopeKind::SubNamespace(namespace_ptr) => {
                            let mut current_namespace = unsafe {&* namespace_ptr};
                            'next_namespace: loop {
                                if current_namespace.resolve_identifier(unsafe {&mut *identifier}) {
                                    continue 'next_unresolved;
                                }
                                match current_namespace.parent {
                                    NamespaceKind::GlobalNamespace(_) => {
                                        panic!("Could not resolve identifier `{}`", unsafe {&mut *identifier}.name);
                                    }
                                    NamespaceKind::SubNamespace(namespace_ptr) => {
                                        current_namespace = unsafe {&*namespace_ptr};
                                        continue 'next_namespace;
                                    }
                                }
                            }
                        }
                        ScopeKind::Function(function_ptr) => {
                            let function = unsafe {&* function_ptr};
                            if function.resolve_identifier(unsafe {&mut *identifier}) {
                                continue 'next_unresolved;
                            }
                            todo!();
                        }
                        // ScopeKind::Block(block_ptr) => {
                        //     let block = unsafe {&*block_ptr};
                        //     if scope.resolve_identifier(unsafe {&mut *identifier}) {
                        //         continue 'next_unresolved;
                        //     }
                        //     if block.parent.is_null() {
                        //         panic!("Could not resolve identifier `{}`", unsafe {&mut *identifier}.name);
                        //     }
                        //     scope = unsafe {&mut *block.parent};
                        // }
                    }
                }
            }
            else {
                unreachable!("Identifier `{}` was stored in unresolved_identifiers but was not unresolved.", unsafe {&*identifier}.name);
            }
        }
    }
}

impl Into<GlobalNamespace> for Parser {
    fn into(mut self) -> GlobalNamespace {
        self.resolve_identifiers();
        return self.ast;
    }
}
