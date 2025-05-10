// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsEngineError {
    #[error("Lexer error at position {position}: {message}")]
    LexerError { position: usize, message: String },
    
    #[error("Parser error at line {line}, column {column}: {message}")]
    ParserError { line: usize, column: usize, message: String },
    
    #[error("Runtime error: {message}")]
    RuntimeError { message: String },
    
    #[error("Type error: {message}")]
    TypeError { message: String },
    
    #[error("Reference error: {message}")]
    ReferenceError { message: String },
    
    #[error("Syntax error: {message}")]
    SyntaxError { message: String },
}