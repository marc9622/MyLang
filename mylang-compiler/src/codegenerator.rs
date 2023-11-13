use std::collections::HashMap;

use inkwell::{values::FunctionValue, types::IntType};
use inkwell::{context::Context, builder::Builder, module::Module};

use crate::ast::{self, IdentifierType, StrRef, IdStr, Scope, DeclType, Expression};

pub fn generate(program: ast::Program) -> StrRef {
    let context = Context::create();
    let builder = context.create_builder();

    let modules: HashMap<IdStr, Module> = {
        let mut map = HashMap::new();
        for (full_name, module) in program.modules {
            map.insert(full_name.clone(), context.create_module(&full_name));
        }
        map
    };

    let module = modules.get(&program.root_module.full_name).unwrap();

    for declaration in program.root_module.declarations {
        use DeclType::*;
        match declaration.decl_type {
            ValueDecl{value_type} => {
                use Expression::*;
                match declaration.expression {
                    Identifier(identifier) => {
                        match identifier.id_type {
                            IdentifierType::Resolved{declaration: other_decl, scope: other_scope} => {
                                let int_type = context.i32_type();
                                let other_module = modules.get(&other_scope.get_module_full_name()).unwrap();
                                let other_global = other_module.get_global(&unsafe {&*other_decl}.identifier).expect("Likely failed because identifier had not yet been declared");
                                let global = module.add_global(int_type, None, &declaration.identifier);
                                let pointer = other_global.as_pointer_value();
                                println!("null: {}, undef: {}, const: {}, ", pointer.is_null(), pointer.is_undef(), pointer.is_const());
                                println!("type: {}, name: {}", pointer.get_type(), pointer.get_name().to_str().unwrap());
                                let pointee = &builder.build_load(int_type, pointer, "load2").unwrap(); // TODO: This panics. I don't know why.
                                global.set_initializer(pointee);
                            },
                            IdentifierType::Unresolved{..} => panic!("Unknown identifier"),
                        };
                    }
                    Integer(integer) => {
                        let int_type = context.i32_type();
                        let value = int_type.const_int(integer.parse::<i64>().unwrap() as u64, true);
                        let global = module.add_global(int_type, None, &declaration.identifier);
                        global.set_initializer(&value);
                    }
                    Decimal(decimal) => {
                        let dec_type = context.f32_type();
                        let value = dec_type.const_float(decimal.parse::<f64>().unwrap());
                        let global = module.add_global(dec_type, None, &declaration.identifier);
                        global.set_initializer(&value);
                    }
                    Block(..) => {
                        todo!();
                    }
                    Type(..) => {
                        todo!();
                    }
                }
            }
            FuncDecl{func_type} => {
                todo!();
            }
        }
    }

    return module.to_string().into();
}

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
