use std::{cell::RefCell, rc::Rc};

use crate::{
    code::OpCode,
    lexer::Scanner,
    parser::{ParseError, Parser},
    token::{Precedence, Token, TokenType},
    value::{Function, FunctionType, Object, Value},
};

#[derive(Debug)]
pub enum CompileError {
    InvalidSyntax,
    InvalidJumpOperation,
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
    fn new(name: String, depth: isize) -> Self {
        Local { name, depth }
    }
}

#[derive(Debug)]
struct CompilerEnv {
    function: Function,
    function_type: FunctionType,
    locals: Vec<Local>,
    local_cnt: usize,
    scope_depth: isize,
    enclosing: Option<Rc<RefCell<CompilerEnv>>>,
}

impl CompilerEnv {
    fn new(enclosing: Option<Rc<RefCell<CompilerEnv>>>) -> Self {
        CompilerEnv {
            function: Function::new(""),
            function_type: FunctionType::Script,
            locals: Vec::new(),
            local_cnt: 0,
            scope_depth: 0,
            enclosing,
        }
    }
}

#[derive(Debug)]
pub struct Compiler {
    parser: Parser,
    env: Rc<RefCell<CompilerEnv>>,
}

impl Compiler {
    pub fn new(source: String) -> Self {
        Compiler {
            parser: Parser::new(Scanner::new(source)),
            env: Rc::new(RefCell::new(CompilerEnv::new(None))),
        }
    }

    fn stack_new_compiler(&mut self, function_type: FunctionType) -> Result<(), CompileError> {
        self.env.borrow_mut().function_type = function_type.clone();
        self.env = Rc::new(RefCell::new(CompilerEnv::new(Some(self.env.clone()))));
        if function_type != FunctionType::Script {
            let name = self.parser.previous_token()?.lexeme;
            self.env.borrow_mut().function.name = name;
        }
        Ok(())
    }

    pub fn compile(&mut self) -> Result<Function, CompileError> {
        self.parser.advance()?;
        while !self.parser.match_token(TokenType::EOF)? {
            self.declaration()?;
        }
        self.parser
            .consume(TokenType::EOF, "Expect end of expression")?;
        self.end_compiler()
    }

    fn end_compiler(&mut self) -> Result<Function, CompileError> {
        let return_function = self.env.borrow().function.clone();
        let enclosing_env = self.env.borrow().enclosing.clone();
        if enclosing_env.is_some() {
            self.env = enclosing_env.unwrap();
        }
        Ok(return_function)
    }

    fn declaration(&mut self) -> Result<(), CompileError> {
        if self.parser.match_token(TokenType::FUN)? {
            self.function_declaration()
        } else if self.parser.match_token(TokenType::VAR)? {
            self.var_declaration()
        } else {
            self.statement()
        }
    }

    fn function_declaration(&mut self) -> Result<(), CompileError> {
        self.parse_variable("Expect function name")?;
        self.mark_initialized();
        self.function(FunctionType::Function)?;
        self.define_variable()?;
        Ok(())
    }

    fn function(&mut self, function_type: FunctionType) -> Result<(), CompileError> {
        self.stack_new_compiler(function_type)?;
        self.begin_scope();
        self.parser
            .consume(TokenType::LEFT_PAREN, "Expect '(' after function name")?;

        if !self.parser.check_token(TokenType::RIGHT_PAREN) {
            loop {
                self.env.borrow_mut().function.arity += 1;
                if self.env.borrow().function.arity > 255 {
                    tracing::error!("Can't have more than 255 parameters.");
                    return Err(CompileError::InvalidSyntax);
                }
                self.parse_variable("Expect parameter name")?;
                self.define_variable()?;
                if !self.parser.match_token(TokenType::COMMA)? {
                    break;
                }
            }
        }
        self.parser
            .consume(TokenType::RIGHT_PAREN, "Expect ')' after parameters")?;
        self.parser
            .consume(TokenType::LEFT_BRACE, "Expect '{' before function body")?;
        self.block()?;

        let function = self.end_compiler()?;
        self.write_op_code(OpCode::Constant(Value::Object(Object::Function(function))))?;

        Ok(())
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
        self.define_variable()?;
        Ok(())
    }

    fn statement(&mut self) -> Result<(), CompileError> {
        if self.parser.match_token(TokenType::PRINT)? {
            self.print_statement()
        } else if self.parser.match_token(TokenType::IF)? {
            self.if_statement()
        } else if self.parser.match_token(TokenType::RETURN)? {
            self.return_statement()
        } else if self.parser.match_token(TokenType::WHILE)? {
            self.while_statement()
        } else if self.parser.match_token(TokenType::FOR)? {
            self.for_statement()
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
        self.write_op_code(OpCode::Print)?;
        Ok(())
    }

    fn if_statement(&mut self) -> Result<(), CompileError> {
        self.parser
            .consume(TokenType::LEFT_PAREN, "Expect '(' after 'if'")?;
        self.expression()?;
        self.parser
            .consume(TokenType::RIGHT_PAREN, "Expect ')' after condition")?;

        let then_jump = self.write_op_code(OpCode::JumpIfFalse(0))?;
        self.write_op_code(OpCode::Pop)?;
        self.statement()?;
        let else_jump = self.write_op_code(OpCode::Jump(0))?;
        self.patch_jump(then_jump)?;
        self.write_op_code(OpCode::Pop)?;

        if self.parser.match_token(TokenType::ELSE)? {
            self.statement()?;
        }
        self.patch_jump(else_jump)?;

        Ok(())
    }

    fn return_statement(&mut self) -> Result<(), CompileError> {
        if self.parser.match_token(TokenType::SEMICOLON)? {
            self.write_op_code(OpCode::Nil)?;
            self.write_op_code(OpCode::Return)?;
        } else {
            self.expression()?;
            self.parser
                .consume(TokenType::SEMICOLON, "Expect ';' after return value")?;
            self.write_op_code(OpCode::Return)?;
        }
        Ok(())
    }

    fn while_statement(&mut self) -> Result<(), CompileError> {
        let loop_start = self.env.borrow().function.codes.len();
        self.parser
            .consume(TokenType::LEFT_PAREN, "Expect '(' after 'while'")?;
        self.expression()?;
        self.parser
            .consume(TokenType::RIGHT_PAREN, "Expect ')' after condition")?;

        let exit_jump = self.write_op_code(OpCode::JumpIfFalse(0))?;
        self.write_op_code(OpCode::Pop)?;
        self.statement()?;
        self.emit_loop(loop_start)?;

        self.patch_jump(exit_jump)?;
        self.write_op_code(OpCode::Pop)?;
        Ok(())
    }

    fn for_statement(&mut self) -> Result<(), CompileError> {
        self.begin_scope();
        self.parser
            .consume(TokenType::LEFT_PAREN, "Expect '(' after 'for'")?;

        // initializer
        if self.parser.match_token(TokenType::SEMICOLON)? {
            // No initializer
        } else if self.parser.match_token(TokenType::VAR)? {
            self.var_declaration()?;
        } else {
            self.expression_statement()?;
        }

        // condition
        let mut loop_start = self.env.borrow().function.codes.len();
        let mut exit_jump = None;
        if !self.parser.match_token(TokenType::SEMICOLON)? {
            self.expression()?;
            self.parser
                .consume(TokenType::SEMICOLON, "Expect ';' after loop condition")?;
            exit_jump = Some(self.write_op_code(OpCode::JumpIfFalse(0))?);
            self.write_op_code(OpCode::Pop)?;
        }

        // increment
        if !self.parser.match_token(TokenType::RIGHT_PAREN)? {
            let body_jump = self.write_op_code(OpCode::Jump(0))?;
            let increment_start = self.env.borrow().function.codes.len();
            self.expression()?;
            self.write_op_code(OpCode::Pop)?;
            self.parser
                .consume(TokenType::RIGHT_PAREN, "Expect ')' after for clauses")?;

            self.emit_loop(loop_start)?;
            loop_start = increment_start;
            self.patch_jump(body_jump)?;
        }

        self.statement()?;
        self.emit_loop(loop_start)?;
        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump)?;
            self.write_op_code(OpCode::Pop)?;
        }
        self.end_scope()?;
        Ok(())
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
        self.write_op_code(OpCode::Pop)?;
        Ok(())
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn number(&mut self, value: f64) -> Result<(), CompileError> {
        let value = Value::Number(value);
        self.write_op_code(OpCode::Constant(value))?;
        Ok(())
    }

    fn string(&mut self, value: String) -> Result<(), CompileError> {
        let object = Object::String(value);
        self.write_op_code(OpCode::Constant(Value::Object(object)))?;
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
            TokenType::MINUS => {
                self.write_op_code(OpCode::Negate)?;
            }
            TokenType::BANG => {
                self.write_op_code(OpCode::Not)?;
            }
            _ => {
                tracing::error!("Invalid operator: {:?}", token.r#type);
                return Err(CompileError::InvalidOperator);
            }
        }

        Ok(())
    }

    fn binary(&mut self) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        self.parse_precedence(token.precedence().next())?;

        match token.r#type {
            TokenType::MINUS => {
                self.write_op_code(OpCode::Subtract)?;
            }
            TokenType::PLUS => {
                self.write_op_code(OpCode::Add)?;
            }
            TokenType::STAR => {
                self.write_op_code(OpCode::Multiply)?;
            }
            TokenType::SLASH => {
                self.write_op_code(OpCode::Divide)?;
            }
            TokenType::BANG_EQUAL => {
                self.write_op_code(OpCode::Equal)?;
                self.write_op_code(OpCode::Not)?;
            }
            TokenType::EQUAL_EQUAL => {
                self.write_op_code(OpCode::Equal)?;
            }
            TokenType::GREATER => {
                self.write_op_code(OpCode::Greater)?;
            }
            TokenType::GREATER_EQUAL => {
                self.write_op_code(OpCode::Less)?;
                self.write_op_code(OpCode::Not)?;
            }
            TokenType::LESS => {
                self.write_op_code(OpCode::Less)?;
            }
            TokenType::LESS_EQUAL => {
                self.write_op_code(OpCode::Greater)?;
                self.write_op_code(OpCode::Not)?;
            }
            _ => {
                tracing::error!("Invalid binary operator: {:?}", token.r#type);
                return Err(CompileError::InvalidOperator);
            }
        }

        Ok(())
    }

    fn literal(&mut self) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        match token.r#type {
            TokenType::FALSE => {
                self.write_op_code(OpCode::False)?;
            }
            TokenType::NIL => {
                self.write_op_code(OpCode::Nil)?;
            }
            TokenType::TRUE => {
                self.write_op_code(OpCode::True)?;
            }
            _ => {
                tracing::error!("Invalid literal: {:?}", token.r#type);
                return Err(CompileError::InvalidOperator);
            }
        }

        Ok(())
    }

    fn variable(&mut self, can_assign: bool) -> Result<(), CompileError> {
        let token = self.parser.previous_token()?;
        self.named_variable(token.lexeme, can_assign)
    }

    fn and(&mut self) -> Result<(), CompileError> {
        let end_jump = self.write_op_code(OpCode::JumpIfFalse(0))?;
        self.parse_precedence(Precedence::And)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    fn or(&mut self) -> Result<(), CompileError> {
        let else_jump = self.write_op_code(OpCode::JumpIfFalse(0))?;
        let end_jump = self.write_op_code(OpCode::Jump(0))?;
        self.patch_jump(else_jump)?;
        self.write_op_code(OpCode::Pop)?;
        self.parse_precedence(Precedence::Or)?;
        self.patch_jump(end_jump)?;
        Ok(())
    }

    fn call(&mut self) -> Result<(), CompileError> {
        let arg_count = self.argument_list()?;
        self.write_op_code(OpCode::Call(arg_count))?;
        Ok(())
    }

    fn argument_list(&mut self) -> Result<usize, CompileError> {
        let mut arg_count = 0;
        if !self.parser.check_token(TokenType::RIGHT_PAREN) {
            loop {
                self.expression()?;
                arg_count += 1;
                if !self.parser.match_token(TokenType::COMMA)? {
                    break;
                }
            }
        }
        self.parser
            .consume(TokenType::RIGHT_PAREN, "Expect ')' after arguments")?;
        Ok(arg_count)
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
            TokenType::AND => self.and(),
            TokenType::OR => self.or(),
            TokenType::LEFT_PAREN => self.call(),
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
            TokenType::AND => Precedence::And,
            TokenType::OR => Precedence::Or,
            TokenType::LEFT_PAREN => Precedence::Call,
            _ => Precedence::None,
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        self.parser.advance()?;
        let can_assign = precedence <= Precedence::Assignment;
        // TODO: prefix expression がない場合は early return しているが、ここはそうなっていない
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
        if self.env.borrow().scope_depth > 0 {
            return Ok(());
        }
        // global variable の処理
        let token = self.parser.previous_token()?;
        let value = Value::Object(Object::String(token.lexeme));
        self.write_op_code(OpCode::Constant(value))?;
        Ok(())
    }

    fn define_variable(&mut self) -> Result<(), CompileError> {
        // local variable の処理
        if self.env.borrow().scope_depth > 0 {
            self.mark_initialized();
            return Ok(());
        }
        // global variable の処理
        self.write_op_code(OpCode::DefineGlobal)?;
        Ok(())
    }

    fn declare_variable(&mut self) -> Result<(), CompileError> {
        // global variable の処理
        if self.env.borrow().scope_depth == 0 {
            return Ok(());
        }

        // local variable の処理
        let name = self.parser.previous_token()?.lexeme;
        // 同じ名前の変数が同じスコープ内で宣言されていないかチェック
        for i in (0..self.env.borrow().local_cnt).rev() {
            let local = &self.env.borrow().locals[i];
            if local.depth != -1 && local.depth < self.env.borrow().scope_depth {
                break;
            }
            if local.name == name {
                tracing::error!("Variable with this name already declared in this scope.");
                return Err(CompileError::InvalidSyntax);
            }
        }

        self.add_local(name);
        Ok(())
    }

    fn named_variable(&mut self, name: String, can_assign: bool) -> Result<(), CompileError> {
        if let Some(index) = self.resolve_local(&name) {
            if can_assign && self.parser.match_token(TokenType::EQUAL)? {
                self.expression()?;
                self.write_op_code(OpCode::SetLocal(index))?;
            } else {
                self.write_op_code(OpCode::GetLocal(index))?;
            }
        } else {
            let value = Value::Object(Object::String(name));
            self.write_op_code(OpCode::Constant(value))?;

            if can_assign && self.parser.match_token(TokenType::EQUAL)? {
                self.expression()?;
                self.write_op_code(OpCode::SetGlobal)?;
            } else {
                self.write_op_code(OpCode::GetGlobal)?;
            }
        }

        Ok(())
    }

    fn write_op_code(&mut self, code: OpCode) -> Result<usize, CompileError> {
        self.env
            .borrow_mut()
            .function
            .codes
            .push((code, self.parser.previous_token()?.location));
        Ok(self.env.borrow().function.codes.len() - 1)
    }

    fn begin_scope(&mut self) {
        self.env.borrow_mut().scope_depth += 1;
    }

    fn end_scope(&mut self) -> Result<(), CompileError> {
        self.env.borrow_mut().scope_depth -= 1;
        while self.env.borrow().local_cnt > 0
            && self.env.borrow().locals[self.env.borrow().local_cnt - 1].depth
                > self.env.borrow().scope_depth
        {
            // TODO: locals からスコープ外の変数を削除しないとうまく動かなかったが正しいのか...?
            self.env.borrow_mut().locals.pop();
            self.write_op_code(OpCode::Pop)?;
            self.env.borrow_mut().local_cnt -= 1;
        }
        Ok(())
    }

    fn add_local(&mut self, name: String) {
        self.env.borrow_mut().locals.push(Local::new(name, -1));
        self.env.borrow_mut().local_cnt += 1;
    }

    fn resolve_local(&mut self, name: &str) -> Option<usize> {
        for i in (0..self.env.borrow().local_cnt).rev() {
            let local = &self.env.borrow().locals[i];
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

    fn mark_initialized(&mut self) {
        let (index, scope_depth) = {
            let borrow_env = self.env.borrow();
            if borrow_env.local_cnt == 0 {
                return;
            }
            (borrow_env.local_cnt - 1, borrow_env.scope_depth)
        };
        self.env.borrow_mut().locals[index].depth = scope_depth;
    }

    fn patch_jump(&mut self, offset: usize) -> Result<(), CompileError> {
        let jump = {
            let borrow_env = self.env.borrow();
            borrow_env.function.codes.len() - offset - 1
        };
        let mut borrow_env = self.env.borrow_mut();
        match borrow_env.function.codes.get(offset) {
            Some((OpCode::JumpIfFalse(_), loc)) => {
                borrow_env.function.codes[offset] = (OpCode::JumpIfFalse(jump), loc.clone());
            }
            Some((OpCode::Jump(_), loc)) => {
                borrow_env.function.codes[offset] = (OpCode::Jump(jump), loc.clone());
            }
            _ => {
                tracing::error!("Invalid jump operand");
                return Err(CompileError::InvalidJumpOperation);
            }
        }

        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) -> Result<(), CompileError> {
        let offset = self.env.borrow().function.codes.len() - loop_start + 1;
        self.write_op_code(OpCode::Loop(offset))?;
        Ok(())
    }
}
