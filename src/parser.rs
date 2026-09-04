
#[derive(Debug, Clone)]
pub enum Statement {
    // let x = 5;
    VarDeclaration {
        name: String,
        initializer: ExpressionNode,
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
        params: Vec<String>,
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
    Variable(String),
    
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,   // The row number (usually 1-indexed for human readability)
    pub column: usize, // The character position in that row
}

#[derive(Debug)]
pub struct Program {
    pub body: Vec<StatementNode>, // A flat list of top-level statements
}

pub struct Parser {
    tokens: Vec<String>,
    index: usize,
    // For a real language, track line/col dynamically. 
    // We'll use a dummy Span placeholder here to keep the parsing logic crisp.
    current_span: Span, 
}

impl Parser {
    pub fn new(tokens: Vec<String>) -> Self {
        Parser {
            tokens,
            index: 0,
            current_span: Span { line: 1, column: 1 },
        }
    }

    // --- Core Navigation Methods ---
    fn peek(&self) -> Option<&String> { self.tokens.get(self.index) }
    
    fn advance(&mut self) -> Option<String> {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            Some(token)
        } else { None }
    }

    fn match_token(&mut self, expected: &str) -> bool {
        if let Some(tok) = self.peek() {
            if tok == expected { self.advance(); return true; }
        }
        false
    }

    // --- Statement Processing Rules (Completely Clutter-Free!) ---

    pub fn parse_program(&mut self) -> Program {
        let mut body = Vec::new();
        while self.peek().is_some() {
            body.push(self.parse_statement());
        }
        Program { body }
    }

    pub fn parse_statement(&mut self) -> StatementNode {
        if let Some(token) = self.peek() {
            match token.as_str() {
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

    // Parses: let x = 5
    fn parse_var_declaration(&mut self) -> StatementNode {
        self.advance(); // consume "let"
        let name = self.advance().expect("Expected variable name after 'let'");
        assert!(self.match_token("="), "Expected '=' after variable name");
        let initializer = self.parse(); // Automatically knows it's done when expressions run out!

        StatementNode {
            span: self.current_span,
            kind: Statement::VarDeclaration { name, initializer },
        }
    }

    // Parses: if ( x ) { ... }
    fn parse_if_statement(&mut self) -> StatementNode {
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
            span: self.current_span,
            kind: Statement::If { condition, then_branch, else_branch },
        }
    }

    // Parses: while ( x ) { ... }
    fn parse_while_loop(&mut self) -> StatementNode {
        self.advance(); // consume "while"
        assert!(self.match_token("("), "Expected '(' after 'while'");
        let condition = self.parse();
        assert!(self.match_token(")"), "Expected ')' after while condition");
        let body = Box::new(self.parse_statement());

        StatementNode {
            span: self.current_span,
            kind: Statement::While { condition, body },
        }
    }

    // Parses: fn my_func ( a , b ) { ... }
    fn parse_function_declaration(&mut self) -> StatementNode {
        self.advance(); // consume "fn"
        let name = self.advance().expect("Expected function name after 'fn'");
        assert!(self.match_token("("), "Expected '(' after function name");
        
        let mut params = Vec::new();
        if self.peek().map(|s| s.as_str()) != Some(")") {
            loop {
                let param = self.advance().expect("Expected parameter name");
                params.push(param);
                if !self.match_token(",") { break; }
            }
        }
        assert!(self.match_token(")"), "Expected ')' after parameter list");
        
        let body_stmt = self.parse_statement();
        let body = match body_stmt.kind {
            Statement::Block(statements) => statements,
            _ => unreachable!(),
        };

        StatementNode {
            span: self.current_span,
            kind: Statement::FunctionDeclaration { name, params, body },
        }
    }

    // Parses: return 5  or  return
    fn parse_return_statement(&mut self) -> StatementNode {
        self.advance(); // consume "return"
        
        let mut value = None;
        // Check if the next token belongs to a different statement or block closer
        if let Some(tok) = self.peek() {
            if tok != "}" && tok != "return" && tok != "let" && tok != "if" && tok != "while" {
                value = Some(self.parse());
            }
        }

        StatementNode {
            span: self.current_span,
            kind: Statement::Return(value),
        }
    }

    // Parses: { stmt1 stmt2 }
    fn parse_block_statement(&mut self) -> StatementNode {
        self.advance(); // consume "{"
        let mut statements = Vec::new();
        while self.peek().map(|s| s.as_str()) != Some("}") {
            statements.push(self.parse_statement());
        }
        self.advance(); // consume "}"

        StatementNode {
            span: self.current_span,
            kind: Statement::Block(statements),
        }
    }

    // Fallback: x = 10  or  print(x)
    fn parse_expression_statement(&mut self) -> StatementNode {
        let expr = self.parse();
        if let Expression::Variable(name) = &expr.kind {
            if self.match_token("=") {
                let value = self.parse();
                return StatementNode {
                    span: self.current_span,
                    kind: Statement::Assignment { name: name.clone(), value },
                };
            }
        }
        StatementNode {
            span: self.current_span,
            kind: Statement::Expression(expr),
        }
    }

    // --- Expression Processing Rules ---
    pub fn parse(&mut self) -> ExpressionNode { self.parse_additive() }

    fn parse_additive(&mut self) -> ExpressionNode {
        let mut left = self.parse_multiplicative();
        while let Some(tok) = self.peek() {
            let operator = match tok.as_str() {
                "+" => BinaryOperator::Add,
                "-" => BinaryOperator::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative();
            left = ExpressionNode {
                span: self.current_span,
                kind: Expression::Binary { operator, left: Box::new(left), right: Box::new(right) },
            };
        }
        left
    }

    fn parse_multiplicative(&mut self) -> ExpressionNode {
        let mut left = self.parse_primary();
        while let Some(tok) = self.peek() {
            let operator = match tok.as_str() {
                "*" => BinaryOperator::Mul,
                "/" => BinaryOperator::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary();
            left = ExpressionNode {
                span: self.current_span,
                kind: Expression::Binary { operator, left: Box::new(left), right: Box::new(right) },
            };
        }
        left
    }

    fn parse_primary(&mut self) -> ExpressionNode {
        let token = self.advance().expect("Unexpected end of expression");

        if token == "(" {
            let sub_expr = self.parse();
            assert!(self.match_token(")"), "Expected closing parenthesis ')'");
            return sub_expr;
        }

        if let Ok(number) = token.parse::<f64>() {
            return ExpressionNode {
                span: self.current_span,
                kind: Expression::Literal(LiteralValue::Number(number)),
            };
        }

        ExpressionNode {
            span: self.current_span,
            kind: Expression::Variable(token),
        }
    }
}
