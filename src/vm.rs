use crate::compiler::OpCode;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Vec<RuntimeValue>),
    Nil,
}

pub struct VirtualMachine {
    bytecode: Vec<OpCode>,
    ip: usize,                      // Instruction Pointer
    stack: Vec<RuntimeValue>,       // Main working data stack
    locals: Vec<RuntimeValue>,      // Local environment variables frame
}

impl VirtualMachine {
    pub fn new(bytecode: Vec<OpCode>) -> Self {
        Self {
            bytecode,
            ip: 0,
            stack: Vec::new(),
            // Pre-fill a small vector workspace array for variable sets
            locals: vec![RuntimeValue::Nil; 256],
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
                    let val = self.locals[slot].clone();
                    self.stack.push(val);
                }
                
                OpCode::SetLocal(slot) => {
                    let val = self.stack.pop().expect("VM Error: SetLocal stack underflow");
                    self.locals[slot] = val;
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
                
                OpCode::LessThan => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Boolean(a < b));
                    }
                }

                OpCode::BuildArray(count) => {
                    let mut items = Vec::new();
                    for _ in 0..count {
                        items.push(self.stack.pop().unwrap());
                    }
                    items.reverse(); // Reverse to balance natural stack pop orders
                    self.stack.push(RuntimeValue::Array(items));
                }

                OpCode::CallMethod { name, arg_count } => {
                    let obj = self.stack.pop().expect("VM Error: Missing object context");
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        args.push(self.stack.pop().unwrap());
                    }
                    args.reverse();

                    match (obj, name.as_str()) {
                        (RuntimeValue::Array(mut list), "append") => {
                            let append_value = args.first().expect("Missing argument for append").clone();
                            list.push(append_value);
                            self.stack.push(RuntimeValue::Array(list)); // Push updated object array back
                        }
                        _ => panic!("Runtime Error: Method call failed or undefined method on target."),
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
                // Inside vm/mod.rs -> VirtualMachine::run() -> match op

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

                OpCode::Equal => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    self.stack.push(RuntimeValue::Boolean(l == r));
                }

                OpCode::GreaterThan => {
                    let r = self.stack.pop().unwrap();
                    let l = self.stack.pop().unwrap();
                    if let (RuntimeValue::Number(a), RuntimeValue::Number(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Boolean(a > b));
                    }
                }

                OpCode::Return => break,

                // Catch-all for any other variants you haven't written backend logic for yet
                _ => todo!("Backend execution logic for opcode variant not implemented yet."),

            }
        }
    }
}
