use core::fmt;
use std::collections::HashMap;

use crate::{
    ast::{
        AssignExpr, BinaryExpr, BlockStmt, Expr, ExprStmt, GroupingExpr, LiteralExpr, PrintStmt,
        Stmt, UnaryExpr, VarDeclStmt, VariableExpr,
    },
    lexer::TokenType,
};

#[derive(Debug)]
pub enum RuntimeError {
    UnexpectedValue,
    UnexpectedOperator,
    UndefinedVariable,
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Null,
    Boolean(bool),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::String(val) => write!(f, "{}", val),
            Value::Number(val) => write!(f, "{}", val),
            Value::Null => write!(f, "nil"),
            Value::Boolean(val) => write!(f, "{}", val),
        }
    }
}

#[derive(Debug, Clone)]
struct Environment {
    enclosing: Option<Box<Environment>>,
    variables: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            enclosing: None,
        }
    }

    pub fn new_with_enclosing(enclosing: Environment) -> Self {
        Environment {
            variables: HashMap::new(),
            enclosing: Some(Box::new(enclosing)),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Result<&Value, RuntimeError> {
        if let Some(value) = self.variables.get(name) {
            return Ok(value);
        }
        if let Some(enclosing) = &self.enclosing {
            return enclosing.get(name);
        }
        Err(RuntimeError::UndefinedVariable)
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            return Ok(());
        }
        if let Some(enclosing) = &mut self.enclosing {
            return enclosing.assign(name, value);
        }
        Err(RuntimeError::UndefinedVariable)
    }
}

pub struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            environment: Environment::new(),
        }
    }

    pub fn interpret(&mut self, stmts: Vec<Stmt>) -> Result<(), RuntimeError> {
        for stmt in stmts {
            self.evaluate_stmt(stmt)?;
        }
        Ok(())
    }

    fn evaluate_stmt(&mut self, stmt: Stmt) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Expr(expr) => self.visit_expr_stmt(*expr),
            Stmt::Print(print) => self.visit_print_stmt(*print),
            Stmt::VarDecl(var_decl) => self.visit_var_decl_stmt(*var_decl),
            Stmt::Block(block) => self.visit_block_stmt(*block),
        }
    }

    fn visit_expr_stmt(&mut self, expr_stmt: ExprStmt) -> Result<(), RuntimeError> {
        self.evaluate_expr(expr_stmt.expr)?;
        Ok(())
    }

    fn visit_print_stmt(&mut self, print_stmt: PrintStmt) -> Result<(), RuntimeError> {
        let value = self.evaluate_expr(print_stmt.expr)?;
        println!("{}", value);
        Ok(())
    }

    fn visit_var_decl_stmt(&mut self, var_decl: VarDeclStmt) -> Result<(), RuntimeError> {
        let value = if let Some(initializer) = var_decl.initializer {
            self.evaluate_expr(initializer)?
        } else {
            Value::Null
        };
        self.environment.define(var_decl.name.lexeme, value);
        Ok(())
    }

    fn visit_block_stmt(&mut self, block: BlockStmt) -> Result<(), RuntimeError> {
        let environment = Environment::new_with_enclosing(self.environment.clone());
        self.evaluate_block(block.statements, environment)
    }

    fn evaluate_block(
        &mut self,
        statements: Vec<Stmt>,
        environment: Environment,
    ) -> Result<(), RuntimeError> {
        let previous = self.environment.clone();
        self.environment = environment;
        for stmt in statements {
            match self.evaluate_stmt(stmt) {
                Ok(_) => {}
                Err(e) => {
                    self.environment = previous;
                    return Err(e);
                }
            }
        }
        self.environment = previous;
        Ok(())
    }

    fn evaluate_expr(&mut self, expression: Expr) -> Result<Value, RuntimeError> {
        match expression {
            Expr::Literal(literal) => self.visit_literal_expr(*literal),
            Expr::Grouping(grouping) => self.visit_grouping_expr(*grouping),
            Expr::Unary(unary) => self.visit_unary_expr(*unary),
            Expr::Binary(binary) => self.visit_binary_expr(*binary),
            Expr::Variable(variable) => self.visit_variable_expr(*variable),
            Expr::Assign(assign) => self.visit_assign_expr(*assign),
        }
    }

    fn visit_literal_expr(&self, literal: LiteralExpr) -> Result<Value, RuntimeError> {
        match literal.value.r#type {
            TokenType::STRING(val) => Ok(Value::String(val)),
            TokenType::NUMBER(val) => Ok(Value::Number(val)),
            TokenType::NIL => Ok(Value::Null),
            TokenType::FALSE => Ok(Value::Boolean(false)),
            TokenType::TRUE => Ok(Value::Boolean(true)),
            _ => panic!("Unexpected token"),
        }
    }

    fn visit_grouping_expr(&mut self, grouping: GroupingExpr) -> Result<Value, RuntimeError> {
        return self.evaluate_expr(grouping.expression);
    }

    fn visit_unary_expr(&mut self, unary: UnaryExpr) -> Result<Value, RuntimeError> {
        let right = self.evaluate_expr(unary.right)?;
        match unary.operator.r#type {
            TokenType::MINUS => match right {
                Value::Number(val) => Ok(Value::Number(-val)),
                _ => Err(RuntimeError::UnexpectedOperator),
            },
            TokenType::BANG => Ok(Value::Boolean(!self.is_truthy(right))),
            _ => Err(RuntimeError::UnexpectedOperator),
        }
    }

    fn visit_binary_expr(&mut self, binary: BinaryExpr) -> Result<Value, RuntimeError> {
        let left = self.evaluate_expr(binary.left)?;
        let right = self.evaluate_expr(binary.right)?;
        match (left, right) {
            (Value::Number(left), Value::Number(right)) => match binary.operator.r#type {
                TokenType::PLUS => Ok(Value::Number(left + right)),
                TokenType::MINUS => Ok(Value::Number(left - right)),
                TokenType::STAR => Ok(Value::Number(left * right)),
                TokenType::SLASH => Ok(Value::Number(left / right)),
                TokenType::GREATER => Ok(Value::Boolean(left > right)),
                TokenType::GREATER_EQUAL => Ok(Value::Boolean(left >= right)),
                TokenType::LESS => Ok(Value::Boolean(left < right)),
                TokenType::LESS_EQUAL => Ok(Value::Boolean(left <= right)),
                TokenType::BANG_EQUAL => Ok(Value::Boolean(left != right)),
                TokenType::EQUAL_EQUAL => Ok(Value::Boolean(left == right)),
                _ => Err(RuntimeError::UnexpectedOperator),
            },
            (Value::String(left), Value::String(right)) => match binary.operator.r#type {
                TokenType::PLUS => Ok(Value::String(format!("{}{}", left, right))),
                TokenType::BANG_EQUAL => Ok(Value::Boolean(left != right)),
                TokenType::EQUAL_EQUAL => Ok(Value::Boolean(left == right)),
                _ => Err(RuntimeError::UnexpectedOperator),
            },
            _ => {
                tracing::error!("Operands must be two numbers or two strings");
                Err(RuntimeError::UnexpectedValue)
            }
        }
    }

    fn visit_variable_expr(&self, variable: VariableExpr) -> Result<Value, RuntimeError> {
        match self.environment.get(&variable.name.lexeme) {
            Ok(value) => Ok(value.clone()),
            Err(e) => Err(e),
        }
    }

    fn visit_assign_expr(&mut self, assign: AssignExpr) -> Result<Value, RuntimeError> {
        let value = self.evaluate_expr(assign.value)?;
        self.environment
            .assign(&assign.name.lexeme, value.clone())?;
        Ok(value)
    }

    fn is_truthy(&self, value: Value) -> bool {
        match value {
            Value::Null => false,
            Value::Boolean(val) => val,
            _ => true,
        }
    }
}
