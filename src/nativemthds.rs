// src/nativemthds.rs
use std::collections::HashMap;
use crate::vm::RuntimeValue;

// The method type signature alias
pub type NativeMethodFn = fn(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue;

pub fn get_native_method_registry() -> HashMap<String, NativeMethodFn> {
    let mut registry: HashMap<String, NativeMethodFn> = HashMap::new();
    
    // Register the available methods
    registry.insert("append".to_string(), method_append);
    registry.insert("get".to_string(), method_get);
    registry.insert("remove".to_string(), method_remove);
    registry.insert("clear".to_string(), method_clear);
    registry.insert("expect".to_string(), method_expect);
    registry.insert("is_numeric".to_string(), method_is_numeric);
    
    registry
}

///  compiler will call this to check if a method requires 'mut' permissions
pub fn is_mutating_method(name: &str) -> bool {
    match name {
        "append" | "clear" | "remove" => true, // array manipulation
        _ => false,
    }
}

fn method_append(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            let append_value = args.first()
                .expect("Runtime Error: Missing argument for append")
                .clone();
            list_ptr.borrow_mut().push(append_value);
            RuntimeValue::Nil
        }
        _ => panic!("Runtime Error: .append() can only be called on Array types."),
    }
}

fn method_get(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::Number(idx_float)) = args.first() {
        let idx = *idx_float as usize;
        
        match obj {
            RuntimeValue::Array(list_ptr) => {
                let list = list_ptr.borrow();
                if idx >= list.len() {
                    panic!("Runtime Error: Array index out of bounds tracking {} on length {}", idx, list.len());
                }
                list[idx].clone()
            }
            RuntimeValue::String(source_str) => {
                if idx >= source_str.len() {
                    panic!("Runtime Error: String index out of bounds tracking {} on length {}", idx, source_str.len());
                }
                let target_char = source_str.chars().nth(idx).unwrap().to_string();
                RuntimeValue::String(target_char)
            }
            _ => panic!("Runtime Error: .get() can only be called on Array or String types."),
        }
    } else {
        panic!("Runtime Error: .get() method requires a numeric index argument.");
    }
}

// Pops an item out of an array by its position coordinate
fn method_remove(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::Number(idx_float)) = args.first() {
        let idx = *idx_float as usize;
        
        match obj {
            RuntimeValue::Array(list_ptr) => {
                let mut list = list_ptr.borrow_mut();
                if idx >= list.len() {
                    panic!("Runtime Error: .remove() index out of bounds tracking {} on length {}", idx, list.len());
                }
                list.remove(idx) // Returns the popped element back to your TCode stack
            }
            _ => panic!("Runtime Error: .remove() can only be called on Array types."),
        }
    } else {
        panic!("Runtime Error: .remove() method requires a numeric index argument.");
    }
}

// Wipes an array completely
fn method_clear(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            list_ptr.borrow_mut().clear();
            RuntimeValue::Nil
        }
        _ => panic!("Runtime Error: .clear() can only be called on Array types."),
    }
}

// Inline assertion fallback guard
fn method_expect(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let error_msg = match args.first() {
        Some(RuntimeValue::String(s)) => s.as_str(),
        _ => "Expected a non-nil value but execution failed.",
    };

    if let RuntimeValue::Nil | RuntimeValue::Boolean(false) = obj {
        panic!("User Program Expect Failure: {}", error_msg);
    } else {
        obj.clone() // Passes the valid non-nil value straight forward
    }
}

fn method_is_numeric(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {                 // the .parse logic finds out if the string is numeric and returns a bool
        RuntimeValue::String(s) => RuntimeValue::Boolean(s.parse::<f64>().is_ok()), 

        _ => panic!("Runtime Error: used method is_numeric() on non-String value.")
    }
}
