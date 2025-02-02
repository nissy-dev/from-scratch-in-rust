use crate::{
    lexer::Scanner,
    token::{Token, TokenType},
};

#[derive(Debug)]
pub enum ParseError {
    SyntaxError,
}

#[derive(Debug)]
pub struct Parser {
    scanner: Scanner,
    current: Option<Token>,
    previous: Option<Token>,
}

impl Parser {
    pub fn new(scanner: Scanner) -> Self {
        Parser {
            scanner,
            current: None,
            previous: None,
        }
    }

    pub fn advance(&mut self) -> Result<(), ParseError> {
        self.previous = self.current.take();
        loop {
            let token = self.scanner.scan_token();
            tracing::debug!("next token: {:?}", token);
            if !matches!(token.r#type, TokenType::Error(_)) {
                self.current = Some(token);
                break;
            }
            self.report_error(&token, "")?;
        }
        Ok(())
    }

    pub fn consume(&mut self, r#type: TokenType, message: &str) -> Result<(), ParseError> {
        if let Some(token) = &self.current {
            if token.r#type == r#type {
                return self.advance();
            }
            return self.report_error(token, message);
        }
        panic!("Parser.consume called with no current token, this situation should not happen");
    }

    pub fn current_token(&self) -> Result<Token, ParseError> {
        self.current.clone().ok_or_else(|| ParseError::SyntaxError)
    }

    pub fn previous_token(&self) -> Result<Token, ParseError> {
        self.previous.clone().ok_or_else(|| ParseError::SyntaxError)
    }

    fn report_error(&self, token: &Token, message: &str) -> Result<(), ParseError> {
        match &token.r#type {
            TokenType::Error(_) => {
                tracing::warn!(
                    "[line {}, col {}] Error: found invalid token: '{}'",
                    token.location.line,
                    token.location.column,
                    token.lexeme,
                );
            }
            TokenType::EOF => {
                tracing::warn!(
                    "[line {}, col {}] Error at end: '{}'",
                    token.location.line,
                    token.location.column,
                    token.lexeme
                );
            }
            _ => {
                tracing::warn!(
                    "[line {}, col {}] Error: {} at '{}'",
                    token.location.line,
                    token.location.column,
                    message,
                    token.lexeme
                );
            }
        }
        Err(ParseError::SyntaxError)
    }
}
