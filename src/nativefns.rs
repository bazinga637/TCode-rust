// src/natives.rs
use std::collections::HashMap;
use std::io::{self, Write};
use crate::vm::RuntimeValue;

pub type NativeFn = fn(args: &[RuntimeValue]) -> RuntimeValue;

pub fn get_native_registry() -> HashMap<String, NativeFn> {
    let mut registry: HashMap<String, NativeFn> = HashMap::new();

    registry.insert("print".to_string(), native_print);
    registry.insert("len".to_string(), native_len);
    registry
}

fn native_print(args: &[RuntimeValue]) -> RuntimeValue {
    for arg in args {
        match arg {
            RuntimeValue::Number(n) => print!("{}", n),
            RuntimeValue::String(s) => print!("{}", s),
            RuntimeValue::Boolean(b) => print!("{}", b),
            // 🌟 Upgraded: Borrow the array to print its contents nicely
            RuntimeValue::Array(list_ptr) => print!("{:?}", list_ptr.borrow()),
            RuntimeValue::Nil => print!("nil"),
        }
        print!(" "); 
    }
    println!(); 
    let _ = io::stdout().flush(); 
    RuntimeValue::Nil 
}

fn native_len(args: &[RuntimeValue]) -> RuntimeValue {
    match args.first() {
        // 🌟 Fixed: Added .borrow() to access the inner Vec before calling .len()
        Some(RuntimeValue::Array(list_ptr)) => RuntimeValue::Number(list_ptr.borrow().len() as f64),
        Some(RuntimeValue::String(s)) => RuntimeValue::Number(s.len() as f64),
        _ => panic!("Runtime Error: len() expects an array or string argument."),
    }
}
