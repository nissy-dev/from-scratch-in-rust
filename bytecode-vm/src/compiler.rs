use std::collections::VecDeque;

use crate::{
    lexer::Scanner,
    parser::{ParseError, Parser},
    token::{Location, Precedence, Token, TokenType},
    value::{Object, Value},
};

#[derive(Debug, Clone)]
pub enum OpCode {
    Return,
    Constant(Value),
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Nil,
    True,
    False,
    Not,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal(usize),
    SetLocal(usize),
}

pub type OpCodes = VecDeque<(OpCode, Location)>;

#[derive(Debug)]
pub enum CompileError {
    InvalidSyntax,
    InvalidOperator,
    ParseError(ParseError),
}

impl From<ParseError> for CompileError {
    fn from(error: ParseError) -> Self {
        CompileError::ParseError(error)
    }
}

#[derive(Debug, Clone)]
struct Local {
    name: String,
    depth: isize,
}

impl Local {
    fn new(name: String) -> Self {
        Local { name, depth: -1 }
    }
}

#[derive(Debug)]
pub struct Compiler {
    parser: Parser,
    codes: OpCodes,
    locals: Vec<Local>,
    scope_depth: isize,
}

impl Compiler {
    pub fn new(source: String) -> Self {
        Compiler {
            parser: Parser::new(Scanner::new(source)),
            codes: VecDeque::new(),
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    pub fn compile(&mut self) -> Result<OpCodes, CompileError> {
        self.parser.advance()?;
        while !self.parser.match_token(TokenType::EOF)? {
            self.declaration()?;
        }
        self.parser
            .consume(TokenType::EOF, "Expect end of expression")?;
        self.write_op_code(OpCode::Return)?;
        Ok(self.codes.clone())
    }

    fn declaration(&mut self) -> Result<(), CompileError> {
        if self.parser.match_token(TokenType::VAR)? {
            self.var_declaration()
        } else {
            self.statement()
        }
    }

    fn var_declaration(&mut self) -> Result<(), CompileError> {
        self.parse_variable("Expect variable name")?;
        if self.parser.match_token(TokenType::EQUAL)? {
            self.expression()?;
        } else {
            self.write_op_code(OpCode::Nil)?;
        }
        self.parser.consume(
            TokenType::SEMICOLON,
            "Expect ';' after variable declaration",
        )?;
        self.define_variable()
    }

    fn statement(&mut self) -> Result<(), CompileError> {
        if self.parser.match_token(TokenType::PRINT)? {
            self.print_statement()
        } else if self.parser.match_token(TokenType::LEFT_BRACE)? {
            self.begin_scope();
            self.block()?;
            self.end_scope()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.parser
            .consume(TokenType::SEMICOLON, "Expect ';' after value")?;
        self.write_op_code(OpCode::Print)
    }

    fn block(&mut self) -> Result<(), CompileError> {
        while !self.parser.check_token(TokenType::RIGHT_BRACE)
            && !self.parser.check_token(TokenType::EOF)
        {
            self.declaration()?;
        }
        self.parser
            .consume(TokenType::RIGHT_BRACE, "Expect '}' after block")?;
        Ok(())
    }

    fn expression_statement(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.parser
            .consume(TokenType::SEMICOLON, "Expect ';' after expression")?;
        self.write_op_code(OpCode::Pop)
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn number(&mut self, value: f64) -> Result<(), CompileError> {
        let value = Value::Number(value);
        self.write_op_code(OpCode::Constant(value))
    }

    fn string(&mut self, value: String) -> Result<(), CompileError> {
        let object = Object::String(value);
        self.write_op_code(OpCode::Constant(Value::Object(object)))
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
            TokenType::MINUS => self.write_op_code(OpCode::Negate),
            TokenType::BANG => self.write_op_code(OpCode::Not),
            _ => {
                tracing::error!("Invalid operator: {:?}", token.r#type);
                Err(CompileError::InvalidOperator)
            }
        }
    }

    fn binary(&mut self) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        self.parse_precedence(token.precedence().next())?;

        match token.r#type {
            TokenType::MINUS => self.write_op_code(OpCode::Subtract),
            TokenType::PLUS => self.write_op_code(OpCode::Add),
            TokenType::STAR => self.write_op_code(OpCode::Multiply),
            TokenType::SLASH => self.write_op_code(OpCode::Divide),
            TokenType::BANG_EQUAL => {
                self.write_op_code(OpCode::Equal)?;
                self.write_op_code(OpCode::Not)
            }
            TokenType::EQUAL_EQUAL => self.write_op_code(OpCode::Equal),
            TokenType::GREATER => self.write_op_code(OpCode::Greater),
            TokenType::GREATER_EQUAL => {
                self.write_op_code(OpCode::Less)?;
                self.write_op_code(OpCode::Not)
            }
            TokenType::LESS => self.write_op_code(OpCode::Less),
            TokenType::LESS_EQUAL => {
                self.write_op_code(OpCode::Greater)?;
                self.write_op_code(OpCode::Not)
            }
            _ => {
                tracing::error!("Invalid binary operator: {:?}", token.r#type);
                Err(CompileError::InvalidOperator)
            }
        }
    }

    fn literal(&mut self) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        match token.r#type {
            TokenType::FALSE => self.write_op_code(OpCode::False),
            TokenType::NIL => self.write_op_code(OpCode::Nil),
            TokenType::TRUE => self.write_op_code(OpCode::True),
            _ => {
                tracing::error!("Invalid literal: {:?}", token.r#type);
                return Err(CompileError::InvalidOperator);
            }
        }
    }

    fn variable(&mut self, can_assign: bool) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        self.named_variable(token.lexeme, can_assign)
    }

    fn parse_prefix_expr(&mut self, token: Token, can_assign: bool) -> Result<(), CompileError> {
        match token.r#type {
            TokenType::NUMBER(value) => self.number(value),
            TokenType::STRING(value) => self.string(value),
            TokenType::LEFT_PAREN => self.grouping(),
            TokenType::MINUS | TokenType::BANG => self.unary(),
            TokenType::FALSE | TokenType::NIL | TokenType::TRUE => self.literal(),
            TokenType::IDENTIFIER => self.variable(can_assign),
            _ => Ok(()),
        }
    }

    fn parse_infix_expr(&mut self, token: Token) -> Result<(), CompileError> {
        match token.r#type {
            TokenType::MINUS | TokenType::PLUS | TokenType::STAR | TokenType::SLASH => {
                self.binary()
            }
            TokenType::EQUAL_EQUAL
            | TokenType::BANG_EQUAL
            | TokenType::GREATER
            | TokenType::GREATER_EQUAL
            | TokenType::LESS
            | TokenType::LESS_EQUAL => self.binary(),
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
        let can_assign = precedence <= Precedence::Assignment;

        self.parse_prefix_expr(self.parser.previous_token()?, can_assign)?;
        while precedence <= self.precedence(&self.parser.current_token()?) {
            self.parser.advance()?;
            self.parse_infix_expr(self.parser.previous_token()?)?;
        }

        if can_assign && self.parser.match_token(TokenType::EQUAL)? {
            tracing::error!("Invalid assignment target.");
            return Err(CompileError::InvalidSyntax);
        }

        Ok(())
    }

    fn parse_variable(&mut self, error_msg: &str) -> Result<(), CompileError> {
        self.parser.consume(TokenType::IDENTIFIER, error_msg)?;
        self.declare_variable()?;

        // local variable の処理
        if self.scope_depth > 0 {
            return Ok(());
        }

        // global variable の処理
        let token = self.parser.previous_token()?;
        let value = Value::Object(Object::String(token.lexeme));
        self.write_op_code(OpCode::Constant(value))
    }

    fn define_variable(&mut self) -> Result<(), CompileError> {
        // local variable の処理
        if self.scope_depth > 0 {
            if let Some(local) = self.locals.last_mut() {
                (*local).depth = self.scope_depth;
            }
            return Ok(());
        }

        // global variable の処理
        self.write_op_code(OpCode::DefineGlobal)
    }

    fn declare_variable(&mut self) -> Result<(), CompileError> {
        // global variable の処理
        if self.scope_depth == 0 {
            return Ok(());
        }

        // local variable の処理
        let name = self.parser.previous_token()?.lexeme;
        // 同じ名前の変数が同じスコープ内で宣言されていないかチェック
        for local in self.locals.iter().rev() {
            if local.depth < self.scope_depth {
                break;
            }
            if local.name == name {
                tracing::error!("Variable with this name already declared in this scope.");
                return Err(CompileError::InvalidSyntax);
            }
        }
        self.locals.push(Local::new(name));
        Ok(())
    }

    fn named_variable(&mut self, name: String, can_assign: bool) -> Result<(), CompileError> {
        if let Some(index) = self.resolve_local(&name) {
            if can_assign && self.parser.match_token(TokenType::EQUAL)? {
                self.expression()?;
                self.write_op_code(OpCode::SetLocal(index))
            } else {
                self.write_op_code(OpCode::GetLocal(index))
            }
        } else {
            let value = Value::Object(Object::String(name));
            self.write_op_code(OpCode::Constant(value))?;

            if can_assign && self.parser.match_token(TokenType::EQUAL)? {
                self.expression()?;
                self.write_op_code(OpCode::SetGlobal)
            } else {
                self.write_op_code(OpCode::GetGlobal)
            }
        }
    }

    fn write_op_code(&mut self, code: OpCode) -> Result<(), CompileError> {
        self.codes
            .push_back((code, self.parser.previous_token()?.location));
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) -> Result<(), CompileError> {
        self.scope_depth -= 1;
        let mut local_cnt = self.locals.len();
        while local_cnt > 0 && self.locals[local_cnt - 1].depth > self.scope_depth {
            self.write_op_code(OpCode::Pop)?;
            local_cnt -= 1;
        }
        Ok(())
    }

    fn resolve_local(&mut self, name: &str) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                if local.depth == -1 {
                    tracing::error!("Can't read local variable in its own initializer.");
                    return None;
                }
                return Some(i);
            }
        }
        None
    }
}
