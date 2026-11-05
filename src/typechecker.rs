use crate::parser::{
    BinaryOperator, Expression, ExpressionNode, LiteralValue, Program, Statement, StatementNode,
    UnaryOperator,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Number,
    String,
    Boolean,
    Array(Box<Type>),
    Nil,
    Undeclared, // Used for untyped fields that allow type inference
}

impl Type {
    /// Maps the Parser's String type annotations into structured Compiler Enums
    pub fn from_str(s: &str) -> Self {
        let cleaned = s.trim().to_lowercase();

        match cleaned.as_str() {
            "number" | "int" | "float" => Type::Number,
            "string" => Type::String,
            "boolean" | "bool" => Type::Boolean,
            "nil" | "null" => Type::Nil,
            s if s.starts_with('[') && s.ends_with(']') => {
                let inner = &s[1..s.len() - 1];
                Type::Array(Box::new(Type::from_str(inner)))
            }
            _ => Type::Undeclared,
        }
    }
}

pub struct TypeScope {
    pub variables: HashMap<String, Type>,
}

pub struct TypeChecker {
    pub scopes: Vec<TypeScope>,
    pub function_registry: HashMap<String, (Vec<Type>, Type)>, // Maps name -> (ParamTypes, ReturnType)
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            scopes: vec![TypeScope {
                variables: HashMap::new(),
            }],
            function_registry: HashMap::new(),
        };

        // Seed native standard library functions into the core environment
        checker.register_native_functions();
        checker
    }

    fn register_native_functions(&mut self) {
        // format: (ParamTypes, ReturnType)
        self.function_registry
            .insert("print".to_string(), (vec![], Type::Nil)); // Variadic fallback
        self.function_registry
            .insert("readline".to_string(), (vec![], Type::String));
        self.function_registry
            .insert("len".to_string(), (vec![], Type::Number));
        self.function_registry.insert(
            "split".to_string(),
            (
                vec![Type::String, Type::String],
                Type::Array(Box::new(Type::String)),
            ),
        );
        self.function_registry
            .insert("to_number".to_string(), (vec![Type::String], Type::Number));
        self.function_registry
            .insert("panic".to_string(), (vec![Type::String], Type::Nil));
        self.function_registry
            .insert("type".to_string(), (vec![], Type::String));
        self.function_registry.insert(
            "range".to_string(),
            (
                vec![Type::Number, Type::Number],
                Type::Array(Box::new(Type::Number)),
            ),
        );
        self.function_registry.insert(
            "assert".to_string(),
            (vec![Type::Boolean, Type::String], Type::Nil),
        );
        self.function_registry
            .insert("exit".to_string(), (vec![Type::Number], Type::Nil));
        self.function_registry
            .insert("chr".to_string(), (vec![Type::Number], Type::String));
        self.function_registry
            .insert("ord".to_string(), (vec![Type::String], Type::Number));
    }

    fn error(&self, message: &str, span: crate::lexer::Span) -> ! {
        eprintln!(
            "\x1B[1;31mType Error [Line {}, Col {}]:\x1B[0m {}",
            span.line, span.column, message
        );
        std::process::exit(1);
    }

    fn push_scope(&mut self) {
        self.scopes.push(TypeScope {
            variables: HashMap::new(),
        });
    }

    fn pop_scope(&mut self) {
        self.scopes
            .pop()
            .expect("Type Checker internal stack error: Scope underflow.");
    }

    fn resolve_variable_type(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.variables.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    fn declare_variable(&mut self, name: String, t: Type) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.variables.insert(name, t);
        }
    }

    /// Primary entry point to type-check an entire Program AST pass
    pub fn check(program: &Program) {
        let mut checker = TypeChecker::new();

        // First Pass: Pre-register all top-level functions so forward-calls work perfectly
        for stmt in &program.body {
            if let Statement::FunctionDeclaration {
                name,
                params,
                body: _,
            } = &stmt.kind
            {
                let mut param_types = Vec::new();
                for param in params {
                    if let Expression::Variable {
                        name: _,
                        is_mutable: _,
                    } = &param.kind
                    {
                        // Function parameters are evaluated as undeclared unless deeply typed later
                        param_types.push(Type::Undeclared);
                    }
                }
                checker
                    .function_registry
                    .insert(name.clone(), (param_types, Type::Undeclared));
            }
        }

        // Second Pass: Check statements sequentially
        for stmt in &program.body {
            checker.check_statement(stmt);
        }
    }

    pub fn check_statement(&mut self, stmt: &StatementNode) {
        match &stmt.kind {
            Statement::VarDeclaration {
                name,
                var_type,
                initializer,
                is_mutable: _,
            } => {
                let declared_type = Type::from_str(var_type);
                let inferred_type = self.infer_expression_type(initializer);

                // Enforce structural type compatibility
                if declared_type != Type::Undeclared
                    && inferred_type != Type::Nil
                    && declared_type != inferred_type
                {
                    self.error(
                        &format!(
                            "Mismatched types. Cannot initialize variable '{}' of type {:?} with value of type {:?}",
                            name, declared_type, inferred_type
                        ),
                        stmt.span,
                    );
                }

                // If no type was specified, let type inference save the day
                let final_type = if declared_type == Type::Undeclared {
                    inferred_type
                } else {
                    declared_type
                };
                self.declare_variable(name.clone(), final_type);
            }

            Statement::Assignment { name, value } => {
                let expected_type = self.resolve_variable_type(name).unwrap_or_else(|| {
                    self.error(
                        &format!("Variable '{}' assigned before declaration.", name),
                        stmt.span,
                    );
                });

                let assigned_type = self.infer_expression_type(value);

                if expected_type != Type::Undeclared
                    && assigned_type != Type::Nil
                    && expected_type != assigned_type
                {
                    self.error(
                        &format!(
                            "Type Mismatch: Cannot assign value of type {:?} to variable '{}' of type {:?}",
                            assigned_type, name, expected_type
                        ),
                        stmt.span,
                    );
                }
            }

            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.infer_expression_type(condition);
                if cond_type != Type::Boolean && cond_type != Type::Undeclared {
                    self.error(
                        "Condition inside 'if' statement must evaluate to a Boolean context.",
                        stmt.span,
                    );
                }
                self.check_statement(then_branch);
                if let Some(else_stmt) = else_branch {
                    self.check_statement(else_stmt);
                }
            }

            Statement::While { condition, body } => {
                let cond_type = self.infer_expression_type(condition);
                if cond_type != Type::Boolean && cond_type != Type::Undeclared {
                    self.error(
                        "Condition inside 'while' loop guard must evaluate to a Boolean context.",
                        stmt.span,
                    );
                }
                self.check_statement(body);
            }

            Statement::For {
                item_name,
                iterable,
                body,
            } => {
                let iter_type = self.infer_expression_type(iterable);
                let element_type = match iter_type {
                    Type::Array(inner) => *inner,
                    Type::String => Type::String, // Iterating strings yields characters
                    Type::Undeclared => Type::Undeclared,
                    _ => self.error("Invalid Collection: 'for' loops can only iterate through Array or String elements.", stmt.span),
                };

                self.push_scope();
                self.declare_variable(item_name.clone(), element_type);
                self.check_statement(body);
                self.pop_scope();
            }

            Statement::Block(statements) => {
                self.push_scope();
                for sub_stmt in statements {
                    self.check_statement(sub_stmt);
                }
                self.pop_scope();
            }

            Statement::FunctionDeclaration {
                name: _,
                params,
                body,
            } => {
                self.push_scope();
                // Inject parameters into local function context
                for param in params {
                    if let Expression::Variable {
                        name,
                        is_mutable: _,
                    } = &param.kind
                    {
                        self.declare_variable(name.clone(), Type::Undeclared);
                    }
                }
                for sub_stmt in body {
                    self.check_statement(sub_stmt);
                }
                self.pop_scope();
            }

            Statement::Match {
                target,
                cases,
                default_branch,
            } => {
                let target_type = self.infer_expression_type(target);
                for case in cases {
                    let pattern_type = self.infer_expression_type(&case.pattern);
                    if target_type != Type::Undeclared
                        && pattern_type != Type::Undeclared
                        && target_type != pattern_type
                    {
                        self.error(&format!("Match pattern type {:?} mismatches structural target evaluation type {:?}", pattern_type, target_type), case.pattern.span);
                    }
                    self.check_statement(&case.body);
                }
                if let Some(default_stmt) = default_branch {
                    self.check_statement(default_stmt);
                }
            }

            Statement::Expression(expr) => {
                self.infer_expression_type(expr);
            }

            Statement::Return(maybe_expr) => {
                if let Some(expr) = maybe_expr {
                    self.infer_expression_type(expr);
                }
            }
        }
    }

    pub fn infer_expression_type(&self, expr: &ExpressionNode) -> Type {
        match &expr.kind {
            Expression::Literal(lit) => match lit {
                LiteralValue::Number(_) => Type::Number,
                LiteralValue::String(_) => Type::String,
                LiteralValue::Boolean(_) => Type::Boolean,
                LiteralValue::Nil => Type::Nil,
            },

            Expression::Variable { name, .. } => {
                self.resolve_variable_type(name).unwrap_or(Type::Undeclared)
            }

            Expression::Binary {
                operator,
                left,
                right,
            } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);

                match operator {
                    BinaryOperator::Add => {
                        if left_type == Type::String || right_type == Type::String {
                            Type::String // Support string concatenation safely
                        } else {
                            Type::Number
                        }
                    }

                    BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div => {
                        if left_type != Type::Number && left_type != Type::Undeclared {
                            self.error("Mathematical equations require numeric terms.", left.span);
                        }
                        if right_type != Type::Number && right_type != Type::Undeclared {
                            self.error("Mathematical equations require numeric terms.", right.span);
                        }
                        Type::Number
                    }

                    BinaryOperator::Equal | BinaryOperator::NotEqual => Type::Boolean,
                    BinaryOperator::LessThan
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::LessEqual
                    | BinaryOperator::GreaterEqual => Type::Boolean,
                    BinaryOperator::And | BinaryOperator::Or => {
                        if left_type != Type::Boolean && left_type != Type::Undeclared {
                            self.error(
                                "Logical short-circuit operators demand boolean conditions.",
                                left.span,
                            );
                        }
                        Type::Boolean
                    }
                }
            }

            Expression::Unary { operator, right } => {
                let right_type = self.infer_expression_type(right);
                match operator {
                    UnaryOperator::Minus => {
                        if right_type != Type::Number && right_type != Type::Undeclared {
                            self.error(
                                "Arithmetic numeric negation (-) demands a number constraint.",
                                right.span,
                            );
                        }
                        Type::Number
                    }

                    UnaryOperator::Not => {
                        if right_type != Type::Boolean && right_type != Type::Undeclared {
                            self.error(
                                "Logical inversion (!) demands a valid boolean evaluation.",
                                right.span,
                            );
                        }
                        Type::Boolean
                    }
                }
            }

            Expression::Array(elements) => {
                if let Some(first) = elements.first() {
                    let inferred_element_type = self.infer_expression_type(first);
                    Type::Array(Box::new(inferred_element_type))
                } else {
                    Type::Array(Box::new(Type::Undeclared)) // Empty array template allows fluid filling
                }
            }

            Expression::Call {
                callee,
                arguments: _,
            } => {
                // Method validation (e.g. x.get(0))
                if let Expression::Get { object, name } = &callee.kind {
                    let obj_type = self.infer_expression_type(object);

                    match name.as_str() {
                        "append" | "clear" | "remove" | "pop" => {
                            if let Type::Array(_) = obj_type {} else if obj_type != Type::Undeclared {
                                self.error("Method Error: Mutation arrays operations can only target Array profiles.", expr.span);
                            }
                            Type::Nil
                        }

                        "get" => {
                            match obj_type {
                                Type::Array(inner) => *inner,
                                Type::String => Type::String,
                                Type::Undeclared => Type::Undeclared,
                                _ => self.error("Method Error: .get() requires an active Array or String context layout.", expr.span),
                            }
                        }

                        "expect" => obj_type, // Passes structural tracking through
                        "is_numeric" | "contains" | "starts_with" | "ends_with" => Type::Boolean,
                        "trim" => Type::String,
                        "join" => Type::String,
                        _ => Type::Undeclared,
                    }
                } else if let Expression::Variable { name, .. } = &callee.kind {
                    // Global functions validation
                    if let Some((_param_types, return_type)) = self.function_registry.get(name) {
                        return_type.clone()
                    } else {
                        Type::Undeclared
                    }
                } else {
                    Type::Undeclared
                }
            }
            Expression::Get { .. } => Type::Undeclared,
        }
    }
}
