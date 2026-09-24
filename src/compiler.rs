#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ownership {
    Owned,
    SharedBorrow,
    MutableBorrow,
}


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
    Negate, // -x
    Not,    // !x
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Shared,    // Equivalent to &
    Mut,       // Equivalent to &mut
}

#[derive(Debug, Clone)]
pub struct Loan {
    pub provider_name: String, // The source variable being borrowed (e.g., "numbers")
    pub kind: BorrowKind,       // Is it a shared or mutable borrow?
    pub borrow_scope_depth: usize, // Which scope level created this lease?
}

#[derive(Debug, Clone)]
pub struct VariableMetadata {
    pub slot: usize,
    pub is_mutable: bool,
    // Track who THIS variable is currently borrowing from
    pub borrowing_from: Option<Loan>, 
    // Track who is actively borrowing FROM this variable
    pub active_loans: Vec<Loan>,      
}

pub struct Scope {
    // Map names to an expanded tracking structure instead of a raw usize slot
    pub variables: HashMap<String, VariableMetadata>,
    pub depth_start_index: usize,
}

pub struct Compiler {
    pub bytecode: Vec<OpCode>,
    pub scopes: Vec<Scope>, // A stack of lexical blocks

    pub function_registry: HashMap<String, usize>, 
    pub locals_count: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
            // Start with a global/root lexical scope
            scopes: vec![Scope { variables: HashMap::new(), depth_start_index: 0 }],

            function_registry: HashMap::new(), 
            locals_count: 0,
        }
    }

    fn error(&self, message: &str, span: crate::lexer::Span) -> ! {
        panic!(
            "Compile Error [Line {}, Col {}]: {}",
            span.line, span.column, message
        );
    }

    /// Look up variable tracking metadata immutably (walking scopes backwards)
    fn resolve_variable_metadata(&self, name: &str) -> Option<&VariableMetadata> {
        for scope in self.scopes.iter().rev() {
            if let Some(metadata) = scope.variables.get(name) {
                return Some(metadata);
            }
        }
        None
    }

    /// Look up variable tracking metadata mutably so we can append/remove loans
    fn resolve_variable_metadata_mut(&mut self, name: &str) -> Option<&mut VariableMetadata> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(metadata) = scope.variables.get_mut(name) {
                return Some(metadata);
            }
        }
        None
    }

    /// Helper to inject default metadata when a new variable is initialized
    fn insert_variable_metadata(&mut self, name: String, slot: usize, is_mutable: bool) {
        let metadata = VariableMetadata {
            slot,
            is_mutable,
            borrowing_from: None,
            active_loans: Vec::new(),
        };
        self.current_scope_mut().variables.insert(name, metadata);
    }

        // Added a `span` parameter to pin error locations
    fn borrow_variable(&mut self, source_name: &str, borrow_kind: BorrowKind, span: crate::lexer::Span) {
        let current_depth = self.scopes.len();
        
        if let Some(metadata) = self.resolve_variable_metadata_mut(source_name) {
            if borrow_kind == BorrowKind::Mut && !metadata.is_mutable {
                self.error(
                    &format!("Cannot implicitly borrow '{}' mutably because it was not declared with explicit 'mut' keyword!", source_name),
                    span
                );
            }

            for active_loan in &metadata.active_loans {
                match (active_loan.kind, borrow_kind) {
                    (BorrowKind::Mut, _) => {
                        self.error(
                            &format!("Cannot borrow '{}' because it is already mutably borrowed exclusively!", source_name),
                            span
                        );
                    }
                    (BorrowKind::Shared, BorrowKind::Mut) => {
                        self.error(
                            &format!("Cannot mutably borrow '{}' because it is currently read-only borrowed by active loops!", source_name),
                            span
                        );
                    }
                    (BorrowKind::Shared, BorrowKind::Shared) => {}
                }
            }

            metadata.active_loans.push(Loan {
                provider_name: source_name.to_string(),
                kind: borrow_kind,
                borrow_scope_depth: current_depth,
            });
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            variables: HashMap::new(),
            depth_start_index: self.locals_count, // Safely bookmark where we started
        });
    }

    fn pop_scope(&mut self) {
        let current_depth = self.scopes.len();
        
        if let Some(popped_scope) = self.scopes.pop() {
            self.locals_count = popped_scope.depth_start_index;
            
            // Walk backwards through remaining parent scopes 
            // and remove loans that were tied to the popped depth level
            for scope in &mut self.scopes {
                for metadata in scope.variables.values_mut() {
                    metadata.active_loans.retain(|loan| loan.borrow_scope_depth < current_depth);
                }
            }
        }
    }

    /// Main entry point helper to translate an AST into VM input parameters
    pub fn compile(program: &Program) -> (Vec<OpCode>, HashMap<String, usize>) {
        let mut compiler = Compiler::new();

        let native_functions = crate::nativefns::get_native_registry();

        for (index, item) in native_functions.keys().enumerate() {
            // Insert native hooks as read-only global entries
            compiler.scopes[0].variables.insert(item.to_string(), VariableMetadata {
                slot: 9999 - index,
                is_mutable: false,
                borrowing_from: None,
                active_loans: Vec::new(),
            });
        }

        compiler.compile_program(program);
        compiler.finish()
    }
    /// Consumes the compiler and returns the generated bytecode 
    /// alongside the tracked user function positions.
    pub fn finish(self) -> (Vec<OpCode>, HashMap<String, usize>) {
        (self.bytecode, self.function_registry)
    }

    // Helper to output bytecode and track its current position
    fn emit(&mut self, op: OpCode) -> usize {
        self.bytecode.push(op);
        self.bytecode.len() - 1
    }

    // Resolve a string variable name to its numerical stack slot (walking scopes backwards)
    fn resolve_local(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(metadata) = scope.variables.get(name) {
                return Some(metadata.slot);
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
            Statement::VarDeclaration { name, initializer, is_mutable, var_type: _ } => {
                self.compile_expression(initializer);

                let slot = self.locals_count; 
                self.insert_variable_metadata(name.clone(), slot, *is_mutable);

                self.locals_count += 1; 
                self.emit(OpCode::SetLocal(slot));
            }

            Statement::Assignment { name, value } => {
                if let Some(metadata) = self.resolve_variable_metadata(name) {
                    if !metadata.is_mutable {
                        self.error(&format!("Cannot assign to variable '{}' because it was not explicitly declared as mutable with 'mut'!", name), stmt.span);
                    }

                    if !metadata.active_loans.is_empty() {
                        self.error(&format!("Cannot mutate variable '{}' because it is currently implicitly borrowed elsewhere!", name), stmt.span);
                    }
                } else {
                    self.error(&format!("Variable '{}' assigned before declaration.", name), stmt.span);
                }
                self.compile_expression(value);
                let slot = self.resolve_local(name).unwrap();
                self.emit(OpCode::SetLocal(slot));

                let current_depth = self.scopes.len();
                for scope in &mut self.scopes {
                    for metadata in scope.variables.values_mut() {
                        // Change '<' to '<=' to safely release temporary locks
                        metadata.active_loans.retain(|loan| loan.borrow_scope_depth < current_depth);
                    }
                }
            }

            Statement::Expression(expr) => {
                self.compile_expression(expr);
                
                // Only emit a Pop if the expression wasn't a standard void method execution!
                // If it's a Call context that targets a method (like .append) which pushes an explicit 
                // Return placeholder or handles its own stack footprint balance, make sure we don't
                // aggressively pop from an empty evaluation stack register.
                match &expr.kind {
                    Expression::Call { callee, .. } => {
                        if let Expression::Get { .. } = &callee.kind {
                            // Methods like x.append() leave Nil on the stack, which Pop clears.
                            // But standalone global function declarations without assignments
                            // shouldn't clobber the stack if they evaluate to void.
                            self.emit(OpCode::Pop);
                        } else {
                            self.emit(OpCode::Pop);
                        }
                    }
                    _ => {
                        self.emit(OpCode::Pop);
                    }
                }
                let current_depth = self.scopes.len();
                for scope in &mut self.scopes {
                    for metadata in scope.variables.values_mut() {
                        // Change '<' to '<=' to safely release the current line's temporary locks
                        metadata.active_loans.retain(|loan| loan.borrow_scope_depth < current_depth);
                    }
                }

            }

            Statement::Block(statements) => {
                // Use the parameterless helper to keep counters aligned
                self.push_scope();
                for sub_stmt in statements {
                    self.compile_statement(sub_stmt);
                }
                // Use the parameterless helper to reclaim local variable stack slots
                self.pop_scope();
            }

            Statement::Match { target, cases, default_branch } => {
                // 🌟 FIX 1: Evaluate the target expression FIRST (pushes value to evaluation stack)
                self.compile_expression(target);
                
                // 🌟 FIX 2: Compute register slot BEFORE pushing the inner block scope!
                let hidden_target_name = format!("$match_target_{}", self.bytecode.len());
                let target_slot = self.locals_count; 
                self.locals_count += 1;
                
                // 🌟 FIX 3: Store the value into the register slot immediately
                self.insert_variable_metadata(hidden_target_name.clone(), target_slot, false);
                self.emit(OpCode::SetLocal(target_slot));

                // 🌟 FIX 4: Safely open the inner case block scope context using the helper
                self.push_scope(); 

                let mut end_jumps = Vec::new(); 
                let mut next_case_jump_ip: Option<usize> = None;

                // Process structural match cases sequentially
                                // Process structural match cases sequentially
                for case in cases {
                    if let Some(prev_false_jump) = next_case_jump_ip {
                        let current_address = self.bytecode.len();
                        if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(prev_false_jump) {
                            *target = current_address;
                        }
                    }

                    self.emit(OpCode::GetLocal(target_slot));
                    self.compile_expression(&case.pattern);
                    self.emit(OpCode::Equal); 

                    next_case_jump_ip = Some(self.emit(OpCode::JumpIfFalse(0)));

                    // 🌟 FIX: Inline block elements directly to prevent double-scoping indexing misalignments
                    match &case.body.kind {
                        Statement::Block(inner_statements) => {
                            for sub_stmt in inner_statements {
                                self.compile_statement(sub_stmt);
                            }
                        }
                        _ => {
                            self.compile_statement(&case.body);
                        }
                    }

                    end_jumps.push(self.emit(OpCode::Jump(0)));
                }


                if let Some(prev_false_jump) = next_case_jump_ip {
                    let fallthrough_address = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(prev_false_jump) {
                        *target = fallthrough_address;
                    }
                }

                // 🌟 FIX A: Calculate the match end right here, BEFORE the default branch compiles!
                // This ensures all successful pattern arms leap completely past the default block.
                let match_end_address = self.bytecode.len();
                
                // Backpatch successful case paths to jump to this safe end-marker address address
                for exit_jump in end_jumps {
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(exit_jump) {
                        *target = match_end_address;
                    }
                }

                // 🌟 FIX B: Compile the default branch down below the escape terminal vector bounds
                if let Some(default_stmt) = default_branch {
                    self.compile_statement(default_stmt);
                } else {
                    self.emit(OpCode::PushNil);
                    self.emit(OpCode::Pop);
                }

                // Pop the match block scope context safely using the parameterless helper
                self.pop_scope();
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
                // 1. Skip compiling function body during normal sequential execution
                let jump_over_func_ip = self.emit(OpCode::Jump(0));
                
                // 2. Track where this function starts in the bytecode array
                let func_entry_point = self.bytecode.len();
                self.function_registry.insert(name.clone(), func_entry_point);

                // 3. Setup a clean local variable scope space for this function
                self.push_scope();

                // 4. Map parameters directly to slots 0, 1, 2 sequentially
                for (i, param) in params.iter().enumerate() {
                    if let Expression::Variable { name, is_mutable } = &param.kind {
                        self.insert_variable_metadata(name.clone(), i, *is_mutable);
                    }
                }

                // 5. Compile inner body statements while tracking the last item
                let len = body.len();
                for (idx, body_stmt) in body.iter().enumerate() {
                    let is_last_statement = idx == len - 1;

                    if is_last_statement {
                        // If the final statement is an implicit expression return, 
                        // evaluate it but DO NOT append an OpCode::Pop instruction!
                        if let Statement::Expression(expr) = &body_stmt.kind {
                            self.compile_expression(expr);
                            continue; // Skip normal statement processing to bypass Pop
                        }
                    }

                    // Otherwise compile the statement normally (which will Pop if it's a mid-block expression)
                    self.compile_statement(body_stmt);
                }

                // 6. Push a fallback Nil onto the stack right before the trailing Return opcode
                // (This is skipped if an implicit expression was compiled, since the stack is populated!)
                // To safely handle it without adding extra stack overhead:
                // We only emit PushNil if the last statement wasn't an implicit expression return.
                let ends_with_expression = !body.is_empty() && matches!(body.last().unwrap().kind, Statement::Expression(_));
                if !ends_with_expression {
                    self.emit(OpCode::PushNil); 
                }

                self.emit(OpCode::Return);
                
                // 1. Clean up and pop the function local scope frame
                self.pop_scope();

                // Reset the local tracking metadata counts and clear the root scope variables!
                // Since functions are isolated packages, we flush previous scratchpads 
                // so subsequent function passes don't inherit old loan collisions.
                if self.scopes.len() == 1 {
                    self.scopes[0].variables.clear();
                    
                    // Re-register the native system hooks into the wiped root scope
                    let native_fns_list = ["print", "readline", "len", "split", "to_number"];
                    for (index, item) in native_fns_list.iter().enumerate() {
                        self.scopes[0].variables.insert(item.to_string(), VariableMetadata {
                            slot: 9999 - index,
                            is_mutable: false,
                            borrowing_from: None,
                            active_loans: Vec::new(),
                        });
                    }
                }

                // 2. Backpatch: Normal outer execution lands safely past function body
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

            Statement::For { item_name, iterable, body } => {
                let hidden_idx_name = format!("$idx_{}", self.bytecode.len());
                let hidden_list_name = format!("$list_{}", self.bytecode.len());
                
                // Implicit Loop Hook: If the container is a variable, register a shared lease
                let mut source_iterable_var = None;
                if let Expression::Variable { name: var_name, .. } = &iterable.kind {
                    source_iterable_var = Some(var_name.clone());
                    self.borrow_variable(var_name, BorrowKind::Shared, stmt.span);
                }

                self.push_scope();

                // --- STEP A: Initializers ---
                self.compile_expression(iterable);
                let list_slot = self.locals_count;
                self.locals_count += 1;
                self.insert_variable_metadata(hidden_list_name.clone(), list_slot, false);
                self.emit(OpCode::SetLocal(list_slot));

                self.emit(OpCode::PushNumber(0.0));
                let idx_slot = self.locals_count;
                self.locals_count += 1;
                self.insert_variable_metadata(hidden_idx_name.clone(), idx_slot, true);
                self.emit(OpCode::SetLocal(idx_slot));

                // --- STEP B: Loop Condition Guard ---
                let loop_start_ip = self.bytecode.len();
                self.emit(OpCode::GetLocal(idx_slot));
                
                self.emit(OpCode::GetLocal(list_slot));
                self.emit(OpCode::CallFunction { name: "len".to_string(), arg_count: 1 });
                self.emit(OpCode::LessThan); 

                let jump_false_ip = self.emit(OpCode::JumpIfFalse(0));

                // --- STEP C: Inner Loop Workspace ---
                let item_slot = self.locals_count;
                self.locals_count += 1;
                self.insert_variable_metadata(item_name.clone(), item_slot, false);
                
                self.emit(OpCode::GetLocal(list_slot));
                self.emit(OpCode::GetLocal(idx_slot));
                self.emit(OpCode::CallMethod { name: "get".to_string(), arg_count: 1 }); 
                self.emit(OpCode::SetLocal(item_slot));

                self.compile_statement(body);

                // --- STEP D: Loop Counter Increment ---
                self.emit(OpCode::GetLocal(idx_slot));
                self.emit(OpCode::PushNumber(1.0));
                self.emit(OpCode::Add);
                self.emit(OpCode::SetLocal(idx_slot));

                self.emit(OpCode::Jump(loop_start_ip));

                let loop_end_ip = self.bytecode.len();
                if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip) {
                    *target = loop_end_ip;
                }

                self.pop_scope();

                // Implicit Loop Hook: Release container lock when loop concludes
                if let Some(var_name) = source_iterable_var {
                    if let Some(meta) = self.resolve_variable_metadata_mut(&var_name) {
                        meta.active_loans.retain(|l| l.provider_name != var_name);
                    }
                }
            }

            Statement::Return(maybe_expr) => {
                if let Some(expr) = maybe_expr {
                    // 1. If an expression exists (e.g., return 5;), compile it onto the evaluation stack
                    self.compile_expression(expr);
                } else {
                    // 2. If it's a blank return (e.g., return;), push Nil onto the stack as a placeholder
                    self.emit(OpCode::PushNil);
                }
                
                // 3. Emit the Return instruction to break execution out of the current call frame
                self.emit(OpCode::Return);
            }

            _ => todo!("Implement remaining Statement variants"),
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
                self.borrow_variable(name, BorrowKind::Shared, expr.span);

                if let Some(slot) = self.resolve_local(name) {
                    self.emit(OpCode::GetLocal(slot));
                } else {
                    self.error(&format!("Variable '{}' used but not declared.", name), expr.span);
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
                    BinaryOperator::NotEqual => { self.emit(OpCode::NotEqual); }
                    BinaryOperator::LessThan => { self.emit(OpCode::LessThan); }
                    BinaryOperator::GreaterThan => { self.emit(OpCode::GreaterThan); }
                    _ => todo!(),
                };
            }

            Expression::Unary { operator, right } => {
                // 1. Compile the inner expression onto the stack first
                self.compile_expression(right);
                
                // 2. Emit the correct modification opcode based on the operator type
                match operator {
                    crate::parser::UnaryOperator::Minus => {
                        self.emit(OpCode::Negate);
                    }
                    crate::parser::UnaryOperator::Not => {
                        self.emit(OpCode::Not);
                    }
                }
            }
            
            Expression::Array(elements) => {
                for item in elements {
                    self.compile_expression(item);
                }
                self.emit(OpCode::BuildArray(elements.len()));
            }

            Expression::Call { callee, arguments } => {
                // --- PATHWAY A: Method Calls (e.g., x.append(y)) ---
                if let Expression::Get { object, name } = &callee.kind {
                    let is_mutating_method = crate::nativemthds::is_mutating_method(name);

                    if let Expression::Variable { name: obj_name, .. } = &object.kind {
                        if is_mutating_method {
                            // Register implicit exclusive mutable lease
                            self.borrow_variable(obj_name, BorrowKind::Mut, expr.span);
                        } else {
                            // Register implicit read-only shared lease
                            self.borrow_variable(obj_name, BorrowKind::Shared, expr.span);
                        }

                        // Manually emit GetLocal for the variable object instead of 
                        // calling compile_expression. This cuts off double-borrowing panics!
                        if let Some(slot) = self.resolve_local(obj_name) {
                            self.emit(OpCode::GetLocal(slot));
                        } else {
                            self.error(&format!("Variable '{}' used before declaration.", obj_name), expr.span);
                        }
                    } else {
                        // Fallback manner for raw elements (e.g., [1, 2].get(0))
                        self.compile_expression(object);
                    }

                    // Compile the parameters normally
                    for arg in arguments {
                        self.compile_expression(arg);
                    }
                    
                    self.emit(OpCode::CallMethod { 
                        name: name.clone(), 
                        arg_count: arguments.len() 
                    });
                    
                    // Return early to balance statement execution frames
                    return; 
                } 

                // --- PATHWAY B: Standalone Global Function Calls (e.g., print(x), test_to_number()) ---
                if let Expression::Variable { name, .. } = &callee.kind {
                    // Compile function parameters onto the stack
                    for arg in arguments {
                        self.compile_expression(arg);
                    }
                    
                    self.emit(OpCode::CallFunction {
                        name: name.clone(),
                        arg_count: arguments.len(),
                    });
                    
                    // 🌟 CRITICAL FIX: Return early here as well! 
                    // This explicitly registers a complete terminal expression loop, 
                    // stopping any fall-through code clobbering.
                    return; 
                } else {
                    self.error("Dynamic/Anonymous function calls are not supported yet.", expr.span);
                }
            }

            Expression::Get { .. } => panic!("Syntax Error: Properties must be explicitly evaluated through a Call context."),
            _ => todo!(),
        }
    }
}
