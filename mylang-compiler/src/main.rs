use std::fs::File;
use std::io::{BufWriter, Write};

use crate::parser::Parser;
use crate::tokenizer::Tokenizer;

mod tokenizer;
mod parser;
mod typechecker;
mod codegenerator;
mod ast;

fn print_and_write_to_file(name: &'static str, header: &str, content: &str) {
    print!("\n### {}:\n\n{}\n", header, content);
    BufWriter::new(File::create(name).unwrap()).write_all(content.as_bytes()).unwrap();
}

fn main() {
    let mut tokenizer = Tokenizer::new(File::open("code/Code.mylang").unwrap());
    print_and_write_to_file("code/Code.tokens", "Tokens", &tokenizer.str());

    let program: ast::Program = {
        let mut parser = Parser::new();
        parser.parse(tokenizer).unwrap();
        let global_namespace: ast::GlobalNamespace = parser.into();
        ast::Program{
            name: "test".into(),
            ast: global_namespace,
        }
    };
    print_and_write_to_file("code/Code.ast", "AST", &(format!("{:#?}", program)));

    print_and_write_to_file("code/Code.ll", "LLVM IR", &codegenerator::generate(program));
}

