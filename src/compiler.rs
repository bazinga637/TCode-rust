// compiler/bytecode.rs

#[derive(Debug, Clone)]
pub enum OpCode {
    PushNumber(f64),
    PushString(String),
    PushBoolean(bool),
    PushNil,
    
    // Memory / Variable Commands
    GetLocal(usize), // Read variable value from index slot
    SetLocal(usize), // Pop value and store into index slot
    
    // Evaluation Commands
    Add, Sub, Mul, Div,
    Equal, NotEqual, LessThan, GreaterThan,
    Pop, // Cleans the stack after a standalone expression statement
    
    // Structures
    BuildArray(usize), // Pop N items to construct an array literal
    CallMethod { name: String, arg_count: usize },
    
    // Jumps (The usize is the instruction pointer index to jump to)
    Jump(usize),
    JumpIfFalse(usize),
    
    CallFunction { name: String, arg_count: usize },
    Return,

}

// compiler/mod.rs
// compiler/mod.rs
use std::collections::HashMap;
use crate::parser::{Program, StatementNode, Statement, ExpressionNode, Expression, LiteralValue, BinaryOperator};

pub struct Scope {
    // Maps variable names to local runtime stack frame slot indices
    variables: HashMap<String, usize>,
}

pub struct Compiler {
    pub bytecode: Vec<OpCode>,
    pub scopes: Vec<Scope>, // A stack of lexical blocks
    
    // 🌟 ADD THIS FIELD HERE:
    pub function_registry: HashMap<String, usize>, 
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
            // Start with a global/root lexical scope
            scopes: vec![Scope { variables: HashMap::new() }],
            
            // 🌟 INITIALIZE IT HERE:
            function_registry: HashMap::new(), 
        }
    }

    // Helper to output bytecode and track its current position
    fn emit(&mut self, op: OpCode) -> usize {
        self.bytecode.push(op);
        self.bytecode.len() - 1
    }

    // Resolve a string variable name to its numerical stack slot (walking scopes backwards)
    fn resolve_local(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.variables.get(name) {
                return Some(slot);
            }
        }
        None
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("Compiler stack error: No active scope found.")
    }

    pub fn compile_program(&mut self, program: &Program) {
        for stmt in &program.body {
            self.compile_statement(stmt);
        }
    }

    pub fn compile_statement(&mut self, stmt: &StatementNode) {
        match &stmt.kind {
            Statement::VarDeclaration { name, initializer, .. } => {
                // 1. Evaluate expression onto stack first
                self.compile_expression(initializer);

                // 2. Map variable name to next free local slot
                let slot = self.current_scope_mut().variables.len();
                self.current_scope_mut().variables.insert(name.clone(), slot);

                // 3. Move the value off the working stack into that local slot
                self.emit(OpCode::SetLocal(slot));
            }
            
            Statement::Assignment { name, value } => {
                self.compile_expression(value);
                if let Some(slot) = self.resolve_local(name) {
                    self.emit(OpCode::SetLocal(slot));
                } else {
                    panic!("Compile Error: Variable '{}' assigned before declaration.", name);
                }
            }
            
            Statement::Expression(expr) => {
                self.compile_expression(expr);
                self.emit(OpCode::Pop); // Discard statement expression results to keep stack balanced
            }
            
            Statement::Block(statements) => {
                self.scopes.push(Scope { variables: HashMap::new() });
                for sub_stmt in statements {
                    self.compile_statement(sub_stmt);
                }
                self.scopes.pop();
            }

            Statement::If { condition, then_branch, else_branch } => {
                // 1. Evaluate condition
                self.compile_expression(condition);
                
                // 2. Emit placeholder jump
                let jump_false_ip = self.emit(OpCode::JumpIfFalse(0));
                
                // 3. Process the true code pathway
                self.compile_statement(then_branch);
                
                if let Some(else_stmt) = else_branch {
                    // Skip else block if we executed then block
                    let jump_end_ip = self.emit(OpCode::Jump(0));
                    
                    // Backpatch: false pathway starts exactly here
                    let else_start_ip = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip) {
                        *target = else_start_ip;
                    }
                    
                    self.compile_statement(else_stmt);
                    
                    // Backpatch: destination after else block completes
                    let post_else_ip = self.bytecode.len();
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(jump_end_ip) {
                        *target = post_else_ip;
                    }
                } else {
                    // No else block present, backpatch directly to the exit point
                    let post_if_ip = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip) {
                        *target = post_if_ip;
                    }
                }
            }
            Statement::FunctionDeclaration { name, params, body } => {
                // 1. Skip compiling function body during sequential execution
                let jump_over_func_ip = self.emit(OpCode::Jump(0));
                
                // 2. Track where this function actually starts in the bytecode array
                let func_entry_point = self.bytecode.len();
                // Assuming you add a `function_registry: HashMap<String, usize>` to your Compiler struct
                self.function_registry.insert(name.clone(), func_entry_point);

                // 3. Setup a clean local variable space for this function
                self.scopes.push(Scope { variables: HashMap::new() });

                // 4. Map parameters to index slots
                for (i, param) in params.iter().enumerate() {
                    if let Expression::Variable { name, .. } = &param.kind {
                        self.current_scope_mut().variables.insert(name.clone(), i);
                    }
                }

                // 5. Compile the inner code block
                for body_stmt in body {
                    self.compile_statement(body_stmt);
                }

                // 6. Explicitly exit the function
                self.emit(OpCode::Return);
                
                // 7. Clean up the scope
                self.scopes.pop();

                // 8. Backpatch: Normal execution lands safely past the function body
                let post_func_ip = self.bytecode.len();
                if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(jump_over_func_ip) {
                    *target = post_func_ip;
                }
            }


            Statement::While { condition, body } => {
                let loop_start_ip = self.bytecode.len();
                self.compile_expression(condition);
                
                let jump_false_ip = self.emit(OpCode::JumpIfFalse(0));
                self.compile_statement(body);
                
                self.emit(OpCode::Jump(loop_start_ip));
                
                let loop_end_ip = self.bytecode.len();
                if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip) {
                    *target = loop_end_ip;
                }
            }
            
            _ => todo!("Implement remaining Statement variants (Functions, Returns)"),
        }
    }

    pub fn compile_expression(&mut self, expr: &ExpressionNode) {
        match &expr.kind {
            Expression::Literal(lit) => match lit {
                LiteralValue::Number(n) => { self.emit(OpCode::PushNumber(*n)); }
                LiteralValue::String(s) => { self.emit(OpCode::PushString(s.clone())); }
                LiteralValue::Boolean(b) => { self.emit(OpCode::PushBoolean(*b)); }
                LiteralValue::Nil => { self.emit(OpCode::PushNil); }
            },
            
            Expression::Variable { name, .. } => {
                if let Some(slot) = self.resolve_local(name) {
                    self.emit(OpCode::GetLocal(slot));
                } else {
                    panic!("Compile Error: Variable '{}' used but not declared.", name);
                }
            }
            
            Expression::Binary { operator, left, right } => {
                self.compile_expression(left);
                self.compile_expression(right);
                match operator {
                    BinaryOperator::Add => { self.emit(OpCode::Add); }
                    BinaryOperator::Sub => { self.emit(OpCode::Sub); }
                    BinaryOperator::Mul => { self.emit(OpCode::Mul); }
                    BinaryOperator::Div => { self.emit(OpCode::Div); }
                    BinaryOperator::Equal => { self.emit(OpCode::Equal); }
                    BinaryOperator::LessThan => { self.emit(OpCode::LessThan); }
                    BinaryOperator::GreaterThan => { self.emit(OpCode::GreaterThan); }
                    _ => todo!(),
                };
            }
            
            Expression::Array(elements) => {
                for item in elements {
                    self.compile_expression(item);
                }
                self.emit(OpCode::BuildArray(elements.len()));
            }

            Expression::Call { callee, arguments } => {
                if let Expression::Get { object, name } = &callee.kind {
                    // This is your method branch (like list.append)
                    for arg in arguments {
                        self.compile_expression(arg);
                    }
                    self.compile_expression(object);
                    self.emit(OpCode::CallMethod { 
                        name: name.clone(), 
                        arg_count: arguments.len() 
                    });
                } else if let Expression::Variable { name, .. } = &callee.kind {
                    // 1. Push all argument values onto the stack first
                    for arg in arguments {
                        self.compile_expression(arg);
                    }
                    
                    // 2. Emit function execution instruction
                    self.emit(OpCode::CallFunction {
                        name: name.clone(),
                        arg_count: arguments.len(),
                    });
                } else {
                    panic!("Compile Error: Dynamic/Anonymous function calls are not supported yet.");
                }
            }

            
            Expression::Get { .. } => panic!("Syntax Error: Properties must be explicitly evaluated through a Call context."),
            _ => todo!(),
        }
    }
}
