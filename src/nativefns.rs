// src/natives.rs
use crate::interpreter::RuntimeValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

pub type NativeFn = fn(args: &[RuntimeValue]) -> RuntimeValue;

pub fn get_native_registry() -> HashMap<String, NativeFn> {
    // registry of functions, includes the name, then function
    let mut registry: HashMap<String, NativeFn> = HashMap::new();

    registry.insert("print".to_string(), native_print);
    registry.insert("readline".to_string(), native_readline);
    registry.insert("len".to_string(), native_len);
    registry.insert("split".to_string(), native_split);
    registry.insert("to_number".to_string(), native_to_number);
    registry.insert("panic".to_string(), native_panic);
    registry.insert("type".to_string(), native_type);
    registry.insert("range".to_string(), native_range);
    registry.insert("assert".to_string(), native_assert);
    registry.insert("exit".to_string(), native_exit);
    registry.insert("chr".to_string(), native_chr);
    registry.insert("ord".to_string(), native_ord);

    registry
}

// turns any list of printable arguments into a RuntimeValue::Error()
macro_rules! error {
    ($first:expr $(, $extra:expr)* $(,)?) => {{
        let err = $first.to_string();
        $(
            let err = format!("{}{}", err, $extra);
        )*
        RuntimeValue::Error(err)
        }};
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
        RuntimeValue::Error(s) => s.to_string(),
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
        _ => error!("Runtime Error: len() expects an array or string argument."),
    }
}

fn native_split(args: &[RuntimeValue]) -> RuntimeValue {
    // split expects 2 arguments: split("target_string", "delimiter")
    if args.len() < 2 {
        error!("Runtime Error: split() expects exactly 2 arguments (string, delimiter).");
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
        (RuntimeValue::String(_), _) => {
            error!("Runtime Error: Second argument of split() must also be a string.")
        }

        (_, RuntimeValue::String(_)) => {
            error!("Runtime Error: First argument of split() must also be a string")
        }

        _ => error!("Runtime Error: Both arguments to split() must be strings."),
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

        _ => error!("Runtime Error: to_number() expects a string or number argument."),
    }
}

fn native_panic(args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::String(msg)) = args.first() {
        error!("User Panic: ", msg)
    } else {
        error!("User Panic: (No error message provided)")
    }
}

fn native_type(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Number(_)) => RuntimeValue::String("number".to_string()),
        Some(RuntimeValue::String(_)) => RuntimeValue::String("string".to_string()),
        Some(RuntimeValue::Boolean(_)) => RuntimeValue::String("boolean".to_string()),
        Some(RuntimeValue::Array(_)) => RuntimeValue::String("array".to_string()),
        Some(RuntimeValue::Nil) => RuntimeValue::String("nil".to_string()),
        Some(RuntimeValue::Error(_)) => RuntimeValue::String("error".to_string()),
        None => error!("Runtime Error: type() expects 1 argument, found 0."),
    }
}

fn native_range(args: &[RuntimeValue]) -> RuntimeValue {
    if args.len() < 2 {
        return error!("Runtime Error: range() expects exactly 2 numeric arguments (start, end).");
    }
    match (&args[0], &args[1]) {
        (RuntimeValue::Number(start), RuntimeValue::Number(end)) => {
            let mut list = Vec::new();
            let mut current = *start;
            while current < *end {
                list.push(RuntimeValue::Number(current));
                current += 1.0;
            }
            RuntimeValue::Array(Rc::new(RefCell::new(list)))
        }
        _ => error!("Runtime Error: Arguments to range() must be numbers."),
    }
}

fn native_assert(args: &[RuntimeValue]) -> RuntimeValue {
    if args.is_empty() {
        return error!("Runtime Error: assert() expects at least a boolean condition argument.");
    }
    let assertion_passed = match &args[0] {
        RuntimeValue::Boolean(b) => *b,
        RuntimeValue::Nil => false,
        _ => true,
    };
    if !assertion_passed {
        if let Some(RuntimeValue::String(custom_msg)) = args.get(1) {
            error!("Assertion Failure: ", custom_msg)
        } else {
            error!("Assertion Failure: Condition evaluated to false.")
        }
    } else {
        RuntimeValue::Nil
    }
}

fn native_exit(args: &[RuntimeValue]) -> RuntimeValue {
    let exit_code = match args.first() {
        Some(RuntimeValue::Number(n)) => *n as i32,
        _ => 0,
    };
    std::process::exit(exit_code);
}

fn native_chr(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::Number(n)) => {
            let code = *n as u32;
            match char::from_u32(code) {
                Some(c) => RuntimeValue::String(c.to_string()),
                None => error!(
                    "Runtime Error: chr() integer value ",
                    code, " is not a valid Unicode scalar."
                ),
            }
        }
        _ => error!("Runtime Error: chr() expects a numeric Unicode integer argument."),
    }
}

fn native_ord(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        Some(RuntimeValue::String(s)) => {
            if let Some(c) = s.chars().next() {
                RuntimeValue::Number(c as u32 as f64)
            } else {
                error!("Runtime Error: ord() cannot read an empty string layout frame.")
            }
        }
        _ => error!("Runtime Error: ord() expects a valid string context argument."),
    }
}
