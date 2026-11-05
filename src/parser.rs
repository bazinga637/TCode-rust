
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

    For {
        item_name: String,
        iterable: ExpressionNode,
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

    Match {
        target: ExpressionNode,
        cases: Vec<MatchCase>,
        default_branch: Option<Box<StatementNode>>,
    },
}

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: ExpressionNode, // The value to compare against (e.g. "what", 5, true)
    pub body: Box<StatementNode>, // The statement or block code execution path
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
    
     // Clean property tracking (e.g., standard_list . append)
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
    Equal, NotEqual, LessThan, GreaterThan, LessEqual, GreaterEqual, // Comparison
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
                "match" => self.parse_match_statement(),
                "while" => self.parse_while_loop(),
                "for" => self.parse_for_loop(),
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
            self.advance(); // consume ":"
            let mut type_tokens = Vec::new();
            
            // Consume tokens until we find the assignment operator '='
            while let Some(tok) = self.peek() {
                if tok == "=" { break; }
                type_tokens.push(self.advance().unwrap().value);
            }
            var_type = type_tokens.join(""); // Merges "List", "[", "int", "]" into "List[int]"
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

        let condition = self.parse();

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

    fn parse_match_statement(&mut self) -> StatementNode {
        let start_span = self.peek_span();
        self.advance(); // consume "match"

        let target = self.parse(); // Parse the evaluation expression target

        let mut cases = Vec::new();
        let mut default_branch = None;

        while self.peek() != Some("}") {
            // Check for the wildcard catch-all pattern "_"
            if self.peek() == Some("_") {
                self.advance(); // consume "_"
                assert!(self.match_token("=>"), "Expected '=>' thick arrow after fallback pattern '_'");
                
                default_branch = Some(Box::new(self.parse_statement()));
                continue;
            }

            // Parse normal value patterns (e.g., "what", 5, or true)
            let pattern = self.parse();
            assert!(self.match_token("=>"), "Expected '=>' thick arrow after match pattern condition");
            
            let body = if self.peek() == Some("{") {
                Box::new(self.parse_block_statement())
            } else {
                Box::new(self.parse_statement())
            };

            cases.push(MatchCase { pattern, body });
        }

        assert!(self.match_token("}"), "Expected '}}' to close match block execution frame");

        StatementNode {
            span: start_span,
            kind: Statement::Match { target, cases, default_branch },
        }
    }

    // Parses: while ( x ) { ... }
    fn parse_while_loop(&mut self) -> StatementNode {
        let start_span = self.peek_span();
        self.advance(); // consume "while"

        let condition = self.parse();

        let body = Box::new(self.parse_statement());

        StatementNode {
            span: start_span,
            kind: Statement::While { condition, body },
        }
    }

    fn parse_for_loop(&mut self) -> StatementNode {
        let start_span = self.peek_span();
        self.advance(); // Consume "for"

        // 1. Capture the iteration variable name
        let item_token = self.advance().expect("Expected variable name after 'for'");
        let item_name = item_token.value;

        // 2. Ensure "in" keyword is present
        assert!(self.match_token("in"), "Expected 'in' keyword after variable name in for loop");

        // 3. Parse the array collection/iterable expression
        let iterable = self.parse();

        // 4. Capture the executable body statements block
        let body = Box::new(self.parse_statement());

        StatementNode {
            span: start_span,
            kind: Statement::For { item_name, iterable, body },
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
                }
            let param: ExpressionNode = ExpressionNode{ span: start_span, kind: Expression::Variable{ name: param_name, is_mutable } };
            params.push(param);

            // break if there's no comma separating the next parameter
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
    // first
    pub fn parse(&mut self) -> ExpressionNode { 
        self.parse_logical_or() 
    }
    
    // second
    fn parse_logical_or(&mut self) -> ExpressionNode {
        let mut left = self.parse_logical_and();

        while self.peek() == Some("||") {
            let start_span = left.span;
            self.advance(); // consume "||"
            let right = self.parse_logical_and();
        
            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator: BinaryOperator::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        left
    }

    // third
    fn parse_logical_and(&mut self) -> ExpressionNode {
        let mut left = self.parse_comparison();

        while self.peek() == Some("&&") {
            let start_span = left.span;
            self.advance(); // consume "&&"
            let right = self.parse_comparison();
        
            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator: BinaryOperator::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        left
    }

    // fourth
    fn parse_comparison(&mut self) -> ExpressionNode {
        let mut left = self.parse_additive();

        while let Some(token) = self.peek() {
            let operator = match token {
                "==" => BinaryOperator::Equal,
                "!=" => BinaryOperator::NotEqual,
                "<" => BinaryOperator::LessThan,
                ">" => BinaryOperator::GreaterThan,
                "<=" => BinaryOperator::LessEqual,
                ">=" => BinaryOperator::GreaterEqual,
                _ => break,
            };

            let start_span = left.span;
            self.advance(); // Consume the comparison operator token
            let right = self.parse_additive();

            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        left
    }

    // Intercepts preceding '-' or '!' symbols before diving into literals
    // seventh
    fn parse_unary(&mut self) -> ExpressionNode {
        if let Some(token) = self.peek() {
            let operator = match token {
                "-" => Some(UnaryOperator::Minus),
                "!" => Some(UnaryOperator::Not),
                _ => None,
            };

            if let Some(op) = operator {
                let start_span = self.peek_span();
                self.advance(); // Consume the operator symbol token
                
                // Recursively call parse_unary to correctly support chained elements like !!true
                let right = self.parse_unary(); 

                return ExpressionNode {
                    span: start_span,
                    kind: Expression::Unary {
                        operator: op,
                        right: Box::new(right),
                    },
                };
            }
        }
        // Fallback naturally to raw numbers/variables if no prefix matches
        self.parse_primary()
    }
    
    // fifth
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

    // sixth
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
        let mut expr = self.parse_unary();

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
        // will parse strings as literals ("a", 'a')
        if (token.starts_with('"') && token.ends_with('"')) || (token.starts_with('\'') && token.ends_with('\'')) {
            assert!(!is_mutable, "Cannot apply 'mut' modifier to a string literal");
            
            // Extract the inside text by stripping out the bounding quotes
            let inner_value = token[1..token.len() - 1].to_string();
            
            return ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::String(inner_value)),
            };
        }

        if token == "true" {
            assert!(!is_mutable, "Cannot apply 'mut' modifier to a boolean literal");
            return ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::Boolean(true)),
            };
        }

        if token == "false" {
            assert!(!is_mutable, "Cannot apply 'mut' modifier to a boolean literal");
            return ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::Boolean(false)),
            };
        }
        
        if token == "nil" || token == "null" {
            assert!(!is_mutable, "Cannot apply 'mut' modifier to a nil literal");
            return ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::Nil),
            };
        }

        ExpressionNode {
            span: start_span,
            kind: Expression::Variable { name: token, is_mutable },
        }
    }
}
