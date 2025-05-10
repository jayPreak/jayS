// src/main.rs
// mod lexer;
// mod parser;
// mod interpreter;
// mod error;

use std::env;
use std::fs;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        // Execute JavaScript from a file
        let file_path = &args[1];
        let source = fs::read_to_string(file_path)?;
        execute_js(&source)?;
    } else {
        // Interactive REPL mode
        repl()?;
    }
    
    Ok(())
}

fn execute_js(source: &str) -> Result<(), Box<dyn std::error::Error>> {
    // This will be implemented as we build our engine
    println!("Source code to execute: {}", source);
    // TODO: Tokenize, parse, and evaluate the source
    
    Ok(())
}

fn repl() -> Result<(), Box<dyn std::error::Error>> {
    println!("MiniJS Engine REPL (press Ctrl+C to exit)");
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().is_empty() {
            continue;
        }
        
        match execute_js(&input) {
            Ok(_) => {},
            Err(e) => println!("Error: {}", e),
        }
    }
}