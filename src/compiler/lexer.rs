use std::fmt;

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lex error at line {}, column {}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for LexError {}

type LexResult<T> = Result<T, LexError>;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Number(u64),
    Identifier(String),
    String(String),
    
    // Keywords
    Let,
    Const,
    Function,
    If,
    Else,
    While,
    For,
    Return,
    True,
    False,
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    AmpersandAmpersand,
    PipePipe,
    
    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Comma,
    Dot,
    
    // Built-ins
    Storage,
    Memory,
    Keccak256,
    Assert,
    
    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, line: usize, column: usize) -> Self {
        Self {
            token_type,
            lexeme,
            line,
            column,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} '{}' at {}:{}", self.token_type, self.lexeme, self.line, self.column)
    }
}

pub struct Lexer {
    input: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
        }
    }
    
    fn error(&self, message: &str) -> LexError {
        LexError {
            message: message.to_string(),
            line: self.line,
            column: self.column,
        }
    }
    
    pub fn tokenize(&mut self) -> LexResult<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while !self.is_at_end() {
            self.skip_whitespace();
            
            if self.is_at_end() {
                break;
            }
            
            let start_line = self.line;
            let start_column = self.column;
            
            match self.advance() {
                '+' => tokens.push(Token::new(TokenType::Plus, "+".to_string(), start_line, start_column)),
                '-' => tokens.push(Token::new(TokenType::Minus, "-".to_string(), start_line, start_column)),
                '*' => tokens.push(Token::new(TokenType::Star, "*".to_string(), start_line, start_column)),
                '/' => {
                    if self.match_char('/') {
                        // Line comment - skip until end of line
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        tokens.push(Token::new(TokenType::Slash, "/".to_string(), start_line, start_column));
                    }
                },
                '%' => tokens.push(Token::new(TokenType::Percent, "%".to_string(), start_line, start_column)),
                '(' => tokens.push(Token::new(TokenType::LeftParen, "(".to_string(), start_line, start_column)),
                ')' => tokens.push(Token::new(TokenType::RightParen, ")".to_string(), start_line, start_column)),
                '{' => tokens.push(Token::new(TokenType::LeftBrace, "{".to_string(), start_line, start_column)),
                '}' => tokens.push(Token::new(TokenType::RightBrace, "}".to_string(), start_line, start_column)),
                '[' => tokens.push(Token::new(TokenType::LeftBracket, "[".to_string(), start_line, start_column)),
                ']' => tokens.push(Token::new(TokenType::RightBracket, "]".to_string(), start_line, start_column)),
                ';' => tokens.push(Token::new(TokenType::Semicolon, ";".to_string(), start_line, start_column)),
                ',' => tokens.push(Token::new(TokenType::Comma, ",".to_string(), start_line, start_column)),
                '.' => tokens.push(Token::new(TokenType::Dot, ".".to_string(), start_line, start_column)),
                
                '=' => {
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenType::EqualEqual, "==".to_string(), start_line, start_column));
                    } else {
                        tokens.push(Token::new(TokenType::Equal, "=".to_string(), start_line, start_column));
                    }
                },
                
                '!' => {
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenType::BangEqual, "!=".to_string(), start_line, start_column));
                    } else {
                        tokens.push(Token::new(TokenType::Bang, "!".to_string(), start_line, start_column));
                    }
                },
                
                '>' => {
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenType::GreaterEqual, ">=".to_string(), start_line, start_column));
                    } else {
                        tokens.push(Token::new(TokenType::Greater, ">".to_string(), start_line, start_column));
                    }
                },
                
                '<' => {
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenType::LessEqual, "<=".to_string(), start_line, start_column));
                    } else {
                        tokens.push(Token::new(TokenType::Less, "<".to_string(), start_line, start_column));
                    }
                },
                
                '&' => {
                    if self.match_char('&') {
                        tokens.push(Token::new(TokenType::AmpersandAmpersand, "&&".to_string(), start_line, start_column));
                    } else {
                        return Err(LexError {
                            message: "Unexpected character '&'".to_string(),
                            line: start_line,
                            column: start_column,
                        });
                    }
                },
                
                '|' => {
                    if self.match_char('|') {
                        tokens.push(Token::new(TokenType::PipePipe, "||".to_string(), start_line, start_column));
                    } else {
                        return Err(LexError {
                            message: "Unexpected character '|'".to_string(),
                            line: start_line,
                            column: start_column,
                        });
                    }
                },
                
                c if c.is_ascii_digit() => {
                    let token = self.number(c, start_line, start_column)?;
                    tokens.push(token);
                },
                
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let token = self.identifier(c, start_line, start_column);
                    tokens.push(token);
                },
                
                '"' => {
                    let token = self.string_literal(start_line, start_column)?;
                    tokens.push(token);
                },
                
                c => {
                    return Err(LexError {
                        message: format!("Unexpected character '{}'", c),
                        line: start_line,
                        column: start_column,
                    });
                }
            }
        }
        
        tokens.push(Token::new(TokenType::Eof, "".to_string(), self.line, self.column));
        Ok(tokens)
    }
    
    fn number(&mut self, first_digit: char, line: usize, column: usize) -> LexResult<Token> {
        let mut value = String::new();
        value.push(first_digit);
        
        // Handle hex literals
        if first_digit == '0' && (self.peek() == 'x' || self.peek() == 'X') {
            self.advance(); // consume 'x'
            value.push('x');
            
            while self.peek().is_ascii_hexdigit() {
                value.push(self.advance());
            }
            
            if value.len() == 2 { // Just "0x"
                return Err(LexError {
                    message: "Invalid hex literal".to_string(),
                    line,
                    column,
                });
            }
            
            let hex_str = &value[2..]; // Remove "0x"
            match u64::from_str_radix(hex_str, 16) {
                Ok(num) => Ok(Token::new(TokenType::Number(num), value, line, column)),
                Err(_) => Err(LexError {
                    message: format!("Invalid hex number '{}'", value),
                    line,
                    column,
                }),
            }
        } else {
            // Regular decimal number
            while self.peek().is_ascii_digit() {
                value.push(self.advance());
            }
            
            match value.parse::<u64>() {
                Ok(num) => Ok(Token::new(TokenType::Number(num), value, line, column)),
                Err(_) => Err(LexError {
                    message: format!("Invalid number '{}'", value),
                    line,
                    column,
                }),
            }
        }
    }
    
    fn identifier(&mut self, first_char: char, line: usize, column: usize) -> Token {
        let mut value = String::new();
        value.push(first_char);
        
        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
            value.push(self.advance());
        }
        
        let token_type = match value.as_str() {
            "let" => TokenType::Let,
            "const" => TokenType::Const,
            "function" => TokenType::Function,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "return" => TokenType::Return,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "storage" => TokenType::Storage,
            "memory" => TokenType::Memory,
            "keccak256" => TokenType::Keccak256,
            "assert" => TokenType::Assert,
            _ => TokenType::Identifier(value.clone()),
        };
        
        Token::new(token_type, value, line, column)
    }
    
    fn string_literal(&mut self, line: usize, column: usize) -> LexResult<Token> {
        let mut value = String::new();
        
        // Skip the opening quote
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 1;
            }
            value.push(self.advance());
        }
        
        if self.is_at_end() {
            return Err(LexError {
                message: "Unterminated string".to_string(),
                line,
                column,
            });
        }
        
        // Consume the closing quote
        self.advance();
        
        Ok(Token::new(TokenType::String(value.clone()), format!("\"{}\"", value), line, column))
    }
    
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                },
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    self.advance();
                },
                _ => break,
            }
        }
    }
    
    fn advance(&mut self) -> char {
        if !self.is_at_end() {
            let c = self.input[self.current];
            self.current += 1;
            self.column += 1;
            c
        } else {
            '\0'
        }
    }
    
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.input[self.current] != expected {
            false
        } else {
            self.current += 1;
            self.column += 1;
            true
        }
    }
    
    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.current]
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.current >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("+ - * / % ( ) { } ; ,");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 12); // 11 tokens + EOF
        assert_eq!(tokens[0].token_type, TokenType::Plus);
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].token_type, TokenType::Star);
        assert_eq!(tokens[3].token_type, TokenType::Slash);
        assert_eq!(tokens[4].token_type, TokenType::Percent);
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 0xFF 123");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Number(42));
        assert_eq!(tokens[1].token_type, TokenType::Number(255));
        assert_eq!(tokens[2].token_type, TokenType::Number(123));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("let function if else return");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::Let);
        assert_eq!(tokens[1].token_type, TokenType::Function);
        assert_eq!(tokens[2].token_type, TokenType::If);
        assert_eq!(tokens[3].token_type, TokenType::Else);
        assert_eq!(tokens[4].token_type, TokenType::Return);
    }

    #[test]
    fn test_identifiers() {
        let mut lexer = Lexer::new("variable_name myFunc _private");
        let tokens = lexer.tokenize().unwrap();
        
        match &tokens[0].token_type {
            TokenType::Identifier(name) => assert_eq!(name, "variable_name"),
            _ => panic!("Expected identifier"),
        }
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("== != >= <= && ||");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0].token_type, TokenType::EqualEqual);
        assert_eq!(tokens[1].token_type, TokenType::BangEqual);
        assert_eq!(tokens[2].token_type, TokenType::GreaterEqual);
        assert_eq!(tokens[3].token_type, TokenType::LessEqual);
        assert_eq!(tokens[4].token_type, TokenType::AmpersandAmpersand);
        assert_eq!(tokens[5].token_type, TokenType::PipePipe);
    }
}
