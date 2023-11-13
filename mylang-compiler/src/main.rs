use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use crate::parser::Parser;
use crate::tokenizer::Tokenizer;

mod tokenizer;
mod parser;
mod typechecker;
mod codegenerator;
mod ast;

fn write_to_file(name: &'static str, content: &str) {
    BufWriter::new(File::create(name).unwrap()).write_all(content.as_bytes()).unwrap();
}

fn main() {
    let mut tokenizer = Tokenizer::new(File::open("example/Code.mylang").unwrap()); {
        let string = tokenizer.str();
        print!("\n### Tokens:\n\n{}\n", string);
        write_to_file("example/Code.tokens", &string);
    }

    let program: ast::Program; {
        let mut parser = Parser::new("test");
        parser.parse(tokenizer).unwrap();
        let root_module: ast::Namespace = parser.into();
        let mut modules: HashMap<ast::IdStr, *const ast::Namespace> = HashMap::new();
        modules.insert(root_module.name.clone(), &root_module);
        program = ast::Program {
            root_module,
            namespaces: modules,
        };
        let string = format!("{:#?}", program);
        print!("\n### AST:\n\n{:#?}\n\n", string);
        write_to_file("example/Code.ast", &string);
    };

    {
        let string = codegenerator::generate(program);
        print!("\n### LLVM IR:\n\n{}\n", string);
        write_to_file("example/Code.ll", &string);
    }
}

