use std::collections::{HashMap, VecDeque};

use inkwell::module::Linkage;
use inkwell::types::{StringRadix, BasicType, BasicMetadataTypeEnum, FloatType, BasicTypeEnum};
use inkwell::{values::FunctionValue, types::IntType};
use inkwell::{context::Context, builder::Builder, module::Module};

use crate::ast::{self, Scope};

type Queue<T> = VecDeque<T>;

pub fn generate(mut program: ast::Program) -> ast::RcStr {
    let context = Context::create();
    let builder = context.create_builder();
    let mut codegen = CodeGen::new(&context, builder);

    for full_name in program.get_all_namespace_full_names().into_iter() {
        codegen.modules.insert(full_name.clone(), codegen.context.create_module(&full_name));
    }

    let mut unprocessed_declarations = Queue::new();
    let all_namespaced_declarations = program.get_all_namespaced_declarations();
    for namespaced_declaration in all_namespaced_declarations.into_iter() {
        unprocessed_declarations.push_back(namespaced_declaration);
    }

    for (full_name, declaration) in unprocessed_declarations.into_iter() {
        let module = codegen.modules.get(full_name).unwrap();

        let ast::Declaration::ValueDecl(value_decl) = declaration; {
            codegen.add_global(module, value_decl);
        }
    }

    return codegen.modules.get("").unwrap().to_string().into();
}

struct CodeGen<'c> {
    context: &'c Context,
    builder: Builder<'c>,
    modules: HashMap<ast::RcStr, Module<'c>>,
}

impl<'c> CodeGen<'c> {
    fn new(context: &'c Context, builder: Builder<'c>) -> Self {
        return CodeGen{context: &context, builder, modules: HashMap::new()};
    }

    fn add_global(&self, module: &Module<'c>, value_decl: &ast::ValueDecl) {
        match &value_decl.decl_kind {
            ast::DeclKind::EmptyDecl => {
                todo!();
            }
            ast::DeclKind::AssignDecl(expression) => {
                self.add_global_assign(module, expression, value_decl);
            }
            ast::DeclKind::FuncDecl(expression) => {
                self.add_global_function(module, expression, value_decl);
            }
        }
    }

    fn add_global_assign(&self, module: &Module<'c>, expression: &ast::Expression, value_decl: &ast::ValueDecl) {
        match expression {
            ast::Expression::Identifier(ref other_identifier) => {
                match other_identifier.id_kind {
                    ast::IdKind::Resolved{scope: other_scope, ..} => {
                        'is_integer: {
                            let int_type = match try_get_type_int(self.context, &value_decl.type_kind) {
                                Some(int_type) => int_type,
                                None => break 'is_integer,
                            };
                            self.add_global_assign_identifier_primitive(module, value_decl, int_type, other_identifier, other_scope);
                            return;
                        }
                        'is_float: {
                            let float_type = match try_get_type_float(self.context, &value_decl.type_kind) {
                                Some(float_type) => float_type,
                                None => break 'is_float,
                            };
                            self.add_global_assign_identifier_primitive(module, value_decl, float_type, other_identifier, other_scope);
                            return;
                        }
                        panic!("Identifier of this type is not implemented");
                    }
                    ast::IdKind::Unresolved{..} => panic!("Unknown identifier"),
                };
            }
            ast::Expression::Integer(literal) => {
                if is_type_int(&value_decl.type_kind) {
                    self.add_global_assign_int(module, value_decl, literal);
                }
                else if is_type_float(&value_decl.type_kind) {
                    self.add_global_assign_float(module, value_decl, literal);
                }
                else {
                    todo!("Integer literal of this type is not implemented");
                }
            }
            ast::Expression::Decimal(literal) => {
                if is_type_float(&value_decl.type_kind) {
                    self.add_global_assign_float(module, value_decl, literal);
                }
                else {
                    todo!("Decimal literal of this type is not implemented");
                }
            }
            ast::Expression::Bool(literal) => {
                if is_type_int(&value_decl.type_kind) {
                    self.add_global_assign_int(module, value_decl, if *literal { "1" } else { "0" });
                }
                else {
                    todo!("Boolean literal of this type is not implemented");
                }
            }
        }
    }

    fn add_global_assign_identifier_primitive<T: BasicType<'c> + Copy>(&self, module: &Module<'c>, value_decl: &ast::ValueDecl, value_type: T, other_identifier: &ast::ScopedId, other_scope: ast::ScopeKind) {
        let global = module.add_global(value_type, None, &value_decl.identifier);
        global.set_linkage(get_linkage(value_decl.public));
        if value_decl.decl_keyword != ast::DeclKeyword::Var {
            global.set_constant(true);
        }

        let other_module = self.modules.get(&other_scope.get_full_name()).unwrap();
        let other_global = other_module.get_global(&other_identifier.name).expect("Likely failed because identifier had not yet been declared");

        global.set_initializer(&other_global.get_initializer().unwrap());

        // Maybe make having non literal values for compile time variables illegal at this point,
        // and instead interpret and evaluate any other expressions during an earlier compilation stage.
        match &value_decl.decl_keyword {
            ast::DeclKeyword::Def => {
            }
            ast::DeclKeyword::Var => {
                // let pointer = other_global.as_pointer_value();
                // println!("null: {}, undef: {}, const: {}, ", pointer.is_null(), pointer.is_undef(), pointer.is_const());
                // println!("type: {}, name: {}", pointer.get_type(), pointer.get_name().to_str().unwrap());
                // let pointee = &self.builder.build_load(value_type, pointer, "load2").unwrap();
                // global.set_initializer(pointee);
            }
            ast::DeclKeyword::Let => {
                // let pointer = other_global.as_pointer_value();
                // println!("null: {}, undef: {}, const: {}, ", pointer.is_null(), pointer.is_undef(), pointer.is_const());
                // println!("type: {}, name: {}", pointer.get_type(), pointer.get_name().to_str().unwrap());
                // let pointee = &self.builder.build_load(value_type, pointer, "load2").unwrap();
                // global.set_initializer(pointee);
            }
        }
    }

    fn add_global_assign_int(&self, module: &Module<'c>, value_decl: &ast::ValueDecl, literal: &str) {
        let int_type = get_type_int(self.context, &value_decl.type_kind);

        let global = module.add_global(int_type, None, &value_decl.identifier);
        global.set_linkage(get_linkage(value_decl.public));
        if value_decl.decl_keyword != ast::DeclKeyword::Var {
            global.set_constant(true);
        }

        let value = int_type.const_int_from_string(literal, StringRadix::Decimal).expect("Failed to parse integer literal");
        global.set_initializer(&value);
    }

    fn add_global_assign_float(&self, module: &Module<'c>, value_decl: &ast::ValueDecl, literal: &str) {
        let float_type = get_type_float(self.context, &value_decl.type_kind);

        let global = module.add_global(float_type, None, &value_decl.identifier);
        global.set_linkage(get_linkage(value_decl.public));
        if value_decl.decl_keyword != ast::DeclKeyword::Var {
            global.set_constant(true);
        }

        let value = float_type.const_float_from_string(literal);
        global.set_initializer(&value);
    }

    fn add_global_function(&self, module: &Module<'c>, expression: &ast::Expression, value_decl: &ast::ValueDecl) {
        let (arguments, return_type) = match &value_decl.type_kind {
            ast::TypeKind::FuncType(func_type) => (func_type.arguments.as_ref(), func_type.return_type.as_ref()),
            _ => panic!("Function did not have a function type"),
        };

        if is_type_int(return_type) {
            self.add_global_int_function(module, &value_decl, arguments, return_type, expression);
        }
        else if is_type_float(return_type) {
            self.add_global_float_function(module, &value_decl, arguments, return_type, expression);
        }
        else {
            todo!("Function declaration of this type is not implemented");
        }
    }

    fn add_global_int_function(&self, module: &Module<'c>, value_decl: &ast::ValueDecl, arguments: &[ast::Argument], return_type: &ast::TypeKind, expression: &ast::Expression) {
        let arg_types = get_argument_types(self.context, arguments);
        let int_type = get_type_int(self.context, return_type);
        let function_type = int_type.fn_type(&arg_types, false);
        let function = module.add_function(&value_decl.identifier, function_type, Some(get_linkage(value_decl.public)));

        match expression {
            ast::Expression::Identifier(identifier) => {
                for index in 0..arguments.len() {
                    let argument_decl = &arguments.get(index).unwrap().decl;
                    if &*identifier.name == match argument_decl {
                        ast::Declaration::ValueDecl(value_decl) => &*value_decl.identifier,
                    } {
                        let entry_block = self.context.append_basic_block(function, "entry");
                        self.builder.position_at_end(entry_block);

                        let ast::Declaration::ValueDecl(value_decl) = argument_decl; {
                            if !return_type.is_same_type(&value_decl.type_kind) {
                                panic!("Cannot return a value of this type from an integer function");
                            }
                            let arg = function.get_nth_param(index as u32).unwrap().into_int_value();
                            self.builder.build_return(Some(&arg)).unwrap();
                        }
                        return;
                    }
                }
                panic!("Identifier not found in function arguments");
            }
            ast::Expression::Integer(literal) => {
                let entry_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry_block);

                let value = int_type.const_int_from_string(literal, StringRadix::Decimal).expect("Failed to parse integer literal");
                self.builder.build_return(Some(&value)).unwrap();
            }
            ast::Expression::Decimal(_) => panic!("Cannot return a decimal literal from an integer function"),
            ast::Expression::Bool(literal) => {
                let entry_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry_block);

                let value = int_type.const_int_from_string(if *literal { "1" } else { "0" }, StringRadix::Decimal).expect("Failed to parse integer literal");
                self.builder.build_return(Some(&value)).unwrap();
            }
        }
    }

    fn add_global_float_function(&self, module: &Module<'c>, value_decl: &ast::ValueDecl, arguments: &[ast::Argument], return_type: &ast::TypeKind, expression: &ast::Expression) {
        let arg_types = get_argument_types(self.context, arguments);
        let float_type = get_type_float(self.context, return_type);
        let function_type = float_type.fn_type(&arg_types, false);
        let function = module.add_function(&value_decl.identifier, function_type, Some(get_linkage(value_decl.public)));

        match expression {
            ast::Expression::Identifier(identifier) => {
                for index in 0..arguments.len() {
                    let argument_decl = &arguments.get(index).unwrap().decl;
                    if &*identifier.name == match argument_decl {
                        ast::Declaration::ValueDecl(value_decl) => &*value_decl.identifier,
                    } {
                        let entry_block = self.context.append_basic_block(function, "entry");
                        self.builder.position_at_end(entry_block);

                        let ast::Declaration::ValueDecl(value_decl) = argument_decl; {
                            if is_type_int(&value_decl.type_kind) {
                                panic!("Cannot return an integer value from a floating point function");
                            }
                            else if is_type_float(&value_decl.type_kind) {
                                let arg = function.get_nth_param(index as u32).unwrap().into_float_value();
                                self.builder.build_return(Some(&arg)).unwrap();
                            }
                            else {
                                panic!("Cannot return a value of this type from a floating point function");
                            }
                        }
                        return;
                    }
                }
                panic!("Identifier not found in function arguments");
            }
            ast::Expression::Integer(_) => panic!("Cannot return an integer literal from a floating point function"),
            ast::Expression::Decimal(decimal) => {
                let entry_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry_block);

                let value = float_type.const_float_from_string(decimal);
                self.builder.build_return(Some(&value)).unwrap();
            }
            ast::Expression::Bool(_) => panic!("Cannot return a boolean literal from a floating point function"),
        }
    }
}

const fn get_linkage(public: bool) -> Linkage {
    return match public {
        true => Linkage::External,
        false => Linkage::Private,
    };
}

const fn is_type_primitive(type_kind: &ast::TypeKind) -> bool {
    return match type_kind {
        ast::TypeKind::Primitive(..) => true,
        _ => false,
    }
}

fn try_get_type_primitive<'c>(context: &'c Context, type_kind: &ast::TypeKind) -> Option<BasicTypeEnum<'c>> {
    use ast::Primitive::*;
    match type_kind {
        ast::TypeKind::Primitive(primitive) => return Some(match primitive {
            I8 => context.i8_type().into(),
            I16 => context.i16_type().into(),
            I32 => context.i32_type().into(),
            I64 => context.i64_type().into(),
            I128 => context.i128_type().into(),
            U8 => context.i8_type().into(),
            U16 => context.i16_type().into(),
            U32 => context.i32_type().into(),
            U64 => context.i64_type().into(),
            U128 => context.i128_type().into(),
            U1 => context.bool_type().into(),
            Bool => context.bool_type().into(),
            F16 => context.f16_type().into(),
            F32 => context.f32_type().into(),
            F64 => context.f64_type().into(),
            F128 => context.f128_type().into(),
        }),
        _ => return None,
    };
}

fn get_type_primitive<'c>(context: &'c Context, type_kind: &ast::TypeKind) -> BasicTypeEnum<'c> {
    return try_get_type_primitive(context, type_kind).expect("Primitive type not implemented");
}

const fn is_type_int(type_kind: &ast::TypeKind) -> bool {
    use ast::Primitive::*;
    return match type_kind {
        ast::TypeKind::Primitive(primitive) => return match primitive {
            I8 | I16 | I32 | I64 | I128 => true,
            U8 | U16 | U32 | U64 | U128 => true,
            U1 | Bool => true,
            _ => false,
        },
        _ => false,
    }
}

fn try_get_type_int<'c>(context: &'c Context, type_kind: &ast::TypeKind) -> Option<IntType<'c>> {
    use ast::Primitive::*;
    return Some(match type_kind {
        ast::TypeKind::Primitive(primitive) => match primitive {
            I8 => context.i8_type(),
            I16 => context.i16_type(),
            I32 => context.i32_type(),
            I64 => context.i64_type(),
            I128 => context.i128_type(),
            U8 => context.i8_type(),
            U16 => context.i16_type(),
            U32 => context.i32_type(),
            U64 => context.i64_type(),
            U128 => context.i128_type(),
            U1 => context.bool_type(),
            Bool => context.bool_type(),
            _ => return None,
        },
        _ => return None,
    });
}

fn get_type_int<'c>(context: &'c Context, type_kind: &ast::TypeKind) -> IntType<'c> {
    return try_get_type_int(context, type_kind).expect("Integer type not implemented");
}

const fn is_type_float(type_kind: &ast::TypeKind) -> bool {
    use ast::Primitive::*;
    return match type_kind {
        ast::TypeKind::Primitive(primitive) => return match primitive {
            F16 | F32 | F64 | F128 => true,
            _ => false,
        },
        _ => false,
    }
}

fn try_get_type_float<'c>(context: &'c Context, type_kind: &ast::TypeKind) -> Option<FloatType<'c>> {
    use ast::Primitive::*;
    return Some(match type_kind {
        ast::TypeKind::Primitive(primitive) => match primitive {
            F16 => context.f16_type(),
            F32 => context.f32_type(),
            F64 => context.f64_type(),
            F128 => context.f128_type(),
            _ => return None,
        },
        _ => return None,
    });
}

fn get_type_float<'c>(context: &'c Context, type_kind: &ast::TypeKind) -> FloatType<'c> {
    return try_get_type_float(context, type_kind).expect("Floating point type not implemented");
}

fn get_argument_types<'c>(context: &'c Context, arguments: &[ast::Argument]) -> Box<[BasicMetadataTypeEnum<'c>]> {
    let mut args = Vec::new();
    for arg in arguments {
        let type_kind = match &arg.decl {
            ast::Declaration::ValueDecl(value_decl) => &value_decl.type_kind,
        };
        args.push(get_type_primitive(context, &type_kind).into());
    }
    return args.into();
}

// Example of creating a function
#[allow(unused)]
fn create_int_add_function<'c>(context: &'c Context, builder: &Builder, module: &Module<'c>, int_type: IntType<'c>, name: &'static str) -> FunctionValue<'c> {
    let func_type = int_type.fn_type(&[int_type.into(), int_type.into()], false);
    let function = module.add_function(name, func_type, None);
    let entry_basic_block = context.append_basic_block(function, "entry");

    builder.position_at_end(entry_basic_block);

    let x = function.get_nth_param(0).unwrap().into_int_value();
    let y = function.get_nth_param(1).unwrap().into_int_value();

    let sum = builder.build_int_add(x, y, "add").unwrap();
    builder.build_return(Some(&sum)).unwrap();

    return function;
}
