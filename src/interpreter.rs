// src/interpreter.rs
use crate::error::JsEngineError;
use crate::parser::{Expr, Stmt, BinaryOp, UnaryOp};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

// JavaScript values
#[derive(Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Function(Rc<JsFunction>),
    NativeFunction(Rc<NativeFunction>),
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Undefined,
}

// Implement debug for Value
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Function(_) => write!(f, "[Function]"),
            Value::NativeFunction(_) => write!(f, "[Native Function]"),
            Value::Object(_) => write!(f, "[Object]"),
            Value::Undefined => write!(f, "undefined"),
        }
    }
}

// Implement display for Value
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Function(_) => write!(f, "[Function]"),
            Value::NativeFunction(_) => write!(f, "[Native Function]"),
            Value::Object(_) => write!(f, "[Object]"),
            Value::Undefined => write!(f, "undefined"),
        }
    }
}

// JavaScript function
pub struct JsFunction {
    pub parameters: Vec<String>,
    pub body: Stmt,
    pub closure: Environment,
}

// Native function type
pub type NativeFunction = dyn Fn(Vec<Value>) -> Result<Value, JsEngineError>;

// Environment for storing variables
#[derive(Clone)]
pub struct Environment {
    values: HashMap<String, Value>,
    enclosing: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
            enclosing: None,
        }
    }
    
    pub fn with_enclosing(enclosing: Rc<RefCell<Environment>>) -> Self {
        Environment {
            values: HashMap::new(),
            enclosing: Some(enclosing),
        }
    }
    
    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }
    
    pub fn get(&self, name: &str) -> Result<Value, JsEngineError> {
        if let Some(value) = self.values.get(name) {
            Ok(value.clone())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.borrow().get(name)
        } else {
            Err(JsEngineError::ReferenceError {
                message: format!("'{}' is not defined", name),
            })
        }
    }
    
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), JsEngineError> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            Ok(())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.borrow_mut().assign(name, value)
        } else {
            Err(JsEngineError::ReferenceError {
                message: format!("'{}' is not defined", name),
            })
        }
    }
}

// Return type for control flow
pub enum ExecutionResult {
    Value(Value),
    Return(Value),
    None,
}

// Interpreter
pub struct Interpreter {
    environment: Rc<RefCell<Environment>>,
    globals: Rc<RefCell<Environment>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let globals = Rc::new(RefCell::new(Environment::new()));
        
        // Define global functions
        let console_log = Rc::new(|args: Vec<Value>| -> Result<Value, JsEngineError> {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg);
            }
            println!();
            Ok(Value::Undefined)
        });
        
        globals.borrow_mut().define(
            "console.log".to_string(),
            Value::NativeFunction(console_log),
        );
        
        Interpreter {
            environment: Rc::clone(&globals),
            globals,
        }
    }
    
    pub fn interpret(&mut self, statements: Vec<Stmt>) -> Result<Value, JsEngineError> {
        let mut last_value = Value::Undefined;
        
        for statement in statements {
            match self.execute(&statement)? {
                ExecutionResult::Value(value) => last_value = value,
                ExecutionResult::Return(value) => return Ok(value),
                ExecutionResult::None => {}
            }
        }
        
        Ok(last_value)
    }
    
    fn execute(&mut self, stmt: &Stmt) -> Result<ExecutionResult, JsEngineError> {
        match stmt {
            Stmt::Expression(expr) => {
                let value = self.evaluate(expr)?;
                Ok(ExecutionResult::Value(value))
            },
            Stmt::Declaration(name, initializer) => {
                let value = if let Some(expr) = initializer {
                    self.evaluate(expr)?
                } else {
                    Value::Undefined
                };
                
                self.environment.borrow_mut().define(name.clone(), value);
                Ok(ExecutionResult::None)
            },
            Stmt::Block(statements) => {
                let previous = Rc::clone(&self.environment);
                self.environment = Rc::new(RefCell::new(Environment::with_enclosing(previous)));
                
                let mut result = ExecutionResult::None;
                
                for statement in statements {
                    result = self.execute(statement)?;
                    
                    if let ExecutionResult::Return(_) = result {
                        break;
                    }
                }
                
                // Restore previous environment
                {
                    let enclosing_env = Rc::clone(&self.environment.borrow().enclosing.as_ref().unwrap());
                    self.environment = enclosing_env;
                }
                
                Ok(result)
            },
            Stmt::If(condition, then_branch, else_branch) => {
                let condition_value = self.evaluate(condition)?;
                
                if self.is_truthy(&condition_value) {
                    self.execute(then_branch)
                } else if let Some(else_stmt) = else_branch {
                    self.execute(else_stmt)
                } else {
                    Ok(ExecutionResult::None)
                }
            },
            Stmt::While(condition, body) => {
                let condition_value = self.evaluate(condition)?;
                
                let mut result = ExecutionResult::None;
                
                while self.is_truthy(&condition_value) {
                    result = self.execute(body)?;
                    
                    if let ExecutionResult::Return(_) = result {
                        break;
                    }
                }
                
                Ok(result)
            },
            Stmt::Return(value) => {
                let return_value = if let Some(expr) = value {
                    self.evaluate(expr)?
                } else {
                    Value::Undefined
                };
                
                Ok(ExecutionResult::Return(return_value))
            },
            Stmt::Function(name, parameters, body) => {
                let function = Value::Function(Rc::new(JsFunction {
                    parameters: parameters.clone(),
                    body: (**body).clone(),
                    closure: self.environment.borrow().clone(),
                }));
                
                self.environment.borrow_mut().define(name.clone(), function);
                Ok(ExecutionResult::None)
            },
        }
    }
    
    fn evaluate(&mut self, expr: &Expr) -> Result<Value, JsEngineError> {
        match expr {
            Expr::Number(value) => Ok(Value::Number(*value)),
            Expr::String(value) => Ok(Value::String(value.clone())),
            Expr::Boolean(value) => Ok(Value::Boolean(*value)),
            Expr::Null => Ok(Value::Null),
            Expr::Variable(name) => self.environment.borrow().get(name),
            Expr::Binary(left, operator, right) => {
                let left_value = self.evaluate(left)?;
                let right_value = self.evaluate(right)?;
                
                match operator {
                    BinaryOp::Add => self.add(&left_value, &right_value),
                    BinaryOp::Subtract => self.subtract(&left_value, &right_value),
                    BinaryOp::Multiply => self.multiply(&left_value, &right_value),
                    BinaryOp::Divide => self.divide(&left_value, &right_value),
                    BinaryOp::Equal => Ok(Value::Boolean(self.is_equal(&left_value, &right_value))),
                    BinaryOp::NotEqual => Ok(Value::Boolean(!self.is_equal(&left_value, &right_value))),
                    BinaryOp::Less => self.less_than(&left_value, &right_value),
                    BinaryOp::LessEqual => self.less_equal(&left_value, &right_value),
                    BinaryOp::Greater => self.greater_than(&left_value, &right_value),
                    BinaryOp::GreaterEqual => self.greater_equal(&left_value, &right_value),
                    BinaryOp::And => {
                        if !self.is_truthy(&left_value) {
                            Ok(left_value)
                        } else {
                            Ok(right_value)
                        }
                    },
                    BinaryOp::Or => {
                        if self.is_truthy(&left_value) {
                            Ok(left_value)
                        } else {
                            Ok(right_value)
                        }
                    },
                }
            },
            Expr::Unary(operator, right) => {
                let right_value = self.evaluate(right)?;
                
                match operator {
                    UnaryOp::Negate => self.negate(&right_value),
                    UnaryOp::Not => Ok(Value::Boolean(!self.is_truthy(&right_value))),
                }
            },
            Expr::Conditional(condition, then_branch, else_branch) => {
                let condition_value = self.evaluate(condition)?;
                
                if self.is_truthy(&condition_value) {
                    self.evaluate(then_branch)
                } else if let Some(else_expr) = else_branch {
                    self.evaluate(else_expr)
                } else {
                    Ok(Value::Undefined)
                }
            },
            Expr::Assign(name, value) => {
                let value = self.evaluate(value)?;
                self.environment.borrow_mut().assign(name, value.clone())?;
                Ok(value)
            },
            Expr::Call(callee, arguments) => {
                let callee_value = self.evaluate(callee)?;
                
                let mut arg_values = Vec::new();
                for argument in arguments {
                    arg_values.push(self.evaluate(argument)?);
                }
                
                self.call(&callee_value, arg_values)
            },
            Expr::Function(parameters, body) => {
                Ok(Value::Function(Rc::new(JsFunction {
                    parameters: parameters.clone(),
                    body: (**body).clone(),
                    closure: self.environment.borrow().clone(),
                })))
            },
        }
    }
    
    fn call(&mut self, callee: &Value, arguments: Vec<Value>) -> Result<Value, JsEngineError> {
        match callee {
            Value::Function(function) => {
                // Create a new environment for the function call
                let mut environment = Environment::with_enclosing(Rc::new(RefCell::new(function.closure.clone())));
                
                // Bind arguments to parameters
                for (i, param) in function.parameters.iter().enumerate() {
                    let value = if i < arguments.len() {
                        arguments[i].clone()
                    } else {
                        Value::Undefined
                    };
                    
                    environment.define(param.clone(), value);
                }
                
                let previous = Rc::clone(&self.environment);
                self.environment = Rc::new(RefCell::new(environment));
                
                // Execute function body
                let result = match self.execute(&function.body)? {
                    ExecutionResult::Return(value) => value,
                    ExecutionResult::Value(value) => value,
                    ExecutionResult::None => Value::Undefined,
                };
                
                // Restore previous environment
                self.environment = previous;
                
                Ok(result)
            },
            Value::NativeFunction(function) => function(arguments),
            _ => Err(JsEngineError::TypeError {
                message: format!("{:?} is not a function", callee),
            }),
        }
    }
    
    // Helper methods for evaluating expressions
    
    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Null => false,
            Value::Undefined => false,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }
    
    fn is_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            _ => false,
        }
    }
    
    fn add(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::String(a), Value::String(b)) => {
                let mut result = a.clone();
                result.push_str(b);
                Ok(Value::String(result))
            },
            (Value::String(a), b) => {
                let mut result = a.clone();
                result.push_str(&format!("{}", b));
                Ok(Value::String(result))
            },
            (a, Value::String(b)) => {
                let mut result = format!("{}", a);
                result.push_str(b);
                Ok(Value::String(result))
            },
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot add {:?} and {:?}", a, b),
            }),
        }
    }
    
    fn subtract(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot subtract {:?} from {:?}", b, a),
            }),
        }
    }
    
    fn multiply(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot multiply {:?} and {:?}", a, b),
            }),
        }
    }
    
    fn divide(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                if *b == 0.0 {
                    // In JavaScript, division by zero returns Infinity
                    Ok(Value::Number(f64::INFINITY))
                } else {
                    Ok(Value::Number(a / b))
                }
            },
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot divide {:?} by {:?}", a, b),
            }),
        }
    }
    
    fn negate(&self, value: &Value) -> Result<Value, JsEngineError> {
        match value {
            Value::Number(n) => Ok(Value::Number(-n)),
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot negate {:?}", value),
            }),
        }
    }
    
    fn less_than(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a < b)),
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot compare {:?} < {:?}", a, b),
            }),
        }
    }
    
    fn less_equal(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a <= b)),
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot compare {:?} <= {:?}", a, b),
            }),
        }
    }
    
    fn greater_than(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a > b)),
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot compare {:?} > {:?}", a, b),
            }),
        }
    }
    
    fn greater_equal(&self, a: &Value, b: &Value) -> Result<Value, JsEngineError> {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a >= b)),
            _ => Err(JsEngineError::TypeError {
                message: format!("Cannot compare {:?} >= {:?}", a, b),
            }),
        }
    }
}