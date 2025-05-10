// src/parser.rs
use crate::error::JsEngineError;
use crate::lexer::{Token, TokenType};
use std::rc::Rc;

// Define our AST nodes
#[derive(Debug, Clone)]
pub enum Expr {
    // Literal values
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    
    // Variables
    Variable(String),
    
    // Operations
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    
    // Control flow
    Conditional(Box<Expr>, Box<Expr>, Option<Box<Expr>>), // condition, then-branch, else-branch
    
    // Variables and functions
    Assign(String, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    
    // Function definition
    Function(Vec<String>, Box<Stmt>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression(Expr),
    Declaration(String, Option<Expr>), // var name = expr
    Block(Vec<Stmt>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    Return(Option<Expr>),
    Function(String, Vec<String>, Box<Stmt>), // name, params, body
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add, Subtract, Multiply, Divide,
    Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual,
    And, Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Negate, Not,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
        }
    }
    
    pub fn parse(&mut self) -> Result<Vec<Stmt>, JsEngineError> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        Ok(statements)
    }
    
    fn declaration(&mut self) -> Result<Stmt, JsEngineError> {
        if self.match_token(&[TokenType::Var, TokenType::Let, TokenType::Const]) {
            return self.var_declaration();
        } else if self.match_token(&[TokenType::Function]) {
            return self.function_declaration();
        }
        
        self.statement()
    }
    
    fn var_declaration(&mut self) -> Result<Stmt, JsEngineError> {
        // Expect an identifier
        let name = match &self.peek().token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => {
                return Err(self.error("Expected variable name."));
            }
        };
        
        self.advance(); // Consume the identifier
        
        // Check for initialization
        let initializer = if self.match_token(&[TokenType::Equal]) {
            Some(self.expression()?)
        } else {
            None
        };
        
        // Expect semicolon
        self.consume(TokenType::Semicolon, "Expected ';' after variable declaration.")?;
        
        Ok(Stmt::Declaration(name, initializer))
    }
    
    fn function_declaration(&mut self) -> Result<Stmt, JsEngineError> {
        // Expect a function name
        let name = match &self.peek().token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => {
                return Err(self.error("Expected function name."));
            }
        };
        
        self.advance(); // Consume the function name
        
        // Parse parameters
        self.consume(TokenType::LeftParen, "Expected '(' after function name.")?;
        
        let mut parameters = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                if parameters.len() >= 255 {
                    return Err(self.error("Cannot have more than 255 parameters."));
                }
                
                match &self.peek().token_type {
                    TokenType::Identifier(name) => {
                        parameters.push(name.clone());
                        self.advance();
                    }
                    _ => {
                        return Err(self.error("Expected parameter name."));
                    }
                }
                
                if !self.match_token(&[TokenType::Comma]) {
                    break;
                }
            }
        }
        
        self.consume(TokenType::RightParen, "Expected ')' after parameters.")?;
        
        // Parse function body
        self.consume(TokenType::LeftBrace, "Expected '{' before function body.")?;
        let body = Box::new(self.block()?);
        
        Ok(Stmt::Function(name, parameters, body))
    }
    
    fn statement(&mut self) -> Result<Stmt, JsEngineError> {
        if self.match_token(&[TokenType::If]) {
            self.if_statement()
        } else if self.match_token(&[TokenType::While]) {
            self.while_statement()
        } else if self.match_token(&[TokenType::Return]) {
            self.return_statement()
        } else if self.match_token(&[TokenType::LeftBrace]) {
            Ok(self.block()?)
        } else {
            self.expression_statement()
        }
    }
    
    fn if_statement(&mut self) -> Result<Stmt, JsEngineError> {
        self.consume(TokenType::LeftParen, "Expected '(' after 'if'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expected ')' after if condition.")?;
        
        let then_branch = Box::new(self.statement()?);
        
        let else_branch = if self.match_token(&[TokenType::Else]) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        
        Ok(Stmt::If(condition, then_branch, else_branch))
    }
    
    fn while_statement(&mut self) -> Result<Stmt, JsEngineError> {
        self.consume(TokenType::LeftParen, "Expected '(' after 'while'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expected ')' after while condition.")?;
        
        let body = Box::new(self.statement()?);
        
        Ok(Stmt::While(condition, body))
    }
    
    fn return_statement(&mut self) -> Result<Stmt, JsEngineError> {
        let value = if !self.check(&TokenType::Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };
        
        self.consume(TokenType::Semicolon, "Expected ';' after return value.")?;
        
        Ok(Stmt::Return(value))
    }
    
    fn block(&mut self) -> Result<Stmt, JsEngineError> {
        let mut statements = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        
        self.consume(TokenType::RightBrace, "Expected '}' after block.")?;
        
        Ok(Stmt::Block(statements))
    }
    
    fn expression_statement(&mut self) -> Result<Stmt, JsEngineError> {
        let expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after expression.")?;
        
        Ok(Stmt::Expression(expr))
    }
    
    fn expression(&mut self) -> Result<Expr, JsEngineError> {
        self.assignment()
    }
    
    fn assignment(&mut self) -> Result<Expr, JsEngineError> {
        let expr = self.or()?;
        
        if self.match_token(&[TokenType::Equal]) {
            let value = self.assignment()?;
            
            if let Expr::Variable(name) = expr {
                return Ok(Expr::Assign(name, Box::new(value)));
            }
            
            return Err(self.error("Invalid assignment target."));
        }
        
        Ok(expr)
    }
    
    fn or(&mut self) -> Result<Expr, JsEngineError> {
        let mut expr = self.and()?;
        
        while self.match_token(&[TokenType::Or]) {
            let right = self.and()?;
            expr = Expr::Binary(Box::new(expr), BinaryOp::Or, Box::new(right));
        }
        
        Ok(expr)
    }
    
    fn and(&mut self) -> Result<Expr, JsEngineError> {
        let mut expr = self.equality()?;
        
        while self.match_token(&[TokenType::And]) {
            let right = self.equality()?;
            expr = Expr::Binary(Box::new(expr), BinaryOp::And, Box::new(right));
        }
        
        Ok(expr)
    }
    
    fn equality(&mut self) -> Result<Expr, JsEngineError> {
        let mut expr = self.comparison()?;
        
        while self.match_token(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = match self.previous().token_type {
                TokenType::BangEqual => BinaryOp::NotEqual,
                TokenType::EqualEqual => BinaryOp::Equal,
                _ => unreachable!(),
            };
            
            let right = self.comparison()?;
            expr = Expr::Binary(Box::new(expr), operator, Box::new(right));
        }
        
        Ok(expr)
    }
    
    fn comparison(&mut self) -> Result<Expr, JsEngineError> {
        let mut expr = self.term()?;
        
        while self.match_token(&[
            TokenType::Greater, TokenType::GreaterEqual,
            TokenType::Less, TokenType::LessEqual,
        ]) {
            let operator = match self.previous().token_type {
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEqual,
                _ => unreachable!(),
            };
            
            let right = self.term()?;
            expr = Expr::Binary(Box::new(expr), operator, Box::new(right));
        }
        
        Ok(expr)
    }
    
    fn term(&mut self) -> Result<Expr, JsEngineError> {
        let mut expr = self.factor()?;
        
        while self.match_token(&[TokenType::Minus, TokenType::Plus]) {
            let operator = match self.previous().token_type {
                TokenType::Minus => BinaryOp::Subtract,
                TokenType::Plus => BinaryOp::Add,
                _ => unreachable!(),
            };
            
            let right = self.factor()?;
            expr = Expr::Binary(Box::new(expr), operator, Box::new(right));
        }
        
        Ok(expr)
    }
    
    fn factor(&mut self) -> Result<Expr, JsEngineError> {
        let mut expr = self.unary()?;
        
        while self.match_token(&[TokenType::Slash, TokenType::Star]) {
            let operator = match self.previous().token_type {
                TokenType::Slash => BinaryOp::Divide,
                TokenType::Star => BinaryOp::Multiply,
                _ => unreachable!(),
            };
            
            let right = self.unary()?;
            expr = Expr::Binary(Box::new(expr), operator, Box::new(right));
        }
        
        Ok(expr)
    }
    
    fn unary(&mut self) -> Result<Expr, JsEngineError> {
        if self.match_token(&[TokenType::Bang, TokenType::Minus]) {
            let operator = match self.previous().token_type {
                TokenType::Bang => UnaryOp::Not,
                TokenType::Minus => UnaryOp::Negate,
                _ => unreachable!(),
            };
            
            let right = self.unary()?;
            return Ok(Expr::Unary(operator, Box::new(right)));
        }
        
        self.call()
    }
    
    fn call(&mut self) -> Result<Expr, JsEngineError> {
        let mut expr = self.primary()?;
        
        loop {
            if self.match_token(&[TokenType::LeftParen]) {
                expr = self.finish_call(expr)?;
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    fn finish_call(&mut self, callee: Expr) -> Result<Expr, JsEngineError> {
        let mut arguments = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                if arguments.len() >= 255 {
                    return Err(self.error("Cannot have more than 255 arguments."));
                }
                
                arguments.push(self.expression()?);
                
                if !self.match_token(&[TokenType::Comma]) {
                    break;
                }
            }
        }
        
        self.consume(TokenType::RightParen, "Expected ')' after arguments.")?;
        
        Ok(Expr::Call(Box::new(callee), arguments))
    }
    
    fn primary(&mut self) -> Result<Expr, JsEngineError> {
        if self.match_token(&[TokenType::False]) {
            Ok(Expr::Boolean(false))
        } else if self.match_token(&[TokenType::True]) {
            Ok(Expr::Boolean(true))
        } else if self.match_token(&[TokenType::Null]) {
            Ok(Expr::Null)
        } else if self.match_token(&[TokenType::Number(0.0)]) {
            // Get the actual number from the previous token
            if let TokenType::Number(value) = &self.previous().token_type {
                Ok(Expr::Number(*value))
            } else {
                unreachable!()
            }
        } else if self.match_token(&[TokenType::String("".to_string())]) {
            // Get the actual string from the previous token
            if let TokenType::String(value) = &self.previous().token_type {
                Ok(Expr::String(value.clone()))
            } else {
                unreachable!()
            }
        } else if self.match_token(&[TokenType::Identifier("".to_string())]) {
            // Get the identifier name from the previous token
            if let TokenType::Identifier(name) = &self.previous().token_type {
                Ok(Expr::Variable(name.clone()))
            } else {
                unreachable!()
            }
        } else if self.match_token(&[TokenType::LeftParen]) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expected ')' after expression.")?;
            Ok(expr)
        } else if self.match_token(&[TokenType::Function]) {
            // Anonymous function
            self.consume(TokenType::LeftParen, "Expected '(' after 'function'.")?;
            
            let mut parameters = Vec::new();
            
            if !self.check(&TokenType::RightParen) {
                loop {
                    if parameters.len() >= 255 {
                        return Err(self.error("Cannot have more than 255 parameters."));
                    }
                    
                    match &self.peek().token_type {
                        TokenType::Identifier(name) => {
                            parameters.push(name.clone());
                            self.advance();
                        }
                        _ => {
                            return Err(self.error("Expected parameter name."));
                        }
                    }
                    
                    if !self.match_token(&[TokenType::Comma]) {
                        break;
                    }
                }
            }
            
            self.consume(TokenType::RightParen, "Expected ')' after parameters.")?;
            
            self.consume(TokenType::LeftBrace, "Expected '{' before function body.")?;
            let body = Box::new(self.block()?);
            
            Ok(Expr::Function(parameters, body))
        } else {
            Err(self.error("Expected expression."))
        }
    }
    
    // Helper methods for the parser
    
    fn match_token(&mut self, types: &[TokenType]) -> bool {
        for token_type in types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }
        
        false
    }
    
    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        
        match (token_type, &self.peek().token_type) {
            (TokenType::Number(_), TokenType::Number(_)) => true,
            (TokenType::String(_), TokenType::String(_)) => true,
            (TokenType::Identifier(_), TokenType::Identifier(_)) => true,
            (a, b) if a == b => true,
            _ => false,
        }
    }
    
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        
        self.previous()
    }
    
    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::EOF)
    }
    
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
    
    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token, JsEngineError> {
        if self.check(&token_type) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }
    
    fn error(&self, message: &str) -> JsEngineError {
        let token = self.peek();
        
        JsEngineError::ParserError {
            line: token.line,
            column: token.column,
            message: message.to_string(),
        }
    }
}