use crate::generator::OpCode;
use crate::lexer::Span;
use crate::nativefns::NativeFn;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Interpreter {
    bytecode: Vec<OpCode>,
    pub ip: usize,
    stack: Vec<RuntimeValue>,
    frames: Vec<CallFrame>,
    natives: HashMap<String, NativeFn>,
    native_methods: HashMap<String, crate::nativemthds::NativeMethodFn>,
    pub user_functions: HashMap<String, usize>,
    pub current_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Array(Rc<RefCell<Vec<RuntimeValue>>>),
    Nil,
    Error(String),
}

#[derive(Debug, Clone)]
struct CallFrame {
    return_address: usize,
    locals: Vec<RuntimeValue>,
}

// turns any list of printable arguments into a RuntimeValue::Error()
macro_rules! error {
    ($first:expr $(, $extra:expr)* $(,)?) => {{
        let err = $first.to_string();
        $(
            let err = format!("{}{}", err, $extra);
        )*
        Err(err)
        }};
}

impl Interpreter {
    pub fn new(
        bytecode: Vec<OpCode>,
        natives: HashMap<String, NativeFn>,
        native_methods: HashMap<String, crate::nativemthds::NativeMethodFn>,
        user_functions: HashMap<String, usize>,
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
            current_span: Span { line: 1, column: 1 },
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        while self.ip < self.bytecode.len() {
            let op = self.bytecode[self.ip].clone();
            self.ip += 1;

            match op {
                OpCode::LineUpdate(span) => self.current_span = span,

                OpCode::PushNumber(n) => self.stack.push(RuntimeValue::Number(n)),
                OpCode::PushString(s) => self.stack.push(RuntimeValue::String(s)),
                OpCode::PushBoolean(b) => self.stack.push(RuntimeValue::Boolean(b)),
                OpCode::PushNil => self.stack.push(RuntimeValue::Nil),

                OpCode::GetLocal(slot) => {
                    let current_frame = self
                        .frames
                        .last()
                        .ok_or("Interpreter Error: No active frame")?;
                    let val = current_frame.locals[slot].clone();
                    self.stack.push(val);
                }

                OpCode::SetLocal(slot) => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: SetLocal stack underflow")?;
                    let current_frame =
                        self.frames.last_mut().ok_or("VM Error: No active frame")?;
                    current_frame.locals[slot] = val;
                }

                OpCode::Pop => {
                    self.stack
                        .pop()
                        .ok_or("Interpreter Error: Pop stack underflow")?;
                }

                OpCode::Negate => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Negate operand missing")?;
                    if let RuntimeValue::Number(n) = val {
                        self.stack.push(RuntimeValue::Number(-n));
                    } else {
                        return error!(
                            "Runtime Error: Negation (-) can only be applied to numeric type values.",
                        );
                    }
                }

                OpCode::Not => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Not operand missing")?;
                    if let RuntimeValue::Boolean(b) = val {
                        self.stack.push(RuntimeValue::Boolean(!b));
                    } else {
                        return error!(
                            "Runtime Error: Logical NOT (!) can only be applied to boolean type values.",
                        );
                    }
                }

                OpCode::Add => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Add right operand missing")?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Add left operand missing")?;

                    match (l, r) {
                        (RuntimeValue::Number(a), RuntimeValue::Number(b)) => {
                            self.stack.push(RuntimeValue::Number(a + b));
                        }
                        (RuntimeValue::String(mut a), RuntimeValue::String(b)) => {
                            a.push_str(&b);
                            self.stack.push(RuntimeValue::String(a));
                        }
                        _ => {
                            return error!(
                                "Runtime Error: Invalid operand types profile for binary addition (+) operation.",
                            );
                        }
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
                        if b == 0.0 {
                            return error!("Runtime Error: Division by zero");
                        }
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
                    let r = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Equal right operand missing")?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Equal left operand missing")?;
                    self.stack.push(RuntimeValue::Boolean(l == r));
                }

                OpCode::NotEqual => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: NotEqual right operand missing")?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: NotEqual left operand missing")?;
                    self.stack.push(RuntimeValue::Boolean(l != r));
                }

                OpCode::And => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: And right operand missing")?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: And left operand missing")?;

                    if let (RuntimeValue::Boolean(a), RuntimeValue::Boolean(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Boolean(a && b));
                    } else {
                        return error!(
                            "Runtime Error: AND (&&) can only be applied to boolean values.",
                        );
                    }
                }

                OpCode::Or => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Or right operand missing")?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Or left operand missing")?;

                    if let (RuntimeValue::Boolean(a), RuntimeValue::Boolean(b)) = (l, r) {
                        self.stack.push(RuntimeValue::Boolean(a && b));
                    } else {
                        return error!(
                            "Runtime Error: OR (||) can only be applied to boolean values.",
                        );
                    }
                }

                OpCode::BuildArray(count) => {
                    let mut items = Vec::new();
                    for _ in 0..count {
                        items.push(self.stack.pop().unwrap());
                    }
                    items.reverse();
                    // Wrap the generated vec in Rc::new(RefCell::new(...))
                    self.stack
                        .push(RuntimeValue::Array(Rc::new(RefCell::new(items))));
                }

                OpCode::CallMethod { name, arg_count } => {
                    let mut args = Vec::new();
                    for _ in 0..arg_count {
                        args.push(
                            self.stack
                                .pop()
                                .ok_or("Interpreter Error: Missing argument for method call")?,
                        );
                    }
                    args.reverse();

                    let obj = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Missing object context")?;

                    if let Some(method_fn) = self.native_methods.get(&name) {
                        let result = method_fn(&obj, &args);
                        self.stack.push(result);
                    } else {
                        return error!(
                            "Runtime Error: Undefined method or type profile mismatch for this object instance",
                        );
                    }
                }

                OpCode::Jump(target) => {
                    self.ip = target;
                }

                OpCode::JumpIfFalse(target) => {
                    let condition = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Missing jump condition")?;
                    if let RuntimeValue::Boolean(false) = condition {
                        self.ip = target;
                    }
                }

                OpCode::CallFunction { name, arg_count } => {
                    if let Some(native_fn) = self.natives.get(&name) {
                        let mut args = Vec::new();
                        for _ in 0..arg_count {
                            args.push(self.stack.pop().unwrap());
                        }
                        args.reverse();
                        let result = native_fn(&args);
                        self.stack.push(result);
                    } else {
                        let function_entry_address = match self.user_functions.get(&name) {
                            Some(&addr) => addr,
                            None => {
                                return error!(
                                    "Runtime Error: Attempted to call an undefined function target",
                                );
                            }
                        };

                        let mut local_workspace = vec![RuntimeValue::Nil; 256];

                        // Ensure right-to-left popping places the first parameter at index 0
                        for i in (0..arg_count).rev() {
                            local_workspace[i] = self
                                .stack
                                .pop()
                                .ok_or("Interpreter Error: Missing function argument")?;
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
                    let return_value = self
                        .stack
                        .pop()
                        .ok_or("Interpreter Error: Return statement stack underflow")?;

                    // Pop the completed function frame off our call stack to restore the caller's layout
                    let completed_frame = self
                        .frames
                        .pop()
                        .ok_or("Interpreter Error: Frame stack underflow")?;

                    // Teleport our instruction pointer back to exactly where the caller left off
                    self.ip = completed_frame.return_address;

                    // Push the return value BACK onto the stack so the caller frame can read it
                    self.stack.push(return_value);
                }
            }
            if let Some(RuntimeValue::Error(_)) = self.stack.last() {
                if let Some(RuntimeValue::Error(msg)) = self.stack.pop() {
                    return Err(msg); // Exits the interpreter loop cleanly with custom message
                }
            }
        }
        Ok(())
    }
}
