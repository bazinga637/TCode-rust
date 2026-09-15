
#[derive(Debug, Clone)]
pub enum Statement {
    // let x = 5;
    VarDeclaration {
        name: String,
        var_type: String,
        initializer: ExpressionNode,
        is_mutable: bool,
    },
    
    // x = 10;
    Assignment {
        name: String,
        value: ExpressionNode,
    },
    
    // if (cond) { then } else { else }
    If {
        condition: ExpressionNode,
        then_branch: Box<StatementNode>,
        else_branch: Option<Box<StatementNode>>, // Optional 'else'
    },
    
    // while (cond) { body }
    While {
        condition: ExpressionNode,
        body: Box<StatementNode>,
    },
    
    // fn name(a, b) { body }
    FunctionDeclaration {
        name: String,
        params: Vec<ExpressionNode>,
        body: Vec<StatementNode>,
    },
    
    // return 5;
    Return(Option<ExpressionNode>),
    
    // { stmt1; stmt2; }
    Block(Vec<StatementNode>),
    
    // A standalone expression acting as a statement (e.g., calling print(x);)
    Expression(ExpressionNode),
}

#[derive(Debug, Clone)]
pub enum Expression {
    // A raw value like 5, "hello", or true
    Literal(LiteralValue),
    
    // Accessing a variable by name (e.g., x)
    Variable {
        name: String,
        is_mutable: bool,
    },
    
     // 🌟 Clean property tracking (e.g., standard_list . append)
    Get {
        object: Box<ExpressionNode>,
        name: String,
    },

    Array(Vec<ExpressionNode>),

    // Binary operations (e.g., a + b, x < 10)
    Binary {
        operator: BinaryOperator,
        left: Box<ExpressionNode>,
        right: Box<ExpressionNode>,
    },
    
    // Unary operations (e.g., -x, !flag)
    Unary {
        operator: UnaryOperator,
        right: Box<ExpressionNode>,
    },
    
    // Calling a function (e.g., add(1, 2))
    Call {
        callee: Box<ExpressionNode>,
        arguments: Vec<ExpressionNode>,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil, // Represents null/none
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div,               // Math
    Equal, NotEqual, LessThan, GreaterThan, // Comparison
    And, Or,                          // Logic
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Minus, // Negation: -x
    Not,   // Logical NOT: !x
}

#[derive(Debug, Clone)]
pub struct ExpressionNode {
    pub span: Span,   // Line and column metadata
    pub kind: Expression,   // The actual enum (Literal, Binary, Call, etc.)
}

// 2. The wrapper for all statements
#[derive(Debug, Clone)]
pub struct StatementNode {
    pub span: Span,   // Line and column metadata
    pub kind: Statement,   // The actual enum (VarDeclaration, If, While, etc.)
}

#[derive(Debug)]
pub struct Program {
    pub body: Vec<StatementNode>, // A flat list of top-level statements
}

use crate::lexer::Token;
use crate::lexer::Span;


pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            index: 0,
        }
    }

    // --- Core Navigation Methods ---
    fn peek(&self) -> Option<&str> { 
        self.tokens.get(self.index).map(|t| t.value.as_str()) 
    }

    // Grab the span of the next token to associate with your AST node
    fn peek_span(&self) -> Span {
        self.tokens.get(self.index)
            .map(|t| t.span)
            .unwrap_or_else(|| {
                // Fallback for EOF: use the span of the very last token if available
                self.tokens.last().map(|t| t.span).unwrap_or(Span { line: 1, column: 1 })
            })
    }
    
    // advances forward by one token if it can
    fn advance(&mut self) -> Option<Token> {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            Some(token)
        } else { None }
    }

    // checks if next token matches 'expected' &str
    fn match_token(&mut self, expected: &str) -> bool {
        if self.peek() == Some(expected) { 
            self.advance(); 
            return true; 
        }
        false
    }

    // parses statement nodes until done then returns the program AST
    pub fn parse_program(&mut self) -> Program {
        let mut body = Vec::new();
        while self.peek().is_some() {
            body.push(self.parse_statement());
        }
        Program { body }
    }

    pub fn parse_statement(&mut self) -> StatementNode {
        if let Some(token) = self.peek() {
            match token {
                "let" => self.parse_var_declaration(),
                "if" => self.parse_if_statement(),
                "while" => self.parse_while_loop(),
                "fn" => self.parse_function_declaration(),
                "return" => self.parse_return_statement(),
                "{" => self.parse_block_statement(),
                _ => self.parse_expression_statement(),
            }
        } else {
            panic!("Unexpected end of input while looking for a statement");
        }
    }

    // parses variable declarations ( let variable = value )
    fn parse_var_declaration(&mut self) -> StatementNode {
        let start_span: Span = self.peek_span(); // Grab the exact position of "let"
        self.advance(); // consume "let"
        
        // assumed not mutable by default, is changed to true if 'mut' keyword is present after 'let'
        let mut is_mutable: bool = false;

        if self.peek() == Some("mut") {
            is_mutable = true;
            self.advance(); 
        }

        // gets variable name
        let name_token: Token = self.advance().expect("Expected variable name after 'let'");
        let name: String = name_token.value;

        // assumes variable type as 'Undeclared' unless typed after ':' ( let variable: type = value)
        let mut var_type: String = "Undeclared".to_string();

        if self.peek() == Some(":") {
            self.advance();
            var_type = self.advance().expect("Expected type after ':' in variable declaration").value;
        }
        
        assert!(self.match_token("="), "Expected '=' after variable name");
        let initializer = self.parse();

        StatementNode {
            span: start_span, // Assign the dynamic starting position
            kind: Statement::VarDeclaration { name, var_type, initializer, is_mutable },
        }
    }


    // Parses: if ( condition ) { body }
    fn parse_if_statement(&mut self) -> StatementNode {
        let start_span = self.peek_span();

        self.advance(); // consume "if"
        assert!(self.match_token("("), "Expected '(' after 'if'");

        let condition = self.parse();
        assert!(self.match_token(")"), "Expected ')' after if condition");
        
        let then_branch = Box::new(self.parse_statement());
        let mut else_branch = None;
        if self.match_token("else") {
            else_branch = Some(Box::new(self.parse_statement()));
        }

        StatementNode {
            span: start_span,
            kind: Statement::If { condition, then_branch, else_branch },
        }
    }

    // Parses: while ( x ) { ... }
    fn parse_while_loop(&mut self) -> StatementNode {
        let start_span = self.peek_span();
        self.advance(); // consume "while"

        assert!(self.match_token("("), "Expected '(' after 'while'");
        let condition = self.parse();

        assert!(self.match_token(")"), "Expected ')' after while condition");
        let body = Box::new(self.parse_statement());

        StatementNode {
            span: start_span,
            kind: Statement::While { condition, body },
        }
    }


    // Parses: fn my_func ( mut a : Type , b ) { ... }
    fn parse_function_declaration(&mut self) -> StatementNode {
        let start_span: Span = self.peek_span();
        self.advance(); // consume "fn"

        let name: String = self.advance().expect("Expected function name after 'fn'").value;
        assert!(self.match_token("("), "Expected '(' after function name");

        let mut params: Vec<ExpressionNode> = Vec::new();

        if self.peek().map(|s| s) != Some(")") {
            loop {
                
                let mut is_mutable: bool = false;

                if self.peek().map(|s| s) == Some("mut") {
                    is_mutable = true;
                    self.advance(); 
                }
                
                let param_name: String = self.advance().expect("Expected parameter name").value;
                

                // 3. Skip type annotations if a colon ':' is present
                if self.match_token(":") {
                    // Keep consuming tokens until we see a ',' or ')'
                    while let Some(token) = self.peek() {
                        if token == "," || token == ")" {
                            break;
                        }
                        self.advance(); // consume type tokens like "List[int]"
                    }
                
                let param: ExpressionNode = ExpressionNode{ span: start_span, kind: Expression::Variable{ name: param_name, is_mutable } };
                params.push(param);

                // break if there's no comma separating the next parameter
                if !self.match_token(",") { break; }
                }
            }
        }

        assert!(self.match_token(")"), "Expected ')' after parameter list");
        
        let body_stmt = self.parse_statement();
        let body = match body_stmt.kind {
            Statement::Block(statements) => statements,
            _ => unreachable!(),
        };
        
        StatementNode {
            span: start_span,
            kind: Statement::FunctionDeclaration { name, params, body },
        }
        
    }
        
        
    // Parses: return 5  or  return
    fn parse_return_statement(&mut self) -> StatementNode {
        let start_span = self.peek_span();
        self.advance(); // consume "return"
        
        let mut value = None;
        // Check if the next token belongs to a different statement or block closer
        if let Some(tok) = self.peek() {
            if tok != "}" && tok != "return" && tok != "let" && tok != "if" && tok != "while" {
                value = Some(self.parse());
            }
        }

        StatementNode {
            span: start_span,
            kind: Statement::Return(value),
        }
    }

    // Parses: { stmt1 stmt2 }
    fn parse_block_statement(&mut self) -> StatementNode {
        let start_span = self.peek_span();
        self.advance(); // consume "{"

        let mut statements = Vec::new();
        while self.peek().map(|s| s) != Some("}") {
            statements.push(self.parse_statement());
        }

        self.advance(); // consume "}"

        StatementNode {
            span: start_span,
            kind: Statement::Block(statements),
        }
    }

    // Fallback: x = 10  or  print(x)
    fn parse_expression_statement(&mut self) -> StatementNode {
        let start_span = self.peek_span();

        let expr = self.parse();

        if let Expression::Variable{name, is_mutable: _} = &expr.kind {
            if self.match_token("=") {
                let value = self.parse();
                return StatementNode {
                    span: start_span,
                    kind: Statement::Assignment { name: name.clone(), value },
                };
            }
        }
        StatementNode {
            span: start_span,
            kind: Statement::Expression(expr),
        }
    }

    // --- Expression Processing Rules ---
    pub fn parse(&mut self) -> ExpressionNode { self.parse_additive() }

    fn parse_additive(&mut self) -> ExpressionNode {
        let mut left = self.parse_multiplicative();
        while let Some(token) = self.peek() {
            let operator = match token {
                "+" => BinaryOperator::Add,
                "-" => BinaryOperator::Sub,
                _ => break,
            };

            let start_span = left.span;
            self.advance();
            let right = self.parse_multiplicative();
            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary { operator, left: Box::new(left), right: Box::new(right) },
            };
        }
        left
    }

        fn parse_multiplicative(&mut self) -> ExpressionNode {
        // Route through our call & property loop interceptor
        let mut left = self.parse_call_or_member();
        while let Some(token) = self.peek() {
            let operator = match token {
                "*" => BinaryOperator::Mul,
                "/" => BinaryOperator::Div,
                _ => break,
            };
            let start_span = left.span;
            self.advance();
            let right = self.parse_call_or_member();
            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary { operator, left: Box::new(left), right: Box::new(right) },
            };
        }
        left
    }

    fn parse_call_or_member(&mut self) -> ExpressionNode {
        let mut expr = self.parse_primary();

        loop {
            let start_span = expr.span;
            if self.match_token("(") {
                // Parse call arguments
                let mut arguments = Vec::new();
                if self.peek().map(|s| s) != Some(")") {
                    loop {
                        arguments.push(self.parse());
                        if !self.match_token(",") { break; }
                    }
                }
                assert!(self.match_token(")"), "Expected ')' after arguments");
                
                expr = ExpressionNode {
                    span: start_span,
                    kind: Expression::Call {
                        callee: Box::new(expr),
                        arguments,
                    },
                };
            } else if self.match_token(".") {
                // Pure member lookup! No more string concatenation hacks.
                let property = self.advance().expect("Expected field or method name after '.'").value;
                expr = ExpressionNode {
                    span: start_span,
                    kind: Expression::Get {
                        object: Box::new(expr),
                        name: property,
                    },
                };
            } else {
                break;
            }
        }

        expr
    }


    fn parse_primary(&mut self) -> ExpressionNode {
        let start_span = self.peek_span(); // Grab the position before consuming tokens
        
        let mut is_mutable = false;
        if self.peek() == Some("mut") {
            is_mutable = true;
            self.advance();
        }

        let token_obj = self.advance().expect("Unexpected end of expression");
        let token = token_obj.value;

        if token == "(" {
            let sub_expr = self.parse();
            assert!(self.match_token(")"), "Expected closing parenthesis ')'");
            return sub_expr; // Keep sub-expression's internal span
        }

        if token == "[" {
            assert!(!is_mutable, "Cannot apply 'mut' modifier to an array literal initialization");
            let mut elements = Vec::new();
            if self.peek() != Some("]") {
                loop {
                    elements.push(self.parse());
                    if !self.match_token(",") { break; }
                }
            }
            assert!(self.match_token("]"), "Expected closing bracket ']' after array elements");
            return ExpressionNode {
                span: start_span,
                kind: Expression::Array(elements),
            };
        }

        if let Ok(number) = token.parse::<f64>() {
            return ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::Number(number)),
            };
        }

        ExpressionNode {
            span: start_span,
            kind: Expression::Variable { name: token, is_mutable },
        }
    }
}
