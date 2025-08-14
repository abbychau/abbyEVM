use crate::compiler::lexer::{Token, TokenType};
use crate::compiler::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

type ParseResult<T> = Result<T, ParseError>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }
    
    pub fn parse(&mut self) -> ParseResult<Program> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        Ok(Program { statements })
    }
    
    // Grammar rules implementation
    
    fn declaration(&mut self) -> ParseResult<Statement> {
        if self.match_token(&TokenType::Let) || self.match_token(&TokenType::Const) {
            self.var_declaration()
        } else if self.match_token(&TokenType::Function) {
            self.function_declaration()
        } else {
            self.statement()
        }
    }
    
    fn var_declaration(&mut self) -> ParseResult<Statement> {
        let name = self.consume_identifier("Expected variable name")?;
        
        // Check for array syntax: let storage[key] = value
        if self.match_token(&TokenType::LeftBracket) {
            let index = self.expression()?;
            self.consume(&TokenType::RightBracket, "Expected ']' after array index")?;
            self.consume(&TokenType::Equal, "Expected '=' after array declaration")?;
            let value = self.expression()?;
            self.consume(&TokenType::Semicolon, "Expected ';' after array declaration")?;
            
            // Convert to appropriate assignment statement
            match name.as_str() {
                "storage" => Ok(Statement::ExprStmt(ExprStmt { 
                    expression: Expression::storage_array_assignment(index, value) 
                })),
                "memory" => Ok(Statement::ExprStmt(ExprStmt { 
                    expression: Expression::MemoryAccess(MemoryAccessExpr::Store(Box::new(index), Box::new(value))) 
                })),
                _ => Err(self.error("Array declaration only supported for storage and memory")),
            }
        } else {
            // Regular variable declaration
            self.consume(&TokenType::Equal, "Expected '=' after variable name")?;
            let initializer = self.expression()?;
            self.consume(&TokenType::Semicolon, "Expected ';' after variable declaration")?;
            Ok(Statement::var_decl(name, initializer))
        }
    }
    
    fn function_declaration(&mut self) -> ParseResult<Statement> {
        let name = self.consume_identifier("Expected function name")?;
        
        self.consume(&TokenType::LeftParen, "Expected '(' after function name")?;
        
        let mut params = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                params.push(self.consume_identifier("Expected parameter name")?);
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.consume(&TokenType::RightParen, "Expected ')' after parameters")?;
        
        self.consume(&TokenType::LeftBrace, "Expected '{' before function body")?;
        let body = self.block()?;
        
        Ok(Statement::func_decl(name, params, body))
    }
    
    fn statement(&mut self) -> ParseResult<Statement> {
        if self.match_token(&TokenType::If) {
            self.if_statement()
        } else if self.match_token(&TokenType::While) {
            self.while_statement()
        } else if self.match_token(&TokenType::Return) {
            self.return_statement()
        } else if self.match_token(&TokenType::LeftBrace) {
            Ok(Statement::Block(self.block()?))
        } else {
            self.expression_statement()
        }
    }
    
    fn if_statement(&mut self) -> ParseResult<Statement> {
        self.consume(&TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RightParen, "Expected ')' after if condition")?;
        
        let then_branch = self.statement()?;
        let else_branch = if self.match_token(&TokenType::Else) {
            Some(self.statement()?)
        } else {
            None
        };
        
        Ok(Statement::if_stmt(condition, then_branch, else_branch))
    }
    
    fn while_statement(&mut self) -> ParseResult<Statement> {
        self.consume(&TokenType::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RightParen, "Expected ')' after while condition")?;
        
        let body = self.statement()?;
        
        Ok(Statement::while_stmt(condition, body))
    }
    
    fn return_statement(&mut self) -> ParseResult<Statement> {
        let value = if self.check(&TokenType::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        
        self.consume(&TokenType::Semicolon, "Expected ';' after return value")?;
        
        Ok(Statement::return_stmt(value))
    }
    
    fn block(&mut self) -> ParseResult<Block> {
        let mut statements = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        self.consume(&TokenType::RightBrace, "Expected '}' after block")?;
        
        Ok(Block { statements })
    }
    
    fn expression_statement(&mut self) -> ParseResult<Statement> {
        let expr = self.expression()?;
        self.consume(&TokenType::Semicolon, "Expected ';' after expression")?;
        Ok(Statement::expr_stmt(expr))
    }
    
    pub fn expression(&mut self) -> ParseResult<Expression> {
        self.assignment()
    }
    
    fn assignment(&mut self) -> ParseResult<Expression> {
        let expr = self.or()?;
        
        if self.match_token(&TokenType::Equal) {
            let value = self.assignment()?;
            
            match expr {
                // Regular variable assignment: x = value
                Expression::Variable(var) => {
                    // Special handling for memory and storage
                    match var.name.as_str() {
                        "memory" => Ok(Expression::memory_assignment(value)),
                        _ => Ok(Expression::assignment(var.name, value)),
                    }
                },
                // Array access assignment: obj[index] = value
                Expression::ArrayAccess(array_access) => {
                    if let Expression::Variable(var) = *array_access.object {
                        match var.name.as_str() {
                            "storage" => Ok(Expression::storage_array_assignment(*array_access.index, value)),
                            "memory" => {
                                // memory[offset] = value -> memory.store(offset, value)
                                Ok(Expression::MemoryAccess(MemoryAccessExpr::Store(Box::new(*array_access.index), Box::new(value))))
                            },
                            _ => Err(self.error("Invalid assignment target")),
                        }
                    } else {
                        Err(self.error("Invalid assignment target"))
                    }
                },
                _ => Err(self.error("Invalid assignment target")),
            }
        } else {
            Ok(expr)
        }
    }
    
    fn or(&mut self) -> ParseResult<Expression> {
        let mut expr = self.and()?;
        
        while self.match_token(&TokenType::PipePipe) {
            let right = self.and()?;
            expr = Expression::binary(expr, BinaryOperator::Or, right);
        }
        
        Ok(expr)
    }
    
    fn and(&mut self) -> ParseResult<Expression> {
        let mut expr = self.equality()?;
        
        while self.match_token(&TokenType::AmpersandAmpersand) {
            let right = self.equality()?;
            expr = Expression::binary(expr, BinaryOperator::And, right);
        }
        
        Ok(expr)
    }
    
    fn equality(&mut self) -> ParseResult<Expression> {
        let mut expr = self.comparison()?;
        
        while let Some(op) = self.match_binary_op(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let right = self.comparison()?;
            expr = Expression::binary(expr, op, right);
        }
        
        Ok(expr)
    }
    
    fn comparison(&mut self) -> ParseResult<Expression> {
        let mut expr = self.term()?;
        
        while let Some(op) = self.match_binary_op(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let right = self.term()?;
            expr = Expression::binary(expr, op, right);
        }
        
        Ok(expr)
    }
    
    fn term(&mut self) -> ParseResult<Expression> {
        let mut expr = self.factor()?;
        
        while let Some(op) = self.match_binary_op(&[TokenType::Minus, TokenType::Plus]) {
            let right = self.factor()?;
            expr = Expression::binary(expr, op, right);
        }
        
        Ok(expr)
    }
    
    fn factor(&mut self) -> ParseResult<Expression> {
        let mut expr = self.unary()?;
        
        while let Some(op) = self.match_binary_op(&[TokenType::Slash, TokenType::Star, TokenType::Percent]) {
            let right = self.unary()?;
            expr = Expression::binary(expr, op, right);
        }
        
        Ok(expr)
    }
    
    fn unary(&mut self) -> ParseResult<Expression> {
        if let Some(op) = self.match_unary_op(&[TokenType::Bang, TokenType::Minus]) {
            let right = self.unary()?;
            return Ok(Expression::unary(op, right));
        }
        
        self.call()
    }
    
    fn call(&mut self) -> ParseResult<Expression> {
        let mut expr = self.primary()?;
        
        loop {
            if self.match_token(&TokenType::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(&TokenType::LeftBracket) {
                // Handle array access: expr[index]
                let index = self.expression()?;
                self.consume(&TokenType::RightBracket, "Expected ']' after array index")?;
                expr = Expression::array_access(expr, index);
            } else if self.match_token(&TokenType::Dot) {
                // Handle member access: expr.property
                let property = self.consume_identifier("Expected property name after '.'")?;
                expr = Expression::member_access(expr, property);
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    fn finish_call(&mut self, callee: Expression) -> ParseResult<Expression> {
        let mut arguments = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.consume(&TokenType::RightParen, "Expected ')' after arguments")?;
        
        Ok(Expression::call(callee, arguments))
    }
    
    fn primary(&mut self) -> ParseResult<Expression> {
        if let Some(token) = self.advance() {
            match &token.token_type {
                TokenType::True => Ok(Expression::boolean(true)),
                TokenType::False => Ok(Expression::boolean(false)),
                TokenType::Number(n) => Ok(Expression::number(*n)),
                TokenType::String(s) => Ok(Expression::string(s.clone())),
                TokenType::Identifier(name) => {
                    Ok(Expression::variable(name.clone()))
                },
                TokenType::Storage => {
                    // Check if it's storage.method() or storage[index]
                    if self.check(&TokenType::Dot) {
                        self.consume(&TokenType::Dot, "Expected '.' after 'storage'")?;
                        self.handle_storage_method()
                    } else {
                        // Just return storage as a variable for array access handling
                        Ok(Expression::variable("storage".to_string()))
                    }
                },
                TokenType::Memory => {
                    // Check if it's memory.method() or memory[index] or memory alone
                    if self.check(&TokenType::Dot) {
                        self.consume(&TokenType::Dot, "Expected '.' after 'memory'")?;
                        self.handle_memory_method()
                    } else {
                        // Just return memory as a variable for assignment/array access handling
                        Ok(Expression::variable("memory".to_string()))
                    }
                },
                TokenType::LeftParen => {
                    let expr = self.expression()?;
                    self.consume(&TokenType::RightParen, "Expected ')' after expression")?;
                    Ok(expr)
                },
                TokenType::LeftBracket => {
                    // Array literal: [element1, element2, ...]
                    let mut elements = Vec::new();
                    
                    if !self.check(&TokenType::RightBracket) {
                        loop {
                            elements.push(self.expression()?);
                            if !self.match_token(&TokenType::Comma) {
                                break;
                            }
                        }
                    }
                    
                    if !self.match_token(&TokenType::RightBracket) {
                        return Err(self.error("Expected ']' after array elements"));
                    }
                    
                    Ok(Expression::array(elements))
                },
                _ => Err(self.error("Expected expression")),
            }
        } else {
            Err(self.error("Unexpected end of input"))
        }
    }
    
    fn handle_builtin_method(&mut self, object: String) -> ParseResult<Expression> {
        match object.as_str() {
            "console" => {
                // Handle console methods like console.log(), console.warn(), console.error()
                if let Some(token) = self.advance() {
                    if let TokenType::Identifier(method) = &token.token_type {
                        let method_name = method.clone();
                        match method_name.as_str() {
                            "log" | "warn" | "error" => {
                                // Return a member access expression that will be handled by call parsing
                                let console_expr = Expression::variable("console".to_string());
                                Ok(Expression::member_access(console_expr, method_name))
                            },
                            _ => Err(ParseError {
                                message: format!("Unknown console method '{}'", method_name),
                                line: token.line,
                                column: token.column,
                            }),
                        }
                    } else {
                        Err(self.error("Expected method name after '.'"))
                    }
                } else {
                    Err(self.error("Expected method name after '.'"))
                }
            },
            _ => Err(self.error(&format!("Unknown method on object '{}'", object))),
        }
    }
    
    fn handle_storage_method(&mut self) -> ParseResult<Expression> {
        if let Some(token) = self.advance() {
            if let TokenType::Identifier(method) = &token.token_type {
                let method_name = method.clone();
                match method_name.as_str() {
                    "get" => {
                        self.consume(&TokenType::LeftParen, "Expected '(' after 'get'")?;
                        let key = self.expression()?;
                        self.consume(&TokenType::RightParen, "Expected ')' after key")?;
                        Ok(Expression::StorageAccess(StorageAccessExpr::Get(Box::new(key))))
                    },
                    "set" => {
                        self.consume(&TokenType::LeftParen, "Expected '(' after 'set'")?;
                        let key = self.expression()?;
                        self.consume(&TokenType::Comma, "Expected ',' after key")?;
                        let value = self.expression()?;
                        self.consume(&TokenType::RightParen, "Expected ')' after value")?;
                        Ok(Expression::StorageAccess(StorageAccessExpr::Set(Box::new(key), Box::new(value))))
                    },
                    _ => Err(ParseError {
                        message: format!("Unknown storage method '{}'", method_name),
                        line: token.line,
                        column: token.column,
                    }),
                }
            } else {
                Err(ParseError {
                    message: "Expected method name after 'storage.'".to_string(),
                    line: token.line,
                    column: token.column,
                })
            }
        } else {
            Err(ParseError {
                message: "Unexpected end of input after 'storage.'".to_string(),
                line: 0,
                column: 0,
            })
        }
    }
    
    fn handle_memory_method(&mut self) -> ParseResult<Expression> {
        if let Some(token) = self.advance() {
            if let TokenType::Identifier(method) = &token.token_type {
                let method_name = method.clone();
                match method_name.as_str() {
                    "load" => {
                        self.consume(&TokenType::LeftParen, "Expected '(' after 'load'")?;
                        let offset = self.expression()?;
                        self.consume(&TokenType::RightParen, "Expected ')' after offset")?;
                        Ok(Expression::MemoryAccess(MemoryAccessExpr::Load(Box::new(offset))))
                    },
                    "store" => {
                        self.consume(&TokenType::LeftParen, "Expected '(' after 'store'")?;
                        let offset = self.expression()?;
                        self.consume(&TokenType::Comma, "Expected ',' after offset")?;
                        let value = self.expression()?;
                        self.consume(&TokenType::RightParen, "Expected ')' after value")?;
                        Ok(Expression::MemoryAccess(MemoryAccessExpr::Store(Box::new(offset), Box::new(value))))
                    },
                    _ => Err(ParseError {
                        message: format!("Unknown memory method '{}'", method_name),
                        line: token.line,
                        column: token.column,
                    }),
                }
            } else {
                Err(ParseError {
                    message: "Expected method name after 'memory.'".to_string(),
                    line: token.line,
                    column: token.column,
                })
            }
        } else {
            Err(ParseError {
                message: "Unexpected end of input after 'memory.'".to_string(),
                line: 0,
                column: 0,
            })
        }
    }
    
    // Helper methods
    
    fn match_binary_op(&mut self, types: &[TokenType]) -> Option<BinaryOperator> {
        for token_type in types {
            if self.check(token_type) {
                self.advance();
                return Some(match token_type {
                    TokenType::Plus => BinaryOperator::Add,
                    TokenType::Minus => BinaryOperator::Subtract,
                    TokenType::Star => BinaryOperator::Multiply,
                    TokenType::Slash => BinaryOperator::Divide,
                    TokenType::Percent => BinaryOperator::Modulo,
                    TokenType::EqualEqual => BinaryOperator::Equal,
                    TokenType::BangEqual => BinaryOperator::NotEqual,
                    TokenType::Greater => BinaryOperator::Greater,
                    TokenType::GreaterEqual => BinaryOperator::GreaterEqual,
                    TokenType::Less => BinaryOperator::Less,
                    TokenType::LessEqual => BinaryOperator::LessEqual,
                    TokenType::AmpersandAmpersand => BinaryOperator::And,
                    TokenType::PipePipe => BinaryOperator::Or,
                    _ => unreachable!(),
                });
            }
        }
        None
    }
    
    fn match_unary_op(&mut self, types: &[TokenType]) -> Option<UnaryOperator> {
        for token_type in types {
            if self.check(token_type) {
                self.advance();
                return Some(match token_type {
                    TokenType::Minus => UnaryOperator::Minus,
                    TokenType::Bang => UnaryOperator::Not,
                    _ => unreachable!(),
                });
            }
        }
        None
    }
    
    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }
    
    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
        }
    }
    
    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.current += 1;
            Some(&self.tokens[self.current - 1])
        } else {
            None
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || 
        matches!(self.peek().token_type, TokenType::Eof)
    }
    
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    
    fn consume(&mut self, token_type: &TokenType, message: &str) -> ParseResult<()> {
        if self.check(token_type) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }
    
    fn consume_identifier(&mut self, message: &str) -> ParseResult<String> {
        if let Some(token) = self.advance() {
            match &token.token_type {
                TokenType::Identifier(name) => Ok(name.clone()),
                TokenType::Storage => Ok("storage".to_string()),
                TokenType::Memory => Ok("memory".to_string()),
                _ => Err(self.error(message))
            }
        } else {
            Err(self.error(message))
        }
    }
    
    fn error(&self, message: &str) -> ParseError {
        let token = if self.is_at_end() && self.current > 0 {
            &self.tokens[self.current - 1]
        } else if !self.is_at_end() {
            &self.tokens[self.current]
        } else {
            // Fallback for empty token list
            return ParseError {
                message: message.to_string(),
                line: 1,
                column: 1,
            };
        };
        
        ParseError {
            message: message.to_string(),
            line: token.line,
            column: token.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;

    fn parse_expression(input: &str) -> ParseResult<Expression> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.expression()
    }

    #[test]
    fn test_simple_expression() {
        let expr = parse_expression("1 + 2").unwrap();
        match expr {
            Expression::Binary(binary) => {
                assert_eq!(binary.operator, BinaryOperator::Add);
            },
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_precedence() {
        let expr = parse_expression("1 + 2 * 3").unwrap();
        match expr {
            Expression::Binary(binary) => {
                assert_eq!(binary.operator, BinaryOperator::Add);
                // Right side should be multiplication
                match &*binary.right {
                    Expression::Binary(right_binary) => {
                        assert_eq!(right_binary.operator, BinaryOperator::Multiply);
                    },
                    _ => panic!("Expected multiplication on right side"),
                }
            },
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_parentheses() {
        let expr = parse_expression("(1 + 2) * 3").unwrap();
        match expr {
            Expression::Binary(binary) => {
                assert_eq!(binary.operator, BinaryOperator::Multiply);
                // Left side should be addition
                match &*binary.left {
                    Expression::Binary(left_binary) => {
                        assert_eq!(left_binary.operator, BinaryOperator::Add);
                    },
                    _ => panic!("Expected addition on left side"),
                }
            },
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_variable() {
        let expr = parse_expression("myVar").unwrap();
        match expr {
            Expression::Variable(var) => {
                assert_eq!(var.name, "myVar");
            },
            _ => panic!("Expected variable expression"),
        }
    }
}
