use crate::{
    ast::{
        AssignExpr, BinaryExpr, BlockStmt, Expr, ExprStmt, GroupingExpr, LiteralExpr, PrintStmt,
        Stmt, UnaryExpr, VarDeclStmt, VariableExpr,
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
        if matches!(self.peek().r#type, TokenType::VAR) {
            self.advance();
            return self.var_declaration();
        }

        self.statement()
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
        if matches!(self.peek().r#type, TokenType::PRINT) {
            self.advance();
            return self.print_statement();
        }

        if matches!(self.peek().r#type, TokenType::LEFT_BRACE) {
            self.advance();
            return self.block_statement();
        }

        self.expression_statement()
    }

    fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = self.expression()?;
        self.consume(TokenType::SEMICOLON)?;
        Ok(Stmt::Print(Box::new(PrintStmt::new(value))))
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
        let expr = self.equality()?;

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
            expr = Expr::Binary(Box::new(BinaryExpr::new(expr, operator.clone(), right)));
        }

        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;

        while matches!(self.peek().r#type, TokenType::MINUS | TokenType::PLUS) {
            let operator = self.advance();
            let right = self.factor()?;
            expr = Expr::Binary(Box::new(BinaryExpr::new(expr, operator.clone(), right)));
        }

        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;

        while matches!(self.peek().r#type, TokenType::SLASH | TokenType::STAR) {
            let operator = self.advance();
            let right = self.unary()?;
            expr = Expr::Binary(Box::new(BinaryExpr::new(expr, operator.clone(), right)));
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.peek().r#type, TokenType::BANG | TokenType::MINUS) {
            let operator = self.advance();
            let right = self.unary()?;
            return Ok(Expr::Unary(Box::new(UnaryExpr::new(operator, right))));
        }

        self.primary()
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
