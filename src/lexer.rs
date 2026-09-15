#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,   
    pub column: usize, 
}

#[derive(Debug, Clone)]
pub struct Token {
    pub value: String,
    pub span: Span,
}

pub struct Lexer {
    chars: Vec<char>, 
    index: usize,     
    line: usize,      
    column: usize,    
}

impl Lexer {
    pub fn new(source_code: String) -> Self {
        Lexer {
            chars: source_code.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    // Look at the character under the cursor without consuming it
    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    // Look two steps ahead (helpful to find "//" comments)
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    // Step forward and safely update coordinates
    fn advance(&mut self) -> Option<char> {
        let current_char = self.peek()?;
        self.index += 1;

        if current_char == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(current_char)
    }

    // Entrypoint: Generates a vector of rich tokens
    pub fn lex(mut self) -> Vec<Token> {
        let symbols = vec!['(', ')', '{', '}', '\n', '*', '+', '-', '=', '|', ':', '.', ',', '[', ']'];
        let ignored_chars = vec![' ', '\t', '\r'];

        let mut tokens: Vec<Token> = vec![];
        let mut current_str = String::new();
        let mut word_start_span = Span { line: 1, column: 1 };

        while let Some(c) = self.peek() {
            
            // If we hit a double slash '//', consume until the end of the line
            if c == '/' && self.peek_next() == Some('/') {
    
                self.advance(); // consume both slashes
                self.advance();
                
                // Skip everything up to the newline
                while let Some(comment_char) = self.peek() {
                    if comment_char == '\n' { break; }
                    self.advance();
                }
                continue;
            }

            // --- 2. Handle Independent Slash ---
            if c == '/' {
                let slash_span = Span { line: self.line, column: self.column };
                self.advance();
                tokens.push(Token { value: "/".to_string(), span: slash_span });
                continue;
            }

            // --- 3. Handle Ignored Whitespace ---
            if ignored_chars.contains(&c) {
                // If we were building a word/number, flush it before skipping space
                if !current_str.is_empty() {
                    tokens.push(Token { value: current_str.clone(), span: word_start_span });
                    current_str.clear();
                }
                self.advance();
                continue;
            }

            // --- 4. Handle Dedicated Symbols ---
            if symbols.contains(&c) {
                // Flush accumulated word buffer first
                if !current_str.is_empty() {
                    tokens.push(Token { value: current_str.clone(), span: word_start_span });
                    current_str.clear();
                }

                // Snap the exact position of this single-character symbol
                let symbol_span = Span { line: self.line, column: self.column };
                let sym = self.advance().unwrap();

                // If you don't want newlines inside your final parser array, skip pushing it
                if sym != '\n' {
                    tokens.push(Token { value: sym.to_string(), span: symbol_span });
                }
                continue;
            }

            // --- 5. Accumulate Identifiers, Keywords, and Literals ---
            if current_str.is_empty() {
                // Mark the starting block coordinate of this multi-character token
                word_start_span = Span { line: self.line, column: self.column };
            }

            current_str.push(c);
            self.advance();

            // Check if keywords are split directly without spacing (e.g. `letx` should be an identifier, but your code checked exact string matches)
            // To emulate your exact custom keyword-flushing strategy safely:
            if current_str == "fn" || current_str == "let" || current_str == "mut" {
                // Look ahead to check if the keyword is ending or part of a bigger word like "let_variable"
                if let Some(next) = self.peek() {
                    if ignored_chars.contains(&next) || symbols.contains(&next) || next == '/' {
                        tokens.push(Token { value: current_str.clone(), span: word_start_span });
                        current_str.clear();
                    }
                }
            }
        }

        // Catch lingering values remaining in the buffers at the end of the text file
        if !current_str.is_empty() {
            tokens.push(Token { value: current_str, span: word_start_span });
        }

        tokens
    }
}
