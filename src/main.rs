
mod lexer;
use lexer::lex;

mod process_tokens;
use process_tokens::process_tokens;

mod parser;
use parser::Parser;

use std::fs;

fn main() {
    let source_code: String = fs::read_to_string("example.tc")
        .expect("Failed to read the file");

    let tokens: Vec<String> = lex(source_code);
    println!("lexed: {:?}", tokens);

    let tokens = process_tokens(tokens);
    println!("processed: {:?}", tokens);

    let mut parser = Parser::new(tokens);

    let ast = parser.parse_program();

    println!("parsed: {:#?}", ast);
}
