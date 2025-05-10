// src/lexer.rs
use crate::error::JsEngineError;
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

// Define all possible token types
#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // Single character tokens
    LeftParen, RightParen, LeftBrace, RightBrace,
    Comma, Dot, Minus, Plus, Semicolon, Slash, Star,
    
    // One or two character tokens
    Bang, BangEqual,
    Equal, EqualEqual,
    Greater, GreaterEqual,
    Less, LessEqual,
    
    // Literals
    Identifier(String),
    String(String),
    Number(f64),
    
    // Keywords
    And, Else, False, Function, If, Null,
    Or, Return, True, Var, While, Let, Const,
    
    // Special tokens
    EOF
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    source: Peekable<Chars<'a>>,
    tokens: Vec<Token>,
    current: usize,
    line: usize,
    column: usize,
    keywords: HashMap<String, TokenType>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut keywords = HashMap::new();
        keywords.insert("and".to_string(), TokenType::And);
        keywords.insert("else".to_string(), TokenType::Else);
        keywords.insert("false".to_string(), TokenType::False);
        keywords.insert("function".to_string(), TokenType::Function);
        keywords.insert("if".to_string(), TokenType::If);
        keywords.insert("null".to_string(), TokenType::Null);
        keywords.insert("or".to_string(), TokenType::Or);
        keywords.insert("return".to_string(), TokenType::Return);
        keywords.insert("true".to_string(), TokenType::True);
        keywords.insert("var".to_string(), TokenType::Var);
        keywords.insert("while".to_string(), TokenType::While);
        keywords.insert("let".to_string(), TokenType::Let);
        keywords.insert("const".to_string(), TokenType::Const);
        
        Lexer {
            source: source.chars().peekable(),
            tokens: Vec::new(),
            current: 0,
            line: 1,
            column: 1,
            keywords,
        }
    }
    
    pub fn scan_tokens(&mut self) -> Result<Vec<Token>, JsEngineError> {
        while let Some(c) = self.advance() {
            self.scan_token(c)?;
        }
        
        // Add EOF token
        self.tokens.push(Token {
            token_type: TokenType::EOF,
            lexeme: "".to_string(),
            line: self.line,
            column: self.column,
        });
        
        Ok(self.tokens.clone())
    }
    
    fn scan_token(&mut self, c: char) -> Result<(), JsEngineError> {
        match c {
            // Single character tokens
            '(' => self.add_token(TokenType::LeftParen, String::from("(")),
            ')' => self.add_token(TokenType::RightParen, String::from(")")),
            '{' => self.add_token(TokenType::LeftBrace, String::from("{")),
            '}' => self.add_token(TokenType::RightBrace, String::from("}")),
            ',' => self.add_token(TokenType::Comma, String::from(",")),
            '.' => self.add_token(TokenType::Dot, String::from(".")),
            '-' => self.add_token(TokenType::Minus, String::from("-")),
            '+' => self.add_token(TokenType::Plus, String::from("+")),
            ';' => self.add_token(TokenType::Semicolon, String::from(";")),
            '*' => self.add_token(TokenType::Star, String::from("*")),
            
            // One or two character tokens
            '!' => {
                if self.match_next('=') {
                    self.add_token(TokenType::BangEqual, String::from("!="));
                } else {
                    self.add_token(TokenType::Bang, String::from("!"));
                }
            },
            '=' => {
                if self.match_next('=') {
                    self.add_token(TokenType::EqualEqual, String::from("=="));
                } else {
                    self.add_token(TokenType::Equal, String::from("="));
                }
            },
            '<' => {
                if self.match_next('=') {
                    self.add_token(TokenType::LessEqual, String::from("<="));
                } else {
                    self.add_token(TokenType::Less, String::from("<"));
                }
            },
            '>' => {
                if self.match_next('=') {
                    self.add_token(TokenType::GreaterEqual, String::from(">="));
                } else {
                    self.add_token(TokenType::Greater, String::from(">"));
                }
            },
            
            // Handle slash or comment
            '/' => {
                if self.match_next('/') {
                    // Comment goes until the end of the line
                    while let Some(&c) = self.source.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash, String::from("/"));
                }
            },
            
            // String literals
            '"' => self.string()?,
            
            // Whitespace
            ' ' | '\r' | '\t' => {
                // Skip whitespace
            },
            '\n' => {
                self.line += 1;
                self.column = 1;
            },
            
            // Number literals
            c if c.is_digit(10) => self.number(c)?,
            
            // Identifiers
            c if self.is_alpha(c) => self.identifier(c)?,
            
            // Unknown character
            _ => {
                return Err(JsEngineError::LexerError {
                    position: self.current,
                    message: format!("Unexpected character: {}", c),
                });
            },
        }
        
        Ok(())
    }
    
    fn advance(&mut self) -> Option<char> {
        if let Some(c) = self.source.next() {
            self.current += 1;
            self.column += 1;
            Some(c)
        } else {
            None
        }
    }
    
    fn match_next(&mut self, expected: char) -> bool {
        if let Some(&next_char) = self.source.peek() {
            if next_char != expected {
                return false;
            }
            
            // Consume the character
            self.source.next();
            self.current += 1;
            self.column += 1;
            true
        } else {
            false
        }
    }
    
    fn add_token(&mut self, token_type: TokenType, lexeme: String) {
        self.tokens.push(Token {
            token_type,
            lexeme,
            line: self.line,
            column: self.column,
        });
    }
    
    fn string(&mut self) -> Result<(), JsEngineError> {
        let start_line = self.line;
        let start_column = self.column - 1; // Because we already consumed the opening quote
        let mut value = String::new();
        
        while let Some(&c) = self.source.peek() {
            if c == '"' {
                break;
            }
            
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            }
            
            value.push(self.advance().unwrap());
        }
        
        // Check if we reached the closing quote
        if self.source.peek().is_none() {
            return Err(JsEngineError::LexerError {
                position: self.current,
                message: "Unterminated string.".to_string(),
            });
        }
        
        // Consume the closing quote
        self.advance();
        
        // Add the token with the string value
        self.tokens.push(Token {
            token_type: TokenType::String(value),
            lexeme: format!("\"{}\"", value),
            line: start_line,
            column: start_column,
        });
        
        Ok(())
    }
    
    fn number(&mut self, first_digit: char) -> Result<(), JsEngineError> {
        let start_column = self.column - 1; // Because we already consumed the first digit
        let mut value = first_digit.to_string();
        
        // Consume digits
        while let Some(&c) = self.source.peek() {
            if !c.is_digit(10) && c != '.' {
                break;
            }
            value.push(self.advance().unwrap());
        }
        
        // Parse the number
        let num_value = match value.parse::<f64>() {
            Ok(n) => n,
            Err(_) => {
                return Err(JsEngineError::LexerError {
                    position: self.current,
                    message: format!("Could not parse number: {}", value),
                });
            }
        };
        
        // Add the token
        self.tokens.push(Token {
            token_type: TokenType::Number(num_value),
            lexeme: value,
            line: self.line,
            column: start_column,
        });
        
        Ok(())
    }
    
    fn identifier(&mut self, first_char: char) -> Result<(), JsEngineError> {
        let start_column = self.column - 1; // Because we already consumed the first character
        let mut name = first_char.to_string();
        
        // Consume identifier characters
        while let Some(&c) = self.source.peek() {
            if !self.is_alphanumeric(c) {
                break;
            }
            name.push(self.advance().unwrap());
        }
        
        // Check if it's a keyword
        let token_type = if let Some(keyword_type) = self.keywords.get(&name) {
            keyword_type.clone()
        } else {
            TokenType::Identifier(name.clone())
        };
        
        // Add the token
        self.tokens.push(Token {
            token_type,
            lexeme: name,
            line: self.line,
            column: start_column,
        });
        
        Ok(())
    }
    
    fn is_alpha(&self, c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }
    
    fn is_alphanumeric(&self, c: char) -> bool {
        self.is_alpha(c) || c.is_digit(10)
    }
}