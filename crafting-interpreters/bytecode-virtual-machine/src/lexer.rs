use crate::token::{Location, Token, TokenType};

#[derive(Debug)]
pub struct Scanner {
    source: String,
    start: usize,
    current: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    pub fn new(source: String) -> Self {
        Scanner {
            source,
            start: 0,
            current: 0,
            line: 1,
            column: 0,
        }
    }

    pub fn scan_token(&mut self) -> Token {
        self.skip_whitespace();

        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenType::EOF);
        }

        let c = self.advance();
        tracing::debug!("current char: {}", c);
        match c {
            '(' => self.make_token(TokenType::LEFT_PAREN),
            ')' => self.make_token(TokenType::RIGHT_PAREN),
            '{' => self.make_token(TokenType::LEFT_BRACE),
            '}' => self.make_token(TokenType::RIGHT_BRACE),
            ';' => self.make_token(TokenType::SEMICOLON),
            ',' => self.make_token(TokenType::COMMA),
            '.' => self.make_token(TokenType::DOT),
            '-' => self.make_token(TokenType::MINUS),
            '+' => self.make_token(TokenType::PLUS),
            '/' => self.make_token(TokenType::SLASH),
            '*' => self.make_token(TokenType::STAR),
            '!' => {
                if self.match_char('=') {
                    self.make_token(TokenType::BANG_EQUAL)
                } else {
                    self.make_token(TokenType::BANG)
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.make_token(TokenType::EQUAL_EQUAL)
                } else {
                    self.make_token(TokenType::EQUAL)
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.make_token(TokenType::LESS_EQUAL)
                } else {
                    self.make_token(TokenType::LESS)
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.make_token(TokenType::GREATER_EQUAL)
                } else {
                    self.make_token(TokenType::GREATER)
                }
            }
            '"' => self.string(),
            _ if self.is_digit(c) => self.number(),
            _ if self.is_alpha(c) => self.identifier(),
            _ => {
                let error = format!("Unexpected character: {}", c);
                tracing::error!("{}", error);
                self.make_token(TokenType::Error(error))
            }
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn is_digit(&self, c: char) -> bool {
        c >= '0' && c <= '9'
    }

    fn is_alpha(&self, c: char) -> bool {
        (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
    }

    fn make_token(&mut self, r#type: TokenType) -> Token {
        let text = self.source[self.start..self.current].to_string();
        let location = Location {
            line: self.line,
            column: self.column,
        };
        Token::new(r#type, text, location)
    }

    fn advance(&mut self) -> char {
        let current_char = self.source.chars().nth(self.current).unwrap();
        self.current += 1;
        self.column += 1;
        current_char
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        let current_char = self.source.chars().nth(self.current).unwrap();
        if current_char != expected {
            return false;
        }
        self.current += 1;
        return true;
    }

    fn skip_whitespace(&mut self) {
        loop {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.advance();
                    self.line += 1;
                    self.column = 0;
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source.chars().nth(self.current).unwrap()
    }

    fn peek_next(&self) -> char {
        if (self.current + 1) >= self.source.len() {
            return '\0';
        }
        self.source.chars().nth(self.current + 1).unwrap()
    }

    fn string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            let char = self.advance();
            if char == '\n' {
                self.line += 1;
                self.column = 0;
            }
        }

        if self.is_at_end() {
            let error = format!("Unterminated string.");
            tracing::error!("{}", error);
            return self.make_token(TokenType::Error(error));
        }

        self.advance();
        let value = self.source[(self.start + 1)..(self.current - 1)].to_string();
        self.make_token(TokenType::STRING(value))
    }

    fn number(&mut self) -> Token {
        while self.is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == '.' && self.is_digit(self.peek_next()) {
            // consume the "."
            self.advance();
            while self.is_digit(self.peek()) {
                self.advance();
            }
        }

        let value = self.source[self.start as usize..self.current as usize]
            .parse::<f64>()
            .unwrap();
        self.make_token(TokenType::NUMBER(value))
    }

    fn identifier(&mut self) -> Token {
        while self.is_alpha(self.peek()) || self.is_digit(self.peek()) {
            self.advance();
        }

        // check reserved words
        let value = &self.source[self.start..self.current];
        let token_type = match value {
            "and" => TokenType::AND,
            "class" => TokenType::CLASS,
            "else" => TokenType::ELSE,
            "false" => TokenType::FALSE,
            "for" => TokenType::FOR,
            "fun" => TokenType::FUN,
            "if" => TokenType::IF,
            "nil" => TokenType::NIL,
            "or" => TokenType::OR,
            "print" => TokenType::PRINT,
            "return" => TokenType::RETURN,
            "super" => TokenType::SUPER,
            "this" => TokenType::THIS,
            "true" => TokenType::TRUE,
            "var" => TokenType::VAR,
            "while" => TokenType::WHILE,
            _ => TokenType::IDENTIFIER,
        };
        self.make_token(token_type)
    }
}
