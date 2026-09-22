// src/natives.rs
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;
use std::cell::RefCell;
use crate::vm::RuntimeValue;

pub type NativeFn = fn(args: &[RuntimeValue]) -> RuntimeValue;

pub fn get_native_registry() -> HashMap<String, NativeFn> {
    // registry of functions, includes the name, then function
    let mut registry: HashMap<String, NativeFn> = HashMap::new();

    registry.insert("print".to_string(), native_print);
    registry.insert("readline".to_string(), native_readline);
    registry.insert("len".to_string(), native_len);
    registry.insert("split".to_string(), native_split);
    registry.insert("to_number".to_string(), native_to_number);
    registry
}

// defines how to handle print() as a native function
fn stringify_value(val: &RuntimeValue) -> String {
    match val {
        RuntimeValue::Number(n) => n.to_string(),
        RuntimeValue::String(s) => s.clone(),
        RuntimeValue::Boolean(b) => b.to_string(),
        RuntimeValue::Nil => "nil".to_string(),
        RuntimeValue::Array(list_ptr) => {
            let list = list_ptr.borrow();
            let mut result = String::new();
            result.push('[');
            for (i, item) in list.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                // Recursively stringify strings/numbers inside the list cleanly
                if let RuntimeValue::String(s) = item {
                    // Wrap strings in quotes ONLY inside arrays to look like a real list
                    result.push_str(&format!("\"{}\"", s));
                } else {
                    result.push_str(&stringify_value(item));
                }
            }
            result.push(']');
            result
        }
    }
}

// Uses the stringify helper to clean up terminal logs
fn native_print(args: &[RuntimeValue]) -> RuntimeValue {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" "); // Add spaces between multiple arguments
        }
        print!("{}", stringify_value(arg)); 
    }
    println!(); 
    let _ = io::stdout().flush(); 
    RuntimeValue::Nil 
}

fn native_readline(_args: &[RuntimeValue]) -> RuntimeValue {
    let mut input = String::new();
    
    // Read the line from the standard input stream
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            // Rust's read_line keeps the trailing '\n' or '\r\n'. 
            // We use .trim_end() to strip it off so it's a clean string.
            let clean_input = input.trim_end().to_string();
            RuntimeValue::String(clean_input)
        }
        Err(_) => {
            // Fallback placeholder if reading input fails (e.g., EOF reached)
            RuntimeValue::Nil
        }
    }
}

// defines len() as a native function
fn native_len(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Array(list_ptr)) => RuntimeValue::Number(list_ptr.borrow().len() as f64),
        Some(RuntimeValue::String(s)) => RuntimeValue::Number(s.len() as f64),
        _ => panic!("Runtime Error: len() expects an array or string argument."),
    }
}

fn native_split(args: &[RuntimeValue]) -> RuntimeValue {
    // split expects 2 arguments: split("target_string", "delimiter")
    if args.len() < 2 {
        panic!("Runtime Error: split() expects exactly 2 arguments (string, delimiter).");
    }

    match (&args[0], &args[1]) {
        (RuntimeValue::String(target), RuntimeValue::String(delimiter)) => {
            // Split the target string by the delimiter and convert segments to RuntimeValue::String
            let segments: Vec<RuntimeValue> = target
                .split(delimiter)
                .map(|s| RuntimeValue::String(s.to_string()))
                .collect();

            // Wrap the vector in your upgraded Rc<RefCell<...>> type matching the VM
            RuntimeValue::Array(Rc::new(RefCell::new(segments)))
        }
        _ => panic!("Runtime Error: Both arguments to split() must be strings."),
    }
}

fn native_to_number(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::String(s)) => {
            // Attempt to parse the string into an f64 float number
            match s.parse::<f64>() {
                Ok(num) => RuntimeValue::Number(num),
                Err(_) => {
                    // Fallback option: if parsing fails (e.g. to_number("abc")), return 0.0 or Nil
                    RuntimeValue::Number(0.0) 
                }
            }
        }
        Some(RuntimeValue::Number(n)) => RuntimeValue::Number(*n), // Already a number, return it
        _ => panic!("Runtime Error: to_number() expects a string or number argument."),
    }
}
