use std::collections::VecDeque;

use crate::{
    lexer::Scanner,
    parser::{ParseError, Parser},
    token::{Location, Precedence, Token, TokenType},
};

#[derive(Debug, Copy, Clone)]
pub enum OpCode {
    Return,
    Constant(Value),
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Copy, Clone)]
pub enum Value {
    Number(f64),
}

pub type OpCodes = VecDeque<(OpCode, Location)>;

#[derive(Debug)]
pub enum CompileError {
    InvalidOperator,
    ParseError(ParseError),
}

impl From<ParseError> for CompileError {
    fn from(error: ParseError) -> Self {
        CompileError::ParseError(error)
    }
}

#[derive(Debug)]
pub struct Compiler {
    parser: Parser,
    pub op_codes: OpCodes,
}

impl Compiler {
    pub fn new(source: String) -> Self {
        Compiler {
            parser: Parser::new(Scanner::new(source)),
            op_codes: VecDeque::new(),
        }
    }

    pub fn compile(&mut self) -> Result<(), CompileError> {
        self.parser.advance()?;
        self.expression()?;
        self.parser
            .consume(TokenType::EOF, "Expect end of expression")?;
        self.write_op_code(OpCode::Return)?;

        Ok(())
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.parse_precedence(Precedence::Assignment)?;
        Ok(())
    }

    fn number(&mut self, value: f64) -> Result<(), CompileError> {
        let value = Value::Number(value);
        self.write_op_code(OpCode::Constant(value))?;
        Ok(())
    }

    fn grouping(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.parser
            .consume(TokenType::RIGHT_PAREN, "Expect ')' after expression")?;
        Ok(())
    }

    fn unary(&mut self) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        self.parse_precedence(Precedence::Unary)?;
        match token.r#type {
            TokenType::MINUS => self.write_op_code(OpCode::Negate)?,
            _ => {
                tracing::error!("Invalid operator: {:?}", token.r#type);
                return Err(CompileError::InvalidOperator);
            }
        }
        Ok(())
    }

    fn binary(&mut self) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        let precedence = token.precedence();
        self.parse_precedence(precedence.next())?;

        match token.r#type {
            TokenType::MINUS => self.write_op_code(OpCode::Subtract)?,
            TokenType::PLUS => self.write_op_code(OpCode::Add)?,
            TokenType::STAR => self.write_op_code(OpCode::Multiply)?,
            TokenType::SLASH => self.write_op_code(OpCode::Divide)?,
            _ => {
                tracing::error!("Invalid binary operator: {:?}", token.r#type);
                return Err(CompileError::InvalidOperator);
            }
        }

        Ok(())
    }

    fn parse_prefix_expr(&mut self, token: &Token) -> Result<(), CompileError> {
        match token.r#type {
            TokenType::NUMBER(value) => self.number(value),
            TokenType::LEFT_PAREN => self.grouping(),
            TokenType::MINUS => self.unary(),
            _ => Ok(()),
        }
    }

    fn parse_infix_expr(&mut self, token: &Token) -> Result<(), CompileError> {
        match token.r#type {
            TokenType::MINUS | TokenType::PLUS | TokenType::STAR | TokenType::SLASH => {
                self.binary()
            }
            _ => Ok(()),
        }
    }

    fn precedence(&self, token: &Token) -> Precedence {
        match token.r#type {
            TokenType::STAR | TokenType::SLASH => Precedence::Factor,
            TokenType::PLUS | TokenType::MINUS => Precedence::Term,
            TokenType::EQUAL_EQUAL | TokenType::BANG_EQUAL => Precedence::Equality,
            TokenType::GREATER
            | TokenType::GREATER_EQUAL
            | TokenType::LESS
            | TokenType::LESS_EQUAL => Precedence::Comparison,
            _ => Precedence::None,
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        self.parser.advance()?;
        // TODO: prefix expression がない場合は early return しているが、ここはそうなっていない
        self.parse_prefix_expr(&self.parser.previous_token()?)?;
        while precedence <= self.precedence(&self.parser.current_token()?) {
            self.parser.advance()?;
            self.parse_infix_expr(&self.parser.previous_token()?)?;
        }
        Ok(())
    }

    fn write_op_code(&mut self, instruction: OpCode) -> Result<(), CompileError> {
        self.op_codes
            .push_back((instruction, self.parser.previous_token()?.location));
        Ok(())
    }
}
