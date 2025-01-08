use core::fmt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::{
        AssignExpr, BinaryExpr, BlockStmt, CallExpr, Expr, ExprStmt, FunctionDeclStmt,
        GroupingExpr, IfStmt, LiteralExpr, LogicalExpr, PrintStmt, ReturnStmt, Stmt, UnaryExpr,
        VarDeclStmt, VariableExpr, WhileStmt,
    },
    lexer::TokenType,
};

#[derive(Debug)]
pub enum RuntimeError {
    UnexpectedValue,
    UnexpectedOperator,
    UndefinedVariable,
    Return(Option<Value>),
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Null,
    Boolean(bool),
    Function(Box<dyn Callable>),
}

pub trait Callable: CallableClone + fmt::Debug {
    fn call(&self, interpreter: &mut Interpreter, arguments: Vec<Value>) -> Value;
}

pub trait CallableClone {
    fn clone_box(&self) -> Box<dyn Callable>;
}

impl<T> CallableClone for T
where
    T: 'static + Callable + Clone,
{
    fn clone_box(&self) -> Box<dyn Callable> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Callable> {
    fn clone(&self) -> Box<dyn Callable> {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
struct ClockFunction;

impl Callable for ClockFunction {
    fn call(&self, _interpreter: &mut Interpreter, _arguments: Vec<Value>) -> Value {
        Value::Number(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                / 1000.0,
        )
    }
}

#[derive(Debug, Clone)]
struct LoxFunction {
    declaration: FunctionDeclStmt,
    closure: Rc<RefCell<Environment>>,
}

impl LoxFunction {
    pub fn new(declaration: FunctionDeclStmt, closure: Rc<RefCell<Environment>>) -> Self {
        LoxFunction {
            declaration,
            closure,
        }
    }
}

impl Callable for LoxFunction {
    fn call(&self, interpreter: &mut Interpreter, arguments: Vec<Value>) -> Value {
        let mut environment = Environment::new_with_enclosing(self.closure.clone());
        for (param, arg) in self.declaration.params.iter().zip(arguments.iter()) {
            environment.define(param.lexeme.clone(), arg.clone());
        }
        match interpreter.evaluate_block(self.declaration.body.clone(), environment) {
            Err(RuntimeError::Return(Some(value))) => value,
            _ => Value::Null,
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(val) => *val,
            _ => true,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::String(val) => write!(f, "{}", val),
            Value::Number(val) => write!(f, "{}", val),
            Value::Null => write!(f, "nil"),
            Value::Boolean(val) => write!(f, "{}", val),
            Value::Function(_) => write!(f, "<fn>"),
        }
    }
}

#[derive(Debug, Clone)]
struct Environment {
    enclosing: Option<Rc<RefCell<Environment>>>,
    variables: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            enclosing: None,
        }
    }

    pub fn new_with_enclosing(enclosing: Rc<RefCell<Environment>>) -> Self {
        Environment {
            variables: HashMap::new(),
            enclosing: Some(enclosing),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Result<Value, RuntimeError> {
        if let Some(value) = self.variables.get(name) {
            return Ok(value.clone());
        }
        if let Some(enclosing) = &self.enclosing {
            return enclosing.borrow().get(name);
        }
        Err(RuntimeError::UndefinedVariable)
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            return Ok(());
        }
        if let Some(enclosing) = &mut self.enclosing {
            enclosing.borrow_mut().assign(name, value)?;
            return Ok(());
        }
        Err(RuntimeError::UndefinedVariable)
    }
}

pub struct Interpreter {
    environment: Rc<RefCell<Environment>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut environment = Environment::new();
        environment.define(
            "clock".to_string(),
            Value::Function(Box::new(ClockFunction {})),
        );
        Interpreter {
            environment: Rc::new(RefCell::new(environment)),
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
            Stmt::FunctionDecl(function_decl) => self.visit_function_decl_stmt(*function_decl),
            Stmt::Block(block) => self.visit_block_stmt(*block),
            Stmt::If(if_stmt) => self.visit_if_stmt(*if_stmt),
            Stmt::While(while_stmt) => self.visit_while_stmt(*while_stmt),
            Stmt::Return(return_stmt) => self.visit_return_stmt(*return_stmt),
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
        self.environment
            .borrow_mut()
            .define(var_decl.name.lexeme, value);
        Ok(())
    }

    fn visit_function_decl_stmt(
        &mut self,
        function_decl: FunctionDeclStmt,
    ) -> Result<(), RuntimeError> {
        let function = Value::Function(Box::new(LoxFunction::new(
            function_decl.clone(),
            self.environment.clone(),
        )));
        self.environment
            .borrow_mut()
            .define(function_decl.name.lexeme, function);
        Ok(())
    }

    fn visit_block_stmt(&mut self, block: BlockStmt) -> Result<(), RuntimeError> {
        let environment = Environment::new_with_enclosing(self.environment.clone());
        self.evaluate_block(block.statements, environment)
    }

    fn visit_if_stmt(&mut self, if_stmt: IfStmt) -> Result<(), RuntimeError> {
        let condition = self.evaluate_expr(if_stmt.condition)?;
        if condition.is_truthy() {
            self.evaluate_stmt(*if_stmt.then_branch)?;
        } else if let Some(else_branch) = if_stmt.else_branch {
            self.evaluate_stmt(*else_branch)?;
        }
        Ok(())
    }

    fn visit_while_stmt(&mut self, while_stmt: WhileStmt) -> Result<(), RuntimeError> {
        while self
            .evaluate_expr(while_stmt.clone().condition)?
            .is_truthy()
        {
            self.evaluate_stmt(*while_stmt.clone().body)?;
        }
        Ok(())
    }

    fn visit_return_stmt(&mut self, return_stmt: ReturnStmt) -> Result<(), RuntimeError> {
        Err(RuntimeError::Return(
            if let Some(value) = return_stmt.value {
                Some(self.evaluate_expr(value)?)
            } else {
                None
            },
        ))
    }

    fn evaluate_block(
        &mut self,
        statements: Vec<Stmt>,
        environment: Environment,
    ) -> Result<(), RuntimeError> {
        let previous = std::mem::replace(&mut self.environment, Rc::new(RefCell::new(environment)));
        let result = self.block_loop(statements);
        self.environment = previous;
        result
    }

    fn block_loop(&mut self, stmts: Vec<Stmt>) -> Result<(), RuntimeError> {
        for stmt in stmts {
            self.evaluate_stmt(stmt)?;
        }
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
            Expr::Logical(logical) => self.visit_logical_expr(*logical),
            Expr::Call(call) => self.visit_call_expr(*call),
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
            TokenType::BANG => Ok(Value::Boolean(!right.is_truthy())),
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
        match self.environment.borrow().get(&variable.name.lexeme) {
            Ok(value) => Ok(value.clone()),
            Err(e) => Err(e),
        }
    }

    fn visit_assign_expr(&mut self, assign: AssignExpr) -> Result<Value, RuntimeError> {
        let value = self.evaluate_expr(assign.value)?;
        self.environment
            .borrow_mut()
            .assign(&assign.name.lexeme, value.clone())?;
        Ok(value)
    }

    fn visit_logical_expr(&mut self, logical: LogicalExpr) -> Result<Value, RuntimeError> {
        let left = self.evaluate_expr(logical.left)?;
        if logical.operator.r#type == TokenType::OR {
            if left.is_truthy() {
                return Ok(left.clone());
            }
        } else {
            if !left.is_truthy() {
                return Ok(left.clone());
            }
        }
        self.evaluate_expr(logical.right)
    }

    fn visit_call_expr(&mut self, call: CallExpr) -> Result<Value, RuntimeError> {
        let callee = self.evaluate_expr(call.callee)?;
        let mut arguments = Vec::new();
        for argument in call.arguments {
            arguments.push(self.evaluate_expr(argument)?);
        }

        match callee {
            Value::Function(function) => Ok(function.call(self, arguments)),
            _ => {
                tracing::error!("Can only call functions and classes");
                Err(RuntimeError::UnexpectedValue)
            }
        }
    }
}
