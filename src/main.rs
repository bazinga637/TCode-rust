mod lexer;
use lexer::Lexer;

mod parser;
use parser::Parser;

mod generator;
use generator::Generator;

mod interpreter;
use interpreter::Interpreter;

mod nativefns;
mod nativemthds; 

use std::fs;

use std::env;

fn main() {
    let _ = File::create(".log"); // wipes log file after each run

    // collect all command-line arguments passed to the program
    let args: Vec<String> = env::args().collect();

    // check if a filename was actually provided
    // args[0] is always the path to the generator executable itself. 
    // args[1] will be the user's input file path.
    if args.len() < 2 {
        println!("Usage: cargo run <filename.tc>");
        std::process::exit(1); // Stop execution early if filename is missing
    }

    let target_filename = &args[1]; // Grab the specified file path string

    print_info_before(&format!("Getting source code from {}", target_filename));
    
    let source_code = fs::read_to_string(target_filename)
        .unwrap_or_else(|_| panic!("Failed to read the file: {}", target_filename));

    print_info_after("Getting source code", None); 

    let _ = log(&source_code);

    let lexer = Lexer::new(source_code);
    let tokens = lexer.lex();

    print_info_after("Tokenizing source code", None);

    let _ = log(&tokens);

    print_info_before("Parsing tokens");
    let mut parser: Parser = Parser::new(tokens);

    let ast: parser::Program = parser.parse_program();

    print_info_after("Parsing tokens", None);

    let _ = log(&ast);

    print_info_before("Generating Bytecode from AST");
    
    let (bytecode, user_functions) = Generator::compile(&ast);

    print_info_after("Generating Bytecode from AST", None);
    
    let _ = log(&bytecode);

    let native_functions = nativefns::get_native_registry();
    let native_methods = nativemthds::get_native_method_registry(); 


    println!("Executing program...");
    // pass the registry map straight into the Interpreter
    let mut interpreter = Interpreter::new(
        bytecode,
        native_functions, 
        native_methods,
        user_functions,
    );

    if let Some(&main_address) = interpreter.user_functions.get("main") {
        interpreter.ip = main_address; // makes the VM start at 'main' function
    } else {
        panic!("Runtime Error: No 'main' function was found in the compiled TCode program.");
    }

    interpreter.run();
}

fn print_info_before(process: &str) {
    println!("[     ] {process}...")
}

fn print_info_after(process: &str, error: Option<std::io::Error>) {
    print!("\x1B[1A\x1B[2K"); //removes previous line so it can be rewritten

    // rewrites line based on if there is an error or not
    match error {
        Some(err) => println!("[ ERR ] {process} FAILED!     \n{err}"),

        None => println!("[ OK! ] {process} DONE!   "),
    };
    
    std::io::stdout().flush().unwrap(); // updates terminal to show edited lines
}

use std::fs::File;
use std::io::Write;
use std::fs::OpenOptions;

fn log<T: std::fmt::Debug>(content: &T) -> std::io::Result<()> {

    let mut file = OpenOptions::new()
        .append(true) // Sets append mode
        .open(".log")?;

    // makes newlines actually appear instead of showing "\n"
    let clean_content = format!("{:#?}", content).replace("\\n", "\n");

    file.write_all(clean_content.as_bytes())?; // writes to log file

    Ok(()) // closes file
}