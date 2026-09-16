use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use crate::compiler::OpCode;
use crate::nativefns::NativeFn;

pub struct VirtualMachine {
    bytecode: Vec<OpCode>,
    pub ip: usize,
    stack: Vec<RuntimeValue>,
    frames: Vec<CallFrame>, 
    natives: HashMap<String, NativeFn>, 
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

impl VirtualMachine {
    // 🌟 Updated: Added user_functions to parameters
    pub fn new(
        bytecode: Vec<OpCode>, 
        natives: HashMap<String, NativeFn>,
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
                
                OpCode::Add => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Number(a + b));
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
                    
                    // 🌟 Wrap the generated vec in Rc::new(RefCell::new(...))
                    self.stack.push(RuntimeValue::Array(Rc::new(RefCell::new(items))));
                }

                OpCode::CallMethod { name, arg_count } => {
                    // 1. 🌟 FIXED: Pop the object context FIRST because it's sitting on top of the stack
                    let obj = self.stack.pop().expect("VM Error: Missing object context");

                    // 2. Pop the arguments sitting underneath it
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        args.push(self.stack.pop().expect("VM Error: Missing argument for method call"));
                    }
                    // Since we pop leftwards down the stack, reverse them to preserve source order
                    args.reverse(); 

                    // 3. Execute the operation on your Rc<RefCell> pointer type
                    match (obj, name.as_str()) {
                        (RuntimeValue::Array(list_ptr), "append") => {
                            let append_value = args.first().expect("Runtime Error: Missing argument for append").clone();
                            
                            // Mutate the array directly in shared memory
                            list_ptr.borrow_mut().push(append_value);
                            
                            // Push Nil back to keep the compiler's following Pop instruction happy
                            self.stack.push(RuntimeValue::Nil); 
                        }
                        _ => panic!("Runtime Error: Method call failed or undefined method on target. Found type profile mismatch."),
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
                        
                        // 🌟 Ensure right-to-left popping places the first parameter at index 0
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

                    // 1. Grab the computed return value (or the placeholder Nil) off the top of the stack
                    let return_value = self.stack.pop().expect("VM Error: Return statement stack underflow");

                    // 2. Pop the completed function frame off our call stack to restore the caller's layout
                    let completed_frame = self.frames.pop().expect("VM Error: Frame stack underflow");
                    
                    // 3. Teleport our instruction pointer back to exactly where the caller left off
                    self.ip = completed_frame.return_address;

                    // 4. Push the return value BACK onto the stack so the caller frame can read it
                    self.stack.push(return_value);
                }
            }
        }
    }
}
