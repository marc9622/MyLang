use std::rc::Rc;
use std::fmt::Debug;
use std::ptr::NonNull;

pub type RcStr = Rc<str>;
pub type IdStr = RcStr;

#[derive(Clone,Copy,Debug)]
pub struct Location {
    pub line: u16,
    pub char: u16,
}

impl Location {
    pub fn str(&self) -> RcStr {
        return format!("line: {}, char: {}", self.line, self.char).into();
    }

    pub fn inc_line(&mut self) {
        self.line += 1;
        self.char = 0;
    }

    pub fn inc_char(&mut self, count: u16) {
        self.char += count;
    }
}

#[derive(Debug)]
pub struct Program {
    pub name: RcStr,
    pub ast: GlobalNamespace,
}

impl Program {
    pub fn get_all_namespace_full_names(&self) -> Box<[RcStr]> {
        let mut vector = Vec::new();
        vector.push("".into());
        for subnamespace in &self.ast.subnamespaces {
            subnamespace.add_all_subnamespace_full_names(&mut vector);
        }
        return vector.into();
    }
    pub fn get_all_namespaced_declarations(&mut self) -> Box<[(RcStr, &Declaration)]> {
        let mut vector = Vec::new();

        for declaration in &self.ast.declarations {
            vector.push(("".into(), declaration));
        }

        for subnamespace in &self.ast.subnamespaces {
            subnamespace.add_all_namespaced_declarations(&mut vector);
        }

        return vector.into();
    }
}

#[derive(Debug)]
pub enum NamespaceKind {
    GlobalNamespace(*const GlobalNamespace),
    SubNamespace(*const SubNamespace),
}

#[derive(Debug)]
pub struct GlobalNamespace {
    pub declarations: Vec<Declaration>,
    pub subnamespaces: Vec<SubNamespace>,
}

impl GlobalNamespace {
    pub fn new() -> GlobalNamespace {
        return GlobalNamespace {
            declarations: Vec::new(),
            subnamespaces: Vec::new(),
        };
    }
}

#[derive(Debug)]
pub struct SubNamespace {
    pub name: RcStr,
    pub full_name: RcStr,
    pub declarations: Vec<Declaration>,
    pub subnamespaces: Vec<SubNamespace>,
    pub parent: NamespaceKind,
}

impl SubNamespace {
    pub fn new(name: RcStr, parent: NonNull<SubNamespace>) -> SubNamespace {
        let mut full_name = unsafe {parent.as_ref()}.get_path().join(".");
        full_name.push('.');
        full_name.push_str(name.as_ref());
        return SubNamespace {
            name,
            full_name: full_name.into(),
            declarations: Vec::new(),
            subnamespaces: Vec::new(),
            parent: NamespaceKind::SubNamespace(parent.as_ptr()),
        };
    }
    fn get_path(&self) -> Box<[RcStr]> {
        let mut path = Vec::new();
        let mut namespace = self;
        loop {
            path.push(namespace.name.clone());
            match namespace.parent {
                NamespaceKind::GlobalNamespace(_) => break,
                NamespaceKind::SubNamespace(parent) => namespace = unsafe {&*parent},
            };
        };
        path.reverse();
        return path.into();
    }
    fn add_all_subnamespace_full_names(&self, vector: &mut Vec<RcStr>) {
        vector.push(self.name.clone());
        for subnamespace in &self.subnamespaces {
            subnamespace.add_all_subnamespace_full_names(vector);
        }
    }
    fn add_all_namespaced_declarations<'a>(&'a self, vector: &mut Vec<(RcStr, &'a Declaration)>) {
        for declaration in &self.declarations {
            vector.push((self.full_name.clone(), declaration));
        }
        for subnamespace in &self.subnamespaces {
            subnamespace.add_all_namespaced_declarations(vector);
        }
    }
}

#[derive(Debug)]
pub enum Declaration {
    ValueDecl(Box<ValueDecl>),
}

impl Declaration {
    fn is_same_type(&self, other: &Declaration) -> bool {
        return match self {
            Declaration::ValueDecl(..) => false,
        }
    }
}

#[derive(Debug)]
pub struct ValueDecl {
    pub public: bool,
    pub decl_keyword: DeclKeyword,
    pub identifier: RcStr,
    pub type_kind: TypeKind,
    pub decl_kind: DeclKind,
}

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum DeclKeyword {
    Var, Let, Def
}

#[derive(Debug)]
pub enum TypeKind {
    Inferred,
    Identifier(Box<ScopedId>),
    FuncType(Box<FuncType>),
    Primitive(Primitive),
}

impl TypeKind {
    pub fn is_same_type(&self, other: &TypeKind) -> bool {
        use TypeKind::*;
        if let Inferred = other {
            panic!("Cannot compare inferred type");
        }
        match self {
            Inferred => panic!("Cannot compare inferred type"),
            Identifier(scoped_id) => {
                match scoped_id.id_kind {
                    IdKind::Resolved{declaration, ..} => {
                        return match other {
                            Identifier(other_scoped_id) => {
                                match other_scoped_id.id_kind {
                                    IdKind::Resolved{declaration: other_declaration, ..} => {
                                        return unsafe {&*declaration}.is_same_type(unsafe {&*other_declaration});
                                    }
                                    IdKind::Unresolved{..} => panic!("Cannot compare unresolved type"),
                                }
                            }
                            _ => false,
                        }
                    }
                    IdKind::Unresolved{..} => panic!("Cannot compare unresolved type"),
                }
            }
            FuncType(func_type) => {
                return match other {
                    FuncType(other_func_type) => func_type.is_same_type(other_func_type),
                    _ => false,
                }
            }
            Primitive(primitive) => {
                return match other {
                    Primitive(other_primitive) => primitive == other_primitive,
                    _ => false,
                }
            }
        };
    }
}

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Primitive {
    U8, U16, U32, U64, U128,
    I8, I16, I32, I64, I128,
        F16, F32, F64, F128,
    U1, Bool,
}

#[derive(Debug)]
pub struct FuncType {
    pub arguments: Box<[Argument]>,
    pub return_type: Box<TypeKind>,
}

impl FuncType {
    fn is_same_type(&self, other: &FuncType) -> bool {
        if !self.return_type.is_same_type(&other.return_type) {
            return false;
        }
        if self.arguments.len() != other.arguments.len() {
            return false;
        }
        for index in 0..self.arguments.len() {
            if !self.arguments.get(index).unwrap().decl.is_same_type(&other.arguments.get(index).unwrap().decl) {
                return false;
            }
        }
        return true;
    }
}

#[derive(Debug)]
pub struct Argument {
    pub decl: Declaration,
}

#[derive(Debug)]
pub enum DeclKind {
    EmptyDecl,
    AssignDecl(Box<Expression>),
    FuncDecl(Box<Expression>),
}

#[derive(Debug)]
pub enum Expression {
    Identifier(Box<ScopedId>),
    Integer(RcStr),
    Decimal(RcStr),
    Bool(bool),
    //Block(Block),
}

#[derive(Debug)]
pub struct ScopedId {
    pub name: RcStr,
    pub id_kind: IdKind,
}

#[derive(Debug)]
pub enum IdKind {
    Resolved {
        declaration: *const Declaration,
        scope: ScopeKind,
    },
    Unresolved {
        scope_used: ScopeKind,
        scope_described: Box<[IdStr]>,
    },
}

#[derive(Copy,Clone,Debug)]
pub enum ScopeKind {
    /// Non-nullable
    GlobalNamespace(*const GlobalNamespace),
    /// Non-nullable
    SubNamespace(*const SubNamespace),
    /// Non-nullable
    Function(*const ValueDecl),
    //Block(*const Block),
}

pub trait Scope {
    fn get_full_name(&self) -> RcStr;
    #[must_use]
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool;
    #[must_use]
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool;
    fn into_scopekind(scope: *const Self) -> ScopeKind;
}

impl Scope for GlobalNamespace {
    fn get_full_name(&self) -> RcStr {
        return "".into();
    }
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
        let Declaration::ValueDecl(value_declaration) = declaration; {
            for decl in &self.declarations {
                let Declaration::ValueDecl(value_decl) = decl; {
                    if value_decl.identifier == value_declaration.identifier {
                        return false;
                    }
                }
            }
        };
        return true;
    }
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
        for decl in &self.declarations {
            let Declaration::ValueDecl(value_decl) = decl; {
                if value_decl.identifier == identifier.name {
                    identifier.id_kind = IdKind::Resolved {
                        declaration: decl,
                        scope: ScopeKind::GlobalNamespace(self),
                    };
                    return true;
                }
            }
        }
        return false;
    }
    fn into_scopekind(namespace_ptr: *const GlobalNamespace) -> ScopeKind {
        return ScopeKind::GlobalNamespace(namespace_ptr);
    }
}

impl Scope for SubNamespace {
    fn get_full_name(&self) -> RcStr {
        return self.full_name.clone();
    }
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
        let Declaration::ValueDecl(value_declaration) = declaration; {
            for decl in &self.declarations {
                let Declaration::ValueDecl(value_decl) = decl; {
                    if value_decl.identifier == value_declaration.identifier {
                        return false;
                    }
                }
            }
        };
        for subnamespace in &self.subnamespaces {
            if !subnamespace.is_unique_identifier(declaration) {
                return false;
            }
        }
        return true;
    }
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
        for decl in &self.declarations {
            let Declaration::ValueDecl(value_decl) = decl; {
                if value_decl.identifier == identifier.name {
                    identifier.id_kind = IdKind::Resolved {
                        declaration: decl,
                        scope: ScopeKind::SubNamespace(self),
                    };
                    return true;
                }
            }
        }
        return false;
    }
    fn into_scopekind(namespace_ptr: *const SubNamespace) -> ScopeKind {
        return ScopeKind::SubNamespace(namespace_ptr);
    }
}

impl Scope for ValueDecl {
    fn get_full_name(&self) -> RcStr {
        todo!("Figure out what to do here");
    }
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
        let Declaration::ValueDecl(value_declaration) = declaration; {
            // Arguments
            if let TypeKind::FuncType(func_type) = &self.type_kind {
                for arg in func_type.arguments.into_iter() {
                    match &arg.decl {
                        Declaration::ValueDecl(value_decl) => {
                            if value_decl.identifier == value_declaration.identifier {
                                return false;
                            }
                        }
                    }
                }
            }
            else {
                panic!("Only function types shouls can be scopes. If reached, this is a bug.");
            }
            // Earlier declarations
            match &self.decl_kind {
                DeclKind::EmptyDecl => panic!("Empty declarations should never contain declarations. If reached, this is a bug."),
                DeclKind::FuncDecl(expression) => {
                    match expression {
                        //Expression::Block...
                        _ => return true,
                    }
                }
                DeclKind::AssignDecl(lambda) => {
                    match lambda {
                        //Expression::Lambda...
                        _ => todo!(),
                    }
                }
            }
        };
    }
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
        if let TypeKind::FuncType(func_type) = &self.type_kind {
            for arg in func_type.arguments.into_iter() {
                match &arg.decl {
                    Declaration::ValueDecl(value_decl) => {
                        if value_decl.identifier == identifier.name {
                            identifier.id_kind = IdKind::Resolved {
                                declaration: &arg.decl,
                                scope: ScopeKind::Function(self),
                            };
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        else {
            panic!("Only function types shouls can be scopes. If reached, this is a bug.");
        }
    }
    fn into_scopekind(function_ptr: *const ValueDecl) -> ScopeKind {
        match unsafe {&*function_ptr}.type_kind {
            TypeKind::FuncType(..) => {},
            TypeKind::Inferred => panic!("Cannot create scope for inferred function"),
            _ => panic!("Cannot create scope for non-function"),
        }
        return ScopeKind::Function(function_ptr);
    }
}

impl Scope for ScopeKind {
    fn get_full_name(&self) -> RcStr {
        match *self {
            ScopeKind::GlobalNamespace(namespace_ptr) => unsafe {&*namespace_ptr}.get_full_name(),
            ScopeKind::SubNamespace(namespace_ptr) => unsafe {&*namespace_ptr}.get_full_name(),
            ScopeKind::Function(function_ptr) => unsafe {&*function_ptr}.get_full_name(),
            //ScopeKind::Block(block_ptr) => unsafe {&*block_ptr}.get_namespace_full_name(),
        }
    }
    fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
        match *self {
            ScopeKind::GlobalNamespace(namespace_ptr) => unsafe {&*namespace_ptr}.is_unique_identifier(declaration),
            ScopeKind::SubNamespace(namespace_ptr) => unsafe {&*namespace_ptr}.is_unique_identifier(declaration),
            ScopeKind::Function(function_ptr) => unsafe {&*function_ptr}.is_unique_identifier(declaration),
            //ScopeKind::Block(block_ptr) => unsafe {&*block_ptr}.is_unique_identifier(declaration),
        }
    }
    fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
        match *self {
            ScopeKind::GlobalNamespace(namespace_ptr) => unsafe {&*namespace_ptr}.resolve_identifier(identifier),
            ScopeKind::SubNamespace(namespace_ptr) => unsafe {&*namespace_ptr}.resolve_identifier(identifier),
            ScopeKind::Function(function_ptr) => unsafe {&*function_ptr}.resolve_identifier(identifier),
            //ScopeKind::Block(block_ptr) => unsafe {&*block_ptr}.resolve_identifier(identifier),
        }
    }
    fn into_scopekind(scoperef: *const ScopeKind) -> ScopeKind {
        return unsafe {&*scoperef}.clone();
    }
}

impl ScopeKind {
    pub fn from_ptr<S: Scope>(pointer: *const S) -> ScopeKind {
        return S::into_scopekind(pointer);
    }
}

// TODO

// #[derive(Debug)]
// pub struct Block {
//     pub name: Option<IdStr>,
//     pub statements: Vec<Statement>,
//     pub parent: *mut ScopeRef,
// }
//
// #[derive(Debug)]
// pub enum Statement {
//     Declaration(Declaration),
//     Block(Block),
// }
//
// impl Scope for Block {
//     fn get_module_full_name(&self) -> IdStr {
//         let mut scope = self;
//         loop { match unsafe {&*scope.parent} {
//             ScopeRef::Module(module_ptr) => return unsafe {&**module_ptr}.get_module_full_name(),
//             ScopeRef::Block(block_ptr) => {
//                 scope = unsafe {&**block_ptr};
//             }
//         }}
//     }
//     fn is_unique_identifier(&self, declaration: &Declaration) -> bool {
//         for stmt in &self.statements {
//             match stmt {
//                 Statement::Declaration(decl) => {
//                     if ptr::eq(decl, declaration) {
//                         return true;
//                     }
//                     if decl.identifier == declaration.identifier {
//                         return false;
//                     }
//                 }
//                 Statement::Block(_) => {
//                     continue;
//                 }
//             }
//         }
//         return true;
//     }
//     fn resolve_identifier(&self, identifier: &mut ScopedId) -> bool {
//         for stmt in &self.statements {
//             match stmt {
//                 Statement::Declaration(decl) => {
//                     if decl.identifier == identifier.name {
//                         identifier.id_type = IdentifierType::Resolved{
//                             declaration: decl,
//                             scope: ScopeRef::Block(self),
//                         };
//                         return true;
//                     }
//                 }
//                 Statement::Block(..) => {
//                     continue;
//                 }
//             }
//         }
//         return false;
//     }
//     fn into_scoperef(block: *const Block) -> ScopeRef {
//         return ScopeRef::Block(block);
//     }
// }
