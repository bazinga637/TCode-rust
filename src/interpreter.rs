use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use crate::generator::OpCode;
use crate::nativefns::NativeFn;

pub struct Interpreter {
    bytecode: Vec<OpCode>,
    pub ip: usize,
    stack: Vec<RuntimeValue>,
    frames: Vec<CallFrame>, 
    natives: HashMap<String, NativeFn>, 
    native_methods: HashMap<String, crate::nativemthds::NativeMethodFn>, 
    // 🌟 Added: Map function names to their bytecode entry addresses
    pub user_functions: HashMap<String, usize>, 
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Rc<RefCell<Vec<RuntimeValue>>>), // Note: For a true borrow checker, you will later want Std::Rc<RefCell<Vec<RuntimeValue>>>
    Nil,
}

#[derive(Debug, Clone)]
struct CallFrame {
    return_address: usize,
    locals: Vec<RuntimeValue>,
}

impl Interpreter {
    pub fn new(
        bytecode: Vec<OpCode>, 
        natives: HashMap<String, NativeFn>,
        native_methods: HashMap<String, crate::nativemthds::NativeMethodFn>, 
        user_functions: HashMap<String, usize>
    ) -> Self {
        let global_frame = CallFrame {
            return_address: 0,
            locals: vec![RuntimeValue::Nil; 256],
        };

        Self {
            bytecode,
            ip: 0,
            stack: Vec::new(),
            frames: vec![global_frame],
            natives,
            native_methods, 
            user_functions,
        }
    }

    pub fn run(&mut self) {
        while self.ip < self.bytecode.len() {
            let op = self.bytecode[self.ip].clone();
            self.ip += 1;

            match op {
                OpCode::PushNumber(n) => self.stack.push(RuntimeValue::Number(n)),
                OpCode::PushString(s) => self.stack.push(RuntimeValue::String(s)),
                OpCode::PushBoolean(b) => self.stack.push(RuntimeValue::Boolean(b)),
                OpCode::PushNil => self.stack.push(RuntimeValue::Nil),
                
                OpCode::GetLocal(slot) => {
                    let current_frame = self.frames.last().expect("VM Error: No active frame");
                    let val = current_frame.locals[slot].clone();
                    self.stack.push(val);
                }

                OpCode::SetLocal(slot) => {
                    let val = self.stack.pop().expect("VM Error: SetLocal stack underflow");
                    let current_frame = self.frames.last_mut().expect("VM Error: No active frame");
                    current_frame.locals[slot] = val;
                }
                
                OpCode::Pop => {
                    self.stack.pop().expect("VM Error: Pop stack underflow");
                }

                OpCode::Negate => {
                    let val = self.stack.pop().expect("VM Error: Negate operand missing");
                    if let RuntimeValue::Number(n) = val {
                        self.stack.push(RuntimeValue::Number(-n));
                    } else {
                        panic!("Runtime Error: Negation (-) can only be applied to numeric type values.");
                    }
                }

                OpCode::Not => {
                    let val = self.stack.pop().expect("VM Error: Not operand missing");
                    if let RuntimeValue::Boolean(b) = val {
                        self.stack.push(RuntimeValue::Boolean(!b));
                    } else {
                        panic!("Runtime Error: Logical NOT (!) can only be applied to boolean type values.");
                    }
                }
                
                OpCode::Add => {
                    let r = self.stack.pop().expect("VM Error: Add right operand missing");
                    let l = self.stack.pop().expect("VM Error: Add left operand missing");
                    
                    match (l, r) {
                        (RuntimeValue::Number(a), RuntimeValue::Number(b)) => {
                            self.stack.push(RuntimeValue::Number(a + b));
                        }
                        (RuntimeValue::String(mut a), RuntimeValue::String(b)) => {
                            a.push_str(&b);
                            self.stack.push(RuntimeValue::String(a));
                        }
                        _ => panic!("Runtime Error: Invalid operand types profile for binary addition (+) operation."),
                    }
                }

                
                OpCode::Sub => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Number(a - b));
                    }
                }

                OpCode::Mul => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Number(a * b));
                    }
                }

                OpCode::Div => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        if b == 0.0 { panic!("Runtime Error: Division by zero"); }
                        self.stack.push(RuntimeValue::Number(a / b));
                    }
                }

                OpCode::LessThan => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Boolean(a < b));
                    }
                }

                OpCode::GreaterThan => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Boolean(a > b));
                    }
                }

                OpCode::Equal => {
                    let r = self.stack.pop().expect("VM Error: Equal right operand missing");
                    let l = self.stack.pop().expect("VM Error: Equal left operand missing");
                    self.stack.push(RuntimeValue::Boolean(l == r));
                }

                OpCode::NotEqual => {
                    let r = self.stack.pop().expect("VM Error: NotEqual right operand missing");
                    let l = self.stack.pop().expect("VM Error: NotEqual left operand missing");
                    self.stack.push(RuntimeValue::Boolean(l != r));
                }

                OpCode::BuildArray(count) => {
                    let mut items = Vec::new();
                    for _ in 0..count {
                        items.push(self.stack.pop().unwrap());
                    }
                    items.reverse(); 
                    
                    // Wrap the generated vec in Rc::new(RefCell::new(...))
                    self.stack.push(RuntimeValue::Array(Rc::new(RefCell::new(items))));
                }

                OpCode::CallMethod { name, arg_count } => {
                    // 1. Pop the arguments sitting on TOP of the stack first
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        args.push(self.stack.pop().expect("VM Error: Missing argument for method call"));
                    }
                    args.reverse(); 

                    // 2. NOW the object context is sitting at the top of the stack. Pop it!
                    let obj = self.stack.pop().expect("VM Error: Missing object context");

                    if let Some(method_fn) = self.native_methods.get(&name) {
                        let result = method_fn(&obj, &args);
                        self.stack.push(result); 
                    } else {
                        panic!("Runtime Error: Undefined method or type profile mismatch for '.{}()'", name);
                    }
                }
                
                OpCode::Jump(target) => {
                    self.ip = target;
                }
                
                OpCode::JumpIfFalse(target) => {
                    let condition = self.stack.pop().expect("VM Error: Missing jump condition");
                    if let RuntimeValue::Boolean(false) = condition {
                        self.ip = target;
                    }
                }

                OpCode::CallFunction { name, arg_count } => {
                    if let Some(native_fn) = self.natives.get(&name) {
                        let mut args = Vec::new();
                        for _ in 0..arg_count { args.push(self.stack.pop().unwrap()); }
                        args.reverse();
                        let result = native_fn(&args);
                        self.stack.push(result);
                    } else {
                        let function_entry_address = *self.user_functions.get(&name)
                            .unwrap_or_else(|| panic!("Runtime Error: Undefined function '{}'", name));

                        let mut local_workspace = vec![RuntimeValue::Nil; 256];
                        
                        // Ensure right-to-left popping places the first parameter at index 0
                        for i in (0..arg_count).rev() {
                            local_workspace[i] = self.stack.pop().expect("VM Error: Missing function argument");
                        }

                        let new_frame = CallFrame {
                            return_address: self.ip, 
                            locals: local_workspace,
                        };

                        self.frames.push(new_frame);
                        self.ip = function_entry_address;
                    }
                }

                OpCode::Return => {
                    if self.frames.len() <= 1 {
                        break; // End execution if we return out of the main script file scope
                    }
                    // Grab the computed return value (or the placeholder Nil) off the top of the stack
                    let return_value = self.stack.pop().expect("VM Error: Return statement stack underflow");

                    // Pop the completed function frame off our call stack to restore the caller's layout
                    let completed_frame = self.frames.pop().expect("VM Error: Frame stack underflow");
                    
                    // Teleport our instruction pointer back to exactly where the caller left off
                    self.ip = completed_frame.return_address;

                    // Push the return value BACK onto the stack so the caller frame can read it
                    self.stack.push(return_value);
                }
            }
        }
    }
}
