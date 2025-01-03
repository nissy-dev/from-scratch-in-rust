use crate::{
    ast::{BinaryExpr, Expr, GroupingExpr, LiteralExpr, UnaryExpr},
    lexer::TokenType,
};

#[derive(Debug)]
pub enum RuntimeError {
    UnexpectedValue,
    UnexpectedOperator,
}

#[derive(Debug)]
pub enum Value {
    String(String),
    Number(f64),
    Null,
    Boolean(bool),
}

pub struct Interpreter {}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {}
    }

    pub fn interpret(&self, expression: Expr) -> Result<Value, RuntimeError> {
        self.evaluate(expression)
    }

    fn visit_literal(&self, literal: LiteralExpr) -> Result<Value, RuntimeError> {
        match literal.value.r#type {
            TokenType::STRING(val) => Ok(Value::String(val)),
            TokenType::NUMBER(val) => Ok(Value::Number(val)),
            TokenType::NIL => Ok(Value::Null),
            TokenType::FALSE => Ok(Value::Boolean(false)),
            TokenType::TRUE => Ok(Value::Boolean(true)),
            _ => panic!("Unexpected token"),
        }
    }

    fn visit_grouping(&self, grouping: GroupingExpr) -> Result<Value, RuntimeError> {
        return self.evaluate(grouping.expression);
    }

    fn visit_unary(&self, unary: UnaryExpr) -> Result<Value, RuntimeError> {
        let right = self.evaluate(unary.right)?;
        match unary.operator.r#type {
            TokenType::MINUS => match right {
                Value::Number(val) => Ok(Value::Number(-val)),
                _ => Err(RuntimeError::UnexpectedOperator),
            },
            TokenType::BANG => Ok(Value::Boolean(!self.is_truthy(right))),
            _ => Err(RuntimeError::UnexpectedOperator),
        }
    }

    fn is_truthy(&self, value: Value) -> bool {
        match value {
            Value::Null => false,
            Value::Boolean(val) => val,
            _ => true,
        }
    }

    fn visit_binary(&self, binary: BinaryExpr) -> Result<Value, RuntimeError> {
        let left = self.evaluate(binary.left)?;
        let right = self.evaluate(binary.right)?;
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

    fn evaluate(&self, expression: Expr) -> Result<Value, RuntimeError> {
        match expression {
            Expr::Literal(literal) => self.visit_literal(*literal),
            Expr::Grouping(grouping) => self.visit_grouping(*grouping),
            Expr::Unary(unary) => self.visit_unary(*unary),
            Expr::Binary(binary) => self.visit_binary(*binary),
        }
    }
}
