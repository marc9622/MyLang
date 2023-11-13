use std::rc::Rc;
use std::{ptr, collections::HashMap};
use std::fmt::Debug;

pub type StrRef = Rc<str>;
pub type IdStr = StrRef;

#[derive(Debug)]
pub struct Program {
    pub root_module: Namespace,
    pub modules: HashMap<IdStr, Namespace>,
}

#[derive(Debug)]
pub struct Namespace {
    pub name: IdStr,
    pub full_name: IdStr,
    pub declarations: Vec<Declaration>,
    pub subnamespace: Vec<Namespace>,
    /// Nullable
    pub parent: *const Namespace,
}

impl Namespace {
    pub fn new_root(name: IdStr) -> Namespace {
        let full_name = name.clone();
        return Namespace {
            name,
            full_name,
            declarations: Vec::new(),
            subnamespace: Vec::new(),
            parent: ptr::null(),
        };
    }
    pub fn new(name: IdStr, parent: *const Namespace) -> Namespace {
        if parent.is_null() {
            panic!();
        }
        let mut full_name = unsafe {&*parent}.get_module_path().join(".");
        full_name.push('.');
        full_name.push_str(name.as_ref());
        return Namespace {
            name,
            full_name: full_name.into(),
            declarations: Vec::new(),
            subnamespace: Vec::new(),
            parent,
        };
    }
    fn get_module_path(&self) -> Box<[IdStr]> {
        let mut path = Vec::new();
        let module = self;
        loop {
            if module.parent.is_null() {
                break;
            }
            path.push(unsafe {*module.parent}.name.clone());
        }
        path.reverse();
        return path.into();
    }
}

#[derive(Debug)]
pub struct Declaration {
    pub identifier: IdStr,
    pub decl_keyword: DeclKeyword,
    pub decl_type: DeclType,
    pub expression: Expression,
}

#[derive(Debug)]
pub enum DeclType {
    ValueDecl{value_type: Option<Type>},
    FuncDecl{func_type: FuncType},
}

impl DeclType {
    pub fn inferred_value() -> DeclType {
        return DeclType::ValueDecl{value_type: None};
    }
}

#[derive(Debug)]
pub enum DeclKeyword {
    Var, Def,
}

#[derive(Debug)]
pub enum Expression {
    Identifier(Box<ScopedId>),
    Type(Type),
    Integer(StrRef),
    Decimal(StrRef),
    Block(Block),
}

#[derive(Debug)]
pub enum Type {
    Inferred,
    Identifier(IdStr),
    Function {
        return_type: Box<Type>,
        parameters: Box<[Declaration]>,
    },
}

#[derive(Debug)]
pub struct FuncType {
    pub parameters: Box<[Declaration]>,
    pub return_type: Box<Type>,
}

#[derive(Debug)]
pub struct Block {
    pub name: Option<IdStr>,
    pub statements: Vec<Statement>,
    pub parent: *mut ScopeRef,
}

#[derive(Debug)]
pub enum Statement {
    Declaration(Declaration),
    Block(Block),
}

#[derive(Debug)]
pub struct ScopedId {
    pub id_type: IdentifierType,
    pub name: IdStr,
}

#[derive(Debug)]
pub enum IdentifierType {
    Resolved {
        declaration: *const Declaration,
        scope: ScopeRef,
    },
    Unresolved {
        scope_used: ScopeRef,
        scope_described: Box<[IdStr]>,
    },
}

pub trait Scope {
    fn get_module_full_name(&self) -> IdStr;
    #[must_use]
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool;
    #[must_use]
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool;
    fn into_scoperef(scope: *const Self) -> ScopeRef;
}

impl Scope for Namespace {
    fn get_module_full_name(&self) -> IdStr {
        return self.full_name.clone();
    }
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
        for decl in &self.declarations {
            if decl.identifier == declaration.identifier {
                return false;
            }
        }
        return true;
    }
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
        for decl in &self.declarations {
            if decl.identifier == identifier.name {
                identifier.id_type = IdentifierType::Resolved{
                    declaration: decl,
                    scope: ScopeRef::Module(self),
                };
                return true;
            }
        }
        return false;
    }
    fn into_scoperef(module: *const Namespace) -> ScopeRef {
        return ScopeRef::Module(module);
    }
}

impl Scope for Block {
    fn get_module_full_name(&self) -> IdStr {
        let mut scope = self;
        loop { match unsafe {&*scope.parent} {
            ScopeRef::Module(module_ptr) => return unsafe {&**module_ptr}.get_module_full_name(),
            ScopeRef::Block(block_ptr) => {
                scope = unsafe {&**block_ptr};
            }
        }}
    }
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
        for stmt in &self.statements {
            match stmt {
                Statement::Declaration(decl) => {
                    if ptr::eq(decl, declaration) {
                        return true;
                    }
                    if decl.identifier == declaration.identifier {
                        return false;
                    }
                }
                Statement::Block(_) => {
                    continue;
                }
            }
        }
        return true;
    }
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
        for stmt in &self.statements {
            match stmt {
                Statement::Declaration(decl) => {
                    if decl.identifier == identifier.name {
                        identifier.id_type = IdentifierType::Resolved{
                            declaration: decl,
                            scope: ScopeRef::Block(self),
                        };
                        return true;
                    }
                }
                Statement::Block(..) => {
                    continue;
                }
            }
        }
        return false;
    }
    fn into_scoperef(block: *const Block) -> ScopeRef {
        return ScopeRef::Block(block);
    }
}

#[derive(Copy,Clone,Debug)]
pub enum ScopeRef {
    Module(*const Namespace),
    Block(*const Block),
}

impl Scope for ScopeRef {
    fn get_module_full_name(&self) -> IdStr {
        match *self {
            ScopeRef::Module(module_ptr) => unsafe {&*module_ptr}.get_module_full_name(),
            ScopeRef::Block(block_ptr) => unsafe {&*block_ptr}.get_module_full_name(),
        }
    }
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
        match *self {
            ScopeRef::Module(module_ptr) => unsafe {&*module_ptr}.is_unique_identifier(declaration),
            ScopeRef::Block(block_ptr) => unsafe {&*block_ptr}.is_unique_identifier(declaration),
        }
    }
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
        match *self {
            ScopeRef::Module(module_ptr) => unsafe {&*module_ptr}.resolve_identifier(identifier),
            ScopeRef::Block(block_ptr) => unsafe {&*block_ptr}.resolve_identifier(identifier),
        }
    }
    fn into_scoperef(scoperef: *const ScopeRef) -> ScopeRef {
        return unsafe {&*scoperef}.clone();
    }
}

impl ScopeRef {
    pub fn from_ptr<S: Scope>(pointer: *const S) -> ScopeRef {
        return S::into_scoperef(pointer);
    }
}
