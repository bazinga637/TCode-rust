// src/nativemthds.rs
use crate::interpreter::RuntimeValue;
use std::collections::HashMap;

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
    registry.insert("pop".to_string(), method_pop);
    registry.insert("contains".to_string(), method_contains);
    registry.insert("trim".to_string(), method_trim);
    registry.insert("starts_with".to_string(), method_starts_with);
    registry.insert("ends_with".to_string(), method_ends_with);
    registry.insert("join".to_string(), method_join);

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

///  compiler will call this to check if a method requires 'mut' permissions
pub fn is_mutating_method(name: &str) -> bool {
    match name {
        "append" | "clear" | "remove" | "pop" => true, // array manipulation
        _ => false,
    }
}

fn method_append(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            let append_value = args
                .first()
                .expect("Runtime Error: Missing argument for append")
                .clone();
            list_ptr.borrow_mut().push(append_value);
            RuntimeValue::Nil
        }
        _ => error!("Runtime Error: .append() can only be called on Array types, not type"),
    }
}

fn method_get(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    if let Some(RuntimeValue::Number(idx_float)) = args.first() {
        let idx = *idx_float as usize;

        match obj {
            RuntimeValue::Array(list_ptr) => {
                let list = list_ptr.borrow();
                if idx >= list.len() {
                    return error!(
                        "Runtime Error: Array index out of bounds tracking ",
                        idx,
                        " on length ",
                        list.len()
                    );
                }
                list[idx].clone()
            }
            RuntimeValue::String(source_str) => {
                if idx >= source_str.len() {
                    return error!(
                        "Runtime Error: String index out of bounds tracking ",
                        idx,
                        " on length ",
                        source_str.len()
                    );
                }
                let target_char = source_str.chars().nth(idx).unwrap().to_string();
                RuntimeValue::String(target_char)
            }
            _ => error!("Runtime Error: .get() can only be called on Array or String types.",),
        }
    } else {
        error!("Runtime Error: .get() method requires a numeric index argument.")
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
                    return error!(
                        "Runtime Error: .remove() index out of bounds tracking {} on length {}",
                        idx,
                        list.len()
                    );
                }
                list.remove(idx) // Returns the popped element back to your TCode stack
            }
            _ => error!("Runtime Error: .remove() can only be called on Array types."),
        }
    } else {
        error!("Runtime Error: .remove() method requires a numeric index argument.")
    }
}

// Wipes an array completely
fn method_clear(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            list_ptr.borrow_mut().clear();
            RuntimeValue::Nil
        }
        _ => error!("Runtime Error: .clear() can only be called on Array types."),
    }
}

// Inline assertion fallback guard
fn method_expect(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let error_msg = match args.first() {
        Some(RuntimeValue::String(s)) => s.as_str(),
        _ => "Expected a non-nil value but execution failed.",
    };

    if let RuntimeValue::Nil | RuntimeValue::Boolean(false) = obj {
        error!("User Program Expect Failure: ", error_msg)
    } else {
        obj.clone() // Passes the valid non-nil value straight forward
    }
}

fn method_is_numeric(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        // the .parse logic finds out if the string is numeric and returns a bool
        RuntimeValue::String(s) => RuntimeValue::Boolean(s.parse::<f64>().is_ok()),

        _ => error!("Runtime Error: used method is_numeric() on non-String value."),
    }
}
fn method_pop(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::Array(list_ptr) => {
            let mut list = list_ptr.borrow_mut();
            list.pop().unwrap_or(RuntimeValue::Nil)
        }
        _ => error!("Runtime Error: .pop() can only be called on Array types."),
    }
}

fn method_contains(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::String(search_target)) = args.first() else {
        return error!("Runtime Error: .contains() method requires a string argument.");
    };
    match obj {
        RuntimeValue::String(source_str) => {
            RuntimeValue::Boolean(source_str.contains(search_target))
        }
        _ => error!("Runtime Error: .contains() can only be called on String types."),
    }
}

fn method_trim(obj: &RuntimeValue, _args: &[RuntimeValue]) -> RuntimeValue {
    match obj {
        RuntimeValue::String(source_str) => RuntimeValue::String(source_str.trim().to_string()),
        _ => error!("Runtime Error: .trim() can only be called on String types."),
    }
}

fn method_starts_with(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::String(prefix)) = args.first() else {
        return error!("Runtime Error: .starts_with() method requires a string argument.");
    };
    match obj {
        RuntimeValue::String(source_str) => RuntimeValue::Boolean(source_str.starts_with(prefix)),
        _ => error!("Runtime Error: .starts_with() can only be called on String types."),
    }
}

fn method_ends_with(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::String(suffix)) = args.first() else {
        return error!("Runtime Error: .ends_with() method requires a string argument.");
    };
    match obj {
        RuntimeValue::String(source_str) => RuntimeValue::Boolean(source_str.ends_with(suffix)),
        _ => error!("Runtime Error: .ends_with() can only be called on String types."),
    }
}

fn method_join(obj: &RuntimeValue, args: &[RuntimeValue]) -> RuntimeValue {
    let Some(RuntimeValue::String(separator)) = args.first() else {
        return error!("Runtime Error: .join() method requires a string separator argument.");
    };
    match obj {
        RuntimeValue::Array(list_ptr) => {
            let list = list_ptr.borrow();
            let mut result_string = String::new();
            for (i, item) in list.iter().enumerate() {
                if i > 0 {
                    result_string.push_str(separator);
                }
                match item {
                    RuntimeValue::String(s) => result_string.push_str(s),
                    RuntimeValue::Number(n) => result_string.push_str(&n.to_string()),
                    RuntimeValue::Boolean(b) => result_string.push_str(&b.to_string()),
                    RuntimeValue::Nil => result_string.push_str("nil"),
                    _ => result_string.push_str("[object]"),
                }
            }
            RuntimeValue::String(result_string)
        }
        _ => error!("Runtime Error: .join() can only be called on Array types."),
    }
}
