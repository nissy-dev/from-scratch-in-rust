use crate::{
    ast::{
        AssignExpr, BinaryExpr, BlockStmt, CallExpr, Expr, ExprStmt, FunctionDeclStmt,
        GroupingExpr, IfStmt, LiteralExpr, LogicalExpr, PrintStmt, ReturnStmt, Stmt, UnaryExpr,
        VarDeclStmt, VariableExpr, WhileStmt,
    },
    lexer::{Token, TokenType},
};

#[derive(Debug)]
pub enum ParseError {
    SyntaxError,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            let decl = self.declaration();
            match decl {
                Ok(decl) => statements.push(decl),
                Err(e) => {
                    tracing::error!("{:?}", e);
                    self.synchronize();
                }
            }
        }
        Ok(statements)
    }

    fn declaration(&mut self) -> Result<Stmt, ParseError> {
        if matches!(self.peek().r#type, TokenType::FUN) {
            self.advance();
            return self.function_declaration();
        }

        if matches!(self.peek().r#type, TokenType::VAR) {
            self.advance();
            return self.var_declaration();
        }

        self.statement()
    }

    fn function_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self.consume(TokenType::IDENTIFIER)?;
        self.consume(TokenType::LEFT_PAREN)?;

        let mut parameters = Vec::new();
        if !matches!(self.peek().r#type, TokenType::RIGHT_PAREN) {
            loop {
                if parameters.len() >= 255 {
                    return Err(self.error(&self.peek(), "Cannot have more than 255 parameters."));
                }
                parameters.push(self.consume(TokenType::IDENTIFIER)?);
                if !matches!(self.peek().r#type, TokenType::COMMA) {
                    break;
                }
                self.advance();
            }
        }
        self.consume(TokenType::RIGHT_PAREN)?;

        self.consume(TokenType::LEFT_BRACE)?;
        let mut body = Vec::new();
        while !matches!(self.peek().r#type, TokenType::RIGHT_BRACE) && !self.is_at_end() {
            body.push(self.declaration()?)
        }
        self.consume(TokenType::RIGHT_BRACE)?;
        Ok(Stmt::FunctionDecl(Box::new(FunctionDeclStmt::new(
            name, parameters, body,
        ))))
    }

    fn var_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self.consume(TokenType::IDENTIFIER)?;
        let initializer = if matches!(self.peek().r#type, TokenType::EQUAL) {
            self.advance();
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::SEMICOLON)?;
        Ok(Stmt::VarDecl(Box::new(VarDeclStmt::new(name, initializer))))
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if matches!(self.peek().r#type, TokenType::FOR) {
            self.advance();
            return self.for_statement();
        }

        if matches!(self.peek().r#type, TokenType::IF) {
            self.advance();
            return self.if_statement();
        }

        if matches!(self.peek().r#type, TokenType::PRINT) {
            self.advance();
            return self.print_statement();
        }

        if matches!(self.peek().r#type, TokenType::RETURN) {
            self.advance();
            return self.return_statement();
        }

        if matches!(self.peek().r#type, TokenType::WHILE) {
            self.advance();
            return self.while_statement();
        }

        if matches!(self.peek().r#type, TokenType::LEFT_BRACE) {
            self.advance();
            return self.block_statement();
        }

        self.expression_statement()
    }

    // for loop は while loop の syntax sugar として処理する
    fn for_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LEFT_PAREN)?;

        let initializer = if matches!(self.peek().r#type, TokenType::SEMICOLON) {
            None
        } else if matches!(self.peek().r#type, TokenType::VAR) {
            self.advance();
            Some(self.var_declaration()?)
        } else {
            Some(self.expression_statement()?)
        };

        let condition = if !matches!(self.peek().r#type, TokenType::SEMICOLON) {
            self.expression()?
        } else {
            Expr::Literal(Box::new(LiteralExpr::new(Token::new(
                TokenType::TRUE,
                "true".to_string(),
                0,
                0,
            ))))
        };
        self.consume(TokenType::SEMICOLON)?;

        let increment = if !matches!(self.peek().r#type, TokenType::RIGHT_PAREN) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::RIGHT_PAREN)?;

        let mut body = self.statement()?;
        if let Some(increment) = increment {
            body = Stmt::Block(Box::new(BlockStmt::new(vec![
                body.clone(),
                Stmt::Expr(Box::new(ExprStmt::new(increment))),
            ])));
        }
        body = Stmt::While(Box::new(WhileStmt::new(condition, Box::new(body.clone()))));
        if let Some(initializer) = initializer {
            body = Stmt::Block(Box::new(BlockStmt::new(vec![initializer, body.clone()])));
        }
        Ok(body)
    }

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LEFT_PAREN)?;
        let condition = self.expression()?;
        self.consume(TokenType::RIGHT_PAREN)?;

        let then_branch = Box::new(self.statement()?);
        let else_branch = if matches!(self.peek().r#type, TokenType::ELSE) {
            self.advance();
            Some(Box::new(self.statement()?))
        } else {
            None
        };

        Ok(Stmt::If(Box::new(IfStmt::new(
            condition,
            then_branch,
            else_branch,
        ))))
    }

    fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = self.expression()?;
        self.consume(TokenType::SEMICOLON)?;
        Ok(Stmt::Print(Box::new(PrintStmt::new(value))))
    }

    fn return_statement(&mut self) -> Result<Stmt, ParseError> {
        let keyword = self.previous();
        let value = if !matches!(self.peek().r#type, TokenType::SEMICOLON) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::SEMICOLON)?;
        Ok(Stmt::Return(Box::new(ReturnStmt::new(keyword, value))))
    }

    fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LEFT_PAREN)?;
        let condition = self.expression()?;
        self.consume(TokenType::RIGHT_PAREN)?;

        let body = Box::new(self.statement()?);
        Ok(Stmt::While(Box::new(WhileStmt::new(condition, body))))
    }

    fn block_statement(&mut self) -> Result<Stmt, ParseError> {
        let mut statements = Vec::new();
        while !matches!(self.peek().r#type, TokenType::RIGHT_BRACE) && !self.is_at_end() {
            statements.push(self.declaration()?)
        }
        self.consume(TokenType::RIGHT_BRACE)?;
        Ok(Stmt::Block(Box::new(BlockStmt::new(statements))))
    }

    fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = self.expression()?;
        self.consume(TokenType::SEMICOLON)?;
        Ok(Stmt::Expr(Box::new(ExprStmt::new(value))))
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.logical_or()?;

        if matches!(self.peek().r#type, TokenType::EQUAL) {
            let equal = self.advance();
            let value = self.assignment()?;

            if let Expr::Variable(variable) = expr {
                return Ok(Expr::Assign(Box::new(AssignExpr::new(
                    variable.name,
                    value,
                ))));
            }

            return Err(self.error(&equal, "Invalid assignment target."));
        }

        Ok(expr)
    }

    fn logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.logical_and()?;

        while matches!(self.peek().r#type, TokenType::OR) {
            let operator = self.advance();
            let right = self.logical_and()?;
            expr = Expr::Logical(Box::new(LogicalExpr::new(expr, operator, right)));
        }

        Ok(expr)
    }

    fn logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;

        while matches!(self.peek().r#type, TokenType::AND) {
            let operator = self.advance();
            let right = self.equality()?;
            expr = Expr::Logical(Box::new(LogicalExpr::new(expr, operator, right)));
        }

        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;

        while matches!(
            self.peek().r#type,
            TokenType::BANG_EQUAL | TokenType::EQUAL_EQUAL
        ) {
            let operator = self.advance();
            let right = self.comparison()?;
            expr = Expr::Binary(Box::new(BinaryExpr::new(expr, operator, right)));
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;

        while matches!(
            self.peek().r#type,
            TokenType::GREATER | TokenType::GREATER_EQUAL | TokenType::LESS | TokenType::LESS_EQUAL
        ) {
            let operator = self.advance();
            let right = self.term()?;
            expr = Expr::Binary(Box::new(BinaryExpr::new(expr, operator, right)));
        }

        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;

        while matches!(self.peek().r#type, TokenType::MINUS | TokenType::PLUS) {
            let operator = self.advance();
            let right = self.factor()?;
            expr = Expr::Binary(Box::new(BinaryExpr::new(expr, operator, right)));
        }

        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;

        while matches!(self.peek().r#type, TokenType::SLASH | TokenType::STAR) {
            let operator = self.advance();
            let right = self.unary()?;
            expr = Expr::Binary(Box::new(BinaryExpr::new(expr, operator, right)));
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.peek().r#type, TokenType::BANG | TokenType::MINUS) {
            let operator = self.advance();
            let right = self.unary()?;
            return Ok(Expr::Unary(Box::new(UnaryExpr::new(operator, right))));
        }

        self.call()
    }

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;

        loop {
            if matches!(self.peek().r#type, TokenType::LEFT_PAREN) {
                self.advance();
                expr = self.finish_call(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        if matches!(
            self.peek().r#type,
            TokenType::FALSE
                | TokenType::TRUE
                | TokenType::NUMBER(_)
                | TokenType::STRING(_)
                | TokenType::NIL
        ) {
            let token = self.advance();
            return Ok(Expr::Literal(Box::new(LiteralExpr::new(token))));
        }

        if matches!(self.peek().r#type, TokenType::IDENTIFIER) {
            let token = self.advance();
            return Ok(Expr::Variable(Box::new(VariableExpr::new(token))));
        }

        if matches!(self.peek().r#type, TokenType::LEFT_PAREN) {
            self.advance();
            let expr = self.expression()?;
            self.consume(TokenType::RIGHT_PAREN)?;
            return Ok(Expr::Grouping(Box::new(GroupingExpr::new(expr))));
        }

        Err(self.error(&self.peek(), "expect expression, but not found."))
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let mut arguments = Vec::new();
        if !matches!(self.peek().r#type, TokenType::RIGHT_PAREN) {
            loop {
                if arguments.len() >= 255 {
                    return Err(self.error(&self.peek(), "Cannot have more than 255 arguments."));
                }
                arguments.push(self.expression()?);
                if !matches!(self.peek().r#type, TokenType::COMMA) {
                    break;
                }
                self.advance();
            }
        }
        let paren = self.consume(TokenType::RIGHT_PAREN)?;

        Ok(Expr::Call(Box::new(CallExpr::new(
            callee, paren, arguments,
        ))))
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }

        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().r#type == TokenType::EOF
    }

    fn peek(&self) -> Token {
        self.tokens[self.current].clone()
    }

    fn previous(&self) -> Token {
        self.tokens[self.current - 1].clone()
    }

    fn consume(&mut self, token_type: TokenType) -> Result<Token, ParseError> {
        let current_token_type = self.peek().r#type;
        if current_token_type == token_type {
            Ok(self.advance())
        } else {
            Err(self.error(&self.peek(), "expect token, but not found."))
        }
    }

    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if matches!(self.previous().r#type, TokenType::SEMICOLON) {
                return;
            }
            if matches!(
                self.peek().r#type,
                TokenType::CLASS
                    | TokenType::FUN
                    | TokenType::VAR
                    | TokenType::FOR
                    | TokenType::IF
                    | TokenType::WHILE
                    | TokenType::PRINT
                    | TokenType::RETURN
            ) {
                return;
            }
            self.advance();
        }
    }

    fn error(&self, token: &Token, message: &str) -> ParseError {
        if token.r#type == TokenType::EOF {
            tracing::warn!("[line {}] Error: {}  at end", token.line, message);
        } else {
            tracing::warn!(
                "[line {}] Error: {} at '{}'",
                token.line,
                message,
                token.lexeme
            );
        }
        return ParseError::SyntaxError;
    }
}
