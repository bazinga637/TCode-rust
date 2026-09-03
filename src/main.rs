
mod lexer;
use lexer::lex;

use std::fs;

fn main() {
    let content: String = fs::read_to_string("example.tc")
        .expect("Failed to read the file");

    let lexed_contnet: Vec<String> = lex(content);
    println!("{:?}", lexed_contnet);
}
