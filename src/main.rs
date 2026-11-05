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

use std::env;
use std::fs;

use std::path::Path;

fn main() {
    let _ = File::create(".log"); // wipes log file after each run

    // collect all command-line arguments passed to the program
    let args: Vec<String> = env::args().collect();

    let mut target_filename = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--test" => (),

            _ => {
                // If it doesn't look like a flag, treat it as our source code filename
                if target_filename.is_none() {
                    target_filename = Some(arg);
                }
            }
        }
    }
    // check if a filename was actually provided
    // args[0] is always the path to the generator executable itself.
    // args[1] will be the user's input file path.
    let target_filename = match target_filename {
        Some(filename) => filename,
        None => {
            println!("Usage: tcodec [FLAGS] <filename.tc>");
            println!("\nFlags:");
            println!("  I aint got any yet");
            std::process::exit(1); // Stop execution early if filename is missing
        }
    };

    let file_path = Path::new(target_filename);

    if !file_path.exists() {
        eprintln!(
            "[ ERR ] The specified file '{}' does not exist.",
            file_path.display()
        );
        std::process::exit(1);
    }

    print_info_before(&format!("Getting source code from {}", file_path.display()));

    let source_code = fs::read_to_string(target_filename)
        .unwrap_or_else(|_| panic!("Failed to read the file: {}", file_path.display()));

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
    let mut interpreter =
        Interpreter::new(bytecode, native_functions, native_methods, user_functions);

    if let Some(&main_address) = interpreter.user_functions.get("main") {
        interpreter.ip = main_address; // makes the VM start at 'main' function
    } else {
        panic!("Runtime Error: No 'main' function was found in the compiled TCode program.");
    }

    match interpreter.run() {
        Ok(_) => println!("[ OK! ] Program execution completed successfully."),

        Err(error_message) => {
            let span = interpreter.current_span;
            eprintln!(
                "\x1B[1;31mRuntime Error [Line {}, Col {}]:\x1B[0m {}",
                span.line, span.column, error_message
            );
            std::process::exit(1);
        }
    }
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
use std::fs::OpenOptions;
use std::io::Write;

fn log<T: std::fmt::Debug>(content: &T) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .append(true) // Sets append mode
        .open(".log")?;

    // makes newlines actually appear instead of showing "\n"
    let clean_content = format!("{:#?}", content).replace("\\n", "\n");

    file.write_all(clean_content.as_bytes())?; // writes to log file

    Ok(()) // closes file
}
