
mod lexer;
use lexer::Lexer;

mod parser;
use parser::Parser;

mod compiler;
use compiler::Compiler;

mod vm; // assuming you named the vm module file vm.rs or vm/mod.rs
use vm::VirtualMachine;

mod nativefns;

use std::fs;


fn main() {

    print_info_before("Getting source code");
    let source_code = fs::read_to_string("example.tc")
        .expect("Failed to read the file");

    print_info_after("Getting source code", None);

    let _ = log(&source_code);

    let lexer = Lexer::new(source_code);
    let tokens = lexer.lex(); // Returns Vec<Token> perfectly tracked with real spans!

    print_info_after("Tokenizing source code", None);

    let _ = log(&tokens);

    print_info_before("Parsing tokens");
    let mut parser: Parser = Parser::new(tokens);

    let ast: parser::Program = parser.parse_program();

    print_info_after("Parsing tokens", None);

    let _ = log(&ast);

    print_info_before("Compiling AST to Bytecode");
    
    let mut compiler = Compiler::new();
    // Force register native names in the global scope so they are valid identifiers
    compiler.scopes[0].variables.insert("print".to_string(), 9999); // Use a sentinel or unique index mapping
    compiler.scopes[0].variables.insert("len".to_string(), 9998);

    compiler.compile_program(&ast);
    let bytecode = compiler.bytecode;

    print_info_after("Compiling AST to Bytecode", None);
    let _ = log(&bytecode);

    let native_functions = nativefns::get_native_registry();
    let user_functions = compiler.function_registry;

    print_info_before("Executing Virtual Machine");

    // 🌟 2. Pass the registry map straight into your VM constructor
    let mut virtual_machine = VirtualMachine::new(bytecode, native_functions, user_functions);

    if let Some(&main_address) = virtual_machine.user_functions.get("main") {
        virtual_machine.ip = main_address; // Points the VM directly to instruction index 7 instead of 0!
    } else {
        panic!("Runtime Error: No 'main' function was found in the compiled TCode program.");
    }

    virtual_machine.run();

    //print_info_after("Executing Virtual Machine", None);

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

fn log<T: std::fmt::Debug>(content: &T) -> std::io::Result<()> {

    let mut file = File::create(".log")?; // creates .log file or erases an already existing .log file

    // makes newlines actually appear instead of showing "\n"
    let clean_content = format!("{:#?}", content).replace("\\n", "\n");


    file.write_all(clean_content.as_bytes())?; // writes to log file

    Ok(()) // closes file
}